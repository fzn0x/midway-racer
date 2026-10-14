//! The main CPU's memory map.
//!
//! The board decodes only address lines A0–A14, so the map repeats every
//! 32 KiB:
//!
//! | Address       | Contents                                           |
//! |---------------|----------------------------------------------------|
//! | `0000`–`1FFF` | Program ROM                                        |
//! | `2000`–`3FFF` | 8 KiB of RAM; `2400`–`3FFF` is scanned as video RAM |
//! | `4000`–`5FFF` | Second ROM area, unpopulated on the racing boards  |
//! | `6000`–`7FFF` | Mirror of the RAM                                  |

use std::fmt;

/// Size of the program ROM area, in bytes.
pub const ROM_SIZE: usize = 0x2000;
/// Size of the RAM, in bytes.
pub const RAM_SIZE: usize = 0x2000;
/// Offset of video RAM within the RAM: the part the video hardware scans
/// out during the visible lines.
pub const VIDEO_RAM_OFFSET: usize = 0x0400;
/// Size of video RAM, in bytes.
pub const VIDEO_RAM_SIZE: usize = RAM_SIZE - VIDEO_RAM_OFFSET;

/// ROM and RAM as seen from the CPU.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Memory {
    rom: Box<[u8; ROM_SIZE]>,
    ram: Box<[u8; RAM_SIZE]>,
}

impl Memory {
    /// Creates the memory with the given program ROM and cleared RAM.
    pub(crate) fn new(rom: Box<[u8; ROM_SIZE]>) -> Self {
        Self {
            rom,
            ram: Box::new([0; RAM_SIZE]),
        }
    }

    /// Reads a byte the way the CPU sees it.
    pub(crate) fn read(&self, address: u16) -> u8 {
        let address = usize::from(address & 0x7fff);
        match address {
            0x0000..=0x1fff => self.rom[address],
            0x4000..=0x5fff => 0x00,
            _ => self.ram[address & 0x1fff],
        }
    }

    /// Writes a byte the way the CPU does: ROM ignores writes.
    pub(crate) fn write(&mut self, address: u16, value: u8) {
        let address = usize::from(address & 0x7fff);
        if matches!(address, 0x2000..=0x3fff | 0x6000..=0x7fff) {
            self.ram[address & 0x1fff] = value;
        }
    }

    /// Returns the whole RAM.
    pub(crate) fn ram(&self) -> &[u8; RAM_SIZE] {
        &self.ram
    }

    /// Returns the part of the RAM the video hardware scans out.
    pub(crate) fn video_ram(&self) -> &[u8] {
        &self.ram[VIDEO_RAM_OFFSET..]
    }
}

impl fmt::Debug for Memory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Memory").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory() -> Memory {
        let mut rom = Box::new([0; ROM_SIZE]);
        for (index, byte) in rom.iter_mut().enumerate() {
            *byte = index as u8;
        }
        Memory::new(rom)
    }

    #[test]
    fn rom_is_read_only() {
        let mut memory = memory();
        assert_eq!(memory.read(0x1234), 0x34);
        memory.write(0x1234, 0xff);
        assert_eq!(memory.read(0x1234), 0x34);
    }

    #[test]
    fn ram_is_mirrored_at_6000() {
        let mut memory = memory();
        memory.write(0x2400, 0xaa);
        assert_eq!(memory.read(0x6400), 0xaa);
        assert_eq!(memory.video_ram()[0], 0xaa);
        memory.write(0x7fff, 0x55);
        assert_eq!(memory.read(0x3fff), 0x55);
        assert_eq!(memory.ram()[0x1fff], 0x55);
    }

    #[test]
    fn address_line_15_is_ignored() {
        let mut memory = memory();
        memory.write(0xa000, 0x42);
        assert_eq!(memory.read(0x2000), 0x42);
        assert_eq!(memory.read(0x8010), 0x10);
    }

    #[test]
    fn unpopulated_rom_area_reads_zero() {
        let mut memory = memory();
        memory.write(0x4000, 0x99);
        assert_eq!(memory.read(0x4000), 0x00);
        assert_eq!(memory.read(0x5fff), 0x00);
    }
}
