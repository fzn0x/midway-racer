//! The main loop: emulating, presenting and pacing frames.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use minifb::{ScaleMode, Window, WindowOptions};
use mw8080bw::timing::FRAME_RATE_HZ;
use mw8080bw::{Gear, Machine};

use crate::display::Display;
use crate::input::{Command, Input};

/// Emulated time per frame, in seconds.
const FRAME_SECONDS: f32 = (1.0 / FRAME_RATE_HZ) as f32;
/// Most frames emulated at once to catch up. Longer stalls, such as while
/// the window is dragged, are skipped rather than fast-forwarded.
const MAX_CATCH_UP_FRAMES: u64 = 4;

/// The emulator window and everything it drives.
pub(crate) struct App {
    machine: Machine,
    window: Window,
    display: Display,
    input: Input,
    clock: FrameClock,
    paused: bool,
    title: String,
}

impl App {
    /// Opens a window showing the machine's picture `scale` times its
    /// original size.
    pub(crate) fn new(machine: Machine, scale: usize, square_pixels: bool) -> Result<Self> {
        let display = Display::new(machine.game(), scale, square_pixels);
        let (width, height) = display.size();
        let title = machine.game().info().title.to_owned();
        let options = WindowOptions {
            resize: true,
            scale_mode: ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        };
        let mut window =
            Window::new(&title, width, height, options).context("cannot open a window")?;
        window.set_target_fps(60);

        Ok(Self {
            machine,
            window,
            display,
            input: Input::default(),
            clock: FrameClock::new(),
            paused: false,
            title,
        })
    }

    /// Runs until the window is closed or the player quits.
    pub(crate) fn run(mut self) -> Result<()> {
        while self.window.is_open() {
            for command in self.input.commands(&self.window) {
                match command {
                    Command::InsertCoin => self.machine.insert_coin(),
                    Command::TogglePause => {
                        self.paused = !self.paused;
                        self.clock = FrameClock::new();
                    }
                    Command::Reset => self.machine.reset(),
                    Command::Quit => return Ok(()),
                }
            }
            if !self.paused {
                for _ in 0..self.clock.frames_due(self.clock.start.elapsed()) {
                    self.emulate_frame();
                }
            }
            self.present()?;
        }
        Ok(())
    }

    fn emulate_frame(&mut self) {
        let controls = self.input.controls(&self.window, FRAME_SECONDS);
        self.machine.set_controls(controls);
        self.machine.run_frame();
    }

    fn present(&mut self) -> Result<()> {
        let (width, height) = self.display.size();
        let picture = self.display.draw(self.machine.screen());
        self.window
            .update_with_buffer(picture, width, height)
            .context("cannot draw the window")?;

        let title = self.status();
        if title != self.title {
            self.window.set_title(&title);
            self.title = title;
        }
        Ok(())
    }

    /// Describes the game's state for the window title.
    fn status(&self) -> String {
        let gear = match self.input.gear() {
            Gear::Low => "low",
            Gear::High => "high",
        };
        let mut status = format!("{} - {gear} gear", self.machine.game().info().title);
        if self.paused {
            status.push_str(" - paused");
        }
        status
    }
}

/// Paces emulation by the wall clock, at the board's refresh rate.
#[derive(Debug)]
struct FrameClock {
    start: Instant,
    frames: u64,
}

impl FrameClock {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            frames: 0,
        }
    }

    /// Returns how many frames to emulate to keep up, `elapsed` after the
    /// clock started.
    #[expect(clippy::cast_sign_loss, reason = "elapsed time is never negative")]
    fn frames_due(&mut self, elapsed: Duration) -> u64 {
        let due = (elapsed.as_secs_f64() * FRAME_RATE_HZ) as u64;
        self.frames = self.frames.max(due.saturating_sub(MAX_CATCH_UP_FRAMES));
        let behind = due - self.frames.min(due);
        self.frames += behind;
        behind
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_keeps_the_refresh_rate() {
        let mut clock = FrameClock::new();
        let total: u64 = (1..=60)
            .map(|step| clock.frames_due(Duration::from_millis(step * 1_000 / 60)))
            .sum();
        assert_eq!(total, 59, "59.54 frames are due after one second");
    }

    #[test]
    fn clock_skips_long_stalls() {
        let mut clock = FrameClock::new();
        assert_eq!(
            clock.frames_due(Duration::from_secs(10)),
            MAX_CATCH_UP_FRAMES
        );
        assert_eq!(clock.frames_due(Duration::from_secs(10)), 0);
        assert_eq!(clock.frames_due(Duration::from_millis(10_017)), 1);
    }
}
