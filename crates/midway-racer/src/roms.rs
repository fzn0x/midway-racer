//! Finding the game's ROMs on disk.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use mw8080bw::{Game, ProgramRom, RomFiles, RomReport};

/// How deep to look inside directories: `roms/lagunar/lagunar.h` is found
/// from `roms`.
const MAX_DEPTH: usize = 2;

/// Loads the program ROM of `game`, or of whichever game the files at
/// `paths` hold, and reports how every chip was matched.
pub(crate) fn load(
    paths: &[impl AsRef<Path>],
    game: Option<Game>,
) -> Result<(ProgramRom, RomReport)> {
    let mut files = RomFiles::new();
    for path in paths {
        let path = path.as_ref();
        if !path.exists() {
            bail!(
                "{} does not exist
{}",
                path.display(),
                expected_sets()
            );
        }
        add_path(&mut files, path, MAX_DEPTH)?;
    }
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

fn add_path(files: &mut RomFiles, path: &Path, depth: usize) -> Result<()> {
    if path.is_dir() {
        if depth == 0 {
            return Ok(());
        }
        let entries =
            fs::read_dir(path).with_context(|| format!("cannot list {}", path.display()))?;
        for entry in entries {
            let entry = entry.with_context(|| format!("cannot list {}", path.display()))?;
            add_path(files, &entry.path(), depth - 1)?;
        }
        Ok(())
    } else {
        let data = fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
        files
            .add_file(path.display().to_string(), data)
            .with_context(|| format!("cannot read {}", path.display()))
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
    use super::*;

    #[test]
    fn missing_directories_explain_what_is_needed() {
        let error = load(&["no/such/directory"], None).unwrap_err().to_string();
        assert!(
            error.contains("no/such/directory does not exist"),
            "{error}"
        );
        assert!(
            error.contains("Laguna Racer (lagunar.zip): lagunar.h"),
            "{error}"
        );
    }

    #[test]
    fn unrelated_files_are_not_mistaken_for_a_game() {
        let directory = std::env::temp_dir().join(format!("midway-racer-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("notes.txt"), b"not a ROM").unwrap();

        let error = load(&[&directory], None).unwrap_err().to_string();
        assert!(
            error.contains("hold none of the supported games"),
            "{error}"
        );
        fs::remove_dir_all(&directory).unwrap();
    }
}
