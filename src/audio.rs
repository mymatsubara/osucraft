use anyhow::Result;
use rodio::{Decoder, OutputStreamHandle, Sink, Source};
use std::{
    cmp::max,
    fs::File,
    io::{BufReader, Read, Seek},
    path::Path,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

pub struct AudioPlayer {
    sink: Sink,
    execution: Option<DecoderExecution>,
}

struct CustomDecoder<R: Read + Seek> {
    decoder: Decoder<R>,
    samples_played: u32,
    shared_update_rate: u32,
    shared_samples_played: Arc<AtomicU32>,
}

struct DecoderExecution {
    samples_played: Arc<AtomicU32>,
    sample_rate: u32,
    channels: u16,
}

impl AudioPlayer {
    pub fn new(stream_handle: &OutputStreamHandle) -> Result<Self> {
        let sink = Sink::try_new(stream_handle)?;
        sink.set_volume(0.25);

        Ok(Self {
            sink,
            execution: None,
        })
    }

    pub fn set_music(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let file = BufReader::new(File::open(path)?);
        let decoder = Decoder::new(file)?;
        let (decoder, execution) = CustomDecoder::new(decoder)?;

        self.sink.stop();
        self.sink.append(decoder);
        self.execution = Some(execution);

        Ok(())
    }

    pub fn play_time(&self) -> Duration {
        if let Some(execution) = self.execution.as_ref() {
            execution.play_time()
        } else {
            Duration::default()
        }
    }

    pub fn play(&self) {
        self.sink.play()
    }

    pub fn pause(&self) {
        self.sink.pause()
    }

    pub fn stop(&self) {
        self.sink.stop()
    }

    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    pub fn has_finished(&self) -> bool {
        self.sink.empty()
    }
}

impl DecoderExecution {
    fn play_time(&self) -> Duration {
        Duration::from_micros(
            (self.samples_played.load(Ordering::Relaxed) as u64 * 1_000_000)
                / (self.sample_rate as u64 * self.channels as u64),
        )
    }
}

impl<R: Read + Seek> CustomDecoder<R> {
    fn new(decoder: Decoder<R>) -> Result<(Self, DecoderExecution)> {
        let shared_samples_played = Arc::new(AtomicU32::new(0));

        let execution = DecoderExecution {
            sample_rate: decoder.sample_rate(),
            samples_played: shared_samples_played.clone(),
            channels: decoder.channels(),
        };

        let shared_threshold = max(decoder.sample_rate() / 1000, 1);

        Ok((
            Self {
                decoder,
                samples_played: 0,
                shared_samples_played,
                shared_update_rate: shared_threshold,
            },
            execution,
        ))
    }
}

impl<R: Read + Seek> Iterator for CustomDecoder<R> {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        self.samples_played += 1;
        if self.samples_played % self.shared_update_rate == 0 {
            self.shared_samples_played
                .store(self.samples_played, Ordering::Relaxed)
        }

        self.decoder.next()
    }
}

impl<R: Read + Seek> Source for CustomDecoder<R> {
    fn current_frame_len(&self) -> Option<usize> {
        self.decoder.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.decoder.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.decoder.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.decoder.total_duration()
    }
}
