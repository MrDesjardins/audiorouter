//! Bounded WAV encoding primitives for the M04 recorder.
//!
//! The writer only operates on a caller-provided `Write + Seek` destination.
//! It does not open paths, create files, or perform realtime scheduling.

use std::io::{Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_CHANNELS: u16 = 2;

/// Caller-owned interleaved samples ready for an off-thread encoder.
pub struct RecordingChunk {
    pub start_frame: u64,
    pub samples: Vec<f32>,
}

/// Fixed-capacity, nonblocking handoff from audio processing to recording.
/// Chunks must be prepared by the caller; queue operations do not allocate,
/// encode, touch files, or wait for a consumer.
pub struct RecordingQueue {
    chunks: crossbeam_queue::ArrayQueue<RecordingChunk>,
    overruns: AtomicU64,
}

impl RecordingQueue {
    pub fn new(capacity: usize) -> Result<Self, RecordingError> {
        if capacity == 0 {
            return Err(RecordingError::InvalidQueueCapacity);
        }
        Ok(Self {
            chunks: crossbeam_queue::ArrayQueue::new(capacity),
            overruns: AtomicU64::new(0),
        })
    }

    pub fn capacity(&self) -> usize {
        self.chunks.capacity()
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    pub fn overruns(&self) -> u64 {
        self.overruns.load(Ordering::Relaxed)
    }

    pub fn try_push(&self, chunk: RecordingChunk) -> Result<(), RecordingChunk> {
        match self.chunks.push(chunk) {
            Ok(()) => Ok(()),
            Err(chunk) => {
                self.overruns.fetch_add(1, Ordering::Relaxed);
                Err(chunk)
            }
        }
    }

    pub fn try_pop(&self) -> Option<RecordingChunk> {
        self.chunks.pop()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WavFormat {
    Pcm16,
    Pcm24,
    Float32,
}

impl WavFormat {
    fn bits(self) -> u16 {
        match self {
            Self::Pcm16 => 16,
            Self::Pcm24 => 24,
            Self::Float32 => 32,
        }
    }

    fn tag(self) -> u16 {
        match self {
            Self::Float32 => 3,
            Self::Pcm16 | Self::Pcm24 => 1,
        }
    }
}

#[derive(Debug)]
pub enum RecordingError {
    InvalidSampleRate,
    InvalidChannels,
    InvalidSampleCount,
    TooManyFrames,
    InvalidQueueCapacity,
    Io(std::io::Error),
}

impl From<std::io::Error> for RecordingError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// A seekable RIFF/WAVE writer with a finalized data length and frame count.
pub struct WavWriter<W> {
    output: W,
    format: WavFormat,
    channels: u16,
    sample_rate: u32,
    frames: u64,
    data_bytes: u64,
    dither: bool,
    rng: u64,
}

impl<W: Write + Seek> WavWriter<W> {
    pub fn new(
        mut output: W,
        format: WavFormat,
        channels: u16,
        sample_rate: u32,
        dither: bool,
    ) -> Result<Self, RecordingError> {
        if !matches!(sample_rate, 44_100 | 48_000) {
            return Err(RecordingError::InvalidSampleRate);
        }
        if !(1..=MAX_CHANNELS).contains(&channels) {
            return Err(RecordingError::InvalidChannels);
        }
        write_header(&mut output, format, channels, sample_rate, 0, 0)?;
        Ok(Self {
            output,
            format,
            channels,
            sample_rate,
            frames: 0,
            data_bytes: 0,
            dither,
            rng: 0x9e37_79b9_7f4a_7c15,
        })
    }

    pub fn frames(&self) -> u64 {
        self.frames
    }

    pub fn data_bytes(&self) -> u64 {
        self.data_bytes
    }

    pub fn write_interleaved(&mut self, samples: &[f32]) -> Result<u64, RecordingError> {
        if samples.len() % usize::from(self.channels) != 0 {
            return Err(RecordingError::InvalidSampleCount);
        }
        let frames = samples.len() / usize::from(self.channels);
        let added = u64::try_from(frames).map_err(|_| RecordingError::TooManyFrames)?;
        self.frames = self
            .frames
            .checked_add(added)
            .ok_or(RecordingError::TooManyFrames)?;
        for sample in samples {
            match self.format {
                WavFormat::Pcm16 => {
                    let value = quantize(*sample, self.dither, &mut self.rng, 32767.0);
                    self.output.write_all(&(value as i16).to_le_bytes())?;
                    self.data_bytes += 2;
                }
                WavFormat::Pcm24 => {
                    let value = quantize(*sample, self.dither, &mut self.rng, 8_388_607.0) as i32;
                    let bytes = value.to_le_bytes();
                    self.output.write_all(&bytes[..3])?;
                    self.data_bytes += 3;
                }
                WavFormat::Float32 => {
                    let value = if sample.is_finite() { *sample } else { 0.0 };
                    self.output.write_all(&value.to_le_bytes())?;
                    self.data_bytes += 4;
                }
            }
        }
        Ok(added)
    }

    pub fn finish(mut self) -> Result<W, RecordingError> {
        let data_size =
            u32::try_from(self.data_bytes).map_err(|_| RecordingError::TooManyFrames)?;
        let riff_size = 36u32
            .checked_add(data_size)
            .ok_or(RecordingError::TooManyFrames)?;
        self.output.seek(SeekFrom::Start(0))?;
        write_header(
            &mut self.output,
            self.format,
            self.channels,
            self.sample_rate,
            riff_size,
            data_size,
        )?;
        self.output.seek(SeekFrom::End(0))?;
        Ok(self.output)
    }
}

fn quantize(sample: f32, dither: bool, rng: &mut u64, scale: f32) -> f32 {
    let finite = if sample.is_finite() { sample } else { 0.0 };
    let noise = if dither {
        let a = next_unit(rng);
        let b = next_unit(rng);
        (a - b) * 0.5 / scale
    } else {
        0.0
    };
    (finite.clamp(-1.0, 1.0) + noise)
        .mul_add(scale, 0.0)
        .round()
        .clamp(-scale - 1.0, scale) // reserve the negative full-scale value safely
}

fn next_unit(rng: &mut u64) -> f32 {
    *rng ^= *rng << 13;
    *rng ^= *rng >> 7;
    *rng ^= *rng << 17;
    (*rng as f64 / u64::MAX as f64) as f32
}

fn write_header<W: Write>(
    output: &mut W,
    format: WavFormat,
    channels: u16,
    sample_rate: u32,
    riff_size: u32,
    data_size: u32,
) -> Result<(), RecordingError> {
    let bits = format.bits();
    let block_align = channels * (bits / 8);
    let byte_rate = sample_rate * u32::from(block_align);
    output.write_all(b"RIFF")?;
    output.write_all(&riff_size.to_le_bytes())?;
    output.write_all(b"WAVEfmt ")?;
    output.write_all(&16u32.to_le_bytes())?;
    output.write_all(&format.tag().to_le_bytes())?;
    output.write_all(&channels.to_le_bytes())?;
    output.write_all(&sample_rate.to_le_bytes())?;
    output.write_all(&byte_rate.to_le_bytes())?;
    output.write_all(&block_align.to_le_bytes())?;
    output.write_all(&bits.to_le_bytes())?;
    output.write_all(b"data")?;
    output.write_all(&data_size.to_le_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn pcm16_finalizes_header_and_counts_frames() {
        let writer =
            WavWriter::new(Cursor::new(Vec::new()), WavFormat::Pcm16, 2, 48_000, false).unwrap();
        let output = writer.finish().unwrap().into_inner();
        assert_eq!(&output[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(output[4..8].try_into().unwrap()), 36);
        assert_eq!(u32::from_le_bytes(output[40..44].try_into().unwrap()), 0);
    }

    #[test]
    fn pcm24_writes_three_bytes_per_sample_and_sanitizes_nonfinite() {
        let mut writer =
            WavWriter::new(Cursor::new(Vec::new()), WavFormat::Pcm24, 1, 44_100, false).unwrap();
        assert_eq!(
            writer
                .write_interleaved(&[f32::NAN, 0.5, f32::INFINITY])
                .unwrap(),
            3
        );
        assert_eq!(writer.frames(), 3);
        assert_eq!(writer.data_bytes(), 9);
        let output = writer.finish().unwrap().into_inner();
        assert_eq!(u32::from_le_bytes(output[40..44].try_into().unwrap()), 9);
        assert_eq!(output.len(), 53);
    }

    #[test]
    fn invalid_shape_and_rate_are_rejected() {
        assert!(matches!(
            WavWriter::new(Cursor::new(Vec::new()), WavFormat::Pcm16, 3, 48_000, false),
            Err(RecordingError::InvalidChannels)
        ));
        assert!(matches!(
            WavWriter::new(Cursor::new(Vec::new()), WavFormat::Pcm16, 1, 96_000, false),
            Err(RecordingError::InvalidSampleRate)
        ));
        let mut writer =
            WavWriter::new(Cursor::new(Vec::new()), WavFormat::Pcm16, 2, 48_000, false).unwrap();
        assert!(matches!(
            writer.write_interleaved(&[0.0]),
            Err(RecordingError::InvalidSampleCount)
        ));
    }

    #[test]
    fn recording_queue_is_bounded_and_reports_overruns() {
        let queue = RecordingQueue::new(1).unwrap();
        let first = RecordingChunk {
            start_frame: 4,
            samples: vec![0.0, 0.5],
        };
        let second = RecordingChunk {
            start_frame: 5,
            samples: vec![0.25, 0.75],
        };
        assert!(queue.try_push(first).is_ok());
        let returned = queue.try_push(second).unwrap_err();
        assert_eq!(returned.start_frame, 5);
        assert_eq!(queue.overruns(), 1);
        assert_eq!(queue.try_pop().unwrap().start_frame, 4);
        assert!(queue.try_pop().is_none());
        assert!(matches!(
            RecordingQueue::new(0),
            Err(RecordingError::InvalidQueueCapacity)
        ));
    }
}
