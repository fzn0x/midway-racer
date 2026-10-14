//! The video hardware: a shift register clocks video RAM out to the
//! monitor, one byte per eight pixels, least significant bit first.

use std::fmt;

use crate::memory::VIDEO_RAM_SIZE;
use crate::timing::VISIBLE_LINES;

/// Width of the raster: 256 pixels of video RAM, plus four more that the
/// shift register keeps clocking out after horizontal blanking starts.
pub const SCREEN_WIDTH: usize = 260;
/// Height of the raster in scan lines.
pub const SCREEN_HEIGHT: usize = VISIBLE_LINES as usize;

/// Bytes of video RAM per scan line.
const BYTES_PER_LINE: usize = 32;
/// The shift register is first loaded four pixels into each line, so
/// video RAM appears four pixels to the right of the line's start.
const FIRST_VIDEO_PIXEL: usize = 4;

/// The monochrome raster as the monitor draws it, before any rotation:
/// one byte per pixel in scan order, 0 for dark and 1 for lit.
#[derive(Clone, PartialEq, Eq)]
pub struct Screen {
    pixels: Box<[u8]>,
}

impl Screen {
    /// Creates a dark screen.
    pub fn new() -> Self {
        Self {
            pixels: vec![0; SCREEN_WIDTH * SCREEN_HEIGHT].into_boxed_slice(),
        }
    }

    /// Returns every pixel in scan order, [`SCREEN_WIDTH`] per line.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Returns whether the pixel at column `x` of line `y` is lit.
    ///
    /// # Panics
    ///
    /// Panics if the position is outside the raster.
    pub fn is_lit(&self, x: usize, y: usize) -> bool {
        assert!(
            x < SCREEN_WIDTH && y < SCREEN_HEIGHT,
            "({x}, {y}) is off screen"
        );
        self.pixels[y * SCREEN_WIDTH + x] != 0
    }

    /// Draws the whole raster from `video_ram`, the memory the hardware
    /// scans out from address `2400`.
    ///
    /// # Panics
    ///
    /// Panics if `video_ram` is shorter than [`VIDEO_RAM_SIZE`] bytes.
    pub fn render(&mut self, video_ram: &[u8]) {
        for y in 0..SCREEN_HEIGHT {
            self.render_line(y, video_ram);
        }
    }

    /// Draws scan line `y` from `video_ram`.
    pub(crate) fn render_line(&mut self, y: usize, video_ram: &[u8]) {
        assert!(video_ram.len() >= VIDEO_RAM_SIZE, "video RAM is too short");
        let bytes = &video_ram[y * BYTES_PER_LINE..][..BYTES_PER_LINE];
        let line = &mut self.pixels[y * SCREEN_WIDTH..][..SCREEN_WIDTH];
        let (blank, scanned) = line.split_at_mut(FIRST_VIDEO_PIXEL);
        blank.fill(0);
        for (&byte, pixels) in bytes.iter().zip(scanned.chunks_exact_mut(8)) {
            for (bit, pixel) in pixels.iter_mut().enumerate() {
                *pixel = byte >> bit & 1;
            }
        }
    }
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Screen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let lit = self.pixels.iter().filter(|&&pixel| pixel != 0).count();
        f.debug_struct("Screen").field("lit_pixels", &lit).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_is_260_by_224() {
        assert_eq!(Screen::new().pixels().len(), 260 * 224);
        assert_eq!(VIDEO_RAM_SIZE, SCREEN_HEIGHT * BYTES_PER_LINE);
    }

    #[test]
    fn bits_scan_out_lsb_first_four_pixels_into_the_line() {
        let mut video_ram = vec![0; VIDEO_RAM_SIZE];
        video_ram[0] = 0b0000_0101;
        video_ram[31] = 0b1000_0000;
        video_ram[223 * 32] = 0xff;

        let mut screen = Screen::new();
        screen.render(&video_ram);

        let line: Vec<u8> = screen.pixels()[..12].to_vec();
        assert_eq!(line, [0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0]);
        assert!(screen.is_lit(259, 0));
        assert!(!screen.is_lit(258, 0));
        assert!((4..12).all(|x| screen.is_lit(x, 223)));
        assert_eq!(
            screen.pixels().iter().filter(|&&pixel| pixel != 0).count(),
            11
        );
    }

    #[test]
    fn rendering_a_line_leaves_the_others_alone() {
        let mut screen = Screen::new();
        let video_ram = vec![0xff; VIDEO_RAM_SIZE];
        screen.render_line(10, &video_ram);
        assert!(screen.is_lit(4, 10));
        assert!(!screen.is_lit(4, 9) && !screen.is_lit(4, 11));
    }
}
