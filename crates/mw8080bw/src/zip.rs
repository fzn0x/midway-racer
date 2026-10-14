//! A minimal zip archive reader, enough for MAME ROM archives.
//!
//! It walks the central directory, supports stored and deflated entries,
//! verifies every entry's CRC-32 and skips entries too large to be ROM
//! dumps.

use std::fmt;

use crate::crc32::crc32;

const END_OF_CENTRAL_DIRECTORY: u32 = 0x0605_4b50;
const CENTRAL_DIRECTORY_HEADER: u32 = 0x0201_4b50;
const LOCAL_FILE_HEADER: u32 = 0x0403_4b50;
const END_OF_CENTRAL_DIRECTORY_SIZE: usize = 22;
const CENTRAL_DIRECTORY_HEADER_SIZE: usize = 46;
const LOCAL_FILE_HEADER_SIZE: usize = 30;

const STORED: u16 = 0;
const DEFLATED: u16 = 8;
const ENCRYPTED_FLAG: u16 = 0x0001;

/// Entries larger than this cannot be ROM dumps of these boards and are
/// skipped rather than inflated.
const MAX_ENTRY_SIZE: usize = 0x1_0000;

/// Returns whether `data` starts like a zip archive.
pub(crate) fn is_zip(data: &[u8]) -> bool {
    data.starts_with(b"PK\x03\x04") || data.starts_with(b"PK\x05\x06")
}

/// Extracts the files of a zip archive as `(name, contents)` pairs.
pub(crate) fn extract(archive: &[u8]) -> Result<Vec<(String, Vec<u8>)>, ZipError> {
    let directory = find_end_of_central_directory(archive)?;
    let entries = usize::from(read_u16(archive, directory + 10)?);
    let mut offset = read_u32(archive, directory + 16)? as usize;

    let mut files = Vec::with_capacity(entries);
    for _ in 0..entries {
        if read_u32(archive, offset)? != CENTRAL_DIRECTORY_HEADER {
            return Err(ZipError::Corrupt);
        }
        let flags = read_u16(archive, offset + 8)?;
        let method = read_u16(archive, offset + 10)?;
        let crc = read_u32(archive, offset + 16)?;
        let compressed_size = read_u32(archive, offset + 20)?;
        let size = read_u32(archive, offset + 24)?;
        let name_length = usize::from(read_u16(archive, offset + 28)?);
        let extra_length = usize::from(read_u16(archive, offset + 30)?);
        let comment_length = usize::from(read_u16(archive, offset + 32)?);
        let local_header = read_u32(archive, offset + 42)?;
        let name_start = offset + CENTRAL_DIRECTORY_HEADER_SIZE;
        let name = slice(archive, name_start, name_length)?;
        let name = String::from_utf8_lossy(name).into_owned();
        offset = name_start + name_length + extra_length + comment_length;

        if name.ends_with('/') || size as usize > MAX_ENTRY_SIZE {
            continue;
        }
        if [compressed_size, size, local_header].contains(&u32::MAX) {
            return Err(ZipError::Unsupported("ZIP64 archives"));
        }
        if flags & ENCRYPTED_FLAG != 0 {
            return Err(ZipError::Unsupported("encrypted entries"));
        }

        let compressed = entry_data(archive, local_header as usize, compressed_size as usize)?;
        let data = match method {
            STORED => compressed.to_vec(),
            DEFLATED => {
                miniz_oxide::inflate::decompress_to_vec_with_limit(compressed, size as usize)
                    .map_err(|_| ZipError::Corrupt)?
            }
            _ => {
                return Err(ZipError::Unsupported(
                    "compression methods other than deflate",
                ));
            }
        };
        if data.len() != size as usize || crc32(&data) != crc {
            return Err(ZipError::ChecksumMismatch { name });
        }
        files.push((name, data));
    }
    Ok(files)
}

/// Finds the end of central directory record, which sits before an
/// optional comment of up to 64 KiB at the end of the archive.
fn find_end_of_central_directory(archive: &[u8]) -> Result<usize, ZipError> {
    let last = archive
        .len()
        .checked_sub(END_OF_CENTRAL_DIRECTORY_SIZE)
        .ok_or(ZipError::NotAnArchive)?;
    let first = last.saturating_sub(usize::from(u16::MAX));
    (first..=last)
        .rev()
        .find(|&offset| read_u32(archive, offset) == Ok(END_OF_CENTRAL_DIRECTORY))
        .ok_or(ZipError::NotAnArchive)
}

/// Returns the (possibly compressed) data of the entry whose local header
/// starts at `offset`.
fn entry_data(archive: &[u8], offset: usize, size: usize) -> Result<&[u8], ZipError> {
    if read_u32(archive, offset)? != LOCAL_FILE_HEADER {
        return Err(ZipError::Corrupt);
    }
    let name_length = usize::from(read_u16(archive, offset + 26)?);
    let extra_length = usize::from(read_u16(archive, offset + 28)?);
    slice(
        archive,
        offset + LOCAL_FILE_HEADER_SIZE + name_length + extra_length,
        size,
    )
}

fn slice(archive: &[u8], offset: usize, length: usize) -> Result<&[u8], ZipError> {
    archive
        .get(offset..offset.checked_add(length).ok_or(ZipError::Corrupt)?)
        .ok_or(ZipError::Corrupt)
}

fn read_u16(archive: &[u8], offset: usize) -> Result<u16, ZipError> {
    let bytes = slice(archive, offset, 2)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(archive: &[u8], offset: usize) -> Result<u32, ZipError> {
    let bytes = slice(archive, offset, 4)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Why a zip archive could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZipError {
    /// The data is not a zip archive.
    NotAnArchive,
    /// The archive is truncated or damaged.
    Corrupt,
    /// The archive uses a feature this reader does not support.
    Unsupported(&'static str),
    /// An entry's contents do not match its recorded checksum.
    ChecksumMismatch {
        /// The name of the entry.
        name: String,
    },
}

impl fmt::Display for ZipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAnArchive => f.write_str("not a zip archive"),
            Self::Corrupt => f.write_str("the zip archive is damaged"),
            Self::Unsupported(feature) => write!(f, "{feature} are not supported"),
            Self::ChecksumMismatch { name } => {
                write!(f, "{name} is damaged inside the zip archive")
            }
        }
    }
}

impl std::error::Error for ZipError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// An archive written by Python's `zipfile`: a stored text file, a
    /// directory, a deflated 2 KiB pattern and an archive comment.
    const SAMPLE: &[u8] = include_bytes!("../tests/fixtures/sample.zip");

    fn pattern() -> Vec<u8> {
        (0..2048).map(|index: u32| (index * 7) as u8).collect()
    }

    #[test]
    fn extracts_stored_and_deflated_entries() {
        assert!(is_zip(SAMPLE));
        let files = extract(SAMPLE).unwrap();
        let names: Vec<_> = files.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["readme.txt", "roms/pattern.bin"]);
        assert_eq!(files[0].1, b"Laguna Racer\n");
        assert_eq!(files[1].1, pattern());
    }

    #[test]
    fn rejects_data_that_is_not_an_archive() {
        assert!(!is_zip(b"lagunar.h"));
        assert_eq!(
            extract(b"definitely not a zip"),
            Err(ZipError::NotAnArchive)
        );
        assert_eq!(extract(&[]), Err(ZipError::NotAnArchive));
    }

    #[test]
    fn detects_truncation() {
        let truncated = &SAMPLE[..SAMPLE.len() / 2];
        assert!(extract(truncated).is_err());
    }

    #[test]
    fn detects_damaged_contents() {
        let mut damaged = SAMPLE.to_vec();
        let text = damaged
            .windows(12)
            .position(|window| window == b"Laguna Racer")
            .unwrap();
        damaged[text] = b'X';
        assert_eq!(
            extract(&damaged),
            Err(ZipError::ChecksumMismatch {
                name: "readme.txt".into()
            })
        );
    }
}
