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

/// Size of the program ROM area, in bytes.
pub const ROM_SIZE: usize = 0x2000;
/// Size of the RAM, in bytes.
pub const RAM_SIZE: usize = 0x2000;
/// Offset of video RAM within the RAM: the part the video hardware scans
/// out during the visible lines.
pub const VIDEO_RAM_OFFSET: usize = 0x0400;
/// Size of video RAM, in bytes.
pub const VIDEO_RAM_SIZE: usize = RAM_SIZE - VIDEO_RAM_OFFSET;
