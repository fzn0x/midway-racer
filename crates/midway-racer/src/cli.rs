//! Command-line arguments.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use mw8080bw::Game;

const CONTROLS: &str = "\
Controls:
  Left/Right, A/D   Steer
  Up, W             Accelerate
  Space, Shift      Shift gear
  C, 5              Insert coin
  1, Enter          Start
  P                 Pause
  F3                Reset
  Esc               Quit";

/// Play Laguna Racer and 280-ZZZAP on an emulated Midway 8080 board.
///
/// The game's ROMs are not included: point the emulator at MAME's
/// lagunar.zip, 280zzzap.zip or 280zzzapa.zip, at a directory holding them,
/// or at the individual dump files.
#[derive(Debug, Parser)]
#[command(version, about, after_help = CONTROLS)]
pub(crate) struct Args {
    /// ROM zip archives, directories or dump files to load the game from.
    #[arg(value_name = "PATH", default_value = "roms")]
    pub(crate) roms: Vec<PathBuf>,

    /// The game to run, when the files could hold several.
    #[arg(short, long, value_enum)]
    pub(crate) game: Option<GameName>,

    /// The DIP switch bank, such as 0x24, instead of the factory settings.
    #[arg(long, value_name = "BITS", value_parser = parse_byte)]
    pub(crate) dip_switches: Option<u8>,

    /// Window size, as a multiple of the original resolution.
    #[arg(short, long, default_value_t = 2, value_parser = clap::value_parser!(u8).range(1..=6))]
    pub(crate) scale: u8,

    /// Draw square pixels instead of the monitor's 4:3 picture.
    #[arg(long)]
    pub(crate) square_pixels: bool,

    /// Check the ROMs, print what was found and exit.
    #[arg(long)]
    pub(crate) verify: bool,
}

/// The games, by their MAME short names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum GameName {
    /// Laguna Racer.
    #[value(name = "lagunar")]
    LagunaRacer,
    /// 280-ZZZAP.
    #[value(name = "280zzzap")]
    Zzzap,
    /// 280-ZZZAP, second ROM set.
    #[value(name = "280zzzapa")]
    ZzzapSet2,
}

impl From<GameName> for Game {
    fn from(name: GameName) -> Self {
        match name {
            GameName::LagunaRacer => Self::LagunaRacer,
            GameName::Zzzap => Self::Zzzap,
            GameName::ZzzapSet2 => Self::ZzzapSet2,
        }
    }
}

/// Parses a byte written in decimal, or in hexadecimal with a `0x` prefix.
fn parse_byte(text: &str) -> Result<u8, String> {
    let parsed = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u8::from_str_radix(hex, 16),
        None => text.parse(),
    };
    parsed.map_err(|_| format!("`{text}` is not a byte, such as 36 or 0x24"))
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn arguments_are_well_formed() {
        Args::command().debug_assert();
    }

    #[test]
    fn game_names_match_mame() {
        for game in GameName::value_variants() {
            let name = game.to_possible_value().unwrap();
            assert_eq!(name.get_name(), Game::from(*game).info().short_name);
        }
    }

    #[test]
    fn bytes_parse_in_decimal_and_hex() {
        assert_eq!(parse_byte("36"), Ok(36));
        assert_eq!(parse_byte("0x24"), Ok(0x24));
        assert_eq!(parse_byte("0XFF"), Ok(0xff));
        assert!(parse_byte("0x100").is_err());
        assert!(parse_byte("fast").is_err());
    }

    #[test]
    fn defaults_look_for_a_roms_directory() {
        let args = Args::try_parse_from(["midway-racer"]).unwrap();
        assert_eq!(args.roms, [PathBuf::from("roms")]);
        assert_eq!(args.scale, 2);
    }
}
