//! Noise-based effects: tyre skid, crash boom and post-crash screech.
//!
//! A reverse-biased zener diode makes noise that op-amp `H4_2` amplifies so
//! hard that its output flips randomly between its rails. Op-amp `H4_1`
//! smooths that into analogue noise, which three active filters shape into
//! the effects.

use super::SoundLatches;
use super::filters::{Biquad, OnePole};

/// How often the noise generator may change level. MAME's high-level
/// model of the generator uses the same rate.
const NOISE_CLOCK_HZ: f64 = 24_000.0;
/// Smoothing filter `H4_1`: corner frequency and gain.
const SMOOTHING_CUTOFF_HZ: f64 = 6_300.0;
const SMOOTHING_GAIN: f64 = 0.8;
/// Tyre skid filter (`H4_3` and `H4_4`, after National Semiconductor's
/// application note 72): a narrow band-pass around 1 kHz.
const SKID_CENTRE_HZ: f64 = 1_000.0;
const SKID_Q: f64 = 25.0;
const SKID_GAIN: f64 = 15.0;
/// Post-crash filter `H5_2`: a low-pass with so much gain that its output
/// is heavily clipped noise.
const POST_CRASH_CUTOFF_HZ: f64 = 1_000.0;
const POST_CRASH_GAIN: f64 = 100.0;
/// Boom filter `H5_1`: a very narrow, very loud band-pass around 60 Hz.
const BOOM_CENTRE_HZ: f64 = 60.0;
const BOOM_Q: f64 = 19.0;
const BOOM_GAIN: f64 = 200.0;
/// How quickly the CD4016 switches at the skid and post-crash outputs
/// fade, to avoid clicks harsher than the hardware's.
const SWITCH_TIME_CONSTANT: f64 = 0.001;

/// The three effects for one sample, each nominally within ±1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct EffectsSample {
    pub(crate) skid: f64,
    pub(crate) post_crash: f64,
    pub(crate) boom: f64,
}

/// The noise generator and the three effect filters.
#[derive(Clone, Debug)]
pub(crate) struct Effects {
    noise: NoiseGenerator,
    smoothing: OnePole,
    skid: Biquad,
    post_crash: Biquad,
    boom: Biquad,
    skid_switch: OnePole,
    post_crash_switch: OnePole,
}

impl Effects {
    pub(crate) fn new(sample_rate: f64) -> Self {
        Self {
            noise: NoiseGenerator::new(sample_rate),
            smoothing: OnePole::low_pass(sample_rate, SMOOTHING_CUTOFF_HZ),
            skid: Biquad::band_pass(sample_rate, SKID_CENTRE_HZ, SKID_Q),
            post_crash: Biquad::low_pass(sample_rate, POST_CRASH_CUTOFF_HZ, 0.5f64.sqrt()),
            boom: Biquad::band_pass(sample_rate, BOOM_CENTRE_HZ, BOOM_Q),
            skid_switch: OnePole::with_time_constant(sample_rate, SWITCH_TIME_CONSTANT),
            post_crash_switch: OnePole::with_time_constant(sample_rate, SWITCH_TIME_CONSTANT),
        }
    }

    pub(crate) fn sample(&mut self, latches: SoundLatches) -> EffectsSample {
        let noise = SMOOTHING_GAIN * self.smoothing.process(self.noise.next());

        // The skid and post-crash filters run all the time; CD4016
        // switches connect their outputs to the mixer.
        let skid = saturate(SKID_GAIN * self.skid.process(noise))
            * self.skid_switch.process(on(latches.skid()));
        let post_crash = saturate(POST_CRASH_GAIN * self.post_crash.process(noise))
            * self.post_crash_switch.process(on(latches.post_crash()));

        // The boom's switch sits at the filter's input instead, so the
        // filter keeps ringing for a moment after it opens.
        let boom_input = if latches.boom() { noise } else { 0.0 };
        let boom = saturate(BOOM_GAIN * self.boom.process(boom_input));

        EffectsSample {
            skid,
            post_crash,
            boom,
        }
    }
}

/// Random two-level noise, like the saturated output of `H4_2`.
#[derive(Clone, Copy, Debug)]
struct NoiseGenerator {
    state: u32,
    phase: f64,
    step: f64,
    level: f64,
}

impl NoiseGenerator {
    fn new(sample_rate: f64) -> Self {
        Self {
            state: 0x2545_f491,
            phase: 0.0,
            step: NOISE_CLOCK_HZ / sample_rate,
            level: 1.0,
        }
    }

    fn next(&mut self) -> f64 {
        self.phase += self.step;
        while self.phase >= 1.0 {
            self.phase -= 1.0;
            // Marsaglia's xorshift32.
            self.state ^= self.state << 13;
            self.state ^= self.state >> 17;
            self.state ^= self.state << 5;
            self.level = if self.state & 0x8000_0000 == 0 {
                -1.0
            } else {
                1.0
            };
        }
        self.level
    }
}

fn on(switch: bool) -> f64 {
    if switch { 1.0 } else { 0.0 }
}

/// Soft-clips a signal into ±1, like an op-amp driven into its rails.
fn saturate(signal: f64) -> f64 {
    signal.tanh()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sound::filters::DcBlocker;

    const RATE: u32 = 48_000;

    fn effects(effects: u8) -> SoundLatches {
        SoundLatches { engine: 0, effects }
    }

    /// Collects `count` samples of one effect.
    fn record(
        generator: &mut Effects,
        latches: SoundLatches,
        count: u32,
        pick: fn(EffectsSample) -> f64,
    ) -> Vec<f64> {
        (0..count)
            .map(|_| pick(generator.sample(latches)))
            .collect()
    }

    fn rms(samples: &[f64]) -> f64 {
        let energy: f64 = samples.iter().map(|sample| sample * sample).sum();
        (energy / f64::from(u32::try_from(samples.len()).unwrap())).sqrt()
    }

    /// Counts sign changes, twice the dominant frequency for narrow-band
    /// signals.
    fn zero_crossings(samples: &[f64]) -> usize {
        samples
            .windows(2)
            .filter(|pair| (pair[0] < 0.0) != (pair[1] < 0.0))
            .count()
    }

    #[test]
    fn effects_are_silent_until_switched_on() {
        let mut generator = Effects::new(f64::from(RATE));
        let samples: Vec<_> = (0..RATE).map(|_| generator.sample(effects(0))).collect();
        assert!(
            samples
                .iter()
                .all(|sample| *sample == EffectsSample::default())
        );
    }

    #[test]
    fn skid_screeches_around_1_khz() {
        let mut generator = Effects::new(f64::from(RATE));
        let skid = record(&mut generator, effects(0x04), RATE, |sample| sample.skid);
        assert!(rms(&skid) > 0.2, "{}", rms(&skid));
        let crossings = zero_crossings(&skid[RATE as usize / 10..]);
        assert!((1_700..2_000).contains(&crossings), "{crossings}");
    }

    #[test]
    fn post_crash_noise_is_loud_and_clipped() {
        let mut generator = Effects::new(f64::from(RATE));
        let screech = record(&mut generator, effects(0x08), RATE, |sample| {
            sample.post_crash
        });
        assert!(rms(&screech) > 0.7, "{}", rms(&screech));
    }

    #[test]
    fn boom_rumbles_around_60_hz_and_rings_after_release() {
        let mut generator = Effects::new(f64::from(RATE));
        let boom = record(&mut generator, effects(0x01), RATE, |sample| sample.boom);
        assert!(rms(&boom) > 0.5, "{}", rms(&boom));

        // The filter's skirts let a little hiss through, so look at the
        // rumble below 150 Hz and check that it carries most of the energy.
        let mut rumble_filter = OnePole::low_pass(f64::from(RATE), 150.0);
        let rumble: Vec<f64> = boom
            .iter()
            .map(|&sample| rumble_filter.process(sample))
            .collect();
        let crossings = zero_crossings(&rumble[RATE as usize / 10..]);
        assert!((90..130).contains(&crossings), "{crossings}");
        let mut hiss_filter = DcBlocker::new(f64::from(RATE), 500.0);
        let hiss: Vec<f64> = boom
            .iter()
            .map(|&sample| hiss_filter.process(sample))
            .collect();
        assert!(
            rms(&hiss) < 0.3 * rms(&boom),
            "{} vs {}",
            rms(&hiss),
            rms(&boom)
        );

        let tail = record(&mut generator, effects(0), RATE, |sample| sample.boom);
        assert!(rms(&tail[..960]) > 0.1, "rings for a while");
        assert!(rms(&tail[RATE as usize / 2..]) < 0.01, "then dies away");
    }

    #[test]
    fn noise_is_deterministic() {
        let mut first = Effects::new(44_100.0);
        let mut second = Effects::new(44_100.0);
        for _ in 0..1_000 {
            assert_eq!(first.sample(effects(0x0d)), second.sample(effects(0x0d)));
        }
    }
}
