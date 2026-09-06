//! Bounded WAV encoding primitives for the M04 recorder.
//!
//! The writer only operates on a caller-provided `Write + Seek` destination.
//! It does not open paths, create files, or perform realtime scheduling.

use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom, Write};
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RecorderState {
    Idle,
    Armed,
    Recording,
    Paused,
    Stopping,
    Completed,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FrameInterval {
    pub start_frame: u64,
    pub end_frame: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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
    InvalidCheckpoint,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecorderCheckpoint {
    pub version: u32,
    pub state: RecorderState,
    pub parts: Vec<RecordingPart>,
    pub pauses: Vec<FrameInterval>,
    pub pause_start: Option<u64>,
    pub last_frame: Option<u64>,
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

    /// Returns a versioned control-plane snapshot suitable for a crash journal.
    /// It contains boundaries only; queued audio and file handles are never
    /// serialized.
    pub fn checkpoint(&self) -> RecorderCheckpoint {
        RecorderCheckpoint {
            version: 1,
            state: self.state,
            parts: self.parts.clone(),
            pauses: self.pauses.clone(),
            pause_start: self.pause_start,
            last_frame: self.last_frame,
        }
    }

    pub fn restore(checkpoint: RecorderCheckpoint) -> Result<Self, RecorderError> {
        if checkpoint.version != 1
            || checkpoint.parts.iter().enumerate().any(|(index, part)| {
                part.index != index as u32
                    || part.end_frame.is_some_and(|end| end < part.start_frame)
            })
            || checkpoint
                .pauses
                .iter()
                .any(|interval| interval.end_frame < interval.start_frame)
            || (checkpoint.state == RecorderState::Paused) != checkpoint.pause_start.is_some()
            || checkpoint.state != RecorderState::Paused && checkpoint.pause_start.is_some()
        {
            return Err(RecorderError::InvalidCheckpoint);
        }
        if let Some(last_frame) = checkpoint.last_frame {
            if checkpoint.parts.iter().any(|part| {
                part.start_frame > last_frame || part.end_frame.is_some_and(|end| end > last_frame)
            }) || checkpoint
                .pauses
                .iter()
                .any(|interval| interval.end_frame > last_frame)
                || checkpoint
                    .pause_start
                    .is_some_and(|start| start > last_frame)
            {
                return Err(RecorderError::InvalidCheckpoint);
            }
        }
        Ok(Self {
            state: checkpoint.state,
            parts: checkpoint.parts,
            pauses: checkpoint.pauses,
            pause_start: checkpoint.pause_start,
            last_frame: checkpoint.last_frame,
        })
    }

    pub fn checkpoint_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.checkpoint())
    }

    pub fn restore_json(document: &str) -> Result<Self, RecorderError> {
        let checkpoint =
            serde_json::from_str(document).map_err(|_| RecorderError::InvalidCheckpoint)?;
        Self::restore(checkpoint)
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

    /// Advances the durable boundary after a worker has committed audio.
    /// This does not change lifecycle state or part boundaries.
    pub fn advance(&mut self, frame: u64) -> Result<(), RecorderError> {
        if self.state != RecorderState::Recording {
            return Err(RecorderError::InvalidTransition {
                state: self.state,
                action: "advance",
            });
        }
        self.require_frame(frame)
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
    Controller(RecorderError),
    InvalidMetadata,
    InvalidWav,
    FlacEncode(String),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WavFileInfo {
    pub format: WavFormat,
    pub channels: u16,
    pub sample_rate: u32,
    pub frames: u64,
    pub data_bytes: u64,
    pub file_bytes: u64,
}

/// Optional RIFF INFO tags written during WAV finalization. Values are kept
/// bounded and UTF-8; the writer never accepts control characters.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WavMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub comment: Option<String>,
}

impl WavMetadata {
    fn validate(&self) -> Result<(), RecordingError> {
        for value in [&self.title, &self.artist, &self.comment]
            .into_iter()
            .flatten()
        {
            if value.chars().count() > 256 || value.chars().any(|character| character.is_control())
            {
                return Err(RecordingError::InvalidMetadata);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlacFileInfo {
    pub channels: u8,
    pub sample_rate: u32,
    pub bits_per_sample: u8,
    pub frames: u64,
    pub file_bytes: u64,
}

const STREAMING_FLAC_BLOCK_FRAMES: usize = 4096;

/// Incremental seekable FLAC writer.
///
/// Audio is emitted as bounded verbatim FLAC frames while writes arrive.
/// STREAMINFO is patched on finish with the final frame count and frame-size
/// bounds. Verbatim subframes intentionally trade compression ratio for a
/// small, deterministic writer that is safe for an off-thread recorder.
pub struct StreamingFlacWriter<W> {
    output: W,
    streaminfo_start: u64,
    channels: u16,
    bits_per_sample: u8,
    streaminfo: [u8; 34],
    frames: u64,
    minimum_frame_size: u32,
    maximum_frame_size: u32,
    dither: bool,
    rng: u64,
}

impl<W: Write + Seek> StreamingFlacWriter<W> {
    pub fn new(
        output: W,
        channels: u16,
        sample_rate: u32,
        bits_per_sample: u8,
        dither: bool,
    ) -> Result<Self, RecordingError> {
        Self::new_with_metadata(
            output,
            channels,
            sample_rate,
            bits_per_sample,
            dither,
            &WavMetadata::default(),
        )
    }

    pub fn new_with_metadata(
        mut output: W,
        channels: u16,
        sample_rate: u32,
        bits_per_sample: u8,
        dither: bool,
        metadata: &WavMetadata,
    ) -> Result<Self, RecordingError> {
        validate_format(WavFormat::Pcm16, channels, sample_rate)?;
        if !matches!(bits_per_sample, 16 | 24) {
            return Err(RecordingError::InvalidSampleCount);
        }
        metadata.validate()?;
        output.write_all(b"fLaC")?;
        output.write_all(&[0x00, 0, 0, 0x22])?;
        let streaminfo_start = 8;
        let mut streaminfo = [0u8; 34];
        streaminfo[0..2].copy_from_slice(&(STREAMING_FLAC_BLOCK_FRAMES as u16).to_be_bytes());
        streaminfo[2..4].copy_from_slice(&(STREAMING_FLAC_BLOCK_FRAMES as u16).to_be_bytes());
        let packed = (u64::from(sample_rate) << 44)
            | (u64::from(channels - 1) << 41)
            | (u64::from(bits_per_sample - 1) << 36);
        streaminfo[10..18].copy_from_slice(&packed.to_be_bytes());
        output.write_all(&streaminfo)?;
        let comments = encode_vorbis_comments(metadata)?;
        let length = u32::try_from(comments.len()).map_err(|_| RecordingError::TooManyFrames)?;
        if length > 0x00ff_ffff {
            return Err(RecordingError::TooManyFrames);
        }
        output.write_all(&[
            0x84,
            (length >> 16) as u8,
            (length >> 8) as u8,
            length as u8,
        ])?;
        output.write_all(&comments)?;
        Ok(Self {
            output,
            streaminfo_start,
            channels,
            bits_per_sample,
            streaminfo,
            frames: 0,
            minimum_frame_size: u32::MAX,
            maximum_frame_size: 0,
            dither,
            rng: 0x9e37_79b9_7f4a_7c15,
        })
    }

    pub fn frames(&self) -> u64 {
        self.frames
    }

    pub fn write_interleaved(&mut self, samples: &[f32]) -> Result<u64, RecordingError> {
        let channels = usize::from(self.channels);
        if samples.len() % channels != 0 {
            return Err(RecordingError::InvalidSampleCount);
        }
        let mut offset = 0;
        while offset < samples.len() {
            let remaining_frames = (samples.len() - offset) / channels;
            let frame_count = remaining_frames.min(STREAMING_FLAC_BLOCK_FRAMES);
            let frame = self.encode_frame(&samples[offset..offset + frame_count * channels])?;
            let frame_size =
                u32::try_from(frame.len()).map_err(|_| RecordingError::TooManyFrames)?;
            self.minimum_frame_size = self.minimum_frame_size.min(frame_size);
            self.maximum_frame_size = self.maximum_frame_size.max(frame_size);
            self.output.write_all(&frame)?;
            self.frames = self
                .frames
                .checked_add(frame_count as u64)
                .ok_or(RecordingError::TooManyFrames)?;
            offset += frame_count * channels;
        }
        Ok((samples.len() / channels) as u64)
    }

    pub fn finish(mut self) -> Result<W, RecordingError> {
        let end = self.output.stream_position()?;
        let minimum = if self.minimum_frame_size == u32::MAX {
            0
        } else {
            self.minimum_frame_size
        };
        self.streaminfo[4..7].copy_from_slice(&minimum.to_be_bytes()[1..]);
        self.streaminfo[7..10].copy_from_slice(&self.maximum_frame_size.to_be_bytes()[1..]);
        let mut packed = u64::from_be_bytes(self.streaminfo[10..18].try_into().unwrap());
        packed = (packed & !((1u64 << 36) - 1)) | self.frames.min((1u64 << 36) - 1);
        self.streaminfo[10..18].copy_from_slice(&packed.to_be_bytes());
        self.output.seek(SeekFrom::Start(self.streaminfo_start))?;
        self.output.write_all(&self.streaminfo)?;
        self.output.seek(SeekFrom::Start(end))?;
        Ok(self.output)
    }

    fn encode_frame(&mut self, samples: &[f32]) -> Result<Vec<u8>, RecordingError> {
        let channels = usize::from(self.channels);
        let frame_count = samples.len() / channels;
        let mut header = BitWriter::new();
        header.write_bits(0x3ffe, 14);
        header.write_bits(0, 1);
        header.write_bits(0, 1);
        header.write_bits(7, 4);
        header.write_bits(0, 4);
        header.write_bits((channels - 1) as u64, 4);
        header.write_bits(0, 3);
        header.write_bits(0, 1);
        write_utf8_number(&mut header, self.frames);
        header.write_bits((frame_count - 1) as u64, 16);
        header.align();
        let mut frame = header.into_bytes();
        frame.push(crc8(&frame));
        let scale = if self.bits_per_sample == 16 {
            32_767.0
        } else {
            8_388_607.0
        };
        for channel in 0..channels {
            let mut subframe = BitWriter::new();
            subframe.write_bits(0, 1);
            subframe.write_bits(1, 6);
            subframe.write_bits(0, 1);
            for frame_index in 0..frame_count {
                let sample = samples[frame_index * channels + channel];
                let value = if sample.is_finite() {
                    quantize(sample, self.dither, &mut self.rng, scale) as i32
                } else {
                    0
                };
                subframe.write_signed(value as i64, u32::from(self.bits_per_sample));
            }
            subframe.align();
            frame.extend_from_slice(&subframe.into_bytes());
        }
        let checksum = crc16(&frame);
        frame.extend_from_slice(&checksum.to_be_bytes());
        Ok(frame)
    }
}

fn encode_vorbis_comments(metadata: &WavMetadata) -> Result<Vec<u8>, RecordingError> {
    let mut comments = Vec::new();
    let vendor = b"AudioRouter";
    comments.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    comments.extend_from_slice(vendor);
    let values = [
        ("TITLE", metadata.title.as_deref()),
        ("ARTIST", metadata.artist.as_deref()),
        ("COMMENT", metadata.comment.as_deref()),
    ];
    let count = values.iter().filter(|(_, value)| value.is_some()).count() as u32;
    comments.extend_from_slice(&count.to_le_bytes());
    for (key, value) in values
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| (key, value)))
    {
        let entry = format!("{key}={value}");
        let length = u32::try_from(entry.len()).map_err(|_| RecordingError::TooManyFrames)?;
        comments.extend_from_slice(&length.to_le_bytes());
        comments.extend_from_slice(entry.as_bytes());
    }
    Ok(comments)
}

struct BitWriter {
    bytes: Vec<u8>,
    current: u8,
    used: u8,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            current: 0,
            used: 0,
        }
    }

    fn write_bits(&mut self, value: u64, count: u32) {
        for bit in (0..count).rev() {
            self.current = (self.current << 1) | (((value >> bit) & 1) as u8);
            self.used += 1;
            if self.used == 8 {
                self.bytes.push(self.current);
                self.current = 0;
                self.used = 0;
            }
        }
    }

    fn write_signed(&mut self, value: i64, count: u32) {
        self.write_bits(value as u64, count);
    }

    fn align(&mut self) {
        if self.used != 0 {
            self.current <<= 8 - self.used;
            self.bytes.push(self.current);
            self.current = 0;
            self.used = 0;
        }
    }

    fn into_bytes(mut self) -> Vec<u8> {
        self.align();
        self.bytes
    }
}

fn write_utf8_number(writer: &mut BitWriter, value: u64) {
    let mut bytes = [0u8; 7];
    let count = if value < 0x80 {
        1
    } else if value < 0x800 {
        2
    } else if value < 0x10000 {
        3
    } else if value < 0x200000 {
        4
    } else if value < 0x4000000 {
        5
    } else {
        6
    };
    let mut remaining = value;
    for index in (1..count).rev() {
        bytes[index] = 0x80 | (remaining as u8 & 0x3f);
        remaining >>= 6;
    }
    bytes[0] = match count {
        1 => remaining as u8,
        2 => 0xc0 | remaining as u8 & 0x1f,
        3 => 0xe0 | remaining as u8 & 0x0f,
        4 => 0xf0 | remaining as u8 & 0x07,
        5 => 0xf8 | remaining as u8 & 0x03,
        _ => 0xfc | remaining as u8 & 0x01,
    };
    for byte in bytes.into_iter().take(count) {
        writer.write_bits(u64::from(byte), 8);
    }
}

fn crc8(bytes: &[u8]) -> u8 {
    let mut crc = 0;
    for byte in bytes {
        crc ^= *byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn crc16(bytes: &[u8]) -> u16 {
    let mut crc = 0;
    for byte in bytes {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x8005
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Batch FLAC encoder for completed in-memory segments. It is intentionally
/// separate from `WavRecorder`: the current dependency API returns one encoded
/// buffer, so it must not be used for unbounded live recording yet.
pub struct FlacBufferEncoder {
    channels: usize,
    sample_rate: u32,
    bits_per_sample: u8,
    samples: Vec<i32>,
}

/// The batch encoder is not the live recorder. Keep its temporary sample
/// buffer bounded until a true incremental FLAC worker is available.
pub const MAX_FLAC_BUFFER_FRAMES: u64 = 48_000 * 60 * 10;

impl FlacBufferEncoder {
    pub fn new(
        channels: usize,
        sample_rate: u32,
        bits_per_sample: u8,
    ) -> Result<Self, RecordingError> {
        if !(1..=2).contains(&channels) {
            return Err(RecordingError::InvalidChannels);
        }
        if !matches!(sample_rate, 44_100 | 48_000) {
            return Err(RecordingError::InvalidSampleRate);
        }
        if !matches!(bits_per_sample, 16 | 24) {
            return Err(RecordingError::InvalidSampleCount);
        }
        Ok(Self {
            channels,
            sample_rate,
            bits_per_sample,
            samples: Vec::new(),
        })
    }

    pub fn frames(&self) -> u64 {
        (self.samples.len() / self.channels) as u64
    }

    pub fn write_interleaved(&mut self, samples: &[f32]) -> Result<u64, RecordingError> {
        if samples.len() % self.channels != 0 {
            return Err(RecordingError::InvalidSampleCount);
        }
        let added_frames = u64::try_from(samples.len() / self.channels)
            .map_err(|_| RecordingError::TooManyFrames)?;
        if self.frames().saturating_add(added_frames) > MAX_FLAC_BUFFER_FRAMES {
            return Err(RecordingError::TooManyFrames);
        }
        let scale = if self.bits_per_sample == 16 {
            32_767.0
        } else {
            8_388_607.0
        };
        self.samples.extend(samples.iter().map(|sample| {
            let value = if sample.is_finite() { *sample } else { 0.0 };
            (value.clamp(-1.0, 1.0) * scale).round() as i32
        }));
        Ok(added_frames)
    }

    pub fn finish(self) -> Result<Vec<u8>, RecordingError> {
        self.finish_with_metadata(&WavMetadata::default())
    }

    pub fn finish_with_metadata(self, metadata: &WavMetadata) -> Result<Vec<u8>, RecordingError> {
        metadata.validate()?;
        if self.samples.is_empty() {
            return Err(RecordingError::InvalidSampleCount);
        }
        let mut planar = vec![Vec::with_capacity(self.frames() as usize); self.channels];
        for frame in self.samples.chunks_exact(self.channels) {
            for (channel, sample) in frame.iter().enumerate() {
                planar[channel].push(*sample);
            }
        }
        let encoded = flac_io::encode(&flac_io::FlacAudio {
            sample_rate: self.sample_rate,
            channels: self.channels as u8,
            bits_per_sample: self.bits_per_sample,
            samples: planar,
        })
        .map_err(|error| RecordingError::FlacEncode(error.to_string()))?;
        add_flac_comments(encoded, metadata)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordingFileStatus {
    Present(WavFileInfo),
    FlacPresent(FlacFileInfo),
    Missing,
    Invalid,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RecordingLibraryError {
    PathOutsideRoot,
    InvalidPath,
    NotFound,
    Io(std::io::ErrorKind),
    InvalidMetadata,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecordingMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub comment: Option<String>,
}

impl RecordingMetadata {
    pub fn validate(&self) -> Result<(), RecordingLibraryError> {
        for value in [&self.title, &self.artist, &self.comment]
            .into_iter()
            .flatten()
        {
            if value.chars().count() > 256 || value.chars().any(|character| character.is_control())
            {
                return Err(RecordingLibraryError::InvalidMetadata);
            }
        }
        Ok(())
    }
}

/// Reads only RIFF INFO metadata from a WAV file. Audio data chunks are
/// skipped without being loaded, and malformed/oversized tag values are
/// ignored so a valid recording remains indexable.
pub fn read_wav_metadata(path: impl AsRef<Path>) -> Result<RecordingMetadata, RecordingError> {
    let mut file = std::fs::File::open(path)?;
    let mut header = [0u8; 12];
    file.read_exact(&mut header)?;
    if &header[..4] != b"RIFF" || &header[8..] != b"WAVE" {
        return Err(RecordingError::InvalidWav);
    }
    let mut metadata = RecordingMetadata::default();
    loop {
        let mut chunk_header = [0u8; 8];
        match file.read_exact(&mut chunk_header) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.into()),
        }
        let length = u64::from(u32::from_le_bytes(chunk_header[4..].try_into().unwrap()));
        if &chunk_header[..4] == b"LIST" && length <= 64 * 1024 {
            let mut payload = vec![0u8; length as usize];
            file.read_exact(&mut payload)?;
            parse_info_metadata(&payload, &mut metadata);
        } else {
            file.seek(SeekFrom::Current(
                i64::try_from(length).map_err(|_| RecordingError::TooManyFrames)?
                    + i64::try_from(length % 2).unwrap(),
            ))?;
        }
    }
    Ok(metadata)
}

fn parse_info_metadata(payload: &[u8], metadata: &mut RecordingMetadata) {
    if payload.len() < 4 || &payload[..4] != b"INFO" {
        return;
    }
    let mut offset = 4;
    while offset + 8 <= payload.len() {
        let id = &payload[offset..offset + 4];
        let length =
            u32::from_le_bytes(payload[offset + 4..offset + 8].try_into().unwrap()) as usize;
        offset += 8;
        if length > payload.len().saturating_sub(offset) {
            return;
        }
        let value = payload[offset..offset + length]
            .split(|byte| *byte == 0)
            .next()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .filter(|value| value.chars().count() <= 256 && !value.chars().any(|c| c.is_control()))
            .map(str::to_owned);
        match (id, value) {
            (b"INAM", Some(value)) => metadata.title = Some(value),
            (b"IART", Some(value)) => metadata.artist = Some(value),
            (b"ICMT", Some(value)) => metadata.comment = Some(value),
            _ => {}
        }
        offset += length + length % 2;
    }
}

/// Reads only Vorbis comments from FLAC metadata blocks. Audio frames are
/// skipped entirely, and malformed comments are ignored without invalidating
/// the otherwise indexable recording.
pub fn read_flac_metadata(path: impl AsRef<Path>) -> Result<RecordingMetadata, RecordingError> {
    let mut file = std::fs::File::open(path)?;
    let mut signature = [0u8; 4];
    file.read_exact(&mut signature)?;
    if &signature != b"fLaC" {
        return Err(RecordingError::FlacEncode("invalid FLAC signature".into()));
    }
    let mut metadata = RecordingMetadata::default();
    loop {
        let mut block_header = [0u8; 4];
        file.read_exact(&mut block_header)?;
        let is_last = block_header[0] & 0x80 != 0;
        let block_type = block_header[0] & 0x7f;
        let length = (usize::from(block_header[1]) << 16)
            | (usize::from(block_header[2]) << 8)
            | usize::from(block_header[3]);
        if block_type == 4 && length <= 64 * 1024 {
            let mut payload = vec![0u8; length];
            file.read_exact(&mut payload)?;
            parse_vorbis_comments(&payload, &mut metadata);
        } else {
            file.seek(SeekFrom::Current(
                i64::try_from(length).map_err(|_| RecordingError::TooManyFrames)?,
            ))?;
        }
        if is_last {
            break;
        }
    }
    Ok(metadata)
}

fn parse_vorbis_comments(payload: &[u8], metadata: &mut RecordingMetadata) {
    if payload.len() < 4 {
        return;
    }
    let vendor_length = u32::from_le_bytes(payload[..4].try_into().unwrap()) as usize;
    let mut offset = 4usize;
    let Some(vendor_end) = offset.checked_add(vendor_length) else {
        return;
    };
    if vendor_end > payload.len() || vendor_end + 4 > payload.len() {
        return;
    }
    offset = vendor_end;
    let count = u32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap()) as usize;
    offset += 4;
    for _ in 0..count {
        if offset + 4 > payload.len() {
            return;
        }
        let length = u32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if length > payload.len().saturating_sub(offset) {
            return;
        }
        let Some(comment) = std::str::from_utf8(&payload[offset..offset + length]).ok() else {
            offset += length;
            continue;
        };
        if let Some((name, value)) = comment.split_once('=') {
            if value.chars().count() <= 256
                && !value.chars().any(|character| character.is_control())
            {
                match name.to_ascii_uppercase().as_str() {
                    "TITLE" => metadata.title = Some(value.to_owned()),
                    "ARTIST" => metadata.artist = Some(value.to_owned()),
                    "COMMENT" => metadata.comment = Some(value.to_owned()),
                    _ => {}
                }
            }
        }
        offset += length;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordingEntry {
    pub id: u64,
    pub session: String,
    pub recorder: String,
    pub path: std::path::PathBuf,
    pub status: RecordingFileStatus,
    pub metadata: RecordingMetadata,
}

/// Root-scoped recording index. It owns metadata entries only; removing an
/// entry deliberately leaves the referenced recording bytes untouched.
pub struct RecordingLibrary {
    root: std::path::PathBuf,
    entries: Vec<RecordingEntry>,
    next_id: u64,
}

impl RecordingLibrary {
    pub fn new(policy: &RecordingPathPolicy) -> Self {
        Self {
            root: policy.root().to_path_buf(),
            entries: Vec::new(),
            next_id: 1,
        }
    }

    pub fn register(
        &mut self,
        session: impl Into<String>,
        recorder: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> Result<u64, RecordingLibraryError> {
        let path = self.validate_path(path.as_ref())?;
        if self.entries.iter().any(|entry| entry.path == path) {
            return Err(RecordingLibraryError::InvalidPath);
        }
        let status = inspect_recording(&path).map_err(|error| match error {
            RecordingError::Io(error) => RecordingLibraryError::Io(error.kind()),
            _ => RecordingLibraryError::InvalidPath,
        })?;
        let metadata = match path.extension().and_then(|extension| extension.to_str()) {
            Some(extension) if extension.eq_ignore_ascii_case("flac") => {
                read_flac_metadata(&path).unwrap_or_default()
            }
            _ => read_wav_metadata(&path).unwrap_or_default(),
        };
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.entries.push(RecordingEntry {
            id,
            session: session.into(),
            recorder: recorder.into(),
            path,
            status,
            metadata,
        });
        Ok(id)
    }

    pub fn refresh(&mut self, id: u64) -> Result<(), RecordingLibraryError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or(RecordingLibraryError::NotFound)?;
        entry.status = inspect_recording(&entry.path).map_err(|error| match error {
            RecordingError::Io(error) => RecordingLibraryError::Io(error.kind()),
            _ => RecordingLibraryError::InvalidPath,
        })?;
        Ok(())
    }

    pub fn list(&self, session: Option<&str>) -> Vec<&RecordingEntry> {
        self.entries
            .iter()
            .filter(|entry| session.map_or(true, |session| entry.session == session))
            .collect()
    }

    pub fn remove_entry(&mut self, id: u64) -> Result<RecordingEntry, RecordingLibraryError> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.id == id)
            .ok_or(RecordingLibraryError::NotFound)?;
        Ok(self.entries.remove(index))
    }

    pub fn set_metadata(
        &mut self,
        id: u64,
        metadata: RecordingMetadata,
    ) -> Result<(), RecordingLibraryError> {
        metadata.validate()?;
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or(RecordingLibraryError::NotFound)?;
        entry.metadata = metadata;
        Ok(())
    }

    fn validate_path(&self, path: &Path) -> Result<PathBuf, RecordingLibraryError> {
        let filename = path.file_name().ok_or(RecordingLibraryError::InvalidPath)?;
        let parent = path
            .parent()
            .ok_or(RecordingLibraryError::InvalidPath)?
            .canonicalize()
            .map_err(|error| RecordingLibraryError::Io(error.kind()))?;
        if parent != self.root || filename.to_string_lossy().contains(['/', '\\']) {
            return Err(RecordingLibraryError::PathOutsideRoot);
        }
        Ok(parent.join(filename))
    }
}

/// Reads the canonical WAV files produced by this crate for library indexing.
/// It validates the fixed PCM/IEEE-float header and exact data bounds but does
/// not decode samples or touch any audio device.
pub fn inspect_wav_file(path: impl AsRef<std::path::Path>) -> Result<WavFileInfo, RecordingError> {
    let mut file = std::fs::File::open(path)?;
    let file_bytes = file.metadata()?.len();
    if file_bytes < 44 {
        return Err(RecordingError::InvalidWav);
    }
    let mut header = [0u8; 44];
    file.read_exact(&mut header)?;
    if &header[0..4] != b"RIFF"
        || &header[8..12] != b"WAVE"
        || &header[12..16] != b"fmt "
        || &header[36..40] != b"data"
        || u32::from_le_bytes(header[16..20].try_into().unwrap()) != 16
    {
        return Err(RecordingError::InvalidWav);
    }
    let format = match (
        u16::from_le_bytes(header[20..22].try_into().unwrap()),
        u16::from_le_bytes(header[34..36].try_into().unwrap()),
    ) {
        (1, 16) => WavFormat::Pcm16,
        (1, 24) => WavFormat::Pcm24,
        (3, 32) => WavFormat::Float32,
        _ => return Err(RecordingError::InvalidWav),
    };
    let channels = u16::from_le_bytes(header[22..24].try_into().unwrap());
    let sample_rate = u32::from_le_bytes(header[24..28].try_into().unwrap());
    validate_format(format, channels, sample_rate)?;
    let block_align = u64::from(u16::from_le_bytes(header[32..34].try_into().unwrap()));
    let expected_align = u64::from(channels) * u64::from(format.bits() / 8);
    if block_align != expected_align {
        return Err(RecordingError::InvalidWav);
    }
    let data_bytes = u64::from(u32::from_le_bytes(header[40..44].try_into().unwrap()));
    if data_bytes % block_align != 0 || data_bytes > file_bytes - 44 {
        return Err(RecordingError::InvalidWav);
    }
    Ok(WavFileInfo {
        format,
        channels,
        sample_rate,
        frames: data_bytes / block_align,
        data_bytes,
        file_bytes,
    })
}

/// Returns a non-fatal library status for a recording path. Missing and
/// malformed files are represented as data so a library listing can continue
/// across stale entries; unrelated I/O failures remain errors.
pub fn inspect_recording(
    path: impl AsRef<std::path::Path>,
) -> Result<RecordingFileStatus, RecordingError> {
    if path
        .as_ref()
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("flac"))
    {
        return match inspect_flac_file(path) {
            Ok(info) => Ok(RecordingFileStatus::FlacPresent(info)),
            Err(RecordingError::InvalidWav) => Ok(RecordingFileStatus::Invalid),
            Err(RecordingError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(RecordingFileStatus::Missing)
            }
            Err(error) => Err(error),
        };
    }
    match inspect_wav_file(path) {
        Ok(info) => Ok(RecordingFileStatus::Present(info)),
        Err(RecordingError::InvalidWav) => Ok(RecordingFileStatus::Invalid),
        Err(RecordingError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(RecordingFileStatus::Missing)
        }
        Err(error) => Err(error),
    }
}

/// Inspects FLAC's STREAMINFO metadata without decoding audio frames.
pub fn inspect_flac_file(
    path: impl AsRef<std::path::Path>,
) -> Result<FlacFileInfo, RecordingError> {
    let mut file = std::fs::File::open(path)?;
    let file_bytes = file.metadata()?.len();
    let mut marker = [0u8; 4];
    file.read_exact(&mut marker)?;
    if &marker != b"fLaC" {
        return Err(RecordingError::InvalidWav);
    }
    let mut stream_info = None;
    loop {
        let mut block_header = [0u8; 4];
        file.read_exact(&mut block_header)?;
        let block_type = block_header[0] & 0x7f;
        let is_last = block_header[0] & 0x80 != 0;
        let length =
            u32::from_be_bytes([0, block_header[1], block_header[2], block_header[3]]) as usize;
        if block_type == 0 {
            if length != 34 {
                return Err(RecordingError::InvalidWav);
            }
            let mut bytes = [0u8; 34];
            file.read_exact(&mut bytes)?;
            let packed = u64::from_be_bytes(bytes[10..18].try_into().unwrap());
            let sample_rate = (packed >> 44) as u32;
            let channels = ((packed >> 41) & 0x07) as u8 + 1;
            let bits_per_sample = ((packed >> 36) & 0x1f) as u8 + 1;
            let frames = packed & 0x0000_ffff_ffff;
            if !matches!(channels, 1 | 2)
                || !matches!(sample_rate, 44_100 | 48_000)
                || !matches!(bits_per_sample, 16 | 24)
                || frames == 0
            {
                return Err(RecordingError::InvalidWav);
            }
            stream_info = Some(FlacFileInfo {
                channels,
                sample_rate,
                bits_per_sample,
                frames,
                file_bytes,
            });
        } else {
            std::io::copy(
                &mut std::io::Read::by_ref(&mut file).take(length as u64),
                &mut std::io::sink(),
            )?;
        }
        if is_last {
            break;
        }
    }
    stream_info.ok_or(RecordingError::InvalidWav)
}

/// Repairs a file emitted by StreamingFlacWriter after an interrupted write.
///
/// Recovery is deliberately limited to AudioRouter's bounded verbatim-frame
/// layout. It scans complete frame headers and CRCs without loading the audio
/// stream, truncates an incomplete tail, and patches STREAMINFO with the
/// number of complete samples. Unsupported FLAC subframes are rejected.
pub fn recover_streaming_flac_file(
    file: &mut std::fs::File,
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u8,
) -> Result<u64, RecordingError> {
    validate_format(WavFormat::Pcm16, channels, sample_rate)?;
    if !matches!(bits_per_sample, 16 | 24) {
        return Err(RecordingError::InvalidSampleCount);
    }
    let length = file.metadata()?.len();
    if length < 42 {
        return Err(RecordingError::InvalidWav);
    }
    file.seek(SeekFrom::Start(0))?;
    let mut marker = [0u8; 4];
    file.read_exact(&mut marker)?;
    if &marker != b"fLaC" {
        return Err(RecordingError::InvalidWav);
    }
    let mut metadata_position = 4u64;
    let mut streaminfo_start = None;
    let audio_start;
    loop {
        file.seek(SeekFrom::Start(metadata_position))?;
        let mut header = [0u8; 4];
        file.read_exact(&mut header)?;
        let is_last = header[0] & 0x80 != 0;
        let block_type = header[0] & 0x7f;
        let block_length = u32::from_be_bytes([0, header[1], header[2], header[3]]) as u64;
        let data_start = metadata_position + 4;
        if block_type == 0 && block_length == 34 && streaminfo_start.is_none() {
            streaminfo_start = Some(data_start);
        }
        let next = data_start
            .checked_add(block_length)
            .ok_or(RecordingError::TooManyFrames)?;
        if next > length {
            return Err(RecordingError::InvalidWav);
        }
        if is_last {
            audio_start = next;
            break;
        }
        metadata_position = next;
    }
    let streaminfo_start = streaminfo_start.ok_or(RecordingError::InvalidWav)?;
    let mut position = audio_start;
    let mut total_frames = 0u64;
    let mut minimum_frame_size = u32::MAX;
    let mut maximum_frame_size = 0u32;
    while position < length {
        let remaining = length - position;
        if remaining < 7 {
            break;
        }
        file.seek(SeekFrom::Start(position))?;
        let mut prefix = [0u8; 16];
        let prefix_length = file.read(&mut prefix)?;
        if prefix_length < 7 {
            break;
        }
        let first = u32::from_be_bytes(prefix[..4].try_into().unwrap());
        if ((first >> 18) & 0x3fff) != 0x3ffe
            || ((first >> 16) & 1) != 0
            || ((first >> 12) & 0x0f) != 7
            || ((first >> 8) & 0x0f) != 0
            || ((first >> 4) & 0x0f) != u32::from(channels - 1)
            || ((first >> 1) & 0x07) != 0
            || (first & 1) != 0
        {
            break;
        }
        let utf8_length = utf8_number_length(prefix[4]).ok_or(RecordingError::InvalidWav)?;
        let header_length = 4 + utf8_length + 2 + 1;
        if prefix_length < header_length {
            break;
        }
        let frame_count =
            u16::from_be_bytes(prefix[4 + utf8_length..6 + utf8_length].try_into().unwrap())
                as usize
                + 1;
        if frame_count > STREAMING_FLAC_BLOCK_FRAMES {
            break;
        }
        if crc8(&prefix[..header_length - 1]) != prefix[header_length - 1] {
            break;
        }
        let subframe_bytes = 1 + (frame_count * usize::from(bits_per_sample)).div_ceil(8);
        let frame_length = header_length
            .checked_add(usize::from(channels) * subframe_bytes)
            .and_then(|value| value.checked_add(2))
            .ok_or(RecordingError::TooManyFrames)?;
        if u64::try_from(frame_length).unwrap() > remaining {
            break;
        }
        let mut frame = vec![0u8; frame_length];
        file.seek(SeekFrom::Start(position))?;
        file.read_exact(&mut frame)?;
        let mut body_position = header_length;
        for _ in 0..channels {
            if frame[body_position] != 0x02 {
                return Err(RecordingError::InvalidWav);
            }
            body_position += subframe_bytes;
        }
        let expected_crc = u16::from_be_bytes(frame[frame_length - 2..].try_into().unwrap());
        if crc16(&frame[..frame_length - 2]) != expected_crc {
            break;
        }
        minimum_frame_size = minimum_frame_size.min(frame_length as u32);
        maximum_frame_size = maximum_frame_size.max(frame_length as u32);
        total_frames = total_frames
            .checked_add(frame_count as u64)
            .ok_or(RecordingError::TooManyFrames)?;
        position += frame_length as u64;
    }
    if total_frames == 0 {
        return Err(RecordingError::InvalidWav);
    }
    file.set_len(position)?;
    file.seek(SeekFrom::Start(streaminfo_start))?;
    let mut streaminfo = [0u8; 34];
    file.read_exact(&mut streaminfo)?;
    streaminfo[4..7].copy_from_slice(&minimum_frame_size.to_be_bytes()[1..]);
    streaminfo[7..10].copy_from_slice(&maximum_frame_size.to_be_bytes()[1..]);
    let mut packed = u64::from_be_bytes(streaminfo[10..18].try_into().unwrap());
    packed = (packed & !((1u64 << 36) - 1)) | total_frames.min((1u64 << 36) - 1);
    streaminfo[10..18].copy_from_slice(&packed.to_be_bytes());
    file.seek(SeekFrom::Start(streaminfo_start))?;
    file.write_all(&streaminfo)?;
    file.seek(SeekFrom::End(0))?;
    Ok(total_frames)
}

fn utf8_number_length(first: u8) -> Option<usize> {
    if first < 0x80 {
        Some(1)
    } else if first & 0xe0 == 0xc0 {
        Some(2)
    } else if first & 0xf0 == 0xe0 {
        Some(3)
    } else if first & 0xf8 == 0xf0 {
        Some(4)
    } else if first & 0xfc == 0xf8 {
        Some(5)
    } else if first & 0xfe == 0xfc {
        Some(6)
    } else {
        None
    }
}

impl<W: Write + Seek> WavWriter<W> {
    pub fn new(
        mut output: W,
        format: WavFormat,
        channels: u16,
        sample_rate: u32,
        dither: bool,
    ) -> Result<Self, RecordingError> {
        validate_format(format, channels, sample_rate)?;
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

    pub fn finish(self) -> Result<W, RecordingError> {
        self.finish_with_metadata(&WavMetadata::default())
    }

    pub fn finish_with_metadata(mut self, metadata: &WavMetadata) -> Result<W, RecordingError> {
        metadata.validate()?;
        let info_chunk = encode_info_chunk(metadata)?;
        let data_size =
            u32::try_from(self.data_bytes).map_err(|_| RecordingError::TooManyFrames)?;
        let riff_size = 36u64
            .checked_add(u64::from(data_size))
            .and_then(|size| size.checked_add(info_chunk.len() as u64))
            .and_then(|size| u32::try_from(size).ok())
            .ok_or(RecordingError::TooManyFrames)?;
        self.output.seek(SeekFrom::End(0))?;
        self.output.write_all(&info_chunk)?;
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

fn add_flac_comments(
    mut encoded: Vec<u8>,
    metadata: &WavMetadata,
) -> Result<Vec<u8>, RecordingError> {
    let values = [
        ("TITLE", metadata.title.as_deref()),
        ("ARTIST", metadata.artist.as_deref()),
        ("COMMENT", metadata.comment.as_deref()),
    ];
    if values.iter().all(|(_, value)| value.is_none()) {
        return Ok(encoded);
    }
    if encoded.len() < 8 || &encoded[..4] != b"fLaC" {
        return Err(RecordingError::FlacEncode(
            "encoder returned an invalid FLAC signature".into(),
        ));
    }
    let header = encoded[4];
    let block_type = header & 0x7f;
    let block_length =
        (usize::from(encoded[5]) << 16) | (usize::from(encoded[6]) << 8) | usize::from(encoded[7]);
    if header & 0x80 == 0 || block_type != 0 || block_length != 34 {
        return Err(RecordingError::FlacEncode(
            "encoder returned an unexpected FLAC metadata layout".into(),
        ));
    }
    let stream_info_end = 8usize
        .checked_add(block_length)
        .ok_or(RecordingError::TooManyFrames)?;
    if encoded.len() < stream_info_end {
        return Err(RecordingError::FlacEncode(
            "encoder returned truncated FLAC streaminfo".into(),
        ));
    }
    let mut payload = Vec::new();
    let vendor = b"audiorouter";
    payload.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    payload.extend_from_slice(vendor);
    let comments = values
        .into_iter()
        .filter_map(|(name, value)| value.map(|value| format!("{name}={value}")))
        .collect::<Vec<_>>();
    payload.extend_from_slice(&(comments.len() as u32).to_le_bytes());
    for comment in comments {
        let bytes = comment.as_bytes();
        let length = u32::try_from(bytes.len()).map_err(|_| RecordingError::TooManyFrames)?;
        payload.extend_from_slice(&length.to_le_bytes());
        payload.extend_from_slice(bytes);
    }
    let length = u32::try_from(payload.len()).map_err(|_| RecordingError::TooManyFrames)?;
    if length > 0x00ff_ffff {
        return Err(RecordingError::TooManyFrames);
    }
    encoded[4] &= 0x7f;
    let mut block = Vec::with_capacity(4 + payload.len());
    block.push(0x84);
    block.push((length >> 16) as u8);
    block.push((length >> 8) as u8);
    block.push(length as u8);
    block.extend_from_slice(&payload);
    encoded.splice(stream_info_end..stream_info_end, block);
    Ok(encoded)
}

fn encode_info_chunk(metadata: &WavMetadata) -> Result<Vec<u8>, RecordingError> {
    let fields = [
        (*b"INAM", metadata.title.as_deref()),
        (*b"IART", metadata.artist.as_deref()),
        (*b"ICMT", metadata.comment.as_deref()),
    ];
    let mut payload = Vec::new();
    payload.extend_from_slice(b"INFO");
    for (id, value) in fields
        .into_iter()
        .filter_map(|(id, value)| value.map(|v| (id, v)))
    {
        let mut bytes = value.as_bytes().to_vec();
        bytes.push(0);
        let length = u32::try_from(bytes.len()).map_err(|_| RecordingError::TooManyFrames)?;
        payload.extend_from_slice(&id);
        payload.extend_from_slice(&length.to_le_bytes());
        payload.extend_from_slice(&bytes);
        if bytes.len() % 2 != 0 {
            payload.push(0);
        }
    }
    if payload.len() == 4 {
        return Ok(Vec::new());
    }
    let size = u32::try_from(payload.len()).map_err(|_| RecordingError::TooManyFrames)?;
    let mut chunk = Vec::with_capacity(8 + payload.len());
    chunk.extend_from_slice(b"LIST");
    chunk.extend_from_slice(&size.to_le_bytes());
    chunk.extend_from_slice(&payload);
    Ok(chunk)
}

fn validate_format(
    _format: WavFormat,
    channels: u16,
    sample_rate: u32,
) -> Result<(), RecordingError> {
    if !matches!(sample_rate, 44_100 | 48_000) {
        return Err(RecordingError::InvalidSampleRate);
    }
    if !(1..=MAX_CHANNELS).contains(&channels) {
        return Err(RecordingError::InvalidChannels);
    }
    Ok(())
}

/// Repairs a WAV file whose final header patch was interrupted.
///
/// Recovery is conservative: only the existing bytes after the 44-byte WAV
/// header are considered, trailing partial samples are removed, and the
/// caller-provided format is used to rebuild the header. The file is never
/// replaced or deleted.
pub fn recover_wav_file(
    file: &mut std::fs::File,
    format: WavFormat,
    channels: u16,
    sample_rate: u32,
) -> Result<u64, RecordingError> {
    validate_format(format, channels, sample_rate)?;
    let length = file.metadata()?.len();
    let data_bytes = length
        .checked_sub(44)
        .ok_or(RecordingError::InvalidSampleCount)?;
    let bytes_per_frame = u64::from(channels) * u64::from(format.bits() / 8);
    let complete_data_bytes = data_bytes - (data_bytes % bytes_per_frame);
    let total_length = 44u64
        .checked_add(complete_data_bytes)
        .ok_or(RecordingError::TooManyFrames)?;
    if complete_data_bytes > u64::from(u32::MAX - 36) {
        return Err(RecordingError::TooManyFrames);
    }
    file.set_len(total_length)?;
    file.seek(SeekFrom::Start(0))?;
    write_header(
        file,
        format,
        channels,
        sample_rate,
        36 + complete_data_bytes as u32,
        complete_data_bytes as u32,
    )?;
    file.seek(SeekFrom::End(0))?;
    Ok(complete_data_bytes / bytes_per_frame)
}

/// A worker-side WAV recorder that joins queue draining with recorder state.
/// The worker owns the encoder; callers retain ownership of the destination.
pub struct WavRecorder<W> {
    writer: WavWriter<W>,
    controller: RecorderController,
    next_frame: Option<u64>,
}

/// Queue-backed FLAC worker using the dependency's bounded batch encoder.
/// This owns no path and emits bytes only at `finish`; a true incremental
/// FLAC file worker remains a separate implementation requirement.
pub struct BufferedFlacRecorder {
    encoder: FlacBufferEncoder,
    controller: RecorderController,
    next_frame: Option<u64>,
}

/// Queue-backed recorder that writes FLAC frames as chunks are drained.
/// Unlike BufferedFlacRecorder, this worker does not retain the complete
/// recording in an in-memory sample buffer.
pub struct StreamingFlacRecorder<W> {
    writer: StreamingFlacWriter<W>,
    controller: RecorderController,
    next_frame: Option<u64>,
}

impl<W: Write + Seek> StreamingFlacRecorder<W> {
    pub fn new(writer: StreamingFlacWriter<W>) -> Self {
        Self {
            writer,
            controller: RecorderController::new(),
            next_frame: None,
        }
    }

    pub fn state(&self) -> RecorderState {
        self.controller.state()
    }

    pub fn checkpoint(&self) -> RecorderCheckpoint {
        self.controller.checkpoint()
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

    pub fn stop(&mut self, frame: u64) -> Result<(), RecorderError> {
        self.controller.stop(frame)
    }

    pub fn fail(&mut self) {
        self.controller.fail();
    }

    pub fn drain_queue(
        &mut self,
        queue: &RecordingQueue,
        maximum_chunks: usize,
    ) -> Result<usize, RecordingError> {
        self.drain_queue_with_checkpoint(queue, maximum_chunks, |_| Ok(()))
    }

    pub fn drain_queue_with_checkpoint<F>(
        &mut self,
        queue: &RecordingQueue,
        maximum_chunks: usize,
        mut persist: F,
    ) -> Result<usize, RecordingError>
    where
        F: FnMut(&RecorderCheckpoint) -> Result<(), RecordingError>,
    {
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
                self.controller.fail();
                return Err(RecordingError::FrameDiscontinuity {
                    expected,
                    actual: chunk.start_frame,
                });
            }
            let frames = chunk.samples.len() / usize::from(self.writer.channels);
            if let Err(error) = self.writer.write_interleaved(&chunk.samples) {
                self.controller.fail();
                return Err(error);
            }
            let end = expected
                .checked_add(frames as u64)
                .ok_or(RecordingError::TooManyFrames)?;
            self.controller
                .advance(end)
                .map_err(RecordingError::Controller)?;
            self.next_frame = Some(end);
            let checkpoint = self.controller.checkpoint();
            if let Err(error) = persist(&checkpoint) {
                self.controller.fail();
                return Err(error);
            }
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

impl BufferedFlacRecorder {
    pub fn new(
        channels: usize,
        sample_rate: u32,
        bits_per_sample: u8,
    ) -> Result<Self, RecordingError> {
        Ok(Self {
            encoder: FlacBufferEncoder::new(channels, sample_rate, bits_per_sample)?,
            controller: RecorderController::new(),
            next_frame: None,
        })
    }

    pub fn state(&self) -> RecorderState {
        self.controller.state()
    }

    /// Returns the validated control-plane boundary snapshot for durable
    /// recovery. Audio samples and encoder buffers are intentionally omitted.
    pub fn checkpoint(&self) -> RecorderCheckpoint {
        self.controller.checkpoint()
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

    pub fn stop(&mut self, frame: u64) -> Result<(), RecorderError> {
        self.controller.stop(frame)
    }

    pub fn fail(&mut self) {
        self.controller.fail();
    }

    pub fn drain_queue(
        &mut self,
        queue: &RecordingQueue,
        maximum_chunks: usize,
    ) -> Result<usize, RecordingError> {
        self.drain_queue_with_checkpoint(queue, maximum_chunks, |_| Ok(()))
    }

    /// Drains audio and invokes the persistence hook after every committed
    /// contiguous chunk. A hook failure fails the recorder so a caller cannot
    /// report durable progress that was not actually persisted.
    pub fn drain_queue_with_checkpoint<F>(
        &mut self,
        queue: &RecordingQueue,
        maximum_chunks: usize,
        mut persist: F,
    ) -> Result<usize, RecordingError>
    where
        F: FnMut(&RecorderCheckpoint) -> Result<(), RecordingError>,
    {
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
                self.controller.fail();
                return Err(RecordingError::FrameDiscontinuity {
                    expected,
                    actual: chunk.start_frame,
                });
            }
            let frames = chunk.samples.len() / self.encoder.channels;
            if let Err(error) = self.encoder.write_interleaved(&chunk.samples) {
                self.controller.fail();
                return Err(error);
            }
            let end = expected
                .checked_add(frames as u64)
                .ok_or(RecordingError::TooManyFrames)?;
            self.controller
                .advance(end)
                .map_err(RecordingError::Controller)?;
            self.next_frame = Some(end);
            let checkpoint = self.controller.checkpoint();
            if let Err(error) = persist(&checkpoint) {
                self.controller.fail();
                return Err(error);
            }
            drained += 1;
        }
        Ok(drained)
    }

    pub fn finish(self) -> Result<Vec<u8>, RecordingError> {
        self.finish_with_metadata(&WavMetadata::default())
    }

    pub fn finish_with_metadata(self, metadata: &WavMetadata) -> Result<Vec<u8>, RecordingError> {
        if self.controller.state() != RecorderState::Completed {
            return Err(RecordingError::NotRecording);
        }
        self.encoder.finish_with_metadata(metadata)
    }
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

    /// Returns the validated control-plane boundary snapshot for durable
    /// recovery. Audio samples and the destination handle are intentionally
    /// omitted.
    pub fn checkpoint(&self) -> RecorderCheckpoint {
        self.controller.checkpoint()
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

    /// Marks the recorder failed after an unrecoverable worker or I/O error.
    /// The destination remains caller-owned so it can be recovered or moved
    /// to quarantine by a higher-level library.
    pub fn fail(&mut self) {
        self.controller.fail();
    }

    pub fn drain_queue(
        &mut self,
        queue: &RecordingQueue,
        maximum_chunks: usize,
    ) -> Result<usize, RecordingError> {
        self.drain_queue_with_checkpoint(queue, maximum_chunks, |_| Ok(()))
    }

    /// Drains audio and invokes the persistence hook after every committed
    /// contiguous chunk. A hook failure fails the recorder before more audio
    /// can be accepted.
    pub fn drain_queue_with_checkpoint<F>(
        &mut self,
        queue: &RecordingQueue,
        maximum_chunks: usize,
        mut persist: F,
    ) -> Result<usize, RecordingError>
    where
        F: FnMut(&RecorderCheckpoint) -> Result<(), RecordingError>,
    {
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
                self.controller.fail();
                return Err(RecordingError::FrameDiscontinuity {
                    expected,
                    actual: chunk.start_frame,
                });
            }
            let frames = chunk.samples.len() / usize::from(self.writer.channels);
            if let Err(error) = self.writer.write_interleaved(&chunk.samples) {
                self.controller.fail();
                return Err(error);
            }
            let end = expected
                .checked_add(frames as u64)
                .ok_or(RecordingError::TooManyFrames)?;
            self.controller
                .advance(end)
                .map_err(RecordingError::Controller)?;
            self.next_frame = Some(end);
            let checkpoint = self.controller.checkpoint();
            if let Err(error) = persist(&checkpoint) {
                self.controller.fail();
                return Err(error);
            }
            drained += 1;
        }
        Ok(drained)
    }

    pub fn finish(self) -> Result<W, RecordingError> {
        self.finish_with_metadata(&WavMetadata::default())
    }

    pub fn finish_with_metadata(self, metadata: &WavMetadata) -> Result<W, RecordingError> {
        if self.controller.state() != RecorderState::Completed {
            return Err(RecordingError::NotRecording);
        }
        self.writer.finish_with_metadata(metadata)
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
    fn wav_metadata_is_written_as_bounded_info_chunks() {
        let mut writer =
            WavWriter::new(Cursor::new(Vec::new()), WavFormat::Pcm16, 1, 48_000, false).unwrap();
        writer.write_interleaved(&[0.25]).unwrap();
        let metadata = WavMetadata {
            title: Some("Take 1".into()),
            artist: Some("AudioRouter".into()),
            comment: Some("voice".into()),
        };
        let output = writer.finish_with_metadata(&metadata).unwrap().into_inner();
        assert!(output.windows(4).any(|window| window == b"LIST"));
        assert!(output.windows(6).any(|window| window == b"Take 1"));
        assert!(output.windows(11).any(|window| window == b"AudioRouter"));
        assert_eq!(u32::from_le_bytes(output[40..44].try_into().unwrap()), 2);
    }

    #[test]
    fn wav_metadata_rejects_control_and_oversized_values() {
        let writer =
            WavWriter::new(Cursor::new(Vec::new()), WavFormat::Pcm16, 1, 48_000, false).unwrap();
        let metadata = WavMetadata {
            title: Some("bad\nvalue".into()),
            ..WavMetadata::default()
        };
        assert!(matches!(
            writer.finish_with_metadata(&metadata),
            Err(RecordingError::InvalidMetadata)
        ));
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
    fn recorder_checkpoint_round_trips_paused_boundaries_without_audio_payload() {
        let mut recorder = RecorderController::new();
        recorder.arm().unwrap();
        recorder.start(100).unwrap();
        recorder.pause(200).unwrap();
        let document = recorder.checkpoint_json().unwrap();
        assert!(!document.contains("samples"));
        let restored = RecorderController::restore_json(&document).unwrap();
        assert_eq!(restored.state(), RecorderState::Paused);
        assert_eq!(restored.parts(), recorder.parts());
        assert_eq!(restored.pause_intervals(), recorder.pause_intervals());
        assert_eq!(restored.checkpoint(), recorder.checkpoint());
    }

    #[test]
    fn recorder_checkpoint_rejects_corrupt_or_inconsistent_state() {
        let mut checkpoint = RecorderController::new().checkpoint();
        checkpoint.version = 2;
        assert!(matches!(
            RecorderController::restore(checkpoint),
            Err(RecorderError::InvalidCheckpoint)
        ));
        assert!(matches!(
            RecorderController::restore_json(r#"{"version":1,"state":"Paused"}"#),
            Err(RecorderError::InvalidCheckpoint)
        ));
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
        assert_eq!(recorder.checkpoint().last_frame, Some(12));
        assert_eq!(recorder.drain_queue(&queue, 8).unwrap(), 1);
        recorder.stop(13).unwrap();
        assert_eq!(recorder.checkpoint().state, RecorderState::Completed);
        let output = recorder.finish().unwrap().into_inner();
        assert_eq!(u32::from_le_bytes(output[40..44].try_into().unwrap()), 6);
    }

    #[test]
    fn wav_worker_persists_checkpoint_after_each_committed_chunk() {
        let writer =
            WavWriter::new(Cursor::new(Vec::new()), WavFormat::Pcm16, 1, 48_000, false).unwrap();
        let mut recorder = WavRecorder::new(writer);
        let queue = RecordingQueue::new(2).unwrap();
        queue
            .try_push(RecordingChunk {
                start_frame: 4,
                samples: vec![0.0, 0.1],
            })
            .unwrap();
        queue
            .try_push(RecordingChunk {
                start_frame: 6,
                samples: vec![0.2],
            })
            .unwrap();
        recorder.arm().unwrap();
        recorder.start(4).unwrap();
        let mut frames = Vec::new();
        assert_eq!(
            recorder
                .drain_queue_with_checkpoint(&queue, 8, |checkpoint| {
                    frames.push(checkpoint.last_frame);
                    Ok(())
                })
                .unwrap(),
            2
        );
        assert_eq!(frames, vec![Some(6), Some(7)]);
    }

    #[test]
    fn checkpoint_persistence_failure_fails_wav_worker() {
        let writer =
            WavWriter::new(Cursor::new(Vec::new()), WavFormat::Pcm16, 1, 48_000, false).unwrap();
        let mut recorder = WavRecorder::new(writer);
        let queue = RecordingQueue::new(1).unwrap();
        queue
            .try_push(RecordingChunk {
                start_frame: 0,
                samples: vec![0.0],
            })
            .unwrap();
        recorder.arm().unwrap();
        recorder.start(0).unwrap();
        let result =
            recorder.drain_queue_with_checkpoint(&queue, 1, |_| Err(RecordingError::InvalidWav));
        assert!(matches!(result, Err(RecordingError::InvalidWav)));
        assert_eq!(recorder.state(), RecorderState::Failed);
    }

    #[test]
    fn buffered_flac_recorder_drains_contiguous_chunks_and_finalizes() {
        let mut recorder = BufferedFlacRecorder::new(1, 48_000, 16).unwrap();
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
        assert_eq!(recorder.drain_queue(&queue, 8).unwrap(), 2);
        assert_eq!(recorder.checkpoint().last_frame, Some(13));
        recorder.stop(13).unwrap();
        let bytes = recorder.finish().unwrap();
        let info = flac_io::info(&bytes).unwrap();
        assert_eq!(info.sample_rate, 48_000);
        assert_eq!(info.channels, 1);
        assert_eq!(info.bits_per_sample, 16);
        assert_eq!(info.total_samples, 3);
    }

    #[test]
    fn flac_metadata_is_written_as_vorbis_comments() {
        let mut encoder = FlacBufferEncoder::new(1, 44_100, 16).unwrap();
        encoder.write_interleaved(&[0.0, 0.25]).unwrap();
        let metadata = WavMetadata {
            title: Some("Take 2".into()),
            artist: None,
            comment: Some("clean".into()),
        };
        let bytes = encoder.finish_with_metadata(&metadata).unwrap();
        assert!(bytes.windows(11).any(|window| window == b"audiorouter"));
        assert!(bytes.windows(12).any(|window| window == b"TITLE=Take 2"));
        assert!(bytes.windows(13).any(|window| window == b"COMMENT=clean"));
        assert_eq!(flac_io::info(&bytes).unwrap().total_samples, 2);
        let path = std::env::temp_dir().join(format!(
            "audiorouter-flac-metadata-{}.flac",
            std::process::id()
        ));
        std::fs::write(&path, &bytes).unwrap();
        assert_eq!(
            read_flac_metadata(&path).unwrap(),
            RecordingMetadata {
                title: Some("Take 2".into()),
                artist: None,
                comment: Some("clean".into()),
            }
        );
        let _ = std::fs::remove_file(path);
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

    #[test]
    fn recovery_patches_header_and_drops_partial_frame() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-recording-recovery-{}.wav",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        write_header(&mut file, WavFormat::Pcm16, 2, 48_000, 0, 0).unwrap();
        file.write_all(&[0; 10]).unwrap();
        file.flush().unwrap();
        assert_eq!(
            recover_wav_file(&mut file, WavFormat::Pcm16, 2, 48_000).unwrap(),
            2
        );
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 52);
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 44);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 8);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn worker_failure_is_terminal_and_prevents_finalization() {
        let writer =
            WavWriter::new(Cursor::new(Vec::new()), WavFormat::Pcm16, 1, 48_000, false).unwrap();
        let mut recorder = WavRecorder::new(writer);
        let queue = RecordingQueue::new(1).unwrap();
        queue
            .try_push(RecordingChunk {
                start_frame: 99,
                samples: vec![0.0],
            })
            .unwrap();
        recorder.arm().unwrap();
        recorder.start(100).unwrap();
        assert!(matches!(
            recorder.drain_queue(&queue, 1),
            Err(RecordingError::FrameDiscontinuity { .. })
        ));
        assert_eq!(recorder.state(), RecorderState::Failed);
        assert!(matches!(
            recorder.finish(),
            Err(RecordingError::NotRecording)
        ));
    }

    #[test]
    fn wav_inspector_reports_file_shape_and_rejects_truncation() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-recording-inspect-{}.wav",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let file = std::fs::File::create(&path).unwrap();
        let mut writer = WavWriter::new(file, WavFormat::Pcm24, 2, 44_100, false).unwrap();
        writer.write_interleaved(&[0.0, 0.25, -0.25, 0.5]).unwrap();
        writer.finish().unwrap();
        let info = inspect_wav_file(&path).unwrap();
        assert_eq!(
            info,
            WavFileInfo {
                format: WavFormat::Pcm24,
                channels: 2,
                sample_rate: 44_100,
                frames: 2,
                data_bytes: 12,
                file_bytes: 56,
            }
        );
        let truncated = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        truncated.set_len(50).unwrap();
        assert!(matches!(
            inspect_wav_file(&path),
            Err(RecordingError::InvalidWav)
        ));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn recording_status_keeps_missing_and_invalid_files_listable() {
        let missing = std::env::temp_dir().join(format!(
            "audiorouter-recording-missing-{}.wav",
            std::process::id()
        ));
        let invalid = std::env::temp_dir().join(format!(
            "audiorouter-recording-invalid-{}.wav",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&missing);
        std::fs::write(&invalid, b"not a wav").unwrap();
        assert_eq!(
            inspect_recording(&missing).unwrap(),
            RecordingFileStatus::Missing
        );
        assert_eq!(
            inspect_recording(&invalid).unwrap(),
            RecordingFileStatus::Invalid
        );
        let _ = std::fs::remove_file(invalid);
    }

    #[test]
    fn library_lists_missing_entries_and_removal_keeps_file() {
        let root = std::env::temp_dir().join(format!(
            "audiorouter-recording-library-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        let policy = RecordingPathPolicy::new(&root).unwrap();
        let (path, file) = policy.create_file("session", "voice", 0, "wav").unwrap();
        let mut writer = WavWriter::new(file, WavFormat::Pcm16, 1, 48_000, false).unwrap();
        writer.write_interleaved(&[0.25, -0.25]).unwrap();
        writer
            .finish_with_metadata(&WavMetadata {
                title: Some("Tagged take".into()),
                artist: Some("AudioRouter".into()),
                comment: Some("from RIFF INFO".into()),
            })
            .unwrap();

        let mut library = RecordingLibrary::new(&policy);
        let id = library.register("session", "voice", &path).unwrap();
        let missing = library
            .register("session", "missing", root.join("missing-1.wav"))
            .unwrap();
        assert!(matches!(
            library.list(Some("session"))[0].status,
            RecordingFileStatus::Present(_)
        ));
        assert_eq!(
            library.list(Some("session"))[0].metadata,
            RecordingMetadata {
                title: Some("Tagged take".into()),
                artist: Some("AudioRouter".into()),
                comment: Some("from RIFF INFO".into()),
            }
        );
        assert_eq!(
            library.list(Some("session"))[1].status,
            RecordingFileStatus::Missing
        );
        library
            .set_metadata(
                id,
                RecordingMetadata {
                    title: Some("Morning voice".into()),
                    artist: Some("AudioRouter".into()),
                    comment: Some("kept locally".into()),
                },
            )
            .unwrap();
        assert_eq!(
            library.list(Some("session"))[0].metadata.title.as_deref(),
            Some("Morning voice")
        );
        assert!(matches!(
            library.set_metadata(
                id,
                RecordingMetadata {
                    title: Some("bad\nvalue".into()),
                    ..RecordingMetadata::default()
                }
            ),
            Err(RecordingLibraryError::InvalidMetadata)
        ));
        library.refresh(missing).unwrap();
        let removed = library.remove_entry(id).unwrap();
        assert_eq!(removed.path, path);
        assert!(path.exists());
        assert!(matches!(
            library.register("other", "voice", root.join("..\\escape.wav")),
            Err(RecordingLibraryError::PathOutsideRoot)
        ));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn flac_buffer_round_trips_pcm16_and_rejects_unshaped_input() {
        let mut encoder = FlacBufferEncoder::new(2, 48_000, 16).unwrap();
        assert_eq!(
            encoder.write_interleaved(&[0.0, 0.5, -0.25, 1.0]).unwrap(),
            2
        );
        let bytes = encoder.finish().unwrap();
        assert_eq!(&bytes[0..4], b"fLaC");
        let decoded = flac_io::decode(&bytes).unwrap();
        assert_eq!(decoded.sample_rate, 48_000);
        assert_eq!(decoded.channels, 2);
        assert_eq!(decoded.bits_per_sample, 16);
        assert_eq!(decoded.samples, vec![vec![0, -8_192], vec![16_384, 32_767]]);

        let mut invalid = FlacBufferEncoder::new(1, 48_000, 16).unwrap();
        assert!(matches!(invalid.write_interleaved(&[0.0, 1.0]), Ok(2)));
        let empty = FlacBufferEncoder::new(1, 48_000, 24).unwrap();
        assert!(matches!(
            empty.finish(),
            Err(RecordingError::InvalidSampleCount)
        ));
    }

    #[test]
    fn streaming_flac_writer_emits_incremental_frames_and_patches_streaminfo() {
        let mut writer =
            StreamingFlacWriter::new(Cursor::new(Vec::new()), 2, 48_000, 16, false).unwrap();
        assert_eq!(
            writer.write_interleaved(&[0.0, 0.25, -0.5, 0.75]).unwrap(),
            2
        );
        assert_eq!(writer.frames(), 2);
        assert_eq!(
            writer
                .write_interleaved(&[0.125, -0.125, 0.5, -0.5, 0.0, 0.0])
                .unwrap(),
            3
        );
        let bytes = writer.finish().unwrap().into_inner();
        assert_eq!(&bytes[..4], b"fLaC");
        let info = flac_io::info(&bytes).unwrap();
        assert_eq!(info.total_samples, 5);
        let decoded = flac_io::decode(&bytes).unwrap();
        assert_eq!(decoded.samples.len(), 2);
        assert_eq!(decoded.samples[0].len(), 5);
        assert_eq!(decoded.samples[1].len(), 5);
    }

    #[test]
    fn streaming_flac_recorder_drains_queue_and_persists_boundaries() {
        let writer =
            StreamingFlacWriter::new(Cursor::new(Vec::new()), 1, 48_000, 16, false).unwrap();
        let mut recorder = StreamingFlacRecorder::new(writer);
        recorder.arm().unwrap();
        recorder.start(10).unwrap();
        let queue = RecordingQueue::new(2).unwrap();
        queue
            .try_push(RecordingChunk {
                start_frame: 10,
                samples: vec![0.1, 0.2],
            })
            .unwrap();
        queue
            .try_push(RecordingChunk {
                start_frame: 12,
                samples: vec![0.3],
            })
            .unwrap();
        let mut persisted = Vec::new();
        assert_eq!(
            recorder
                .drain_queue_with_checkpoint(&queue, 2, |checkpoint| {
                    persisted.push(checkpoint.last_frame);
                    Ok(())
                })
                .unwrap(),
            2
        );
        assert_eq!(persisted, [Some(12), Some(13)]);
        recorder.stop(13).unwrap();
        let bytes = recorder.finish().unwrap().into_inner();
        assert_eq!(flac_io::info(&bytes).unwrap().total_samples, 3);
    }

    #[test]
    fn streaming_flac_writer_preserves_bounded_metadata() {
        let metadata = WavMetadata {
            title: Some("Live take".into()),
            artist: Some("AudioRouter".into()),
            comment: Some("incremental".into()),
        };
        let mut writer = StreamingFlacWriter::new_with_metadata(
            Cursor::new(Vec::new()),
            1,
            44_100,
            24,
            false,
            &metadata,
        )
        .unwrap();
        writer.write_interleaved(&[0.25, -0.25]).unwrap();
        let path = std::env::temp_dir().join(format!(
            "audiorouter-streaming-flac-metadata-{}.flac",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, writer.finish().unwrap().into_inner()).unwrap();
        assert_eq!(
            read_flac_metadata(&path).unwrap(),
            RecordingMetadata {
                title: metadata.title.clone(),
                artist: metadata.artist.clone(),
                comment: metadata.comment.clone(),
            }
        );
        assert_eq!(inspect_flac_file(&path).unwrap().frames, 2);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn streaming_flac_recovery_truncates_an_incomplete_tail() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-streaming-flac-recovery-{}.flac",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let mut writer =
            StreamingFlacWriter::new(Cursor::new(Vec::new()), 2, 48_000, 16, false).unwrap();
        writer
            .write_interleaved(&[0.0, 0.25, -0.5, 0.75, 0.1, -0.1])
            .unwrap();
        let mut bytes = writer.finish().unwrap().into_inner();
        bytes.extend_from_slice(&[0xaa, 0xbb, 0xcc]);
        std::fs::write(&path, bytes).unwrap();
        let before = std::fs::metadata(&path).unwrap().len();
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        assert_eq!(
            recover_streaming_flac_file(&mut file, 2, 48_000, 16).unwrap(),
            3
        );
        let after = file.metadata().unwrap().len();
        assert!(after < before);
        assert_eq!(inspect_flac_file(&path).unwrap().frames, 3);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn flac_inspection_reads_streaminfo_and_library_status() {
        let encoder = {
            let mut encoder = FlacBufferEncoder::new(2, 48_000, 16).unwrap();
            encoder.write_interleaved(&[0.0, 0.25, -0.5, 0.75]).unwrap();
            encoder.finish().unwrap()
        };
        let path = std::env::temp_dir().join(format!(
            "audiorouter-recording-inspect-{}.flac",
            std::process::id()
        ));
        std::fs::write(&path, encoder).unwrap();
        assert_eq!(
            inspect_flac_file(&path).unwrap(),
            FlacFileInfo {
                channels: 2,
                sample_rate: 48_000,
                bits_per_sample: 16,
                frames: 2,
                file_bytes: std::fs::metadata(&path).unwrap().len(),
            }
        );
        assert!(matches!(
            inspect_recording(&path),
            Ok(RecordingFileStatus::FlacPresent(FlacFileInfo {
                channels: 2,
                sample_rate: 48_000,
                bits_per_sample: 16,
                frames: 2,
                ..
            }))
        ));
        std::fs::write(&path, b"not flac").unwrap();
        assert_eq!(
            inspect_recording(&path).unwrap(),
            RecordingFileStatus::Invalid
        );
        let _ = std::fs::remove_file(path);
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
