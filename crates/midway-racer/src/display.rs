//! Scaling the picture for the window, in the monitor's shape.

use mw8080bw::{Game, Orientation, Rgb, SCREEN_HEIGHT, Screen};

/// The picture scaled for the window, in minifb's `0RGB` pixel format.
#[derive(Debug)]
pub(crate) struct Display {
    game: Game,
    picture: Vec<u32>,
    picture_width: usize,
    buffer: Vec<u32>,
    width: usize,
    /// The picture column shown in each buffer column.
    columns: Vec<usize>,
    /// The picture row shown in each buffer row.
    rows: Vec<usize>,
}

impl Display {
    /// Prepares to show `game`'s picture `scale` times its original size.
    ///
    /// The cabinet's 4:3 monitor did not have square pixels: unless
    /// `square_pixels` is set, the 224-pixel side is scaled by `scale` and
    /// the 260-pixel side stretched to restore the monitor's shape.
    pub(crate) fn new(game: Game, scale: usize, square_pixels: bool) -> Self {
        let orientation = game.info().orientation;
        let (picture_width, picture_height) = orientation.picture_size();
        let (width, height) = if square_pixels {
            (picture_width * scale, picture_height * scale)
        } else {
            let short = SCREEN_HEIGHT * scale;
            let long = short * 4 / 3;
            match orientation {
                Orientation::Horizontal => (long, short),
                Orientation::Vertical => (short, long),
            }
        };
        Self {
            game,
            picture: vec![0; picture_width * picture_height],
            picture_width,
            buffer: vec![0; width * height],
            width,
            columns: (0..width).map(|x| x * picture_width / width).collect(),
            rows: (0..height).map(|y| y * picture_height / height).collect(),
        }
    }

    /// Returns the width and height of the scaled picture.
    pub(crate) fn size(&self) -> (usize, usize) {
        (self.width, self.rows.len())
    }

    /// Draws `screen` and returns the scaled picture.
    pub(crate) fn draw(&mut self, screen: &Screen) -> &[u32] {
        screen.draw_picture(self.game, &mut self.picture, to_0rgb);
        for (line, &row) in self.buffer.chunks_exact_mut(self.width).zip(&self.rows) {
            let source = &self.picture[row * self.picture_width..][..self.picture_width];
            for (pixel, &column) in line.iter_mut().zip(&self.columns) {
                *pixel = source[column];
            }
        }
        &self.buffer
    }
}

fn to_0rgb(color: Rgb) -> u32 {
    u32::from(color.red) << 16 | u32::from(color.green) << 8 | u32::from(color.blue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_three_pictures_keep_the_monitor_shape() {
        assert_eq!(Display::new(Game::LagunaRacer, 2, false).size(), (448, 597));
        assert_eq!(Display::new(Game::Zzzap, 3, false).size(), (896, 672));
    }

    #[test]
    fn square_pixels_scale_both_sides_alike() {
        assert_eq!(Display::new(Game::LagunaRacer, 2, true).size(), (448, 520));
        assert_eq!(Display::new(Game::Zzzap, 1, true).size(), (260, 224));
    }

    #[test]
    fn every_picture_pixel_is_shown() {
        let display = Display::new(Game::LagunaRacer, 1, false);
        assert_eq!(display.columns.first(), Some(&0));
        assert_eq!(display.columns.last(), Some(&223));
        assert_eq!(display.rows.last(), Some(&259));
        assert!(display.rows.windows(2).all(|pair| pair[1] - pair[0] <= 1));
    }

    #[test]
    fn draws_a_dark_screen_in_black() {
        let mut display = Display::new(Game::Zzzap, 1, false);
        assert!(display.draw(&Screen::new()).iter().all(|&pixel| pixel == 0));
        assert_eq!(to_0rgb(Rgb::new(0x12, 0x34, 0x56)), 0x0012_3456);
    }
}
