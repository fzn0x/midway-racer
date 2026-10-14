//! Emulation of Midway's 8080-based black-and-white arcade board, as used
//! by the racing games *Laguna Racer* (1977) and *280-ZZZAP* (1976).
//!
//! The hardware is modelled after MAME's `mw8080bw` driver and the original
//! schematics: an Intel 8080 at 1.9968 MHz, 8 KiB of RAM scanned out as a
//! 1-bit 256×224 bitmap, an MB14241 barrel shifter, a watchdog timer and a
//! discrete-logic sound board.

pub mod controls;
mod crc32;
pub mod game;
mod machine;
pub mod mb14241;
pub mod memory;
pub mod rom;
pub mod timing;
pub mod video;
mod watchdog;
mod zip;

pub use controls::{Controls, DigitalPedal, DigitalSteering, Gear};
pub use game::{DipSetting, DipSwitch, Game, GameInfo, Orientation, OverlayBand, Rgb, RomChip};
pub use machine::{Machine, SoundLatches};
pub use mb14241::Mb14241;
pub use rom::{ChipCheck, ChipStatus, ProgramRom, RomError, RomFile, RomFiles, RomReport};
pub use video::{SCREEN_HEIGHT, SCREEN_WIDTH, Screen};
pub use zip::ZipError;
