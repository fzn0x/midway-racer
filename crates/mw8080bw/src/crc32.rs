//! CRC-32 (IEEE 802.3), as used by zip archives and MAME's ROM listings.

const POLYNOMIAL: u32 = 0xedb8_8320;
const TABLE: [u32; 256] = table();

const fn table() -> [u32; 256] {
    let mut table = [0; 256];
    let mut index = 0;
    while index < table.len() {
        let mut crc = index as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 == 0 {
                crc >> 1
            } else {
                crc >> 1 ^ POLYNOMIAL
            };
            bit += 1;
        }
        table[index] = crc;
        index += 1;
    }
    table
}

/// Computes the CRC-32 of `data`.
pub(crate) fn crc32(data: &[u8]) -> u32 {
    !data.iter().fold(!0, |crc, &byte| {
        TABLE[usize::from(crc as u8 ^ byte)] ^ crc >> 8
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn empty_input_has_a_zero_checksum() {
        assert_eq!(crc32(&[]), 0);
    }
}
