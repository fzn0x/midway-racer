//! Playing the sound board through the default audio device.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};

use anyhow::{Context, Result, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, ErrorKind, FromSample, SampleFormat, SizedSample, Stream, StreamConfig};

/// How much each sample of the fade-out keeps of the previous one when
/// the emulator falls behind: an underrun fades over a few milliseconds
/// instead of clicking.
const UNDERRUN_FADE: f32 = 0.995;

/// A stream to the default output device, fed with mono samples.
pub(crate) struct AudioOutput {
    /// Kept alive to keep playing.
    _stream: Stream,
    sender: Sender<Vec<f32>>,
    queued: Arc<AtomicUsize>,
    failed: Arc<AtomicBool>,
    sample_rate: u32,
}

impl AudioOutput {
    /// Opens the default output device at its preferred sample rate.
    pub(crate) fn open() -> Result<Self> {
        let device = cpal::default_host()
            .default_output_device()
            .context("no audio output device")?;
        let supported = device
            .default_output_config()
            .context("cannot query the audio output device")?;
        let config = supported.config();

        let (sender, receiver) = mpsc::channel();
        let queued = Arc::new(AtomicUsize::new(0));
        let failed = Arc::new(AtomicBool::new(false));
        let feeder = Feeder::new(receiver, Arc::clone(&queued));
        let stream = match supported.sample_format() {
            SampleFormat::F32 => build::<f32>(&device, config, feeder, &failed),
            SampleFormat::F64 => build::<f64>(&device, config, feeder, &failed),
            SampleFormat::I16 => build::<i16>(&device, config, feeder, &failed),
            SampleFormat::I32 => build::<i32>(&device, config, feeder, &failed),
            SampleFormat::U16 => build::<u16>(&device, config, feeder, &failed),
            SampleFormat::U8 => build::<u8>(&device, config, feeder, &failed),
            format => bail!("the audio device's {format} samples are not supported"),
        }?;
        stream.play().context("cannot start audio playback")?;

        Ok(Self {
            _stream: stream,
            sender,
            queued,
            failed,
            sample_rate: config.sample_rate,
        })
    }

    /// Returns the device's sample rate.
    pub(crate) fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Returns how many samples are waiting to be played.
    pub(crate) fn queued(&self) -> usize {
        self.queued.load(Ordering::Acquire)
    }

    /// Returns whether the stream broke, for instance because the device
    /// was unplugged.
    pub(crate) fn has_failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }

    /// Queues samples for playback.
    pub(crate) fn play(&self, samples: Vec<f32>) {
        self.queued.fetch_add(samples.len(), Ordering::AcqRel);
        if self.sender.send(samples).is_err() {
            self.failed.store(true, Ordering::Release);
        }
    }
}

fn build<T>(
    device: &Device,
    config: StreamConfig,
    mut feeder: Feeder,
    failed: &Arc<AtomicBool>,
) -> Result<Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels);
    let failed = Arc::clone(failed);
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                for frame in data.chunks_mut(channels) {
                    frame.fill(T::from_sample(feeder.next_sample()));
                }
            },
            move |error| {
                eprintln!("audio: {error}");
                if !matches!(error.kind(), ErrorKind::Xrun | ErrorKind::RealtimeDenied) {
                    failed.store(true, Ordering::Release);
                }
            },
            None,
        )
        .context("cannot open an audio stream")
}

/// Hands the samples queued by the emulator to the audio callback.
struct Feeder {
    receiver: Receiver<Vec<f32>>,
    chunk: Vec<f32>,
    position: usize,
    queued: Arc<AtomicUsize>,
    last: f32,
}

impl Feeder {
    fn new(receiver: Receiver<Vec<f32>>, queued: Arc<AtomicUsize>) -> Self {
        Self {
            receiver,
            chunk: Vec::new(),
            position: 0,
            queued,
            last: 0.0,
        }
    }

    /// Returns the next sample, fading to silence while none is queued.
    fn next_sample(&mut self) -> f32 {
        while self.position == self.chunk.len() {
            let Ok(chunk) = self.receiver.try_recv() else {
                self.last *= UNDERRUN_FADE;
                return self.last;
            };
            self.chunk = chunk;
            self.position = 0;
        }
        self.last = self.chunk[self.position];
        self.position += 1;
        self.queued.fetch_sub(1, Ordering::AcqRel);
        self.last
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feeder_plays_chunks_in_order_then_fades_out() {
        let (sender, receiver) = mpsc::channel();
        let queued = Arc::new(AtomicUsize::new(3));
        let mut feeder = Feeder::new(receiver, Arc::clone(&queued));
        sender.send(vec![0.1, 0.2]).unwrap();
        sender.send(Vec::new()).unwrap();
        sender.send(vec![0.5]).unwrap();

        let played: Vec<f32> = (0..3).map(|_| feeder.next_sample()).collect();
        assert_eq!(played, [0.1, 0.2, 0.5]);
        assert_eq!(queued.load(Ordering::Acquire), 0);

        let faded = feeder.next_sample();
        assert!(faded > 0.0 && faded < 0.5);
    }
}
