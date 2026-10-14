//! The engine note.
//!
//! The pedal level written to the sound latch charges capacitor C18, whose
//! voltage op-amp `H5_4` turns into an "engine speed" drive. That drive sets
//! the frequency of three LM3900 triangle oscillators: the two slower ones
//! form the signal, and the fastest one sweeps the control pin of an
//! MC3340 attenuator, chopping the signal into the engine's buzz.

use super::SoundLatches;
use super::filters::{DcBlocker, smoothing};
use crate::game::Game;

/// Oscillator drive, the speed voltage above the LM3900's ~0.5 V input
/// level, with the pedal released: `H5_4` bottoms out at 0.7 V.
const DRIVE_IDLE: f64 = 0.2;
/// Oscillator drive with the pedal floored: `H5_4` tops out at 4.1 V.
const DRIVE_FULL: f64 = 3.6;
/// How far C18 must sit above its target before diode D4 opens the fast
/// discharge path through R52.
const DIODE_DROP: f64 = 0.6;

const R22: f64 = 470.0;
const R52: f64 = 10e3;
const R53: f64 = 47e3;
const R56: f64 = 270.0;
const R59: f64 = 560e3;
const R60: f64 = 10e3;
const C13: f64 = 10e-6;
const C19: f64 = 2.2e-6;

/// Frequencies of the two signal oscillators in low gear with the pedal
/// floored, from MAME's netlist (the schematics round them to 76 and
/// 315 Hz).
const SIGNAL_FREQUENCIES: [f64; 2] = [87.0, 343.0];

/// Bias of the MC3340's control pin, divided from 12 V by R23 and R25.
const CONTROL_BIAS: f64 = 12.0 * 3.3 / (10.0 + 3.3);
/// Peak swing of the modulating oscillator at the control pin.
const CONTROL_SWING: f64 = 0.8;
/// Control voltage above which the MC3340 starts attenuating.
const ATTENUATION_KNEE: f64 = 2.6;
/// Attenuation per volt above the knee.
const ATTENUATION_SLOPE: f64 = 36.0;
/// The MC3340's attenuation range, in decibels.
const MAX_ATTENUATION: f64 = 90.0;

/// The engine sound generator.
#[derive(Clone, Debug)]
pub(crate) struct Engine {
    sample_period: f64,
    frequencies: [f64; 3],
    charge: f64,
    discharge: f64,
    cut_off: f64,
    gear_charge: f64,
    gear_discharge: f64,
    mute_step: f64,
    /// Drive set by C18's voltage.
    drive: f64,
    /// Voltage across C19, the gear capacitor.
    gear_capacitor: f64,
    /// How far D1 has pulled the control pin towards silence, 0 to 1.
    mute: f64,
    phases: [f64; 3],
    /// Coupling capacitor C16, between the oscillators and the MC3340.
    coupling: DcBlocker,
}

impl Engine {
    pub(crate) fn new(game: Game, sample_rate: f64) -> Self {
        // Laguna Racer's smaller C18 makes the engine rev faster, and its
        // 100 kΩ R30 (110 kΩ on 280-ZZZAP) raises the modulator's pitch.
        let (c18, modulator_frequency) = match game {
            Game::LagunaRacer => (22e-6, 511.0),
            Game::Zzzap | Game::ZzzapSet2 => (47e-6, 465.0),
        };
        let [low, middle] = SIGNAL_FREQUENCIES;
        Self {
            sample_period: sample_rate.recip(),
            frequencies: [low, middle, modulator_frequency],
            charge: smoothing(sample_rate, R53 * c18),
            discharge: smoothing(sample_rate, R52 * R53 / (R52 + R53) * c18),
            cut_off: smoothing(sample_rate, R56 * c18),
            gear_charge: smoothing(sample_rate, R59 * C19),
            gear_discharge: smoothing(sample_rate, R60 * C19),
            mute_step: smoothing(sample_rate, R22 * C13),
            drive: 0.0,
            gear_capacitor: 0.0,
            mute: 0.0,
            phases: [0.0, 0.25, 0.5],
            coupling: DcBlocker::new(sample_rate, 5.0),
        }
    }

    /// Produces the next sample, nominally within ±1.
    pub(crate) fn sample(&mut self, latches: SoundLatches) -> f64 {
        let drive = self.update_drive(latches);

        let mut triangles = [0.0; 3];
        for ((phase, frequency), triangle) in self
            .phases
            .iter_mut()
            .zip(self.frequencies)
            .zip(&mut triangles)
        {
            *phase = (*phase + frequency * drive / DRIVE_FULL * self.sample_period).fract();
            *triangle = 4.0 * (*phase - 0.5).abs() - 1.0;
        }

        // Diodes D2 and D3 combine the two slow oscillators into a ragged
        // wave, which C16 couples to the MC3340's input.
        let signal = self.coupling.process(triangles[0].max(triangles[1]));

        // The fast oscillator sweeps the control pin, letting the signal
        // through in needle-like bursts. ENGINE SOUND OFF pulls the pin
        // high through D1, muting the engine.
        let control = CONTROL_BIAS + CONTROL_SWING * triangles[2];
        let attenuation =
            (ATTENUATION_SLOPE * (control - ATTENUATION_KNEE)).clamp(0.0, MAX_ATTENUATION);
        let muted = if latches.engine_off() { 1.0 } else { 0.0 };
        self.mute += (muted - self.mute) * self.mute_step;
        signal * decibels_to_gain(-attenuation) * (1.0 - self.mute)
    }

    /// Advances C18 and C19 by one sample and returns the oscillators'
    /// drive.
    fn update_drive(&mut self, latches: SoundLatches) -> f64 {
        // C18 charges towards the pedal level through R53 and discharges
        // faster once D4 conducts, which gives the engine its inertia.
        // ENGINE SOUND OFF drains it through R56 almost at once.
        let (target, step) = if latches.engine_off() {
            (0.0, self.cut_off)
        } else {
            let target = DRIVE_IDLE + (DRIVE_FULL - DRIVE_IDLE) * f64::from(latches.pedal()) / 15.0;
            let step = if self.drive - target > DIODE_DROP {
                self.discharge
            } else {
                self.charge
            };
            (target, step)
        };
        self.drive += (target - self.drive) * step;

        // In high gear C19 joins H5_4's feedback path through R59: it
        // first halves the drive, then lets it recover as it charges, so
        // the note drops on shifting up and changes more slowly. In low
        // gear R60 keeps C19 discharged.
        if latches.high_gear() {
            let drive = f64::midpoint(self.drive, self.gear_capacitor);
            self.gear_capacitor += (drive - self.gear_capacitor) * self.gear_charge;
            drive
        } else {
            if latches.low_gear() {
                self.gear_capacitor -= self.gear_capacitor * self.gear_discharge;
            }
            self.drive
        }
    }

    #[cfg(test)]
    fn drive(&self) -> f64 {
        self.drive
    }
}

fn decibels_to_gain(decibels: f64) -> f64 {
    10f64.powf(decibels / 20.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    fn latches(engine: u8, effects: u8) -> SoundLatches {
        SoundLatches { engine, effects }
    }

    fn engine(game: Game) -> Engine {
        Engine::new(game, f64::from(RATE))
    }

    /// Runs the engine for `seconds` and returns the RMS of its output.
    fn run(engine: &mut Engine, latches: SoundLatches, seconds: f64) -> f64 {
        let mut energy = 0.0;
        let mut count = 0.0;
        while count < seconds * f64::from(RATE) {
            energy += engine.sample(latches).powi(2);
            count += 1.0;
        }
        (energy / count).sqrt()
    }

    /// Counts the samples until the drive crosses `threshold`.
    fn samples_to_cross(engine: &mut Engine, latches: SoundLatches, threshold: f64) -> u32 {
        let rising = engine.drive() < threshold;
        (1..=10 * RATE)
            .find(|_| {
                engine.sample(latches);
                (engine.drive() > threshold) == rising
            })
            .expect("the drive never crossed the threshold")
    }

    #[test]
    fn drive_settles_at_the_pedal_level() {
        let mut engine = engine(Game::LagunaRacer);
        run(&mut engine, latches(0x2f, 0), 10.0);
        assert!((engine.drive() - DRIVE_FULL).abs() < 1e-3);

        run(&mut engine, latches(0x20, 0), 10.0);
        assert!((engine.drive() - DRIVE_IDLE).abs() < 1e-3);
    }

    #[test]
    fn engine_slows_down_faster_than_it_revs_up() {
        let mut engine = engine(Game::LagunaRacer);
        let rev_up = samples_to_cross(&mut engine, latches(0x2f, 0), 3.0);
        run(&mut engine, latches(0x2f, 0), 10.0);
        let slow_down = samples_to_cross(&mut engine, latches(0x20, 0), 0.8);
        assert!(slow_down * 2 < rev_up, "{slow_down} vs {rev_up}");
    }

    #[test]
    fn laguna_racer_revs_faster_than_280_zzzap() {
        let mut laguna = engine(Game::LagunaRacer);
        let mut zzzap = engine(Game::Zzzap);
        run(&mut laguna, latches(0x2f, 0), 0.5);
        run(&mut zzzap, latches(0x2f, 0), 0.5);
        assert!(laguna.drive() > zzzap.drive() * 1.5);
    }

    #[test]
    fn shifting_up_drops_the_note_until_c19_charges() {
        let mut engine = engine(Game::LagunaRacer);
        run(&mut engine, latches(0x2f, 0), 10.0);

        let shifted = latches(0x1f, 0);
        engine.sample(shifted);
        let drive_after_shift = f64::midpoint(engine.drive(), engine.gear_capacitor);
        assert!((drive_after_shift / DRIVE_FULL - 0.5).abs() < 0.01);

        run(&mut engine, shifted, 15.0);
        assert!((engine.gear_capacitor - DRIVE_FULL).abs() < 0.01);
    }

    #[test]
    fn engine_sound_off_silences_the_engine() {
        let mut engine = engine(Game::LagunaRacer);
        let running = run(&mut engine, latches(0x2f, 0), 4.0);
        assert!(running > 0.05, "{running}");

        run(&mut engine, latches(0x2f, 0x02), 0.1);
        let silenced = run(&mut engine, latches(0x2f, 0x02), 1.0);
        assert!(silenced < 1e-3, "{silenced}");
    }

    #[test]
    fn output_stays_bounded() {
        let mut engine = engine(Game::Zzzap);
        for pedal in [0x2f, 0x1f, 0x20, 0x28] {
            for _ in 0..RATE {
                let sample = engine.sample(latches(pedal, 0));
                assert!(sample.is_finite() && sample.abs() <= 2.0);
            }
        }
    }
}
