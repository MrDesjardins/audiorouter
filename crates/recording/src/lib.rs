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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FlacFileInfo {
    pub channels: u8,
    pub sample_rate: u32,
    pub bits_per_sample: u8,
    pub frames: u64,
    pub file_bytes: u64,
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
        if self.samples.is_empty() {
            return Err(RecordingError::InvalidSampleCount);
        }
        let mut planar = vec![Vec::with_capacity(self.frames() as usize); self.channels];
        for frame in self.samples.chunks_exact(self.channels) {
            for (channel, sample) in frame.iter().enumerate() {
                planar[channel].push(*sample);
            }
        }
        flac_io::encode(&flac_io::FlacAudio {
            sample_rate: self.sample_rate,
            channels: self.channels as u8,
            bits_per_sample: self.bits_per_sample,
            samples: planar,
        })
        .map_err(|error| RecordingError::FlacEncode(error.to_string()))
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
        let metadata = RecordingMetadata::default();
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
            drained += 1;
        }
        Ok(drained)
    }

    pub fn finish(self) -> Result<Vec<u8>, RecordingError> {
        if self.controller.state() != RecorderState::Completed {
            return Err(RecordingError::NotRecording);
        }
        self.encoder.finish()
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
        writer.finish().unwrap();

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
