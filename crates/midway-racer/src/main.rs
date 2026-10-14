//! Plays Laguna Racer and 280-ZZZAP on an emulated Midway 8080 board.

mod app;
mod cli;
mod display;
mod input;
mod roms;

use anyhow::Result;
use clap::Parser;
use mw8080bw::{Game, Machine};

fn main() -> Result<()> {
    let args = cli::Args::parse();
    let (rom, report) = roms::load(&args.roms, args.game.map(Game::from))?;

    if args.verify {
        println!("{report}");
        if report.is_verified() {
            println!("Every chip matches a known good dump.");
        } else {
            println!("Playable, but some chips do not match known good dumps.");
        }
        return Ok(());
    }
    if !report.is_verified() {
        eprintln!("warning: some chips do not match known good dumps, the game may misbehave");
        eprintln!("{report}");
    }

    let mut machine = Machine::new(rom);
    if let Some(dip_switches) = args.dip_switches {
        machine.set_dip_switches(dip_switches);
    }
    let info = machine.game().info();
    println!(
        "{} ({}, {}): insert a coin with C. Run with --help to list the controls.",
        info.title, info.manufacturer, info.year
    );

    app::App::new(machine, usize::from(args.scale), args.square_pixels)?.run()
}
