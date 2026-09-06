//! Bounded WAV encoding primitives for the M04 recorder.
//!
//! The writer only operates on a caller-provided `Write + Seek` destination.
//! It does not open paths, create files, or perform realtime scheduling.

use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_CHANNELS: u16 = 2;

#[derive(Debug)]
pub enum PathPolicyError {
    RootNotAbsolute,
    NetworkRoot,
    RootUnavailable(std::io::Error),
    RootNotDirectory,
    RootReparsePoint,
    UnsupportedExtension,
    PathEscapesRoot,
    FileExists,
    Io(std::io::Error),
}

/// Validates a user-approved local recording root and creates non-overwriting
/// recording files beneath its canonical directory.
pub struct RecordingPathPolicy {
    root: PathBuf,
}

impl RecordingPathPolicy {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, PathPolicyError> {
        let root = root.as_ref();
        if !root.is_absolute() {
            return Err(PathPolicyError::RootNotAbsolute);
        }
        let text = root.as_os_str().to_string_lossy();
        if text.starts_with("\\\\") || text.starts_with("//") {
            return Err(PathPolicyError::NetworkRoot);
        }
        let root_metadata =
            std::fs::symlink_metadata(root).map_err(PathPolicyError::RootUnavailable)?;
        if is_reparse_point(&root_metadata) {
            return Err(PathPolicyError::RootReparsePoint);
        }
        let root = root
            .canonicalize()
            .map_err(PathPolicyError::RootUnavailable)?;
        if !root.is_dir() {
            return Err(PathPolicyError::RootNotDirectory);
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn create_file(
        &self,
        session: &str,
        recorder: &str,
        sequence: u64,
        extension: &str,
    ) -> Result<(PathBuf, std::fs::File), PathPolicyError> {
        let extension = sanitize_component(extension);
        if !matches!(extension.to_ascii_lowercase().as_str(), "wav" | "flac") {
            return Err(PathPolicyError::UnsupportedExtension);
        }
        let filename = format!(
            "{}-{}-{}.{}",
            sanitize_component(session),
            sanitize_component(recorder),
            sequence,
            extension
        );
        let path = self.root.join(filename);
        let parent = path
            .parent()
            .and_then(|parent| parent.canonicalize().ok())
            .ok_or(PathPolicyError::PathEscapesRoot)?;
        if !parent.starts_with(&self.root) {
            return Err(PathPolicyError::PathEscapesRoot);
        }
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    PathPolicyError::FileExists
                } else {
                    PathPolicyError::Io(error)
                }
            })?;
        Ok((path, file))
    }
}

#[cfg(windows)]
fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

pub fn sanitize_component(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            character if character.is_control() => '_',
            character => character,
        })
        .collect::<String>();
    while sanitized.ends_with([' ', '.']) {
        sanitized.pop();
    }
    if sanitized.is_empty() {
        sanitized.push('_');
    }
    let stem = sanitized.split('.').next().unwrap_or_default();
    let upper = stem.to_ascii_uppercase();
    if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.starts_with("COM") && upper[3..].parse::<u8>().is_ok())
        || (upper.starts_with("LPT") && upper[3..].parse::<u8>().is_ok())
    {
        sanitized.insert(0, '_');
    }
    sanitized.chars().take(80).collect()
}

/// Caller-owned interleaved samples ready for an off-thread encoder.
#[derive(Debug)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecorderState {
    Idle,
    Armed,
    Recording,
    Paused,
    Stopping,
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameInterval {
    pub start_frame: u64,
    pub end_frame: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordingPart {
    pub index: u32,
    pub start_frame: u64,
    pub end_frame: Option<u64>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RecorderError {
    InvalidTransition {
        state: RecorderState,
        action: &'static str,
    },
    FrameWentBackwards,
}

/// Control-plane recorder state machine. File encoding and queue draining are
/// separate worker responsibilities; this type only records exact boundaries.
pub struct RecorderController {
    state: RecorderState,
    parts: Vec<RecordingPart>,
    pauses: Vec<FrameInterval>,
    pause_start: Option<u64>,
    last_frame: Option<u64>,
}

impl RecorderController {
    pub fn new() -> Self {
        Self {
            state: RecorderState::Idle,
            parts: Vec::new(),
            pauses: Vec::new(),
            pause_start: None,
            last_frame: None,
        }
    }

    pub fn state(&self) -> RecorderState {
        self.state
    }

    pub fn parts(&self) -> &[RecordingPart] {
        &self.parts
    }

    pub fn pause_intervals(&self) -> &[FrameInterval] {
        &self.pauses
    }

    pub fn arm(&mut self) -> Result<(), RecorderError> {
        match self.state {
            RecorderState::Idle => {
                self.state = RecorderState::Armed;
                Ok(())
            }
            RecorderState::Completed => {
                self.parts.clear();
                self.pauses.clear();
                self.pause_start = None;
                self.last_frame = None;
                self.state = RecorderState::Armed;
                Ok(())
            }
            state => Err(RecorderError::InvalidTransition {
                state,
                action: "arm",
            }),
        }
    }

    pub fn start(&mut self, frame: u64) -> Result<(), RecorderError> {
        self.require_frame(frame)?;
        if self.state != RecorderState::Armed {
            return Err(RecorderError::InvalidTransition {
                state: self.state,
                action: "start",
            });
        }
        self.parts.push(RecordingPart {
            index: self.parts.len() as u32,
            start_frame: frame,
            end_frame: None,
        });
        self.state = RecorderState::Recording;
        Ok(())
    }

    pub fn pause(&mut self, frame: u64) -> Result<(), RecorderError> {
        self.require_frame(frame)?;
        if self.state != RecorderState::Recording {
            return Err(RecorderError::InvalidTransition {
                state: self.state,
                action: "pause",
            });
        }
        self.pause_start = Some(frame);
        self.state = RecorderState::Paused;
        Ok(())
    }

    pub fn resume(&mut self, frame: u64) -> Result<(), RecorderError> {
        self.require_frame(frame)?;
        if self.state != RecorderState::Paused {
            return Err(RecorderError::InvalidTransition {
                state: self.state,
                action: "resume",
            });
        }
        let start = self.pause_start.take().unwrap();
        self.pauses.push(FrameInterval {
            start_frame: start,
            end_frame: frame,
        });
        self.state = RecorderState::Recording;
        Ok(())
    }

    pub fn split(&mut self, frame: u64) -> Result<(), RecorderError> {
        self.require_frame(frame)?;
        if !matches!(self.state, RecorderState::Recording | RecorderState::Paused) {
            return Err(RecorderError::InvalidTransition {
                state: self.state,
                action: "split",
            });
        }
        self.close_current_part(frame);
        self.parts.push(RecordingPart {
            index: self.parts.len() as u32,
            start_frame: frame,
            end_frame: None,
        });
        Ok(())
    }

    pub fn stop(&mut self, frame: u64) -> Result<(), RecorderError> {
        self.require_frame(frame)?;
        if !matches!(self.state, RecorderState::Recording | RecorderState::Paused) {
            return Err(RecorderError::InvalidTransition {
                state: self.state,
                action: "stop",
            });
        }
        if let Some(start) = self.pause_start.take() {
            self.pauses.push(FrameInterval {
                start_frame: start,
                end_frame: frame,
            });
        }
        self.state = RecorderState::Stopping;
        self.close_current_part(frame);
        self.state = RecorderState::Completed;
        Ok(())
    }

    pub fn fail(&mut self) {
        self.state = RecorderState::Failed;
        self.pause_start = None;
    }

    fn require_frame(&mut self, frame: u64) -> Result<(), RecorderError> {
        if self.last_frame.is_some_and(|last| frame < last) {
            return Err(RecorderError::FrameWentBackwards);
        }
        self.last_frame = Some(frame);
        Ok(())
    }

    fn close_current_part(&mut self, frame: u64) {
        if let Some(part) = self.parts.last_mut() {
            part.end_frame = Some(frame);
        }
    }
}

impl Default for RecorderController {
    fn default() -> Self {
        Self::new()
    }
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
    NotRecording,
    FrameDiscontinuity { expected: u64, actual: u64 },
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

/// A worker-side WAV recorder that joins queue draining with recorder state.
/// The worker owns the encoder; callers retain ownership of the destination.
pub struct WavRecorder<W> {
    writer: WavWriter<W>,
    controller: RecorderController,
    next_frame: Option<u64>,
}

impl<W: Write + Seek> WavRecorder<W> {
    pub fn new(writer: WavWriter<W>) -> Self {
        Self {
            writer,
            controller: RecorderController::new(),
            next_frame: None,
        }
    }

    pub fn state(&self) -> RecorderState {
        self.controller.state()
    }

    pub fn controller(&self) -> &RecorderController {
        &self.controller
    }

    pub fn arm(&mut self) -> Result<(), RecorderError> {
        self.controller.arm()
    }

    pub fn start(&mut self, frame: u64) -> Result<(), RecorderError> {
        self.controller.start(frame)?;
        self.next_frame = Some(frame);
        Ok(())
    }

    pub fn pause(&mut self, frame: u64) -> Result<(), RecorderError> {
        self.controller.pause(frame)
    }

    pub fn resume(&mut self, frame: u64) -> Result<(), RecorderError> {
        self.controller.resume(frame)?;
        self.next_frame = Some(frame);
        Ok(())
    }

    pub fn split(&mut self, frame: u64) -> Result<(), RecorderError> {
        self.controller.split(frame)
    }

    pub fn stop(&mut self, frame: u64) -> Result<(), RecorderError> {
        self.controller.stop(frame)
    }

    pub fn drain_queue(
        &mut self,
        queue: &RecordingQueue,
        maximum_chunks: usize,
    ) -> Result<usize, RecordingError> {
        if self.controller.state() != RecorderState::Recording {
            return Err(RecordingError::NotRecording);
        }
        let mut drained = 0;
        while drained < maximum_chunks {
            let Some(chunk) = queue.try_pop() else {
                break;
            };
            let expected = self.next_frame.unwrap_or(chunk.start_frame);
            if chunk.start_frame != expected {
                return Err(RecordingError::FrameDiscontinuity {
                    expected,
                    actual: chunk.start_frame,
                });
            }
            let frames = chunk.samples.len() / usize::from(self.writer.channels);
            self.writer.write_interleaved(&chunk.samples)?;
            self.next_frame = Some(expected + frames as u64);
            drained += 1;
        }
        Ok(drained)
    }

    pub fn finish(self) -> Result<W, RecordingError> {
        if self.controller.state() != RecorderState::Completed {
            return Err(RecordingError::NotRecording);
        }
        self.writer.finish()
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

    #[test]
    fn recorder_tracks_pause_and_split_frame_boundaries() {
        let mut recorder = RecorderController::new();
        assert!(matches!(
            recorder.start(0),
            Err(RecorderError::InvalidTransition {
                state: RecorderState::Idle,
                action: "start"
            })
        ));
        recorder.arm().unwrap();
        recorder.start(100).unwrap();
        recorder.pause(200).unwrap();
        recorder.resume(240).unwrap();
        recorder.split(300).unwrap();
        recorder.stop(400).unwrap();
        assert_eq!(recorder.state(), RecorderState::Completed);
        assert_eq!(
            recorder.pause_intervals(),
            &[FrameInterval {
                start_frame: 200,
                end_frame: 240
            }]
        );
        assert_eq!(recorder.parts()[0].end_frame, Some(300));
        assert_eq!(recorder.parts()[1].start_frame, 300);
        assert_eq!(recorder.parts()[1].end_frame, Some(400));
    }

    #[test]
    fn recorder_rejects_backwards_frames_and_restarts_with_new_parts() {
        let mut recorder = RecorderController::new();
        recorder.arm().unwrap();
        recorder.start(10).unwrap();
        assert_eq!(recorder.pause(9), Err(RecorderError::FrameWentBackwards));
        recorder.stop(20).unwrap();
        recorder.arm().unwrap();
        recorder.start(5).unwrap();
        assert_eq!(recorder.parts().len(), 1);
        assert_eq!(recorder.parts()[0].start_frame, 5);
    }

    #[test]
    fn wav_recorder_drains_contiguous_chunks_and_finalizes() {
        let writer =
            WavWriter::new(Cursor::new(Vec::new()), WavFormat::Pcm16, 1, 48_000, false).unwrap();
        let mut recorder = WavRecorder::new(writer);
        let queue = RecordingQueue::new(2).unwrap();
        queue
            .try_push(RecordingChunk {
                start_frame: 10,
                samples: vec![0.0, 0.5],
            })
            .unwrap();
        queue
            .try_push(RecordingChunk {
                start_frame: 12,
                samples: vec![-0.5],
            })
            .unwrap();
        recorder.arm().unwrap();
        recorder.start(10).unwrap();
        assert_eq!(recorder.drain_queue(&queue, 1).unwrap(), 1);
        assert_eq!(recorder.drain_queue(&queue, 8).unwrap(), 1);
        recorder.stop(13).unwrap();
        let output = recorder.finish().unwrap().into_inner();
        assert_eq!(u32::from_le_bytes(output[40..44].try_into().unwrap()), 6);
    }

    #[test]
    fn path_policy_sanitizes_components_and_never_overwrites() {
        let root =
            std::env::temp_dir().join(format!("audiorouter-recording-path-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        assert_eq!(sanitize_component("CON:take?.wav"), "CON_take_.wav");
        let policy = RecordingPathPolicy::new(&root).unwrap();
        assert!(matches!(
            policy.create_file("voice", "main", 0, "mp3"),
            Err(PathPolicyError::UnsupportedExtension)
        ));
        let (path, _file) = policy.create_file("voice/main", "CON", 1, "wav").unwrap();
        assert!(path.starts_with(policy.root()));
        assert!(matches!(
            policy.create_file("voice/main", "CON", 1, "wav"),
            Err(PathPolicyError::FileExists)
        ));
        drop(_file);
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn path_policy_rejects_symlink_roots() {
        let base = std::env::temp_dir().join(format!(
            "audiorouter-recording-reparse-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir(&base).unwrap();
        let target = base.join("target");
        let link = base.join("link");
        std::fs::create_dir(&target).unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(matches!(
            RecordingPathPolicy::new(&link),
            Err(PathPolicyError::RootReparsePoint)
        ));
        let _ = std::fs::remove_dir_all(base);
    }
}
