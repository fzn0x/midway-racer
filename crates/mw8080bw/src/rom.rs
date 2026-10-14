//! Finding, verifying and assembling a game's program ROM from dump files.

use std::fmt;

use crate::crc32::crc32;
use crate::game::{Game, RomChip};
use crate::memory::ROM_SIZE;
use crate::zip::{self, ZipError};

/// A named file that may hold a ROM chip dump.
#[derive(Clone, PartialEq, Eq)]
pub struct RomFile {
    /// The file name, possibly with a directory path.
    pub name: String,
    /// The file contents.
    pub data: Vec<u8>,
}

impl fmt::Debug for RomFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RomFile")
            .field("name", &self.name)
            .field("size", &self.data.len())
            .finish()
    }
}

/// A collection of candidate ROM dump files, such as the contents of a
/// MAME ROM archive or of a directory.
///
/// Chips are recognised by checksum whatever their file names, and by
/// MAME's file names otherwise. A single file holding a whole ROM set
/// back to back is recognised too.
#[derive(Clone, Debug, Default)]
pub struct RomFiles {
    files: Vec<RomFile>,
}

impl RomFiles {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file to the collection.
    pub fn add(&mut self, name: impl Into<String>, data: Vec<u8>) {
        self.files.push(RomFile {
            name: name.into(),
            data,
        });
    }

    /// Adds a file to the collection, or the files inside it when it is a
    /// zip archive such as MAME's `lagunar.zip`.
    ///
    /// # Errors
    ///
    /// Returns an error when a zip archive cannot be read.
    pub fn add_file(&mut self, name: impl Into<String>, data: Vec<u8>) -> Result<(), ZipError> {
        let name = name.into();
        if zip::is_zip(&data) {
            for (entry, contents) in zip::extract(&data)? {
                self.add(format!("{name}/{entry}"), contents);
            }
        } else {
            self.add(name, data);
        }
        Ok(())
    }

    /// Returns the files in the collection.
    pub fn files(&self) -> &[RomFile] {
        &self.files
    }

    /// Checks every chip of `game` against the collection.
    pub fn check(&self, game: Game) -> RomReport {
        let chips = self
            .locate_all(game.info().roms)
            .into_iter()
            .map(|(check, _)| check)
            .collect();
        RomReport { game, chips }
    }

    /// Returns the supported game the collection matches best, if it
    /// matches any at all.
    pub fn identify(&self) -> Option<Game> {
        Game::ALL
            .into_iter()
            .map(|game| (game, self.check(game).score()))
            .filter(|&(_, score)| score > 0)
            // `max_by_key` keeps the last maximum; reverse to prefer the
            // earlier games on ties.
            .rev()
            .max_by_key(|&(_, score)| score)
            .map(|(game, _)| game)
    }

    /// Assembles the program ROM of `game` from the collection.
    ///
    /// Chips whose file has the expected name and size but an unexpected
    /// checksum are accepted, as MAME does, because they may be legitimate
    /// alternative dumps; [`RomFiles::check`] reports them.
    ///
    /// # Errors
    ///
    /// Returns [`RomError::Incomplete`] when a chip is missing or has the
    /// wrong size.
    pub fn program_rom(&self, game: Game) -> Result<ProgramRom, RomError> {
        self.assemble(game, game.info().roms)
    }

    fn assemble(&self, game: Game, chips: &[RomChip]) -> Result<ProgramRom, RomError> {
        let located = self.locate_all(chips);
        let mut image = Box::new([0; ROM_SIZE]);
        let mut complete = true;
        for (check, data) in &located {
            let chip = check.chip;
            match data {
                Some(data) => image[chip.offset..chip.offset + chip.size].copy_from_slice(data),
                None => complete = false,
            }
        }

        if complete {
            Ok(ProgramRom { game, image })
        } else {
            let chips = located.into_iter().map(|(check, _)| check).collect();
            Err(RomError::Incomplete(RomReport { game, chips }))
        }
    }

    fn locate_all(&self, chips: &[RomChip]) -> Vec<(ChipCheck, Option<&[u8]>)> {
        let set_size = chips.iter().map(|chip| chip.size).sum();
        chips
            .iter()
            .map(|&chip| {
                let (status, data) = self.locate(&chip, set_size);
                (ChipCheck { chip, status }, data)
            })
            .collect()
    }

    /// Finds the data of `chip`, first by checksum in files of the chip's
    /// size or of the whole set's size, then by name.
    fn locate(&self, chip: &RomChip, set_size: usize) -> (ChipStatus, Option<&[u8]>) {
        for file in &self.files {
            let data = if file.data.len() == chip.size {
                &file.data[..]
            } else if file.data.len() == set_size {
                &file.data[chip.offset..chip.offset + chip.size]
            } else {
                continue;
            };
            if crc32(data) == chip.crc32 {
                let file = file.name.clone();
                return (ChipStatus::Verified { file }, Some(data));
            }
        }

        let named = self
            .files
            .iter()
            .find(|file| base_name(&file.name).eq_ignore_ascii_case(chip.name));
        match named {
            Some(file) if file.data.len() == chip.size => (
                ChipStatus::ChecksumMismatch {
                    file: file.name.clone(),
                    crc32: crc32(&file.data),
                },
                Some(&file.data),
            ),
            Some(file) => (
                ChipStatus::WrongSize {
                    file: file.name.clone(),
                    size: file.data.len(),
                },
                None,
            ),
            None => (ChipStatus::Missing, None),
        }
    }
}

/// Strips any directory components from a path.
fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// How each chip of a game's ROM set was matched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RomReport {
    /// The game whose ROM set was checked.
    pub game: Game,
    /// One entry per chip, in address order.
    pub chips: Vec<ChipCheck>,
}

impl RomReport {
    /// Returns whether every chip was found with a matching checksum.
    pub fn is_verified(&self) -> bool {
        self.chips
            .iter()
            .all(|check| matches!(check.status, ChipStatus::Verified { .. }))
    }

    /// Returns whether the game can run: every chip was found with the
    /// right size, even if some checksums differ.
    pub fn is_usable(&self) -> bool {
        self.chips.iter().all(|check| check.status.is_usable())
    }

    /// Ranks how well the files match: verified chips count double.
    fn score(&self) -> usize {
        self.chips
            .iter()
            .map(|check| match check.status {
                ChipStatus::Verified { .. } => 2,
                ChipStatus::ChecksumMismatch { .. } | ChipStatus::WrongSize { .. } => 1,
                ChipStatus::Missing => 0,
            })
            .sum()
    }
}

impl fmt::Display for RomReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ROM set", self.game.info().title)?;
        for check in &self.chips {
            write!(f, "\n  {}: {}", check.chip.name, check.status)?;
        }
        Ok(())
    }
}

/// The result of looking for one ROM chip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChipCheck {
    /// The chip that was looked for.
    pub chip: RomChip,
    /// What was found.
    pub status: ChipStatus,
}

/// What was found for a ROM chip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChipStatus {
    /// A file with the expected checksum was found.
    Verified {
        /// The name of the file.
        file: String,
    },
    /// A file with the chip's name and size was found, but its checksum
    /// differs: a bad or alternative dump.
    ChecksumMismatch {
        /// The name of the file.
        file: String,
        /// The file's actual CRC-32.
        crc32: u32,
    },
    /// A file with the chip's name was found, but its size is wrong.
    WrongSize {
        /// The name of the file.
        file: String,
        /// The file's actual size in bytes.
        size: usize,
    },
    /// No file matched.
    Missing,
}

impl ChipStatus {
    /// Returns whether the chip's data was found with the right size.
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Verified { .. } | Self::ChecksumMismatch { .. })
    }
}

impl fmt::Display for ChipStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Verified { file } => write!(f, "found in {file}"),
            Self::ChecksumMismatch { file, crc32 } => {
                write!(f, "{file} has an unexpected checksum ({crc32:08x})")
            }
            Self::WrongSize { file, size } => write!(f, "{file} has the wrong size ({size} bytes)"),
            Self::Missing => f.write_str("missing"),
        }
    }
}

/// A game's assembled program ROM image.
#[derive(Clone, PartialEq, Eq)]
pub struct ProgramRom {
    game: Game,
    image: Box<[u8; ROM_SIZE]>,
}

impl ProgramRom {
    /// Wraps a raw image of the program ROM area, padding it with zeroes,
    /// without verifying it. Useful for tests and homebrew programs.
    ///
    /// # Errors
    ///
    /// Returns [`RomError::TooLarge`] when `image` exceeds the ROM area.
    pub fn from_image(game: Game, image: &[u8]) -> Result<Self, RomError> {
        if image.len() > ROM_SIZE {
            return Err(RomError::TooLarge { size: image.len() });
        }
        let mut padded = Box::new([0; ROM_SIZE]);
        padded[..image.len()].copy_from_slice(image);
        Ok(Self {
            game,
            image: padded,
        })
    }

    /// Returns the game the ROM belongs to.
    pub fn game(&self) -> Game {
        self.game
    }

    /// Returns the image of the whole program ROM area.
    pub fn image(&self) -> &[u8; ROM_SIZE] {
        &self.image
    }
}

impl fmt::Debug for ProgramRom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgramRom")
            .field("game", &self.game)
            .finish_non_exhaustive()
    }
}

/// Why a program ROM could not be built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RomError {
    /// Some chips are missing or have the wrong size.
    Incomplete(RomReport),
    /// A raw image is larger than the ROM area.
    TooLarge {
        /// The size of the image in bytes.
        size: usize,
    },
}

impl fmt::Display for RomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete(report) => write!(f, "incomplete {report}"),
            Self::TooLarge { size } => write!(
                f,
                "a {size}-byte image does not fit the {ROM_SIZE}-byte ROM area"
            ),
        }
    }
}

impl std::error::Error for RomError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two 4-byte chips and their contents.
    fn test_set() -> ([RomChip; 2], [Vec<u8>; 2]) {
        let data = [vec![1, 2, 3, 4], vec![5, 6, 7, 8]];
        let chip = |name, offset, data: &[u8]| RomChip {
            name,
            offset,
            size: data.len(),
            crc32: crc32(data),
        };
        (
            [chip("a.bin", 0, &data[0]), chip("b.bin", 4, &data[1])],
            data,
        )
    }

    #[test]
    fn chips_are_found_by_checksum_whatever_their_names() {
        let (chips, data) = test_set();
        let mut files = RomFiles::new();
        files.add("second", data[1].clone());
        files.add("first", data[0].clone());

        let rom = files.assemble(Game::LagunaRacer, &chips).unwrap();
        assert_eq!(rom.image()[..8], [1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(rom.image()[8..].iter().all(|&byte| byte == 0));
    }

    #[test]
    fn a_combined_image_provides_every_chip() {
        let (chips, _) = test_set();
        let mut files = RomFiles::new();
        files.add("combined.bin", vec![1, 2, 3, 4, 5, 6, 7, 8]);

        let rom = files.assemble(Game::LagunaRacer, &chips).unwrap();
        assert_eq!(rom.image()[..8], [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn named_chips_with_bad_checksums_are_usable_but_reported() {
        let (chips, data) = test_set();
        let mut files = RomFiles::new();
        files.add("roms/A.BIN", vec![9, 9, 9, 9]);
        files.add("b.bin", data[1].clone());

        let located = files.locate_all(&chips);
        assert_eq!(
            located[0].0.status,
            ChipStatus::ChecksumMismatch {
                file: "roms/A.BIN".into(),
                crc32: crc32(&[9, 9, 9, 9]),
            }
        );
        assert!(located[0].0.status.is_usable());
        let rom = files.assemble(Game::LagunaRacer, &chips).unwrap();
        assert_eq!(rom.image()[..4], [9, 9, 9, 9]);
    }

    #[test]
    fn missing_or_truncated_chips_are_errors() {
        let (chips, _) = test_set();
        let mut files = RomFiles::new();
        files.add("a.bin", vec![1, 2]);

        let Err(RomError::Incomplete(report)) = files.assemble(Game::LagunaRacer, &chips) else {
            panic!("an incomplete set must not assemble");
        };
        assert_eq!(
            report.chips[0].status,
            ChipStatus::WrongSize {
                file: "a.bin".into(),
                size: 2,
            }
        );
        assert_eq!(report.chips[1].status, ChipStatus::Missing);
        assert!(!report.is_usable());
        assert_eq!(
            report.to_string(),
            "Laguna Racer ROM set\n  a.bin: a.bin has the wrong size (2 bytes)\n  b.bin: missing"
        );
    }

    #[test]
    fn identification_needs_at_least_one_match() {
        assert_eq!(RomFiles::new().identify(), None);

        let mut files = RomFiles::new();
        files.add("unrelated.bin", vec![0; 0x800]);
        assert_eq!(files.identify(), None);

        let mut files = RomFiles::new();
        files.add("lagunar.g", vec![0; 0x800]);
        assert_eq!(files.identify(), Some(Game::LagunaRacer));
        assert!(files.check(Game::LagunaRacer).chips[1].status.is_usable());
    }

    #[test]
    fn real_sets_report_every_chip() {
        let report = RomFiles::new().check(Game::Zzzap);
        assert_eq!(report.chips.len(), 6);
        assert!(!report.is_verified());
    }

    #[test]
    fn archives_are_expanded_and_loose_files_kept() {
        let mut files = RomFiles::new();
        let archive = include_bytes!("../tests/fixtures/sample.zip").to_vec();
        files.add_file("sample.zip", archive).unwrap();
        files.add_file("loose.bin", vec![1, 2, 3]).unwrap();

        let names: Vec<_> = files
            .files()
            .iter()
            .map(|file| file.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "sample.zip/readme.txt",
                "sample.zip/roms/pattern.bin",
                "loose.bin"
            ]
        );
        assert_eq!(
            files.add_file("broken.zip", b"PK\x03\x04".to_vec()),
            Err(ZipError::NotAnArchive)
        );
    }

    #[test]
    fn raw_images_are_padded_and_bounded() {
        let rom = ProgramRom::from_image(Game::Zzzap, &[0xc3, 0x00, 0x00]).unwrap();
        assert_eq!(rom.game(), Game::Zzzap);
        assert_eq!(rom.image()[..4], [0xc3, 0x00, 0x00, 0x00]);
        assert_eq!(
            ProgramRom::from_image(Game::Zzzap, &[0; ROM_SIZE + 1]),
            Err(RomError::TooLarge { size: ROM_SIZE + 1 })
        );
    }
}
