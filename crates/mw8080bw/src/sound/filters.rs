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
