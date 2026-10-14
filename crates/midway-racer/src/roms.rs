//! Finding the game's ROMs on disk.

use std::fs;
use std::path::Path;

use anyhow::{Result, bail};
use mw8080bw::{Game, ProgramRom, RomFiles, RomReport};

/// How deep to look inside directories: `roms/lagunar/lagunar.h` is found
/// from `roms`.
const MAX_DEPTH: usize = 2;
/// Files larger than this cannot be ROM sets of these games, such as the
/// disk images MAME keeps beside its ROMs, and are skipped unread.
const MAX_FILE_SIZE: u64 = 4 * 1024 * 1024;
/// Skipped files reported one by one before summarising the rest.
const MAX_REPORTED_SKIPS: usize = 5;

/// Loads the program ROM of `game`, or of whichever game the files at
/// `paths` hold, and reports how every chip was matched.
///
/// Files that cannot be read, such as damaged archives, are skipped with a
/// warning so that one bad file among many does not stop the game.
pub(crate) fn load(
    paths: &[impl AsRef<Path>],
    game: Option<Game>,
) -> Result<(ProgramRom, RomReport)> {
    let mut files = RomFiles::new();
    let mut skipped = Vec::new();
    for path in paths {
        let path = path.as_ref();
        if !path.exists() {
            bail!("{} does not exist\n{}", path.display(), expected_sets());
        }
        add_path(&mut files, path, MAX_DEPTH, &mut skipped);
    }
    report_skipped(&skipped);

    if files.files().is_empty() {
        bail!(
            "no ROM files found in {}\n{}",
            describe(paths),
            expected_sets()
        );
    }
    let Some(game) = game.or_else(|| files.identify()) else {
        bail!(
            "the files in {} hold none of the supported games\n{}",
            describe(paths),
            expected_sets()
        );
    };
    let report = files.check(game);
    let rom = files.program_rom(game)?;
    Ok((rom, report))
}

/// Adds the files under `path`, noting the ones skipped and why.
fn add_path(files: &mut RomFiles, path: &Path, depth: usize, skipped: &mut Vec<String>) {
    if path.is_dir() {
        if depth == 0 {
            return;
        }
        match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    add_path(files, &entry.path(), depth - 1, skipped);
                }
            }
            Err(error) => skipped.push(format!("{}: {error}", path.display())),
        }
    } else if let Err(reason) = add_file(files, path) {
        skipped.push(format!("{}: {reason}", path.display()));
    }
}

fn add_file(files: &mut RomFiles, path: &Path) -> Result<(), String> {
    let size = fs::metadata(path).map_err(|error| error.to_string())?.len();
    if size > MAX_FILE_SIZE {
        return Err("too large to be a ROM set".to_owned());
    }
    let data = fs::read(path).map_err(|error| error.to_string())?;
    files
        .add_file(path.display().to_string(), data)
        .map_err(|error| error.to_string())
}

fn report_skipped(skipped: &[String]) {
    for reason in skipped.iter().take(MAX_REPORTED_SKIPS) {
        eprintln!("warning: skipped {reason}");
    }
    if skipped.len() > MAX_REPORTED_SKIPS {
        eprintln!(
            "warning: skipped {} more files",
            skipped.len() - MAX_REPORTED_SKIPS
        );
    }
}

fn describe(paths: &[impl AsRef<Path>]) -> String {
    let names: Vec<_> = paths
        .iter()
        .map(|path| path.as_ref().display().to_string())
        .collect();
    names.join(", ")
}

/// Explains which files each game needs.
fn expected_sets() -> String {
    let sets: Vec<_> = Game::ALL
        .iter()
        .map(|game| {
            let info = game.info();
            let chips: Vec<_> = info.roms.iter().map(|chip| chip.name).collect();
            format!(
                "  {} ({}.zip): {}",
                info.title,
                info.short_name,
                chips.join(", ")
            )
        })
        .collect();
    format!(
        "Pass the path of the game's ROMs, or put them in a roms directory.\n\
         Expected one of these ROM sets, as in MAME:\n{}",
        sets.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A fresh directory under the system's temporary directory.
    fn scratch_directory(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("midway-racer-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn missing_directories_explain_what_is_needed() {
        let error = load(&["no/such/directory"], None).unwrap_err().to_string();
        assert!(
            error.contains("no/such/directory does not exist\n"),
            "{error}"
        );
        assert!(
            error.contains("Laguna Racer (lagunar.zip): lagunar.h"),
            "{error}"
        );
    }

    #[test]
    fn unrelated_files_are_not_mistaken_for_a_game() {
        let directory = scratch_directory("unrelated");
        fs::write(directory.join("notes.txt"), b"not a ROM").unwrap();

        let error = load(&[&directory], None).unwrap_err().to_string();
        assert!(
            error.contains("hold none of the supported games"),
            "{error}"
        );
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn damaged_and_oversized_files_do_not_stop_the_game() {
        let directory = scratch_directory("damaged");
        for chip in Game::LagunaRacer.info().roms {
            fs::write(directory.join(chip.name), vec![0; chip.size]).unwrap();
        }
        fs::write(directory.join("broken.zip"), b"PK\x03\x04 not really").unwrap();
        let huge = fs::File::create(directory.join("disk.chd")).unwrap();
        huge.set_len(MAX_FILE_SIZE + 1).unwrap();
        drop(huge);

        let (rom, report) = load(&[&directory], None).unwrap();
        assert_eq!(rom.game(), Game::LagunaRacer);
        assert!(report.is_usable());

        let mut skipped = Vec::new();
        add_path(&mut RomFiles::new(), &directory, MAX_DEPTH, &mut skipped);
        skipped.sort();
        assert_eq!(skipped.len(), 2, "{skipped:?}");
        assert!(skipped[0].ends_with("not a zip archive"), "{skipped:?}");
        assert!(
            skipped[1].ends_with("too large to be a ROM set"),
            "{skipped:?}"
        );
        fs::remove_dir_all(&directory).unwrap();
    }
}
