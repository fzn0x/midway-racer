//! Emulation of Midway's 8080-based black-and-white arcade board, as used
//! by the racing games *Laguna Racer* (1977) and *280-ZZZAP* (1976).
//!
//! The hardware is modelled after MAME's `mw8080bw` driver and the original
//! schematics: an Intel 8080 at 1.9968 MHz, 8 KiB of RAM scanned out as a
//! 1-bit 256×224 bitmap, an MB14241 barrel shifter, a watchdog timer and a
//! discrete-logic sound board.

mod crc32;
pub mod game;
pub mod memory;
pub mod rom;
mod zip;

pub use game::{DipSetting, DipSwitch, Game, GameInfo, Orientation, OverlayBand, Rgb, RomChip};
pub use rom::{ChipCheck, ChipStatus, ProgramRom, RomError, RomFile, RomFiles, RomReport};
pub use zip::ZipError;
