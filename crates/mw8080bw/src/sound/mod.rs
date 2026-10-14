//! The 280-ZZZAP and Laguna Racer sound board.
//!
//! The board is entirely analogue. Rather than simulating it component by
//! component, as MAME's netlist does, each stage is modelled by its
//! behaviour, with RC time constants, oscillator frequencies and filter
//! characteristics taken from the schematics and MAME's netlist notes.

mod engine;
mod filters;

use engine::Engine;
use filters::DcBlocker;

use crate::game::Game;

/// Output of the engine stage in the final mix.
const ENGINE_LEVEL: f64 = 0.35;

/// The values last written to the sound board's two latches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SoundLatches {
    /// Port 2: accelerator level in bits 0–3, high gear in bit 4 and low
    /// gear in bit 5.
    pub engine: u8,
    /// Port 5: boom in bit 0, engine off in bit 1, tyre skid in bit 2,
    /// post-crash noise in bit 3 and the coin counter in bit 5.
    pub effects: u8,
}

impl SoundLatches {
    /// Returns the accelerator level driving the engine note, 0 to 15.
    pub const fn pedal(self) -> u8 {
        self.engine & 0x0f
    }

    /// Returns whether the engine note is set for high gear.
    pub const fn high_gear(self) -> bool {
        self.engine & 0x10 != 0
    }

    /// Returns whether the engine note is set for low gear.
    pub const fn low_gear(self) -> bool {
        self.engine & 0x20 != 0
    }

    /// Returns whether the crash boom is sounding.
    pub const fn boom(self) -> bool {
        self.effects & 0x01 != 0
    }

    /// Returns whether the engine is silenced, after a crash or between
    /// games.
    pub const fn engine_off(self) -> bool {
        self.effects & 0x02 != 0
    }

    /// Returns whether the tyres are squealing.
    pub const fn skid(self) -> bool {
        self.effects & 0x04 != 0
    }

    /// Returns whether the screech that follows a crash is sounding.
    pub const fn post_crash(self) -> bool {
        self.effects & 0x08 != 0
    }
}

/// A synthesizer for the sound board, driven by its latches.
#[derive(Clone, Debug)]
pub struct SoundBoard {
    sample_rate: u32,
    latches: SoundLatches,
    engine: Engine,
    /// The coupling capacitors of the output amplifier, C24 with R73.
    output: DcBlocker,
}

impl SoundBoard {
    /// Creates the sound board of `game`, producing `sample_rate` samples
    /// per second.
    ///
    /// # Panics
    ///
    /// Panics if `sample_rate` is zero.
    pub fn new(game: Game, sample_rate: u32) -> Self {
        assert!(sample_rate > 0, "the sample rate must be positive");
        let rate = f64::from(sample_rate);
        Self {
            sample_rate,
            latches: SoundLatches::default(),
            engine: Engine::new(game, rate),
            output: DcBlocker::new(rate, 16.0),
        }
    }

    /// Returns the number of samples produced per second.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Returns the latch values the board is playing.
    pub fn latches(&self) -> SoundLatches {
        self.latches
    }

    /// Changes the latch values the board is playing.
    pub fn set_latches(&mut self, latches: SoundLatches) {
        self.latches = latches;
    }

    /// Fills `out` with mono samples in the range -1 to 1.
    pub fn render(&mut self, out: &mut [f32]) {
        for sample in out {
            let mix = ENGINE_LEVEL * self.engine.sample(self.latches);
            *sample = self.output.process(mix).clamp(-1.0, 1.0) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latch_bits_decode() {
        let latches = SoundLatches {
            engine: 0x3a,
            effects: 0x0f,
        };
        assert_eq!(latches.pedal(), 0x0a);
        assert!(latches.high_gear() && latches.low_gear());
        assert!(latches.boom() && latches.engine_off() && latches.skid() && latches.post_crash());
        assert!(!SoundLatches::default().engine_off());
    }

    #[test]
    fn renders_a_bounded_engine_note() {
        let mut board = SoundBoard::new(Game::LagunaRacer, 44_100);
        board.set_latches(SoundLatches {
            engine: 0x2f,
            effects: 0,
        });
        let mut samples = vec![0.0; 44_100];
        board.render(&mut samples);
        assert!(samples.iter().all(|sample| sample.abs() <= 1.0));
        assert!(samples.iter().any(|sample| sample.abs() > 0.01));
        assert_eq!(board.sample_rate(), 44_100);
        assert_eq!(board.latches().pedal(), 15);
    }

    #[test]
    #[should_panic(expected = "sample rate")]
    fn rejects_a_zero_sample_rate() {
        SoundBoard::new(Game::LagunaRacer, 0);
    }
}
