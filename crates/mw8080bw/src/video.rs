//! The video hardware: a shift register clocks video RAM out to the
//! monitor, one byte per eight pixels, least significant bit first.

use std::fmt;

use crate::game::{Game, Orientation, Rgb};
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

    /// Draws the picture the player of `game` sees into `out`: rotated to
    /// the cabinet's monitor orientation and tinted by its overlay film.
    /// `encode` turns each colour into the caller's pixel format.
    ///
    /// The picture has the size [`Orientation::picture_size`] gives,
    /// row by row.
    ///
    /// # Panics
    ///
    /// Panics if `out` does not hold exactly one picture.
    pub fn draw_picture<P: Copy>(&self, game: Game, out: &mut [P], encode: impl Fn(Rgb) -> P) {
        let info = game.info();
        let (width, height) = info.orientation.picture_size();
        assert_eq!(
            out.len(),
            width * height,
            "the output must hold one picture"
        );

        let dark = encode(Rgb::BLACK);
        let lit: [P; SCREEN_HEIGHT] = std::array::from_fn(|y| {
            let band = info
                .overlay
                .iter()
                .find(|band| (band.top..band.bottom).contains(&y));
            encode(band.map_or(Rgb::WHITE, |band| band.color))
        });
        let color = |x: usize, y: usize| {
            if self.pixels[y * SCREEN_WIDTH + x] == 0 {
                dark
            } else {
                lit[y]
            }
        };

        match info.orientation {
            Orientation::Horizontal => {
                for (index, pixel) in out.iter_mut().enumerate() {
                    *pixel = color(index % SCREEN_WIDTH, index / SCREEN_WIDTH);
                }
            }
            // Turning the monitor clockwise takes raster pixel (x, y) to
            // picture pixel (SCREEN_HEIGHT - 1 - y, x).
            Orientation::Vertical => {
                for (index, pixel) in out.iter_mut().enumerate() {
                    let (column, row) = (index % SCREEN_HEIGHT, index / SCREEN_HEIGHT);
                    *pixel = color(row, SCREEN_HEIGHT - 1 - column);
                }
            }
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

    /// A screen with the given raster pixels lit; `x` must be at least 4.
    fn screen_with(lit: &[(usize, usize)]) -> Screen {
        let mut video_ram = vec![0; VIDEO_RAM_SIZE];
        for &(x, y) in lit {
            let bit = x - FIRST_VIDEO_PIXEL;
            video_ram[y * BYTES_PER_LINE + bit / 8] |= 1 << (bit % 8);
        }
        let mut screen = Screen::new();
        screen.render(&video_ram);
        screen
    }

    fn picture(screen: &Screen, game: Game) -> Vec<Rgb> {
        let (width, height) = game.info().orientation.picture_size();
        let mut out = vec![Rgb::new(1, 2, 3); width * height];
        screen.draw_picture(game, &mut out, |color| color);
        out
    }

    #[test]
    fn horizontal_pictures_keep_the_raster_layout() {
        let screen = screen_with(&[(5, 3)]);
        let out = picture(&screen, Game::Zzzap);
        assert_eq!(out[3 * SCREEN_WIDTH + 5], Rgb::WHITE);
        assert_eq!(
            out.iter().filter(|&&color| color == Rgb::BLACK).count(),
            out.len() - 1
        );
    }

    #[test]
    fn vertical_pictures_are_turned_clockwise() {
        let screen = screen_with(&[(4, 0), (259, 223), (100, 10)]);
        let out = picture(&screen, Game::LagunaRacer);
        let at = |x: usize, y: usize| out[y * SCREEN_HEIGHT + x];
        assert_eq!(at(223, 4), Rgb::WHITE, "top left goes to top right");
        assert_eq!(at(0, 259), Rgb::WHITE, "bottom right goes to bottom left");
        assert_eq!(at(213, 100), Rgb::WHITE);
        assert_eq!(out.iter().filter(|&&color| color == Rgb::WHITE).count(), 3);
    }

    #[test]
    fn overlay_tints_lit_pixels_only() {
        let screen = screen_with(&[(10, 100), (10, 200)]);
        let out = picture(&screen, Game::Zzzap);
        assert_eq!(out[100 * SCREEN_WIDTH + 10], Rgb::WHITE);
        assert_eq!(out[200 * SCREEN_WIDTH + 10], Rgb::new(0x33, 0x66, 0xff));
        assert_eq!(out[200 * SCREEN_WIDTH + 11], Rgb::BLACK);
    }

    #[test]
    #[should_panic(expected = "one picture")]
    fn drawing_into_a_wrongly_sized_buffer_panics() {
        Screen::new().draw_picture(Game::LagunaRacer, &mut [0u32; 10], |_| 0);
    }
}
