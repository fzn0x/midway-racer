//! The watchdog: a 555 monostable that resets the board unless the
//! program retriggers it in time.

use crate::timing::CPU_CLOCK_HZ;

/// The monostable's period, 1.1 × R × C with R = 1 MΩ and C = 1 µF:
/// 1.1 s, in CPU cycles.
pub(crate) const PERIOD_CYCLES: u64 = CPU_CLOCK_HZ as u64 * 11 / 10;

/// The watchdog's state: when it fires unless retriggered first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Watchdog {
    deadline: u64,
}

impl Watchdog {
    /// Arms the watchdog at CPU cycle `now`.
    pub(crate) const fn new(now: u64) -> Self {
        Self {
            deadline: now + PERIOD_CYCLES,
        }
    }

    /// Retriggers the monostable at CPU cycle `now`.
    pub(crate) const fn kick(&mut self, now: u64) {
        *self = Self::new(now);
    }

    /// Returns whether the watchdog has fired by CPU cycle `now`.
    pub(crate) const fn has_fired(self, now: u64) -> bool {
        now >= self.deadline
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_after_1_1_seconds_without_a_kick() {
        assert_eq!(PERIOD_CYCLES, 2_196_480);
        let mut watchdog = Watchdog::new(100);
        assert!(!watchdog.has_fired(100 + PERIOD_CYCLES - 1));
        assert!(watchdog.has_fired(100 + PERIOD_CYCLES));

        watchdog.kick(1_000_000);
        assert!(!watchdog.has_fired(100 + PERIOD_CYCLES));
        assert!(watchdog.has_fired(1_000_000 + PERIOD_CYCLES));
    }
}
