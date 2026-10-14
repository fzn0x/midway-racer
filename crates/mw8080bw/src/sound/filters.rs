//! Signal processing building blocks.

use std::f64::consts::TAU;

/// A first-order low-pass filter, the digital twin of an RC network.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct OnePole {
    coefficient: f64,
    state: f64,
}

impl OnePole {
    /// Creates a filter with an RC `time_constant` in seconds.
    pub(crate) fn with_time_constant(sample_rate: f64, time_constant: f64) -> Self {
        Self {
            coefficient: smoothing(sample_rate, time_constant),
            state: 0.0,
        }
    }

    /// Creates a filter with a -3 dB `cutoff` in hertz.
    pub(crate) fn low_pass(sample_rate: f64, cutoff: f64) -> Self {
        Self::with_time_constant(sample_rate, 1.0 / (TAU * cutoff))
    }

    /// Filters one sample, returning the low-passed output.
    pub(crate) fn process(&mut self, input: f64) -> f64 {
        self.state += (input - self.state) * self.coefficient;
        self.state
    }
}

/// A first-order high-pass filter, the digital twin of a coupling
/// capacitor: it removes DC and the lowest frequencies.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DcBlocker {
    low_pass: OnePole,
}

impl DcBlocker {
    /// Creates a filter with a -3 dB `cutoff` in hertz.
    pub(crate) fn new(sample_rate: f64, cutoff: f64) -> Self {
        Self {
            low_pass: OnePole::low_pass(sample_rate, cutoff),
        }
    }

    /// Filters one sample.
    pub(crate) fn process(&mut self, input: f64) -> f64 {
        input - self.low_pass.process(input)
    }
}

/// A second-order filter in transposed direct form II, with coefficients
/// from Robert Bristow-Johnson's audio EQ cookbook.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    s1: f64,
    s2: f64,
}

impl Biquad {
    /// Creates a band-pass filter with unity gain at `centre` hertz.
    pub(crate) fn band_pass(sample_rate: f64, centre: f64, q: f64) -> Self {
        let (cos, alpha) = Self::prototype(sample_rate, centre, q);
        Self::normalized([alpha, 0.0, -alpha], [1.0 + alpha, -2.0 * cos, 1.0 - alpha])
    }

    /// Creates a low-pass filter with its corner at `cutoff` hertz.
    pub(crate) fn low_pass(sample_rate: f64, cutoff: f64, q: f64) -> Self {
        let (cos, alpha) = Self::prototype(sample_rate, cutoff, q);
        let b = (1.0 - cos) / 2.0;
        Self::normalized([b, 2.0 * b, b], [1.0 + alpha, -2.0 * cos, 1.0 - alpha])
    }

    /// Returns the cosine of the angular frequency and the bandwidth term.
    fn prototype(sample_rate: f64, frequency: f64, q: f64) -> (f64, f64) {
        let omega = TAU * frequency / sample_rate;
        (omega.cos(), omega.sin() / (2.0 * q))
    }

    fn normalized([b0, b1, b2]: [f64; 3], [a0, a1, a2]: [f64; 3]) -> Self {
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            s1: 0.0,
            s2: 0.0,
        }
    }

    /// Filters one sample.
    pub(crate) fn process(&mut self, input: f64) -> f64 {
        let output = self.b0 * input + self.s1;
        self.s1 = self.b1 * input - self.a1 * output + self.s2;
        self.s2 = self.b2 * input - self.a2 * output;
        output
    }
}

/// Returns the per-sample step of an RC charge with `time_constant`
/// seconds: `1 - e^(-dt/τ)`.
pub(crate) fn smoothing(sample_rate: f64, time_constant: f64) -> f64 {
    -(-1.0 / (sample_rate * time_constant)).exp_m1()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f64 = 48_000.0;

    /// Peak amplitude of a filter's steady-state response to a sine,
    /// measured over the second half of half a second.
    fn gain(mut filter: impl FnMut(f64) -> f64, frequency: f64) -> f64 {
        (0..24_000)
            .map(|index| filter((TAU * frequency * f64::from(index) / RATE).sin()))
            .skip(12_000)
            .fold(0.0, |peak, sample: f64| peak.max(sample.abs()))
    }

    #[test]
    fn rc_smoothing_reaches_63_percent_after_one_time_constant() {
        let mut filter = OnePole::with_time_constant(RATE, 0.01);
        let mut output = 0.0;
        for _ in 0..480 {
            output = filter.process(1.0);
        }
        assert!((output - (1.0 - (-1.0f64).exp())).abs() < 1e-3);
    }

    #[test]
    fn low_pass_halves_power_at_cutoff() {
        let mut filter = OnePole::low_pass(RATE, 1_000.0);
        let gain = gain(|sample| filter.process(sample), 1_000.0);
        assert!((gain - 0.5f64.sqrt()).abs() < 0.03, "{gain}");
    }

    #[test]
    fn band_pass_peaks_at_its_centre() {
        let mut filter = Biquad::band_pass(RATE, 1_000.0, 25.0);
        assert!((gain(|sample| filter.process(sample), 1_000.0) - 1.0).abs() < 0.02);

        let mut filter = Biquad::band_pass(RATE, 1_000.0, 25.0);
        assert!(gain(|sample| filter.process(sample), 2_000.0) < 0.05);
    }

    #[test]
    fn low_pass_passes_bass_and_cuts_treble() {
        let mut filter = Biquad::low_pass(RATE, 1_000.0, 0.5f64.sqrt());
        assert!((gain(|sample| filter.process(sample), 50.0) - 1.0).abs() < 0.01);

        let mut filter = Biquad::low_pass(RATE, 1_000.0, 0.5f64.sqrt());
        assert!(gain(|sample| filter.process(sample), 10_000.0) < 0.02);
    }

    #[test]
    fn dc_blocker_removes_offset_but_passes_audio() {
        let mut filter = DcBlocker::new(RATE, 10.0);
        let mut output = 0.0;
        for _ in 0..48_000 {
            output = filter.process(1.0);
        }
        assert!(output.abs() < 1e-3);

        let mut filter = DcBlocker::new(RATE, 10.0);
        assert!(gain(|sample| filter.process(sample), 1_000.0) > 0.99);
    }
}
