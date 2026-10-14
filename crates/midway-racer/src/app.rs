//! The main loop: emulating, presenting and pacing frames.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use minifb::{ScaleMode, Window, WindowOptions};
use mw8080bw::timing::FRAME_RATE_HZ;
use mw8080bw::{Gear, Machine};

use crate::audio::AudioOutput;
use crate::cli::Args;
use crate::display::Display;
use crate::input::{Command, Input};

/// Emulated time per frame, in seconds.
const FRAME_SECONDS: f32 = (1.0 / FRAME_RATE_HZ) as f32;
/// Most frames emulated at once to catch up. Longer stalls, such as while
/// the window is dragged, are skipped rather than fast-forwarded.
const MAX_CATCH_UP_FRAMES: u64 = 4;
/// Sound kept queued ahead of the audio device, in seconds: enough to
/// ride out the main loop's sleeps.
const AUDIO_LEAD_SECONDS: f64 = 0.066;
/// Volume change per key press, in percent.
const VOLUME_STEP: u8 = 10;
/// Loudest volume, in percent.
const MAX_VOLUME: u8 = 200;

/// The emulator window and everything it drives.
pub(crate) struct App {
    machine: Machine,
    window: Window,
    display: Display,
    input: Input,
    /// Paces emulation by the sound card's clock when there is sound.
    audio: Option<AudioOutput>,
    /// Paces emulation by the wall clock when there is no sound.
    clock: FrameClock,
    volume: u8,
    muted: bool,
    paused: bool,
    title: String,
}

impl App {
    /// Opens a window for `machine`, playing its sound through `audio`.
    pub(crate) fn new(
        mut machine: Machine,
        audio: Option<AudioOutput>,
        args: &Args,
    ) -> Result<Self> {
        let display = Display::new(machine.game(), usize::from(args.scale), args.square_pixels);
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
        if let Some(audio) = &audio {
            machine.enable_audio(audio.sample_rate());
        }

        Ok(Self {
            machine,
            window,
            display,
            input: Input::new(),
            audio,
            clock: FrameClock::new(),
            volume: args.volume,
            muted: args.mute,
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
                    Command::ToggleMute => self.muted = !self.muted,
                    Command::VolumeDown => self.set_volume(self.volume.saturating_sub(VOLUME_STEP)),
                    Command::VolumeUp => self.set_volume(self.volume.saturating_add(VOLUME_STEP)),
                    Command::Quit => return Ok(()),
                }
            }
            if !self.paused {
                for _ in 0..self.frames_due() {
                    self.emulate_frame();
                }
            }
            self.present()?;
        }
        Ok(())
    }

    fn set_volume(&mut self, volume: u8) {
        self.volume = volume.min(MAX_VOLUME);
        self.muted = false;
    }

    /// Returns how many frames to emulate now: enough to keep the audio
    /// device fed, or to keep up with the wall clock without sound.
    fn frames_due(&mut self) -> u64 {
        if self.audio.as_ref().is_some_and(AudioOutput::has_failed) {
            eprintln!("warning: lost the audio device, continuing without sound");
            self.audio = None;
            self.machine.disable_audio();
            self.clock = FrameClock::new();
        }
        match &self.audio {
            Some(audio) => audio_frames_due(audio.queued(), audio.sample_rate()),
            None => self.clock.frames_due(self.clock.start.elapsed()),
        }
    }

    fn emulate_frame(&mut self) {
        let controls = self.input.controls(&self.window, FRAME_SECONDS);
        self.machine.set_controls(controls);
        self.machine.run_frame();

        if let Some(audio) = &self.audio {
            let gain = if self.muted {
                0.0
            } else {
                f32::from(self.volume) / 100.0
            };
            let samples = self.machine.audio_samples();
            audio.play(
                samples
                    .iter()
                    .map(|sample| (sample * gain).clamp(-1.0, 1.0))
                    .collect(),
            );
        }
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
            Gear::Low => "low gear",
            Gear::High => "high gear",
        };
        let mut parts = vec![self.machine.game().info().title.to_owned(), gear.to_owned()];
        if self.paused {
            parts.push("paused".to_owned());
        }
        if self.audio.is_none() {
            parts.push("no sound".to_owned());
        } else if self.muted {
            parts.push("muted".to_owned());
        } else if self.volume != 100 {
            parts.push(format!("volume {}%", self.volume));
        }
        parts.join(" - ")
    }
}

/// Returns how many frames to emulate to keep [`AUDIO_LEAD_SECONDS`] of
/// sound queued, given `queued` samples waiting at `sample_rate`.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "sample counts are small and the frame count is positive"
)]
fn audio_frames_due(queued: usize, sample_rate: u32) -> u64 {
    let rate = f64::from(sample_rate);
    let missing = (AUDIO_LEAD_SECONDS * rate - queued as f64).max(0.0);
    let frames = (missing * FRAME_RATE_HZ / rate).ceil() as u64;
    frames.min(MAX_CATCH_UP_FRAMES)
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

    #[test]
    fn audio_pacing_tops_up_the_device_queue() {
        // 66 ms at 48 kHz is 3168 samples, about four frames of 806.
        assert_eq!(audio_frames_due(0, 48_000), 4);
        assert_eq!(audio_frames_due(2_000, 48_000), 2);
        assert_eq!(audio_frames_due(3_000, 48_000), 1);
        assert_eq!(audio_frames_due(3_168, 48_000), 0);
        assert_eq!(audio_frames_due(10_000, 44_100), 0);
    }
}
