//! Boots the real games, for those who own their ROMs.
//!
//! The ROMs are not part of this repository. Point `MIDWAY_RACER_ROMS` at a
//! zip archive or a directory holding MAME's sets, then run:
//!
//! ```sh
//! MIDWAY_RACER_ROMS=~/roms cargo test --release -p mw8080bw --test real_roms -- --ignored --nocapture
//! ```
//!
//! Each game runs its attract mode, takes a coin and is driven for half a
//! minute. The watchdog must never reset the board, which it would if the
//! program crashed, and the game must draw and make sound.

use std::path::Path;
use std::{env, fs};

use mw8080bw::{Controls, Game, Gear, Machine, RomFiles, Screen};

const ATTRACT_FRAMES: u32 = 600;
const DRIVING_FRAMES: u32 = 1_800;

fn rom_files() -> RomFiles {
    let path = env::var_os("MIDWAY_RACER_ROMS")
        .expect("set MIDWAY_RACER_ROMS to a zip archive or directory holding the ROMs");
    let mut files = RomFiles::new();
    add(&mut files, Path::new(&path), 2);
    files
}

fn add(files: &mut RomFiles, path: &Path, depth: usize) {
    if path.is_dir() {
        if depth > 0 {
            for entry in fs::read_dir(path).expect("the ROM directory is readable") {
                add(
                    files,
                    &entry.expect("the ROM directory is readable").path(),
                    depth - 1,
                );
            }
        }
    } else {
        let data = fs::read(path).expect("the ROM file is readable");
        files
            .add_file(path.display().to_string(), data)
            .expect("the ROM archive is readable");
    }
}

fn lit_pixels(screen: &Screen) -> usize {
    screen.pixels().iter().filter(|&&pixel| pixel != 0).count()
}

fn loudest(samples: &[f32]) -> f32 {
    samples
        .iter()
        .fold(0.0, |peak, sample| peak.max(sample.abs()))
}

fn play(game: Game) {
    let title = game.info().title;
    let files = rom_files();
    let report = files.check(game);
    if !report.is_usable() {
        println!("skipping {title}: {report}");
        return;
    }
    let mut machine = Machine::new(files.program_rom(game).expect("the ROM set is usable"));
    machine.enable_audio(48_000);

    let mut most_lit = 0;
    for _ in 0..ATTRACT_FRAMES {
        machine.run_frame();
        most_lit = most_lit.max(lit_pixels(machine.screen()));
    }
    assert_eq!(
        machine.watchdog_resets(),
        0,
        "{title} crashed in attract mode"
    );
    assert!(most_lit > 0, "{title} drew nothing in attract mode");

    machine.insert_coin();
    let mut peak: f32 = 0.0;
    for frame in 0..DRIVING_FRAMES {
        // Weave gently, floor the pedal and shift up after ten seconds.
        let seconds = f64::from(frame) / mw8080bw::timing::FRAME_RATE_HZ;
        machine.set_controls(Controls {
            steering: (seconds.sin() * 0.4) as f32,
            accelerator: 1.0,
            gear: if seconds < 10.0 {
                Gear::Low
            } else {
                Gear::High
            },
            start: frame < 30,
        });
        machine.run_frame();
        peak = peak.max(loudest(machine.audio_samples()));
    }
    assert_eq!(
        machine.watchdog_resets(),
        0,
        "{title} crashed while driving"
    );
    assert!(peak > 0.01, "{title} made no sound");

    println!(
        "{title}: {} frames without a watchdog reset, up to {most_lit} lit pixels in attract mode, sound peaking at {peak:.2}",
        machine.frame()
    );
}

#[test]
#[ignore = "needs the game's ROMs, see the module documentation"]
fn laguna_racer_runs() {
    play(Game::LagunaRacer);
}

#[test]
#[ignore = "needs the game's ROMs, see the module documentation"]
fn zzzap_runs() {
    play(Game::Zzzap);
}

#[test]
#[ignore = "needs the game's ROMs, see the module documentation"]
fn zzzap_set_2_runs() {
    play(Game::ZzzapSet2);
}
