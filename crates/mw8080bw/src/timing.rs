//! Clocks and raster timing, all derived from the 19.968 MHz crystal.

/// Frequency of the master crystal.
pub const MASTER_CLOCK_HZ: u32 = 19_968_000;
/// The CPU clock, a tenth of the master clock: 1.9968 MHz.
pub const CPU_CLOCK_HZ: u32 = MASTER_CLOCK_HZ / 10;
/// The pixel clock, a quarter of the master clock: 4.992 MHz.
pub const PIXEL_CLOCK_HZ: u32 = MASTER_CLOCK_HZ / 4;

/// Pixel clocks per scan line, horizontal blanking included.
pub const PIXELS_PER_LINE: u32 = 320;
/// Scan lines per frame, vertical blanking included.
pub const LINES_PER_FRAME: u32 = 262;
/// Scan lines drawn before vertical blanking starts.
pub const VISIBLE_LINES: u32 = 224;

/// CPU cycles per scan line.
pub const CYCLES_PER_LINE: u32 = PIXELS_PER_LINE * CPU_CLOCK_HZ / PIXEL_CLOCK_HZ;
/// CPU cycles per frame.
pub const CYCLES_PER_FRAME: u32 = CYCLES_PER_LINE * LINES_PER_FRAME;
/// The refresh rate, about 59.54 frames per second.
pub const FRAME_RATE_HZ: f64 = CPU_CLOCK_HZ as f64 / CYCLES_PER_FRAME as f64;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_timings_match_the_hardware() {
        assert_eq!(CPU_CLOCK_HZ, 1_996_800);
        assert_eq!(CYCLES_PER_LINE, 128);
        assert_eq!(CYCLES_PER_FRAME, 33_536);
        assert!((FRAME_RATE_HZ - 59.541_985).abs() < 1e-6);
    }
}
