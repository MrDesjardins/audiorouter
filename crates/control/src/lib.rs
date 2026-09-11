//! Portable control-plane façade for M01.
//!
//! Transport, authorization, and durable storage are deliberately separate
//! follow-up layers. This façade proves that all adapters can share one domain
//! authority and that unsupported audio capabilities are discoverable.

use audiorouter_domain::{
    format_validation_errors, inspect_routes, node_registry, validate_session, ApiMethodSpec,
    CrashRecoveryTracker, EntityId, EventLog, EventReplayError, FakeRuntime, GraphStore, NodeKind,
    PermissionScope, RecoveryDecision, RecoveryMode, RuntimeError, RuntimeState, Session,
    VirtualBusRegistry, API_METHODS,
};
use audiorouter_engine::{
    AudioBlock, AudioTap, AudioTapSet, RecorderTapBindings, RuntimeGeneration, VirtualBusBridgeSet,
    VirtualBusBridgeSetError,
};
use audiorouter_protocol::{
    decode_rpc_frame, encode_frame, FrameError, JsonRpcRequest, JsonRpcResponse, RpcMessage,
    MAX_METHOD_NAME_BYTES, MAX_REQUEST_ID_BYTES,
};
use audiorouter_recording::{
    BufferedFlacRecorder, PathPolicyError, RecorderController, RecorderState, RecordingChunk,
    RecordingError, RecordingPathPolicy, RecordingQueue, SegmentedWavRecorder,
    StreamingFlacRecorder, StreamingFlacWriter, WavFormat, WavRecorder, WavWriter,
};
use audiorouter_storage::{
    GraphPlanRecord, RecordingRecord, Storage, StorageError, GRAPH_PLAN_RETENTION_SECONDS,
    MAX_PENDING_PLAN_RECORDS, MAX_RECORDING_ID_BYTES, MAX_RECORDING_LIST_ITEMS,
};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MUTATION_RATE_PER_SECOND: f64 = 20.0;
const MUTATION_BURST: f64 = 40.0;
const MAX_MUTATION_BUCKETS: usize = 256;
const MUTATION_BUCKET_RETENTION: Duration = Duration::from_secs(10 * 60);
const MAX_CONTROL_VALUE_DEPTH: usize = 32;
const MAX_CONTROL_STRING_BYTES: usize = 4096;
const MAX_CONTROL_VALUE_COUNT: usize = 8192;
const MAX_EVENT_SUBSCRIPTION_ITEMS: usize = 500;
const MAX_SESSION_LIST_ITEMS: usize = 500;
const MAX_GRAPH_HISTORY_ITEMS: usize = 100;
const MAX_REVISION_CURSOR_BYTES: usize = 20;
const MAX_GRAPH_DIFF_ITEMS: usize = 3;
const MAX_GRAPH_AFFECTED_DESTINATIONS: usize = audiorouter_domain::MAX_NODES_PER_SESSION;
const MAX_DEVICE_LIST_ITEMS: usize = 500;
const MAX_VIRTUAL_DEVICE_LIST_ITEMS: usize = 500;
const MAX_PROCESSOR_CATALOG_ITEMS: usize = 7;
/// Maximum simultaneously armed/active recorder controllers across sessions.
const MAX_ACTIVE_RECORDERS: usize = audiorouter_engine::MAX_AUDIO_TAPS;
/// Maximum number of bounded queue-drain passes a recorder finalization may
/// perform. A producer that keeps refilling a queue must not make a stop
/// operation loop forever; the caller receives a recoverable finalization
/// error and the recorder remains owned by the worker.
const MAX_RECORDER_FINALIZATION_PASSES: usize = 4096;
const MAX_RESPONSE_BANDS: usize = 8;
const MAX_RESPONSE_FREQUENCIES: usize = 256;
const MAX_MEMORY_OPERATION_OUTCOMES: usize = 100;
/// Maximum number of distinct plugin scan roots retained for `plugins.list`.
const MAX_PLUGIN_INVENTORY_ROOTS: usize = 64;
const MAX_PLAN_REQUIRED_SCOPES: usize = 1;
const MAX_PLAN_WARNINGS: usize = 1;
const STATE_CATEGORIES: [&str; 15] = [
    "session.created",
    "session.deleted",
    "graph.committed",
    "runtime.crashed",
    "runtime.started",
    "runtime.activated",
    "runtime.stopped",
    "privacy.muteEnabled",
    "privacy.muteDisabled",
    "virtualDevice.changed",
    "recorder.changed",
    "recording.metadataChanged",
    "recording.renamed",
    "recording.entryRemoved",
    "recording.recycled",
];
const APPLICATION_SNAPSHOT_TTL: std::time::Duration = std::time::Duration::from_millis(100);
const VIRTUAL_DEVICE_PLAN_TTL: Duration = Duration::from_secs(5 * 60);

/// Result returned by a backend-owned encoder worker during graceful stop.
/// `file_finalized` is intentionally explicit so a control-state transition
/// cannot be mistaken for a durable file finalization.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RecorderFinalizationOutcome {
    pub state: String,
    pub file_finalized: bool,
    pub recoverable: bool,
}

/// A finalized file discovered by a recorder worker on the lifecycle thread.
/// The control plane persists this metadata separately from the recorder
/// checkpoint; the realtime tap never constructs or touches library rows.
pub type FinalizedRecording = RecordingRecord;

/// Explicit identity required before a file worker may publish a library row.
/// The worker never guesses session, recorder, or path ownership from a file
/// handle; callers must provide all three on the lifecycle thread.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileRecordingIdentity {
    pub session_id: String,
    pub recorder_id: String,
    pub path: std::path::PathBuf,
}

fn finalized_flac_recording(
    identity: &FileRecordingIdentity,
    run_id: &str,
    start_time: &str,
) -> Result<FinalizedRecording, String> {
    let info = audiorouter_recording::inspect_flac_file(&identity.path)
        .map_err(|error| format!("FLAC library inspection failed: {error:?}"))?;
    let path = identity
        .path
        .to_str()
        .ok_or_else(|| "FLAC recording path is not valid Unicode".to_owned())?
        .to_owned();
    let id = format!(
        "{}-{}-{}",
        identity.session_id, identity.recorder_id, run_id
    );
    if id.len() > MAX_RECORDING_ID_BYTES
        || identity.session_id.is_empty()
        || identity.recorder_id.is_empty()
    {
        return Err("FLAC recording identity exceeds its bound".into());
    }
    Ok(FinalizedRecording {
        id,
        session_id: identity.session_id.clone(),
        recorder_id: identity.recorder_id.clone(),
        path,
        format: "flac".into(),
        channels: u16::from(info.channels),
        sample_rate: info.sample_rate,
        frames: info.frames,
        file_bytes: info.file_bytes,
        start_time: start_time.to_owned(),
        state: "completed".into(),
        missing: false,
        title: None,
        artist: None,
        comment: None,
    })
}

fn finalized_wav_recording(
    identity: &FileRecordingIdentity,
    run_id: &str,
    start_time: &str,
) -> Result<FinalizedRecording, String> {
    let info = audiorouter_recording::inspect_wav_file(&identity.path)
        .map_err(|error| format!("WAV library inspection failed: {error:?}"))?;
    let path = identity
        .path
        .to_str()
        .ok_or_else(|| "WAV recording path is not valid Unicode".to_owned())?
        .to_owned();
    let id = format!(
        "{}-{}-{}",
        identity.session_id, identity.recorder_id, run_id
    );
    if id.len() > MAX_RECORDING_ID_BYTES
        || identity.session_id.is_empty()
        || identity.recorder_id.is_empty()
    {
        return Err("WAV recording identity exceeds its bound".into());
    }
    Ok(FinalizedRecording {
        id,
        session_id: identity.session_id.clone(),
        recorder_id: identity.recorder_id.clone(),
        path,
        format: "wav".into(),
        channels: info.channels,
        sample_rate: info.sample_rate,
        frames: info.frames,
        file_bytes: info.file_bytes,
        start_time: start_time.to_owned(),
        state: "completed".into(),
        missing: false,
        title: None,
        artist: None,
        comment: None,
    })
}

/// Backend-owned recording worker boundary. Implementations own their queue,
/// encoder, and destination handle; the control plane owns the lifecycle
/// decision and will stop a session only after this method reports a finalized
/// file. The frame is the last committed control-plane boundary.
pub trait RecorderWorker: Send {
    /// Exposes the worker's preallocated queue observer for a prepared engine
    /// tap set. The control plane never invokes it from the audio callback.
    fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
        None
    }

    /// Supplies explicit file ownership metadata before lifecycle start.
    /// Workers that do not own a single file reject this configuration rather
    /// than allowing the control plane to guess a library path.
    fn set_library_identity(&mut self, _identity: FileRecordingIdentity) -> Result<(), String> {
        Err("recorder worker does not support single-file library identity".into())
    }

    fn arm(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn start(&mut self, _frame: u64) -> Result<(), String> {
        Ok(())
    }

    fn pause(&mut self, _frame: u64) -> Result<(), String> {
        Ok(())
    }

    fn resume(&mut self, _frame: u64) -> Result<(), String> {
        Ok(())
    }

    fn split(&mut self, _frame: u64) -> Result<(), String> {
        Err("attached recorder worker does not support file splitting".into())
    }

    /// Return durable file metadata produced by the most recent successful
    /// finalization. Implementations that do not own a path may return an
    /// empty list; callers must never infer a library row from that absence.
    fn finalized_recordings(&self) -> Vec<FinalizedRecording> {
        Vec::new()
    }

    fn finalize(&mut self, frame: u64) -> Result<RecorderFinalizationOutcome, String>;
}

/// Concrete file-backed worker used by the backend integration layer.
///
/// The queue is fed by the graph adapter and is never touched by the audio
/// callback during finalization. The control plane may call `finalize` only
/// from its non-realtime lifecycle path; each drain pass is bounded so a
/// malformed or unexpectedly large queue cannot turn one operation into an
/// unbounded inner loop.
pub struct WavRecorderWorker {
    recorder: Option<WavRecorder<std::fs::File>>,
    queue: Arc<RecordingQueue>,
    maximum_chunks_per_pass: usize,
    library_identity: Option<FileRecordingIdentity>,
    started_at: Option<String>,
    run_id: Option<String>,
    finalized_recordings: Vec<FinalizedRecording>,
}

impl WavRecorderWorker {
    pub fn new(
        output: std::fs::File,
        format: WavFormat,
        channels: u16,
        sample_rate: u32,
        queue_capacity: usize,
        maximum_chunks_per_pass: usize,
    ) -> Result<Self, String> {
        Self::new_with_dither(
            output,
            format,
            channels,
            sample_rate,
            false,
            queue_capacity,
            maximum_chunks_per_pass,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_dither(
        output: std::fs::File,
        format: WavFormat,
        channels: u16,
        sample_rate: u32,
        dither: bool,
        queue_capacity: usize,
        maximum_chunks_per_pass: usize,
    ) -> Result<Self, String> {
        if maximum_chunks_per_pass == 0 {
            return Err("maximum recorder drain pass must be positive".into());
        }
        let writer = WavWriter::new(output, format, channels, sample_rate, dither)
            .map_err(|error| format!("WAV writer initialization failed: {error:?}"))?;
        let queue = RecordingQueue::new_pooled(
            queue_capacity,
            usize::from(channels),
            (audiorouter_recording::MAX_RECORDING_CHUNK_SAMPLES / usize::from(channels)).max(1),
        )
        .map_err(|error| format!("recording queue initialization failed: {error:?}"))?;
        Ok(Self {
            recorder: Some(WavRecorder::new(writer)),
            queue: Arc::new(queue),
            maximum_chunks_per_pass,
            library_identity: None,
            started_at: None,
            run_id: None,
            finalized_recordings: Vec::new(),
        })
    }

    /// Enables explicit durable library publication for this file worker.
    pub fn set_library_identity(&mut self, identity: FileRecordingIdentity) {
        self.library_identity = Some(identity);
    }

    pub fn arm(&mut self) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "WAV recorder is already finalized".to_owned())?
            .arm()
            .map_err(|error| format!("WAV recorder arm failed: {error:?}"))
    }

    pub fn start(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "WAV recorder is already finalized".to_owned())?
            .start(frame)
            .map_err(|error| format!("WAV recorder start failed: {error:?}"))?;
        self.started_at = Some(unix_epoch_seconds().to_string());
        self.run_id = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                .to_string(),
        );
        Ok(())
    }

    /// Enqueues caller-prepared samples. This method is intended for the
    /// graph adapter, which must prepare the chunk before entering the audio
    /// callback and treat a rejected chunk as an overrun.
    pub fn try_push(&self, chunk: RecordingChunk) -> Result<(), RecordingChunk> {
        self.queue.try_push(chunk)
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn audio_tap(&self) -> RecorderAudioTap {
        RecorderAudioTap::new(self.queue.clone())
    }
}

impl RecorderWorker for WavRecorderWorker {
    fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
        Some(Arc::new(self.audio_tap()))
    }

    fn set_library_identity(&mut self, identity: FileRecordingIdentity) -> Result<(), String> {
        WavRecorderWorker::set_library_identity(self, identity);
        Ok(())
    }

    fn finalized_recordings(&self) -> Vec<FinalizedRecording> {
        self.finalized_recordings.clone()
    }

    fn arm(&mut self) -> Result<(), String> {
        WavRecorderWorker::arm(self)
    }

    fn start(&mut self, frame: u64) -> Result<(), String> {
        WavRecorderWorker::start(self, frame)
    }

    fn pause(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "WAV recorder is already finalized".to_owned())?
            .pause(frame)
            .map_err(|error| format!("WAV recorder pause failed: {error:?}"))
    }

    fn resume(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "WAV recorder is already finalized".to_owned())?
            .resume(frame)
            .map_err(|error| format!("WAV recorder resume failed: {error:?}"))
    }

    fn finalize(&mut self, frame: u64) -> Result<RecorderFinalizationOutcome, String> {
        let mut recorder = self
            .recorder
            .take()
            .ok_or_else(|| "WAV recorder was finalized more than once".to_owned())?;
        let mut completed = false;
        for _ in 0..MAX_RECORDER_FINALIZATION_PASSES {
            match recorder.stop_and_drain(&self.queue, frame, self.maximum_chunks_per_pass) {
                Ok(_) => {
                    completed = true;
                    break;
                }
                Err(RecordingError::QueueNotEmpty) => continue,
                Err(error) => {
                    self.recorder = Some(recorder);
                    return Err(format!("WAV recorder finalization failed: {error:?}"));
                }
            }
        }
        if !completed {
            self.recorder = Some(recorder);
            return Err("WAV recorder finalization exceeded its bounded drain budget".into());
        }
        let output = recorder
            .finish()
            .map_err(|error| format!("WAV file finalization failed: {error:?}"))?;
        output
            .sync_all()
            .map_err(|error| format!("WAV file sync failed: {error}"))?;
        if let Some(identity) = &self.library_identity {
            let start_time = self
                .started_at
                .as_deref()
                .ok_or_else(|| "WAV finalized before start".to_owned())?;
            let run_id = self
                .run_id
                .as_deref()
                .ok_or_else(|| "WAV finalized without a run identity".to_owned())?;
            self.finalized_recordings =
                vec![finalized_wav_recording(identity, run_id, start_time)?];
        }
        Ok(RecorderFinalizationOutcome {
            state: "completed".into(),
            file_finalized: true,
            recoverable: false,
        })
    }
}

/// File formats supported by the lifecycle-owned recorder factory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileRecorderFormat {
    Wav(WavFormat),
    Flac { bits_per_sample: u8 },
}

impl FileRecorderFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Wav(_) => "wav",
            Self::Flac { .. } => "flac",
        }
    }
}

pub const FILE_RECORDER_CONFIG_VERSION: u32 = 1;

/// Versioned, bounded configuration for lifecycle-owned file recorder
/// creation. The destination root remains a separately approved policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileRecorderConfig<'a> {
    pub version: u32,
    pub session_id: &'a str,
    pub recorder_id: &'a str,
    pub sequence: u64,
    pub format: FileRecorderFormat,
    pub channels: u16,
    pub sample_rate: u32,
    pub dither: bool,
    pub queue_capacity: usize,
    pub maximum_chunks_per_pass: usize,
}

impl FileRecorderConfig<'_> {
    fn validate(&self) -> Result<(), String> {
        if self.version != FILE_RECORDER_CONFIG_VERSION {
            return Err("unsupported file recorder configuration version".into());
        }
        if self.session_id.is_empty()
            || self.recorder_id.is_empty()
            || self.session_id.len() > audiorouter_domain::MAX_ENTITY_ID_BYTES
            || self.recorder_id.len() > audiorouter_domain::MAX_ENTITY_ID_BYTES
        {
            return Err("file recorder identity is empty or exceeds its bound".into());
        }
        if !matches!(self.channels, 1 | 2) || !matches!(self.sample_rate, 44_100 | 48_000) {
            return Err("file recorder format shape is unsupported".into());
        }
        if !matches!(
            self.queue_capacity,
            1..=audiorouter_recording::MAX_RECORDING_QUEUE_CHUNKS
        ) || self.maximum_chunks_per_pass == 0
        {
            return Err("file recorder queue limits are invalid".into());
        }
        if let FileRecorderFormat::Flac { bits_per_sample } = self.format {
            if !matches!(bits_per_sample, 16 | 24) {
                return Err("FLAC bit depth is unsupported".into());
            }
        }
        Ok(())
    }
}

/// Creates a path-owned recorder on the lifecycle thread. The returned path
/// is the exact path created by `RecordingPathPolicy`; the worker is already
/// configured with the same explicit library identity before it is returned.
#[allow(clippy::too_many_arguments)]
pub fn create_file_recorder(
    policy: &RecordingPathPolicy,
    session_id: &str,
    recorder_id: &str,
    sequence: u64,
    format: FileRecorderFormat,
    channels: u16,
    sample_rate: u32,
    dither: bool,
    queue_capacity: usize,
    maximum_chunks_per_pass: usize,
) -> Result<(std::path::PathBuf, Box<dyn RecorderWorker>), String> {
    create_file_recorder_with_config(
        policy,
        &FileRecorderConfig {
            version: FILE_RECORDER_CONFIG_VERSION,
            session_id,
            recorder_id,
            sequence,
            format,
            channels,
            sample_rate,
            dither,
            queue_capacity,
            maximum_chunks_per_pass,
        },
    )
}

pub fn create_file_recorder_with_config(
    policy: &RecordingPathPolicy,
    config: &FileRecorderConfig<'_>,
) -> Result<(std::path::PathBuf, Box<dyn RecorderWorker>), String> {
    config.validate()?;
    let (path, file) = policy
        .create_file(
            config.session_id,
            config.recorder_id,
            config.sequence,
            config.format.extension(),
        )
        .map_err(format_path_policy_error)?;
    let identity = FileRecordingIdentity {
        session_id: config.session_id.to_owned(),
        recorder_id: config.recorder_id.to_owned(),
        path: path.clone(),
    };
    let worker: Box<dyn RecorderWorker> = match config.format {
        FileRecorderFormat::Wav(wav_format) => {
            let mut worker = WavRecorderWorker::new_with_dither(
                file,
                wav_format,
                config.channels,
                config.sample_rate,
                config.dither,
                config.queue_capacity,
                config.maximum_chunks_per_pass,
            )?;
            worker.set_library_identity(identity);
            Box::new(worker)
        }
        FileRecorderFormat::Flac { bits_per_sample } => {
            let mut worker = BufferedFlacRecorderWorker::new(
                file,
                usize::from(config.channels),
                config.sample_rate,
                bits_per_sample,
                config.queue_capacity,
                config.maximum_chunks_per_pass,
            )?;
            worker.set_library_identity(identity);
            Box::new(worker)
        }
    };
    Ok((path, worker))
}

type SegmentedWavFactory = Box<dyn FnMut(u32) -> Result<std::fs::File, RecordingError> + Send>;

/// Control-plane worker that creates bounded WAV segments through the
/// approved recording path policy. File creation and rotation stay on the
/// worker/lifecycle side; the audio tap only submits pooled chunks.
pub struct SegmentedWavRecorderWorker {
    recorder: Option<SegmentedWavRecorder<std::fs::File, SegmentedWavFactory>>,
    queue: Arc<RecordingQueue>,
    maximum_chunks_per_pass: usize,
    session_id: String,
    recorder_id: String,
    format: WavFormat,
    channels: u16,
    sample_rate: u32,
    paths: Arc<std::sync::Mutex<Vec<std::path::PathBuf>>>,
    started_at: Option<String>,
    run_id: Option<String>,
    finalized_recordings: Vec<FinalizedRecording>,
}

impl SegmentedWavRecorderWorker {
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_default_segment_frames(
        policy: RecordingPathPolicy,
        session: &str,
        recorder_name: &str,
        format: WavFormat,
        channels: u16,
        sample_rate: u32,
        queue_capacity: usize,
        maximum_chunks_per_pass: usize,
    ) -> Result<Self, String> {
        let max_segment_frames =
            audiorouter_recording::default_wav_segment_frames(format, channels, sample_rate)
                .map_err(|error| format!("invalid default WAV segment boundary: {error:?}"))?;
        Self::new(
            policy,
            session,
            recorder_name,
            format,
            channels,
            sample_rate,
            queue_capacity,
            maximum_chunks_per_pass,
            max_segment_frames,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        policy: RecordingPathPolicy,
        session: &str,
        recorder_name: &str,
        format: WavFormat,
        channels: u16,
        sample_rate: u32,
        queue_capacity: usize,
        maximum_chunks_per_pass: usize,
        max_segment_frames: u64,
    ) -> Result<Self, String> {
        if maximum_chunks_per_pass == 0 {
            return Err("maximum recorder drain pass must be positive".into());
        }
        let (initial_path, initial_file) = policy
            .create_file(session, recorder_name, 0, "wav")
            .map_err(format_path_policy_error)?;
        let session_id = session.to_owned();
        let recorder_id = recorder_name.to_owned();
        let paths = Arc::new(std::sync::Mutex::new(vec![initial_path]));
        let factory_paths = paths.clone();
        let factory_session = session_id.clone();
        let factory_recorder = recorder_id.clone();
        let factory: SegmentedWavFactory = Box::new(move |index| {
            let (path, file) = policy
                .create_file(&factory_session, &factory_recorder, u64::from(index), "wav")
                .map_err(|error| {
                    RecordingError::Io(std::io::Error::other(format_path_policy_error(error)))
                })?;
            factory_paths
                .lock()
                .map_err(|_| {
                    RecordingError::Io(std::io::Error::other("recording path registry poisoned"))
                })?
                .push(path);
            Ok(file)
        });
        let recorder = SegmentedWavRecorder::new(
            initial_file,
            factory,
            format,
            channels,
            sample_rate,
            false,
            max_segment_frames,
        )
        .map_err(|error| format!("segmented WAV writer initialization failed: {error:?}"))?;
        let queue = RecordingQueue::new_pooled(
            queue_capacity,
            usize::from(channels),
            (audiorouter_recording::MAX_RECORDING_CHUNK_SAMPLES / usize::from(channels)).max(1),
        )
        .map_err(|error| format!("recording queue initialization failed: {error:?}"))?;
        Ok(Self {
            recorder: Some(recorder),
            queue: Arc::new(queue),
            maximum_chunks_per_pass,
            session_id,
            recorder_id,
            format,
            channels,
            sample_rate,
            paths,
            started_at: None,
            run_id: None,
            finalized_recordings: Vec::new(),
        })
    }

    pub fn arm(&mut self) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "segmented WAV recorder is already finalized".to_owned())?
            .arm()
            .map_err(|error| format!("segmented WAV recorder arm failed: {error:?}"))
    }

    pub fn start(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "segmented WAV recorder is already finalized".to_owned())?
            .start(frame)
            .map_err(|error| format!("segmented WAV recorder start failed: {error:?}"))?;
        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_string();
        self.started_at = Some(unix_epoch_seconds().to_string());
        self.run_id = Some(run_id);
        Ok(())
    }

    pub fn try_push(&self, chunk: RecordingChunk) -> Result<(), RecordingChunk> {
        self.queue.try_push(chunk)
    }

    pub fn audio_tap(&self) -> RecorderAudioTap {
        RecorderAudioTap::new(self.queue.clone())
    }
}

impl RecorderWorker for SegmentedWavRecorderWorker {
    fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
        Some(Arc::new(self.audio_tap()))
    }

    fn arm(&mut self) -> Result<(), String> {
        SegmentedWavRecorderWorker::arm(self)
    }

    fn start(&mut self, frame: u64) -> Result<(), String> {
        SegmentedWavRecorderWorker::start(self, frame)
    }

    fn finalized_recordings(&self) -> Vec<FinalizedRecording> {
        self.finalized_recordings.clone()
    }

    fn pause(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "segmented WAV recorder is already finalized".to_owned())?
            .pause(frame)
            .map_err(|error| format!("segmented WAV recorder pause failed: {error:?}"))
    }

    fn resume(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "segmented WAV recorder is already finalized".to_owned())?
            .resume(frame)
            .map_err(|error| format!("segmented WAV recorder resume failed: {error:?}"))
    }

    fn split(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "segmented WAV recorder is already finalized".to_owned())?
            .split(frame)
            .map_err(|error| format!("segmented WAV recorder split failed: {error:?}"))
    }

    fn finalize(&mut self, frame: u64) -> Result<RecorderFinalizationOutcome, String> {
        let mut recorder = self
            .recorder
            .take()
            .ok_or_else(|| "segmented WAV recorder was finalized more than once".to_owned())?;
        let mut completed = false;
        for _ in 0..MAX_RECORDER_FINALIZATION_PASSES {
            match recorder.stop_and_drain(&self.queue, frame, self.maximum_chunks_per_pass) {
                Ok(_) => {
                    completed = true;
                    break;
                }
                Err(RecordingError::QueueNotEmpty) => continue,
                Err(error) => {
                    self.recorder = Some(recorder);
                    return Err(format!("segmented WAV finalization failed: {error:?}"));
                }
            }
        }
        if !completed {
            self.recorder = Some(recorder);
            return Err("segmented WAV finalization exceeded its bounded drain budget".into());
        }
        let outputs = recorder
            .finish()
            .map_err(|error| format!("segmented WAV file finalization failed: {error:?}"))?;
        let paths = self
            .paths
            .lock()
            .map_err(|_| "recording path registry poisoned".to_owned())?
            .clone();
        if outputs.len() != paths.len() {
            return Err("segmented WAV output metadata count mismatch".into());
        }
        let bytes_per_sample = match self.format {
            WavFormat::Pcm16 => 2,
            WavFormat::Pcm24 => 3,
            WavFormat::Float32 => 4,
        };
        let bytes_per_frame = bytes_per_sample * u64::from(self.channels);
        let start_time = self
            .started_at
            .clone()
            .ok_or_else(|| "segmented WAV finalized before start".to_owned())?;
        let run_id = self
            .run_id
            .clone()
            .ok_or_else(|| "segmented WAV finalized without a run identity".to_owned())?;
        let mut finalized = Vec::with_capacity(outputs.len());
        for (index, output) in outputs.into_iter().enumerate() {
            output
                .sync_all()
                .map_err(|error| format!("segmented WAV file sync failed: {error}"))?;
            let file_bytes = output
                .metadata()
                .map_err(|error| format!("segmented WAV metadata failed: {error}"))?
                .len();
            let data_bytes = file_bytes
                .checked_sub(44)
                .ok_or_else(|| "segmented WAV file is shorter than its header".to_owned())?;
            if data_bytes % bytes_per_frame != 0 {
                return Err("segmented WAV data is not frame aligned".into());
            }
            let id = format!(
                "{}-{}-{}-{}",
                self.session_id, self.recorder_id, run_id, index
            );
            if id.len() > MAX_RECORDING_ID_BYTES {
                return Err("segmented WAV recording identity exceeds its bound".into());
            }
            let path = paths[index]
                .to_str()
                .ok_or_else(|| "segmented WAV path is not valid Unicode".to_owned())?
                .to_owned();
            finalized.push(FinalizedRecording {
                id,
                session_id: self.session_id.clone(),
                recorder_id: self.recorder_id.clone(),
                path,
                format: "wav".into(),
                channels: self.channels,
                sample_rate: self.sample_rate,
                frames: data_bytes / bytes_per_frame,
                file_bytes,
                start_time: start_time.clone(),
                state: "completed".into(),
                missing: false,
                title: None,
                artist: None,
                comment: None,
            });
        }
        self.finalized_recordings = finalized;
        Ok(RecorderFinalizationOutcome {
            state: "completed".into(),
            file_finalized: true,
            recoverable: false,
        })
    }
}

fn format_path_policy_error(error: PathPolicyError) -> String {
    format!("recording path policy rejected file creation: {error:?}")
}

/// Concrete buffered-FLAC worker for offline and non-realtime recording
/// integration. The encoder retains compressed output until finalization;
/// native realtime wiring must use the streaming worker once that adapter is
/// available.
pub struct BufferedFlacRecorderWorker {
    recorder: Option<BufferedFlacRecorder>,
    queue: Arc<RecordingQueue>,
    output: Option<std::fs::File>,
    maximum_chunks_per_pass: usize,
    library_identity: Option<FileRecordingIdentity>,
    started_at: Option<String>,
    run_id: Option<String>,
    finalized_recordings: Vec<FinalizedRecording>,
}

impl BufferedFlacRecorderWorker {
    pub fn new(
        output: std::fs::File,
        channels: usize,
        sample_rate: u32,
        bits_per_sample: u8,
        queue_capacity: usize,
        maximum_chunks_per_pass: usize,
    ) -> Result<Self, String> {
        if maximum_chunks_per_pass == 0 {
            return Err("maximum recorder drain pass must be positive".into());
        }
        let recorder = BufferedFlacRecorder::new(channels, sample_rate, bits_per_sample)
            .map_err(|error| format!("FLAC writer initialization failed: {error:?}"))?;
        let queue = RecordingQueue::new_pooled(
            queue_capacity,
            channels,
            (audiorouter_recording::MAX_RECORDING_CHUNK_SAMPLES / channels).max(1),
        )
        .map_err(|error| format!("recording queue initialization failed: {error:?}"))?;
        Ok(Self {
            recorder: Some(recorder),
            queue: Arc::new(queue),
            output: Some(output),
            maximum_chunks_per_pass,
            library_identity: None,
            started_at: None,
            run_id: None,
            finalized_recordings: Vec::new(),
        })
    }

    /// Enables explicit durable library publication for this file worker.
    pub fn set_library_identity(&mut self, identity: FileRecordingIdentity) {
        self.library_identity = Some(identity);
    }

    pub fn arm(&mut self) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "FLAC recorder is already finalized".to_owned())?
            .arm()
            .map_err(|error| format!("FLAC recorder arm failed: {error:?}"))
    }

    pub fn start(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "FLAC recorder is already finalized".to_owned())?
            .start(frame)
            .map_err(|error| format!("FLAC recorder start failed: {error:?}"))?;
        self.started_at = Some(unix_epoch_seconds().to_string());
        self.run_id = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                .to_string(),
        );
        Ok(())
    }

    pub fn try_push(&self, chunk: RecordingChunk) -> Result<(), RecordingChunk> {
        self.queue.try_push(chunk)
    }

    pub fn audio_tap(&self) -> RecorderAudioTap {
        RecorderAudioTap::new(self.queue.clone())
    }
}

impl RecorderWorker for BufferedFlacRecorderWorker {
    fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
        Some(Arc::new(self.audio_tap()))
    }

    fn set_library_identity(&mut self, identity: FileRecordingIdentity) -> Result<(), String> {
        BufferedFlacRecorderWorker::set_library_identity(self, identity);
        Ok(())
    }

    fn finalized_recordings(&self) -> Vec<FinalizedRecording> {
        self.finalized_recordings.clone()
    }

    fn arm(&mut self) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "FLAC recorder is already finalized".to_owned())?
            .arm()
            .map_err(|error| format!("FLAC recorder arm failed: {error:?}"))
    }

    fn start(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "FLAC recorder is already finalized".to_owned())?
            .start(frame)
            .map_err(|error| format!("FLAC recorder start failed: {error:?}"))?;
        self.started_at = Some(unix_epoch_seconds().to_string());
        self.run_id = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                .to_string(),
        );
        Ok(())
    }

    fn pause(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "FLAC recorder is already finalized".to_owned())?
            .pause(frame)
            .map_err(|error| format!("FLAC recorder pause failed: {error:?}"))
    }

    fn resume(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "FLAC recorder is already finalized".to_owned())?
            .resume(frame)
            .map_err(|error| format!("FLAC recorder resume failed: {error:?}"))
    }

    fn finalize(&mut self, frame: u64) -> Result<RecorderFinalizationOutcome, String> {
        let mut recorder = self
            .recorder
            .take()
            .ok_or_else(|| "FLAC recorder was finalized more than once".to_owned())?;
        let mut completed = false;
        for _ in 0..MAX_RECORDER_FINALIZATION_PASSES {
            match recorder.stop_and_drain(&self.queue, frame, self.maximum_chunks_per_pass) {
                Ok(_) => {
                    completed = true;
                    break;
                }
                Err(RecordingError::QueueNotEmpty) => continue,
                Err(error) => {
                    self.recorder = Some(recorder);
                    return Err(format!("FLAC recorder finalization failed: {error:?}"));
                }
            }
        }
        if !completed {
            self.recorder = Some(recorder);
            return Err("FLAC recorder finalization exceeded its bounded drain budget".into());
        }
        let encoded = recorder
            .finish()
            .map_err(|error| format!("FLAC encoding failed: {error:?}"))?;
        let mut output = self
            .output
            .take()
            .ok_or_else(|| "FLAC output was already finalized".to_owned())?;
        std::io::Write::write_all(&mut output, &encoded)
            .and_then(|()| output.sync_all())
            .map_err(|error| format!("FLAC file finalization failed: {error}"))?;
        if let Some(identity) = &self.library_identity {
            let start_time = self
                .started_at
                .as_deref()
                .ok_or_else(|| "FLAC finalized before start".to_owned())?;
            let run_id = self
                .run_id
                .as_deref()
                .ok_or_else(|| "FLAC finalized without a run identity".to_owned())?;
            self.finalized_recordings =
                vec![finalized_flac_recording(identity, run_id, start_time)?];
        }
        Ok(RecorderFinalizationOutcome {
            state: "completed".into(),
            file_finalized: true,
            recoverable: false,
        })
    }
}

/// Concrete incremental FLAC worker intended for eventual realtime graph
/// attachment. Unlike the buffered worker, encoded frames are written as the
/// queue is drained; only the seekable STREAMINFO patch remains for finish.
pub struct StreamingFlacRecorderWorker {
    recorder: Option<StreamingFlacRecorder<std::fs::File>>,
    queue: Arc<RecordingQueue>,
    maximum_chunks_per_pass: usize,
    library_identity: Option<FileRecordingIdentity>,
    started_at: Option<String>,
    run_id: Option<String>,
    finalized_recordings: Vec<FinalizedRecording>,
}

impl StreamingFlacRecorderWorker {
    pub fn new(
        output: std::fs::File,
        channels: u16,
        sample_rate: u32,
        bits_per_sample: u8,
        queue_capacity: usize,
        maximum_chunks_per_pass: usize,
    ) -> Result<Self, String> {
        if maximum_chunks_per_pass == 0 {
            return Err("maximum recorder drain pass must be positive".into());
        }
        let writer =
            StreamingFlacWriter::new(output, channels, sample_rate, bits_per_sample, false)
                .map_err(|error| {
                    format!("streaming FLAC writer initialization failed: {error:?}")
                })?;
        let queue = RecordingQueue::new_pooled(
            queue_capacity,
            usize::from(channels),
            (audiorouter_recording::MAX_RECORDING_CHUNK_SAMPLES / usize::from(channels)).max(1),
        )
        .map_err(|error| format!("recording queue initialization failed: {error:?}"))?;
        Ok(Self {
            recorder: Some(StreamingFlacRecorder::new(writer)),
            queue: Arc::new(queue),
            maximum_chunks_per_pass,
            library_identity: None,
            started_at: None,
            run_id: None,
            finalized_recordings: Vec::new(),
        })
    }

    /// Enables explicit durable library publication for this file worker.
    pub fn set_library_identity(&mut self, identity: FileRecordingIdentity) {
        self.library_identity = Some(identity);
    }

    pub fn arm(&mut self) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "streaming FLAC recorder is already finalized".to_owned())?
            .arm()
            .map_err(|error| format!("streaming FLAC recorder arm failed: {error:?}"))
    }

    pub fn start(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "streaming FLAC recorder is already finalized".to_owned())?
            .start(frame)
            .map_err(|error| format!("streaming FLAC recorder start failed: {error:?}"))?;
        self.started_at = Some(unix_epoch_seconds().to_string());
        self.run_id = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                .to_string(),
        );
        Ok(())
    }

    pub fn try_push(&self, chunk: RecordingChunk) -> Result<(), RecordingChunk> {
        self.queue.try_push(chunk)
    }

    pub fn audio_tap(&self) -> RecorderAudioTap {
        RecorderAudioTap::new(self.queue.clone())
    }
}

impl RecorderWorker for StreamingFlacRecorderWorker {
    fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
        Some(Arc::new(self.audio_tap()))
    }

    fn set_library_identity(&mut self, identity: FileRecordingIdentity) -> Result<(), String> {
        StreamingFlacRecorderWorker::set_library_identity(self, identity);
        Ok(())
    }

    fn finalized_recordings(&self) -> Vec<FinalizedRecording> {
        self.finalized_recordings.clone()
    }

    fn arm(&mut self) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "streaming FLAC recorder is already finalized".to_owned())?
            .arm()
            .map_err(|error| format!("streaming FLAC recorder arm failed: {error:?}"))
    }

    fn start(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "streaming FLAC recorder is already finalized".to_owned())?
            .start(frame)
            .map_err(|error| format!("streaming FLAC recorder start failed: {error:?}"))?;
        self.started_at = Some(unix_epoch_seconds().to_string());
        self.run_id = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                .to_string(),
        );
        Ok(())
    }

    fn pause(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "streaming FLAC recorder is already finalized".to_owned())?
            .pause(frame)
            .map_err(|error| format!("streaming FLAC recorder pause failed: {error:?}"))
    }

    fn resume(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "streaming FLAC recorder is already finalized".to_owned())?
            .resume(frame)
            .map_err(|error| format!("streaming FLAC recorder resume failed: {error:?}"))
    }

    fn finalize(&mut self, frame: u64) -> Result<RecorderFinalizationOutcome, String> {
        let mut recorder = self
            .recorder
            .take()
            .ok_or_else(|| "streaming FLAC recorder was finalized more than once".to_owned())?;
        let mut completed = false;
        for _ in 0..MAX_RECORDER_FINALIZATION_PASSES {
            match recorder.stop_and_drain(&self.queue, frame, self.maximum_chunks_per_pass) {
                Ok(_) => {
                    completed = true;
                    break;
                }
                Err(RecordingError::QueueNotEmpty) => continue,
                Err(error) => {
                    self.recorder = Some(recorder);
                    return Err(format!("streaming FLAC finalization failed: {error:?}"));
                }
            }
        }
        if !completed {
            self.recorder = Some(recorder);
            return Err(
                "streaming FLAC recorder finalization exceeded its bounded drain budget".into(),
            );
        }
        let output = recorder
            .finish()
            .map_err(|error| format!("streaming FLAC file finalization failed: {error:?}"))?;
        output
            .sync_all()
            .map_err(|error| format!("streaming FLAC file sync failed: {error}"))?;
        if let Some(identity) = &self.library_identity {
            let start_time = self
                .started_at
                .as_deref()
                .ok_or_else(|| "streaming FLAC finalized before start".to_owned())?;
            let run_id = self
                .run_id
                .as_deref()
                .ok_or_else(|| "streaming FLAC finalized without a run identity".to_owned())?;
            self.finalized_recordings =
                vec![finalized_flac_recording(identity, run_id, start_time)?];
        }
        Ok(RecorderFinalizationOutcome {
            state: "completed".into(),
            file_finalized: true,
            recoverable: false,
        })
    }
}

/// Allocation-free bridge from a processed engine block to a pooled recorder.
/// The worker owns the queue's consumer side; this tap owns no audio buffers
/// and only uses chunks acquired from the worker's preallocated pool.
pub struct RecorderAudioTap {
    queue: Arc<RecordingQueue>,
}

impl RecorderAudioTap {
    fn new(queue: Arc<RecordingQueue>) -> Self {
        Self { queue }
    }
}

impl AudioTap for RecorderAudioTap {
    fn on_processed_block(&self, start_frame: u64, block: &AudioBlock) {
        let Some(mut chunk) = self.queue.try_acquire() else {
            return;
        };
        let sample_count = block.channels().saturating_mul(block.frames());
        if sample_count > chunk.samples.len() {
            self.queue.recycle(chunk);
            return;
        }
        chunk.samples.truncate(sample_count);
        chunk.start_frame = start_frame;
        for frame in 0..block.frames() {
            for channel in 0..block.channels() {
                let sample = block
                    .channel(channel)
                    .and_then(|samples| samples.get(frame))
                    .copied()
                    .filter(|sample| sample.is_finite())
                    .unwrap_or(0.0);
                chunk.samples[frame * block.channels() + channel] = sample;
            }
        }
        if let Err(chunk) = self.queue.try_commit(chunk) {
            self.queue.recycle(chunk);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum VirtualBusOperation {
    Create { id: EntityId, name: String },
    Rename { id: EntityId, name: String },
    SetEnabled { id: EntityId, enabled: bool },
    Delete { id: EntityId },
}

#[derive(Clone, Debug)]
struct VirtualBusPlan {
    operation: VirtualBusOperation,
    expires_at: Instant,
}

fn unix_epoch_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn unix_epoch_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn allocate_timestamped_plan_id<F>(
    prefix: &str,
    timestamp_millis: u128,
    counter: &mut u64,
    is_occupied: F,
    exhausted_message: &'static str,
) -> Result<EntityId, ControlError>
where
    F: Fn(&EntityId) -> bool,
{
    loop {
        let current = *counter;
        let plan_id = EntityId::new(format!("{prefix}-{timestamp_millis}-{current}"));
        if !is_occupied(&plan_id) {
            *counter = current.saturating_add(1);
            return Ok(plan_id);
        }
        if current == u64::MAX {
            return Err(ControlError::InvalidRequest(exhausted_message.into()));
        }
        *counter += 1;
    }
}

fn allocate_counter_plan_id<F>(
    prefix: &str,
    counter: &mut u64,
    is_occupied: F,
    exhausted_message: &'static str,
) -> Result<EntityId, ControlError>
where
    F: Fn(&EntityId) -> bool,
{
    loop {
        let current = *counter;
        let plan_id = EntityId::new(format!("{prefix}-{current}"));
        if !is_occupied(&plan_id) {
            *counter = current.saturating_add(1);
            return Ok(plan_id);
        }
        if current == u64::MAX {
            return Err(ControlError::InvalidRequest(exhausted_message.into()));
        }
        *counter += 1;
    }
}

fn remaining_persisted_plan_duration(
    expires_at: i64,
    now: i64,
    maximum: Duration,
) -> Option<Duration> {
    expires_at
        .checked_sub(now)
        .filter(|remaining| *remaining > 0)
        .and_then(|remaining| u64::try_from(remaining).ok())
        .map(Duration::from_secs)
        .map(|remaining| remaining.min(maximum))
}

#[derive(Debug)]
struct MutationBucket {
    tokens: f64,
    last_refill: Instant,
}

#[derive(Debug, Default)]
struct MutationRateLimiter {
    buckets: HashMap<String, MutationBucket>,
}

impl MutationRateLimiter {
    fn allow(&mut self, client_id: &str) -> Result<(), u64> {
        self.allow_at(client_id, Instant::now())
    }

    fn allow_at(&mut self, client_id: &str, now: Instant) -> Result<(), u64> {
        if !self.buckets.contains_key(client_id) && self.buckets.len() >= MAX_MUTATION_BUCKETS {
            self.buckets.retain(|_, bucket| {
                now.saturating_duration_since(bucket.last_refill) <= MUTATION_BUCKET_RETENTION
            });
            if self.buckets.len() >= MAX_MUTATION_BUCKETS {
                return Err(1_000);
            }
        }
        let bucket = self
            .buckets
            .entry(client_id.to_owned())
            .or_insert_with(|| MutationBucket {
                tokens: MUTATION_BURST,
                last_refill: now,
            });
        let elapsed = now
            .saturating_duration_since(bucket.last_refill)
            .as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * MUTATION_RATE_PER_SECOND).min(MUTATION_BURST);
        bucket.last_refill = now;
        if bucket.tokens < 1.0 {
            let retry_after_ms =
                (((1.0 - bucket.tokens) / MUTATION_RATE_PER_SECOND) * 1000.0).ceil() as u64;
            return Err(retry_after_ms.max(1));
        }
        bucket.tokens -= 1.0;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MethodDescription {
    pub name: &'static str,
    pub description: &'static str,
    pub permission: audiorouter_domain::PermissionScope,
    pub side_effect: audiorouter_domain::SideEffectClass,
    pub input_schema: Value,
    pub output_schema: Value,
}

impl From<ApiMethodSpec> for MethodDescription {
    fn from(spec: ApiMethodSpec) -> Self {
        Self {
            name: spec.name,
            description: method_description(spec.name),
            permission: spec.permission,
            side_effect: spec.side_effect,
            input_schema: method_input_schema(spec.name),
            output_schema: method_output_schema(spec.name),
        }
    }
}

fn method_description(name: &str) -> &'static str {
    match name {
        "system.describe" => "Describe protocol capabilities, methods, node types, and limits.",
        "system.handshake" => "Negotiate a compatible protocol version before requests.",
        "status.get" => "Return backend, runtime, and audio availability status.",
        "system.diagnostics" => "Return a redacted backend diagnostic snapshot.",
        "clients.list" => "List enrolled local client identities and roles.",
        "clients.authorize" => "Authorize a client with an explicit built-in role.",
        "clients.revoke" => "Revoke a client enrollment without deleting its audit record.",
        "operations.get" => "Read the durable outcome of an idempotent operation.",
        "operations.cancel" => "Cancel a pending operation when it has not completed.",
        "recordings.list" => "List persisted recording metadata without touching audio files.",
        "recorders.list" => "List live in-memory recorder states and frame boundaries.",
        "recorders.create" => "Create and attach an unarmed file recorder under the approved root.",
        "recorders.arm" => "Arm a session recorder without opening an audio device.",
        "recorders.start" => "Start a recorder at an explicit engine frame boundary.",
        "recorders.pause" => "Pause a recorder at an explicit engine frame boundary.",
        "recorders.resume" => "Resume a recorder at an explicit engine frame boundary.",
        "recorders.split" => "Split a recorder at an explicit engine frame boundary.",
        "recorders.stop" => "Stop a recorder at an explicit engine frame boundary.",
        "recordings.get" => {
            "Read one persisted recording metadata resource without touching its file."
        }
        "recordings.recovery" => {
            "Read a validated recorder recovery checkpoint without touching audio files."
        }
        "recordings.reveal" => "Reveal a recorded file in the operating system file browser.",
        "recordings.preview" => "Inspect recording file format metadata without decoding audio.",
        "recordings.setMetadata" => {
            "Update recording metadata without changing audio content or path."
        }
        "recordings.rename" => "Rename a recording within its approved directory.",
        "safety.setPrivacyMute" => "Latch or clear process-local privacy mute for capture paths.",
        "recovery.clearSafeMode" => {
            "Clear the latched crash-recovery safe mode after an operator confirms stability."
        }
        "startup.get" => "Report the desired sign-in startup policy and registration capability.",
        "startup.plan" => "Preview a sign-in startup policy change without applying OS registration.",
        "startup.apply" => "Apply a validated sign-in startup policy when native registration is available.",
        "recordings.removeEntry" => "Remove a recording library entry without deleting its file.",
        "recordings.recycle" => {
            "Move a recording to the operating system Recycle Bin after explicit confirmation."
        }
        "devices.list" => "List authoritative audio endpoint descriptors.",
        "plugins.scan" => "Inspect an explicitly selected plugin directory without loading plugin code.",
        "plugins.list" => "List the last bounded plugin scan inventory without scanning or loading plugin code.",
        "plugins.retry" => "Explicitly refresh a plugin inventory after a prior scan failure or quarantine decision.",
        "plugins.inspect" => "Inspect one explicitly selected plugin binary without loading plugin code.",
        "virtualDevices.list" => "List managed virtual bus desired state without activating endpoints.",
        "virtualDevices.plan" => "Validate a managed virtual bus lifecycle change without applying it.",
        "virtualDevices.apply" => "Apply a validated virtual bus lifecycle plan to desired state.",
        "apps.list" | "applications.list" => {
            "List discoverable application identities and observed Windows audio-session activity for binding."
        }
        "nodes.types" => "List supported node types and their availability.",
        "routes.inspect" => "Inspect upstream route provenance for a destination node.",
        "graph.history" => "List bounded committed graph revisions for a session.",
        "graph.undoPlan" => "Prepare an inverse graph plan from retained history.",
        "events.subscribe" => "Replay retained state events from an optional cursor.",
        "nodes.describe" => "Describe node types, availability, and realtime cost.",
        "presets.list" => "List explainable built-in processing presets.",
        "processors.list" => "List built-in DSP processor metadata and availability.",
        "processors.response" => "Evaluate a bounded parametric-EQ magnitude response using the DSP coefficient path.",
        "sessions.get" => "Return one session resource by opaque identifier.",
        "sessions.export" => "Export one persisted canonical session document without changing state.",
        "sessions.importPlan" => "Validate a stopped session import without persisting it.",
        "sessions.importCommit" => "Commit a previously validated stopped session import.",
        "sessions.list" => "List session resources with stable cursor pagination.",
        "sessions.create" => "Create a validated stopped session resource.",
        "sessions.duplicate" => "Clone a session into a new stopped resource.",
        "sessions.delete" => "Delete a stopped session resource and its history.",
        "graph.plan" => "Validate and preview a graph candidate without mutation.",
        "graph.commit" => "Commit an unexpired graph plan with idempotent mutation.",
        "session.start" | "sessions.start" => {
            "Start a session runtime through the available backend."
        }
        "session.stop" | "sessions.stop" => {
            "Stop a session runtime and publish its lifecycle result."
        }
        _ => "Invoke an AudioRouter control-plane method.",
    }
}

fn object_schema(properties: Value, required: &[&str]) -> Value {
    let required = required
        .iter()
        .map(|value| Value::String((*value).into()))
        .collect::<Vec<_>>();
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

fn method_input_schema(name: &str) -> Value {
    match name {
        "system.handshake" => object_schema(
            json!({
                "protocolVersion": {
                    "type": "object",
                    "properties": {
                        "major": { "type": "integer", "minimum": 0 },
                        "minor": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["major", "minor"],
                    "additionalProperties": false
                }
            }),
            &["protocolVersion"],
        ),
        "clients.authorize" => object_schema(
            json!({
                "clientId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "role": { "enum": ["observer", "editor", "operator"] },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["clientId", "role", "idempotencyKey"],
        ),
        "clients.revoke" => object_schema(
            json!({
                "clientId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["clientId", "idempotencyKey"],
        ),
        "operations.get" => object_schema(
            json!({ "operationId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["operationId"],
        ),
        "operations.cancel" => object_schema(
            json!({
                "operationId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["operationId", "idempotencyKey"],
        ),
        "recordings.list" => object_schema(
            json!({
                "sessionId": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "cursor": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_RECORDING_LIST_ITEMS }
            }),
            &[],
        ),
        "recorders.list" => object_schema(json!({}), &[]),
        "recorders.create" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "recorderId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "format": { "enum": ["wavPcm16", "wavPcm24", "wavFloat32", "flac16", "flac24"] },
                "sequence": { "type": "integer", "minimum": 0 },
                "channels": { "type": "integer", "enum": [1, 2] },
                "sampleRate": { "type": "integer", "enum": [44100, 48000] },
                "dither": { "type": "boolean" },
                "queueCapacity": { "type": "integer", "minimum": 1, "maximum": audiorouter_recording::MAX_RECORDING_QUEUE_CHUNKS },
                "maximumChunksPerPass": { "type": "integer", "minimum": 1 },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &[
                "sessionId",
                "recorderId",
                "format",
                "sequence",
                "channels",
                "sampleRate",
                "queueCapacity",
                "maximumChunksPerPass",
                "idempotencyKey",
            ],
        ),
        "recorders.arm" => recorder_input_schema(false),
        "recorders.start" | "recorders.pause" | "recorders.resume" | "recorders.split"
        | "recorders.stop" => recorder_input_schema(true),
        "devices.list" => object_schema(
            json!({
                "cursor": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_DEVICE_LIST_ITEMS },
                "includeInactive": { "type": "boolean" }
            }),
            &[],
        ),
        "plugins.scan" => object_schema(
            json!({
                "directory": { "type": "string", "minLength": 1 }
            }),
            &["directory"],
        ),
        "plugins.list" => object_schema(
            json!({
                "directory": { "type": "string", "minLength": 1 }
            }),
            &["directory"],
        ),
        "plugins.retry" => object_schema(
            json!({
                "directory": { "type": "string", "minLength": 1 },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["directory", "idempotencyKey"],
        ),
        "plugins.inspect" => object_schema(
            json!({
                "path": { "type": "string", "minLength": 1 }
            }),
            &["path"],
        ),
        "virtualDevices.list" => object_schema(
            json!({
                "cursor": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_VIRTUAL_DEVICE_LIST_ITEMS }
            }),
            &[],
        ),
        "virtualDevices.plan" => object_schema(
            json!({
                "operation": virtual_device_operation_schema()
            }),
            &["operation"],
        ),
        "virtualDevices.apply" => object_schema(
            json!({
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["planId", "idempotencyKey"],
        ),
        "recordings.get" => object_schema(
            json!({ "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES } }),
            &["recordingId"],
        ),
        "recordings.recovery" => object_schema(
            json!({
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "cursor": { "type": ["string", "null"], "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_RECORDING_LIST_ITEMS }
            }),
            &[],
        ),
        "recordings.reveal" => object_schema(
            json!({ "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES } }),
            &["recordingId"],
        ),
        "recordings.preview" => object_schema(
            json!({ "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES } }),
            &["recordingId"],
        ),
        "recordings.setMetadata" => object_schema(
            json!({
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "title": { "type": ["string", "null"], "maxLength": 256 },
                "artist": { "type": ["string", "null"], "maxLength": 256 },
                "comment": { "type": ["string", "null"], "maxLength": 256 },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["recordingId", "idempotencyKey"],
        ),
        "recordings.rename" => object_schema(
            json!({
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "newPath": { "type": "string", "minLength": 1 },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["recordingId", "newPath", "idempotencyKey"],
        ),
        "safety.setPrivacyMute" => object_schema(
            json!({
                "muted": { "type": "boolean" },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["muted", "idempotencyKey"],
        ),
        "recovery.clearSafeMode" => object_schema(
            json!({ "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["idempotencyKey"],
        ),
        "startup.get" => object_schema(json!({}), &[]),
        "startup.plan" => object_schema(json!({ "enabled": { "type": "boolean" } }), &["enabled"]),
        "startup.apply" => object_schema(
            json!({
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["planId", "idempotencyKey"],
        ),
        "recordings.removeEntry" => object_schema(
            json!({ "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES }, "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES } }),
            &["recordingId", "idempotencyKey"],
        ),
        "recordings.recycle" => object_schema(
            json!({
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "confirm": { "type": "boolean" },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["recordingId"],
        ),
        "sessions.get" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } }),
            &["sessionId"],
        ),
        "sessions.export" => object_schema(
            json!({ "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } }),
            &["sessionId"],
        ),
        "sessions.importPlan" => {
            object_schema(json!({ "session": session_item_schema() }), &["session"])
        }
        "sessions.importCommit" => object_schema(
            json!({
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["planId", "idempotencyKey"],
        ),
        "sessions.delete" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["sessionId", "idempotencyKey"],
        ),
        "session.start" | "sessions.start" | "session.stop" | "sessions.stop" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["sessionId", "idempotencyKey"],
        ),
        "sessions.list" => object_schema(
            json!({
                "cursor": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_SESSION_LIST_ITEMS }
            }),
            &[],
        ),
        "sessions.create" => object_schema(
            json!({
                "session": session_item_schema(),
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["session", "idempotencyKey"],
        ),
        "sessions.duplicate" => object_schema(
            json!({
                "sourceSessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "name": { "type": ["string", "null"] },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
            }),
            &["sourceSessionId", "sessionId", "idempotencyKey"],
        ),
        "routes.inspect" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "destinationNode": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
            }),
            &["sessionId", "destinationNode"],
        ),
        "graph.history" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "cursor": { "type": ["string", "null"], "maxLength": MAX_REVISION_CURSOR_BYTES },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_GRAPH_HISTORY_ITEMS }
            }),
            &["sessionId"],
        ),
        "graph.undoPlan" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "baseRevision": { "type": "integer", "minimum": 0 }
            }),
            &["sessionId", "baseRevision"],
        ),
        "events.subscribe" => object_schema(
            json!({
                "afterSequence": { "type": "integer", "minimum": 0 },
                "backendEpoch": { "type": "integer", "minimum": 0 },
                "categories": {
                    "type": "array",
                    "items": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "maxItems": 32
                },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_EVENT_SUBSCRIPTION_ITEMS },
                "sessionId": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
            }),
            &[],
        ),
        "graph.plan" => object_schema(
            json!({
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "baseRevision": { "type": "integer", "minimum": 0 },
                "candidate": session_item_schema()
            }),
            &["sessionId", "baseRevision", "candidate"],
        ),
        "graph.commit" => object_schema(
            json!({
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "baseRevision": { "type": "integer", "minimum": 0 },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES },
                "acknowledgments": {
                    "type": ["array", "null"],
                    "items": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "maxItems": 100
                }
            }),
            &["planId", "baseRevision", "idempotencyKey"],
        ),
        "processors.response" => object_schema(
            json!({
                "sampleRateHz": { "type": "number", "minimum": 8000, "maximum": 192000 },
                "bands": { "type": "array", "maxItems": MAX_RESPONSE_BANDS, "items": {
                    "type": "object", "properties": {
                        "enabled": { "type": "boolean" },
                        "type": { "enum": ["peaking", "lowShelf", "highShelf", "lowPass", "highPass", "notch"] },
                        "frequencyHz": { "type": "number", "minimum": 20, "maximum": 20000 },
                        "q": { "type": "number", "minimum": 0.1, "maximum": 20 },
                        "gainDb": { "type": "number", "minimum": -24, "maximum": 24 }
                    }, "required": ["type", "frequencyHz", "q", "gainDb"], "additionalProperties": false
                }},
                "frequenciesHz": { "type": "array", "minItems": 1, "maxItems": MAX_RESPONSE_FREQUENCIES, "items": { "type": "number", "minimum": 1, "maximum": 96000 } }
            }),
            &["sampleRateHz", "bands", "frequenciesHz"],
        ),
        _ => object_schema(json!({}), &[]),
    }
}

fn recorder_input_schema(frame_required: bool) -> Value {
    let mut properties = json!({
        "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "idempotencyKey": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES }
    });
    if frame_required {
        properties["frame"] = json!({ "type": "integer", "minimum": 0 });
    }
    if frame_required {
        object_schema(properties, &["sessionId", "frame", "idempotencyKey"])
    } else {
        object_schema(properties, &["sessionId", "idempotencyKey"])
    }
}

fn method_output_schema(name: &str) -> Value {
    match name {
        "recorders.create" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "recorderId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "format": { "enum": ["wavPcm16", "wavPcm24", "wavFloat32", "flac16", "flac24"] },
                "path": { "type": "string", "minLength": 1 },
                "state": { "const": "idle" },
                "armed": { "const": false }
            },
            "required": ["sessionId", "recorderId", "format", "path", "state", "armed"],
            "additionalProperties": false
        }),
        "recorders.list" => json!({
            "type": "array",
            "maxItems": audiorouter_domain::MAX_ACTIVE_SESSIONS,
            "items": {
                "type": "object",
                "properties": {
                    "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                    "state": { "enum": ["idle", "armed", "recording", "paused", "stopping", "completed", "failed"] },
                    "lastFrame": { "type": ["integer", "null"], "minimum": 0 }
                },
                "required": ["sessionId", "state", "lastFrame"],
                "additionalProperties": false
            }
        }),
        "recorders.arm" | "recorders.start" | "recorders.pause" | "recorders.resume"
        | "recorders.split" | "recorders.stop" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "enum": ["idle", "armed", "recording", "paused", "stopping", "completed", "failed"] },
                "parts": { "type": "array", "maxItems": audiorouter_recording::MAX_CHECKPOINT_PARTS },
                "pauses": { "type": "array", "maxItems": audiorouter_recording::MAX_CHECKPOINT_PAUSES },
                "lastFrame": { "type": ["integer", "null"] }
            },
            "required": ["sessionId", "state", "parts", "pauses", "lastFrame"],
            "additionalProperties": false
        }),
        "system.describe" => json!({
            "type": "object",
            "properties": {
                "protocolVersion": {
                    "type": "object",
                    "properties": {
                        "major": { "type": "integer", "minimum": 0 },
                        "minor": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["major", "minor"],
                    "additionalProperties": false
                },
                "schemaVersion": { "type": "integer", "minimum": 0 },
                "build": { "type": "string", "minLength": 1 },
                "methods": {
                    "type": "array",
                    "maxItems": API_METHODS.len(),
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "minLength": 1 },
                            "description": { "type": "string", "minLength": 1 },
                            "permission": { "type": "string", "minLength": 1 },
                            "sideEffect": { "type": "string", "minLength": 1 },
                            "inputSchema": { "type": "object" },
                            "outputSchema": { "type": "object" }
                        },
                        "required": ["name", "description", "permission", "sideEffect", "inputSchema", "outputSchema"],
                        "additionalProperties": false
                    }
                },
                "nodeTypes": { "type": "array", "maxItems": audiorouter_domain::node_registry().len(), "items": { "type": "object" } },
                "processors": { "type": "array", "maxItems": MAX_PROCESSOR_CATALOG_ITEMS, "items": processor_item_schema() },
                "presets": {
                    "type": "object",
                    "properties": {
                        "voiceChains": {
                            "type": "array",
                            "maxItems": audiorouter_dsp::VoiceChainPresetId::ALL.len(),
                            "items": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "string", "minLength": 1 },
                                    "version": { "const": 1 },
                                    "name": { "type": "string", "minLength": 1 },
                                    "description": { "type": "string", "minLength": 1 }
                                },
                                "required": ["id", "version", "name", "description"],
                                "additionalProperties": false
                            }
                        },
                        "eq": {
                            "type": "array",
                            "maxItems": audiorouter_dsp::EqPresetId::ALL.len(),
                            "items": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "string", "minLength": 1 },
                                    "version": { "const": 1 },
                                    "name": { "type": "string", "minLength": 1 },
                                    "description": { "type": "string", "minLength": 1 }
                                },
                                "required": ["id", "version", "name", "description"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["voiceChains", "eq"],
                    "additionalProperties": false
                },
                "limits": {
                    "type": "object",
                    "properties": {
                        "maxNodesPerSession": { "type": "integer", "minimum": 1 },
                        "maxEdgesPerSession": { "type": "integer", "minimum": 1 },
                        "maxNodesGlobal": { "type": "integer", "minimum": 1 },
                        "maxEdgesGlobal": { "type": "integer", "minimum": 1 },
                        "maxSessionsGlobal": { "type": "integer", "minimum": 1 },
                        "maxActiveSessions": { "type": "integer", "minimum": 1 },
                        "maxActiveRecorders": { "type": "integer", "minimum": 1 },
                        "maxClientEnrollments": { "type": "integer", "minimum": 1 },
                        "maxOperationJournalEntries": { "type": "integer", "minimum": 1 },
                        "maxVirtualBuses": { "type": "integer", "minimum": 1 },
                        "maxVirtualBusNameChars": { "type": "integer", "minimum": 1 },
                        "maxEntityIdBytes": { "type": "integer", "minimum": 1 },
                        "maxDisplayNameBytes": { "type": "integer", "minimum": 1 },
                        "maxPortNameBytes": { "type": "integer", "minimum": 1 },
                        "maxPortsPerNode": { "type": "integer", "minimum": 1 },
                        "maxChannelMatrixCoefficients": { "type": "integer", "minimum": 1 },
                        "maxControlValueDepth": { "type": "integer", "minimum": 1 },
                        "maxControlStringBytes": { "type": "integer", "minimum": 1 },
                        "maxControlValueCount": { "type": "integer", "minimum": 1 },
                        "maxMethodNameBytes": { "type": "integer", "minimum": 1 },
                        "maxRequestIdBytes": { "type": "integer", "minimum": 1 },
                        "maxRevisionCursorBytes": { "type": "integer", "minimum": 1 }
                    },
                    "required": ["maxNodesPerSession", "maxEdgesPerSession", "maxNodesGlobal", "maxEdgesGlobal", "maxSessionsGlobal", "maxActiveSessions", "maxActiveRecorders", "maxClientEnrollments", "maxOperationJournalEntries", "maxVirtualBuses", "maxVirtualBusNameChars", "maxEntityIdBytes", "maxDisplayNameBytes", "maxPortNameBytes", "maxPortsPerNode", "maxChannelMatrixCoefficients", "maxControlValueDepth", "maxControlStringBytes", "maxControlValueCount", "maxMethodNameBytes", "maxRequestIdBytes", "maxRevisionCursorBytes"],
                    "additionalProperties": false
                },
                "events": {
                    "type": "object",
                    "properties": {
                        "stateCategories": { "type": "array", "maxItems": STATE_CATEGORIES.len(), "items": { "type": "string", "minLength": 1 } },
                        "meterReplay": { "const": false },
                        "retention": {
                            "type": "object",
                            "properties": {
                                "maxEvents": { "type": "integer", "minimum": 1 },
                                "maxAgeSeconds": { "type": "integer", "minimum": 1 }
                            },
                            "required": ["maxEvents", "maxAgeSeconds"],
                            "additionalProperties": false
                        }
                    },
                    "required": ["stateCategories", "meterReplay", "retention"],
                    "additionalProperties": false
                }
            },
            "required": ["protocolVersion", "schemaVersion", "build", "methods", "nodeTypes", "processors", "presets", "limits", "events"],
            "additionalProperties": false
        }),
        "system.handshake" => json!({
            "type": "object",
            "properties": {
                "compatible": { "const": true },
                "requested": {
                    "type": "object",
                    "properties": {
                        "major": { "type": "integer", "minimum": 0 },
                        "minor": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["major", "minor"],
                    "additionalProperties": false
                },
                "negotiated": {
                    "type": "object",
                    "properties": {
                        "major": { "const": 1 },
                        "minor": { "const": 0 }
                    },
                    "required": ["major", "minor"],
                    "additionalProperties": false
                },
                "schemaVersion": { "type": "integer", "minimum": 0 }
            },
            "required": ["compatible", "requested", "negotiated", "schemaVersion"],
            "additionalProperties": false
        }),
        "status.get" => status_output_schema(),
        "system.diagnostics" => diagnostics_output_schema(),
        "recovery.clearSafeMode" => json!({
            "type": "object",
            "properties": {
                "safeMode": { "const": false },
                "recentCrashes": { "type": "integer", "minimum": 0 },
                "persistence": { "enum": ["durable", "memory"] }
            },
            "required": ["safeMode", "recentCrashes", "persistence"],
            "additionalProperties": false
        }),
        "startup.get" => json!({
            "type": "object",
            "properties": {
                "enabled": { "const": false },
                "registration": { "const": "unavailable" },
                "reason": { "type": "string", "minLength": 1 }
            },
            "required": ["enabled", "registration", "reason"],
            "additionalProperties": false
        }),
        "startup.plan" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "enabled": { "type": "boolean" },
                "registration": { "const": "unavailable" },
                "reason": { "type": "string", "minLength": 1 },
                "requiredScopes": { "type": "array", "maxItems": MAX_PLAN_REQUIRED_SCOPES, "items": { "type": "string" } },
                "warnings": { "type": "array", "maxItems": MAX_PLAN_WARNINGS, "items": { "type": "string" } }
            },
            "required": ["planId", "enabled", "registration", "reason", "requiredScopes", "warnings"],
            "additionalProperties": false
        }),
        "startup.apply" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "const": "unavailable" },
                "registration": { "const": "unavailable" },
                "reason": { "type": "string", "minLength": 1 }
            },
            "required": ["planId", "state", "registration", "reason"],
            "additionalProperties": false
        }),
        "sessions.list" => {
            let item = session_item_schema();
            json!({
                "type": "object",
                "properties": {
                    "items": { "type": "array", "maxItems": MAX_SESSION_LIST_ITEMS, "items": item },
                    "nextCursor": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
                },
                "required": ["items", "nextCursor"],
                "additionalProperties": false
            })
        }
        "graph.history" => {
            let item = session_item_schema();
            json!({
                "type": "object",
                "properties": {
                    "items": { "type": "array", "maxItems": MAX_GRAPH_HISTORY_ITEMS, "items": item },
                    "nextCursor": { "type": ["string", "null"], "maxLength": MAX_REVISION_CURSOR_BYTES }
                },
                "required": ["items", "nextCursor"],
                "additionalProperties": false
            })
        }
        "sessions.get" | "sessions.export" => session_item_schema(),
        "sessions.importPlan" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "expiresInMs": { "type": "integer", "minimum": 1 },
                "session": session_item_schema()
            },
            "required": ["planId", "expiresInMs", "session"],
            "additionalProperties": false
        }),
        "sessions.importCommit" => json!({
            "type": "object",
            "properties": {
                "session": session_item_schema(),
                "state": { "const": "stopped" },
                "imported": { "const": true }
            },
            "required": ["session", "state", "imported"],
            "additionalProperties": false
        }),
        "sessions.create" | "sessions.duplicate" => json!({
            "type": "object",
            "properties": {
                "session": session_item_schema(),
                "state": { "const": "stopped" }
            },
            "required": ["session", "state"],
            "additionalProperties": false
        }),
        "sessions.delete" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "deleted": { "const": true }
            },
            "required": ["sessionId", "deleted"],
            "additionalProperties": false
        }),
        "session.start" | "sessions.start" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "const": "running" },
                "generation": { "type": "integer", "minimum": 1 },
                "runtime": { "const": "fake" }
            },
            "required": ["sessionId", "state", "generation", "runtime"],
            "additionalProperties": false
        }),
        "session.stop" | "sessions.stop" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "state": { "const": "stopped" },
                "runtime": { "const": "fake" },
                "recorders": {
                    "type": "array",
                    "maxItems": audiorouter_domain::MAX_ACTIVE_SESSIONS,
                    "items": {
                        "type": "object",
                        "properties": {
                            "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                            "state": { "const": "completed" },
                            "fileFinalized": { "const": true },
                            "recoverable": { "const": false }
                        },
                        "required": ["sessionId", "state", "fileFinalized", "recoverable"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["sessionId", "state", "runtime", "recorders"],
            "additionalProperties": false
        }),
        "graph.undoPlan" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "baseRevision": { "type": "integer", "minimum": 0 },
                "expiresInMs": { "type": "integer", "minimum": 1 }
            },
            "required": ["planId", "baseRevision", "expiresInMs"],
            "additionalProperties": false
        }),
        "safety.setPrivacyMute" => json!({
            "type": "object",
            "properties": {
                "muted": { "type": "boolean" },
                "persistence": { "enum": ["durable", "memory"] },
                "audioEffect": { "type": "string", "minLength": 1 }
            },
            "required": ["muted", "persistence", "audioEffect"],
            "additionalProperties": false
        }),
        "events.subscribe" => json!({
            "type": "object",
            "properties": {
                "backendEpoch": { "type": "integer", "minimum": 0 },
                "events": {
                    "type": "array",
                    "maxItems": MAX_EVENT_SUBSCRIPTION_ITEMS,
                    "items": {
                        "type": "object",
                        "properties": {
                            "sequence": { "type": "integer", "minimum": 1 },
                            "backendEpoch": { "type": "integer", "minimum": 0 },
                            "resourceRevision": { "type": "integer", "minimum": 0 },
                            "operationId": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_EVENT_OPERATION_ID_BYTES },
                            "category": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_EVENT_CATEGORY_BYTES },
                            "sessionId": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
                        },
                        "required": ["sequence", "backendEpoch", "resourceRevision", "operationId", "category", "sessionId"],
                        "additionalProperties": false
                    }
                },
                "nextSequence": { "type": "integer", "minimum": 0 },
                "resyncRequired": { "type": "boolean" },
                "reason": { "type": "string", "minLength": 1 },
                "snapshot": {
                    "type": "object",
                    "properties": {
                        "sessions": {
                            "type": "object",
                            "properties": {
                                "items": { "type": "array", "maxItems": MAX_EVENT_SUBSCRIPTION_ITEMS, "items": session_item_schema() },
                "nextCursor": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
                            },
                            "required": ["items", "nextCursor"],
                            "additionalProperties": false
                        }
                    },
                    "required": ["sessions"],
                    "additionalProperties": false
                }
            },
            "required": ["backendEpoch", "events", "nextSequence"],
            "additionalProperties": false
        }),
        "routes.inspect" => json!({
            "type": "object",
            "properties": {
                "destinationNode": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "reachable": { "type": "boolean" },
                "complete": { "type": "boolean" },
                "paths": {
                    "type": "array",
                    "maxItems": audiorouter_domain::MAX_ROUTE_PATHS,
                    "items": {
                        "type": "object",
                        "properties": {
                            "nodes": { "type": "array", "maxItems": audiorouter_domain::MAX_NODES_PER_SESSION, "items": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } },
                            "edges": { "type": "array", "maxItems": audiorouter_domain::MAX_EDGES_PER_SESSION, "items": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES } },
                            "channelMaps": { "type": "array", "maxItems": audiorouter_domain::MAX_EDGES_PER_SESSION, "items": { "type": "array", "maxItems": audiorouter_domain::MAX_CHANNEL_MATRIX_COEFFICIENTS, "items": { "type": "number", "minimum": -2, "maximum": 2 } } },
                            "latencySamples": { "type": "integer", "minimum": 0 }
                        },
                        "required": ["nodes", "edges", "channelMaps", "latencySamples"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["destinationNode", "reachable", "complete", "paths"],
            "additionalProperties": false
        }),
        "graph.plan" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "baseRevision": { "type": "integer", "minimum": 0 },
                "expiresInMs": { "type": "integer", "minimum": 1 },
                "diff": { "type": "array", "maxItems": MAX_GRAPH_DIFF_ITEMS },
                "affectedDestinations": {
                    "type": "array",
                    "maxItems": MAX_GRAPH_AFFECTED_DESTINATIONS,
                    "items": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": audiorouter_domain::MAX_DISPLAY_NAME_BYTES
                    }
                },
                "warnings": { "type": "array", "maxItems": MAX_PLAN_WARNINGS, "items": { "type": "string", "minLength": 1 } },
                "requiredScopes": { "type": "array", "maxItems": MAX_PLAN_REQUIRED_SCOPES, "items": { "type": "string", "minLength": 1 } }
            },
            "required": ["planId", "baseRevision", "expiresInMs", "diff", "affectedDestinations", "warnings", "requiredScopes"],
            "additionalProperties": false
        }),
        "graph.commit" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "revision": { "type": "integer", "minimum": 0 },
                "idempotentReplay": { "type": "boolean" },
                "activation": { "type": "object" }
            },
            "required": ["sessionId", "revision"],
            "additionalProperties": false
        }),
        "operations.get" => json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "operationId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES },
                        "operation": { "type": "string", "minLength": 1 },
                        "status": { "const": "completed" },
                        "durable": { "type": "boolean" },
                        "revision": { "type": "integer", "minimum": 0 },
                        "createdAt": { "type": ["integer", "null"] },
                        "result": { "type": "object" }
                    },
                    "required": ["operationId", "operation", "status", "durable", "revision", "createdAt", "result"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "operationId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES },
                        "status": { "const": "unknown" },
                        "durable": { "const": false }
                    },
                    "required": ["operationId", "status", "durable"],
                    "additionalProperties": false
                }
            ]
        }),
        "operations.cancel" => json!({
            "type": "object",
            "properties": {
                "operationId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES },
                "status": { "const": "completed" },
                "cancelled": { "const": false },
                "reason": { "const": "alreadyCompleted" }
            },
            "required": ["operationId", "status", "cancelled", "reason"],
            "additionalProperties": false
        }),
        "apps.list" | "applications.list" => json!({
            "type": "array",
            "maxItems": audiorouter_windows_audio::MAX_APPLICATIONS,
            "items": {
                "type": "object",
                "properties": {
                    "processId": { "type": "integer", "minimum": 1 },
                    "executable": { "type": "string", "maxLength": 260 },
                    "executablePath": { "type": ["string", "null"], "maxLength": 32768 },
                    "creationTime100ns": { "type": ["string", "null"] },
                    "audioActivity": { "enum": ["active", "inactive", "none"] },
                    "captureCapability": { "enum": ["observed", "notObserved"] },
                    "audioSessionCount": { "type": "integer", "minimum": 0 },
                    "activeAudioSessionCount": { "type": "integer", "minimum": 0 },
                    "captureSessionCount": { "type": "integer", "minimum": 0 },
                    "renderSessionCount": { "type": "integer", "minimum": 0 },
                    "audioDisplayNames": {
                        "type": "array",
                        "maxItems": audiorouter_windows_audio::MAX_APPLICATION_AUDIO_DISPLAY_NAMES,
                        "items": {
                            "type": "string",
                            "maxLength": audiorouter_windows_audio::MAX_APPLICATION_AUDIO_DISPLAY_NAME_BYTES
                        }
                    }
                },
                "required": ["processId", "executable", "executablePath", "creationTime100ns", "audioActivity", "captureCapability", "audioSessionCount", "activeAudioSessionCount", "captureSessionCount", "renderSessionCount", "audioDisplayNames"],
                "additionalProperties": false
            }
        }),
        "devices.list" => {
            let item = device_item_schema();
            json!({
                "oneOf": [
                    { "type": "array", "maxItems": MAX_DEVICE_LIST_ITEMS, "items": item.clone() },
                    {
                        "type": "object",
                        "properties": {
                            "items": { "type": "array", "maxItems": MAX_DEVICE_LIST_ITEMS, "items": item },
                                "nextCursor": { "type": ["string", "null"], "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES }
                        },
                        "required": ["items", "nextCursor"],
                        "additionalProperties": false
                    }
                ]
            })
        }
        "plugins.scan" | "plugins.list" | "plugins.retry" => json!({
            "type": "object",
            "properties": {
                "directory": { "type": "string", "minLength": 1 },
                "entries": {
                    "type": "array",
                    "maxItems": audiorouter_plugin_host::MAX_SCAN_CANDIDATES,
                    "items": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "minLength": 1 },
                            "identity": {
                                "type": ["object", "null"],
                                "properties": {
                                    "path": { "type": "string", "minLength": 1 },
                                    "binaryPath": { "type": "string", "minLength": 1 },
                                    "format": { "enum": ["vst3", "vst2", "unknown"] },
                                    "architecture": { "enum": ["x64", "x86", "arm64", "unknown"] },
                                    "fileBytes": { "type": "integer", "minimum": 1, "maximum": audiorouter_plugin_host::MAX_PLUGIN_BYTES },
                                    "sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
                                    "vendor": { "type": ["string", "null"], "maxLength": 128 },
                                    "version": { "type": ["string", "null"], "maxLength": 128 },
                                    "classIds": { "type": "array", "maxItems": 256, "items": { "type": "string", "maxLength": 32 } },
                                    "compatibility": { "enum": ["supportedVst3X64", "supportedVst2X64Gated", "unsupportedFormat"] }
                                },
                                "required": ["path", "binaryPath", "format", "architecture", "fileBytes", "sha256", "vendor", "version", "classIds", "compatibility"],
                                "additionalProperties": false
                            },
                            "error": { "type": ["string", "null"] },
                            "errorCode": { "enum": ["outsideConfiguredRoot", "unsupportedExtension", "missing", "tooLarge", "notPe", "unsupportedArchitecture", "cancelled", "deadlineExceeded", "io", null] }
                        },
                        "required": ["path", "identity", "error", "errorCode"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["directory", "entries"],
            "additionalProperties": false
        }),
        "plugins.inspect" => json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "minLength": 1 },
                "identity": {
                    "type": ["object", "null"],
                    "properties": {
                        "path": { "type": "string", "minLength": 1 },
                        "binaryPath": { "type": "string", "minLength": 1 },
                        "format": { "enum": ["vst3", "vst2", "unknown"] },
                        "architecture": { "enum": ["x64", "x86", "arm64", "unknown"] },
                        "fileBytes": { "type": "integer", "minimum": 1, "maximum": audiorouter_plugin_host::MAX_PLUGIN_BYTES },
                        "sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
                        "vendor": { "type": ["string", "null"], "maxLength": 128 },
                        "version": { "type": ["string", "null"], "maxLength": 128 },
                        "classIds": { "type": "array", "maxItems": 256, "items": { "type": "string", "maxLength": 32 } },
                        "compatibility": { "enum": ["supportedVst3X64", "supportedVst2X64Gated", "unsupportedFormat"] }
                    },
                    "required": ["path", "binaryPath", "format", "architecture", "fileBytes", "sha256", "vendor", "version", "classIds", "compatibility"],
                    "additionalProperties": false
                },
                "error": { "type": ["string", "null"] },
                "errorCode": { "enum": ["outsideConfiguredRoot", "unsupportedExtension", "missing", "tooLarge", "notPe", "unsupportedArchitecture", "cancelled", "deadlineExceeded", "io", null] }
            },
            "required": ["path", "identity", "error", "errorCode"],
            "additionalProperties": false
        }),
        "virtualDevices.list" => json!({
            "oneOf": [
                {
                    "type": "array",
                    "maxItems": audiorouter_domain::MAX_VIRTUAL_BUSES,
                    "items": virtual_device_item_schema()
                },
                {
                    "type": "object",
                    "properties": {
                        "items": { "type": "array", "maxItems": MAX_VIRTUAL_DEVICE_LIST_ITEMS, "items": virtual_device_item_schema() },
                        "nextCursor": { "type": ["string", "null"] }
                    },
                    "required": ["items", "nextCursor"],
                    "additionalProperties": false
                }
            ]
        }),
        "virtualDevices.plan" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1 },
                "expiresInMs": { "type": "integer", "minimum": 1 },
                "operation": virtual_device_operation_schema(),
                "availability": {
                    "type": "object",
                    "properties": {
                        "status": { "const": "unavailable" },
                        "reason": { "type": "string", "minLength": 1 }
                    },
                    "required": ["status", "reason"],
                    "additionalProperties": false
                },
                "requiredScopes": { "type": "array", "maxItems": MAX_PLAN_REQUIRED_SCOPES, "items": { "type": "string" } },
                "warnings": { "type": "array", "maxItems": MAX_PLAN_WARNINGS, "items": { "type": "string" } }
            },
            "required": ["planId", "expiresInMs", "operation", "availability", "requiredScopes", "warnings"],
            "additionalProperties": false
        }),
        "virtualDevices.apply" => json!({
            "type": "object",
            "properties": {
                "planId": { "type": "string", "minLength": 1 },
                "state": { "const": "applied" },
                "availability": { "type": "object" },
                "operation": virtual_device_operation_schema()
            },
            "required": ["planId", "state", "availability", "operation"],
            "additionalProperties": false
        }),
        "nodes.types" | "nodes.describe" => json!({
            "type": "array",
            "maxItems": audiorouter_domain::node_registry().len(),
            "items": node_type_item_schema()
        }),
        "presets.list" => json!({
            "type": "object",
            "properties": {
                "voiceChains": {
                    "type": "array",
                    "maxItems": audiorouter_dsp::VoiceChainPresetId::ALL.len(),
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "minLength": 1 },
                            "version": { "const": 1 },
                            "name": { "type": "string", "minLength": 1 },
                            "description": { "type": "string", "minLength": 1 }
                        },
                        "required": ["id", "version", "name", "description"],
                        "additionalProperties": false
                    }
                },
                "eq": {
                    "type": "array",
                    "maxItems": audiorouter_dsp::EqPresetId::ALL.len(),
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "minLength": 1 },
                            "version": { "const": 1 },
                            "name": { "type": "string", "minLength": 1 },
                            "description": { "type": "string", "minLength": 1 }
                        },
                        "required": ["id", "version", "name", "description"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["voiceChains", "eq"],
            "additionalProperties": false
        }),
        "processors.list" => json!({
            "type": "array",
            "maxItems": MAX_PROCESSOR_CATALOG_ITEMS,
            "items": processor_item_schema()
        }),
        "processors.response" => json!({
            "type": "object",
            "properties": {
                "frequenciesHz": { "type": "array", "minItems": 1, "maxItems": MAX_RESPONSE_FREQUENCIES, "items": { "type": "number" } },
                "magnitudeDb": { "type": "array", "minItems": 1, "maxItems": MAX_RESPONSE_FREQUENCIES, "items": { "type": "number" } }
            }, "required": ["frequenciesHz", "magnitudeDb"], "additionalProperties": false
        }),
        "clients.list" => json!({
            "type": "array",
            "maxItems": audiorouter_storage::MAX_CLIENT_ENROLLMENTS,
            "items": {
                "type": "object",
                "properties": {
                    "clientId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                    "role": { "enum": ["observer", "editor", "operator"] },
                    "revoked": { "type": "boolean" }
                },
                "required": ["clientId", "role", "revoked"],
                "additionalProperties": false
            }
        }),
        "clients.authorize" => json!({
            "type": "object",
            "properties": {
                "clientId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "role": { "enum": ["observer", "editor", "operator"] },
                "revoked": { "const": false }
            },
            "required": ["clientId", "role", "revoked"],
            "additionalProperties": false
        }),
        "clients.revoke" => json!({
            "type": "object",
            "properties": {
                "clientId": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                "revoked": { "const": true },
                "changed": { "type": "boolean" }
            },
            "required": ["clientId", "revoked", "changed"],
            "additionalProperties": false
        }),
        "recordings.list" => {
            let item = recording_item_schema();
            json!({
                "oneOf": [
                    { "type": "array", "maxItems": MAX_RECORDING_LIST_ITEMS, "items": item.clone() },
                    {
                        "type": "object",
                        "properties": {
                            "items": { "type": "array", "maxItems": MAX_RECORDING_LIST_ITEMS, "items": item },
                            "nextCursor": { "type": ["string", "null"] }
                        },
                        "required": ["items", "nextCursor"],
                        "additionalProperties": false
                    }
                ]
            })
        }
        "recordings.get" => recording_item_schema(),
        "recordings.recovery" => json!({
            "oneOf": [{
                "type": "object",
                "properties": {
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "status": { "enum": ["missing", "available"] },
                "checkpoint": recorder_checkpoint_schema()
                },
                "required": ["recordingId", "status"],
                "additionalProperties": false
            }, {
                "type": "object",
                "properties": {
                    "items": {
                        "type": "array",
                        "maxItems": MAX_RECORDING_LIST_ITEMS,
                        "items": {
                            "type": "object",
                            "properties": {
                                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                                "status": { "enum": ["missing", "available", "invalid"] },
                                "checkpoint": recorder_checkpoint_schema()
                            },
                            "required": ["recordingId", "status"],
                            "additionalProperties": false
                        }
                    },
                    "nextCursor": { "type": ["string", "null"], "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES }
                },
                "required": ["items", "nextCursor"],
                "additionalProperties": false
            }]
        }),
        "recordings.preview" => json!({
            "type": "object",
            "properties": {
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "preview": {
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "status": { "const": "present" },
                                "format": { "const": "wav" },
                                "channels": { "type": "integer", "minimum": 1, "maximum": 2 },
                                "sampleRate": { "type": "integer", "minimum": 1 },
                                "frames": { "type": "integer", "minimum": 0 },
                                "dataBytes": { "type": "integer", "minimum": 0 },
                                "fileBytes": { "type": "integer", "minimum": 0 }
                            },
                            "required": ["status", "format", "channels", "sampleRate", "frames", "dataBytes", "fileBytes"],
                            "additionalProperties": false
                        },
                        {
                            "type": "object",
                            "properties": {
                                "status": { "const": "present" },
                                "format": { "const": "flac" },
                                "channels": { "type": "integer", "minimum": 1, "maximum": 2 },
                                "sampleRate": { "type": "integer", "minimum": 1 },
                                "bitsPerSample": { "type": "integer", "minimum": 1 },
                                "frames": { "type": "integer", "minimum": 0 },
                                "fileBytes": { "type": "integer", "minimum": 0 }
                            },
                            "required": ["status", "format", "channels", "sampleRate", "bitsPerSample", "frames", "fileBytes"],
                            "additionalProperties": false
                        },
                        { "type": "object", "properties": { "status": { "enum": ["missing", "invalid"] } }, "required": ["status"], "additionalProperties": false }
                    ]
                }
            },
            "required": ["recordingId", "preview"],
            "additionalProperties": false
        }),
        "recordings.setMetadata" => json!({
            "type": "object",
            "properties": {
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "updated": { "const": true }
            },
            "required": ["recordingId", "updated"],
            "additionalProperties": false
        }),
        "recordings.rename" => json!({
            "type": "object",
            "properties": {
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "renamed": { "const": true },
                "path": { "type": "string", "minLength": 1 },
                "fileAction": { "const": "renamed" }
            },
            "required": ["recordingId", "renamed", "path", "fileAction"],
            "additionalProperties": false
        }),
        "recordings.removeEntry" => json!({
            "type": "object",
            "properties": {
                "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                "removed": { "const": true },
                "fileAction": { "const": "none" }
            },
            "required": ["recordingId", "removed", "fileAction"],
            "additionalProperties": false
        }),
        "recordings.reveal" => json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                        "path": { "type": "string", "minLength": 1 },
                        "revealed": { "const": true }
                    },
                    "required": ["recordingId", "path", "revealed"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                        "path": { "type": "string", "minLength": 1 },
                        "revealed": { "const": false },
                        "reason": { "const": "missing" }
                    },
                    "required": ["recordingId", "path", "revealed", "reason"],
                    "additionalProperties": false
                }
            ]
        }),
        "recordings.recycle" => json!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                        "path": { "type": "string", "minLength": 1 },
                        "fileAction": { "const": "none" },
                        "reason": { "const": "missing" }
                    },
                    "required": ["recordingId", "path", "fileAction", "reason"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                        "path": { "type": "string", "minLength": 1 },
                        "fileAction": { "const": "recycle" },
                        "preview": { "const": true }
                    },
                    "required": ["recordingId", "path", "fileAction", "preview"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                        "path": { "type": "string", "minLength": 1 },
                        "fileAction": { "const": "recycled" },
                        "missing": { "const": true }
                    },
                    "required": ["recordingId", "path", "fileAction", "missing"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "recordingId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
                        "path": { "type": "string", "minLength": 1 },
                        "fileAction": { "const": "none" },
                        "reason": { "const": "recycleUnavailable" }
                    },
                    "required": ["recordingId", "path", "fileAction", "reason"],
                    "additionalProperties": false
                }
            ]
        }),
        _ => json!({ "type": "object" }),
    }
}

fn status_output_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "build": { "type": "string" },
            "audio": { "const": "unavailable" },
            "deviceDiscovery": { "const": "available" },
            "reason": { "type": "string", "minLength": 1 },
            "storage": { "enum": ["memory", "sqlite"] },
            "sessionCount": { "type": "integer", "minimum": 0 },
            "activeSessionCount": { "type": "integer", "minimum": 0 },
            "activeSessionIds": {
                "type": "array",
                "maxItems": audiorouter_domain::MAX_ACTIVE_SESSIONS,
                "items": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES
                }
            },
            "privacyMute": {
                "type": "object",
                "properties": {
                    "muted": { "type": "boolean" },
                    "persistence": { "enum": ["durable", "memory"] },
                    "audioEffect": { "type": "string", "minLength": 1 }
                },
                "required": ["muted", "persistence", "audioEffect"],
                "additionalProperties": false
            },
            "recovery": {
                "type": "object",
                "properties": {
                    "safeMode": { "type": "boolean" },
                    "recentCrashes": { "type": "integer", "minimum": 0 },
                    "persistence": { "enum": ["durable", "memory"] }
                },
                "required": ["safeMode", "recentCrashes", "persistence"],
                "additionalProperties": false
            },
            "eventCursor": {
                "type": "object",
                "properties": {
                    "backendEpoch": { "type": "integer", "minimum": 0 },
                    "latestSequence": { "type": "integer", "minimum": 0 }
                },
                "required": ["backendEpoch", "latestSequence"],
                "additionalProperties": false
            }
        },
        "required": ["build", "audio", "deviceDiscovery", "reason", "storage", "sessionCount", "activeSessionCount", "activeSessionIds", "privacyMute", "recovery", "eventCursor"],
        "additionalProperties": false
    })
}

fn diagnostics_output_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "build": { "type": "string" },
            "backend": { "const": "control-plane" },
            "storage": { "enum": ["memory", "sqlite"] },
            "audio": {
                "type": "object",
                "properties": {
                    "state": { "const": "unavailable" },
                    "reason": { "type": "string", "minLength": 1 }
                },
                "required": ["state", "reason"],
                "additionalProperties": false
            },
            "nativeAdapter": { "const": "implemented-not-activated" },
            "privacyMute": {
                "type": "object",
                "properties": {
                    "muted": { "type": "boolean" },
                    "persistence": { "enum": ["durable", "memory"] }
                },
                "required": ["muted", "persistence"],
                "additionalProperties": false
            },
            "recovery": {
                "type": "object",
                "properties": {
                    "safeMode": { "type": "boolean" },
                    "recentCrashes": { "type": "integer", "minimum": 0 },
                    "persistence": { "enum": ["durable", "memory"] }
                },
                "required": ["safeMode", "recentCrashes", "persistence"],
                "additionalProperties": false
            },
            "eventLog": {
                "type": "object",
                "properties": {
                    "latestSequence": { "type": "integer", "minimum": 0 },
                    "retained": { "type": "integer", "minimum": 0 }
                },
                "required": ["latestSequence", "retained"],
                "additionalProperties": false
            },
            "redacted": { "const": true }
        },
        "required": ["build", "backend", "storage", "audio", "nativeAdapter", "privacyMute", "recovery", "eventLog", "redacted"],
        "additionalProperties": false
    })
}

fn recording_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
            "sessionId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
            "recorderId": { "type": "string", "minLength": 1, "maxLength": audiorouter_storage::MAX_RECORDING_ID_BYTES },
            "path": { "type": "string", "minLength": 1 },
            "format": { "enum": ["wav", "flac"] },
            "channels": { "enum": [1, 2] },
            "sampleRate": { "enum": [44100, 48000] },
            "frames": { "type": "integer", "minimum": 0 },
            "fileBytes": { "type": "integer", "minimum": 0 },
            "startTime": { "type": "string", "minLength": 1 },
            "state": { "enum": ["armed", "recording", "paused", "completed", "failed"] },
            "missing": { "type": "boolean" },
            "title": { "type": ["string", "null"], "maxLength": 256 },
            "artist": { "type": ["string", "null"], "maxLength": 256 },
            "comment": { "type": ["string", "null"], "maxLength": 256 }
        },
        "required": [
            "id", "sessionId", "recorderId", "path", "format", "channels",
            "sampleRate", "frames", "fileBytes", "startTime", "state",
            "missing", "title", "artist", "comment"
        ],
        "additionalProperties": false
    })
}

fn recorder_checkpoint_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "version": { "const": 1 },
            "state": { "enum": ["Idle", "Armed", "Recording", "Paused", "Stopping", "Completed", "Failed"] },
            "parts": {
                "type": "array",
                "maxItems": audiorouter_recording::MAX_CHECKPOINT_PARTS,
                "items": {
                    "type": "object",
                    "properties": {
                        "index": { "type": "integer", "minimum": 0 },
                        "start_frame": { "type": "integer", "minimum": 0 },
                        "end_frame": { "type": ["integer", "null"], "minimum": 0 }
                    },
                    "required": ["index", "start_frame", "end_frame"],
                    "additionalProperties": false
                }
            },
            "pauses": {
                "type": "array",
                "maxItems": audiorouter_recording::MAX_CHECKPOINT_PAUSES,
                "items": {
                    "type": "object",
                    "properties": {
                        "start_frame": { "type": "integer", "minimum": 0 },
                        "end_frame": { "type": "integer", "minimum": 0 }
                    },
                    "required": ["start_frame", "end_frame"],
                    "additionalProperties": false
                }
            },
            "pause_start": { "type": ["integer", "null"], "minimum": 0 },
            "last_frame": { "type": ["integer", "null"], "minimum": 0 },
            "stop_frame": { "type": ["integer", "null"], "minimum": 0 }
        },
        "required": ["version", "state", "parts", "pauses", "pause_start", "last_frame", "stop_frame"],
        "additionalProperties": false
    })
}

fn device_item_schema() -> Value {
    json!({
        "oneOf": [active_device_item_schema(), inactive_device_item_schema()]
    })
}

fn active_device_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "minLength": 1 },
            "name": { "type": "string", "minLength": 1, "maxLength": 512 },
            "direction": { "enum": ["capture", "render"] },
            "state": { "const": "active" },
            "defaultRoles": {
                "type": "array",
                "items": { "enum": ["console", "multimedia", "communications"] },
                "uniqueItems": true,
                "maxItems": 3
            },
            "format": {
                "type": "object",
                "properties": {
                    "sampleRateHz": { "type": "integer", "minimum": 1 },
                    "channels": { "type": "integer", "minimum": 1 },
                    "bitsPerSample": { "type": "integer", "minimum": 1 },
                    "formatTag": { "type": "integer", "minimum": 0 },
                    "bytesPerFrame": { "type": "integer", "minimum": 1 }
                },
                "required": ["sampleRateHz", "channels", "bitsPerSample", "formatTag", "bytesPerFrame"],
                "additionalProperties": false
            },
            "periods": {
                "type": "object",
                "properties": {
                    "default100ns": { "type": "integer", "minimum": 0 },
                    "minimum100ns": { "type": "integer", "minimum": 0 }
                },
                "required": ["default100ns", "minimum100ns"],
                "additionalProperties": false
            }
        },
        "required": ["id", "name", "direction", "state", "defaultRoles", "format", "periods"],
        "additionalProperties": false
    })
}

fn inactive_device_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "minLength": 1 },
            "name": { "type": "string", "minLength": 1, "maxLength": 512 },
            "direction": { "enum": ["capture", "render"] },
            "state": { "enum": ["disabled", "unplugged", "notPresent", "unknown"] },
            "defaultRoles": {
                "type": "array",
                "items": { "enum": ["console", "multimedia", "communications"] },
                "uniqueItems": true,
                "maxItems": 3
            }
        },
        "required": ["id", "name", "direction", "state", "defaultRoles"],
        "additionalProperties": false
    })
}

fn virtual_device_operation_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": { "enum": ["create", "rename", "setEnabled", "delete"] },
            "id": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
            "name": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_VIRTUAL_BUS_NAME_CHARS },
            "enabled": { "type": "boolean" }
        },
        "required": ["action", "id"],
        "additionalProperties": false
    })
}

fn virtual_device_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
            "name": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_VIRTUAL_BUS_NAME_CHARS },
            "direction": { "const": "bidirectional" },
            "channels": { "const": 2 },
            "enabled": { "type": "boolean" },
            "availability": {
                "type": "object",
                "properties": {
                    "status": { "const": "unavailable" },
                    "reason": { "type": "string", "minLength": 1 }
                },
                "required": ["status", "reason"],
                "additionalProperties": false
            },
            "endpointIds": {
                "type": "object",
                "properties": {
                    "render": { "type": ["string", "null"] },
                    "capture": { "type": ["string", "null"] }
                },
                "required": ["render", "capture"],
                "additionalProperties": false
            },
            "capabilities": {
                "type": "object",
                "properties": {
                    "render": { "const": false },
                    "capture": { "const": false },
                    "channels": { "const": 2 }
                },
                "required": ["render", "capture", "channels"],
                "additionalProperties": false
            },
            "privilege": { "const": "deviceAdministration" },
            "restartRequired": { "const": false },
            "clientImpacts": { "type": "array", "items": { "type": "string" } },
            "leaseOwner": { "type": ["string", "null"] }
        },
        "required": ["id", "name", "direction", "channels", "enabled", "availability", "endpointIds", "capabilities", "privilege", "restartRequired", "clientImpacts", "leaseOwner"],
        "additionalProperties": false
    })
}

fn node_type_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "type": { "type": "string", "pattern": "^[a-z0-9-]+@[0-9]+$" },
            "availability": {
                "type": "object",
                "properties": {
                    "status": { "enum": ["available", "unavailable"] },
                    "reason": { "type": "string", "minLength": 1 }
                },
                "required": ["status"],
                "additionalProperties": false
            },
            "realtimeCostClass": { "type": "string", "minLength": 1 },
            "latencySamples": { "type": "integer", "minimum": 0 },
            "parameters": {
                "type": "array",
                "maxItems": audiorouter_domain::MAX_PARAMETERS_PER_NODE,
                "items": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "type": { "enum": ["boolean", "number"] },
                        "unit": { "type": "string", "minLength": 1 },
                        "minimum": { "type": "number" },
                        "maximum": { "type": "number" },
                        "default": {}
                    },
                    "required": ["name", "type", "default"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["type", "availability", "realtimeCostClass", "latencySamples", "parameters"],
        "additionalProperties": false
    })
}

fn processor_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "minLength": 1 },
            "version": { "type": "integer", "minimum": 1 },
            "category": { "type": "string", "minLength": 1 },
            "availability": {
                "type": "object",
                "properties": {
                    "status": { "enum": ["available", "unavailable"] },
                    "reason": { "type": "string", "minLength": 1 }
                },
                "required": ["status"],
                "additionalProperties": false
            },
            "latencySamples": { "type": "integer", "minimum": 0 },
            "parameters": {
                "type": "array",
                "maxItems": audiorouter_domain::MAX_PARAMETERS_PER_NODE,
                "items": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "minLength": 1 },
                        "type": { "enum": ["boolean", "number", "string"] },
                        "unit": { "type": "string", "minLength": 1 },
                        "minimum": { "type": "number" },
                        "maximum": { "type": "number" },
                        "default": {}
                    },
                    "required": ["name", "type"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["id", "version", "category", "availability", "latencySamples", "parameters"],
        "additionalProperties": false
    })
}

fn session_item_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES, "description": "UTF-8 byte limit is advertised in limits.maxEntityIdBytes when applicable." },
            "name": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_DISPLAY_NAME_BYTES, "description": "Maximum 256 UTF-8 bytes." },
            "schemaVersion": { "const": 1 },
            "revision": { "type": "integer", "minimum": 0 },
            "nodes": {
                "type": "array",
                "maxItems": audiorouter_domain::MAX_NODES_PER_SESSION,
                "items": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                        "kind": { "type": "string", "minLength": 1 },
                        "typeVersion": { "const": 1 },
                        "name": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_DISPLAY_NAME_BYTES, "description": "Maximum 256 UTF-8 bytes." },
                        "enabled": { "type": "boolean" },
                        "bypass": { "type": "boolean" },
                        "parameters": {
                            "type": "object",
                            "maxProperties": audiorouter_domain::MAX_PARAMETERS_PER_NODE,
                            "propertyNames": {
                                "maxLength": audiorouter_domain::MAX_PARAMETER_NAME_BYTES,
                                "description": "Maximum 128 UTF-8 bytes per parameter name."
                            }
                        },
                        "ports": {
                            "type": "array",
                                "maxItems": audiorouter_domain::MAX_PORTS_PER_NODE,
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_PORT_NAME_BYTES, "description": "Maximum 128 UTF-8 bytes." },
                                    "direction": { "enum": ["input", "output"] },
                                    "channels": { "type": "integer", "minimum": 1, "maximum": 2 }
                                },
                                "required": ["name", "direction", "channels"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["id", "kind", "typeVersion", "name", "enabled", "bypass", "parameters", "ports"],
                    "additionalProperties": false
                }
            },
            "edges": {
                "type": "array",
                "maxItems": audiorouter_domain::MAX_EDGES_PER_SESSION,
                "items": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                        "sourceNode": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                        "sourcePort": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_PORT_NAME_BYTES },
                        "destinationNode": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_ENTITY_ID_BYTES },
                        "destinationPort": { "type": "string", "minLength": 1, "maxLength": audiorouter_domain::MAX_PORT_NAME_BYTES },
                        "matrix": {
                            "type": "array",
                            "maxItems": audiorouter_domain::MAX_CHANNEL_MATRIX_COEFFICIENTS,
                            "items": { "type": "number", "minimum": -2.0, "maximum": 2.0 }
                        },
                        "enabled": { "type": "boolean" }
                    },
                    "required": ["id", "sourceNode", "sourcePort", "destinationNode", "destinationPort", "matrix", "enabled"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["id", "name", "schemaVersion", "revision", "nodes", "edges"],
        "additionalProperties": false
    })
}

#[derive(Debug, Eq, PartialEq)]
pub enum ControlError {
    InvalidRequest(String),
    Audio {
        code: &'static str,
        hresult: u32,
        retryable: bool,
        remediation: &'static str,
        message: String,
    },
    PluginScan(audiorouter_plugin_host::ScanError),
    IdempotencyConflict,
    Store(audiorouter_domain::StoreError),
    Json(String),
    Storage(String),
    CorruptDatabase(String),
}

fn audio_control_error(error: audiorouter_windows_audio::AudioError) -> ControlError {
    ControlError::Audio {
        code: error.kind().code(),
        hresult: error.hresult(),
        retryable: error.is_retryable(),
        remediation: error.remediation(),
        message: error.to_string(),
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ClientGrant {
    scopes: std::collections::HashSet<PermissionScope>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientRole {
    Observer,
    Editor,
    Operator,
}

impl ClientGrant {
    pub fn read_only() -> Self {
        Self::with_scopes([PermissionScope::Read])
    }

    pub fn with_scopes(scopes: impl IntoIterator<Item = PermissionScope>) -> Self {
        Self {
            scopes: scopes.into_iter().collect(),
        }
    }

    /// Map an enrolled role to the narrowest built-in grant for that role.
    /// Capture, recording, and device administration are never implied by these
    /// convenience roles and require a separately constructed explicit grant.
    pub fn for_role(role: ClientRole) -> Self {
        match role {
            ClientRole::Observer => Self::read_only(),
            ClientRole::Editor => {
                Self::with_scopes([PermissionScope::Read, PermissionScope::GraphWrite])
            }
            ClientRole::Operator => Self::with_scopes([
                PermissionScope::Read,
                PermissionScope::GraphWrite,
                PermissionScope::SessionControl,
            ]),
        }
    }

    fn allows(&self, scope: PermissionScope) -> bool {
        self.scopes.contains(&scope)
    }
}

impl From<audiorouter_domain::StoreError> for ControlError {
    fn from(error: audiorouter_domain::StoreError) -> Self {
        Self::Store(error)
    }
}

pub struct ControlPlane {
    store: GraphStore,
    build: String,
    runtimes: HashMap<EntityId, FakeRuntime>,
    recorders: HashMap<EntityId, RecorderController>,
    recorder_workers: HashMap<EntityId, Box<dyn RecorderWorker>>,
    recorder_node_workers: HashMap<EntityId, Box<dyn RecorderWorker>>,
    recorder_node_states: HashMap<EntityId, RecorderController>,
    recorder_node_sessions: HashMap<EntityId, EntityId>,
    recording_policy: Option<RecordingPathPolicy>,
    storage: Option<Storage>,
    enrollments: HashMap<String, (ClientRole, bool)>,
    events: EventLog,
    mutation_limiter: MutationRateLimiter,
    operation_outcomes: HashMap<String, Value>,
    operation_names: HashMap<String, String>,
    operation_order: VecDeque<String>,
    idempotency_hashes: HashMap<String, String>,
    application_snapshot: Option<(Instant, Value)>,
    plugin_inventories: HashMap<String, Value>,
    plugin_inventory_order: VecDeque<String>,
    privacy_muted: bool,
    recovery_tracker: CrashRecoveryTracker,
    virtual_buses: VirtualBusRegistry,
    virtual_bridges: VirtualBusBridgeSet,
    virtual_bus_plans: HashMap<EntityId, VirtualBusPlan>,
    next_virtual_bus_plan: u64,
    startup_plans: HashMap<EntityId, (bool, Instant)>,
    next_startup_plan: u64,
    session_import_plans: HashMap<EntityId, (Session, Instant)>,
    next_session_import_plan: u64,
    active_idempotency_scope: Option<String>,
    endpoint_monitor: Option<audiorouter_windows_audio::EndpointMonitor>,
}

impl Default for ControlPlane {
    fn default() -> Self {
        Self::new("dev")
    }
}

impl ControlPlane {
    pub fn new(build: impl Into<String>) -> Self {
        Self {
            store: GraphStore::default(),
            build: build.into(),
            runtimes: HashMap::new(),
            recorders: HashMap::new(),
            recorder_workers: HashMap::new(),
            recorder_node_workers: HashMap::new(),
            recorder_node_states: HashMap::new(),
            recorder_node_sessions: HashMap::new(),
            recording_policy: None,
            storage: None,
            enrollments: HashMap::new(),
            events: EventLog::new(1),
            mutation_limiter: MutationRateLimiter::default(),
            operation_outcomes: HashMap::new(),
            operation_names: HashMap::new(),
            operation_order: VecDeque::new(),
            idempotency_hashes: HashMap::new(),
            application_snapshot: None,
            plugin_inventories: HashMap::new(),
            plugin_inventory_order: VecDeque::new(),
            privacy_muted: false,
            recovery_tracker: CrashRecoveryTracker::default(),
            virtual_buses: VirtualBusRegistry::default(),
            virtual_bridges: VirtualBusBridgeSet::new(
                8,
                2,
                audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
            )
            .expect("valid default virtual bridge collection"),
            virtual_bus_plans: HashMap::new(),
            next_virtual_bus_plan: 1,
            startup_plans: HashMap::new(),
            next_startup_plan: 1,
            session_import_plans: HashMap::new(),
            next_session_import_plan: 1,
            active_idempotency_scope: None,
            endpoint_monitor: None,
        }
    }

    pub fn try_with_storage(
        build: impl Into<String>,
        storage: Storage,
    ) -> Result<Self, audiorouter_storage::StorageError> {
        // Fail closed if the durable latch cannot be read: a persistence
        // failure must never silently unmute a capture path.
        let privacy_muted = storage.load_privacy_mute()?;
        let recording_policy = storage
            .load_recording_root()?
            .map(RecordingPathPolicy::new)
            .transpose()
            .map_err(|error| {
                audiorouter_storage::StorageError::InvalidRecording(format!(
                    "invalid recording root: {error:?}"
                ))
            })?;
        let virtual_buses = storage.load_virtual_buses()?;
        let mut persisted_sessions = Vec::new();
        let mut session_cursor = None;
        loop {
            let page = storage.list_sessions_after(
                session_cursor.as_deref(),
                audiorouter_storage::MAX_SESSION_LIST_ITEMS,
            )?;
            if page.is_empty() {
                break;
            }
            let page_len = page.len();
            session_cursor = page.last().map(|session| session.id.as_str().to_owned());
            persisted_sessions.extend(page);
            if page_len < audiorouter_storage::MAX_SESSION_LIST_ITEMS {
                break;
            }
        }
        let mut store = GraphStore::default();
        for id in storage.list_graph_plan_ids()? {
            if let Some(counter) = id
                .strip_prefix("plan-")
                .and_then(|suffix| suffix.parse::<u64>().ok())
            {
                store.advance_plan_counter(counter);
            }
        }
        for session in persisted_sessions {
            let history = storage.load_history(&session.id, 100)?;
            if history.is_empty() {
                store.insert_session(session).map_err(|error| {
                    audiorouter_storage::StorageError::InvalidSession(format!("{error:?}"))
                })?;
            } else {
                store.restore_history(history).map_err(|error| {
                    audiorouter_storage::StorageError::InvalidSession(format!("{error:?}"))
                })?;
            }
        }
        let now = unix_epoch_seconds();
        let mut virtual_bus_plans = HashMap::new();
        for (id, operation, expires_at) in storage.load_virtual_device_plans()? {
            let Some(remaining) =
                remaining_persisted_plan_duration(expires_at, now, VIRTUAL_DEVICE_PLAN_TTL)
            else {
                continue;
            };
            let operation = virtual_bus_operation_from_value(&operation).map_err(|_| {
                audiorouter_storage::StorageError::InvalidPlan(
                    "invalid persisted virtual-device plan".into(),
                )
            })?;
            virtual_bus_plans.insert(
                id,
                VirtualBusPlan {
                    operation,
                    expires_at: Instant::now() + remaining,
                },
            );
        }
        let mut startup_plans = HashMap::new();
        for (id, enabled, expires_at) in storage.load_startup_plans()? {
            let Some(remaining) =
                remaining_persisted_plan_duration(expires_at, now, VIRTUAL_DEVICE_PLAN_TTL)
            else {
                continue;
            };
            startup_plans.insert(id, (enabled, Instant::now() + remaining));
        }
        // Claim a new epoch only after every persisted state surface has been
        // read and validated successfully; failed startup must not mutate the
        // durable database while reporting an initialization error.
        let backend_epoch = storage.claim_backend_epoch()?;
        Ok(Self {
            store,
            build: build.into(),
            runtimes: HashMap::new(),
            recorders: HashMap::new(),
            recorder_workers: HashMap::new(),
            recorder_node_workers: HashMap::new(),
            recorder_node_states: HashMap::new(),
            recorder_node_sessions: HashMap::new(),
            recording_policy,
            storage: Some(storage),
            enrollments: HashMap::new(),
            events: EventLog::new(backend_epoch),
            mutation_limiter: MutationRateLimiter::default(),
            operation_outcomes: HashMap::new(),
            operation_names: HashMap::new(),
            operation_order: VecDeque::new(),
            idempotency_hashes: HashMap::new(),
            application_snapshot: None,
            plugin_inventories: HashMap::new(),
            plugin_inventory_order: VecDeque::new(),
            privacy_muted,
            recovery_tracker: CrashRecoveryTracker::default(),
            virtual_buses,
            virtual_bridges: VirtualBusBridgeSet::new(
                8,
                2,
                audiorouter_engine::PROCESSING_QUANTUM_FRAMES,
            )
            .expect("valid default virtual bridge collection"),
            virtual_bus_plans,
            next_virtual_bus_plan: 1,
            startup_plans,
            next_startup_plan: 1,
            session_import_plans: HashMap::new(),
            next_session_import_plan: 1,
            active_idempotency_scope: None,
            endpoint_monitor: None,
        })
    }

    pub fn with_storage(build: impl Into<String>, storage: Storage) -> Self {
        Self::try_with_storage(build, storage).unwrap_or_else(|error| {
            panic!("AudioRouter storage initialization failed closed: {error:?}")
        })
    }

    /// Configure the backend-owned recording root. The policy is validated
    /// before replacing the current in-memory policy and is persisted before
    /// the new policy becomes active.
    pub fn configure_recording_root(
        &mut self,
        root: impl AsRef<std::path::Path>,
    ) -> Result<(), ControlError> {
        let policy = RecordingPathPolicy::new(root.as_ref()).map_err(|error| {
            ControlError::InvalidRequest(format!("invalid recording root: {error:?}"))
        })?;
        if let Some(storage) = &self.storage {
            storage
                .save_recording_root(root.as_ref())
                .map_err(storage_error)?;
        }
        self.recording_policy = Some(policy);
        Ok(())
    }

    fn scoped_idempotency_key(&self, method: &str, key: &str) -> String {
        self.active_idempotency_scope
            .as_ref()
            .map(|client| format!("{client}\0{method}\0{key}"))
            .unwrap_or_else(|| key.to_owned())
    }

    fn remember_plugin_inventory(&mut self, directory: String, result: Value) {
        if !self.plugin_inventories.contains_key(&directory) {
            self.plugin_inventory_order.push_back(directory.clone());
        }
        self.plugin_inventories.insert(directory, result);
        while self.plugin_inventory_order.len() > MAX_PLUGIN_INVENTORY_ROOTS {
            if let Some(oldest) = self.plugin_inventory_order.pop_front() {
                self.plugin_inventories.remove(&oldest);
            }
        }
    }

    fn operation_lookup_keys(&self, operation_id: &str) -> Vec<String> {
        self.active_idempotency_scope
            .as_ref()
            .map(|client| {
                vec![
                    format!("{client}\0graph.commit\0{operation_id}"),
                    format!("{client}\0virtualDevices.apply\0{operation_id}"),
                    format!("{client}\0recordings.setMetadata\0{operation_id}"),
                    format!("{client}\0recordings.rename\0{operation_id}"),
                    format!("{client}\0recordings.removeEntry\0{operation_id}"),
                    format!("{client}\0recordings.recycle\0{operation_id}"),
                    format!("{client}\0sessions.delete\0{operation_id}"),
                    format!("{client}\0sessions.create\0{operation_id}"),
                    format!("{client}\0sessions.duplicate\0{operation_id}"),
                    format!("{client}\0safety.setPrivacyMute\0{operation_id}"),
                    format!("{client}\0recovery.clearSafeMode\0{operation_id}"),
                    format!("{client}\0clients.authorize\0{operation_id}"),
                    format!("{client}\0clients.revoke\0{operation_id}"),
                    format!("{client}\0operations.cancel\0{operation_id}"),
                ]
            })
            .unwrap_or_else(|| vec![operation_id.to_owned()])
    }

    fn remember_operation_outcome(
        &mut self,
        idempotency_key: &str,
        result: Value,
        operation: &str,
        request_hash: Option<&str>,
    ) {
        if !self.operation_outcomes.contains_key(idempotency_key) {
            while self.operation_outcomes.len() >= MAX_MEMORY_OPERATION_OUTCOMES {
                let Some(oldest) = self.operation_order.pop_front() else {
                    break;
                };
                if self.operation_outcomes.remove(&oldest).is_some() {
                    self.operation_names.remove(&oldest);
                    self.idempotency_hashes.remove(&oldest);
                    break;
                }
            }
            self.operation_order.push_back(idempotency_key.to_owned());
        }
        self.operation_outcomes
            .insert(idempotency_key.to_owned(), result);
        self.operation_names
            .insert(idempotency_key.to_owned(), operation.to_owned());
        if let Some(hash) = request_hash {
            self.idempotency_hashes
                .insert(idempotency_key.to_owned(), hash.to_owned());
        } else {
            self.idempotency_hashes.remove(idempotency_key);
        }
    }

    fn request_hash(value: &Value) -> String {
        let mut digest = Sha256::new();
        digest.update(serde_json::to_vec(value).unwrap_or_default());
        format!("{:x}", digest.finalize())
    }

    fn lookup_idempotent_result(
        &self,
        key: &str,
        request_hash: &str,
    ) -> Result<Option<Value>, ControlError> {
        if let Some(result) = self.operation_outcomes.get(key) {
            if self.idempotency_hashes.get(key).map(String::as_str) != Some(request_hash) {
                return Err(ControlError::InvalidRequest(
                    "idempotency key is already used for a different request".into(),
                ));
            }
            return Ok(Some(result.clone()));
        }
        let Some(storage) = &self.storage else {
            return Ok(None);
        };
        storage
            .journal_result_checked(key, request_hash)
            .map_err(storage_error)?
            .map(|result| {
                serde_json::from_str(&result).map_err(|error| ControlError::Json(error.to_string()))
            })
            .transpose()
    }

    fn journal_idempotent_result(
        &mut self,
        key: &str,
        operation: &str,
        request_hash: &str,
        result: &Value,
    ) -> Result<(), ControlError> {
        if let Some(storage) = &self.storage {
            let encoded = serde_json::to_string(result)
                .map_err(|error| ControlError::Json(error.to_string()))?;
            storage
                .journal_commit_with_hash(key, operation, &encoded, 0, request_hash)
                .map_err(storage_error)?;
        }
        self.remember_operation_outcome(key, result.clone(), operation, Some(request_hash));
        Ok(())
    }

    pub fn create_virtual_bus(
        &mut self,
        id: EntityId,
        name: impl Into<String>,
    ) -> Result<(), ControlError> {
        let bridge_id = id.clone();
        let checkpoint = self.virtual_buses.clone();
        self.virtual_buses
            .create(id, name)
            .map_err(virtual_bus_control_error)?;
        if let Err(error) = self.virtual_bridges.ensure(bridge_id.clone()) {
            self.virtual_buses = checkpoint;
            return Err(virtual_bridge_control_error(error));
        }
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_buses(&self.virtual_buses) {
                self.virtual_buses = checkpoint;
                let _ = self.virtual_bridges.remove(&bridge_id);
                return Err(storage_error(error));
            }
        }
        Ok(())
    }

    pub fn rename_virtual_bus(
        &mut self,
        id: &EntityId,
        name: impl Into<String>,
    ) -> Result<(), ControlError> {
        let checkpoint = self.virtual_buses.clone();
        self.virtual_buses
            .rename(id, name)
            .map_err(virtual_bus_control_error)?;
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_buses(&self.virtual_buses) {
                self.virtual_buses = checkpoint;
                return Err(storage_error(error));
            }
        }
        Ok(())
    }

    pub fn set_virtual_bus_enabled(
        &mut self,
        id: &EntityId,
        enabled: bool,
    ) -> Result<(), ControlError> {
        let checkpoint = self.virtual_buses.clone();
        self.virtual_buses
            .set_enabled(id, enabled)
            .map_err(virtual_bus_control_error)?;
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_buses(&self.virtual_buses) {
                self.virtual_buses = checkpoint;
                return Err(storage_error(error));
            }
        }
        if !enabled {
            if let Some(bridge) = self.virtual_bridges.get(id) {
                bridge.deactivate();
            }
        }
        Ok(())
    }

    pub fn delete_virtual_bus(&mut self, id: &EntityId) -> Result<(), ControlError> {
        let checkpoint = self.virtual_buses.clone();
        self.virtual_buses
            .delete(id)
            .map_err(virtual_bus_control_error)?;
        if self.virtual_bridges.get(id).is_some() {
            self.virtual_bridges
                .remove(id)
                .map_err(virtual_bridge_control_error)?;
        }
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_buses(&self.virtual_buses) {
                self.virtual_buses = checkpoint;
                let _ = self.virtual_bridges.ensure(id.clone());
                return Err(storage_error(error));
            }
        }
        Ok(())
    }

    fn sync_virtual_bridge_operation(
        &mut self,
        operation: &VirtualBusOperation,
    ) -> Result<(), ControlError> {
        match operation {
            VirtualBusOperation::Create { id, .. } => self
                .virtual_bridges
                .ensure(id.clone())
                .map(|_| ())
                .map_err(virtual_bridge_control_error),
            VirtualBusOperation::SetEnabled { .. } => Ok(()),
            VirtualBusOperation::Delete { id } => {
                if self.virtual_bridges.get(id).is_some() {
                    self.virtual_bridges
                        .remove(id)
                        .map_err(virtual_bridge_control_error)?;
                }
                Ok(())
            }
            VirtualBusOperation::Rename { .. } => Ok(()),
        }
    }

    fn rollback_virtual_bridge_operation(
        &mut self,
        operation: &VirtualBusOperation,
    ) -> Result<(), ControlError> {
        match operation {
            VirtualBusOperation::Create { id, .. } => {
                if self.virtual_bridges.get(id).is_some() {
                    self.virtual_bridges
                        .remove(id)
                        .map_err(virtual_bridge_control_error)?;
                }
            }
            VirtualBusOperation::Delete { id } => {
                self.virtual_bridges
                    .ensure(id.clone())
                    .map(|_| ())
                    .map_err(virtual_bridge_control_error)?;
            }
            VirtualBusOperation::Rename { .. } | VirtualBusOperation::SetEnabled { .. } => {}
        }
        Ok(())
    }

    pub fn enroll_client(
        &mut self,
        client_id: impl Into<String>,
        role: ClientRole,
    ) -> Result<(), ControlError> {
        let client_id = client_id.into();
        if client_id.is_empty() {
            return Err(ControlError::InvalidRequest("client_id is required".into()));
        }
        if client_id.len() > audiorouter_domain::MAX_ENTITY_ID_BYTES {
            return Err(ControlError::InvalidRequest("client_id is too long".into()));
        }
        if self.storage.is_none()
            && !self.enrollments.contains_key(&client_id)
            && self.enrollments.len() >= audiorouter_storage::MAX_CLIENT_ENROLLMENTS
        {
            return Err(ControlError::InvalidRequest(
                "client enrollment limit reached".into(),
            ));
        }
        if let Some(storage) = &self.storage {
            storage
                .save_client_enrollment(&client_id, role_name(role))
                .map_err(storage_error)?;
        }
        self.enrollments.insert(client_id, (role, false));
        Ok(())
    }

    pub fn revoke_client(&mut self, client_id: &str) -> Result<bool, ControlError> {
        if client_id.len() > audiorouter_domain::MAX_ENTITY_ID_BYTES {
            return Err(ControlError::InvalidRequest("client_id is too long".into()));
        }
        let changed = if let Some(storage) = &self.storage {
            storage
                .revoke_client_enrollment(client_id)
                .map_err(storage_error)?
        } else {
            self.enrollments
                .get_mut(client_id)
                .map(|entry| {
                    let changed = !entry.1;
                    entry.1 = true;
                    changed
                })
                .unwrap_or(false)
        };
        if let Some(entry) = self.enrollments.get_mut(client_id) {
            entry.1 = true;
        }
        Ok(changed)
    }

    pub fn grant_for_client(&self, client_id: &str) -> Result<Option<ClientGrant>, ControlError> {
        let enrollment = self.enrollments.get(client_id).copied();
        let enrollment = match (enrollment, &self.storage) {
            (Some(value), _) => Some(value),
            (None, Some(storage)) => storage
                .load_client_enrollment(client_id)
                .map_err(storage_error)?
                .and_then(|(role, revoked)| role_from_name(&role).map(|role| (role, revoked))),
            (None, None) => None,
        };
        Ok(enrollment
            .filter(|(_, revoked)| !revoked)
            .map(|(role, _)| ClientGrant::for_role(role)))
    }

    fn client_records(&self) -> Result<Vec<Value>, ControlError> {
        let records = if let Some(storage) = &self.storage {
            storage.list_client_enrollments().map_err(storage_error)?
        } else {
            let mut records = self
                .enrollments
                .iter()
                .map(|(client_id, (role, revoked))| {
                    (client_id.clone(), role_name(*role).to_owned(), *revoked)
                })
                .collect::<Vec<_>>();
            records.sort_by(|left, right| left.0.cmp(&right.0));
            records
        };
        Ok(records
            .into_iter()
            .map(|(client_id, role, revoked)| {
                json!({
                    "clientId": client_id,
                    "role": role,
                    "revoked": revoked
                })
            })
            .collect())
    }

    fn dispatch_clients_list(&self) -> Result<Value, ControlError> {
        Ok(Value::Array(self.client_records()?))
    }

    fn dispatch_client_authorize(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params
            .ok_or_else(|| ControlError::InvalidRequest("clientId and role are required".into()))?;
        let client_id = params
            .get("clientId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("clientId is required".into()))?;
        let role_name_value = params
            .get("role")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("role is required".into()))?;
        let role = role_from_name(role_name_value)
            .ok_or_else(|| ControlError::InvalidRequest("unknown client role".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = (
            self.scoped_idempotency_key("clients.authorize", idempotency_key),
            Self::request_hash(&json!({ "clientId": client_id, "role": role_name_value })),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        self.enroll_client(client_id, role)?;
        let result = json!({ "clientId": client_id, "role": role_name_value, "revoked": false });
        self.journal_idempotent_result(&operation.0, "clients.authorize", &operation.1, &result)?;
        Ok(result)
    }

    fn dispatch_client_revoke(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("clientId is required".into()))?;
        let client_id = params
            .get("clientId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("clientId is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = (
            self.scoped_idempotency_key("clients.revoke", idempotency_key),
            Self::request_hash(&json!({ "clientId": client_id })),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        let changed = self.revoke_client(client_id)?;
        let result = json!({ "clientId": client_id, "revoked": true, "changed": changed });
        self.journal_idempotent_result(&operation.0, "clients.revoke", &operation.1, &result)?;
        Ok(result)
    }

    fn dispatch_operation_get(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("operationId is required".into()))?;
        let operation_id = params
            .get("operationId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("operationId is required".into()))?;
        let lookup_keys = self.operation_lookup_keys(operation_id);
        if let Some(storage) = &self.storage {
            for lookup_key in &lookup_keys {
                if let Some((operation, result, revision, created_at)) = storage
                    .operation_status(lookup_key)
                    .map_err(storage_error)?
                {
                    let result: Value = serde_json::from_str(&result)
                        .map_err(|error| ControlError::Json(error.to_string()))?;
                    return Ok(json!({
                        "operationId": operation_id,
                        "operation": operation,
                        "status": "completed",
                        "durable": true,
                        "revision": revision,
                        "createdAt": created_at,
                        "result": result
                    }));
                }
            }
        }
        for lookup_key in &lookup_keys {
            if let Some(result) = self.operation_outcomes.get(lookup_key) {
                let operation = self
                    .operation_names
                    .get(lookup_key)
                    .map(String::as_str)
                    .unwrap_or("graph.commit");
                return Ok(json!({
                    "operationId": operation_id,
                    "operation": operation,
                    "status": "completed",
                    "durable": false,
                    "revision": result["revision"],
                    "createdAt": Value::Null,
                    "result": result
                }));
            }
        }
        if self.storage.is_some() {
            return Err(ControlError::InvalidRequest("operation not found".into()));
        }
        Ok(json!({
            "operationId": operation_id,
            "status": "unknown",
            "durable": false
        }))
    }

    fn dispatch_operation_cancel(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("operationId is required".into()))?;
        let operation_id = params
            .get("operationId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("operationId is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let request = json!({ "operationId": operation_id });
        let operation = (
            self.scoped_idempotency_key("operations.cancel", idempotency_key),
            Self::request_hash(&request),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        let lookup_keys = self.operation_lookup_keys(operation_id);
        let exists = if let Some(storage) = &self.storage {
            lookup_keys.iter().try_fold(false, |found, key| {
                Ok::<_, ControlError>(
                    found
                        || storage
                            .operation_status(key)
                            .map_err(storage_error)?
                            .is_some(),
                )
            })?
        } else {
            lookup_keys
                .iter()
                .any(|key| self.operation_outcomes.contains_key(key))
        };
        if !exists {
            return Err(ControlError::InvalidRequest("operation not found".into()));
        }
        let result = json!({
            "operationId": operation_id,
            "status": "completed",
            "cancelled": false,
            "reason": "alreadyCompleted"
        });
        self.journal_idempotent_result(&operation.0, "operations.cancel", &operation.1, &result)?;
        Ok(result)
    }

    /// Attach the encoder/queue owner for one session. Replacing an existing
    /// worker is rejected so a live destination cannot be orphaned silently.
    pub fn attach_recorder_worker(
        &mut self,
        session_id: EntityId,
        worker: Box<dyn RecorderWorker>,
    ) -> Result<(), ControlError> {
        if self.recorder_workers.contains_key(&session_id) {
            return Err(ControlError::InvalidRequest(
                "recorder worker is already attached".into(),
            ));
        }
        self.recorder_workers.insert(session_id, worker);
        Ok(())
    }

    /// Build the bounded engine observer set for one attached recorder. The
    /// returned set is immutable by convention after construction and can be
    /// handed to the realtime scheduler's tap-set method.
    pub fn recorder_tap_set(&self, session_id: &EntityId) -> Result<AudioTapSet, ControlError> {
        let worker = self.recorder_workers.get(session_id).ok_or_else(|| {
            ControlError::InvalidRequest("recorder worker is not attached".into())
        })?;
        let tap = worker.shared_audio_tap().ok_or_else(|| {
            ControlError::InvalidRequest("recorder worker has no realtime tap".into())
        })?;
        let mut set = AudioTapSet::new();
        set.add_shared(tap)
            .map_err(|_| ControlError::InvalidRequest("recorder tap capacity exceeded".into()))?;
        Ok(set)
    }

    /// Bind control-owned recorder workers to the session's validated recorder
    /// nodes. The compatibility session worker is accepted for one node;
    /// multiple nodes require one independently attached node worker each.
    pub fn recorder_tap_bindings(
        &self,
        session_id: &EntityId,
        generation: RuntimeGeneration,
    ) -> Result<RecorderTapBindings, ControlError> {
        let session = self.get_session(session_id)?;
        let recorder_nodes: Vec<&EntityId> = session
            .nodes
            .iter()
            .filter(|node| node.enabled && node.kind == NodeKind::Recorder)
            .map(|node| &node.id)
            .collect();
        if recorder_nodes.is_empty() {
            return Err(ControlError::InvalidRequest(
                "session must contain an enabled recorder node".into(),
            ));
        }
        let recorder_node_count = recorder_nodes.len();
        let mut bindings = RecorderTapBindings::new();
        for node_id in recorder_nodes {
            let tap = if let Some(worker) = self.recorder_node_workers.get(node_id) {
                worker.shared_audio_tap()
            } else if recorder_node_count == 1 {
                self.recorder_workers
                    .get(session_id)
                    .and_then(|worker| worker.shared_audio_tap())
            } else {
                None
            }
            .ok_or_else(|| {
                ControlError::InvalidRequest("recorder node worker is not attached".into())
            })?;
            bindings
                .add_shared(node_id.as_str(), generation, tap)
                .map_err(|_| {
                    ControlError::InvalidRequest("recorder node binding is invalid".into())
                })?;
        }
        Ok(bindings)
    }

    /// Attach a worker and independent lifecycle state to one validated
    /// recorder node. The node worker is the independent-sink path; JSON-RPC
    /// lifecycle addressing remains session-scoped until recorder IDs are
    /// promoted into that API.
    pub fn attach_recorder_worker_to_node(
        &mut self,
        session_id: &EntityId,
        node_id: EntityId,
        worker: Box<dyn RecorderWorker>,
    ) -> Result<(), ControlError> {
        let session = self.get_session(session_id)?;
        let node = session
            .nodes
            .iter()
            .find(|node| node.id == node_id && node.kind == NodeKind::Recorder && node.enabled)
            .ok_or_else(|| {
                ControlError::InvalidRequest("enabled recorder node is not in the session".into())
            })?;
        let _ = node;
        if self.recorder_node_workers.contains_key(&node_id) {
            return Err(ControlError::InvalidRequest(
                "recorder node worker is already attached".into(),
            ));
        }
        if worker.shared_audio_tap().is_none() {
            return Err(ControlError::InvalidRequest(
                "recorder worker has no realtime tap".into(),
            ));
        }
        self.recorder_node_workers.insert(node_id.clone(), worker);
        self.recorder_node_states
            .insert(node_id.clone(), RecorderController::new());
        self.recorder_node_sessions
            .insert(node_id, session_id.clone());
        Ok(())
    }

    /// Apply one lifecycle boundary to one node-owned recorder. All file and
    /// checkpoint work stays on this control/lifecycle path, never the audio
    /// callback. The returned shape mirrors the session recorder response but
    /// identifies the independent node.
    pub fn control_recorder_node(
        &mut self,
        node_id: &EntityId,
        method: &str,
        frame: Option<u64>,
    ) -> Result<Value, ControlError> {
        let session_id = self
            .recorder_node_sessions
            .get(node_id)
            .cloned()
            .ok_or_else(|| ControlError::InvalidRequest("recorder node is not attached".into()))?;
        let session_revision = self.get_session(&session_id)?.revision;
        if method == "recorders.arm"
            && !self
                .recorder_node_states
                .get(node_id)
                .is_some_and(|recorder| recorder_is_active(recorder.state()))
            && self
                .recorders
                .values()
                .chain(self.recorder_node_states.values())
                .filter(|recorder| recorder_is_active(recorder.state()))
                .count()
                >= MAX_ACTIVE_RECORDERS
        {
            return Err(ControlError::InvalidRequest(
                "active recorder limit reached".into(),
            ));
        }
        let frame = |required: bool| {
            if required {
                frame.ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))
            } else {
                Ok(frame.unwrap_or_default())
            }
        };
        let mut finalized_recordings = Vec::new();
        {
            let worker = self.recorder_node_workers.get_mut(node_id).ok_or_else(|| {
                ControlError::InvalidRequest("recorder node worker is not attached".into())
            })?;
            let result = match method {
                "recorders.arm" => worker.arm(),
                "recorders.start" => worker.start(frame(true)?),
                "recorders.pause" => worker.pause(frame(true)?),
                "recorders.resume" => worker.resume(frame(true)?),
                "recorders.split" => worker.split(frame(true)?),
                "recorders.stop" => {
                    let outcome = worker.finalize(frame(true)?).map_err(|error| {
                        ControlError::InvalidRequest(format!(
                            "recorder finalization failed: {error}"
                        ))
                    })?;
                    if outcome.state != "completed"
                        || !outcome.file_finalized
                        || outcome.recoverable
                    {
                        return Err(ControlError::InvalidRequest(
                            "recorder finalization did not produce a completed file".into(),
                        ));
                    }
                    finalized_recordings = worker.finalized_recordings();
                    Ok(())
                }
                _ => return Err(ControlError::InvalidRequest("method not found".into())),
            };
            result.map_err(|error| {
                ControlError::InvalidRequest(format!("recorder worker transition failed: {error}"))
            })?;
        }
        let (checkpoint, state, parts, pauses) = {
            let recorder = self.recorder_node_states.get_mut(node_id).ok_or_else(|| {
                ControlError::InvalidRequest("recorder node state is not attached".into())
            })?;
            let result = match method {
                "recorders.arm" => recorder.arm(),
                "recorders.start" => recorder.start(frame(true)?),
                "recorders.pause" => recorder.pause(frame(true)?),
                "recorders.resume" => recorder.resume(frame(true)?),
                "recorders.split" => recorder.split(frame(true)?),
                "recorders.stop" => recorder.stop(frame(true)?),
                _ => return Err(ControlError::InvalidRequest("method not found".into())),
            };
            result.map_err(|error| {
                ControlError::InvalidRequest(format!("recorder transition failed: {error:?}"))
            })?;
            let checkpoint = recorder.checkpoint();
            let state = recorder_state_name(recorder.state());
            let parts = checkpoint
                .parts
                .iter()
                .map(|part| {
                    json!({
                        "index": part.index,
                        "startFrame": part.start_frame,
                        "endFrame": part.end_frame,
                    })
                })
                .collect::<Vec<_>>();
            let pauses = checkpoint
                .pauses
                .iter()
                .map(|pause| {
                    json!({
                        "startFrame": pause.start_frame,
                        "endFrame": pause.end_frame,
                    })
                })
                .collect::<Vec<_>>();
            (checkpoint, state, parts, pauses)
        };
        if let Some(storage) = &self.storage {
            storage
                .save_recording_checkpoint(node_id.as_str(), &checkpoint)
                .map_err(storage_error)?;
            for recording in &finalized_recordings {
                storage.save_recording(recording).map_err(storage_error)?;
            }
        }
        let last_frame = checkpoint.last_frame;
        let result = json!({
            "nodeId": node_id,
            "sessionId": session_id,
            "state": state,
            "parts": parts,
            "pauses": pauses,
            "lastFrame": last_frame,
        });
        self.events
            .append(session_revision, None, "recorder.changed", Some(session_id));
        if method == "recorders.stop" {
            self.recorder_node_workers.remove(node_id);
            self.recorder_node_states.remove(node_id);
            self.recorder_node_sessions.remove(node_id);
        }
        Ok(result)
    }

    /// Attach a single-file worker and configure its durable library identity
    /// before it can be started. This is the factory/session boundary for
    /// callers that create WAV or FLAC workers outside the control plane.
    pub fn attach_recorder_worker_with_identity(
        &mut self,
        session_id: EntityId,
        identity: FileRecordingIdentity,
        mut worker: Box<dyn RecorderWorker>,
    ) -> Result<(), ControlError> {
        worker
            .set_library_identity(identity)
            .map_err(ControlError::InvalidRequest)?;
        self.attach_recorder_worker(session_id, worker)
    }

    /// Create and attach a path-owned WAV/FLAC recorder before any lifecycle
    /// command can arm it. File allocation and worker construction happen on
    /// this control/lifecycle boundary, never in the audio callback.
    #[allow(clippy::too_many_arguments)]
    pub fn create_and_attach_file_recorder(
        &mut self,
        policy: &RecordingPathPolicy,
        session_id: EntityId,
        recorder_id: &str,
        sequence: u64,
        format: FileRecorderFormat,
        channels: u16,
        sample_rate: u32,
        dither: bool,
        queue_capacity: usize,
        maximum_chunks_per_pass: usize,
    ) -> Result<std::path::PathBuf, ControlError> {
        let (path, worker) = create_file_recorder(
            policy,
            session_id.as_str(),
            recorder_id,
            sequence,
            format,
            channels,
            sample_rate,
            dither,
            queue_capacity,
            maximum_chunks_per_pass,
        )
        .map_err(ControlError::InvalidRequest)?;
        self.attach_recorder_worker(session_id, worker)?;
        Ok(path)
    }

    pub fn create_and_attach_file_recorder_with_config(
        &mut self,
        policy: &RecordingPathPolicy,
        session_id: EntityId,
        config: &FileRecorderConfig<'_>,
    ) -> Result<std::path::PathBuf, ControlError> {
        if config.session_id != session_id.as_str() {
            return Err(ControlError::InvalidRequest(
                "file recorder session identity does not match attachment".into(),
            ));
        }
        let (path, worker) = create_file_recorder_with_config(policy, config)
            .map_err(ControlError::InvalidRequest)?;
        if let Err(error) = self.attach_recorder_worker(session_id, worker) {
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
        Ok(path)
    }

    /// Create and attach using the backend-owned, persisted recording root.
    /// No caller-supplied destination policy is accepted at this boundary.
    pub fn create_and_attach_configured_file_recorder(
        &mut self,
        session_id: EntityId,
        config: &FileRecorderConfig<'_>,
    ) -> Result<std::path::PathBuf, ControlError> {
        if config.session_id != session_id.as_str() {
            return Err(ControlError::InvalidRequest(
                "file recorder session identity does not match attachment".into(),
            ));
        }
        let (path, worker) = {
            let policy = self.recording_policy.as_ref().ok_or_else(|| {
                ControlError::InvalidRequest("recording root is not configured".into())
            })?;
            create_file_recorder_with_config(policy, config)
                .map_err(ControlError::InvalidRequest)?
        };
        if let Err(error) = self.attach_recorder_worker(session_id, worker) {
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
        Ok(path)
    }

    pub fn insert_session(&mut self, session: Session) -> Result<(), ControlError> {
        let checkpoint = self.store.clone();
        self.store
            .insert_session(session.clone())
            .map_err(ControlError::from)?;
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_session(&session) {
                self.store = checkpoint;
                return Err(storage_error(error));
            }
        }
        self.events.append(
            session.revision,
            None,
            "session.created",
            Some(session.id.clone()),
        );
        Ok(())
    }

    pub fn create_session(&mut self, session: Session) -> Result<Value, ControlError> {
        if session.revision != 0 {
            return Err(ControlError::InvalidRequest(
                "new sessions must start at revision 0".into(),
            ));
        }
        self.insert_session(session.clone())?;
        Ok(json!({ "session": session, "state": "stopped" }))
    }

    pub fn duplicate_session(
        &mut self,
        source_id: &EntityId,
        duplicate_id: EntityId,
        name: Option<String>,
    ) -> Result<Value, ControlError> {
        self.ensure_session_loaded(source_id)?;
        let source = self.get_session(source_id)?.clone();
        if self.store.session(&duplicate_id).is_some() {
            return Err(ControlError::InvalidRequest(
                "duplicate session ID already exists".into(),
            ));
        }
        let duplicate = Session {
            id: duplicate_id,
            name: name.unwrap_or_else(|| format!("{} (copy)", source.name)),
            revision: 0,
            ..source
        };
        self.create_session(duplicate)
    }

    pub fn delete_session(&mut self, id: &EntityId) -> Result<Value, ControlError> {
        self.ensure_session_loaded(id)?;
        let session = self.get_session(id)?.clone();
        if self
            .runtimes
            .get(id)
            .map(|runtime| runtime.state() == RuntimeState::Running)
            .unwrap_or(false)
        {
            return Err(ControlError::InvalidRequest(
                "stop the session before deleting it".into(),
            ));
        }
        let checkpoint = self.store.clone();
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.delete_session(id) {
                self.store = checkpoint;
                return Err(storage_error(error));
            }
        }
        self.store.remove_session(id).map_err(ControlError::from)?;
        self.runtimes.remove(id);
        for node in session
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Recorder)
        {
            self.recorder_node_workers.remove(&node.id);
        }
        self.events
            .append(session.revision, None, "session.deleted", Some(id.clone()));
        Ok(json!({ "sessionId": id, "deleted": true }))
    }

    pub fn describe(&self) -> Value {
        let methods: Vec<MethodDescription> = API_METHODS.iter().copied().map(Into::into).collect();
        let nodes: Vec<Value> = node_registry().into_iter().map(|spec| {
            let availability = match spec.availability {
                audiorouter_domain::CapabilityAvailability::Available => json!({ "status": "available" }),
                audiorouter_domain::CapabilityAvailability::Unavailable(reason) => json!({ "status": "unavailable", "reason": reason }),
            };
            json!({ "type": format!("{}@{}", spec.kind.type_name(), spec.version), "availability": availability, "realtimeCostClass": spec.realtime_cost_class, "latencySamples": spec.latency_samples, "parameters": Self::node_parameter_schema(spec.kind) })
        }).collect();
        let voice_chains = audiorouter_dsp::VoiceChainPresetId::ALL
            .into_iter()
            .map(|preset| {
                json!({
                    "id": preset.id(),
                    "version": preset.version(),
                    "name": preset.name(),
                    "description": preset.description()
                })
            })
            .collect::<Vec<_>>();
        let eq = audiorouter_dsp::EqPresetId::ALL
            .into_iter()
            .map(|preset| {
                json!({
                    "id": preset.id(),
                    "version": preset.version(),
                    "name": preset.name(),
                    "description": preset.description()
                })
            })
            .collect::<Vec<_>>();
        json!({
            "protocolVersion": { "major": 1, "minor": 0 },
            "schemaVersion": 1,
            "build": self.build,
            "methods": methods,
            "nodeTypes": nodes,
            "processors": Self::processor_catalog(),
            "presets": { "voiceChains": voice_chains, "eq": eq },
            "limits": {
                "maxNodesPerSession": audiorouter_domain::MAX_NODES_PER_SESSION,
                "maxEdgesPerSession": audiorouter_domain::MAX_EDGES_PER_SESSION,
                "maxNodesGlobal": audiorouter_domain::MAX_NODES_GLOBAL,
                "maxEdgesGlobal": audiorouter_domain::MAX_EDGES_GLOBAL,
                "maxSessionsGlobal": audiorouter_domain::MAX_SESSIONS_GLOBAL,
                "maxActiveSessions": audiorouter_domain::MAX_ACTIVE_SESSIONS,
                "maxActiveRecorders": MAX_ACTIVE_RECORDERS,
                "maxClientEnrollments": audiorouter_storage::MAX_CLIENT_ENROLLMENTS,
                "maxOperationJournalEntries": audiorouter_storage::MAX_OPERATION_JOURNAL_ENTRIES,
                "maxVirtualBuses": audiorouter_domain::MAX_VIRTUAL_BUSES,
                "maxVirtualBusNameChars": audiorouter_domain::MAX_VIRTUAL_BUS_NAME_CHARS,
                "maxEntityIdBytes": audiorouter_domain::MAX_ENTITY_ID_BYTES,
                "maxDisplayNameBytes": audiorouter_domain::MAX_DISPLAY_NAME_BYTES,
                "maxPortNameBytes": audiorouter_domain::MAX_PORT_NAME_BYTES,
                "maxPortsPerNode": audiorouter_domain::MAX_PORTS_PER_NODE,
                "maxChannelMatrixCoefficients": audiorouter_domain::MAX_CHANNEL_MATRIX_COEFFICIENTS,
                "maxControlValueDepth": MAX_CONTROL_VALUE_DEPTH,
                "maxControlStringBytes": MAX_CONTROL_STRING_BYTES,
                "maxControlValueCount": MAX_CONTROL_VALUE_COUNT,
                "maxMethodNameBytes": MAX_METHOD_NAME_BYTES,
                "maxRequestIdBytes": MAX_REQUEST_ID_BYTES,
                "maxRevisionCursorBytes": MAX_REVISION_CURSOR_BYTES
            },
            "events": {
                "stateCategories": STATE_CATEGORIES,
                "meterReplay": false,
                "retention": {
                    "maxEvents": audiorouter_domain::MAX_RETAINED_EVENTS,
                    "maxAgeSeconds": 900
                }
            }
        })
    }

    fn node_parameter_schema(kind: audiorouter_domain::NodeKind) -> Value {
        match kind {
            audiorouter_domain::NodeKind::Gain => json!([{
                "name": "gainDb",
                "type": "number",
                "unit": "dB",
                "minimum": -60.0,
                "maximum": 24.0,
                "default": 0.0
            }]),
            audiorouter_domain::NodeKind::Mute => json!([{
                "name": "muted",
                "type": "boolean",
                "default": false
            }]),
            audiorouter_domain::NodeKind::ParametricEq => Self::parametric_eq_parameters(),
            audiorouter_domain::NodeKind::Compressor => json!([
                { "name": "thresholdDb", "type": "number", "unit": "dBFS", "minimum": -60.0, "maximum": 0.0, "default": -18.0 },
                { "name": "ratio", "type": "number", "minimum": 1.0, "maximum": 20.0, "default": 3.0 },
                { "name": "attackMs", "type": "number", "unit": "ms", "minimum": 0.1, "maximum": 200.0, "default": 10.0 },
                { "name": "releaseMs", "type": "number", "unit": "ms", "minimum": 10.0, "maximum": 2000.0, "default": 150.0 },
                { "name": "kneeDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 24.0, "default": 6.0 },
                { "name": "makeupDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 24.0, "default": 0.0 }
            ]),
            audiorouter_domain::NodeKind::Gate => json!([
                { "name": "thresholdDb", "type": "number", "unit": "dBFS", "minimum": -80.0, "maximum": 0.0, "default": -45.0 },
                { "name": "rangeDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 80.0, "default": 60.0 },
                { "name": "attackMs", "type": "number", "unit": "ms", "minimum": 0.1, "maximum": 100.0, "default": 5.0 },
                { "name": "hysteresisDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 12.0, "default": 3.0 },
                { "name": "ratio", "type": "number", "minimum": 1.0, "maximum": 20.0, "default": 4.0 },
                { "name": "holdMs", "type": "number", "unit": "ms", "minimum": 0.0, "maximum": 1000.0, "default": 50.0 },
                { "name": "releaseMs", "type": "number", "unit": "ms", "minimum": 10.0, "maximum": 2000.0, "default": 150.0 }
            ]),
            audiorouter_domain::NodeKind::Limiter => json!([
                { "name": "ceilingDb", "type": "number", "unit": "dBFS", "minimum": -12.0, "maximum": 0.0, "default": -1.0 },
                { "name": "lookaheadMs", "type": "number", "unit": "ms", "minimum": 0.0, "maximum": 10.0, "default": 5.0 },
                { "name": "releaseMs", "type": "number", "unit": "ms", "minimum": 10.0, "maximum": 1000.0, "default": 100.0 }
            ]),
            audiorouter_domain::NodeKind::Delay => json!([
                { "name": "delayMs", "type": "number", "unit": "ms", "minimum": 0.0, "maximum": 1000.0, "default": 0.0 }
            ]),
            audiorouter_domain::NodeKind::GraphicEq => json!((0..10)
                .map(|index| json!({
                    "name": format!("band{index}Db"), "type": "number", "unit": "dB",
                    "minimum": -18.0, "maximum": 18.0, "default": 0.0
                }))
                .collect::<Vec<_>>()),
            audiorouter_domain::NodeKind::Pitch => json!([
                { "name": "semitones", "type": "number", "unit": "semitones", "minimum": -12.0, "maximum": 12.0, "default": 0.0 },
                { "name": "cents", "type": "number", "unit": "cents", "minimum": -100.0, "maximum": 100.0, "default": 0.0 }
            ]),
            _ => json!([]),
        }
    }

    fn parametric_eq_parameters() -> Value {
        let mut parameters = vec![
            json!({ "name": "frequencyHz", "type": "number", "unit": "Hz", "minimum": 20.0, "maximum": 20000.0, "default": 1000.0 }),
            json!({ "name": "q", "type": "number", "minimum": 0.1, "maximum": 20.0, "default": 1.0 }),
            json!({ "name": "gainDb", "type": "number", "unit": "dB", "minimum": -24.0, "maximum": 24.0, "default": 0.0 }),
        ];
        for index in 0..8 {
            parameters.extend([
                json!({ "name": format!("band{index}Enabled"), "type": "boolean", "default": false }),
                json!({ "name": format!("band{index}Type"), "type": "string", "enum": ["peaking", "lowShelf", "highShelf", "lowPass", "highPass", "notch"], "default": "peaking" }),
                json!({ "name": format!("band{index}FrequencyHz"), "type": "number", "unit": "Hz", "minimum": 20.0, "maximum": 20000.0, "default": 1000.0 }),
                json!({ "name": format!("band{index}Q"), "type": "number", "minimum": 0.1, "maximum": 20.0, "default": 1.0 }),
                json!({ "name": format!("band{index}GainDb"), "type": "number", "unit": "dB", "minimum": -24.0, "maximum": 24.0, "default": 0.0 }),
            ]);
        }
        Value::Array(parameters)
    }

    fn processor_catalog() -> Value {
        let available = json!({ "status": "available" });
        json!([
            {
                "id": "graphicEq", "version": 1, "category": "equalizer",
                "availability": available, "latencySamples": 0,
                "parameters": (0..10).map(|index| json!({ "name": format!("band{index}Db"), "type": "number", "unit": "dB", "minimum": -18.0, "maximum": 18.0, "default": 0.0 })).collect::<Vec<_>>()
            },
            {
                "id": "parametricEq", "version": 1, "category": "equalizer",
                "availability": available, "latencySamples": 0,
                "parameters": Self::parametric_eq_parameters()
            },
            {
                "id": "gate", "version": 1, "category": "dynamics",
                "availability": available, "latencySamples": 0,
                "parameters": [
                    { "name": "thresholdDb", "type": "number", "unit": "dBFS", "minimum": -80.0, "maximum": 0.0, "default": -45.0 },
                    { "name": "rangeDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 80.0, "default": 60.0 },
                    { "name": "hysteresisDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 12.0, "default": 3.0 },
                    { "name": "ratio", "type": "number", "minimum": 1.0, "maximum": 20.0, "default": 4.0 },
                    { "name": "attackMs", "type": "number", "unit": "ms", "minimum": 0.1, "maximum": 100.0, "default": 5.0 },
                    { "name": "holdMs", "type": "number", "unit": "ms", "minimum": 0.0, "maximum": 1000.0, "default": 50.0 },
                    { "name": "releaseMs", "type": "number", "unit": "ms", "minimum": 10.0, "maximum": 2000.0, "default": 150.0 }
                ]
            },
            {
                "id": "compressor", "version": 1, "category": "dynamics",
                "availability": available, "latencySamples": 0,
                "parameters": [
                    { "name": "thresholdDb", "type": "number", "unit": "dBFS", "minimum": -60.0, "maximum": 0.0, "default": -18.0 },
                    { "name": "ratio", "type": "number", "minimum": 1.0, "maximum": 20.0, "default": 3.0 },
                    { "name": "attackMs", "type": "number", "unit": "ms", "minimum": 0.1, "maximum": 200.0, "default": 10.0 },
                    { "name": "releaseMs", "type": "number", "unit": "ms", "minimum": 10.0, "maximum": 2000.0, "default": 150.0 },
                    { "name": "kneeDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 24.0, "default": 6.0 },
                    { "name": "makeupDb", "type": "number", "unit": "dB", "minimum": 0.0, "maximum": 24.0, "default": 0.0 }
                ]
            },
            {
                "id": "limiter", "version": 1, "category": "dynamics",
                "availability": available, "latencySamples": 240,
                "parameters": [
                    { "name": "ceilingDb", "type": "number", "unit": "dBFS", "minimum": -12.0, "maximum": 0.0, "default": -1.0 },
                    { "name": "lookaheadMs", "type": "number", "unit": "ms", "minimum": 0.0, "maximum": 10.0, "default": 5.0 },
                    { "name": "releaseMs", "type": "number", "unit": "ms", "minimum": 10.0, "maximum": 1000.0, "default": 100.0 }
                ]
            },
            {
                "id": "delay", "version": 1, "category": "time",
                "availability": available, "latencySamples": 0,
                "parameters": [{ "name": "delayMs", "type": "number", "unit": "ms", "minimum": 0.0, "maximum": 1000.0, "default": 0.0 }]
            },
            {
                "id": "pitch", "version": 1, "category": "pitch",
                "availability": available, "latencySamples": 1024,
                "parameters": [
                    { "name": "semitones", "type": "number", "unit": "semitones", "minimum": -12.0, "maximum": 12.0, "default": 0.0 },
                    { "name": "cents", "type": "number", "unit": "cents", "minimum": -100.0, "maximum": 100.0, "default": 0.0 }
                ]
            }
        ])
    }

    fn recovery_status(&self) -> Result<(usize, bool), ControlError> {
        self.storage.as_ref().map_or(Ok((0, false)), |storage| {
            let status = storage
                .recovery_status(unix_epoch_seconds() as u64)
                .map_err(storage_error)?;
            Ok((status.recent_crashes, status.safe_mode))
        })
    }

    /// Record one backend/runtime crash and return the bounded recovery
    /// decision that a future process supervisor must apply. This boundary
    /// records policy state only: it never creates a process, starts a session,
    /// resumes a route, or opens an audio stream.
    pub fn record_runtime_crash(
        &mut self,
        timestamp_seconds: u64,
    ) -> Result<RecoveryDecision, ControlError> {
        let (mode, recent_crashes) = if let Some(storage) = &self.storage {
            storage
                .record_recovery_crash(timestamp_seconds)
                .map_err(storage_error)?;
            let status = storage
                .recovery_status(timestamp_seconds)
                .map_err(storage_error)?;
            (
                if status.safe_mode {
                    RecoveryMode::SafeMode
                } else {
                    RecoveryMode::RestoreEligible
                },
                status.recent_crashes,
            )
        } else {
            let mode = self.recovery_tracker.record_crash(timestamp_seconds);
            let recent_crashes = self.recovery_tracker.crash_count(timestamp_seconds);
            (mode, recent_crashes)
        };
        let mut session_ids = if mode == RecoveryMode::SafeMode || recent_crashes == 0 {
            Vec::new()
        } else {
            self.runtimes
                .iter()
                .filter(|(id, runtime)| {
                    runtime.state() == RuntimeState::Running
                        && !self.recorders.get(*id).is_some_and(|recorder| {
                            matches!(
                                recorder.state(),
                                RecorderState::Armed
                                    | RecorderState::Recording
                                    | RecorderState::Paused
                                    | RecorderState::Stopping
                            )
                        })
                })
                .map(|(id, _)| id.clone())
                .collect()
        };
        session_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        Ok(RecoveryDecision { mode, session_ids })
    }

    /// Apply one supervisor recovery decision to the portable runtime model.
    ///
    /// A crashed backend has no usable live runtime, so every currently
    /// running fake runtime is stopped before the policy result is applied.
    /// Only non-recording sessions returned by `record_runtime_crash` are
    /// restarted, and safe mode therefore leaves all sessions stopped. This
    /// method is deliberately limited to the fake runtime boundary: it does
    /// not create a process, open an audio stream, or claim native route
    /// recovery.
    pub fn recover_after_runtime_crash(
        &mut self,
        timestamp_seconds: u64,
    ) -> Result<RecoveryDecision, ControlError> {
        let decision = self.record_runtime_crash(timestamp_seconds)?;
        let crashed_session_ids = self
            .runtimes
            .iter()
            .filter(|(_, runtime)| runtime.state() == RuntimeState::Running)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for session_id in &crashed_session_ids {
            let revision = self.get_session(session_id)?.revision;
            self.events
                .append(revision, None, "runtime.crashed", Some(session_id.clone()));
        }
        for runtime in self.runtimes.values_mut() {
            if runtime.state() == RuntimeState::Running {
                runtime.stop();
            }
        }
        if decision.mode == RecoveryMode::RestoreEligible {
            for session_id in &decision.session_ids {
                self.session_start(session_id)?;
            }
        }
        Ok(decision)
    }

    fn status_snapshot(&self) -> Result<Value, ControlError> {
        let mut active_session_ids = self
            .runtimes
            .iter()
            .filter(|(_, runtime)| runtime.state() == RuntimeState::Running)
            .map(|(id, _)| id.as_str().to_owned())
            .collect::<Vec<_>>();
        active_session_ids.sort();
        let session_count = if let Some(storage) = &self.storage {
            storage.count_sessions().map_err(storage_error)?
        } else {
            self.store.sessions(500).len()
        };
        let (recent_recovery_crashes, recovery_safe_mode) = self.recovery_status()?;
        Ok(json!({
            "build": self.build,
            "audio": "unavailable",
            "deviceDiscovery": "available",
            "reason": "native endpoint routing is implemented but not activated; exact bindings and a production driver are required",
            "storage": if self.storage.is_some() { "sqlite" } else { "memory" },
            "sessionCount": session_count,
            "activeSessionCount": active_session_ids.len(),
            "activeSessionIds": active_session_ids,
            "privacyMute": {
                "muted": self.privacy_muted,
                "persistence": if self.storage.is_some() { "durable" } else { "memory" },
                "audioEffect": "process-local-when-realtime-backend-is-available"
            },
            "recovery": {
                "safeMode": recovery_safe_mode,
                "recentCrashes": recent_recovery_crashes,
                "persistence": if self.storage.is_some() { "durable" } else { "memory" }
            },
            "eventCursor": {
                "backendEpoch": self.events.backend_epoch(),
                "latestSequence": self.events.latest_sequence()
            }
        }))
    }

    pub fn get_session(&self, id: &EntityId) -> Result<&Session, ControlError> {
        self.store
            .session(id)
            .ok_or(ControlError::InvalidRequest("session not found".into()))
    }

    pub fn inspect_routes(
        &self,
        session_id: &EntityId,
        destination_node: &EntityId,
    ) -> Result<Value, ControlError> {
        let session = self.get_session(session_id)?;
        serde_json::to_value(inspect_routes(session, destination_node).map_err(|errors| {
            ControlError::InvalidRequest(format!(
                "invalid graph: {}",
                format_validation_errors(&errors)
            ))
        })?)
        .map_err(|error| ControlError::Json(error.to_string()))
    }

    pub fn graph_history(
        &self,
        session_id: &EntityId,
        limit: usize,
    ) -> Result<Value, ControlError> {
        self.graph_history_page(session_id, None, limit)
            .map(|page| page["items"].clone())
    }

    pub fn graph_history_page(
        &self,
        session_id: &EntityId,
        before_revision: Option<u64>,
        limit: usize,
    ) -> Result<Value, ControlError> {
        let limit = limit.clamp(1, MAX_GRAPH_HISTORY_ITEMS);
        let history = if self.store.session(session_id).is_some() {
            self.store
                .history_before(session_id, before_revision, limit + 1)
        } else if let Some(storage) = &self.storage {
            storage
                .load_history_before(session_id, before_revision, limit + 1)
                .map_err(storage_error)?
        } else {
            Vec::new()
        };
        let has_more = history.len() > limit;
        let mut history = history;
        history.truncate(limit);
        let next_cursor = has_more
            .then(|| history.last().map(|session| session.revision))
            .flatten()
            .map(|revision| revision.to_string());
        Ok(json!({ "items": history, "nextCursor": next_cursor }))
    }

    pub fn graph_undo_plan(
        &mut self,
        session_id: &EntityId,
        base_revision: u64,
    ) -> Result<EntityId, ControlError> {
        self.ensure_session_loaded(session_id)?;
        if self.store.history(session_id, 2).len() < 2 {
            if let Some(storage) = &self.storage {
                let entries = storage
                    .load_history(session_id, 100)
                    .map_err(storage_error)?;
                self.store
                    .restore_history(entries)
                    .map_err(ControlError::from)?;
            }
        }
        self.store
            .undo_plan(session_id, base_revision)
            .map_err(Into::into)
    }

    pub fn sessions_list(&self, limit: usize) -> Result<Value, ControlError> {
        self.sessions_list_page(None, limit)
            .map(|page| page["items"].clone())
    }

    pub fn sessions_list_page(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, ControlError> {
        if !(1..=MAX_SESSION_LIST_ITEMS).contains(&limit) {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 500".into(),
            ));
        }
        let sessions = if let Some(storage) = &self.storage {
            storage
                .list_sessions_after(cursor, limit)
                .map_err(storage_error)?
        } else {
            self.store.sessions_after(cursor, limit)
        };
        let next_cursor = (sessions.len() == limit)
            .then(|| {
                sessions
                    .last()
                    .map(|session| session.id.as_str().to_owned())
            })
            .flatten();
        Ok(json!({ "items": sessions, "nextCursor": next_cursor }))
    }

    pub fn plan_graph(
        &mut self,
        session_id: &EntityId,
        base_revision: u64,
        candidate: Session,
    ) -> Result<EntityId, ControlError> {
        let checkpoint = self.store.clone();
        let plan_id = self
            .store
            .plan_graph(session_id, base_revision, candidate.clone())
            .map_err(ControlError::from)?;
        if let Some(storage) = &self.storage {
            let expires_at = unix_epoch_seconds() + GRAPH_PLAN_RETENTION_SECONDS;
            if let Err(error) = storage.save_graph_plan(&GraphPlanRecord {
                id: plan_id.as_str().to_owned(),
                session_id: session_id.as_str().to_owned(),
                base_revision,
                candidate,
                expires_at,
            }) {
                self.store = checkpoint;
                return Err(storage_error(error));
            }
        }
        Ok(plan_id)
    }

    pub fn commit_graph(
        &mut self,
        plan_id: &EntityId,
        base_revision: u64,
        idempotency_key: &str,
    ) -> Result<Value, ControlError> {
        self.commit_graph_scoped(plan_id, base_revision, idempotency_key, idempotency_key)
    }

    fn commit_graph_scoped(
        &mut self,
        plan_id: &EntityId,
        base_revision: u64,
        idempotency_key: &str,
        display_operation_id: &str,
    ) -> Result<Value, ControlError> {
        let checkpoint = self.store.clone();
        let fingerprint = format!("graph.commit:{}:{}", plan_id.as_str(), base_revision);
        let request_hash = format!("{:x}", Sha256::digest(fingerprint.as_bytes()));
        if let Some(storage) = &self.storage {
            if let Some(result) = storage
                .journal_result_checked(idempotency_key, &request_hash)
                .map_err(storage_error)?
            {
                let mut response: Value = serde_json::from_str(&result)
                    .map_err(|error| ControlError::Json(error.to_string()))?;
                response["idempotentReplay"] = json!(true);
                response["activation"] = json!({ "state": "pending", "runtime": "fake" });
                return Ok(response);
            }
        }
        let result = match self
            .store
            .commit_graph(plan_id, base_revision, idempotency_key)
        {
            Ok(result) => result,
            Err(audiorouter_domain::StoreError::PlanNotFound) => {
                let storage = self.storage.as_ref().ok_or(ControlError::from(
                    audiorouter_domain::StoreError::PlanNotFound,
                ))?;
                let durable = storage
                    .load_graph_plan(plan_id.as_str())
                    .map_err(storage_error)?
                    .ok_or(ControlError::from(
                        audiorouter_domain::StoreError::PlanNotFound,
                    ))?;
                let Some(remaining) = remaining_persisted_plan_duration(
                    durable.expires_at,
                    unix_epoch_seconds(),
                    Duration::from_secs(GRAPH_PLAN_RETENTION_SECONDS as u64),
                ) else {
                    storage
                        .delete_graph_plan(plan_id.as_str())
                        .map_err(storage_error)?;
                    return Err(ControlError::from(
                        audiorouter_domain::StoreError::PlanExpired,
                    ));
                };
                let durable_session_id = EntityId::new(durable.session_id.clone());
                self.ensure_session_loaded(&durable_session_id)?;
                self.store
                    .restore_plan_with_ttl(
                        EntityId::new(durable.id),
                        &durable_session_id,
                        durable.base_revision,
                        durable.candidate,
                        remaining,
                    )
                    .map_err(ControlError::from)?;
                self.store
                    .commit_graph(plan_id, base_revision, idempotency_key)
                    .map_err(ControlError::from)?
            }
            Err(error) => return Err(ControlError::from(error)),
        };
        if let Some(storage) = &self.storage {
            let session = self.store.session(&result.session_id).ok_or_else(|| {
                ControlError::InvalidRequest("committed session not found".into())
            })?;
            let result_document = serde_json::to_string(&result)
                .map_err(|error| ControlError::Json(error.to_string()))?;
            if let Err(error) = storage.save_session_with_journal_with_hash(
                session,
                idempotency_key,
                "graph.commit",
                &result_document,
                &request_hash,
                None,
            ) {
                self.store = checkpoint;
                return Err(storage_error(error));
            }
            storage
                .delete_graph_plan(plan_id.as_str())
                .map_err(storage_error)?;
        }
        if !result.idempotent_replay {
            self.events.append(
                result.revision,
                Some(display_operation_id.into()),
                "graph.committed",
                Some(result.session_id.clone()),
            );
        }
        let mut response =
            serde_json::to_value(&result).map_err(|error| ControlError::Json(error.to_string()))?;
        if !result.idempotent_replay
            && self
                .runtimes
                .get(&result.session_id)
                .map(|runtime| runtime.state() == RuntimeState::Running)
                .unwrap_or(false)
        {
            let session = self
                .store
                .session(&result.session_id)
                .cloned()
                .ok_or_else(|| {
                    ControlError::InvalidRequest("committed session not found".into())
                })?;
            let runtime = self.runtimes.get_mut(&result.session_id).unwrap();
            runtime.prepare(&session).map_err(|error| match error {
                RuntimeError::InvalidGraph(errors) => ControlError::InvalidRequest(format!(
                    "invalid graph: {}",
                    format_validation_errors(&errors)
                )),
                RuntimeError::NotPrepared => {
                    ControlError::InvalidRequest("session was not prepared".into())
                }
            })?;
            let generation = runtime
                .start()
                .map_err(|_| ControlError::InvalidRequest("session was not prepared".into()))?;
            self.events.append(
                result.revision,
                Some(display_operation_id.into()),
                "runtime.activated",
                Some(result.session_id.clone()),
            );
            response["activation"] =
                json!({ "state": "running", "generation": generation, "runtime": "fake" });
        } else {
            response["activation"] = json!({ "state": "pending", "runtime": "fake" });
        }
        self.remember_operation_outcome(idempotency_key, response.clone(), "graph.commit", None);
        Ok(response)
    }

    pub fn session_start(&mut self, id: &EntityId) -> Result<Value, ControlError> {
        self.ensure_session_loaded(id)?;
        let session = self.get_session(id)?.clone();
        if let Some(runtime) = self.runtimes.get(id) {
            if runtime.state() == RuntimeState::Running {
                return Ok(
                    json!({ "sessionId": id, "state": "running", "generation": runtime.generation(), "runtime": "fake" }),
                );
            }
        }
        if self
            .runtimes
            .values()
            .filter(|runtime| runtime.state() == RuntimeState::Running)
            .count()
            >= audiorouter_domain::MAX_ACTIVE_SESSIONS
        {
            return Err(ControlError::InvalidRequest(
                "active session limit reached".into(),
            ));
        }
        let runtime = self.runtimes.entry(id.clone()).or_default();
        runtime.prepare(&session).map_err(|error| match error {
            RuntimeError::InvalidGraph(errors) => ControlError::InvalidRequest(format!(
                "invalid graph: {}",
                format_validation_errors(&errors)
            )),
            RuntimeError::NotPrepared => {
                ControlError::InvalidRequest("session was not prepared".into())
            }
        })?;
        let generation = runtime
            .start()
            .map_err(|_| ControlError::InvalidRequest("session was not prepared".into()))?;
        self.events
            .append(session.revision, None, "runtime.started", Some(id.clone()));
        Ok(
            json!({ "sessionId": id, "state": "running", "generation": generation, "runtime": "fake" }),
        )
    }

    pub fn session_stop(&mut self, id: &EntityId) -> Result<Value, ControlError> {
        self.ensure_session_loaded(id)?;
        let has_node_worker = self
            .get_session(id)?
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Recorder)
            .any(|node| self.recorder_node_workers.contains_key(&node.id));
        if has_node_worker {
            return Err(ControlError::InvalidRequest(
                "node-keyed recorder lifecycle must be finalized before stopping the session"
                    .into(),
            ));
        }
        let mut recorder_outcomes = Vec::new();
        let active_frame = self.recorders.get(id).and_then(|recorder| {
            matches!(
                recorder.state(),
                RecorderState::Recording | RecorderState::Paused | RecorderState::Stopping
            )
            .then(|| recorder.checkpoint().last_frame)
            .flatten()
        });
        if let Some(frame) = active_frame {
            let worker = self.recorder_workers.get_mut(id).ok_or_else(|| {
                ControlError::InvalidRequest(
                    "finalize the active recorder before stopping the session".into(),
                )
            })?;
            let outcome = worker.finalize(frame).map_err(|error| {
                ControlError::InvalidRequest(format!("recorder finalization failed: {error}"))
            })?;
            if outcome.state != "completed" || !outcome.file_finalized || outcome.recoverable {
                return Err(ControlError::InvalidRequest(
                    "recorder finalization did not produce a completed file".into(),
                ));
            }
            let finalized_recordings = worker.finalized_recordings();
            let recorder = self.recorders.get_mut(id).ok_or_else(|| {
                ControlError::InvalidRequest("active recorder state disappeared".into())
            })?;
            let boundary_result = if recorder.state() == RecorderState::Stopping {
                recorder.complete()
            } else {
                recorder.stop(frame)
            };
            boundary_result.map_err(|error| {
                ControlError::InvalidRequest(format!(
                    "recorder boundary finalization failed: {error:?}"
                ))
            })?;
            let checkpoint = recorder.checkpoint();
            if let Some(storage) = &self.storage {
                storage
                    .save_recording_checkpoint(id.as_str(), &checkpoint)
                    .map_err(storage_error)?;
                for recording in &finalized_recordings {
                    storage.save_recording(recording).map_err(storage_error)?;
                }
            }
            self.events.append(
                self.get_session(id)?.revision,
                None,
                "recorder.changed",
                Some(id.clone()),
            );
            self.recorder_workers.remove(id);
            recorder_outcomes.push(json!({
                "sessionId": id,
                "state": "completed",
                "fileFinalized": true,
                "recoverable": false
            }));
        }
        let revision = self.get_session(id)?.revision;
        if let Some(runtime) = self.runtimes.get_mut(id) {
            runtime.stop();
        }
        self.events
            .append(revision, None, "runtime.stopped", Some(id.clone()));
        Ok(json!({
            "sessionId": id,
            "state": "stopped",
            "runtime": "fake",
            "recorders": recorder_outcomes
        }))
    }

    fn ensure_session_loaded(&mut self, id: &EntityId) -> Result<(), ControlError> {
        if self.store.session(id).is_some() {
            return Ok(());
        }
        let Some(storage) = &self.storage else {
            return Ok(());
        };
        if let Some(session) = storage.load_session(id).map_err(storage_error)? {
            self.store
                .insert_session(session)
                .map_err(ControlError::from)?;
        }
        Ok(())
    }

    pub fn dispatch(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();
        if request.validate().is_err() {
            return JsonRpcResponse::failure(id, -32600, "invalid request");
        }
        let mutating = is_mutating_method(&request.method);
        if request.is_notification() && mutating {
            return JsonRpcResponse::failure(
                None,
                -32600,
                "mutating notifications are not supported",
            );
        }
        let result =
            validate_method_params(&request.method, request.params.as_ref()).and_then(|_| {
                match request.method.as_str() {
                    "system.describe" => Ok(self.describe()),
                    "system.handshake" => self.dispatch_handshake(request.params),
                    "status.get" => self.status_snapshot(),
                    "system.diagnostics" => {
                        let (recent_recovery_crashes, recovery_safe_mode) =
                            self.recovery_status()?;
                        Ok(json!({
                        "build": self.build,
                        "backend": "control-plane",
                        "storage": if self.storage.is_some() { "sqlite" } else { "memory" },
                        "audio": {
                            "state": "unavailable",
                            "reason": "native endpoint routing is implemented but not activated; exact bindings and a production driver are required"
                        },
                        "nativeAdapter": "implemented-not-activated",
                        "privacyMute": {
                            "muted": self.privacy_muted,
                            "persistence": if self.storage.is_some() { "durable" } else { "memory" }
                        },
                        "recovery": {
                            "safeMode": recovery_safe_mode,
                            "recentCrashes": recent_recovery_crashes,
                            "persistence": if self.storage.is_some() { "durable" } else { "memory" }
                        },
                        "eventLog": {
                            "latestSequence": self.events.latest_sequence(),
                            "retained": self.events.len()
                        },
                        "redacted": true
                        }))
                    }
                    "clients.list" => self.dispatch_clients_list(),
                    "clients.authorize" => self.dispatch_client_authorize(request.params),
                    "clients.revoke" => self.dispatch_client_revoke(request.params),
                    "operations.get" => self.dispatch_operation_get(request.params),
                    "operations.cancel" => self.dispatch_operation_cancel(request.params),
                    "recordings.list" => self.dispatch_recordings_list(request.params),
                    "recorders.list" => self.dispatch_recorders_list(request.params),
                    "recorders.create" => self.dispatch_recorder_create(request.params),
                    "recorders.arm" | "recorders.start" | "recorders.pause"
                    | "recorders.resume" | "recorders.split" | "recorders.stop" => {
                        self.dispatch_recorder(request.method.as_str(), request.params)
                    }
                    "recordings.get" => self.dispatch_recordings_get(request.params),
                    "recordings.recovery" => self.dispatch_recording_recovery(request.params),
                    "recordings.reveal" => self.dispatch_recording_reveal(request.params),
                    "recordings.preview" => self.dispatch_recordings_preview(request.params),
                    "recordings.setMetadata" => self.dispatch_recording_metadata(request.params),
                    "recordings.rename" => self.dispatch_recording_rename(request.params),
                    "recordings.removeEntry" => self.dispatch_recording_remove(request.params),
                    "recordings.recycle" => self.dispatch_recording_recycle(request.params),
                    "safety.setPrivacyMute" => self.dispatch_privacy_mute(request.params),
                    "recovery.clearSafeMode" => self.dispatch_recovery_clear(request.params),
                    "startup.get" => Ok(json!({
                        "enabled": false,
                        "registration": "unavailable",
                        "reason": "sign-in startup registration is not implemented in this build"
                    })),
                    "startup.plan" => self.dispatch_startup_plan(request.params),
                    "startup.apply" => self.dispatch_startup_apply(request.params),
                    "devices.list" => self.dispatch_devices_list(request.params),
                    "plugins.scan" => self.dispatch_plugins_scan(request.params),
                    "plugins.list" => self.dispatch_plugins_list(request.params),
                    "plugins.retry" => self.dispatch_plugins_retry(request.params),
                    "plugins.inspect" => self.dispatch_plugins_inspect(request.params),
                    "virtualDevices.list" => self.dispatch_virtual_devices_list(request.params),
                    "virtualDevices.plan" => self.dispatch_virtual_devices_plan(request.params),
                    "virtualDevices.apply" => self.dispatch_virtual_devices_apply(request.params),
                    "apps.list" | "applications.list" => self.dispatch_apps_list(),
                    "nodes.types" => Ok(self.describe()["nodeTypes"].clone()),
                    "nodes.describe" => Ok(self.describe()["nodeTypes"].clone()),
                    "presets.list" => Ok(self.describe()["presets"].clone()),
                    "processors.list" => Ok(self.describe()["processors"].clone()),
                    "processors.response" => self.dispatch_processors_response(request.params),
                    "sessions.get" => self.dispatch_session_get(request.params),
                    "sessions.export" => self.dispatch_session_export(request.params),
                    "sessions.importPlan" => self.dispatch_session_import_plan(request.params),
                    "sessions.importCommit" => self.dispatch_session_import_commit(request.params),
                    "sessions.list" => self.dispatch_sessions_list(request.params),
                    "sessions.create" => self.dispatch_session_create(request.params),
                    "sessions.duplicate" => self.dispatch_session_duplicate(request.params),
                    "sessions.delete" => self.dispatch_session_delete(request.params),
                    "routes.inspect" => self.dispatch_routes_inspect(request.params),
                    "graph.history" => self.dispatch_graph_history(request.params),
                    "graph.undoPlan" => self.dispatch_graph_undo_plan(request.params),
                    "events.subscribe" => self.dispatch_events_subscribe(request.params),
                    "session.start" | "sessions.start" => {
                        self.dispatch_session_start(request.params)
                    }
                    "session.stop" | "sessions.stop" => self.dispatch_session_stop(request.params),
                    "graph.plan" => self.dispatch_plan(request.params),
                    "graph.commit" => self.dispatch_commit(request.params),
                    _ => Err(ControlError::InvalidRequest("method not found".into())),
                }
            });
        match result {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err(ControlError::InvalidRequest(message)) if message == "method not found" => {
                JsonRpcResponse::failure(id, -32601, message)
            }
            Err(ControlError::InvalidRequest(message)) => {
                JsonRpcResponse::failure(id, -32602, message)
            }
            Err(error) => application_error_response(id, error),
        }
    }

    pub fn dispatch_authorized(
        &mut self,
        request: JsonRpcRequest,
        grant: &ClientGrant,
    ) -> JsonRpcResponse {
        self.dispatch_authorized_with_client(request, grant, None)
    }

    /// Dispatch an authenticated request with a stable client identity for
    /// mutation rate limiting. The identity is intentionally supplied by the
    /// transport after it has authenticated the caller.
    pub fn dispatch_authorized_for_client(
        &mut self,
        request: JsonRpcRequest,
        client_id: &str,
        grant: &ClientGrant,
    ) -> JsonRpcResponse {
        self.dispatch_authorized_with_client(request, grant, Some(client_id))
    }

    fn dispatch_authorized_with_client(
        &mut self,
        request: JsonRpcRequest,
        grant: &ClientGrant,
        client_id: Option<&str>,
    ) -> JsonRpcResponse {
        if request.validate().is_err() {
            return self.dispatch(request);
        }
        let id = request.id.clone();
        let Some(spec) = API_METHODS.iter().find(|spec| spec.name == request.method) else {
            return self.dispatch(request);
        };
        if !grant.allows(spec.permission) {
            let mut response = JsonRpcResponse::failure(
                id,
                -32001,
                format!("permission denied: {:?}", spec.permission),
            );
            if let Some(error) = response.error.as_mut() {
                error.data = Some(application_error_data("permissionDenied"));
            }
            return response;
        }
        if is_mutating_method(&request.method) {
            if let Some(client_id) = client_id {
                if let Err(retry_after_ms) = self.mutation_limiter.allow(client_id) {
                    let mut response = JsonRpcResponse::failure(id, -32000, "rate limited");
                    if let Some(error) = response.error.as_mut() {
                        error.data = Some(json!({
                            "code": "rateLimited",
                            "fieldPath": Value::Null,
                            "resourceIds": [],
                            "retryable": true,
                            "remediation": "wait for the retry hint before sending another mutation",
                            "retryAfterMs": retry_after_ms
                        }));
                    }
                    return response;
                }
            }
        }
        let previous_scope = std::mem::replace(
            &mut self.active_idempotency_scope,
            client_id
                .filter(|client| !client.is_empty())
                .map(str::to_owned),
        );
        let response = self.dispatch(request);
        self.active_idempotency_scope = previous_scope;
        response
    }

    pub fn dispatch_message(&mut self, message: RpcMessage) -> Vec<JsonRpcResponse> {
        match message {
            RpcMessage::Single(request) => {
                let omit = request.is_notification() && !is_mutating_method(&request.method);
                let response = self.dispatch(request);
                if omit {
                    Vec::new()
                } else {
                    vec![response]
                }
            }
            RpcMessage::Batch(requests) => requests
                .into_iter()
                .filter_map(|request| {
                    let omit = request.is_notification() && !is_mutating_method(&request.method);
                    let response = self.dispatch(request);
                    if omit {
                        None
                    } else {
                        Some(response)
                    }
                })
                .collect(),
        }
    }

    /// Dispatch a parsed message through the caller's explicit permission grant.
    /// Authorization runs before method parameters are interpreted or state is
    /// mutated, including for batched messages and notifications.
    pub fn dispatch_message_authorized(
        &mut self,
        message: RpcMessage,
        grant: &ClientGrant,
    ) -> Vec<JsonRpcResponse> {
        self.dispatch_message_authorized_with_client(message, grant, None)
    }

    pub fn dispatch_message_authorized_for_client(
        &mut self,
        message: RpcMessage,
        client_id: &str,
        grant: &ClientGrant,
    ) -> Vec<JsonRpcResponse> {
        self.dispatch_message_authorized_with_client(message, grant, Some(client_id))
    }

    fn dispatch_message_authorized_with_client(
        &mut self,
        message: RpcMessage,
        grant: &ClientGrant,
        client_id: Option<&str>,
    ) -> Vec<JsonRpcResponse> {
        match message {
            RpcMessage::Single(request) => {
                let omit = request.is_notification()
                    && !matches!(
                        request.method.as_str(),
                        "graph.plan"
                            | "graph.undoPlan"
                            | "graph.commit"
                            | "session.start"
                            | "sessions.start"
                            | "session.stop"
                            | "sessions.stop"
                    );
                let response = self.dispatch_authorized_with_client(request, grant, client_id);
                if omit {
                    Vec::new()
                } else {
                    vec![response]
                }
            }
            RpcMessage::Batch(requests) => requests
                .into_iter()
                .filter_map(|request| {
                    let omit = request.is_notification()
                        && !matches!(
                            request.method.as_str(),
                            "graph.plan"
                                | "graph.undoPlan"
                                | "graph.commit"
                                | "session.start"
                                | "sessions.start"
                                | "session.stop"
                                | "sessions.stop"
                        );
                    let response = self.dispatch_authorized_with_client(request, grant, client_id);
                    if omit {
                        None
                    } else {
                        Some(response)
                    }
                })
                .collect(),
        }
    }

    pub fn dispatch_frame(&mut self, frame: &[u8]) -> Result<Vec<Vec<u8>>, FrameError> {
        let message = decode_rpc_frame(frame)?;
        self.dispatch_message(message)
            .into_iter()
            .map(|response| encode_frame(&response))
            .collect()
    }

    pub fn dispatch_frame_authorized(
        &mut self,
        frame: &[u8],
        grant: &ClientGrant,
    ) -> Result<Vec<Vec<u8>>, FrameError> {
        let message = decode_rpc_frame(frame)?;
        self.dispatch_message_authorized(message, grant)
            .into_iter()
            .map(|response| encode_frame(&response))
            .collect()
    }

    pub fn dispatch_frame_authorized_for_client(
        &mut self,
        frame: &[u8],
        client_id: &str,
        grant: &ClientGrant,
    ) -> Result<Vec<Vec<u8>>, FrameError> {
        let message = decode_rpc_frame(frame)?;
        self.dispatch_message_authorized_for_client(message, client_id, grant)
            .into_iter()
            .map(|response| encode_frame(&response))
            .collect()
    }

    fn dispatch_plan(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params
            .ok_or_else(|| ControlError::InvalidRequest("graph.plan params are required".into()))?;
        let session_id: EntityId = serde_json::from_value(
            params
                .get("sessionId")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?,
        )
        .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))?;
        let base_revision = params
            .get("baseRevision")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("baseRevision is required".into()))?;
        let candidate: Session = serde_json::from_value(
            params
                .get("candidate")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("candidate is required".into()))?,
        )
        .map_err(|error| ControlError::InvalidRequest(error.to_string()))?;
        self.ensure_session_loaded(&session_id)?;
        let existing = self.get_session(&session_id)?.clone();
        let diff = graph_diff(&existing, &candidate);
        let affected_destinations = candidate
            .nodes
            .iter()
            .filter(|node| node.kind == audiorouter_domain::NodeKind::PhysicalOutput)
            .map(|node| node.name.clone())
            .collect::<Vec<_>>();
        let plan_id = self.plan_graph(&session_id, base_revision, candidate)?;
        Ok(json!({
            "planId": plan_id,
            "baseRevision": base_revision,
            "expiresInMs": 300000,
            "diff": diff,
            "affectedDestinations": affected_destinations,
            "warnings": [],
            "requiredScopes": ["graph.write"]
        }))
    }

    fn dispatch_handshake(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params
            .ok_or_else(|| ControlError::InvalidRequest("protocolVersion is required".into()))?;
        let version = params
            .get("protocolVersion")
            .and_then(Value::as_object)
            .ok_or_else(|| ControlError::InvalidRequest("protocolVersion is required".into()))?;
        let major = version
            .get("major")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("protocol major is required".into()))?;
        let minor = version
            .get("minor")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("protocol minor is required".into()))?;
        if major != 1 {
            return Err(ControlError::InvalidRequest(format!(
                "unsupported protocol major: {major}"
            )));
        }
        Ok(json!({
            "compatible": true,
            "requested": { "major": major, "minor": minor },
            "negotiated": { "major": 1, "minor": 0 },
            "schemaVersion": 1
        }))
    }

    fn dispatch_commit(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("graph.commit params are required".into())
        })?;
        if let Some(acknowledgments) = params.get("acknowledgments") {
            let acknowledgments = acknowledgments.as_array().ok_or_else(|| {
                ControlError::InvalidRequest("acknowledgments must be an array or null".into())
            })?;
            if acknowledgments.iter().any(|value| match value.as_str() {
                Some(value) => value.is_empty() || value.len() > 128,
                None => true,
            }) {
                return Err(ControlError::InvalidRequest(
                    "acknowledgments must contain non-empty warning IDs".into(),
                ));
            }
            if !acknowledgments.is_empty() {
                return Err(ControlError::InvalidRequest(
                    "no warnings on this plan require acknowledgment".into(),
                ));
            }
        }
        let plan_id: EntityId = serde_json::from_value(
            params
                .get("planId")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("planId is required".into()))?,
        )
        .map_err(|_| ControlError::InvalidRequest("invalid planId".into()))?;
        let base_revision = params
            .get("baseRevision")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("baseRevision is required".into()))?;
        let key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let scoped_key = self.scoped_idempotency_key("graph.commit", key);
        self.commit_graph_scoped(&plan_id, base_revision, &scoped_key, key)
    }

    fn dispatch_session_start(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and idempotencyKey are required".into())
        })?;
        let id = session_id_from_params(Some(params.clone()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = (
            self.scoped_idempotency_key("sessions.start", idempotency_key),
            Self::request_hash(&json!({ "sessionId": id, "action": "start" })),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        let result = self.session_start(&id)?;
        self.journal_idempotent_result(&operation.0, "sessions.start", &operation.1, &result)?;
        Ok(result)
    }

    fn dispatch_session_get(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let id = session_id_from_params(params)?;
        self.ensure_session_loaded(&id)?;
        serde_json::to_value(self.get_session(&id)?)
            .map_err(|error| ControlError::Json(error.to_string()))
    }

    fn dispatch_session_export(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let id = session_id_from_params(params)?;
        self.ensure_session_loaded(&id)?;
        serde_json::to_value(self.get_session(&id)?)
            .map_err(|error| ControlError::Json(error.to_string()))
    }

    fn dispatch_session_import_plan(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("session is required".into()))?;
        let session: Session = serde_json::from_value(
            params
                .get("session")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("session is required".into()))?,
        )
        .map_err(|error| ControlError::InvalidRequest(error.to_string()))?;
        validate_session(&session).map_err(|errors| {
            ControlError::InvalidRequest(format!(
                "invalid session import: {}",
                format_validation_errors(&errors)
            ))
        })?;
        if self.store.session(&session.id).is_some() {
            return Err(ControlError::InvalidRequest(
                "session already exists".into(),
            ));
        }
        let now = Instant::now();
        self.session_import_plans
            .retain(|_, (_, expires_at)| *expires_at > now);
        if self.session_import_plans.len() >= MAX_PENDING_PLAN_RECORDS {
            return Err(ControlError::InvalidRequest(
                "too many pending session import plans".into(),
            ));
        }
        let plan_id = allocate_counter_plan_id(
            "session-import",
            &mut self.next_session_import_plan,
            |id| self.session_import_plans.contains_key(id),
            "session import plan ID space is exhausted",
        )?;
        self.session_import_plans.insert(
            plan_id.clone(),
            (session.clone(), Instant::now() + VIRTUAL_DEVICE_PLAN_TTL),
        );
        Ok(json!({ "planId": plan_id, "expiresInMs": 300000, "session": session }))
    }

    fn dispatch_session_import_commit(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("planId and idempotencyKey are required".into())
        })?;
        let plan_id = params
            .get("planId")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("planId is required".into()))?;
        let key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|key| !key.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let plan_entity_id = EntityId::new(plan_id);
        let scoped_key = self.scoped_idempotency_key("sessions.importCommit", key);
        let hash = Self::request_hash(&json!({ "planId": plan_id }));
        if let Some(result) = self.lookup_idempotent_result(&scoped_key, &hash)? {
            return Ok(result);
        }
        let Some((session, expires_at)) = self.session_import_plans.get(&plan_entity_id).cloned()
        else {
            return Err(ControlError::InvalidRequest("import plan not found".into()));
        };
        if expires_at <= Instant::now() {
            self.session_import_plans.remove(&plan_entity_id);
            return Err(ControlError::InvalidRequest("import plan expired".into()));
        }
        if self.store.session(&session.id).is_some() {
            return Err(ControlError::InvalidRequest(
                "session already exists".into(),
            ));
        }
        self.insert_session(session.clone())?;
        self.session_import_plans.remove(&plan_entity_id);
        let result = json!({ "session": session, "state": "stopped", "imported": true });
        self.journal_idempotent_result(&scoped_key, "sessions.importCommit", &hash, &result)?;
        Ok(result)
    }

    fn dispatch_session_create(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("session is required".into()))?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let session: Session = serde_json::from_value(
            params
                .get("session")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("session is required".into()))?,
        )
        .map_err(|error| ControlError::InvalidRequest(error.to_string()))?;
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({ "session": session });
                (
                    self.scoped_idempotency_key("sessions.create", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let result = self.create_session(session)?;
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "sessions.create", &hash, &result)?;
        }
        Ok(result)
    }

    fn dispatch_session_duplicate(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("duplicate parameters are required".into())
        })?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let source_id: EntityId =
            serde_json::from_value(params.get("sourceSessionId").cloned().ok_or_else(|| {
                ControlError::InvalidRequest("sourceSessionId is required".into())
            })?)
            .map_err(|_| ControlError::InvalidRequest("invalid sourceSessionId".into()))?;
        let duplicate_id: EntityId = serde_json::from_value(
            params
                .get("sessionId")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?,
        )
        .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))?;
        let name = params
            .get("name")
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| ControlError::InvalidRequest("name must be a string".into()))
            })
            .transpose()?;
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({
                    "sourceSessionId": source_id,
                    "sessionId": duplicate_id,
                    "name": name
                });
                (
                    self.scoped_idempotency_key("sessions.duplicate", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let result = self.duplicate_session(&source_id, duplicate_id, name)?;
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "sessions.duplicate", &hash, &result)?;
        }
        Ok(result)
    }

    fn dispatch_session_delete(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and idempotencyKey are required".into())
        })?;
        let id = session_id_from_params(Some(params.clone()))?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({ "sessionId": id });
                (
                    self.scoped_idempotency_key("sessions.delete", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let result = self.delete_session(&id)?;
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "sessions.delete", &hash, &result)?;
        }
        Ok(result)
    }

    fn dispatch_sessions_list(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let cursor = params
            .as_ref()
            .and_then(|value| value.get("cursor"))
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| ControlError::InvalidRequest("cursor must be a string".into()))
            })
            .transpose()?;
        let limit = params
            .as_ref()
            .and_then(|value| value.get("limit"))
            .and_then(Value::as_u64)
            .unwrap_or(100);
        self.sessions_list_page(cursor, limit as usize)
    }

    fn dispatch_session_stop(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and idempotencyKey are required".into())
        })?;
        let id = session_id_from_params(Some(params.clone()))?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({ "sessionId": id, "action": "stop" });
                (
                    self.scoped_idempotency_key("sessions.stop", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let result = self.session_stop(&id)?;
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "sessions.stop", &hash, &result)?;
        }
        Ok(result)
    }

    fn dispatch_routes_inspect(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("routes.inspect params are required".into())
        })?;
        let session_id: EntityId = serde_json::from_value(
            params
                .get("sessionId")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?,
        )
        .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))?;
        let destination_node: EntityId =
            serde_json::from_value(params.get("destinationNode").cloned().ok_or_else(|| {
                ControlError::InvalidRequest("destinationNode is required".into())
            })?)
            .map_err(|_| ControlError::InvalidRequest("invalid destinationNode".into()))?;
        self.inspect_routes(&session_id, &destination_node)
    }

    fn dispatch_graph_history(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("graph.history params are required".into())
        })?;
        let session_id: EntityId = serde_json::from_value(
            params
                .get("sessionId")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?,
        )
        .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))?;
        let limit = params.get("limit").and_then(Value::as_u64).unwrap_or(100);
        if limit == 0 || limit > 100 {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 100".into(),
            ));
        }
        let before_revision = params
            .get("cursor")
            .filter(|value| !value.is_null())
            .map(|value| {
                let cursor = value.as_str().ok_or_else(|| {
                    ControlError::InvalidRequest("cursor must be a string".into())
                })?;
                if cursor.len() > MAX_REVISION_CURSOR_BYTES {
                    return Err(ControlError::InvalidRequest(
                        "history cursor exceeds the maximum length".into(),
                    ));
                }
                cursor
                    .parse::<u64>()
                    .map_err(|_| ControlError::InvalidRequest("invalid history cursor".into()))
            })
            .transpose()?;
        self.graph_history_page(&session_id, before_revision, limit as usize)
    }

    fn dispatch_graph_undo_plan(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("graph.undoPlan params are required".into())
        })?;
        let session_id: EntityId = serde_json::from_value(
            params
                .get("sessionId")
                .cloned()
                .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?,
        )
        .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))?;
        let base_revision = params
            .get("baseRevision")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("baseRevision is required".into()))?;
        let plan_id = self.graph_undo_plan(&session_id, base_revision)?;
        Ok(json!({ "planId": plan_id, "baseRevision": base_revision, "expiresInMs": 300000 }))
    }

    fn dispatch_recorder_create(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("recorder creation parameters are required".into())
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let recorder_id = params
            .get("recorderId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("recorderId is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let request_hash = Self::request_hash(&params);
        let scoped_key = self.scoped_idempotency_key("recorders.create", idempotency_key);
        if let Some(result) = self.lookup_idempotent_result(&scoped_key, &request_hash)? {
            return Ok(result);
        }
        let sequence = params
            .get("sequence")
            .and_then(Value::as_u64)
            .ok_or_else(|| ControlError::InvalidRequest("sequence is required".into()))?;
        let channels = params
            .get("channels")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| ControlError::InvalidRequest("channels is required".into()))?;
        let sample_rate = params
            .get("sampleRate")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| ControlError::InvalidRequest("sampleRate is required".into()))?;
        let queue_capacity = params
            .get("queueCapacity")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| ControlError::InvalidRequest("queueCapacity is required".into()))?;
        let maximum_chunks_per_pass = params
            .get("maximumChunksPerPass")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| {
                ControlError::InvalidRequest("maximumChunksPerPass is required".into())
            })?;
        let dither = params
            .get("dither")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let format_name = params
            .get("format")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("format is required".into()))?;
        let format = match format_name {
            "wavPcm16" => FileRecorderFormat::Wav(WavFormat::Pcm16),
            "wavPcm24" => FileRecorderFormat::Wav(WavFormat::Pcm24),
            "wavFloat32" => FileRecorderFormat::Wav(WavFormat::Float32),
            "flac16" => FileRecorderFormat::Flac {
                bits_per_sample: 16,
            },
            "flac24" => FileRecorderFormat::Flac {
                bits_per_sample: 24,
            },
            _ => {
                return Err(ControlError::InvalidRequest(
                    "unsupported recorder format".into(),
                ))
            }
        };
        let session = EntityId::new(session_id);
        if self.store.session(&session).is_none() {
            return Err(ControlError::InvalidRequest("session not found".into()));
        }
        let config = FileRecorderConfig {
            version: FILE_RECORDER_CONFIG_VERSION,
            session_id,
            recorder_id,
            sequence,
            format,
            channels,
            sample_rate,
            dither,
            queue_capacity,
            maximum_chunks_per_pass,
        };
        let path = self.create_and_attach_configured_file_recorder(session, &config)?;
        let result = json!({
            "sessionId": session_id,
            "recorderId": recorder_id,
            "format": format_name,
            "path": path,
            "state": "idle",
            "armed": false,
        });
        self.journal_idempotent_result(&scoped_key, "recorders.create", &request_hash, &result)?;
        Ok(result)
    }

    fn dispatch_recorder(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("sessionId and idempotencyKey are required".into())
        })?;
        let session_id = params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
        let session_id = EntityId::new(session_id);
        if self.store.session(&session_id).is_none() {
            return Err(ControlError::InvalidRequest("session not found".into()));
        }
        let frame = params.get("frame").and_then(Value::as_u64);
        let idempotency = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|key| !key.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let request_hash = Self::request_hash(&json!({
            "method": method,
            "sessionId": session_id,
            "frame": frame,
        }));
        let scoped_key = self.scoped_idempotency_key(method, &idempotency);
        if let Some(result) = self.lookup_idempotent_result(&scoped_key, &request_hash)? {
            return Ok(result);
        }
        let restored = if self.recorders.contains_key(&session_id) {
            None
        } else {
            self.storage
                .as_ref()
                .and_then(|storage| {
                    storage
                        .load_recording_checkpoint(session_id.as_str())
                        .transpose()
                })
                .transpose()
                .map_err(storage_error)?
        };
        if let Some(checkpoint) = restored {
            let recorder = RecorderController::restore(checkpoint).map_err(|error| {
                ControlError::InvalidRequest(format!("recorder checkpoint is invalid: {error:?}"))
            })?;
            self.recorders.insert(session_id.clone(), recorder);
        }
        if method == "recorders.arm"
            && !self
                .recorders
                .get(&session_id)
                .is_some_and(|recorder| recorder_is_active(recorder.state()))
            && self
                .recorders
                .values()
                .filter(|recorder| recorder_is_active(recorder.state()))
                .count()
                >= MAX_ACTIVE_RECORDERS
        {
            return Err(ControlError::InvalidRequest(
                "active recorder limit reached".into(),
            ));
        }
        let mut worker_finalized = false;
        let mut finalized_recordings = Vec::new();
        if let Some(worker) = self.recorder_workers.get_mut(&session_id) {
            let worker_result = match method {
                "recorders.arm" => worker.arm(),
                "recorders.start" => worker.start(
                    frame
                        .ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
                ),
                "recorders.pause" => worker.pause(
                    frame
                        .ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
                ),
                "recorders.resume" => worker.resume(
                    frame
                        .ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
                ),
                "recorders.split" => worker.split(
                    frame
                        .ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
                ),
                "recorders.stop" => {
                    let frame = frame
                        .ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?;
                    let outcome = worker.finalize(frame).map_err(|error| {
                        ControlError::InvalidRequest(format!(
                            "recorder finalization failed: {error}"
                        ))
                    })?;
                    if outcome.state != "completed"
                        || !outcome.file_finalized
                        || outcome.recoverable
                    {
                        return Err(ControlError::InvalidRequest(
                            "recorder finalization did not produce a completed file".into(),
                        ));
                    }
                    worker_finalized = true;
                    finalized_recordings = worker.finalized_recordings();
                    Ok(())
                }
                _ => Err("method not found".into()),
            };
            worker_result.map_err(|error| {
                ControlError::InvalidRequest(format!("recorder worker transition failed: {error}"))
            })?;
        }
        let recorder = self.recorders.entry(session_id.clone()).or_default();
        let result = match method {
            "recorders.arm" => recorder.arm(),
            "recorders.start" => recorder.start(
                frame.ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
            ),
            "recorders.pause" => recorder.pause(
                frame.ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
            ),
            "recorders.resume" => recorder.resume(
                frame.ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
            ),
            "recorders.split" => recorder.split(
                frame.ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
            ),
            "recorders.stop" => recorder.stop(
                frame.ok_or_else(|| ControlError::InvalidRequest("frame is required".into()))?,
            ),
            _ => return Err(ControlError::InvalidRequest("method not found".into())),
        };
        result.map_err(|error| {
            ControlError::InvalidRequest(format!("recorder transition failed: {error:?}"))
        })?;
        let checkpoint = recorder.checkpoint();
        let result = json!({
            "sessionId": session_id,
            "state": recorder_state_name(recorder.state()),
            "parts": checkpoint.parts.iter().map(|part| json!({
                "index": part.index,
                "startFrame": part.start_frame,
                "endFrame": part.end_frame,
            })).collect::<Vec<_>>(),
            "pauses": checkpoint.pauses.iter().map(|pause| json!({
                "startFrame": pause.start_frame,
                "endFrame": pause.end_frame,
            })).collect::<Vec<_>>(),
            "lastFrame": checkpoint.last_frame,
        });
        if let Some(storage) = &self.storage {
            storage
                .save_recording_checkpoint(session_id.as_str(), &checkpoint)
                .map_err(storage_error)?;
            for recording in &finalized_recordings {
                storage.save_recording(recording).map_err(storage_error)?;
            }
        }
        self.journal_idempotent_result(&scoped_key, method, &request_hash, &result)?;
        let revision = self
            .store
            .session(&session_id)
            .map(|session| session.revision)
            .unwrap_or_default();
        self.events
            .append(revision, None, "recorder.changed", Some(session_id.clone()));
        if worker_finalized {
            self.recorder_workers.remove(&session_id);
        }
        Ok(result)
    }

    fn dispatch_recordings_list(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let session_id = params.get("sessionId").and_then(Value::as_str);
        let paged = params.get("cursor").is_some() || params.get("limit").is_some();
        let cursor = params
            .get("cursor")
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| ControlError::InvalidRequest("cursor must be a string".into()))
            })
            .transpose()?;
        let limit = params.get("limit").and_then(Value::as_u64).unwrap_or(100);
        if !(1..=MAX_RECORDING_LIST_ITEMS as u64).contains(&limit) {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 500".into(),
            ));
        }
        let Some(storage) = &self.storage else {
            return Ok(if paged {
                json!({ "items": [], "nextCursor": null })
            } else {
                json!([])
            });
        };
        let (records, has_more) = storage
            .list_recordings_page(
                session_id,
                cursor,
                if paged {
                    limit as usize
                } else {
                    MAX_RECORDING_LIST_ITEMS
                },
            )
            .map_err(storage_error)?;
        if !paged && has_more {
            return Err(ControlError::InvalidRequest(
                "recordings.list requires cursor pagination when more than 500 records exist"
                    .into(),
            ));
        }
        let values = records
            .into_iter()
            .map(|record| {
                json!({
                    "id": record.id,
                    "sessionId": record.session_id,
                    "recorderId": record.recorder_id,
                    "path": record.path,
                    "format": record.format,
                    "channels": record.channels,
                    "sampleRate": record.sample_rate,
                    "frames": record.frames,
                    "fileBytes": record.file_bytes,
                    "startTime": record.start_time,
                    "state": record.state,
                    "missing": record.missing,
                    "title": record.title,
                    "artist": record.artist,
                    "comment": record.comment
                })
            })
            .collect::<Vec<_>>();
        if paged {
            let next_cursor = has_more
                .then(|| values.last().and_then(|value| value["id"].as_str()))
                .flatten();
            Ok(json!({ "items": values, "nextCursor": next_cursor }))
        } else {
            Ok(json!(values))
        }
    }

    fn dispatch_recorders_list(&self, params: Option<Value>) -> Result<Value, ControlError> {
        if let Some(params) = params {
            if !params.is_object() || !params.as_object().is_some_and(|object| object.is_empty()) {
                return Err(ControlError::InvalidRequest(
                    "recorders.list does not accept parameters".into(),
                ));
            }
        }
        let mut recorders = self
            .recorders
            .iter()
            .map(|(session_id, recorder)| {
                let checkpoint = recorder.checkpoint();
                json!({
                    "sessionId": session_id,
                    "state": recorder_state_name(recorder.state()),
                    "lastFrame": checkpoint.last_frame
                })
            })
            .collect::<Vec<_>>();
        recorders
            .sort_by(|left, right| left["sessionId"].as_str().cmp(&right["sessionId"].as_str()));
        Ok(Value::Array(recorders))
    }

    fn dispatch_recordings_get(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let recording_id = params
            .as_ref()
            .and_then(|params| {
                params
                    .get("recordingId")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let Some(storage) = &self.storage else {
            return Err(ControlError::InvalidRequest("recording not found".into()));
        };
        let record = storage
            .get_recording(&recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        Ok(json!({
            "id": record.id,
            "sessionId": record.session_id,
            "recorderId": record.recorder_id,
            "path": record.path,
            "format": record.format,
            "channels": record.channels,
            "sampleRate": record.sample_rate,
            "frames": record.frames,
            "fileBytes": record.file_bytes,
            "startTime": record.start_time,
            "state": record.state,
            "missing": record.missing,
            "title": record.title,
            "artist": record.artist,
            "comment": record.comment
        }))
    }

    fn dispatch_recording_recovery(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let storage = self.storage.as_ref().ok_or_else(|| {
            ControlError::InvalidRequest("recording recovery is unavailable".into())
        })?;
        let params = params.unwrap_or_else(|| json!({}));
        let recording_id_supplied = params.get("recordingId").is_some();
        let recording_id = params
            .get("recordingId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned);
        if recording_id_supplied && recording_id.is_none() {
            return Err(ControlError::InvalidRequest(
                "recordingId must be a non-empty string".into(),
            ));
        }
        if recording_id.is_none() {
            let cursor = params
                .get("cursor")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty());
            let limit = params
                .get("limit")
                .and_then(Value::as_u64)
                .map(|value| usize::try_from(value).unwrap_or(usize::MAX))
                .unwrap_or(MAX_RECORDING_LIST_ITEMS);
            let (ids, has_more) = storage
                .list_recording_checkpoint_ids(cursor, limit)
                .map_err(storage_error)?;
            let mut items = Vec::with_capacity(ids.len());
            for id in &ids {
                let item = match storage.load_recording_checkpoint(id) {
                    Ok(Some(checkpoint)) => json!({
                        "recordingId": id,
                        "status": "available",
                        "checkpoint": checkpoint
                    }),
                    Ok(None) => json!({
                        "recordingId": id,
                        "status": "missing"
                    }),
                    Err(StorageError::InvalidRecording(_)) => json!({
                        "recordingId": id,
                        "status": "invalid"
                    }),
                    Err(error) => return Err(storage_error(error)),
                };
                items.push(item);
            }
            let next_cursor = has_more.then(|| ids.last().cloned()).flatten();
            return Ok(json!({ "items": items, "nextCursor": next_cursor }));
        }
        let recording_id = recording_id.expect("recording ID checked above");
        let Some(checkpoint) = storage
            .load_recording_checkpoint(&recording_id)
            .map_err(storage_error)?
        else {
            return Ok(json!({
                "recordingId": recording_id,
                "status": "missing"
            }));
        };
        Ok(json!({
            "recordingId": recording_id,
            "status": "available",
            "checkpoint": checkpoint
        }))
    }

    fn dispatch_recording_reveal(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let recording_id = params
            .and_then(|params| {
                params
                    .get("recordingId")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let record = storage
            .get_recording(&recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let path = std::path::Path::new(&record.path);
        if !path.is_absolute() {
            return Err(ControlError::InvalidRequest(
                "recording path must be absolute".into(),
            ));
        }
        if !path.is_file() {
            return Ok(
                json!({ "recordingId": recording_id, "path": record.path, "revealed": false, "reason": "missing" }),
            );
        }
        audiorouter_storage::validate_recording_file_path(path).map_err(storage_error)?;
        #[cfg(windows)]
        let revealed = std::process::Command::new("explorer.exe")
            .args(["/select,", &record.path])
            .spawn()
            .map(|_| true)
            .map_err(|error| {
                ControlError::InvalidRequest(format!("unable to reveal recording: {error}"))
            })?;
        #[cfg(not(windows))]
        let revealed = false;
        Ok(json!({ "recordingId": recording_id, "path": record.path, "revealed": revealed }))
    }

    fn dispatch_recordings_preview(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let recording_id = params
            .and_then(|params| {
                params
                    .get("recordingId")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let record = storage
            .get_recording(&recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let path = std::path::Path::new(&record.path);
        if !path.is_absolute() {
            return Err(ControlError::InvalidRequest(
                "recording path must be absolute".into(),
            ));
        }
        if path.is_file() {
            audiorouter_storage::validate_recording_file_path(path).map_err(storage_error)?;
        }
        let status = audiorouter_recording::inspect_recording(&record.path).map_err(|error| {
            ControlError::InvalidRequest(format!("recording preview failed: {error:?}"))
        })?;
        let result = match status {
            audiorouter_recording::RecordingFileStatus::Present(info) => json!({
                "status": "present",
                "format": "wav",
                "channels": info.channels,
                "sampleRate": info.sample_rate,
                "frames": info.frames,
                "dataBytes": info.data_bytes,
                "fileBytes": info.file_bytes
            }),
            audiorouter_recording::RecordingFileStatus::FlacPresent(info) => json!({
                "status": "present",
                "format": "flac",
                "channels": info.channels,
                "sampleRate": info.sample_rate,
                "bitsPerSample": info.bits_per_sample,
                "frames": info.frames,
                "fileBytes": info.file_bytes
            }),
            audiorouter_recording::RecordingFileStatus::Missing => json!({ "status": "missing" }),
            audiorouter_recording::RecordingFileStatus::Invalid => json!({ "status": "invalid" }),
        };
        Ok(json!({ "recordingId": recording_id, "preview": result }))
    }

    fn dispatch_recording_metadata(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let recording_id = params
            .get("recordingId")
            .and_then(Value::as_str)
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({
                    "recordingId": recording_id,
                    "title": params.get("title").and_then(Value::as_str),
                    "artist": params.get("artist").and_then(Value::as_str),
                    "comment": params.get("comment").and_then(Value::as_str)
                });
                (
                    self.scoped_idempotency_key("recordings.setMetadata", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let session_id = storage
            .get_recording(recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?
            .session_id;
        let updated = storage
            .update_recording_metadata(
                recording_id,
                params.get("title").and_then(Value::as_str),
                params.get("artist").and_then(Value::as_str),
                params.get("comment").and_then(Value::as_str),
            )
            .map_err(storage_error)?;
        if !updated {
            return Err(ControlError::InvalidRequest("recording not found".into()));
        }
        let result = json!({ "recordingId": recording_id, "updated": true });
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "recordings.setMetadata", &hash, &result)?;
        }
        self.events.append(
            0,
            None,
            "recording.metadataChanged",
            Some(EntityId::new(session_id)),
        );
        Ok(result)
    }

    fn dispatch_recording_rename(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("recordingId and newPath are required".into())
        })?;
        let recording_id = params
            .get("recordingId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let new_path = params
            .get("newPath")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("newPath is required".into()))?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({ "recordingId": recording_id, "newPath": new_path });
                (
                    self.scoped_idempotency_key("recordings.rename", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let session_id = storage
            .get_recording(recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?
            .session_id;
        if !storage
            .rename_recording(recording_id, new_path)
            .map_err(storage_error)?
        {
            return Err(ControlError::InvalidRequest("recording not found".into()));
        }
        let result = json!({
            "recordingId": recording_id,
            "renamed": true,
            "path": new_path,
            "fileAction": "renamed"
        });
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "recordings.rename", &hash, &result)?;
        }
        self.events.append(
            0,
            None,
            "recording.renamed",
            Some(EntityId::new(session_id)),
        );
        Ok(result)
    }

    fn dispatch_recording_remove(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("recordingId and idempotencyKey are required".into())
        })?;
        let recording_id = params
            .get("recordingId")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({ "recordingId": recording_id });
                (
                    self.scoped_idempotency_key("recordings.removeEntry", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let session_id = storage
            .get_recording(&recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?
            .session_id;
        if !storage
            .remove_recording_entry(&recording_id)
            .map_err(storage_error)?
        {
            return Err(ControlError::InvalidRequest("recording not found".into()));
        }
        let result = json!({ "recordingId": recording_id, "removed": true, "fileAction": "none" });
        if let Some((key, hash)) = operation {
            self.journal_idempotent_result(&key, "recordings.removeEntry", &hash, &result)?;
        }
        self.events.append(
            0,
            None,
            "recording.entryRemoved",
            Some(EntityId::new(session_id)),
        );
        Ok(result)
    }

    fn dispatch_recording_recycle(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("recordingId and confirm are required".into())
        })?;
        let recording_id = params
            .get("recordingId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("recordingId is required".into()))?;
        let confirm = params
            .get("confirm")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if confirm {
            params
                .get("idempotencyKey")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    ControlError::InvalidRequest(
                        "idempotencyKey is required for confirmed recycle".into(),
                    )
                })?;
        }
        let storage = self
            .storage
            .as_ref()
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let record = storage
            .get_recording(recording_id)
            .map_err(storage_error)?
            .ok_or_else(|| ControlError::InvalidRequest("recording not found".into()))?;
        let path = std::path::Path::new(&record.path);
        if !path.is_absolute() {
            return Err(ControlError::InvalidRequest(
                "recording path must be absolute".into(),
            ));
        }
        if !path.is_file() {
            return Ok(
                json!({ "recordingId": recording_id, "path": record.path, "fileAction": "none", "reason": "missing" }),
            );
        }
        audiorouter_storage::validate_recording_file_path(path).map_err(storage_error)?;
        if !confirm {
            return Ok(
                json!({ "recordingId": recording_id, "path": record.path, "fileAction": "recycle", "preview": true }),
            );
        }
        let operation = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(|key| {
                let request = json!({ "recordingId": recording_id, "confirm": true });
                (
                    self.scoped_idempotency_key("recordings.recycle", key),
                    Self::request_hash(&request),
                )
            });
        if let Some((key, hash)) = &operation {
            if let Some(previous) = self.lookup_idempotent_result(key, hash)? {
                return Ok(previous);
            }
        }
        #[cfg(windows)]
        {
            trash::delete(path).map_err(|error| {
                ControlError::InvalidRequest(format!("recycleUnavailable: {error}"))
            })?;
            storage
                .set_recording_missing(recording_id, true)
                .map_err(storage_error)?;
            let result = json!({ "recordingId": recording_id, "path": record.path, "fileAction": "recycled", "missing": true });
            if let Some((key, hash)) = operation {
                self.journal_idempotent_result(&key, "recordings.recycle", &hash, &result)?;
            }
            self.events.append(
                0,
                None,
                "recording.recycled",
                Some(EntityId::new(record.session_id.clone())),
            );
            Ok(result)
        }
        #[cfg(not(windows))]
        {
            let _ = path;
            Ok(
                json!({ "recordingId": recording_id, "path": record.path, "fileAction": "none", "reason": "recycleUnavailable" }),
            )
        }
    }

    fn dispatch_privacy_mute(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("muted and idempotencyKey are required".into())
        })?;
        let muted = params
            .get("muted")
            .and_then(Value::as_bool)
            .ok_or_else(|| ControlError::InvalidRequest("muted is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = (
            self.scoped_idempotency_key("safety.setPrivacyMute", idempotency_key),
            Self::request_hash(&json!({ "muted": muted })),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        if let Some(storage) = &self.storage {
            storage.save_privacy_mute(muted).map_err(storage_error)?;
        }
        self.privacy_muted = muted;
        self.events.append(
            0,
            None,
            if muted {
                "privacy.muteEnabled"
            } else {
                "privacy.muteDisabled"
            },
            None,
        );
        let result = json!({
            "muted": muted,
            "persistence": if self.storage.is_some() { "durable" } else { "memory" },
            "audioEffect": "process-local-when-realtime-backend-is-available"
        });
        self.journal_idempotent_result(
            &operation.0,
            "safety.setPrivacyMute",
            &operation.1,
            &result,
        )?;
        Ok(result)
    }

    fn dispatch_recovery_clear(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let operation = (
            self.scoped_idempotency_key("recovery.clearSafeMode", idempotency_key),
            Self::request_hash(&json!({ "action": "clear" })),
        );
        if let Some(previous) = self.lookup_idempotent_result(&operation.0, &operation.1)? {
            return Ok(previous);
        }
        if let Some(storage) = &self.storage {
            storage.clear_recovery_crashes().map_err(storage_error)?;
        }
        self.recovery_tracker.clear_after_stable_run();
        self.events
            .append(0, None, "recovery.safeModeCleared", None);
        let result = json!({
            "safeMode": false,
            "recentCrashes": 0,
            "persistence": if self.storage.is_some() { "durable" } else { "memory" }
        });
        self.journal_idempotent_result(
            &operation.0,
            "recovery.clearSafeMode",
            &operation.1,
            &result,
        )?;
        Ok(result)
    }

    fn dispatch_events_subscribe(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        if let Some(requested_epoch) = params.get("backendEpoch").and_then(Value::as_u64) {
            if requested_epoch != self.events.backend_epoch() {
                return Ok(json!({
                    "backendEpoch": self.events.backend_epoch(),
                    "resyncRequired": true,
                    "reason": "backendEpochChanged",
                    "snapshot": { "sessions": self.sessions_list_page(None, 500)? },
                    "events": [],
                    "nextSequence": self.events.latest_sequence()
                }));
            }
        }
        let after_sequence = params
            .get("afterSequence")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let limit = params.get("limit").and_then(Value::as_u64).unwrap_or(100);
        if !(1..=MAX_EVENT_SUBSCRIPTION_ITEMS as u64).contains(&limit) {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 500".into(),
            ));
        }
        let session_filter = params
            .get("sessionId")
            .map(|value| {
                serde_json::from_value::<EntityId>(value.clone())
                    .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))
            })
            .transpose()?;
        let category_filter = params
            .get("categories")
            .map(|value| {
                let categories = value.as_array().ok_or_else(|| {
                    ControlError::InvalidRequest("categories must be an array".into())
                })?;
                if categories.is_empty() || categories.len() > 32 {
                    return Err(ControlError::InvalidRequest(
                        "categories must contain between 1 and 32 items".into(),
                    ));
                }
                categories
                    .iter()
                    .map(|category| {
                        let category = category.as_str().ok_or_else(|| {
                            ControlError::InvalidRequest("event categories must be strings".into())
                        })?;
                        if category.is_empty() || category.chars().count() > 128 {
                            return Err(ControlError::InvalidRequest(
                                "event category must contain 1 to 128 characters".into(),
                            ));
                        }
                        Ok(category.to_owned())
                    })
                    .collect::<Result<Vec<_>, ControlError>>()
            })
            .transpose()?;
        let (events, next_sequence) = match self.events.since_page(after_sequence, limit as usize) {
            Ok(page) => page,
            Err(EventReplayError::InvalidLimit) => {
                return Err(ControlError::InvalidRequest(
                    "limit must be between 1 and 500".into(),
                ));
            }
            Err(EventReplayError::ResyncRequired) => {
                return Ok(json!({
                    "backendEpoch": self.events.backend_epoch(),
                    "resyncRequired": true,
                    "snapshot": { "sessions": self.sessions_list_page(None, 500)? },
                    "events": [],
                    "nextSequence": self.events.latest_sequence()
                }));
            }
        };
        let events = events
            .into_iter()
            .filter(|event| {
                let session_matches = session_filter
                    .as_ref()
                    .map(|id| event.session_id.is_none() || event.session_id.as_ref() == Some(id))
                    .unwrap_or(true);
                let category_matches = category_filter
                    .as_ref()
                    .map(|categories| {
                        categories
                            .iter()
                            .any(|category| category == &event.category)
                    })
                    .unwrap_or(true);
                session_matches && category_matches
            })
            .collect::<Vec<_>>();
        Ok(json!({
            "backendEpoch": self.events.backend_epoch(),
            "events": events,
            "nextSequence": next_sequence,
        }))
    }

    fn dispatch_devices_list(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let paged = params.get("cursor").is_some() || params.get("limit").is_some();
        let cursor = params
            .get("cursor")
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| ControlError::InvalidRequest("cursor must be a string".into()))
            })
            .transpose()?;
        let limit = params.get("limit").and_then(Value::as_u64).unwrap_or(100);
        let include_inactive = params
            .get("includeInactive")
            .map(|value| {
                value.as_bool().ok_or_else(|| {
                    ControlError::InvalidRequest("includeInactive must be a boolean".into())
                })
            })
            .transpose()?
            .unwrap_or(false);
        if !(1..=MAX_DEVICE_LIST_ITEMS as u64).contains(&limit) {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 500".into(),
            ));
        }
        if self.endpoint_monitor.is_none() {
            self.endpoint_monitor = Some(
                audiorouter_windows_audio::EndpointMonitor::start().map_err(audio_control_error)?,
            );
        }
        let (endpoint_changes, endpoints) = {
            let monitor = self
                .endpoint_monitor
                .as_mut()
                .expect("endpoint monitor initialized above");
            let endpoint_changes = monitor.poll_changes().map_err(audio_control_error)?;
            let endpoints = monitor.snapshot().to_vec();
            (endpoint_changes, endpoints)
        };
        self.record_endpoint_changes(!endpoint_changes.is_empty());
        let defaults = audiorouter_windows_audio::enumerate_default_endpoint_bindings()
            .map_err(audio_control_error)?;
        let display_info = audiorouter_windows_audio::enumerate_active_endpoint_display_info()
            .map_err(audio_control_error)?;
        let endpoint_states = include_inactive
            .then(audiorouter_windows_audio::enumerate_endpoint_states)
            .transpose()
            .map_err(audio_control_error)?
            .unwrap_or_default();
        let mut devices = endpoints
            .into_iter()
            .map(|endpoint| {
                let bytes_per_frame = endpoint.bytes_per_frame().map_err(audio_control_error)?;
                let default_roles = defaults
                    .iter()
                    .filter(|binding| {
                        binding.endpoint_id == endpoint.id
                            && binding.direction == endpoint.direction
                    })
                    .map(|binding| binding.role.as_str())
                    .collect::<Vec<_>>();
                let name = display_info
                    .iter()
                    .find(|info| info.id == endpoint.id && info.direction == endpoint.direction)
                    .map(|info| info.name.as_str())
                    .unwrap_or("Unknown audio endpoint");
                Ok(json!({
                    "id": endpoint.id,
                    "name": name,
                    "direction": match endpoint.direction {
                        audiorouter_windows_audio::EndpointDirection::Capture => "capture",
                        audiorouter_windows_audio::EndpointDirection::Render => "render",
                    },
                    "state": "active",
                    "defaultRoles": default_roles,
                    "format": {
                        "sampleRateHz": endpoint.sample_rate_hz,
                        "channels": endpoint.channels,
                        "bitsPerSample": endpoint.bits_per_sample,
                        "formatTag": endpoint.format_tag,
                        "bytesPerFrame": bytes_per_frame,
                    },
                    "periods": {
                        "default100ns": endpoint.default_period_100ns,
                        "minimum100ns": endpoint.minimum_period_100ns,
                    },
                }))
            })
            .collect::<Result<Vec<_>, ControlError>>()?;
        if include_inactive {
            for endpoint in endpoint_states {
                if matches!(
                    endpoint.state,
                    audiorouter_windows_audio::EndpointState::Active
                ) || devices.iter().any(|device| {
                    device["id"] == endpoint.id
                        && device["direction"]
                            == match endpoint.direction {
                                audiorouter_windows_audio::EndpointDirection::Capture => "capture",
                                audiorouter_windows_audio::EndpointDirection::Render => "render",
                            }
                }) {
                    continue;
                }
                let direction = match endpoint.direction {
                    audiorouter_windows_audio::EndpointDirection::Capture => "capture",
                    audiorouter_windows_audio::EndpointDirection::Render => "render",
                };
                let default_roles = defaults
                    .iter()
                    .filter(|binding| {
                        binding.endpoint_id == endpoint.id
                            && binding.direction == endpoint.direction
                    })
                    .map(|binding| binding.role.as_str())
                    .collect::<Vec<_>>();
                let name = display_info
                    .iter()
                    .find(|info| info.id == endpoint.id && info.direction == endpoint.direction)
                    .map(|info| info.name.as_str())
                    .unwrap_or("Unknown audio endpoint");
                let state = match endpoint.state {
                    audiorouter_windows_audio::EndpointState::Disabled => "disabled",
                    audiorouter_windows_audio::EndpointState::Unplugged => "unplugged",
                    audiorouter_windows_audio::EndpointState::NotPresent => "notPresent",
                    audiorouter_windows_audio::EndpointState::Unknown(_) => "unknown",
                    audiorouter_windows_audio::EndpointState::Active => unreachable!(),
                };
                devices.push(json!({
                    "id": endpoint.id,
                    "name": name,
                    "direction": direction,
                    "state": state,
                    "defaultRoles": default_roles,
                }));
            }
        }
        devices.sort_by(|left, right| left["id"].as_str().cmp(&right["id"].as_str()));
        if !paged && devices.len() > MAX_DEVICE_LIST_ITEMS {
            return Err(ControlError::InvalidRequest(
                "devices.list requires cursor pagination when more than 500 endpoints exist".into(),
            ));
        }
        if let Some(cursor) = cursor {
            let Some(index) = devices.iter().position(|device| device["id"] == cursor) else {
                return Err(ControlError::InvalidRequest("invalid device cursor".into()));
            };
            devices.drain(..=index);
        }
        if !paged {
            return Ok(json!(devices));
        }
        let has_more = devices.len() > limit as usize;
        devices.truncate(limit as usize);
        let next_cursor = has_more
            .then(|| devices.last().and_then(|device| device["id"].as_str()))
            .flatten();
        Ok(json!({ "items": devices, "nextCursor": next_cursor }))
    }

    fn record_endpoint_changes(&mut self, changed: bool) {
        if changed {
            // EventLog is the bounded notification surface. Endpoint details
            // are intentionally refetched through devices.list so events do
            // not duplicate an unbounded or stale device payload.
            self.events.append(0, None, "devices.changed", None);
        }
    }

    fn dispatch_plugins_scan(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let directory = params
            .as_ref()
            .and_then(|value| value.get("directory"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("directory is required".into()))?;
        let root = std::path::Path::new(directory);
        if !root.is_absolute() {
            return Err(ControlError::InvalidRequest(
                "directory path must be absolute".into(),
            ));
        }
        let entries =
            audiorouter_plugin_host::scan_directory(root).map_err(ControlError::PluginScan)?;
        let result = json!({
            "directory": directory,
            "entries": entries.into_iter().map(|entry| {
                let identity = entry.identity.map(|identity| json!({
                    "path": identity.path,
                    "binaryPath": identity.binary_path,
                    "format": match identity.format {
                        audiorouter_plugin_host::PluginFormat::Vst3 => "vst3",
                        audiorouter_plugin_host::PluginFormat::Vst2 => "vst2",
                        audiorouter_plugin_host::PluginFormat::Unknown => "unknown",
                    },
                    "architecture": match identity.architecture {
                        audiorouter_plugin_host::PeArchitecture::X64 => "x64",
                        audiorouter_plugin_host::PeArchitecture::X86 => "x86",
                        audiorouter_plugin_host::PeArchitecture::Arm64 => "arm64",
                        audiorouter_plugin_host::PeArchitecture::Unknown => "unknown",
                    },
                    "fileBytes": identity.file_bytes,
                    "sha256": identity.sha256,
                    "vendor": identity.metadata.vendor,
                    "version": identity.metadata.version,
                    "classIds": identity.metadata.class_ids,
                    "compatibility": match identity.compatibility() {
                        audiorouter_plugin_host::PluginCompatibility::SupportedVst3X64 => "supportedVst3X64",
                        audiorouter_plugin_host::PluginCompatibility::SupportedVst2X64Gated => "supportedVst2X64Gated",
                        audiorouter_plugin_host::PluginCompatibility::UnsupportedFormat => "unsupportedFormat",
                    }
                }));
                json!({
                    "path": entry.path,
                    "identity": identity,
                    "error": entry.error.as_ref().map(|error| format!("{error:?}")),
                    "errorCode": entry.error.as_ref().map(|error| error.code())
                })
            }).collect::<Vec<_>>()
        });
        self.remember_plugin_inventory(directory.to_owned(), result.clone());
        Ok(result)
    }

    fn dispatch_plugins_list(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let directory = params
            .as_ref()
            .and_then(|value| value.get("directory"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("directory is required".into()))?;
        if !std::path::Path::new(directory).is_absolute() {
            return Err(ControlError::InvalidRequest(
                "directory path must be absolute".into(),
            ));
        }
        Ok(self
            .plugin_inventories
            .get(directory)
            .cloned()
            .unwrap_or_else(|| json!({ "directory": directory, "entries": [] })))
    }

    fn dispatch_plugins_retry(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let directory = params
            .get("directory")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("directory is required".into()))?;
        let key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let scoped_key = self.scoped_idempotency_key("plugins.retry", key);
        let hash = Self::request_hash(&json!({ "directory": directory }));
        if let Some(result) = self.lookup_idempotent_result(&scoped_key, &hash)? {
            return Ok(result);
        }
        let result = self.dispatch_plugins_scan(Some(json!({ "directory": directory })))?;
        self.journal_idempotent_result(&scoped_key, "plugins.retry", &hash, &result)?;
        Ok(result)
    }

    fn dispatch_plugins_inspect(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let path = params
            .as_ref()
            .and_then(|value| value.get("path"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("path is required".into()))?;
        let candidate = std::path::Path::new(path);
        if !candidate.is_absolute() {
            return Err(ControlError::InvalidRequest("path must be absolute".into()));
        }
        let root = candidate.parent().ok_or_else(|| {
            ControlError::InvalidRequest("path must have a parent directory".into())
        })?;
        let result = match audiorouter_plugin_host::inspect_binary(candidate, &[root.to_path_buf()])
        {
            Ok(identity) => json!({
                "path": path,
                "identity": {
                    "path": identity.path,
                    "binaryPath": identity.binary_path,
                    "format": match identity.format {
                        audiorouter_plugin_host::PluginFormat::Vst3 => "vst3",
                        audiorouter_plugin_host::PluginFormat::Vst2 => "vst2",
                        audiorouter_plugin_host::PluginFormat::Unknown => "unknown",
                    },
                    "architecture": match identity.architecture {
                        audiorouter_plugin_host::PeArchitecture::X64 => "x64",
                        audiorouter_plugin_host::PeArchitecture::X86 => "x86",
                        audiorouter_plugin_host::PeArchitecture::Arm64 => "arm64",
                        audiorouter_plugin_host::PeArchitecture::Unknown => "unknown",
                    },
                    "fileBytes": identity.file_bytes,
                    "sha256": identity.sha256,
                    "vendor": identity.metadata.vendor,
                    "version": identity.metadata.version,
                    "classIds": identity.metadata.class_ids,
                    "compatibility": match identity.compatibility() {
                        audiorouter_plugin_host::PluginCompatibility::SupportedVst3X64 => "supportedVst3X64",
                        audiorouter_plugin_host::PluginCompatibility::SupportedVst2X64Gated => "supportedVst2X64Gated",
                        audiorouter_plugin_host::PluginCompatibility::UnsupportedFormat => "unsupportedFormat",
                    }
                },
                "error": null,
                "errorCode": null
            }),
            Err(error) => json!({
                "path": path,
                "identity": null,
                "error": format!("{error:?}"),
                "errorCode": error.code()
            }),
        };
        Ok(result)
    }

    fn dispatch_startup_plan(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let enabled = params
            .as_ref()
            .and_then(|value| value.get("enabled"))
            .and_then(Value::as_bool)
            .ok_or_else(|| ControlError::InvalidRequest("enabled is required".into()))?;
        let now = Instant::now();
        self.startup_plans
            .retain(|_, (_, expires_at)| *expires_at > now);
        if self.startup_plans.len() >= MAX_PENDING_PLAN_RECORDS {
            return Err(ControlError::InvalidRequest(
                "too many pending startup plans".into(),
            ));
        }
        let timestamp_millis = unix_epoch_millis();
        let plan_id = allocate_timestamped_plan_id(
            "startup-plan",
            timestamp_millis,
            &mut self.next_startup_plan,
            |id| self.startup_plans.contains_key(id),
            "startup plan ID space is exhausted",
        )?;
        self.startup_plans.insert(
            plan_id.clone(),
            (enabled, Instant::now() + VIRTUAL_DEVICE_PLAN_TTL),
        );
        if let Some(storage) = &self.storage {
            let expires_at = unix_epoch_seconds() + VIRTUAL_DEVICE_PLAN_TTL.as_secs() as i64;
            if let Err(error) = storage.save_startup_plan(&plan_id, enabled, expires_at) {
                self.startup_plans.remove(&plan_id);
                return Err(storage_error(error));
            }
        }
        Ok(json!({
            "planId": plan_id,
            "enabled": enabled,
            "registration": "unavailable",
            "reason": "sign-in startup registration is not implemented in this build",
            "requiredScopes": ["sessionControl"],
            "warnings": ["planning does not change Windows startup registration"]
        }))
    }

    fn dispatch_startup_apply(&mut self, params: Option<Value>) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("planId is required".into()))?;
        let plan_id = params
            .get("planId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("planId is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let plan_id_value = EntityId::new(plan_id);
        let Some((enabled, expires_at)) = self.startup_plans.get(&plan_id_value).copied() else {
            return Err(ControlError::InvalidRequest(
                "startup plan was not found".into(),
            ));
        };
        if Instant::now() >= expires_at {
            self.startup_plans.remove(&plan_id_value);
            return Err(ControlError::InvalidRequest(
                "startup plan has expired".into(),
            ));
        }
        let scoped_key = self.scoped_idempotency_key("startup.apply", idempotency_key);
        let request_hash = Self::request_hash(&json!({ "planId": plan_id, "enabled": enabled }));
        if let Some(previous) = self.operation_outcomes.get(&scoped_key) {
            if self
                .idempotency_hashes
                .get(&scoped_key)
                .is_some_and(|hash| hash == &request_hash)
            {
                return Ok(previous.clone());
            }
            return Err(ControlError::IdempotencyConflict);
        }
        let result = json!({
            "planId": plan_id,
            "state": "unavailable",
            "registration": "unavailable",
            "reason": "sign-in startup registration is not implemented in this build"
        });
        self.journal_idempotent_result(&scoped_key, "startup.apply", &request_hash, &result)?;
        Ok(result)
    }

    fn dispatch_virtual_devices_plan(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("operation is required".into()))?;
        let operation = virtual_bus_operation_from_value(
            params
                .get("operation")
                .ok_or_else(|| ControlError::InvalidRequest("operation is required".into()))?,
        )?;
        let mut candidate = self.virtual_buses.clone();
        apply_virtual_bus_operation(&mut candidate, &operation)?;
        let now = Instant::now();
        self.virtual_bus_plans
            .retain(|_, plan| plan.expires_at > now);
        if self.virtual_bus_plans.len() >= MAX_PENDING_PLAN_RECORDS {
            return Err(ControlError::InvalidRequest(
                "too many pending virtual-device plans".into(),
            ));
        }
        let timestamp_millis = unix_epoch_millis();
        let plan_id = allocate_timestamped_plan_id(
            "virtual-plan",
            timestamp_millis,
            &mut self.next_virtual_bus_plan,
            |id| self.virtual_bus_plans.contains_key(id),
            "virtual-device plan ID space is exhausted",
        )?;
        let expires_at = unix_epoch_seconds() + VIRTUAL_DEVICE_PLAN_TTL.as_secs() as i64;
        self.virtual_bus_plans.insert(
            plan_id.clone(),
            VirtualBusPlan {
                operation: operation.clone(),
                expires_at: Instant::now() + VIRTUAL_DEVICE_PLAN_TTL,
            },
        );
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_device_plan(
                &plan_id,
                &virtual_bus_operation_value(&operation),
                expires_at,
            ) {
                self.virtual_bus_plans.remove(&plan_id);
                return Err(storage_error(error));
            }
        }
        Ok(json!({
            "planId": plan_id,
            "expiresInMs": VIRTUAL_DEVICE_PLAN_TTL.as_millis(),
            "operation": virtual_bus_operation_value(&operation),
            "availability": {
                "status": "unavailable",
                "reason": "requires M03 managed virtual driver"
            },
            "requiredScopes": ["deviceAdministration"],
            "warnings": ["desired state can be stored, but Windows endpoints remain unavailable until the managed driver is installed"]
        }))
    }

    fn dispatch_virtual_devices_apply(
        &mut self,
        params: Option<Value>,
    ) -> Result<Value, ControlError> {
        let params =
            params.ok_or_else(|| ControlError::InvalidRequest("planId is required".into()))?;
        let plan_id = params
            .get("planId")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("planId is required".into()))?;
        let idempotency_key = params
            .get("idempotencyKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ControlError::InvalidRequest("idempotencyKey is required".into()))?;
        let storage_key = self.scoped_idempotency_key("virtualDevices.apply", idempotency_key);
        let request_hash = virtual_device_request_hash(plan_id);
        if let Some(previous) = self.operation_outcomes.get(&storage_key) {
            if self
                .idempotency_hashes
                .get(&storage_key)
                .is_some_and(|hash| hash == &request_hash)
            {
                return Ok(previous.clone());
            }
            return Err(ControlError::IdempotencyConflict);
        }
        if let Some(storage) = &self.storage {
            if let Some(previous) = storage
                .journal_result_checked(&storage_key, &request_hash)
                .map_err(storage_error)?
            {
                let previous: Value = serde_json::from_str(&previous)
                    .map_err(|error| ControlError::Json(error.to_string()))?;
                self.remember_operation_outcome(
                    &storage_key,
                    previous.clone(),
                    "virtualDevices.apply",
                    Some(&request_hash),
                );
                return Ok(previous);
            }
        }
        let plan = self
            .virtual_bus_plans
            .get(&EntityId::new(plan_id))
            .cloned()
            .ok_or_else(|| ControlError::InvalidRequest("virtual device plan not found".into()))?;
        if plan.expires_at <= Instant::now() {
            self.virtual_bus_plans.remove(&EntityId::new(plan_id));
            return Err(ControlError::InvalidRequest(
                "virtual device plan expired".into(),
            ));
        }
        let checkpoint = self.virtual_buses.clone();
        apply_virtual_bus_operation(&mut self.virtual_buses, &plan.operation)?;
        if let Err(error) = self.sync_virtual_bridge_operation(&plan.operation) {
            self.virtual_buses = checkpoint;
            return Err(error);
        }
        let result = json!({
            "planId": plan_id,
            "state": "applied",
            "availability": {
                "status": "unavailable",
                "reason": "requires M03 managed virtual driver"
            },
            "operation": virtual_bus_operation_value(&plan.operation)
        });
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save_virtual_buses_and_journal(
                &self.virtual_buses,
                &EntityId::new(plan_id),
                &storage_key,
                &request_hash,
                &result,
            ) {
                self.virtual_buses = checkpoint;
                let _ = self.rollback_virtual_bridge_operation(&plan.operation);
                return Err(storage_error(error));
            }
        }
        if let VirtualBusOperation::SetEnabled { id, enabled: false } = &plan.operation {
            if let Some(bridge) = self.virtual_bridges.get(id) {
                bridge.deactivate();
            }
        }
        self.virtual_bus_plans.remove(&EntityId::new(plan_id));
        self.remember_operation_outcome(
            &storage_key,
            result.clone(),
            "virtualDevices.apply",
            Some(&request_hash),
        );
        self.events
            .append(0, Some(plan_id.to_owned()), "virtualDevice.changed", None);
        Ok(result)
    }

    fn dispatch_virtual_devices_list(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.unwrap_or_else(|| json!({}));
        let paged = params.get("cursor").is_some() || params.get("limit").is_some();
        let cursor = params
            .get("cursor")
            .filter(|value| !value.is_null())
            .map(|value| {
                value
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| ControlError::InvalidRequest("cursor must be a string".into()))
            })
            .transpose()?;
        let limit = params.get("limit").and_then(Value::as_u64).unwrap_or(100);
        if !(1..=MAX_VIRTUAL_DEVICE_LIST_ITEMS as u64).contains(&limit) {
            return Err(ControlError::InvalidRequest(
                "limit must be between 1 and 500".into(),
            ));
        }
        let mut buses = self
            .virtual_buses
            .list()
            .iter()
            .map(|bus| {
                json!({
                    "id": bus.id(),
                    "name": bus.name(),
                    "direction": "bidirectional",
                    "channels": bus.channels(),
                    "enabled": bus.enabled(),
                    "availability": {
                        "status": "unavailable",
                        "reason": "requires M03 managed virtual driver"
                    },
                    "endpointIds": { "render": null, "capture": null },
                    "capabilities": { "render": false, "capture": false, "channels": 2 },
                    "privilege": "deviceAdministration",
                    "restartRequired": false,
                    "clientImpacts": [],
                    "leaseOwner": bus.lease().owner()
                })
            })
            .collect::<Vec<_>>();
        if let Some(cursor) = cursor {
            let Some(index) = buses.iter().position(|bus| bus["id"] == cursor) else {
                return Err(ControlError::InvalidRequest(
                    "invalid virtual device cursor".into(),
                ));
            };
            buses.drain(..=index);
        }
        if !paged {
            return Ok(json!(buses));
        }
        let has_more = buses.len() > limit as usize;
        buses.truncate(limit as usize);
        let next_cursor = has_more
            .then(|| buses.last().and_then(|bus| bus["id"].as_str()))
            .flatten();
        Ok(json!({ "items": buses, "nextCursor": next_cursor }))
    }

    fn dispatch_apps_list(&mut self) -> Result<Value, ControlError> {
        if let Some((captured_at, snapshot)) = &self.application_snapshot {
            if captured_at.elapsed() < APPLICATION_SNAPSHOT_TTL {
                return Ok(snapshot.clone());
            }
        }
        let applications =
            audiorouter_windows_audio::enumerate_applications().map_err(audio_control_error)?;
        let audio = audiorouter_windows_audio::enumerate_application_audio()
            .map_err(audio_control_error)?;
        let snapshot = json!(applications
            .into_iter()
            .map(|application| {
                let session = audio.iter().find(|item| item.process_id == application.process_id);
                json!({
                    "processId": application.process_id,
                    "executable": application.executable,
                    "executablePath": application.executable_path,
                    "creationTime100ns": application.creation_time_100ns.map(|value| value.to_string()),
                    "audioActivity": session.map_or("none", |item| if item.active_session_count > 0 { "active" } else { "inactive" }),
                    "captureCapability": session.map_or("notObserved", |item| if item.capture_session_count > 0 { "observed" } else { "notObserved" }),
                    "audioSessionCount": session.map_or(0, |item| item.total_session_count),
                    "activeAudioSessionCount": session.map_or(0, |item| item.active_session_count),
                    "captureSessionCount": session.map_or(0, |item| item.capture_session_count),
                    "renderSessionCount": session.map_or(0, |item| item.render_session_count),
                    "audioDisplayNames": session.map_or_else(Vec::new, |item| item.display_names.clone()),
                })
            })
            .collect::<Vec<_>>());
        self.application_snapshot = Some((Instant::now(), snapshot.clone()));
        Ok(snapshot)
    }

    fn dispatch_processors_response(&self, params: Option<Value>) -> Result<Value, ControlError> {
        let params = params.ok_or_else(|| {
            ControlError::InvalidRequest("response parameters are required".into())
        })?;
        let sample_rate = params["sampleRateHz"]
            .as_f64()
            .ok_or_else(|| ControlError::InvalidRequest("sampleRateHz is required".into()))?
            as f32;
        let frequencies = params["frequenciesHz"]
            .as_array()
            .ok_or_else(|| ControlError::InvalidRequest("frequenciesHz is required".into()))?;
        if frequencies.is_empty() || frequencies.len() > MAX_RESPONSE_FREQUENCIES {
            return Err(ControlError::InvalidRequest(
                "frequenciesHz count is outside the bounded response limit".into(),
            ));
        }
        let mut bands = [None; 8];
        let band_values = params["bands"]
            .as_array()
            .ok_or_else(|| ControlError::InvalidRequest("bands is required".into()))?;
        if band_values.len() > MAX_RESPONSE_BANDS {
            return Err(ControlError::InvalidRequest(
                "too many response bands".into(),
            ));
        }
        for (index, band) in band_values.iter().enumerate() {
            if band.get("enabled").and_then(Value::as_bool) == Some(false) {
                continue;
            }
            let kind = match band["type"].as_str() {
                Some("peaking") => audiorouter_dsp::FilterKind::Peaking,
                Some("lowShelf") => audiorouter_dsp::FilterKind::LowShelf,
                Some("highShelf") => audiorouter_dsp::FilterKind::HighShelf,
                Some("lowPass") => audiorouter_dsp::FilterKind::LowPass,
                Some("highPass") => audiorouter_dsp::FilterKind::HighPass,
                Some("notch") => audiorouter_dsp::FilterKind::Notch,
                _ => {
                    return Err(ControlError::InvalidRequest(format!(
                        "invalid response band {index} type"
                    )))
                }
            };
            let number = |name: &str| {
                band[name]
                    .as_f64()
                    .map(|value| value as f32)
                    .ok_or_else(|| {
                        ControlError::InvalidRequest(format!(
                            "response band {index} {name} is required"
                        ))
                    })
            };
            bands[index] = Some(audiorouter_dsp::BiquadParams {
                kind,
                frequency_hz: number("frequencyHz")?,
                q: number("q")?,
                gain_db: number("gainDb")?,
                sample_rate,
            });
        }
        let eq = audiorouter_dsp::ParametricEq::new(bands, 1).map_err(|error| {
            ControlError::InvalidRequest(format!("invalid response EQ: {error:?}"))
        })?;
        let frequencies = frequencies
            .iter()
            .map(|value| {
                value.as_f64().map(|value| value as f32).ok_or_else(|| {
                    ControlError::InvalidRequest("frequenciesHz must contain numbers".into())
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let magnitude_db = frequencies
            .iter()
            .map(|frequency| {
                eq.magnitude_db_at(*frequency).map_err(|error| {
                    ControlError::InvalidRequest(format!("invalid response frequency: {error:?}"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(json!({ "frequenciesHz": frequencies, "magnitudeDb": magnitude_db }))
    }
}

fn session_id_from_params(params: Option<Value>) -> Result<EntityId, ControlError> {
    let params =
        params.ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?;
    serde_json::from_value(
        params
            .get("sessionId")
            .cloned()
            .ok_or_else(|| ControlError::InvalidRequest("sessionId is required".into()))?,
    )
    .map_err(|_| ControlError::InvalidRequest("invalid sessionId".into()))
}

fn virtual_bus_operation_from_value(value: &Value) -> Result<VirtualBusOperation, ControlError> {
    let action = value
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| ControlError::InvalidRequest("operation.action is required".into()))?;
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(EntityId::new)
        .ok_or_else(|| ControlError::InvalidRequest("operation.id is required".into()))?;
    match action {
        "create" => Ok(VirtualBusOperation::Create {
            id,
            name: value
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| ControlError::InvalidRequest("operation.name is required".into()))?
                .to_owned(),
        }),
        "rename" => Ok(VirtualBusOperation::Rename {
            id,
            name: value
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| ControlError::InvalidRequest("operation.name is required".into()))?
                .to_owned(),
        }),
        "setEnabled" => Ok(VirtualBusOperation::SetEnabled {
            id,
            enabled: value
                .get("enabled")
                .and_then(Value::as_bool)
                .ok_or_else(|| {
                    ControlError::InvalidRequest("operation.enabled is required".into())
                })?,
        }),
        "delete" => Ok(VirtualBusOperation::Delete { id }),
        _ => Err(ControlError::InvalidRequest(
            "operation.action must be create, rename, setEnabled, or delete".into(),
        )),
    }
}

fn apply_virtual_bus_operation(
    registry: &mut VirtualBusRegistry,
    operation: &VirtualBusOperation,
) -> Result<(), ControlError> {
    let result = match operation {
        VirtualBusOperation::Create { id, name } => registry.create(id.clone(), name),
        VirtualBusOperation::Rename { id, name } => registry.rename(id, name),
        VirtualBusOperation::SetEnabled { id, enabled } => registry.set_enabled(id, *enabled),
        VirtualBusOperation::Delete { id } => registry.delete(id),
    };
    result.map_err(virtual_bus_control_error)
}

fn virtual_bus_operation_value(operation: &VirtualBusOperation) -> Value {
    match operation {
        VirtualBusOperation::Create { id, name } => {
            json!({ "action": "create", "id": id, "name": name })
        }
        VirtualBusOperation::Rename { id, name } => {
            json!({ "action": "rename", "id": id, "name": name })
        }
        VirtualBusOperation::SetEnabled { id, enabled } => {
            json!({ "action": "setEnabled", "id": id, "enabled": enabled })
        }
        VirtualBusOperation::Delete { id } => json!({ "action": "delete", "id": id }),
    }
}

fn graph_diff(before: &Session, after: &Session) -> Vec<Value> {
    let mut diff = Vec::new();
    if before.name != after.name {
        diff.push(json!({
            "path": "/name",
            "before": &before.name,
            "after": &after.name,
        }));
    }
    if before.nodes != after.nodes {
        diff.push(json!({
            "path": "/nodes",
            "before": &before.nodes,
            "after": &after.nodes,
        }));
    }
    if before.edges != after.edges {
        diff.push(json!({
            "path": "/edges",
            "before": &before.edges,
            "after": &after.edges,
        }));
    }
    diff
}

fn validate_method_params(method: &str, params: Option<&Value>) -> Result<(), ControlError> {
    let Some(params) = params else {
        return Ok(());
    };
    let mut value_count = 0;
    validate_control_value_budget(params, 0, &mut value_count)?;
    let Some(object) = params.as_object() else {
        return Err(ControlError::InvalidRequest(
            "method params must be an object".into(),
        ));
    };
    let allowed: &[&str] = match method {
        "sessions.get" | "sessions.export" => &["sessionId"],
        "sessions.importPlan" => &["session"],
        "sessions.importCommit" => &["planId", "idempotencyKey"],
        "sessions.delete" => &["sessionId", "idempotencyKey"],
        "session.start" | "sessions.start" | "session.stop" | "sessions.stop" => {
            &["sessionId", "idempotencyKey"]
        }
        "sessions.list" => &["cursor", "limit"],
        "sessions.create" => &["session", "idempotencyKey"],
        "sessions.duplicate" => &["sourceSessionId", "sessionId", "name", "idempotencyKey"],
        "routes.inspect" => &["sessionId", "destinationNode"],
        "graph.history" => &["sessionId", "cursor", "limit"],
        "graph.undoPlan" => &["sessionId", "baseRevision"],
        "events.subscribe" => &[
            "afterSequence",
            "backendEpoch",
            "categories",
            "limit",
            "sessionId",
        ],
        "graph.plan" => &["sessionId", "baseRevision", "candidate"],
        "graph.commit" => &[
            "planId",
            "baseRevision",
            "idempotencyKey",
            "acknowledgments",
        ],
        "system.handshake" => &["protocolVersion"],
        "clients.authorize" => &["clientId", "role", "idempotencyKey"],
        "clients.revoke" => &["clientId", "idempotencyKey"],
        "operations.get" => &["operationId"],
        "operations.cancel" => &["operationId", "idempotencyKey"],
        "recordings.list" => &["sessionId", "cursor", "limit"],
        "recorders.list" => &[],
        "recorders.create" => &[
            "sessionId",
            "recorderId",
            "format",
            "sequence",
            "channels",
            "sampleRate",
            "dither",
            "queueCapacity",
            "maximumChunksPerPass",
            "idempotencyKey",
        ],
        "recorders.arm" => &["sessionId", "idempotencyKey"],
        "recorders.start" | "recorders.pause" | "recorders.resume" | "recorders.split"
        | "recorders.stop" => &["sessionId", "frame", "idempotencyKey"],
        "recordings.get" | "recordings.reveal" | "recordings.preview" => &["recordingId"],
        "recordings.recovery" => &["recordingId", "cursor", "limit"],
        "recordings.setMetadata" => &[
            "recordingId",
            "title",
            "artist",
            "comment",
            "idempotencyKey",
        ],
        "recordings.rename" => &["recordingId", "newPath", "idempotencyKey"],
        "safety.setPrivacyMute" => &["muted", "idempotencyKey"],
        "recovery.clearSafeMode" => &["idempotencyKey"],
        "recordings.removeEntry" => &["recordingId", "idempotencyKey"],
        "recordings.recycle" => &["recordingId", "confirm", "idempotencyKey"],
        "devices.list" => &["cursor", "limit"],
        "plugins.scan" => &["directory"],
        "plugins.list" => &["directory"],
        "plugins.retry" => &["directory", "idempotencyKey"],
        "plugins.inspect" => &["path"],
        "virtualDevices.list" => &["cursor", "limit"],
        "virtualDevices.plan" => &["operation"],
        "virtualDevices.apply" => &["planId", "idempotencyKey"],
        "startup.plan" => &["enabled"],
        "startup.apply" => &["planId", "idempotencyKey"],
        "system.describe" | "status.get" | "system.diagnostics" | "startup.get" | "apps.list"
        | "applications.list" | "nodes.types" | "nodes.describe" | "presets.list"
        | "processors.list" | "clients.list" => &[],
        "processors.response" => &["sampleRateHz", "bands", "frequenciesHz"],
        _ => return Ok(()),
    };
    if let Some(field) = object
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err(ControlError::InvalidRequest(format!(
            "unknown parameter: {field}"
        )));
    }
    Ok(())
}

fn validate_control_value_budget(
    value: &Value,
    depth: usize,
    value_count: &mut usize,
) -> Result<(), ControlError> {
    *value_count = value_count.checked_add(1).ok_or_else(|| {
        ControlError::InvalidRequest("method parameters exceed the value budget".into())
    })?;
    if *value_count > MAX_CONTROL_VALUE_COUNT {
        return Err(ControlError::InvalidRequest(
            "method parameters contain too many JSON values".into(),
        ));
    }
    if depth > MAX_CONTROL_VALUE_DEPTH {
        return Err(ControlError::InvalidRequest(
            "method parameters exceed the maximum JSON nesting depth".into(),
        ));
    }
    match value {
        Value::String(string) if string.len() > MAX_CONTROL_STRING_BYTES => {
            Err(ControlError::InvalidRequest(
                "method parameter string exceeds the maximum length".into(),
            ))
        }
        Value::Array(values) => values
            .iter()
            .try_for_each(|value| validate_control_value_budget(value, depth + 1, value_count)),
        Value::Object(values) => values.iter().try_for_each(|(key, value)| {
            if key.len() > MAX_CONTROL_STRING_BYTES {
                return Err(ControlError::InvalidRequest(
                    "method parameter key exceeds the maximum length".into(),
                ));
            }
            validate_control_value_budget(value, depth + 1, value_count)
        }),
        _ => Ok(()),
    }
}

fn role_name(role: ClientRole) -> &'static str {
    match role {
        ClientRole::Observer => "observer",
        ClientRole::Editor => "editor",
        ClientRole::Operator => "operator",
    }
}

fn role_from_name(name: &str) -> Option<ClientRole> {
    match name {
        "observer" => Some(ClientRole::Observer),
        "editor" => Some(ClientRole::Editor),
        "operator" => Some(ClientRole::Operator),
        _ => None,
    }
}

fn is_mutating_method(method: &str) -> bool {
    API_METHODS
        .iter()
        .find(|spec| spec.name == method)
        .is_some_and(|spec| spec.side_effect != audiorouter_domain::SideEffectClass::ReadOnly)
}

fn storage_error(error: StorageError) -> ControlError {
    match error {
        StorageError::CorruptDatabase(_) => {
            ControlError::CorruptDatabase("database integrity check failed".into())
        }
        StorageError::IdempotencyConflict => ControlError::IdempotencyConflict,
        StorageError::InvalidSession(message)
        | StorageError::InvalidBundle(message)
        | StorageError::InvalidRecording(message)
        | StorageError::InvalidPluginState(message)
        | StorageError::InvalidEnrollment(message)
        | StorageError::InvalidPlan(message)
        | StorageError::InvalidJournal(message)
        | StorageError::InvalidBackupPath(message) => ControlError::InvalidRequest(message),
        StorageError::JournalLimitReached => {
            ControlError::InvalidRequest("operation journal limit reached".into())
        }
        StorageError::DocumentTooLarge { maximum, .. } => ControlError::InvalidRequest(format!(
            "document exceeds the maximum permitted size of {maximum} bytes"
        )),
        StorageError::InvalidRecoveryTimestamp => {
            ControlError::InvalidRequest("invalid recovery timestamp".into())
        }
        StorageError::Io(_) => ControlError::Storage("storage I/O operation failed".into()),
        StorageError::Sql(_) => ControlError::Storage("database operation failed".into()),
        StorageError::Json(_) => ControlError::Storage("stored document is invalid".into()),
    }
}

fn recorder_state_name(state: audiorouter_recording::RecorderState) -> &'static str {
    match state {
        audiorouter_recording::RecorderState::Idle => "idle",
        audiorouter_recording::RecorderState::Armed => "armed",
        audiorouter_recording::RecorderState::Recording => "recording",
        audiorouter_recording::RecorderState::Paused => "paused",
        audiorouter_recording::RecorderState::Stopping => "stopping",
        audiorouter_recording::RecorderState::Completed => "completed",
        audiorouter_recording::RecorderState::Failed => "failed",
    }
}

fn recorder_is_active(state: RecorderState) -> bool {
    matches!(
        state,
        RecorderState::Armed
            | RecorderState::Recording
            | RecorderState::Paused
            | RecorderState::Stopping
    )
}

fn virtual_bus_control_error(error: audiorouter_domain::VirtualBusError) -> ControlError {
    ControlError::InvalidRequest(format!("virtual bus operation rejected: {error:?}"))
}

fn virtual_bridge_control_error(error: VirtualBusBridgeSetError) -> ControlError {
    let message = match error {
        VirtualBusBridgeSetError::InvalidCapacity => "invalid virtual bridge capacity",
        VirtualBusBridgeSetError::MissingBus => "virtual bridge is not registered",
        VirtualBusBridgeSetError::CapacityReached => "virtual bridge capacity reached",
        VirtualBusBridgeSetError::Queue(_) => "virtual bridge queue shape is invalid",
    };
    ControlError::InvalidRequest(message.into())
}

fn virtual_device_request_hash(plan_id: &str) -> String {
    let fingerprint = format!("virtualDevices.apply:{plan_id}");
    format!("{:x}", Sha256::digest(fingerprint.as_bytes()))
}

fn application_error_response(id: Option<Value>, error: ControlError) -> JsonRpcResponse {
    let code = match &error {
        ControlError::Store(error) => match error {
            audiorouter_domain::StoreError::SessionNotFound => "notFound",
            audiorouter_domain::StoreError::PlanNotFound => "notFound",
            audiorouter_domain::StoreError::PlanExpired => "planExpired",
            audiorouter_domain::StoreError::PlanLimitReached => "planLimitReached",
            audiorouter_domain::StoreError::InvalidGraph(_) => "invalidGraph",
            audiorouter_domain::StoreError::RevisionConflict { .. } => "revisionConflict",
            audiorouter_domain::StoreError::EmptyIdempotencyKey => "invalidRequest",
            audiorouter_domain::StoreError::IdempotencyKeyTooLong => "invalidRequest",
            audiorouter_domain::StoreError::NoUndoAvailable => "noUndoAvailable",
            audiorouter_domain::StoreError::IdempotencyConflict => "idempotencyConflict",
        },
        ControlError::Storage(_) => "storageFailure",
        ControlError::CorruptDatabase(_) => "corruptDatabase",
        ControlError::Json(_) => "internalError",
        ControlError::IdempotencyConflict => "idempotencyConflict",
        ControlError::InvalidRequest(_) => "invalidRequest",
        ControlError::Audio { code, .. } => code,
        ControlError::PluginScan(error) => error.code(),
    };
    let message = match &error {
        ControlError::Audio { message, .. } => message.clone(),
        _ => format!("{error:?}"),
    };
    let audio_details = match &error {
        ControlError::Audio {
            hresult,
            retryable,
            remediation,
            ..
        } => Some((*hresult, *retryable, *remediation)),
        _ => None,
    };
    let mut response = JsonRpcResponse::failure(id, -32000, message);
    if let Some(error) = response.error.as_mut() {
        let mut data = application_error_data(code);
        if let Some((hresult, retryable, remediation)) = audio_details {
            data["hresult"] = json!(hresult);
            data["retryable"] = json!(retryable);
            data["remediation"] = json!(remediation);
        }
        error.data = Some(data);
    }
    response
}

fn application_error_data(code: &str) -> Value {
    let (retryable, remediation) = match code {
        "revisionConflict" => (
            true,
            "read the latest session revision and create a new plan",
        ),
        "planExpired" => (true, "create a new plan from the current session revision"),
        "planLimitReached" => (
            true,
            "wait for an existing plan to expire, then retry planning",
        ),
        "storageFailure" => (
            true,
            "inspect backend health and retry after the failure is resolved",
        ),
        "corruptDatabase" => (
            false,
            "open a validated backup or restore into a new database destination",
        ),
        "deviceUnavailable" => (
            true,
            "inspect device availability and rebind the affected resource",
        ),
        "permissionDenied" => (
            false,
            "request the required permission scope for the target operation",
        ),
        "invalidRoot" | "tooManyCandidates" | "cancelled" => (
            false,
            "choose a valid configured directory or explicitly retry the scan",
        ),
        "deadlineExceeded" | "io" => (
            true,
            "retry the bounded scan after checking directory availability",
        ),
        "invalidRequest" | "invalidGraph" => {
            (false, "correct the request using the discovered schema")
        }
        _ => (
            false,
            "inspect the error code and correct or explicitly retry the operation",
        ),
    };
    json!({
        "code": code,
        "fieldPath": Value::Null,
        "resourceIds": [],
        "retryable": retryable,
        "remediation": remediation
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_domain::{Edge, Node, NodeKind, Port, PortDirection};
    use audiorouter_engine::{RuntimeGeneration, RuntimeGraph, RuntimeProcessor};

    struct TestRecorderWorker;

    impl RecorderWorker for TestRecorderWorker {
        fn finalize(&mut self, _frame: u64) -> Result<RecorderFinalizationOutcome, String> {
            Ok(RecorderFinalizationOutcome {
                state: "completed".into(),
                file_finalized: true,
                recoverable: false,
            })
        }
    }

    struct HookRecorderWorker {
        hooks: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl RecorderWorker for HookRecorderWorker {
        fn arm(&mut self) -> Result<(), String> {
            self.hooks.lock().unwrap().push("arm".into());
            Ok(())
        }

        fn start(&mut self, frame: u64) -> Result<(), String> {
            self.hooks.lock().unwrap().push(format!("start:{frame}"));
            Ok(())
        }

        fn split(&mut self, frame: u64) -> Result<(), String> {
            self.hooks.lock().unwrap().push(format!("split:{frame}"));
            Ok(())
        }

        fn finalize(&mut self, _frame: u64) -> Result<RecorderFinalizationOutcome, String> {
            Ok(RecorderFinalizationOutcome {
                state: "completed".into(),
                file_finalized: true,
                recoverable: false,
            })
        }
    }

    #[test]
    fn timestamped_plan_id_allocator_skips_collisions_and_bounds_exhaustion() {
        for prefix in ["startup-plan", "virtual-plan"] {
            let occupied = EntityId::new(format!("{prefix}-123-1"));
            let mut counter = 1;
            let allocated = allocate_timestamped_plan_id(
                prefix,
                123,
                &mut counter,
                |id| id == &occupied,
                "plan IDs exhausted",
            )
            .unwrap();
            assert_eq!(allocated.as_str(), format!("{prefix}-123-2"));
            assert_eq!(counter, 3);

            counter = u64::MAX;
            assert!(matches!(
                allocate_timestamped_plan_id(
                    prefix,
                    123,
                    &mut counter,
                    |_| true,
                    "plan IDs exhausted",
                ),
                Err(ControlError::InvalidRequest(message)) if message == "plan IDs exhausted"
            ));
            assert_eq!(counter, u64::MAX);
        }
    }

    #[test]
    fn counter_plan_id_allocator_skips_collisions_and_bounds_exhaustion() {
        let occupied = EntityId::new("session-import-1");
        let mut counter = 1;
        let allocated = allocate_counter_plan_id(
            "session-import",
            &mut counter,
            |id| id == &occupied,
            "plan IDs exhausted",
        )
        .unwrap();
        assert_eq!(allocated.as_str(), "session-import-2");
        assert_eq!(counter, 3);

        counter = u64::MAX;
        assert!(matches!(
            allocate_counter_plan_id(
                "session-import",
                &mut counter,
                |_| true,
                "plan IDs exhausted",
            ),
            Err(ControlError::InvalidRequest(message)) if message == "plan IDs exhausted"
        ));
        assert_eq!(counter, u64::MAX);
    }

    #[test]
    fn persisted_plan_duration_rejects_expired_and_caps_far_future_values() {
        let maximum = Duration::from_secs(300);
        assert_eq!(remaining_persisted_plan_duration(99, 100, maximum), None);
        assert_eq!(remaining_persisted_plan_duration(100, 100, maximum), None);
        assert_eq!(
            remaining_persisted_plan_duration(101, 100, maximum),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            remaining_persisted_plan_duration(i64::MAX, 0, maximum),
            Some(maximum)
        );
        assert_eq!(
            remaining_persisted_plan_duration(i64::MIN, i64::MAX, maximum),
            None
        );
    }

    #[test]
    fn mutation_rate_limiter_enforces_burst_and_refill_rate() {
        let mut limiter = MutationRateLimiter::default();
        let start = Instant::now();
        for _ in 0..40 {
            assert!(limiter.allow_at("client", start).is_ok());
        }
        assert_eq!(limiter.allow_at("client", start), Err(50));
        assert!(limiter
            .allow_at("client", start + std::time::Duration::from_millis(50))
            .is_ok());
    }

    #[test]
    fn mutation_rate_limiter_bounds_distinct_client_retention() {
        let mut limiter = MutationRateLimiter::default();
        let start = Instant::now();
        for index in 0..MAX_MUTATION_BUCKETS {
            assert!(limiter.allow_at(&format!("client-{index}"), start).is_ok());
        }
        assert_eq!(limiter.buckets.len(), MAX_MUTATION_BUCKETS);
        assert_eq!(limiter.allow_at("new-client", start), Err(1_000));

        let after_retention = start + MUTATION_BUCKET_RETENTION + Duration::from_millis(1);
        assert!(limiter.allow_at("new-client", after_retention).is_ok());
        assert!(limiter.buckets.len() <= MAX_MUTATION_BUCKETS);
    }

    #[test]
    fn authenticated_dispatch_returns_rate_limit_metadata() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original).unwrap();
        let grant = ClientGrant::for_role(ClientRole::Operator);
        let request = || JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "session.start".into(),
            params: Some(json!({
                "sessionId": "session",
                "idempotencyKey": "rate-limit-start"
            })),
        };
        for _ in 0..40 {
            assert!(plane
                .dispatch_authorized_for_client(request(), "client", &grant)
                .result
                .is_some());
        }
        let response = plane.dispatch_authorized_for_client(request(), "client", &grant);
        assert_eq!(response.error.as_ref().unwrap().code, -32000);
        let data = response.error.as_ref().unwrap().data.as_ref().unwrap();
        assert_eq!(data["code"], "rateLimited");
        let retry_after_ms = data["retryAfterMs"].as_u64().unwrap();
        assert!((1..=50).contains(&retry_after_ms));
        assert_eq!(data["retryable"], true);
    }

    #[test]
    fn sessions_list_supports_stable_cursor_pages() {
        let mut plane = ControlPlane::default();
        for id in ["a", "b", "c"] {
            let mut value = session();
            value.id = EntityId::new(id);
            plane.insert_session(value).unwrap();
        }
        let first = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "sessions.list".into(),
                params: Some(json!({ "limit": 2 })),
            })
            .result
            .unwrap();
        assert_eq!(first["items"].as_array().unwrap().len(), 2);
        assert_eq!(first["items"][0]["id"], "a");
        assert_eq!(first["nextCursor"], "b");
        let second = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(2)),
                method: "sessions.list".into(),
                params: Some(json!({ "cursor": "b", "limit": 2 })),
            })
            .result
            .unwrap();
        assert_eq!(second["items"].as_array().unwrap().len(), 1);
        assert_eq!(second["items"][0]["id"], "c");
        assert!(second["nextCursor"].is_null());
    }

    #[test]
    fn storage_startup_restores_all_bounded_sessions() {
        let storage = Storage::open_memory().unwrap();
        for index in 0..audiorouter_domain::MAX_SESSIONS_GLOBAL {
            let mut value = session();
            value.id = EntityId::new(format!("session-{index:03}"));
            value.nodes.clear();
            value.edges.clear();
            storage.save_session(&value).unwrap();
        }

        let plane = ControlPlane::try_with_storage("paged-startup", storage).unwrap();
        let result = plane.sessions_list_page(None, 500).unwrap();
        assert_eq!(
            result["items"].as_array().unwrap().len(),
            audiorouter_domain::MAX_SESSIONS_GLOBAL
        );
        assert!(result["nextCursor"].is_null());
    }

    #[test]
    fn virtual_devices_list_exposes_empty_managed_inventory_without_activation() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "virtualDevices.list".into(),
            params: None,
        });
        let result = response
            .result
            .unwrap_or_else(|| panic!("unexpected response error: {:?}", response.error));
        assert_eq!(result, json!([]));

        let paged = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "virtualDevices.list".into(),
            params: Some(json!({ "limit": 1 })),
        });
        let paged_result = paged
            .result
            .unwrap_or_else(|| panic!("unexpected paged response error: {:?}", paged.error));
        assert_eq!(paged_result, json!({ "items": [], "nextCursor": null }));
    }

    #[test]
    fn virtual_devices_plan_apply_is_revisionless_and_idempotent() {
        let mut plane = ControlPlane::default();
        let planned = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(3)),
                method: "virtualDevices.plan".into(),
                params: Some(json!({
                    "operation": {
                        "action": "create",
                        "id": "bus-1",
                        "name": "Desktop In"
                    }
                })),
            })
            .result
            .unwrap();
        assert_eq!(planned["availability"]["status"], "unavailable");
        let plan_id = planned["planId"].as_str().unwrap().to_owned();
        let request = |id, plan_id: &str| JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(id)),
            method: "virtualDevices.apply".into(),
            params: Some(json!({ "planId": plan_id, "idempotencyKey": "create-bus-1" })),
        };
        let applied = plane.dispatch(request(4, &plan_id)).result.unwrap();
        assert_eq!(applied["state"], "applied");
        let events = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(40)),
                method: "events.subscribe".into(),
                params: Some(json!({
                    "afterSequence": 0,
                    "sessionId": "unrelated-session"
                })),
            })
            .result
            .unwrap();
        assert_eq!(events["events"][0]["category"], "virtualDevice.changed");
        let operation = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(4)),
                method: "operations.get".into(),
                params: Some(json!({ "operationId": "create-bus-1" })),
            })
            .result
            .unwrap();
        assert_eq!(operation["operation"], "virtualDevices.apply");
        let replay = plane.dispatch(request(5, &plan_id)).result.unwrap();
        assert_eq!(replay, applied);
        let second_plan = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(5)),
                method: "virtualDevices.plan".into(),
                params: Some(json!({
                    "operation": {
                        "action": "create",
                        "id": "bus-2",
                        "name": "Desktop Out"
                    }
                })),
            })
            .result
            .unwrap()["planId"]
            .as_str()
            .unwrap()
            .to_owned();
        let conflict = plane.dispatch(request(6, &second_plan));
        assert_eq!(
            conflict.error.as_ref().unwrap().data.as_ref().unwrap()["code"],
            "idempotencyConflict"
        );
        let inventory = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(7)),
                method: "virtualDevices.list".into(),
                params: None,
            })
            .result
            .unwrap();
        assert_eq!(inventory[0]["name"], "Desktop In");
        assert_eq!(inventory[0]["endpointIds"]["render"], Value::Null);
        assert_eq!(
            inventory[0]["capabilities"],
            json!({ "render": false, "capture": false, "channels": 2 })
        );
        assert_eq!(inventory[0]["privilege"], "deviceAdministration");
        assert_eq!(inventory[0]["restartRequired"], false);
        assert_eq!(inventory[0]["clientImpacts"], json!([]));
    }

    #[test]
    fn virtual_devices_dispatch_enforces_eight_bus_capacity() {
        let mut plane = ControlPlane::default();
        for index in 0..audiorouter_domain::MAX_VIRTUAL_BUSES {
            let planned = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(index as u64)),
                method: "virtualDevices.plan".into(),
                params: Some(json!({
                    "operation": {
                        "action": "create",
                        "id": format!("bus-{index}"),
                        "name": format!("Bus {index}")
                    }
                })),
            });
            let plan_id = planned.result.unwrap()["planId"]
                .as_str()
                .unwrap()
                .to_owned();
            let applied = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(100 + index as u64)),
                method: "virtualDevices.apply".into(),
                params: Some(json!({
                    "planId": plan_id,
                    "idempotencyKey": format!("create-bus-{index}")
                })),
            });
            assert_eq!(applied.result.unwrap()["state"], "applied");
        }

        let inventory = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(200)),
                method: "virtualDevices.list".into(),
                params: None,
            })
            .result
            .unwrap();
        assert_eq!(
            inventory.as_array().unwrap().len(),
            audiorouter_domain::MAX_VIRTUAL_BUSES
        );

        let overflow = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(201)),
            method: "virtualDevices.plan".into(),
            params: Some(json!({
                "operation": { "action": "create", "id": "bus-overflow", "name": "Overflow" }
            })),
        });
        assert!(overflow.error.is_some());
        assert!(overflow.error.unwrap().message.contains("LimitReached"));
    }

    #[test]
    fn storage_backed_virtual_device_plan_survives_control_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-virtual-plan-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let plan_id = {
            let mut plane = ControlPlane::with_storage("plan-first", Storage::open(&path).unwrap());
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(7)),
                method: "virtualDevices.plan".into(),
                params: Some(json!({
                    "operation": {
                        "action": "create",
                        "id": "bus-1",
                        "name": "Desktop In"
                    }
                })),
            });
            response.result.unwrap()["planId"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        let mut restarted =
            ControlPlane::with_storage("plan-second", Storage::open(&path).unwrap());
        let applied = restarted.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(8)),
            method: "virtualDevices.apply".into(),
            params: Some(json!({ "planId": plan_id, "idempotencyKey": "restart-apply" })),
        });
        assert_eq!(applied.result.unwrap()["state"], "applied");
        let operation = restarted.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(8)),
            method: "operations.get".into(),
            params: Some(json!({ "operationId": "restart-apply" })),
        });
        assert_eq!(
            operation.result.unwrap()["operation"],
            "virtualDevices.apply"
        );
        let mut replayed = ControlPlane::with_storage("plan-third", Storage::open(&path).unwrap());
        let replay = replayed.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(9)),
            method: "virtualDevices.apply".into(),
            params: Some(json!({ "planId": plan_id, "idempotencyKey": "restart-apply" })),
        });
        assert_eq!(replay.result.unwrap()["state"], "applied");
        let conflict = replayed.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(10)),
            method: "virtualDevices.apply".into(),
            params: Some(json!({
                "planId": "different-plan",
                "idempotencyKey": "restart-apply"
            })),
        });
        assert_eq!(
            conflict.error.as_ref().unwrap().data.as_ref().unwrap()["code"],
            "idempotencyConflict"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn session_create_and_delete_protect_running_resources() {
        let mut plane = ControlPlane::default();
        let mut created = session();
        created.id = EntityId::new("created");
        let result = plane.create_session(created.clone()).unwrap();
        assert_eq!(result["state"], "stopped");
        let duplicate = plane
            .duplicate_session(
                &created.id,
                EntityId::new("copy"),
                Some("Copied session".into()),
            )
            .unwrap();
        assert_eq!(duplicate["session"]["id"], "copy");
        assert_eq!(duplicate["session"]["name"], "Copied session");
        assert_eq!(duplicate["session"]["revision"], 0);
        assert!(matches!(
            plane.duplicate_session(&created.id, EntityId::new("copy"), None),
            Err(ControlError::InvalidRequest(message))
                if message == "duplicate session ID already exists"
        ));
        assert_eq!(plane.delete_session(&created.id).unwrap()["deleted"], true);
        assert_eq!(
            plane.delete_session(&EntityId::new("copy")).unwrap()["deleted"],
            true
        );
        assert!(matches!(
            plane.delete_session(&created.id),
            Err(ControlError::InvalidRequest(message)) if message == "session not found"
        ));

        let mut running = session();
        running.id = EntityId::new("running");
        plane.create_session(running.clone()).unwrap();
        plane.session_start(&running.id).unwrap();
        assert!(matches!(
            plane.delete_session(&running.id),
            Err(ControlError::InvalidRequest(message)) if message == "stop the session before deleting it"
        ));
        plane.session_stop(&running.id).unwrap();
        assert_eq!(plane.delete_session(&running.id).unwrap()["deleted"], true);
    }

    #[test]
    fn ephemeral_plan_maps_bound_pending_entries_and_prune_expired_entries() {
        let mut plane = ControlPlane::default();

        for index in 0..MAX_PENDING_PLAN_RECORDS {
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(index as u64)),
                method: "startup.plan".into(),
                params: Some(json!({ "enabled": true })),
            });
            assert!(response.result.is_some(), "startup plan {index} failed");
        }
        let startup_overflow = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(MAX_PENDING_PLAN_RECORDS as u64)),
            method: "startup.plan".into(),
            params: Some(json!({ "enabled": true })),
        });
        assert!(startup_overflow
            .error
            .as_ref()
            .is_some_and(|error| error.message.contains("too many pending startup plans")));

        for index in 0..MAX_PENDING_PLAN_RECORDS {
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!((1000 + index) as u64)),
                method: "virtualDevices.plan".into(),
                params: Some(json!({
                    "operation": {
                        "action": "create",
                        "id": "bus-pending",
                        "name": "Pending"
                    }
                })),
            });
            assert!(response.result.is_some(), "virtual plan {index} failed");
        }
        let virtual_overflow = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2000)),
            method: "virtualDevices.plan".into(),
            params: Some(json!({
                "operation": {
                    "action": "create",
                    "id": "bus-pending",
                    "name": "Pending"
                }
            })),
        });
        assert!(virtual_overflow.error.as_ref().is_some_and(|error| error
            .message
            .contains("too many pending virtual-device plans")));

        for index in 0..MAX_PENDING_PLAN_RECORDS {
            let mut imported = session();
            imported.id = EntityId::new(format!("import-{index}"));
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!((3000 + index) as u64)),
                method: "sessions.importPlan".into(),
                params: Some(json!({ "session": imported })),
            });
            assert!(response.result.is_some(), "import plan {index} failed");
        }
        let import_overflow = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4000)),
            method: "sessions.importPlan".into(),
            params: Some(json!({ "session": session() })),
        });
        assert!(import_overflow.error.as_ref().is_some_and(|error| error
            .message
            .contains("too many pending session import plans")));

        let existing_startup = plane.startup_plans.keys().next().cloned().unwrap();
        plane.startup_plans.remove(&existing_startup);
        plane.startup_plans.insert(
            EntityId::new("expired-startup"),
            (true, Instant::now() - Duration::from_secs(1)),
        );
        let recovered = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4001)),
            method: "startup.plan".into(),
            params: Some(json!({ "enabled": false })),
        });
        assert!(recovered.result.is_some());
    }

    #[test]
    fn application_errors_include_stable_codes() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "changed".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "graph.commit".into(),
            params: Some(json!({
                "planId": plan,
                "baseRevision": 1,
                "idempotencyKey": "conflict"
            })),
        });
        assert_eq!(response.error.as_ref().unwrap().code, -32000);
        let data = response.error.as_ref().unwrap().data.as_ref().unwrap();
        assert_eq!(data["code"], "revisionConflict");
        assert_eq!(data["retryable"], true);
        assert!(data["remediation"].as_str().unwrap().contains("new plan"));
    }

    fn session() -> Session {
        Session {
            id: EntityId::new("session"),
            name: "test".into(),
            schema_version: 1,
            revision: 0,
            nodes: vec![
                Node {
                    id: EntityId::new("in"),
                    kind: NodeKind::PhysicalInput,
                    type_version: 1,
                    name: "Input".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Output,
                        channels: 1,
                    }],
                },
                Node {
                    id: EntityId::new("out"),
                    kind: NodeKind::PhysicalOutput,
                    type_version: 1,
                    name: "Output".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Input,
                        channels: 1,
                    }],
                },
            ],
            edges: vec![Edge {
                id: EntityId::new("edge"),
                source_node: EntityId::new("in"),
                source_port: "main".into(),
                destination_node: EntityId::new("out"),
                destination_port: "main".into(),
                matrix: vec![1.0],
                enabled: true,
            }],
        }
    }

    #[test]
    fn describe_exposes_versions_methods_limits_and_unavailable_nodes() {
        let plane = ControlPlane::new("test-build");
        let description = plane.describe();
        assert_eq!(description["build"], "test-build");
        assert_eq!(description["protocolVersion"]["major"], 1);
        let describe_method = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "system.describe")
            .unwrap();
        for field in describe_method["outputSchema"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
        {
            assert!(
                description.get(field).is_some(),
                "system.describe required field {field} missing from response"
            );
        }
        for field in describe_method["outputSchema"]["properties"]["limits"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
        {
            assert!(
                description["limits"].get(field).is_some(),
                "system.describe limits field {field} missing from response"
            );
        }
        for field in describe_method["outputSchema"]["properties"]["events"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
        {
            assert!(
                description["events"].get(field).is_some(),
                "system.describe events field {field} missing from response"
            );
        }
        assert_eq!(description["limits"]["maxNodesPerSession"], 64);
        assert_eq!(description["limits"]["maxNodesGlobal"], 128);
        assert_eq!(description["limits"]["maxEdgesGlobal"], 256);
        assert_eq!(
            description["limits"]["maxSessionsGlobal"],
            audiorouter_domain::MAX_SESSIONS_GLOBAL
        );
        assert_eq!(description["limits"]["maxActiveSessions"], 2);
        assert_eq!(
            description["limits"]["maxClientEnrollments"],
            audiorouter_storage::MAX_CLIENT_ENROLLMENTS
        );
        assert_eq!(
            description["limits"]["maxOperationJournalEntries"],
            audiorouter_storage::MAX_OPERATION_JOURNAL_ENTRIES
        );
        assert_eq!(description["limits"]["maxVirtualBuses"], 8);
        assert_eq!(
            description["limits"]["maxVirtualBusNameChars"],
            audiorouter_domain::MAX_VIRTUAL_BUS_NAME_CHARS
        );
        assert_eq!(
            description["limits"]["maxEntityIdBytes"],
            audiorouter_domain::MAX_ENTITY_ID_BYTES
        );
        assert_eq!(
            description["limits"]["maxDisplayNameBytes"],
            audiorouter_domain::MAX_DISPLAY_NAME_BYTES
        );
        assert_eq!(
            description["limits"]["maxPortNameBytes"],
            audiorouter_domain::MAX_PORT_NAME_BYTES
        );
        assert_eq!(
            description["limits"]["maxPortsPerNode"],
            audiorouter_domain::MAX_PORTS_PER_NODE
        );
        assert_eq!(
            description["limits"]["maxChannelMatrixCoefficients"],
            audiorouter_domain::MAX_CHANNEL_MATRIX_COEFFICIENTS
        );
        assert_eq!(
            description["limits"]["maxControlValueDepth"],
            MAX_CONTROL_VALUE_DEPTH
        );
        assert_eq!(
            description["limits"]["maxControlStringBytes"],
            MAX_CONTROL_STRING_BYTES
        );
        assert_eq!(
            description["limits"]["maxControlValueCount"],
            MAX_CONTROL_VALUE_COUNT
        );
        assert_eq!(
            description["limits"]["maxMethodNameBytes"],
            MAX_METHOD_NAME_BYTES
        );
        assert_eq!(
            description["limits"]["maxRequestIdBytes"],
            MAX_REQUEST_ID_BYTES
        );
        assert_eq!(
            description["limits"]["maxRevisionCursorBytes"],
            MAX_REVISION_CURSOR_BYTES
        );
        let create = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "sessions.create")
            .unwrap();
        let session_schema = &create["outputSchema"]["properties"]["session"];
        assert_eq!(session_schema["properties"]["name"]["maxLength"], 256);
        assert_eq!(session_schema["properties"]["nodes"]["maxItems"], 64);
        assert_eq!(session_schema["properties"]["edges"]["maxItems"], 128);
        let node_schema = &session_schema["properties"]["nodes"]["items"];
        assert_eq!(node_schema["properties"]["ports"]["maxItems"], 16);
        assert_eq!(
            node_schema["properties"]["parameters"]["maxProperties"],
            audiorouter_domain::MAX_PARAMETERS_PER_NODE
        );
        assert_eq!(
            node_schema["properties"]["parameters"]["propertyNames"]["maxLength"],
            128
        );
        assert_eq!(
            node_schema["properties"]["ports"]["items"]["properties"]["channels"]["maximum"],
            2
        );
        let edge_schema = &session_schema["properties"]["edges"]["items"];
        assert_eq!(edge_schema["properties"]["matrix"]["maxItems"], 4);
        assert_eq!(
            edge_schema["properties"]["matrix"]["items"]["minimum"],
            -2.0
        );
        assert_eq!(edge_schema["properties"]["matrix"]["items"]["maximum"], 2.0);
        let create_input = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "sessions.create")
            .unwrap();
        assert_eq!(
            create_input["inputSchema"]["properties"]["session"]["properties"]["nodes"]["maxItems"],
            64
        );
        let import_input = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "sessions.importPlan")
            .unwrap();
        assert_eq!(
            import_input["inputSchema"]["properties"]["session"]["properties"]["edges"]["maxItems"],
            128
        );
        let graph_plan_input = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "graph.plan")
            .unwrap();
        assert_eq!(
            graph_plan_input["inputSchema"]["properties"]["candidate"]["properties"]["nodes"]
                ["maxItems"],
            64
        );
        let graph_plan = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "graph.plan")
            .unwrap();
        assert_eq!(
            graph_plan["outputSchema"]["properties"]["diff"]["maxItems"],
            MAX_GRAPH_DIFF_ITEMS
        );
        assert_eq!(
            graph_plan["outputSchema"]["properties"]["affectedDestinations"]["maxItems"],
            MAX_GRAPH_AFFECTED_DESTINATIONS
        );
        assert_eq!(
            graph_plan["outputSchema"]["properties"]["affectedDestinations"]["items"]["maxLength"],
            audiorouter_domain::MAX_DISPLAY_NAME_BYTES
        );
        assert_eq!(
            graph_plan["outputSchema"]["properties"]["requiredScopes"]["maxItems"],
            MAX_PLAN_REQUIRED_SCOPES
        );
        assert_eq!(
            graph_plan["outputSchema"]["properties"]["warnings"]["maxItems"],
            MAX_PLAN_WARNINGS
        );
        assert_eq!(description["events"]["retention"]["maxEvents"], 10_000);
        assert_eq!(description["events"]["retention"]["maxAgeSeconds"], 900);
        assert_eq!(description["events"]["meterReplay"], false);
        let events_method = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "events.subscribe")
            .unwrap();
        assert_eq!(
            events_method["outputSchema"]["properties"]["events"]["maxItems"],
            MAX_EVENT_SUBSCRIPTION_ITEMS
        );
        assert_eq!(
            events_method["outputSchema"]["properties"]["events"]["items"]["properties"]
                ["category"]["maxLength"],
            audiorouter_domain::MAX_EVENT_CATEGORY_BYTES
        );
        assert_eq!(
            events_method["inputSchema"]["properties"]["limit"]["maximum"],
            MAX_EVENT_SUBSCRIPTION_ITEMS
        );
        let recovery = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "recordings.recovery")
            .unwrap();
        assert_eq!(
            recovery["outputSchema"]["oneOf"][0]["properties"]["checkpoint"]["properties"]["parts"]
                ["maxItems"],
            audiorouter_recording::MAX_CHECKPOINT_PARTS
        );
        assert_eq!(
            recovery["outputSchema"]["oneOf"][0]["properties"]["checkpoint"]["properties"]
                ["pauses"]["maxItems"],
            audiorouter_recording::MAX_CHECKPOINT_PAUSES
        );
        assert_eq!(
            recovery["outputSchema"]["oneOf"][0]["properties"]["checkpoint"]["properties"]
                ["stop_frame"]["type"],
            json!(["integer", "null"])
        );
        assert_eq!(
            recovery["outputSchema"]["oneOf"][1]["properties"]["items"]["items"]["properties"]
                ["checkpoint"]["properties"]["parts"]["maxItems"],
            audiorouter_recording::MAX_CHECKPOINT_PARTS
        );
        let recorder_transition = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "recorders.start")
            .unwrap();
        assert_eq!(
            recorder_transition["outputSchema"]["properties"]["parts"]["maxItems"],
            audiorouter_recording::MAX_CHECKPOINT_PARTS
        );
        assert_eq!(
            recorder_transition["outputSchema"]["properties"]["pauses"]["maxItems"],
            audiorouter_recording::MAX_CHECKPOINT_PAUSES
        );
        let virtual_devices = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "virtualDevices.list")
            .unwrap();
        assert_eq!(
            virtual_devices["outputSchema"]["oneOf"][1]["properties"]["items"]["maxItems"],
            MAX_VIRTUAL_DEVICE_LIST_ITEMS
        );
        assert_eq!(
            virtual_devices["inputSchema"]["properties"]["limit"]["maximum"],
            MAX_VIRTUAL_DEVICE_LIST_ITEMS
        );
        assert_eq!(
            virtual_devices["outputSchema"]["oneOf"][0]["items"]["properties"]["id"]["maxLength"],
            audiorouter_domain::MAX_ENTITY_ID_BYTES
        );
        let virtual_device_plan = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "virtualDevices.plan")
            .unwrap();
        assert_eq!(
            virtual_device_plan["inputSchema"]["properties"]["operation"]["properties"]["id"]
                ["maxLength"],
            audiorouter_domain::MAX_ENTITY_ID_BYTES
        );
        assert_eq!(
            virtual_device_plan["outputSchema"]["properties"]["operation"]["properties"]["id"]
                ["maxLength"],
            audiorouter_domain::MAX_ENTITY_ID_BYTES
        );
        let virtual_device_apply = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "virtualDevices.apply")
            .unwrap();
        assert_eq!(
            virtual_device_apply["outputSchema"]["properties"]["operation"]["properties"]["name"]
                ["maxLength"],
            audiorouter_domain::MAX_VIRTUAL_BUS_NAME_CHARS
        );
        assert!(description["events"]["stateCategories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|category| category == "graph.committed"));
        assert_eq!(
            description["events"]["stateCategories"]
                .as_array()
                .unwrap()
                .len(),
            STATE_CATEGORIES.len()
        );
        let describe_schema = method_output_schema("system.describe");
        assert_eq!(
            describe_schema["properties"]["events"]["properties"]["stateCategories"]["maxItems"],
            STATE_CATEGORIES.len()
        );
        assert!(description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "graph.plan"));
        assert!(description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "nodes.describe"));
        assert!(description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "sessions.get"));
        assert!(description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "applications.list"));
        assert!(description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "system.diagnostics"));
        assert!(description["nodeTypes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["type"] == "physical-input@1"
                && node["availability"]["status"] == "unavailable"));
        assert_eq!(description["processors"].as_array().unwrap().len(), 7);
        assert_eq!(
            description["processors"][0]["availability"]["status"],
            "available"
        );
        assert_eq!(
            description["processors"][0]["parameters"][0]["type"],
            "number"
        );
        assert_eq!(
            description["methods"]
                .as_array()
                .unwrap()
                .iter()
                .find(|method| method["name"] == "processors.list")
                .unwrap()["outputSchema"]["items"]["properties"]["availability"]["properties"]
                ["status"]["enum"][1],
            "unavailable"
        );
        let pitch = description["processors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|processor| processor["id"] == "pitch")
            .unwrap();
        assert_eq!(pitch["latencySamples"], 1024);
        assert_eq!(pitch["availability"]["status"], "available");
        let parametric_eq = description["processors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|processor| processor["id"] == "parametricEq")
            .unwrap();
        assert_eq!(parametric_eq["parameters"][0]["default"], 1000.0);
        assert_eq!(parametric_eq["parameters"][1]["default"], 1.0);
        let compressor = description["processors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|processor| processor["id"] == "compressor")
            .unwrap();
        assert_eq!(compressor["parameters"][4]["name"], "kneeDb");
        assert_eq!(compressor["parameters"][4]["default"], 6.0);
        let gate = description["processors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|processor| processor["id"] == "gate")
            .unwrap();
        assert_eq!(gate["parameters"][2]["name"], "hysteresisDb");
        assert_eq!(gate["parameters"][5]["name"], "holdMs");
        assert_eq!(gate["parameters"][5]["default"], 50.0);
        let gain = description["nodeTypes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["type"] == "gain@1")
            .unwrap();
        assert_eq!(gain["parameters"][0]["name"], "gainDb");
        assert_eq!(gain["parameters"][0]["minimum"], -60.0);
        assert_eq!(gain["parameters"][0]["maximum"], 24.0);
        assert_eq!(
            description["presets"]["voiceChains"][0]["id"],
            "voiceNeutral"
        );
        assert_eq!(description["presets"]["voiceChains"][0]["version"], 1);
        assert_eq!(
            description["presets"]["voiceChains"][1]["name"],
            "Voice gate and compression"
        );
        assert!(description["presets"]["voiceChains"][0]["description"]
            .as_str()
            .is_some_and(|value| !value.is_empty()));
        assert_eq!(description["presets"]["eq"].as_array().unwrap().len(), 3);
        assert_eq!(description["presets"]["eq"][1]["id"], "hum50Hz");
        assert_eq!(description["presets"]["eq"][1]["version"], 1);
        let presets = ControlPlane::default().dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "presets.list".into(),
            params: None,
        });
        let result = presets.result.unwrap();
        assert_eq!(result["voiceChains"].as_array().unwrap().len(), 2);
        assert_eq!(result["eq"].as_array().unwrap().len(), 3);
        assert!(result["voiceChains"]
            .as_array()
            .unwrap()
            .iter()
            .all(|preset| preset["version"] == 1));
        let processors = ControlPlane::default().dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "processors.list".into(),
            params: None,
        });
        let result = processors.result.unwrap();
        assert_eq!(result.as_array().unwrap().len(), 7);
        assert_eq!(result[0]["availability"]["status"], "available");
    }

    #[test]
    fn plugin_scan_is_read_only_and_keeps_invalid_candidates_visible() {
        let root = std::env::temp_dir().join(format!(
            "audiorouter-control-plugin-scan-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("candidate.dll"), b"not a PE binary").unwrap();
        let mut plane = ControlPlane::default();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "plugins.scan".into(),
            params: Some(json!({ "directory": root.to_string_lossy() })),
        };
        let denied = plane.dispatch_authorized(request.clone(), &ClientGrant::read_only());
        assert_eq!(
            denied.error.unwrap().data.unwrap()["code"],
            "permissionDenied"
        );
        let response = plane.dispatch_authorized(
            request,
            &ClientGrant::with_scopes([PermissionScope::PluginScan]),
        );
        assert!(response.error.is_none());
        let result = response.result.unwrap();
        assert_eq!(result["directory"], root.to_string_lossy().to_string());
        assert_eq!(result["entries"].as_array().unwrap().len(), 1);
        assert!(result["entries"][0]["identity"].is_null());
        assert!(result["entries"][0]["error"].is_string());
        assert_eq!(result["entries"][0]["errorCode"], "notPe");
        let listed = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(3)),
                method: "plugins.list".into(),
                params: Some(json!({ "directory": root.to_string_lossy() })),
            },
            &ClientGrant::with_scopes([PermissionScope::PluginScan]),
        );
        assert_eq!(listed.result.unwrap(), result);
        let retry_request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "plugins.retry".into(),
            params: Some(json!({
                "directory": root.to_string_lossy(),
                "idempotencyKey": "plugin-retry"
            })),
        };
        let retried = plane.dispatch_authorized(
            retry_request.clone(),
            &ClientGrant::with_scopes([PermissionScope::PluginScan]),
        );
        assert_eq!(
            retried.result.as_ref().unwrap()["entries"],
            result["entries"]
        );
        let replayed = plane.dispatch_authorized(
            retry_request,
            &ClientGrant::with_scopes([PermissionScope::PluginScan]),
        );
        assert_eq!(replayed.result.unwrap(), retried.result.unwrap());
        let inspected = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(2)),
                method: "plugins.inspect".into(),
                params: Some(json!({ "path": root.join("candidate.dll").to_string_lossy() })),
            },
            &ClientGrant::with_scopes([PermissionScope::PluginScan]),
        );
        assert!(inspected.error.is_none());
        let inspected_result = inspected.result.unwrap();
        assert!(inspected_result["identity"].is_null());
        assert_eq!(inspected_result["errorCode"], "notPe");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_inventory_cache_evicts_oldest_scan_roots() {
        let mut plane = ControlPlane::default();
        for index in 0..=MAX_PLUGIN_INVENTORY_ROOTS {
            plane.remember_plugin_inventory(
                format!("C:\\plugin-root-{index}"),
                json!({ "directory": format!("C:\\plugin-root-{index}"), "entries": [] }),
            );
        }

        assert_eq!(plane.plugin_inventories.len(), MAX_PLUGIN_INVENTORY_ROOTS);
        assert!(!plane.plugin_inventories.contains_key("C:\\plugin-root-0"));
        assert!(plane
            .plugin_inventories
            .contains_key(&format!("C:\\plugin-root-{MAX_PLUGIN_INVENTORY_ROOTS}")));
        assert_eq!(
            plane.plugin_inventory_order.len(),
            MAX_PLUGIN_INVENTORY_ROOTS
        );
    }

    #[test]
    fn plugin_scan_directory_failures_have_stable_application_codes() {
        let root = std::env::temp_dir().join(format!(
            "audiorouter-control-plugin-scan-missing-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let response = ControlPlane::default().dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "plugins.scan".into(),
                params: Some(json!({ "directory": root.to_string_lossy() })),
            },
            &ClientGrant::with_scopes([PermissionScope::PluginScan]),
        );
        assert_eq!(response.error.unwrap().data.unwrap()["code"], "invalidRoot");
    }

    #[test]
    fn plugin_inspect_discovery_has_typed_identity_schema() {
        let method = ControlPlane::default().describe()["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "plugins.inspect")
            .unwrap()
            .clone();
        assert_eq!(method["permission"], "pluginScan");
        assert_eq!(
            method["outputSchema"]["properties"]["identity"]["type"][1],
            "null"
        );
        assert_eq!(
            method["outputSchema"]["properties"]["identity"]["required"]
                .as_array()
                .unwrap()
                .len(),
            10
        );
        assert_eq!(
            method["outputSchema"]["properties"]["identity"]["properties"]["fileBytes"]["maximum"],
            audiorouter_plugin_host::MAX_PLUGIN_BYTES
        );
        assert_eq!(
            method["outputSchema"]["properties"]["identity"]["properties"]["classIds"]["maxItems"],
            256
        );
        assert!(method["outputSchema"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "errorCode"));
        assert_eq!(
            method["outputSchema"]["properties"]["errorCode"]["enum"]
                .as_array()
                .unwrap()
                .len(),
            10
        );
    }

    #[test]
    fn describe_exposes_input_and_output_schemas_for_methods() {
        let document = ControlPlane::default().describe();
        let methods = document["methods"].as_array().unwrap().clone();
        let commit = methods
            .iter()
            .find(|method| method["name"] == "graph.commit")
            .unwrap();
        assert_eq!(
            commit["description"],
            "Commit an unexpired graph plan with idempotent mutation."
        );
        assert_eq!(
            commit["inputSchema"]["required"],
            json!(["planId", "baseRevision", "idempotencyKey"])
        );
        for (method_name, field_name) in [
            ("graph.commit", "planId"),
            ("virtualDevices.apply", "planId"),
            ("startup.apply", "planId"),
            ("sessions.importCommit", "planId"),
            ("sessions.get", "sessionId"),
            ("sessions.export", "sessionId"),
            ("sessions.duplicate", "sourceSessionId"),
            ("routes.inspect", "destinationNode"),
            ("graph.plan", "sessionId"),
            ("graph.undoPlan", "sessionId"),
        ] {
            let method = methods
                .iter()
                .find(|method| method["name"] == method_name)
                .unwrap();
            assert_eq!(
                method["inputSchema"]["properties"][field_name]["maxLength"],
                audiorouter_domain::MAX_ENTITY_ID_BYTES,
                "{method_name} input {field_name} bound"
            );
        }
        for method_name in ["operations.get", "operations.cancel"] {
            let method = methods
                .iter()
                .find(|method| method["name"] == method_name)
                .unwrap();
            assert_eq!(
                method["inputSchema"]["properties"]["operationId"]["maxLength"],
                audiorouter_storage::MAX_IDEMPOTENCY_KEY_BYTES,
                "{method_name} input operationId bound"
            );
        }
        for method_name in [
            "startup.plan",
            "startup.apply",
            "sessions.importPlan",
            "graph.undoPlan",
            "graph.plan",
        ] {
            let method = methods
                .iter()
                .find(|method| method["name"] == method_name)
                .unwrap();
            assert_eq!(
                method["outputSchema"]["properties"]["planId"]["maxLength"],
                audiorouter_domain::MAX_ENTITY_ID_BYTES,
                "{method_name} output planId bound"
            );
        }
        let events = methods
            .iter()
            .find(|method| method["name"] == "events.subscribe")
            .unwrap();
        assert_eq!(
            events["outputSchema"]["properties"]["snapshot"]["properties"]["sessions"]
                ["properties"]["nextCursor"]["maxLength"],
            audiorouter_domain::MAX_ENTITY_ID_BYTES,
            "events snapshot session cursor bound"
        );
        assert_eq!(commit["outputSchema"]["type"], "object");
        let devices = methods
            .iter()
            .find(|method| method["name"] == "devices.list")
            .unwrap();
        assert_eq!(devices["inputSchema"]["additionalProperties"], false);
        assert_eq!(
            devices["inputSchema"]["properties"]["limit"]["maximum"],
            500
        );
        assert_eq!(
            devices["outputSchema"]["oneOf"][1]["properties"]["items"]["items"]["oneOf"][0]
                ["properties"]["direction"]["enum"],
            json!(["capture", "render"])
        );
        assert_eq!(
            devices["outputSchema"]["oneOf"][0]["maxItems"],
            MAX_DEVICE_LIST_ITEMS
        );
        let virtual_devices = methods
            .iter()
            .find(|method| method["name"] == "virtualDevices.list")
            .unwrap();
        assert_eq!(
            virtual_devices["outputSchema"]["oneOf"][0]["maxItems"],
            audiorouter_domain::MAX_VIRTUAL_BUSES
        );
        assert_eq!(
            devices["outputSchema"]["oneOf"][1]["properties"]["items"]["items"]["oneOf"][0]
                ["properties"]["format"]["required"],
            json!([
                "sampleRateHz",
                "channels",
                "bitsPerSample",
                "formatTag",
                "bytesPerFrame"
            ])
        );
        let node_types = methods
            .iter()
            .find(|method| method["name"] == "nodes.types")
            .unwrap();
        assert_eq!(
            node_types["outputSchema"]["items"]["properties"]["type"]["type"],
            "string"
        );
        assert_eq!(
            node_types["outputSchema"]["items"]["properties"]["parameters"]["items"]["required"],
            json!(["name", "type", "default"])
        );
        let clients = methods
            .iter()
            .find(|method| method["name"] == "clients.list")
            .unwrap();
        assert_eq!(
            clients["outputSchema"]["items"]["properties"]["role"]["enum"],
            json!(["observer", "editor", "operator"])
        );
        let status = methods
            .iter()
            .find(|method| method["name"] == "status.get")
            .unwrap();
        assert_eq!(
            status["outputSchema"]["properties"]["audio"]["const"],
            "unavailable"
        );
        assert_eq!(
            status["outputSchema"]["properties"]["activeSessionIds"]["maxItems"],
            audiorouter_domain::MAX_ACTIVE_SESSIONS
        );
        assert_eq!(
            status["outputSchema"]["properties"]["activeSessionIds"]["items"]["maxLength"],
            audiorouter_domain::MAX_ENTITY_ID_BYTES
        );
        assert_eq!(
            status["outputSchema"]["properties"]["eventCursor"]["required"],
            json!(["backendEpoch", "latestSequence"])
        );
        let diagnostics = methods
            .iter()
            .find(|method| method["name"] == "system.diagnostics")
            .unwrap();
        assert_eq!(
            diagnostics["outputSchema"]["properties"]["redacted"]["const"],
            true
        );
        assert_eq!(
            diagnostics["outputSchema"]["properties"]["eventLog"]["required"],
            json!(["latestSequence", "retained"])
        );
        let startup = methods
            .iter()
            .find(|method| method["name"] == "startup.get")
            .unwrap();
        assert_eq!(
            startup["outputSchema"]["properties"]["registration"]["const"],
            "unavailable"
        );
        let event_categories = document["events"]["stateCategories"].as_array().unwrap();
        for category in [
            "virtualDevice.changed",
            "recording.metadataChanged",
            "recording.renamed",
            "recording.entryRemoved",
            "recording.recycled",
        ] {
            assert!(event_categories.iter().any(|value| value == category));
        }
        let recovery_clear = methods
            .iter()
            .find(|method| method["name"] == "recovery.clearSafeMode")
            .unwrap();
        assert_eq!(
            recovery_clear["outputSchema"]["properties"]["safeMode"]["const"],
            false
        );
        let sessions = methods
            .iter()
            .find(|method| method["name"] == "sessions.list")
            .unwrap();
        assert_eq!(
            sessions["outputSchema"]["properties"]["nextCursor"]["type"],
            json!(["string", "null"])
        );
        assert_eq!(
            sessions["outputSchema"]["properties"]["items"]["items"]["properties"]["revision"]
                ["minimum"],
            0
        );
        assert_eq!(
            sessions["outputSchema"]["properties"]["items"]["maxItems"],
            MAX_SESSION_LIST_ITEMS
        );
        let history = methods
            .iter()
            .find(|method| method["name"] == "graph.history")
            .unwrap();
        assert_eq!(
            history["outputSchema"]["properties"]["items"]["maxItems"],
            MAX_GRAPH_HISTORY_ITEMS
        );
        assert_eq!(
            history["inputSchema"]["properties"]["cursor"]["maxLength"],
            MAX_REVISION_CURSOR_BYTES
        );
        assert_eq!(
            history["outputSchema"]["properties"]["nextCursor"]["maxLength"],
            MAX_REVISION_CURSOR_BYTES
        );
        let session_get = methods
            .iter()
            .find(|method| method["name"] == "sessions.get")
            .unwrap();
        assert_eq!(session_get["outputSchema"]["type"], "object");
        assert_eq!(
            session_get["outputSchema"]["properties"]["nodes"]["type"],
            "array"
        );
        assert_eq!(
            session_get["outputSchema"]["properties"]["edges"]["items"]["properties"]["sourceNode"]
                ["type"],
            "string"
        );
        let routes = methods
            .iter()
            .find(|method| method["name"] == "routes.inspect")
            .unwrap();
        assert_eq!(
            routes["outputSchema"]["properties"]["paths"]["maxItems"],
            audiorouter_domain::MAX_ROUTE_PATHS
        );
        let handshake = methods
            .iter()
            .find(|method| method["name"] == "system.handshake")
            .unwrap();
        assert_eq!(
            handshake["outputSchema"]["properties"]["negotiated"]["properties"]["major"]["const"],
            1
        );
        let describe = methods
            .iter()
            .find(|method| method["name"] == "system.describe")
            .unwrap();
        assert_eq!(
            describe["outputSchema"]["properties"]["methods"]["type"],
            "array"
        );
        assert_eq!(
            describe["outputSchema"]["properties"]["methods"]["maxItems"],
            API_METHODS.len()
        );
        assert_eq!(
            describe["outputSchema"]["properties"]["nodeTypes"]["maxItems"],
            audiorouter_domain::node_registry().len()
        );
        assert_eq!(
            describe["outputSchema"]["properties"]["processors"]["maxItems"],
            MAX_PROCESSOR_CATALOG_ITEMS
        );
        assert_eq!(
            describe["outputSchema"]["properties"]["presets"]["properties"]["voiceChains"]
                ["maxItems"],
            audiorouter_dsp::VoiceChainPresetId::ALL.len()
        );
        assert_eq!(
            describe["outputSchema"]["properties"]["presets"]["properties"]["eq"]["maxItems"],
            audiorouter_dsp::EqPresetId::ALL.len()
        );
        assert_eq!(
            describe["outputSchema"]["properties"]["events"]["properties"]["meterReplay"]["const"],
            false
        );
        let session_create = methods
            .iter()
            .find(|method| method["name"] == "sessions.create")
            .unwrap();
        assert_eq!(
            session_create["outputSchema"]["properties"]["session"]["properties"]["nodes"]["type"],
            "array"
        );
        let session_start = methods
            .iter()
            .find(|method| method["name"] == "session.start")
            .unwrap();
        assert_eq!(
            session_start["outputSchema"]["properties"]["generation"]["minimum"],
            1
        );
        let undo = methods
            .iter()
            .find(|method| method["name"] == "graph.undoPlan")
            .unwrap();
        assert_eq!(
            undo["outputSchema"]["required"],
            json!(["planId", "baseRevision", "expiresInMs"])
        );
        let privacy = methods
            .iter()
            .find(|method| method["name"] == "safety.setPrivacyMute")
            .unwrap();
        assert_eq!(
            privacy["outputSchema"]["properties"]["muted"]["type"],
            "boolean"
        );
        let authorize = methods
            .iter()
            .find(|method| method["name"] == "clients.authorize")
            .unwrap();
        assert_eq!(
            authorize["outputSchema"]["properties"]["revoked"]["const"],
            false
        );
        let metadata = methods
            .iter()
            .find(|method| method["name"] == "recordings.setMetadata")
            .unwrap();
        assert_eq!(
            metadata["outputSchema"]["required"],
            json!(["recordingId", "updated"])
        );
        let rename = methods
            .iter()
            .find(|method| method["name"] == "recordings.rename")
            .unwrap();
        assert_eq!(
            rename["outputSchema"]["properties"]["fileAction"]["const"],
            "renamed"
        );
        let reveal = methods
            .iter()
            .find(|method| method["name"] == "recordings.reveal")
            .unwrap();
        assert_eq!(reveal["outputSchema"]["oneOf"].as_array().unwrap().len(), 2);
        let preview = methods
            .iter()
            .find(|method| method["name"] == "recordings.preview")
            .unwrap();
        assert_eq!(
            preview["outputSchema"]["properties"]["preview"]["oneOf"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        let recycle = methods
            .iter()
            .find(|method| method["name"] == "recordings.recycle")
            .unwrap();
        assert_eq!(
            recycle["outputSchema"]["oneOf"].as_array().unwrap().len(),
            4
        );
        let history = methods
            .iter()
            .find(|method| method["name"] == "graph.history")
            .unwrap();
        assert_eq!(
            history["outputSchema"]["properties"]["items"]["type"],
            "array"
        );
        let routes = methods
            .iter()
            .find(|method| method["name"] == "routes.inspect")
            .unwrap();
        assert_eq!(
            routes["outputSchema"]["properties"]["reachable"]["type"],
            "boolean"
        );
        assert_eq!(
            routes["outputSchema"]["properties"]["paths"]["items"]["required"],
            json!(["nodes", "edges", "channelMaps", "latencySamples"])
        );
        assert_eq!(
            routes["outputSchema"]["properties"]["complete"]["type"],
            "boolean"
        );
        let plugins = methods
            .iter()
            .find(|method| method["name"] == "plugins.scan")
            .unwrap();
        assert_eq!(
            plugins["outputSchema"]["properties"]["entries"]["maxItems"],
            audiorouter_plugin_host::MAX_SCAN_CANDIDATES
        );
        let node_types = methods
            .iter()
            .find(|method| method["name"] == "nodes.types")
            .unwrap();
        assert_eq!(
            node_types["outputSchema"]["maxItems"],
            audiorouter_domain::node_registry().len()
        );
        let processors = methods
            .iter()
            .find(|method| method["name"] == "processors.list")
            .unwrap();
        assert_eq!(
            processors["outputSchema"]["maxItems"],
            MAX_PROCESSOR_CATALOG_ITEMS
        );
        let presets = methods
            .iter()
            .find(|method| method["name"] == "presets.list")
            .unwrap();
        assert_eq!(
            presets["outputSchema"]["properties"]["voiceChains"]["maxItems"],
            audiorouter_dsp::VoiceChainPresetId::ALL.len()
        );
        assert_eq!(
            presets["outputSchema"]["properties"]["eq"]["maxItems"],
            audiorouter_dsp::EqPresetId::ALL.len()
        );
        assert_eq!(
            routes["outputSchema"]["properties"]["paths"]["items"]["properties"]["nodes"]
                ["maxItems"],
            audiorouter_domain::MAX_NODES_PER_SESSION
        );
        assert_eq!(
            routes["outputSchema"]["properties"]["paths"]["items"]["properties"]["edges"]
                ["maxItems"],
            audiorouter_domain::MAX_EDGES_PER_SESSION
        );
        assert_eq!(
            routes["outputSchema"]["properties"]["paths"]["items"]["properties"]["channelMaps"]
                ["items"]["maxItems"],
            audiorouter_domain::MAX_CHANNEL_MATRIX_COEFFICIENTS
        );
        let plan = methods
            .iter()
            .find(|method| method["name"] == "graph.plan")
            .unwrap();
        assert_eq!(
            plan["outputSchema"]["properties"]["expiresInMs"]["minimum"],
            1
        );
        assert_eq!(
            plan["outputSchema"]["properties"]["warnings"]["type"],
            "array"
        );
        let commit = methods
            .iter()
            .find(|method| method["name"] == "graph.commit")
            .unwrap();
        for method_name in [
            "sessions.delete",
            "session.start",
            "sessions.start",
            "session.stop",
            "sessions.stop",
            "graph.commit",
        ] {
            let method = methods
                .iter()
                .find(|method| method["name"] == method_name)
                .unwrap();
            assert_eq!(
                method["outputSchema"]["properties"]["sessionId"]["maxLength"],
                audiorouter_domain::MAX_ENTITY_ID_BYTES,
                "{method_name} output sessionId bound"
            );
        }
        assert_eq!(
            commit["outputSchema"]["properties"]["revision"]["minimum"],
            0
        );
        let operation = methods
            .iter()
            .find(|method| method["name"] == "operations.get")
            .unwrap();
        assert_eq!(
            operation["outputSchema"]["oneOf"][1]["properties"]["status"]["const"],
            "unknown"
        );
        let cancel = methods
            .iter()
            .find(|method| method["name"] == "operations.cancel")
            .unwrap();
        assert_eq!(
            cancel["outputSchema"]["properties"]["reason"]["const"],
            "alreadyCompleted"
        );
        let applications = methods
            .iter()
            .find(|method| method["name"] == "applications.list")
            .unwrap();
        assert_eq!(
            applications["outputSchema"]["items"]["properties"]["audioActivity"]["enum"][0],
            "active"
        );
        assert_eq!(
            applications["outputSchema"]["items"]["properties"]["captureSessionCount"]["type"],
            "integer"
        );
        assert_eq!(
            applications["outputSchema"]["items"]["properties"]["renderSessionCount"]["type"],
            "integer"
        );
        assert_eq!(
            applications["outputSchema"]["items"]["properties"]["executable"]["maxLength"],
            260
        );
        assert_eq!(
            applications["outputSchema"]["items"]["properties"]["executablePath"]["maxLength"],
            32_768
        );
        assert_eq!(
            applications["outputSchema"]["maxItems"],
            audiorouter_windows_audio::MAX_APPLICATIONS
        );
        assert_eq!(
            applications["outputSchema"]["items"]["properties"]["audioDisplayNames"]["maxItems"],
            audiorouter_windows_audio::MAX_APPLICATION_AUDIO_DISPLAY_NAMES
        );
        assert_eq!(
            applications["outputSchema"]["items"]["properties"]["audioDisplayNames"]["items"]
                ["maxLength"],
            audiorouter_windows_audio::MAX_APPLICATION_AUDIO_DISPLAY_NAME_BYTES
        );
        let recordings = methods
            .iter()
            .find(|method| method["name"] == "recordings.list")
            .unwrap();
        assert_eq!(
            recordings["outputSchema"]["oneOf"][1]["properties"]["items"]["items"]["properties"]
                ["format"]["enum"],
            json!(["wav", "flac"])
        );
        assert_eq!(
            recordings["outputSchema"]["oneOf"][1]["properties"]["nextCursor"]["type"],
            json!(["string", "null"])
        );
        assert_eq!(
            recordings["outputSchema"]["oneOf"][1]["properties"]["items"]["maxItems"],
            MAX_RECORDING_LIST_ITEMS
        );
        assert_eq!(
            recordings["outputSchema"]["oneOf"][0]["maxItems"],
            MAX_RECORDING_LIST_ITEMS
        );
        let recording = methods
            .iter()
            .find(|method| method["name"] == "recordings.get")
            .unwrap();
        assert_eq!(
            recording["outputSchema"]["properties"]["sampleRate"]["enum"],
            json!([44100, 48000])
        );
        for method_name in [
            "recordings.get",
            "recordings.recovery",
            "recordings.preview",
            "recordings.setMetadata",
            "recordings.rename",
            "recordings.reveal",
            "recordings.recycle",
            "recordings.removeEntry",
        ] {
            let method = methods
                .iter()
                .find(|method| method["name"] == method_name)
                .unwrap();
            let schema = &method["outputSchema"];
            let schema = if method_name == "recordings.get"
                || method_name == "recordings.preview"
                || method_name == "recordings.setMetadata"
                || method_name == "recordings.rename"
                || method_name == "recordings.removeEntry"
            {
                schema
            } else {
                &schema["oneOf"][0]
            };
            let field = if method_name == "recordings.get" {
                "id"
            } else {
                "recordingId"
            };
            assert_eq!(
                schema["properties"][field]["maxLength"],
                audiorouter_storage::MAX_RECORDING_ID_BYTES,
                "{method_name} output identity bound"
            );
        }
    }

    #[test]
    fn dispatch_rejects_parameters_outside_discovered_schema() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "sessions.list".into(),
            params: Some(json!({ "unexpected": true })),
        });
        assert_eq!(response.error.unwrap().code, -32602);

        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "devices.list".into(),
            params: Some(json!([])),
        });
        assert_eq!(response.error.unwrap().code, -32602);
    }

    #[test]
    fn dispatch_rejects_overdeep_and_oversized_parameter_values() {
        let mut nested = json!("leaf");
        for _ in 0..=MAX_CONTROL_VALUE_DEPTH {
            nested = json!({ "nested": nested });
        }
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "sessions.list".into(),
            params: Some(nested),
        });
        assert_eq!(response.error.unwrap().code, -32602);

        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "sessions.list".into(),
            params: Some(json!({ "cursor": "x".repeat(MAX_CONTROL_STRING_BYTES + 1) })),
        });
        assert_eq!(response.error.unwrap().code, -32602);

        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "sessions.list".into(),
            params: Some(json!({ "values": vec![json!(null); MAX_CONTROL_VALUE_COUNT] })),
        });
        assert_eq!(response.error.unwrap().code, -32602);
    }

    #[test]
    fn devices_list_rejects_invalid_paging_parameters_before_enumeration() {
        let mut plane = ControlPlane::default();
        for (id, params) in [
            (1, json!({ "limit": 0 })),
            (2, json!({ "limit": 501 })),
            (3, json!({ "cursor": 42 })),
        ] {
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(id)),
                method: "devices.list".into(),
                params: Some(params),
            });
            assert_eq!(response.error.unwrap().code, -32602);
        }
    }

    #[test]
    fn canonical_application_list_alias_uses_the_same_discovery_result() {
        let mut plane = ControlPlane::default();
        let legacy = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(5)),
            method: "apps.list".into(),
            params: None,
        });
        let canonical = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(6)),
            method: "applications.list".into(),
            params: None,
        });
        assert_eq!(legacy.result, canonical.result);
        let applications = legacy.result.unwrap();
        assert!(applications.as_array().unwrap().iter().all(|application| {
            application.get("processId").is_some()
                && application.get("executable").is_some()
                && application.get("executablePath").is_some()
                && application.get("audioActivity").is_some()
                && application.get("captureCapability").is_some()
                && application.get("audioSessionCount").is_some()
                && application.get("activeAudioSessionCount").is_some()
                && application.get("captureSessionCount").is_some()
                && application.get("renderSessionCount").is_some()
                && application.get("audioDisplayNames").is_some()
        }));
    }

    #[test]
    fn nullable_optional_parameters_are_treated_as_omitted() {
        let mut plane = ControlPlane::default();
        let mut source = session();
        source.id = EntityId::new("source");
        plane.insert_session(source).unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(7)),
            method: "sessions.duplicate".into(),
            params: Some(json!({
                "sourceSessionId": "source",
                "sessionId": "copy",
                "name": null,
                "idempotencyKey": "duplicate-null-name"
            })),
        });
        assert!(response.result.is_some());
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(8)),
            method: "sessions.list".into(),
            params: Some(json!({ "cursor": null })),
        });
        assert!(response.result.is_some());
    }

    #[test]
    fn diagnostics_are_redacted_and_report_backend_state() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(9)),
            method: "system.diagnostics".into(),
            params: None,
        });
        let result = response.result.unwrap();
        assert_eq!(result["backend"], "control-plane");
        assert_eq!(result["storage"], "memory");
        assert_eq!(result["nativeAdapter"], "implemented-not-activated");
        assert_eq!(result["redacted"], true);
        assert!(result.get("path").is_none());
    }

    #[test]
    fn status_snapshot_tracks_sessions_and_event_cursor() {
        let mut plane = ControlPlane::default();
        let initial = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(15)),
            method: "status.get".into(),
            params: None,
        });
        assert_eq!(initial.result.as_ref().unwrap()["sessionCount"], 0);
        let value = session();
        plane.insert_session(value.clone()).unwrap();
        let status = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(16)),
            method: "status.get".into(),
            params: None,
        });
        let result = status.result.unwrap();
        assert_eq!(result["sessionCount"], 1);
        assert_eq!(result["activeSessionCount"], 0);
        assert_eq!(
            result["reason"],
            "native endpoint routing is implemented but not activated; exact bindings and a production driver are required"
        );
        assert_eq!(result["eventCursor"]["latestSequence"], 1);
    }

    #[test]
    fn storage_backed_virtual_bus_inventory_survives_control_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-virtual-bus-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        {
            let mut plane =
                ControlPlane::with_storage("virtual-bus-first", Storage::open(&path).unwrap());
            plane
                .create_virtual_bus(EntityId::new("bus-1"), "Desktop In")
                .unwrap();
            plane
                .set_virtual_bus_enabled(&EntityId::new("bus-1"), false)
                .unwrap();
        }
        let mut restarted =
            ControlPlane::with_storage("virtual-bus-second", Storage::open(&path).unwrap());
        let result = restarted
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "virtualDevices.list".into(),
                params: None,
            })
            .result
            .unwrap();
        assert_eq!(result[0]["id"], "bus-1");
        assert_eq!(result[0]["enabled"], false);
        assert_eq!(result[0]["leaseOwner"], Value::Null);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn runtime_crash_recording_returns_one_bounded_memory_recovery_decision() {
        let mut plane = ControlPlane::default();
        let value = session();
        let session_id = value.id.clone();
        plane.insert_session(value).unwrap();
        plane.session_start(&session_id).unwrap();

        let first = plane.record_runtime_crash(100).unwrap();
        assert_eq!(first.mode, RecoveryMode::RestoreEligible);
        assert_eq!(first.session_ids, vec![session_id.clone()]);

        plane.record_runtime_crash(101).unwrap();
        let third = plane.record_runtime_crash(102).unwrap();
        assert_eq!(third.mode, RecoveryMode::SafeMode);
        assert!(third.session_ids.is_empty());
    }

    #[test]
    fn runtime_crash_recovery_restarts_only_eligible_fake_sessions() {
        let mut plane = ControlPlane::default();
        let mut running = session();
        running.id = EntityId::new("running");
        let mut recording = session();
        recording.id = EntityId::new("recording");
        plane.insert_session(running).unwrap();
        plane.insert_session(recording).unwrap();
        plane.session_start(&EntityId::new("running")).unwrap();
        plane.session_start(&EntityId::new("recording")).unwrap();

        let recorder = RecorderController::new();
        plane.recorders.insert(EntityId::new("recording"), recorder);
        plane
            .recorders
            .get_mut(&EntityId::new("recording"))
            .unwrap()
            .arm()
            .unwrap();
        plane
            .recorders
            .get_mut(&EntityId::new("recording"))
            .unwrap()
            .start(0)
            .unwrap();

        let decision = plane.recover_after_runtime_crash(100).unwrap();
        assert_eq!(decision.mode, RecoveryMode::RestoreEligible);
        assert_eq!(decision.session_ids, vec![EntityId::new("running")]);
        let status = plane.status_snapshot().unwrap();
        assert_eq!(status["activeSessionIds"], json!(["running"]));
        let events = plane.events.since(0, 500).unwrap();
        let categories = events
            .iter()
            .map(|event| event.category.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            &categories[2..],
            [
                "runtime.started",
                "runtime.started",
                "runtime.crashed",
                "runtime.crashed",
                "runtime.started"
            ]
        );
    }

    #[test]
    fn runtime_crash_recovery_enters_safe_mode_without_restarting_sessions() {
        let mut plane = ControlPlane::default();
        let value = session();
        let session_id = value.id.clone();
        plane.insert_session(value).unwrap();
        plane.session_start(&session_id).unwrap();
        plane.recover_after_runtime_crash(100).unwrap();
        plane.session_start(&session_id).unwrap();
        plane.recover_after_runtime_crash(101).unwrap();
        plane.session_start(&session_id).unwrap();

        let decision = plane.recover_after_runtime_crash(102).unwrap();
        assert_eq!(decision.mode, RecoveryMode::SafeMode);
        assert!(decision.session_ids.is_empty());
        assert_eq!(
            plane.status_snapshot().unwrap()["activeSessionIds"],
            json!([])
        );
    }

    #[test]
    fn durable_runtime_crash_recovery_applies_policy_and_keeps_safe_mode_latched() {
        let storage = Storage::open_memory().unwrap();
        let mut plane = ControlPlane::with_storage("durable-recovery", storage);
        let value = session();
        let session_id = value.id.clone();
        plane.insert_session(value).unwrap();
        plane.session_start(&session_id).unwrap();

        let first = plane.recover_after_runtime_crash(100).unwrap();
        assert_eq!(first.mode, RecoveryMode::RestoreEligible);
        assert_eq!(
            plane.status_snapshot().unwrap()["activeSessionIds"],
            json!([session_id.as_str()])
        );
        plane.recover_after_runtime_crash(101).unwrap();
        plane.session_start(&session_id).unwrap();
        let third = plane.recover_after_runtime_crash(102).unwrap();
        assert_eq!(third.mode, RecoveryMode::SafeMode);
        assert_eq!(
            plane.status_snapshot().unwrap()["recovery"]["safeMode"],
            true
        );
        assert_eq!(
            plane.status_snapshot().unwrap()["activeSessionIds"],
            json!([])
        );
    }

    #[test]
    fn runtime_crash_recovery_candidates_are_sorted() {
        let mut plane = ControlPlane::default();
        let mut first = session();
        first.id = EntityId::new("z-session");
        let mut second = session();
        second.id = EntityId::new("a-session");
        plane.insert_session(first).unwrap();
        plane.insert_session(second).unwrap();
        plane.session_start(&EntityId::new("z-session")).unwrap();
        plane.session_start(&EntityId::new("a-session")).unwrap();

        let decision = plane.record_runtime_crash(100).unwrap();
        assert_eq!(
            decision.session_ids,
            vec![EntityId::new("a-session"), EntityId::new("z-session")]
        );
    }

    #[test]
    fn clearing_memory_safe_mode_resets_the_crash_tracker() {
        let mut plane = ControlPlane::default();
        plane.record_runtime_crash(100).unwrap();
        plane.record_runtime_crash(101).unwrap();
        assert_eq!(
            plane.record_runtime_crash(102).unwrap().mode,
            RecoveryMode::SafeMode
        );

        let cleared = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(88)),
            method: "recovery.clearSafeMode".into(),
            params: Some(json!({ "idempotencyKey": "recovery-clear-1" })),
        });
        assert_eq!(cleared.result.as_ref().unwrap()["safeMode"], false);
        let replay = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(89)),
            method: "recovery.clearSafeMode".into(),
            params: Some(json!({ "idempotencyKey": "recovery-clear-1" })),
        });
        assert_eq!(replay.result.unwrap(), cleared.result.unwrap());
        let decision = plane.record_runtime_crash(103).unwrap();
        assert_eq!(decision.mode, RecoveryMode::RestoreEligible);
    }

    #[test]
    fn runtime_crash_recording_persists_the_safe_mode_decision() {
        let storage = Storage::open_memory().unwrap();
        let mut plane = ControlPlane::with_storage("recovery-supervisor", storage);
        let value = session();
        let session_id = value.id.clone();
        plane.insert_session(value).unwrap();
        plane.session_start(&session_id).unwrap();

        let now = unix_epoch_seconds() as u64;
        plane.record_runtime_crash(now).unwrap();
        plane.record_runtime_crash(now + 1).unwrap();
        let decision = plane.record_runtime_crash(now + 2).unwrap();
        assert_eq!(decision.mode, RecoveryMode::SafeMode);
        assert!(decision.session_ids.is_empty());
        let status = plane.status_snapshot().unwrap();
        assert_eq!(status["recovery"]["safeMode"], true);
        assert_eq!(status["recovery"]["recentCrashes"], 3);
    }

    #[test]
    fn status_and_diagnostics_expose_persisted_recovery_safe_mode() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-recovery-status-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let storage = Storage::open(&path).unwrap();
        let timestamp = unix_epoch_seconds() as u64;
        for offset in 0..3 {
            storage.record_recovery_crash(timestamp + offset).unwrap();
        }
        let mut plane = ControlPlane::with_storage("recovery", storage);
        let status = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(17)),
            method: "status.get".into(),
            params: None,
        });
        let status_result = status.result.unwrap();
        assert_eq!(status_result["recovery"]["safeMode"], true);
        assert_eq!(status_result["recovery"]["recentCrashes"], 3);
        let diagnostics = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(18)),
            method: "system.diagnostics".into(),
            params: None,
        });
        let diagnostics_result = diagnostics.result.unwrap();
        assert_eq!(diagnostics_result["recovery"]["safeMode"], true);
        assert_eq!(diagnostics_result["recovery"]["recentCrashes"], 3);
        let cleared = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(19)),
            method: "recovery.clearSafeMode".into(),
            params: Some(json!({ "idempotencyKey": "recovery-clear-2" })),
        });
        assert_eq!(cleared.result.unwrap()["safeMode"], false);
        let status_after = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(20)),
            method: "status.get".into(),
            params: None,
        });
        assert_eq!(status_after.result.unwrap()["recovery"]["recentCrashes"], 0);
        drop(plane);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn privacy_mute_is_authorized_and_durable_across_control_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-privacy-mute-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let mut first = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
        let enabled = first.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(19)),
            method: "safety.setPrivacyMute".into(),
            params: Some(json!({ "muted": true, "idempotencyKey": "privacy-1" })),
        });
        assert_eq!(enabled.result.unwrap()["muted"], true);
        let denied = first.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(20)),
                method: "safety.setPrivacyMute".into(),
                params: Some(json!({ "muted": false })),
            },
            &ClientGrant::read_only(),
        );
        assert_eq!(denied.error.unwrap().code, -32001);
        drop(first);
        let mut second = ControlPlane::with_storage("second", Storage::open(&path).unwrap());
        let status = second
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(21)),
                method: "status.get".into(),
                params: None,
            })
            .result
            .unwrap();
        assert_eq!(status["privacyMute"]["muted"], true);
        let replay = second.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(22)),
            method: "safety.setPrivacyMute".into(),
            params: Some(json!({ "muted": true, "idempotencyKey": "privacy-1" })),
        });
        assert_eq!(replay.result.unwrap()["muted"], true);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn client_enrollment_api_lists_authorizes_and_revokes() {
        let mut plane = ControlPlane::default();
        let grant = ClientGrant::with_scopes([PermissionScope::DeviceAdministration]);
        let authorize = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(10)),
                method: "clients.authorize".into(),
                params: Some(json!({
                    "clientId": "desktop",
                    "role": "editor",
                    "idempotencyKey": "authorize-desktop-1"
                })),
            },
            &grant,
        );
        assert_eq!(authorize.result.unwrap()["revoked"], false);
        let listed = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(11)),
            method: "clients.list".into(),
            params: None,
        });
        assert_eq!(listed.result.unwrap()[0]["clientId"], "desktop");
        let revoked = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(12)),
                method: "clients.revoke".into(),
                params: Some(json!({
                    "clientId": "desktop",
                    "idempotencyKey": "revoke-desktop-1"
                })),
            },
            &grant,
        );
        assert_eq!(revoked.result.unwrap()["changed"], true);
        let oversized = "x".repeat(audiorouter_domain::MAX_ENTITY_ID_BYTES + 1);
        assert!(plane
            .enroll_client(oversized.clone(), ClientRole::Observer)
            .is_err());
        assert!(plane.revoke_client(&oversized).is_err());
    }

    #[test]
    fn handshake_negotiates_minor_versions_and_rejects_unknown_major() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "system.handshake".into(),
            params: Some(json!({ "protocolVersion": { "major": 1, "minor": 99 } })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["compatible"], true);
        assert_eq!(result["negotiated"], json!({ "major": 1, "minor": 0 }));

        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "system.handshake".into(),
            params: Some(json!({ "protocolVersion": { "major": 2, "minor": 0 } })),
        });
        assert_eq!(response.error.unwrap().code, -32602);
    }

    #[test]
    fn handshake_golden_fixture_matches_dispatch_response() {
        let request: JsonRpcRequest = serde_json::from_str(include_str!(
            "../../../tests/fixtures/system-handshake-request.json"
        ))
        .unwrap();
        let expected: JsonRpcResponse = serde_json::from_str(include_str!(
            "../../../tests/fixtures/system-handshake-response.json"
        ))
        .unwrap();
        assert_eq!(ControlPlane::default().dispatch(request), expected);
    }

    #[test]
    fn routes_inspect_dispatch_returns_desired_provenance() {
        let mut plane = ControlPlane::default();
        let graph = session();
        plane.insert_session(graph).unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "routes.inspect".into(),
            params: Some(json!({
                "sessionId": "session",
                "destinationNode": "out"
            })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["reachable"], true);
        assert_eq!(result["paths"][0]["nodes"], json!(["in", "out"]));
        assert_eq!(result["paths"][0]["edges"], json!(["edge"]));
        assert_eq!(result["paths"][0]["channelMaps"], json!([[1.0]]));
        assert_eq!(result["paths"][0]["latencySamples"], 0);
    }

    #[test]
    fn graph_history_dispatch_returns_newest_snapshot_first() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "revision-one".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "history-api").unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "graph.history".into(),
            params: Some(json!({ "sessionId": "session", "limit": 1 })),
        });
        let history = response.result.unwrap();
        assert_eq!(history["items"].as_array().unwrap().len(), 1);
        assert_eq!(history["items"][0]["revision"], 1);
        assert_eq!(history["items"][0]["name"], "revision-one");
        assert_eq!(history["nextCursor"], "1");
        let oversized = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "graph.history".into(),
            params: Some(json!({
                "sessionId": "session",
                "cursor": "9".repeat(MAX_REVISION_CURSOR_BYTES + 1)
            })),
        });
        assert_eq!(oversized.error.unwrap().code, -32602);
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "graph.history".into(),
            params: Some(json!({ "sessionId": "session", "cursor": "1", "limit": 1 })),
        });
        let history = response.result.unwrap();
        assert_eq!(history["items"][0]["revision"], 0);
        assert!(history["nextCursor"].is_null());
    }

    #[test]
    fn graph_undo_plan_dispatches_through_revision_checked_planning() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "revision-one".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "undo-api").unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "graph.undoPlan".into(),
            params: Some(json!({ "sessionId": "session", "baseRevision": 1 })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["baseRevision"], 1);
        assert!(result["planId"].as_str().unwrap().starts_with("plan-"));
    }

    #[test]
    fn graph_undo_plan_hydrates_prior_history_after_control_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-undo-restart-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        {
            let mut plane = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
            let original = session();
            plane.insert_session(original.clone()).unwrap();
            let mut candidate = original.clone();
            candidate.name = "persisted-edit".into();
            let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
            plane.commit_graph(&plan, 0, "restart-undo").unwrap();
        }
        let mut restarted = ControlPlane::with_storage("restarted", Storage::open(&path).unwrap());
        let response = restarted.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(5)),
            method: "graph.undoPlan".into(),
            params: Some(json!({ "sessionId": "session", "baseRevision": 1 })),
        });
        assert!(response.result.unwrap()["planId"].as_str().is_some());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn events_subscribe_replays_filtered_control_state() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "evented".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "event-commit").unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "events.subscribe".into(),
            params: Some(json!({ "sessionId": "session", "afterSequence": 0 })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["backendEpoch"], 1);
        assert_eq!(result["events"].as_array().unwrap().len(), 2);
        assert_eq!(result["events"][1]["operationId"], "event-commit");
    }

    #[test]
    fn endpoint_change_signal_replays_through_event_cursor() {
        let mut plane = ControlPlane::default();
        plane.record_endpoint_changes(false);
        plane.record_endpoint_changes(true);
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(44)),
            method: "events.subscribe".into(),
            params: Some(json!({
                "afterSequence": 0,
                "categories": ["devices.changed"]
            })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["events"].as_array().unwrap().len(), 1);
        assert_eq!(result["events"][0]["category"], "devices.changed");
        assert_eq!(result["events"][0]["resourceRevision"], 0);
    }

    #[test]
    fn events_subscribe_page_cursor_does_not_skip_retained_events() {
        let mut plane = ControlPlane::default();
        for index in 0..=500 {
            plane.events.append(
                index,
                Some(format!("page-{index}")),
                "state.test",
                Some(EntityId::new("session")),
            );
        }

        let first = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(40)),
            method: "events.subscribe".into(),
            params: Some(json!({ "afterSequence": 0, "limit": 500 })),
        });
        let first_result = first.result.unwrap();
        assert_eq!(first_result["events"].as_array().unwrap().len(), 500);
        assert_eq!(first_result["nextSequence"], 500);

        let second = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(41)),
            method: "events.subscribe".into(),
            params: Some(json!({ "afterSequence": 500, "limit": 500 })),
        });
        let second_result = second.result.unwrap();
        assert_eq!(second_result["events"].as_array().unwrap().len(), 1);
        assert_eq!(second_result["events"][0]["resourceRevision"], 500);
        assert_eq!(second_result["events"][0]["operationId"], "page-500");
        assert_eq!(second_result["nextSequence"], 501);
    }

    #[test]
    fn events_subscribe_filters_by_category_and_rejects_unbounded_filters() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "evented".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "category-filter").unwrap();

        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(6)),
            method: "events.subscribe".into(),
            params: Some(json!({
                "afterSequence": 0,
                "categories": ["graph.committed"]
            })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["events"].as_array().unwrap().len(), 1);
        assert_eq!(result["events"][0]["category"], "graph.committed");

        let too_many_categories = vec!["state.test"; 33];
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(7)),
            method: "events.subscribe".into(),
            params: Some(json!({
                "categories": too_many_categories
            })),
        });
        assert_eq!(response.error.unwrap().code, -32602);
    }

    #[test]
    fn startup_get_reports_unavailable_without_side_effects() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "startup.get".into(),
            params: None,
        });
        let result = response.result.unwrap();
        assert_eq!(result["enabled"], false);
        assert_eq!(result["registration"], "unavailable");
        assert_eq!(
            result["reason"],
            "sign-in startup registration is not implemented in this build"
        );
    }

    #[test]
    fn startup_plan_and_apply_are_explicitly_fail_closed() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "startup.plan".into(),
            params: Some(json!({ "enabled": true })),
        });
        let plan = response.result.unwrap();
        assert_eq!(plan["enabled"], true);
        assert_eq!(plan["registration"], "unavailable");
        assert_eq!(plan["requiredScopes"], json!(["sessionControl"]));
        let plan_id = plan["planId"].as_str().unwrap().to_owned();

        let apply = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "startup.apply".into(),
            params: Some(json!({ "planId": plan_id, "idempotencyKey": "startup-1" })),
        });
        let result = apply.result.unwrap();
        assert_eq!(result["state"], "unavailable");
        assert_eq!(result["registration"], "unavailable");
        assert_eq!(
            result["reason"],
            "sign-in startup registration is not implemented in this build"
        );
    }

    #[test]
    fn storage_backed_startup_plan_survives_control_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-startup-plan-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let plan_id = {
            let mut plane =
                ControlPlane::with_storage("startup-first", Storage::open(&path).unwrap());
            plane
                .dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(1)),
                    method: "startup.plan".into(),
                    params: Some(json!({ "enabled": true })),
                })
                .result
                .unwrap()["planId"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        let mut restarted =
            ControlPlane::with_storage("startup-second", Storage::open(&path).unwrap());
        let applied = restarted.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "startup.apply".into(),
            params: Some(json!({ "planId": plan_id, "idempotencyKey": "startup-restart" })),
        });
        assert_eq!(applied.result.unwrap()["state"], "unavailable");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn events_subscribe_returns_snapshot_when_cursor_expired() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        for _ in 0..=audiorouter_domain::MAX_RETAINED_EVENTS {
            plane.events.append(0, None, "state.test", None);
        }
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(5)),
            method: "events.subscribe".into(),
            params: Some(json!({ "afterSequence": 1 })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["resyncRequired"], true);
        assert_eq!(
            result["snapshot"]["sessions"]["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(result["events"].as_array().unwrap().is_empty());
    }

    #[test]
    fn events_subscribe_requires_resync_when_backend_epoch_changes() {
        let mut plane = ControlPlane::default();
        plane.insert_session(session()).unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(6)),
            method: "events.subscribe".into(),
            params: Some(json!({ "backendEpoch": 999, "afterSequence": 0 })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["resyncRequired"], true);
        assert_eq!(result["reason"], "backendEpochChanged");
        assert_eq!(result["backendEpoch"], 1);
        assert_eq!(
            result["snapshot"]["sessions"]["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn durable_control_restart_advances_backend_epoch() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-control-epoch-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let first_epoch = {
            let mut plane = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
            plane
                .dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(1)),
                    method: "status.get".into(),
                    params: None,
                })
                .result
                .unwrap()["eventCursor"]["backendEpoch"]
                .as_u64()
                .unwrap()
        };
        let second_epoch = {
            let mut plane = ControlPlane::with_storage("second", Storage::open(&path).unwrap());
            plane
                .dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(2)),
                    method: "status.get".into(),
                    params: None,
                })
                .result
                .unwrap()["eventCursor"]["backendEpoch"]
                .as_u64()
                .unwrap()
        };
        assert_eq!(second_epoch, first_epoch + 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn recordings_list_dispatch_exposes_storage_metadata_without_file_actions() {
        let storage = Storage::open_memory().unwrap();
        storage
            .save_recording(&audiorouter_storage::RecordingRecord {
                id: "recording-1".into(),
                session_id: "session".into(),
                recorder_id: "recorder".into(),
                path: "C:\\recordings\\one.wav".into(),
                format: "wav".into(),
                channels: 2,
                sample_rate: 48_000,
                frames: 96_000,
                file_bytes: 384_000,
                start_time: "2026-09-06T00:00:00Z".into(),
                state: "completed".into(),
                missing: false,
                title: Some("Test".into()),
                artist: None,
                comment: None,
            })
            .unwrap();
        storage
            .save_recording(&audiorouter_storage::RecordingRecord {
                id: "recording-2".into(),
                session_id: "session".into(),
                recorder_id: "recorder".into(),
                path: "C:\\recordings\\two.wav".into(),
                format: "wav".into(),
                channels: 2,
                sample_rate: 48_000,
                frames: 48_000,
                file_bytes: 192_000,
                start_time: "2026-09-06T01:00:00Z".into(),
                state: "completed".into(),
                missing: false,
                title: None,
                artist: None,
                comment: None,
            })
            .unwrap();
        let mut plane = ControlPlane::with_storage("recordings", storage);
        let denied = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(7)),
                method: "recordings.list".into(),
                params: Some(json!({ "sessionId": "session" })),
            },
            &ClientGrant::read_only(),
        );
        assert_eq!(
            denied.error.unwrap().data.unwrap()["code"],
            "permissionDenied"
        );
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(8)),
            method: "recordings.list".into(),
            params: Some(json!({ "sessionId": "session" })),
        });
        let result = response.result.unwrap();
        assert_eq!(result.as_array().unwrap().len(), 2);
        assert_eq!(result[0]["id"], "recording-1");
        assert_eq!(result[0]["missing"], false);
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(10)),
            method: "recordings.list".into(),
            params: Some(json!({ "sessionId": "session", "limit": 1 })),
        });
        let page = response.result.unwrap();
        assert_eq!(page["items"][0]["id"], "recording-1");
        assert_eq!(page["nextCursor"], "recording-1");
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(11)),
            method: "recordings.list".into(),
            params: Some(json!({
                "sessionId": "session",
                "cursor": "recording-1",
                "limit": 1
            })),
        });
        let page = response.result.unwrap();
        assert_eq!(page["items"][0]["id"], "recording-2");
        assert_eq!(page["nextCursor"], Value::Null);
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(9)),
            method: "recordings.get".into(),
            params: Some(json!({ "recordingId": "recording-1" })),
        });
        assert_eq!(response.result.unwrap()["title"], "Test");
    }

    #[test]
    fn recording_recovery_dispatch_returns_validated_checkpoint_or_missing() {
        let storage = Storage::open_memory().unwrap();
        let mut recorder = audiorouter_recording::RecorderController::new();
        recorder.arm().unwrap();
        recorder.start(100).unwrap();
        storage
            .save_recording_checkpoint("recovery-recording", &recorder.checkpoint())
            .unwrap();
        let mut plane = ControlPlane::with_storage("recovery", storage);
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "recordings.recovery".into(),
            params: Some(json!({ "recordingId": "recovery-recording" })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["status"], "available");
        assert_eq!(result["checkpoint"]["state"], "Recording");
        let missing = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "recordings.recovery".into(),
            params: Some(json!({ "recordingId": "missing" })),
        });
        assert_eq!(missing.result.unwrap()["status"], "missing");
    }

    #[test]
    fn recording_recovery_without_id_lists_bounded_checkpoint_entries() {
        let storage = Storage::open_memory().unwrap();
        let mut recorder = audiorouter_recording::RecorderController::new();
        recorder.arm().unwrap();
        recorder.start(100).unwrap();
        storage
            .save_recording_checkpoint("recovery-a", &recorder.checkpoint())
            .unwrap();
        storage
            .save_recording_checkpoint("recovery-b", &recorder.checkpoint())
            .unwrap();
        let mut plane = ControlPlane::with_storage("recovery-list", storage);
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "recordings.recovery".into(),
            params: Some(json!({ "limit": 1 })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["items"].as_array().unwrap().len(), 1);
        assert_eq!(result["items"][0]["recordingId"], "recovery-a");
        assert_eq!(result["items"][0]["status"], "available");
        let cursor = result["nextCursor"].as_str().unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "recordings.recovery".into(),
            params: Some(json!({ "cursor": cursor, "limit": 1 })),
        });
        assert_eq!(
            response.result.unwrap()["items"][0]["recordingId"],
            "recovery-b"
        );
    }

    #[test]
    fn live_recorders_list_reports_in_memory_state_and_frame() {
        let mut plane = ControlPlane::default();
        plane.insert_session(session()).unwrap();
        let arm = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "recorders.arm".into(),
            params: Some(json!({ "sessionId": "session", "idempotencyKey": "arm-live-list" })),
        });
        assert_eq!(arm.result.unwrap()["state"], "armed");
        let start = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "recorders.start".into(),
            params: Some(json!({ "sessionId": "session", "frame": 128, "idempotencyKey": "start-live-list" })),
        });
        assert_eq!(start.result.unwrap()["state"], "recording");
        let listed = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "recorders.list".into(),
            params: None,
        });
        assert_eq!(
            listed.result.unwrap(),
            json!([{
                "sessionId": "session",
                "state": "recording",
                "lastFrame": 128
            }])
        );
    }

    #[test]
    fn recording_recycle_preview_never_moves_the_file_and_missing_is_safe() {
        let path =
            std::env::temp_dir().join(format!("audiorouter-recycle-{}.wav", std::process::id()));
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, b"test recording").unwrap();
        let storage = Storage::open_memory().unwrap();
        storage
            .save_recording(&audiorouter_storage::RecordingRecord {
                id: "recording-recycle".into(),
                session_id: "session".into(),
                recorder_id: "recorder".into(),
                path: path.to_string_lossy().into_owned(),
                format: "wav".into(),
                channels: 1,
                sample_rate: 44_100,
                frames: 10,
                file_bytes: 14,
                start_time: "2026-09-06T00:00:00Z".into(),
                state: "completed".into(),
                missing: false,
                title: None,
                artist: None,
                comment: None,
            })
            .unwrap();
        let mut plane = ControlPlane::with_storage("recording-recycle", storage);
        let preview = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(11)),
            method: "recordings.recycle".into(),
            params: Some(json!({ "recordingId": "recording-recycle" })),
        });
        assert_eq!(preview.result.unwrap()["preview"], true);
        assert!(path.is_file());
        let preview_events = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(13)),
                method: "events.subscribe".into(),
                params: Some(json!({ "afterSequence": 0, "sessionId": "session" })),
            })
            .result
            .unwrap();
        assert!(preview_events["events"].as_array().unwrap().is_empty());
        std::fs::remove_file(&path).unwrap();
        let missing = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(12)),
            method: "recordings.recycle".into(),
            params: Some(json!({
                "recordingId": "recording-recycle",
                "confirm": true,
                "idempotencyKey": "recycle-missing"
            })),
        });
        assert_eq!(missing.result.unwrap()["reason"], "missing");
    }

    #[test]
    fn recording_metadata_mutation_requires_record_scope_and_preserves_file_path() {
        let storage = Storage::open_memory().unwrap();
        storage
            .save_recording(&audiorouter_storage::RecordingRecord {
                id: "recording-edit".into(),
                session_id: "session".into(),
                recorder_id: "recorder".into(),
                path: "C:\\recordings\\keep.wav".into(),
                format: "wav".into(),
                channels: 1,
                sample_rate: 44_100,
                frames: 10,
                file_bytes: 44,
                start_time: "2026-09-06T00:00:00Z".into(),
                state: "completed".into(),
                missing: false,
                title: None,
                artist: None,
                comment: None,
            })
            .unwrap();
        let mut plane = ControlPlane::with_storage("recording-edit", storage);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(10)),
            method: "recordings.setMetadata".into(),
            params: Some(json!({
                "recordingId": "recording-edit",
                "title": "Edited",
                "idempotencyKey": "metadata-edit-1"
            })),
        };
        assert!(plane
            .dispatch_authorized(request.clone(), &ClientGrant::read_only())
            .error
            .is_some());
        let response = plane.dispatch_authorized(
            request,
            &ClientGrant::with_scopes([PermissionScope::Record]),
        );
        assert_eq!(response.result.unwrap()["updated"], true);
        let replay = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(14)),
                method: "recordings.setMetadata".into(),
                params: Some(json!({
                    "recordingId": "recording-edit",
                    "title": "Edited",
                    "idempotencyKey": "metadata-edit-1"
                })),
            },
            &ClientGrant::with_scopes([PermissionScope::Record]),
        );
        assert_eq!(replay.result.unwrap()["updated"], true);
        let conflict = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(15)),
                method: "recordings.setMetadata".into(),
                params: Some(json!({
                    "recordingId": "recording-edit",
                    "title": "Different",
                    "idempotencyKey": "metadata-edit-1"
                })),
            },
            &ClientGrant::with_scopes([PermissionScope::Record]),
        );
        assert!(conflict.error.is_some());
        let record = plane
            .storage
            .as_ref()
            .unwrap()
            .get_recording("recording-edit")
            .unwrap()
            .unwrap();
        assert_eq!(record.path, "C:\\recordings\\keep.wav");
        assert_eq!(record.title.as_deref(), Some("Edited"));
        let events = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(12)),
                method: "events.subscribe".into(),
                params: Some(json!({ "afterSequence": 0, "sessionId": "session" })),
            })
            .result
            .unwrap();
        assert_eq!(events["events"][0]["category"], "recording.metadataChanged");
        let response = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(11)),
                method: "recordings.removeEntry".into(),
                params: Some(json!({
                    "recordingId": "recording-edit",
                    "idempotencyKey": "remove-recording-edit"
                })),
            },
            &ClientGrant::with_scopes([PermissionScope::Record]),
        );
        assert_eq!(response.result.unwrap()["fileAction"], "none");
        assert!(plane
            .storage
            .as_ref()
            .unwrap()
            .get_recording("recording-edit")
            .unwrap()
            .is_none());
        let events = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(13)),
                method: "events.subscribe".into(),
                params: Some(json!({ "afterSequence": 1, "sessionId": "session" })),
            })
            .result
            .unwrap();
        assert_eq!(events["events"][0]["category"], "recording.entryRemoved");
    }

    #[test]
    fn control_plane_uses_shared_plan_commit_authority() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "changed".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        let result = plane.commit_graph(&plan, 0, "op-1").unwrap();
        assert_eq!(result["revision"], 1);
        assert_eq!(plane.get_session(&original.id).unwrap().name, "changed");
    }

    #[test]
    fn graph_plan_persistence_failure_rolls_back_the_in_memory_plan() {
        let storage = Storage::open_memory().unwrap();
        let original = session();
        let mut plane = ControlPlane::with_storage("plan-rollback", storage);
        plane.insert_session(original.clone()).unwrap();
        for index in 0..audiorouter_domain::MAX_PENDING_GRAPH_PLANS {
            plane
                .storage
                .as_ref()
                .unwrap()
                .save_graph_plan(&GraphPlanRecord {
                    id: format!("filled-plan-{index}"),
                    session_id: original.id.as_str().into(),
                    base_revision: 0,
                    candidate: original.clone(),
                    expires_at: i64::MAX,
                })
                .unwrap();
        }
        let mut candidate = original.clone();
        candidate.name = "must-not-survive".into();

        let result = plane.plan_graph(&original.id, 0, candidate);
        assert!(
            matches!(result, Err(ControlError::InvalidRequest(_))),
            "{result:?}"
        );
        assert!(matches!(
            plane.commit_graph(&EntityId::new("plan-2"), 0, "rollback-check"),
            Err(ControlError::Store(
                audiorouter_domain::StoreError::PlanNotFound
            ))
        ));
    }

    #[test]
    fn authorized_idempotency_keys_are_scoped_to_client_and_method() {
        let mut plane =
            ControlPlane::with_storage("scoped-idempotency", Storage::open_memory().unwrap());
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let grant = ClientGrant::for_role(ClientRole::Editor);
        let same_key = "shared-client-key";

        let mut first_candidate = original.clone();
        first_candidate.name = "first-client-change".into();
        let first_plan = plane.plan_graph(&original.id, 0, first_candidate).unwrap();
        let first = plane.dispatch_authorized_for_client(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "graph.commit".into(),
                params: Some(json!({
                    "planId": first_plan,
                    "baseRevision": 0,
                    "idempotencyKey": same_key
                })),
            },
            "client-a",
            &grant,
        );
        assert_eq!(first.result.unwrap()["revision"], 1);

        let committed = plane.get_session(&original.id).unwrap().clone();
        let mut second_candidate = committed.clone();
        second_candidate.name = "second-client-change".into();
        let second_plan = plane.plan_graph(&original.id, 1, second_candidate).unwrap();
        let second = plane.dispatch_authorized_for_client(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(2)),
                method: "graph.commit".into(),
                params: Some(json!({
                    "planId": second_plan,
                    "baseRevision": 1,
                    "idempotencyKey": same_key
                })),
            },
            "client-b",
            &grant,
        );
        assert_eq!(second.result.unwrap()["revision"], 2);
        assert_eq!(
            plane.get_session(&original.id).unwrap().name,
            "second-client-change"
        );

        let mut operation_lookup = |id: i32, client_id: &str| {
            plane
                .dispatch_authorized_for_client(
                    JsonRpcRequest {
                        jsonrpc: "2.0".into(),
                        id: Some(json!(id)),
                        method: "operations.get".into(),
                        params: Some(json!({ "operationId": same_key })),
                    },
                    client_id,
                    &grant,
                )
                .result
                .unwrap()
        };
        assert_eq!(operation_lookup(3, "client-a")["revision"], 1);
        assert_eq!(operation_lookup(4, "client-b")["revision"], 2);
    }

    #[test]
    fn dispatch_rejects_mutating_notifications_and_unknown_methods() {
        let mut plane = ControlPlane::default();
        let notification = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: None,
            method: "graph.commit".into(),
            params: None,
        };
        assert_eq!(plane.dispatch(notification).error.unwrap().code, -32600);
        let privacy_notification = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: None,
            method: "safety.setPrivacyMute".into(),
            params: Some(json!({ "muted": true })),
        };
        assert_eq!(
            plane.dispatch(privacy_notification).error.unwrap().code,
            -32600
        );
        for method in ["operations.cancel", "recordings.rename"] {
            let notification = JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: None,
                method: method.into(),
                params: None,
            };
            assert_eq!(plane.dispatch(notification).error.unwrap().code, -32600);
        }
        let unknown = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "no.such.method".into(),
            params: None,
        };
        assert_eq!(plane.dispatch(unknown).error.unwrap().code, -32601);
    }

    #[test]
    fn mutation_classifier_matches_authoritative_method_metadata() {
        for method in API_METHODS {
            assert_eq!(
                is_mutating_method(method.name),
                method.side_effect != audiorouter_domain::SideEffectClass::ReadOnly,
                "mutation classification drifted for {}",
                method.name
            );
        }
    }

    #[test]
    fn dispatch_plan_and_commit_use_json_contracts() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "via-api".into();
        let plan_request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "graph.plan".into(),
            params: Some(
                json!({ "sessionId": "session", "baseRevision": 0, "candidate": candidate }),
            ),
        };
        let plan_result = plane.dispatch(plan_request).result.unwrap();
        assert_eq!(plan_result["diff"][0]["path"], "/name");
        assert_eq!(plan_result["requiredScopes"], json!(["graph.write"]));
        let plan_id = plan_result["planId"].clone();
        let commit_request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "graph.commit".into(),
            params: Some(
                json!({ "planId": plan_id, "baseRevision": 0, "idempotencyKey": "api-op" }),
            ),
        };
        assert_eq!(
            plane.dispatch(commit_request).result.unwrap()["revision"],
            1
        );
    }

    #[test]
    fn fake_session_lifecycle_is_idempotent_and_stoppable() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let first = plane.session_start(&original.id).unwrap();
        assert_eq!(first["state"], "running");
        assert_eq!(first["generation"], 1);
        assert_eq!(plane.session_start(&original.id).unwrap()["generation"], 1);
        assert_eq!(
            plane.session_stop(&original.id).unwrap()["state"],
            "stopped"
        );
        let events = plane.events.since(0, 10).unwrap();
        assert_eq!(events.last().unwrap().resource_revision, original.revision);
        assert_eq!(plane.session_start(&original.id).unwrap()["generation"], 2);
    }

    #[test]
    fn session_stop_refuses_to_orphan_an_active_recorder() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        plane.session_start(&original.id).unwrap();
        let mut recorder = RecorderController::new();
        recorder.arm().unwrap();
        recorder.start(128).unwrap();
        plane.recorders.insert(original.id.clone(), recorder);

        assert!(matches!(
            plane.session_stop(&original.id),
            Err(ControlError::InvalidRequest(message))
                if message == "finalize the active recorder before stopping the session"
        ));
        assert_eq!(plane.runtimes[&original.id].state(), RuntimeState::Running);
    }

    #[test]
    fn recorder_api_forwards_lifecycle_boundaries_to_attached_worker() {
        let hooks = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        plane
            .attach_recorder_worker(
                original.id.clone(),
                Box::new(HookRecorderWorker {
                    hooks: hooks.clone(),
                }),
            )
            .unwrap();

        for (id, method, frame) in [
            (1, "recorders.arm", None),
            (2, "recorders.start", Some(0)),
            (3, "recorders.split", Some(128)),
        ] {
            let params = match frame {
                Some(frame) => json!({
                    "sessionId": original.id,
                    "frame": frame,
                    "idempotencyKey": format!("hook-{id}")
                }),
                None => json!({
                    "sessionId": original.id,
                    "idempotencyKey": format!("hook-{id}")
                }),
            };
            assert!(plane
                .dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(id)),
                    method: method.into(),
                    params: Some(params),
                })
                .result
                .is_some());
        }
        assert_eq!(*hooks.lock().unwrap(), vec!["arm", "start:0", "split:128"]);
        assert_eq!(
            plane.recorders[&original.id].checkpoint().last_frame,
            Some(128)
        );
    }

    #[test]
    fn recorder_api_stop_finalizes_attached_wav_before_completion() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-control-api-stop-{}.wav",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let worker = WavRecorderWorker::new(file, WavFormat::Pcm16, 1, 48_000, 1, 1).unwrap();
        worker
            .try_push(RecordingChunk {
                start_frame: 0,
                samples: vec![0.25, -0.25],
            })
            .unwrap();

        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        plane
            .attach_recorder_worker(original.id.clone(), Box::new(worker))
            .unwrap();
        for (id, method, frame) in [
            (1, "recorders.arm", None),
            (2, "recorders.start", Some(0)),
            (3, "recorders.stop", Some(2)),
        ] {
            let params = match frame {
                Some(frame) => json!({
                    "sessionId": original.id,
                    "frame": frame,
                    "idempotencyKey": format!("api-stop-{id}")
                }),
                None => json!({
                    "sessionId": original.id,
                    "idempotencyKey": format!("api-stop-{id}")
                }),
            };
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(id)),
                method: method.into(),
                params: Some(params),
            });
            assert!(response.result.is_some(), "{method}: {response:?}");
        }
        assert!(plane.recorder_workers.is_empty());
        assert_eq!(
            audiorouter_recording::inspect_wav_file(&path)
                .unwrap()
                .frames,
            2
        );
        assert_eq!(
            plane.recorders[&original.id].state(),
            RecorderState::Completed
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn recorder_factory_creates_attaches_and_indexes_a_wav_before_arm() {
        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("audiorouter-control-factory-{run_id}"));
        std::fs::create_dir_all(&root).unwrap();
        let policy = RecordingPathPolicy::new(&root).unwrap();
        let mut plane = ControlPlane::with_storage("factory", Storage::open_memory().unwrap());
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        plane.configure_recording_root(&root).unwrap();
        let invalid = FileRecorderConfig {
            version: FILE_RECORDER_CONFIG_VERSION + 1,
            session_id: original.id.as_str(),
            recorder_id: "invalid",
            sequence: 0,
            format: FileRecorderFormat::Wav(WavFormat::Pcm16),
            channels: 1,
            sample_rate: 48_000,
            dither: false,
            queue_capacity: 8,
            maximum_chunks_per_pass: 1,
        };
        assert!(create_file_recorder_with_config(&policy, &invalid).is_err());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        let config = FileRecorderConfig {
            version: FILE_RECORDER_CONFIG_VERSION,
            session_id: original.id.as_str(),
            recorder_id: "voice",
            sequence: 0,
            format: FileRecorderFormat::Wav(WavFormat::Pcm16),
            channels: 1,
            sample_rate: 48_000,
            dither: false,
            queue_capacity: 8,
            maximum_chunks_per_pass: 1,
        };
        let path = plane
            .create_and_attach_configured_file_recorder(original.id.clone(), &config)
            .unwrap();
        assert!(path.is_file());

        for (id, method, frame) in [
            (1, "recorders.arm", None),
            (2, "recorders.start", Some(0)),
            (3, "recorders.stop", Some(0)),
        ] {
            let params = match frame {
                Some(frame) => json!({
                    "sessionId": original.id,
                    "frame": frame,
                    "idempotencyKey": format!("factory-{id}")
                }),
                None => json!({
                    "sessionId": original.id,
                    "idempotencyKey": format!("factory-{id}")
                }),
            };
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(id)),
                method: method.into(),
                params: Some(params),
            });
            assert!(response.result.is_some(), "{method}: {response:?}");
        }
        let rows = plane
            .storage
            .as_ref()
            .unwrap()
            .list_recordings(Some(original.id.as_str()))
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].format, "wav");
        assert_eq!(rows[0].path, path.to_str().unwrap());
        assert_eq!(rows[0].frames, 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn attached_wav_recorder_exposes_one_prebuilt_realtime_tap() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-control-tap-{}.wav",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let worker = WavRecorderWorker::new(file, WavFormat::Pcm16, 1, 48_000, 4, 1).unwrap();
        let mut plane = ControlPlane::default();
        let session_id = EntityId::new("tap-session");
        plane
            .attach_recorder_worker(session_id.clone(), Box::new(worker))
            .unwrap();

        let taps = plane.recorder_tap_set(&session_id).unwrap();
        assert_eq!(taps.len(), 1);
        assert!(!taps.is_empty());
        assert!(plane
            .recorder_tap_set(&EntityId::new("missing-session"))
            .is_err());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn recorder_binding_requires_one_validated_node_and_matching_generation() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-control-binding-{}.wav",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let worker = WavRecorderWorker::new(file, WavFormat::Pcm16, 1, 48_000, 4, 1).unwrap();
        let mut graph = session();
        graph.nodes.push(Node {
            id: EntityId::new("recorder-node"),
            kind: NodeKind::Recorder,
            type_version: 1,
            name: "Recorder".into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: vec![],
        });
        let session_id = graph.id.clone();
        let mut plane = ControlPlane::default();
        plane.insert_session(graph).unwrap();
        plane
            .attach_recorder_worker(session_id.clone(), Box::new(worker))
            .unwrap();

        let bindings = plane
            .recorder_tap_bindings(&session_id, RuntimeGeneration::new(7))
            .unwrap();
        let taps = bindings
            .tap_set_for_generation(RuntimeGeneration::new(7), &["recorder-node"])
            .unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(taps.len(), 1);
        assert!(matches!(
            bindings.tap_set_for_generation(RuntimeGeneration::new(8), &["recorder-node"]),
            Err(audiorouter_engine::RecorderTapBindingError::StaleGeneration)
        ));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn independent_recorder_nodes_bind_distinct_workers_and_taps() {
        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let paths = [
            std::env::temp_dir().join(format!("audiorouter-control-node-a-{run_id}.wav")),
            std::env::temp_dir().join(format!("audiorouter-control-node-b-{run_id}.wav")),
        ];
        let make_worker = |path: &std::path::Path| {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .unwrap();
            WavRecorderWorker::new(file, WavFormat::Pcm16, 1, 48_000, 4, 1).unwrap()
        };
        let mut graph = session();
        for (id, name) in [("recorder-a", "A"), ("recorder-b", "B")] {
            graph.nodes.push(Node {
                id: EntityId::new(id),
                kind: NodeKind::Recorder,
                type_version: 1,
                name: name.into(),
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![],
            });
        }
        let session_id = graph.id.clone();
        let mut plane = ControlPlane::default();
        plane.insert_session(graph).unwrap();
        plane
            .attach_recorder_worker_to_node(
                &session_id,
                EntityId::new("recorder-a"),
                Box::new(make_worker(&paths[0])),
            )
            .unwrap();
        plane
            .attach_recorder_worker_to_node(
                &session_id,
                EntityId::new("recorder-b"),
                Box::new(make_worker(&paths[1])),
            )
            .unwrap();

        let bindings = plane
            .recorder_tap_bindings(&session_id, RuntimeGeneration::new(9))
            .unwrap();
        assert_eq!(bindings.len(), 2);
        let taps = bindings
            .tap_set_for_generation(RuntimeGeneration::new(9), &["recorder-a", "recorder-b"])
            .unwrap();
        assert_eq!(taps.len(), 2);
        for node_id in ["recorder-a", "recorder-b"] {
            plane
                .control_recorder_node(&EntityId::new(node_id), "recorders.arm", None)
                .unwrap();
            plane
                .control_recorder_node(&EntityId::new(node_id), "recorders.start", Some(0))
                .unwrap();
            let stopped = plane
                .control_recorder_node(&EntityId::new(node_id), "recorders.stop", Some(0))
                .unwrap();
            assert_eq!(stopped["state"], "completed");
        }
        assert!(plane.recorder_node_workers.is_empty());
        assert!(plane.session_stop(&session_id).is_ok());
        assert!(plane
            .attach_recorder_worker_to_node(
                &session_id,
                EntityId::new("missing"),
                Box::new(make_worker(
                    &std::env::temp_dir()
                        .join(format!("audiorouter-control-node-missing-{run_id}.wav"))
                )),
            )
            .is_err());

        plane.delete_session(&session_id).unwrap();
        assert!(plane.recorder_node_workers.is_empty());

        for path in paths {
            let _ = std::fs::remove_file(path);
        }
        let _ = std::fs::remove_file(
            std::env::temp_dir().join(format!("audiorouter-control-node-missing-{run_id}.wav")),
        );
    }

    #[test]
    fn recorder_create_api_is_idempotent_and_does_not_arm() {
        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("audiorouter-control-create-api-{run_id}"));
        std::fs::create_dir_all(&root).unwrap();
        let mut plane = ControlPlane::with_storage("create-api", Storage::open_memory().unwrap());
        plane.configure_recording_root(&root).unwrap();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "recorders.create".into(),
            params: Some(json!({
                "sessionId": original.id,
                "recorderId": "voice",
                "format": "wavPcm24",
                "sequence": 0,
                "channels": 1,
                "sampleRate": 48000,
                "dither": true,
                "queueCapacity": 8,
                "maximumChunksPerPass": 1,
                "idempotencyKey": "create-api-1"
            })),
        };
        let first = plane.dispatch(request.clone());
        let first_result = first.result.clone().unwrap();
        assert_eq!(first_result["state"], "idle");
        assert_eq!(first_result["armed"], false);
        assert_eq!(first_result["format"], "wavPcm24");
        let second = plane.dispatch(request);
        assert_eq!(second.result.unwrap(), first_result);
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        assert_eq!(plane.recorders.len(), 0);
        let duplicate = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "recorders.create".into(),
            params: Some(json!({
                "sessionId": original.id,
                "recorderId": "voice",
                "format": "wavPcm24",
                "sequence": 1,
                "channels": 1,
                "sampleRate": 48000,
                "queueCapacity": 8,
                "maximumChunksPerPass": 1,
                "idempotencyKey": "create-api-duplicate"
            })),
        };
        let failed = plane.dispatch(duplicate);
        assert!(failed.error.is_some());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn session_stop_uses_attached_worker_and_reports_finalization() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        plane.session_start(&original.id).unwrap();
        let mut recorder = RecorderController::new();
        recorder.arm().unwrap();
        recorder.start(128).unwrap();
        plane.recorders.insert(original.id.clone(), recorder);
        plane
            .attach_recorder_worker(original.id.clone(), Box::new(TestRecorderWorker))
            .unwrap();

        let result = plane.session_stop(&original.id).unwrap();
        assert_eq!(result["state"], "stopped");
        assert_eq!(result["recorders"][0]["state"], "completed");
        assert_eq!(result["recorders"][0]["fileFinalized"], true);
        assert_eq!(plane.runtimes[&original.id].state(), RuntimeState::Stopped);
        assert_eq!(
            plane.recorders[&original.id].state(),
            RecorderState::Completed
        );
        assert!(!plane.recorder_workers.contains_key(&original.id));
    }

    #[test]
    fn concrete_wav_worker_drains_and_finalizes_before_session_stop() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-control-worker-{}.wav",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut worker = WavRecorderWorker::new(file, WavFormat::Float32, 1, 48_000, 8, 1).unwrap();
        worker.arm().unwrap();
        worker.start(0).unwrap();
        let tap = worker.audio_tap();
        let processor = RuntimeProcessor::default();
        processor.publish(RuntimeGraph::prepare(RuntimeGeneration::new(1), vec![]));
        let mut block = AudioBlock::new(1, 2).unwrap();
        block
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[0.25, -0.25]);
        assert_eq!(
            processor.process_with_tap(&mut block, 0, &tap),
            Some(RuntimeGeneration::new(1))
        );

        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        plane.session_start(&original.id).unwrap();
        let mut recorder = RecorderController::new();
        recorder.arm().unwrap();
        recorder.start(0).unwrap();
        recorder.advance(2).unwrap();
        plane.recorders.insert(original.id.clone(), recorder);
        plane
            .attach_recorder_worker_with_identity(
                original.id.clone(),
                FileRecordingIdentity {
                    session_id: original.id.as_str().to_owned(),
                    recorder_id: "voice".into(),
                    path: path.clone(),
                },
                Box::new(worker),
            )
            .unwrap();

        let result = plane.session_stop(&original.id).unwrap();
        assert_eq!(result["recorders"][0]["fileFinalized"], true);
        let info = audiorouter_recording::inspect_wav_file(&path).unwrap();
        assert_eq!(info.frames, 2);
        assert_eq!(info.sample_rate, 48_000);
        assert_eq!(
            plane.recorders[&original.id].state(),
            RecorderState::Completed
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn segmented_wav_worker_finalizes_policy_owned_segments() {
        let root = std::env::temp_dir().join(format!(
            "audiorouter-control-segmented-worker-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        let policy = RecordingPathPolicy::new(&root).unwrap();
        let mut worker = SegmentedWavRecorderWorker::new(
            policy,
            "session:raw",
            "voice?raw",
            WavFormat::Pcm16,
            1,
            48_000,
            1,
            1,
            2,
        )
        .unwrap();
        worker.arm().unwrap();
        worker.start(0).unwrap();
        worker
            .try_push(RecordingChunk {
                start_frame: 0,
                samples: vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5],
            })
            .unwrap();

        let outcome = worker.finalize(6).unwrap();
        assert_eq!(outcome.state, "completed");
        assert!(outcome.file_finalized);
        let recordings = worker.finalized_recordings();
        assert_eq!(recordings.len(), 3);
        assert!(recordings.iter().all(|recording| {
            recording.session_id == "session:raw"
                && recording.recorder_id == "voice?raw"
                && recording.state == "completed"
                && !recording.missing
                && recording.frames == 2
                && recording.file_bytes > 44
        }));
        drop(worker);
        let mut paths = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        paths.sort();
        assert_eq!(paths.len(), 3);
        for path in &paths {
            assert_eq!(
                audiorouter_recording::inspect_wav_file(path)
                    .unwrap()
                    .frames,
                2
            );
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn segmented_wav_worker_stop_persists_finalized_library_rows() {
        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "audiorouter-control-library-root-{}-{run_id}",
            std::process::id()
        ));
        let database = std::env::temp_dir().join(format!(
            "audiorouter-control-library-{}-{run_id}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&database);
        std::fs::create_dir(&root).unwrap();
        let policy = RecordingPathPolicy::new(&root).unwrap();
        let worker = SegmentedWavRecorderWorker::new(
            policy,
            "session",
            "voice",
            WavFormat::Pcm16,
            1,
            48_000,
            2,
            1,
            2,
        )
        .unwrap();
        worker
            .try_push(RecordingChunk {
                start_frame: 0,
                samples: vec![0.25, -0.25],
            })
            .unwrap();

        let original = session();
        let mut plane =
            ControlPlane::with_storage("library-test", Storage::open(&database).unwrap());
        plane.insert_session(original.clone()).unwrap();
        plane
            .attach_recorder_worker(original.id.clone(), Box::new(worker))
            .unwrap();
        for (id, method, frame) in [
            (1, "recorders.arm", None),
            (2, "recorders.start", Some(0)),
            (3, "recorders.stop", Some(2)),
        ] {
            let params = match frame {
                Some(frame) => json!({
                    "sessionId": original.id,
                    "frame": frame,
                    "idempotencyKey": format!("library-{id}")
                }),
                None => json!({
                    "sessionId": original.id,
                    "idempotencyKey": format!("library-{id}")
                }),
            };
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(id)),
                method: method.into(),
                params: Some(params),
            });
            assert!(response.result.is_some(), "{method}: {response:?}");
        }
        drop(plane);

        let storage = Storage::open(&database).unwrap();
        let records = storage.list_recordings(Some("session")).unwrap();
        assert_eq!(records.len(), 1);
        assert!(records[0].id.starts_with("session-voice-"));
        assert!(records[0].id.ends_with("-0"));
        assert_eq!(records[0].frames, 2);
        assert!(!records[0].missing);
        assert!(std::path::Path::new(&records[0].path).is_file());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&database);
    }

    #[test]
    fn concrete_buffered_flac_worker_finalizes_before_session_stop() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-control-worker-{}.flac",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut worker = BufferedFlacRecorderWorker::new(file, 1, 48_000, 16, 8, 1).unwrap();
        worker.arm().unwrap();
        worker.start(0).unwrap();
        worker
            .try_push(RecordingChunk {
                start_frame: 0,
                samples: vec![0.25, -0.25],
            })
            .unwrap();

        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        plane.session_start(&original.id).unwrap();
        let mut recorder = RecorderController::new();
        recorder.arm().unwrap();
        recorder.start(0).unwrap();
        recorder.advance(2).unwrap();
        plane.recorders.insert(original.id.clone(), recorder);
        plane
            .attach_recorder_worker(original.id.clone(), Box::new(worker))
            .unwrap();

        let result = plane.session_stop(&original.id).unwrap();
        assert_eq!(result["recorders"][0]["fileFinalized"], true);
        let info = audiorouter_recording::inspect_flac_file(&path).unwrap();
        assert_eq!(info.frames, 2);
        assert_eq!(info.sample_rate, 48_000);
        assert_eq!(
            plane.recorders[&original.id].state(),
            RecorderState::Completed
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn concrete_streaming_flac_worker_writes_frames_before_finalize() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-control-streaming-worker-{}.flac",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut worker = StreamingFlacRecorderWorker::new(file, 1, 48_000, 16, 8, 1).unwrap();
        worker.set_library_identity(FileRecordingIdentity {
            session_id: "session".into(),
            recorder_id: "voice".into(),
            path: path.clone(),
        });
        worker.arm().unwrap();
        worker.start(0).unwrap();
        worker
            .try_push(RecordingChunk {
                start_frame: 0,
                samples: vec![0.25, -0.25],
            })
            .unwrap();
        let outcome = worker.finalize(2).unwrap();
        assert_eq!(outcome.state, "completed");
        let info = audiorouter_recording::inspect_flac_file(&path).unwrap();
        assert_eq!(info.frames, 2);
        let rows = worker.finalized_recordings();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].format, "flac");
        assert_eq!(rows[0].frames, 2);
        assert_eq!(rows[0].file_bytes, info.file_bytes);
        assert!(!rows[0].missing);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn keyed_session_lifecycle_replays_and_conflicts_durably() {
        let storage = Storage::open_memory().unwrap();
        let mut plane = ControlPlane::with_storage("lifecycle-idempotency", storage);
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let start = || JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "sessions.start".into(),
            params: Some(json!({ "sessionId": "session", "idempotencyKey": "start-1" })),
        };
        let first = plane.dispatch(start()).result.unwrap();
        let replay = plane.dispatch(start()).result.unwrap();
        assert_eq!(first, replay);
        let mut other = session();
        other.id = EntityId::new("other");
        plane.insert_session(other).unwrap();
        let same_key = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "sessions.start".into(),
            params: Some(json!({ "sessionId": "other", "idempotencyKey": "start-1" })),
        });
        assert!(same_key.error.is_some());
        let stop = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "sessions.stop".into(),
            params: Some(json!({ "sessionId": "session", "idempotencyKey": "stop-1" })),
        });
        assert_eq!(stop.result.as_ref().unwrap()["state"], "stopped");
        let stop_replay = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "sessions.stop".into(),
            params: Some(json!({ "sessionId": "session", "idempotencyKey": "stop-1" })),
        });
        assert_eq!(stop.result.unwrap(), stop_replay.result.unwrap());
    }

    #[test]
    fn commit_reactivates_a_running_fake_session_as_one_generation() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        assert_eq!(plane.session_start(&original.id).unwrap()["generation"], 1);
        let mut candidate = original.clone();
        candidate.name = "live-edit".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        let result = plane.commit_graph(&plan, 0, "live-edit-op").unwrap();
        assert_eq!(result["activation"]["state"], "running");
        assert_eq!(result["activation"]["generation"], 2);
        assert_eq!(plane.session_start(&original.id).unwrap()["generation"], 2);
    }

    #[test]
    fn session_lifecycle_requires_an_existing_session() {
        let mut plane = ControlPlane::default();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "session.start".into(),
            params: Some(json!({ "sessionId": "missing" })),
        };
        assert_eq!(plane.dispatch(request).error.unwrap().code, -32602);
    }

    #[test]
    fn session_start_enforces_the_two_active_session_limit() {
        let mut plane = ControlPlane::default();
        for id in ["one", "two", "three"] {
            let mut graph = session();
            graph.id = EntityId::new(id);
            plane.insert_session(graph).unwrap();
        }
        plane.session_start(&EntityId::new("one")).unwrap();
        plane.session_start(&EntityId::new("two")).unwrap();
        assert!(matches!(
            plane.session_start(&EntityId::new("three")),
            Err(ControlError::InvalidRequest(message)) if message == "active session limit reached"
        ));
    }

    #[test]
    fn recorder_arm_enforces_the_eight_recorder_global_limit() {
        let mut plane = ControlPlane::default();
        for index in 0..=MAX_ACTIVE_RECORDERS {
            let id = EntityId::new(format!("recorder-session-{index}"));
            let mut graph = session();
            graph.id = id.clone();
            plane.insert_session(graph).unwrap();
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(index)),
                method: "recorders.arm".into(),
                params: Some(json!({
                    "sessionId": id,
                    "idempotencyKey": format!("arm-{index}")
                })),
            });
            if index < MAX_ACTIVE_RECORDERS {
                assert!(response.result.is_some(), "arm {index}: {response:?}");
            } else {
                assert!(matches!(
                    response.error,
                    Some(error) if error.message.contains("active recorder limit reached")
                ));
            }
        }
    }

    #[test]
    fn storage_backed_control_persists_session_and_commit() {
        let storage = Storage::open_memory().unwrap();
        let mut plane = ControlPlane::with_storage("persistent-test", storage);
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "persisted-change".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        let result = plane.commit_graph(&plan, 0, "persist-op").unwrap();
        assert_eq!(plane.get_session(&original.id).unwrap().revision, 1);
        assert!(result["revision"] == 1);
    }

    #[test]
    fn operations_get_returns_durable_commit_outcome() {
        let storage = Storage::open_memory().unwrap();
        let mut plane = ControlPlane::with_storage("operation-test", storage);
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "operation-change".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "operation-id").unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(13)),
            method: "operations.get".into(),
            params: Some(json!({ "operationId": "operation-id" })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["status"], "completed");
        assert_eq!(result["durable"], true);
        assert_eq!(result["revision"], 1);
        assert_eq!(result["result"]["revision"], 1);
    }

    #[test]
    fn operations_get_returns_live_memory_outcome_without_claiming_durability() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "memory-operation".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "memory-operation-id").unwrap();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(14)),
            method: "operations.get".into(),
            params: Some(json!({ "operationId": "memory-operation-id" })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["status"], "completed");
        assert_eq!(result["durable"], false);
        assert_eq!(result["result"]["revision"], 1);
    }

    #[test]
    fn memory_operation_retention_evicts_in_insertion_order() {
        let mut plane = ControlPlane::default();
        for index in 0..=MAX_MEMORY_OPERATION_OUTCOMES {
            plane.remember_operation_outcome(
                &format!("operation-{index}"),
                json!({ "index": index }),
                "test.operation",
                None,
            );
        }
        assert!(!plane.operation_outcomes.contains_key("operation-0"));
        assert!(plane.operation_outcomes.contains_key("operation-1"));
        assert!(plane
            .operation_outcomes
            .contains_key(&format!("operation-{MAX_MEMORY_OPERATION_OUTCOMES}")));
        assert_eq!(plane.operation_order.len(), MAX_MEMORY_OPERATION_OUTCOMES);
    }

    #[test]
    fn operations_cancel_reports_completed_operations_without_undoing_them() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "cancel-check".into();
        let plan = plane.plan_graph(&original.id, 0, candidate).unwrap();
        plane.commit_graph(&plan, 0, "cancel-check-key").unwrap();
        let missing_key = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(15)),
            method: "operations.cancel".into(),
            params: Some(json!({ "operationId": "cancel-check-key" })),
        });
        assert_eq!(missing_key.error.unwrap().code, -32602);
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(16)),
            method: "operations.cancel".into(),
            params: Some(json!({
                "operationId": "cancel-check-key",
                "idempotencyKey": "cancel-request-key"
            })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["status"], "completed");
        assert_eq!(result["cancelled"], false);
        assert_eq!(result["reason"], "alreadyCompleted");
        assert_eq!(plane.get_session(&original.id).unwrap().revision, 1);
    }

    #[test]
    fn graph_commit_acknowledgments_are_validated_and_cannot_grant_scope() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(17)),
            method: "graph.commit".into(),
            params: Some(json!({
                "planId": "plan-1",
                "baseRevision": 0,
                "idempotencyKey": "ack-test",
                "acknowledgments": ["unverified-feedback"]
            })),
        });
        assert!(response.error.unwrap().message.contains("no warnings"));

        let invalid = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(18)),
            method: "graph.commit".into(),
            params: Some(json!({
                "planId": "plan-1",
                "baseRevision": 0,
                "idempotencyKey": "ack-test",
                "acknowledgments": [12]
            })),
        });
        assert!(invalid.error.unwrap().message.contains("warning IDs"));
    }

    #[test]
    fn durable_commit_replays_before_plan_lookup_after_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-idempotency-restart-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let original = session();
        let mut first = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
        first.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "durable-change".into();
        let plan = first.plan_graph(&original.id, 0, candidate).unwrap();
        first.commit_graph(&plan, 0, "restart-key").unwrap();
        drop(first);

        let mut second = ControlPlane::with_storage("second", Storage::open(&path).unwrap());
        let replay = second.commit_graph(&plan, 0, "restart-key").unwrap();
        assert_eq!(replay["idempotentReplay"], true);
        assert_eq!(replay["revision"], 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn durable_uncommitted_plan_survives_control_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-plan-restart-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let original = session();
        let mut first = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
        first.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.name = "survives-restart".into();
        let plan = first.plan_graph(&original.id, 0, candidate).unwrap();
        drop(first);

        let mut second = ControlPlane::with_storage("second", Storage::open(&path).unwrap());
        let committed = second.commit_graph(&plan, 0, "restart-plan-key").unwrap();
        assert_eq!(committed["revision"], 1);
        assert_eq!(committed["idempotentReplay"], false);
        assert_eq!(
            second
                .dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(15)),
                    method: "sessions.get".into(),
                    params: Some(json!({ "sessionId": "session" })),
                })
                .result
                .unwrap()["name"],
            "survives-restart"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn restart_does_not_reuse_a_durable_graph_plan_id() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-plan-id-restart-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let original = session();
        let mut first = ControlPlane::with_storage("first", Storage::open(&path).unwrap());
        first.insert_session(original.clone()).unwrap();
        let first_plan = first.plan_graph(&original.id, 0, original.clone()).unwrap();
        assert_eq!(first_plan.as_str(), "plan-1");
        drop(first);

        let mut second = ControlPlane::with_storage("second", Storage::open(&path).unwrap());
        let mut candidate = original.clone();
        candidate.name = "second-plan".into();
        let second_plan = second.plan_graph(&original.id, 0, candidate).unwrap();
        assert_eq!(second_plan.as_str(), "plan-2");
        let storage = second.storage.as_ref().unwrap();
        assert_eq!(
            storage
                .load_graph_plan("plan-1")
                .unwrap()
                .unwrap()
                .candidate
                .name,
            "test"
        );
        assert_eq!(
            storage
                .load_graph_plan("plan-2")
                .unwrap()
                .unwrap()
                .candidate
                .name,
            "second-plan"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn framed_request_round_trips_through_dispatch() {
        let mut plane = ControlPlane::default();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(9)),
            method: "system.describe".into(),
            params: None,
        };
        let frame = audiorouter_protocol::encode_frame(&request).unwrap();
        let responses = plane.dispatch_frame(&frame).unwrap();
        let response: JsonRpcResponse = audiorouter_protocol::decode_frame(&responses[0]).unwrap();
        assert_eq!(response.id, Some(json!(9)));
        assert!(response.result.unwrap()["methods"].is_array());
    }

    #[test]
    fn scoped_authorization_denies_mutation_before_dispatch() {
        let mut plane = ControlPlane::default();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "graph.commit".into(),
            params: None,
        };
        let response = plane.dispatch_authorized(request, &ClientGrant::read_only());
        assert_eq!(response.error.unwrap().code, -32001);
    }

    #[test]
    fn authorized_dispatch_validates_known_methods_before_authorization() {
        let mut plane = ControlPlane::default();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!({ "nested": "x".repeat(MAX_REQUEST_ID_BYTES) })),
            method: "graph.commit".into(),
            params: None,
        };
        let response = plane.dispatch_authorized(request, &ClientGrant::read_only());
        assert_eq!(response.error.unwrap().code, -32600);
    }

    #[test]
    fn virtual_bus_lifecycle_keeps_bridge_identity_and_drains_on_disable_delete() {
        let mut plane = ControlPlane::default();
        let id = EntityId::new("bus-stable");
        plane.create_virtual_bus(id.clone(), "Stable bus").unwrap();
        let bridge = plane.virtual_bridges.get(&id).unwrap();
        bridge.activate(1).unwrap();
        plane.set_virtual_bus_enabled(&id, false).unwrap();
        assert!(!bridge.is_active());
        assert!(plane.virtual_bridges.get(&id).is_some());
        plane.delete_virtual_bus(&id).unwrap();
        assert!(plane.virtual_bridges.get(&id).is_none());
    }

    #[test]
    fn virtual_device_lifecycle_requires_explicit_device_administration_scope() {
        let request = |method: &str, params: Option<Value>| JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(40)),
            method: method.into(),
            params,
        };
        for grant in [
            ClientGrant::read_only(),
            ClientGrant::for_role(ClientRole::Editor),
            ClientGrant::for_role(ClientRole::Operator),
        ] {
            let mut plane = ControlPlane::default();
            let plan = plane.dispatch_authorized(
                request(
                    "virtualDevices.plan",
                    Some(json!({ "operation": { "action": "create", "id": "bus-1", "name": "Desktop" } })),
                ),
                &grant,
            );
            assert_eq!(plan.error.unwrap().code, -32001);
            let apply = plane.dispatch_authorized(
                request(
                    "virtualDevices.apply",
                    Some(json!({ "planId": "plan-1", "idempotencyKey": "key-1" })),
                ),
                &grant,
            );
            assert_eq!(apply.error.unwrap().code, -32001);
        }
        let mut plane = ControlPlane::default();
        let grant = ClientGrant::with_scopes([PermissionScope::DeviceAdministration]);
        let response = plane.dispatch_authorized(
            request(
                "virtualDevices.plan",
                Some(json!({ "operation": { "action": "create", "id": "bus-1", "name": "Desktop" } })),
            ),
            &grant,
        );
        assert!(response.result.is_some());
    }

    #[test]
    fn scoped_authorization_allows_discovery_read() {
        let mut plane = ControlPlane::default();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(5)),
            method: "system.describe".into(),
            params: None,
        };
        let response = plane.dispatch_authorized(request, &ClientGrant::read_only());
        assert!(response.result.unwrap()["methods"].is_array());
    }

    #[test]
    fn authorized_framed_dispatch_denies_mutation_before_parameter_parsing() {
        let mut plane = ControlPlane::default();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(6)),
            method: "graph.commit".into(),
            params: None,
        };
        let frame = audiorouter_protocol::encode_frame(&request).unwrap();
        let responses = plane
            .dispatch_frame_authorized(&frame, &ClientGrant::read_only())
            .unwrap();
        let response: JsonRpcResponse = audiorouter_protocol::decode_frame(&responses[0]).unwrap();
        assert_eq!(response.error.unwrap().code, -32001);
    }

    #[test]
    fn authorized_batch_preserves_allowed_and_denied_responses_in_order() {
        let mut plane = ControlPlane::default();
        let message = RpcMessage::Batch(vec![
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "system.describe".into(),
                params: None,
            },
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(2)),
                method: "graph.commit".into(),
                params: None,
            },
        ]);
        let responses = plane.dispatch_message_authorized(message, &ClientGrant::read_only());
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].id, Some(json!(1)));
        assert!(responses[0].result.is_some());
        assert_eq!(responses[1].id, Some(json!(2)));
        assert_eq!(responses[1].error.as_ref().unwrap().code, -32001);
    }

    #[test]
    fn built_in_roles_are_deny_by_default_for_sensitive_scopes() {
        assert!(ClientGrant::for_role(ClientRole::Observer).allows(PermissionScope::Read));
        assert!(!ClientGrant::for_role(ClientRole::Observer).allows(PermissionScope::GraphWrite));
        assert!(ClientGrant::for_role(ClientRole::Editor).allows(PermissionScope::GraphWrite));
        assert!(!ClientGrant::for_role(ClientRole::Editor).allows(PermissionScope::SessionControl));
        assert!(ClientGrant::for_role(ClientRole::Operator).allows(PermissionScope::SessionControl));
        assert!(!ClientGrant::for_role(ClientRole::Operator).allows(PermissionScope::Capture));
        assert!(!ClientGrant::for_role(ClientRole::Operator)
            .allows(PermissionScope::DeviceAdministration));
        assert!(!ClientGrant::read_only().allows(PermissionScope::PluginScan));
        assert!(ClientGrant::with_scopes([PermissionScope::PluginScan])
            .allows(PermissionScope::PluginScan));
    }

    #[test]
    fn enrollment_lookup_denies_unknown_and_revoked_clients() {
        let mut plane = ControlPlane::new("enrollment-test");
        assert!(plane.grant_for_client("unknown").unwrap().is_none());
        plane.enroll_client("client", ClientRole::Editor).unwrap();
        let grant = plane.grant_for_client("client").unwrap().unwrap();
        assert!(grant.allows(PermissionScope::GraphWrite));
        assert!(!grant.allows(PermissionScope::SessionControl));
        assert!(plane.revoke_client("client").unwrap());
        assert!(plane.grant_for_client("client").unwrap().is_none());
        assert!(!plane.revoke_client("client").unwrap());
    }

    #[test]
    fn in_memory_client_enrollments_are_bounded() {
        let mut plane = ControlPlane::new("enrollment-limit");
        for index in 0..audiorouter_storage::MAX_CLIENT_ENROLLMENTS {
            plane
                .enroll_client(format!("client-{index}"), ClientRole::Observer)
                .unwrap();
        }
        assert!(matches!(
            plane.enroll_client("client-overflow", ClientRole::Observer),
            Err(ControlError::InvalidRequest(message))
                if message == "client enrollment limit reached"
        ));
        assert_eq!(
            plane.client_records().unwrap().len(),
            audiorouter_storage::MAX_CLIENT_ENROLLMENTS
        );
    }

    #[test]
    fn storage_backed_enrollment_persists_and_revokes() {
        let storage = Storage::open_memory().unwrap();
        let mut first = ControlPlane::with_storage("enrollment-persist", storage);
        first
            .enroll_client("operator", ClientRole::Operator)
            .unwrap();
        assert!(first.grant_for_client("operator").unwrap().is_some());
        assert!(first.revoke_client("operator").unwrap());
        assert!(first.grant_for_client("operator").unwrap().is_none());
    }

    #[test]
    fn file_backed_enrollment_authorizes_after_control_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-enrollment-restart-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        {
            let mut first =
                ControlPlane::with_storage("enrollment-first", Storage::open(&path).unwrap());
            first
                .enroll_client("operator", ClientRole::Operator)
                .unwrap();
        }
        let mut second =
            ControlPlane::with_storage("enrollment-second", Storage::open(&path).unwrap());
        let grant = second.grant_for_client("operator").unwrap().unwrap();
        assert!(grant.allows(PermissionScope::SessionControl));
        let response = second.dispatch_authorized_for_client(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(22)),
                method: "recovery.clearSafeMode".into(),
                params: Some(json!({ "idempotencyKey": "recovery-clear-3" })),
            },
            "operator",
            &grant,
        );
        assert_eq!(response.result.unwrap()["safeMode"], false);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn read_only_notifications_produce_no_response() {
        let mut plane = ControlPlane::default();
        let notification = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: None,
            method: "system.describe".into(),
            params: None,
        };
        assert!(plane
            .dispatch_message(RpcMessage::Single(notification))
            .is_empty());
    }

    #[test]
    fn corrupt_database_error_has_non_retryable_recovery_code() {
        let response = application_error_response(
            Some(json!(1)),
            ControlError::CorruptDatabase("integrity check failed".into()),
        );
        let error = response.error.unwrap();
        let data = error.data.unwrap();
        assert_eq!(data["code"], "corruptDatabase");
        assert_eq!(data["retryable"], false);
    }

    #[test]
    fn audio_error_response_preserves_hresult_and_contention_guidance() {
        let response = application_error_response(
            Some(json!(1)),
            ControlError::Audio {
                code: "deviceInUse",
                hresult: 0x8889_000A,
                retryable: true,
                remediation:
                    "identify the owning stream, select another endpoint, or close it and retry",
                message: "audio endpoint is busy".into(),
            },
        );
        let error = response.error.unwrap();
        assert_eq!(error.message, "audio endpoint is busy");
        let data = error.data.unwrap();
        assert_eq!(data["code"], "deviceInUse");
        assert_eq!(data["hresult"], 0x8889_000A_u32);
        assert_eq!(data["retryable"], true);
        assert_eq!(
            data["remediation"],
            "identify the owning stream, select another endpoint, or close it and retry"
        );
    }

    #[test]
    fn storage_error_mapping_does_not_leak_os_paths() {
        let mapped = storage_error(StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            r"C:\private\recordings\secret.wav",
        )));
        assert_eq!(
            mapped,
            ControlError::Storage("storage I/O operation failed".into())
        );
        let response = application_error_response(Some(json!(1)), mapped);
        let message = response.error.unwrap().message;
        assert!(!message.contains("secret.wav"));
        assert!(!message.contains("C:\\private"));
    }

    #[test]
    fn recorder_lifecycle_preserves_frame_boundaries_without_audio_access() {
        let mut plane = ControlPlane::default();
        plane.create_session(session()).unwrap();
        let grant = ClientGrant::with_scopes([PermissionScope::Read, PermissionScope::Record]);
        for (method, params) in [
            (
                "recorders.arm",
                json!({"sessionId": "session", "idempotencyKey": "arm-lifecycle"}),
            ),
            (
                "recorders.start",
                json!({"sessionId": "session", "frame": 10, "idempotencyKey": "start-lifecycle"}),
            ),
            (
                "recorders.pause",
                json!({"sessionId": "session", "frame": 20, "idempotencyKey": "pause-lifecycle"}),
            ),
            (
                "recorders.resume",
                json!({"sessionId": "session", "frame": 30, "idempotencyKey": "resume-lifecycle"}),
            ),
            (
                "recorders.split",
                json!({"sessionId": "session", "frame": 40, "idempotencyKey": "split-lifecycle"}),
            ),
            (
                "recorders.stop",
                json!({"sessionId": "session", "frame": 50, "idempotencyKey": "stop-lifecycle"}),
            ),
        ] {
            let response = plane.dispatch_authorized(
                JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(method)),
                    method: method.into(),
                    params: Some(params),
                },
                &grant,
            );
            assert!(response.error.is_none(), "{method}: {:?}", response.error);
        }
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(7)),
            method: "recorders.stop".into(),
            params: Some(
                json!({"sessionId": "session", "frame": 50, "idempotencyKey": "stop-unauthed"}),
            ),
        });
        assert!(response.error.is_some());
    }

    #[test]
    fn recorder_idempotency_replays_and_rejects_hash_conflicts() {
        let mut plane = ControlPlane::default();
        plane.create_session(session()).unwrap();
        let grant = ClientGrant::with_scopes([PermissionScope::Read, PermissionScope::Record]);
        let request = |frame| JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(frame)),
            method: "recorders.start".into(),
            params: Some(json!({
                "sessionId": "session",
                "frame": frame,
                "idempotencyKey": "start-once"
            })),
        };
        let arm = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "recorders.arm".into(),
                params: Some(json!({
                    "sessionId": "session",
                    "idempotencyKey": "arm-once"
                })),
            },
            &grant,
        );
        assert!(arm.error.is_none());
        let first = plane.dispatch_authorized(request(10), &grant);
        let first_result = first.result.clone().unwrap();
        let replay = plane.dispatch_authorized(request(10), &grant);
        assert_eq!(replay.result.unwrap(), first_result);
        let conflict = plane.dispatch_authorized(request(11), &grant);
        assert_eq!(
            conflict.error.unwrap().message,
            "idempotency key is already used for a different request"
        );
    }

    #[test]
    fn recorder_checkpoint_survives_control_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-recorder-control-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        {
            let mut plane =
                ControlPlane::with_storage("recorder-first", Storage::open(&path).unwrap());
            plane.insert_session(session()).unwrap();
            assert!(plane
                .dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(1)),
                    method: "recorders.arm".into(),
                    params: Some(json!({"sessionId": "session", "idempotencyKey": "arm-restart"})),
                })
                .result
                .is_some());
            assert!(plane
                .dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(2)),
                    method: "recorders.start".into(),
                    params: Some(json!({"sessionId": "session", "frame": 128, "idempotencyKey": "start-restart"})),
                })
                .result
                .is_some());
        }
        let mut restarted =
            ControlPlane::with_storage("recorder-second", Storage::open(&path).unwrap());
        let response = restarted.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "recorders.pause".into(),
            params: Some(json!({
                "sessionId": "session",
                "frame": 256,
                "idempotencyKey": "pause-restart"
            })),
        });
        assert_eq!(response.result.unwrap()["state"], "paused");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn session_import_plan_and_commit_are_validated_and_idempotent() {
        let mut plane = ControlPlane::default();
        let mut imported = session();
        imported.id = EntityId::new("imported-session");
        let duplicate_candidate = imported.clone();
        let planned = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "sessions.importPlan".into(),
            params: Some(json!({"session": imported})),
        });
        let plan = planned.result.unwrap();
        assert_eq!(plan["session"]["id"], "imported-session");
        let commit = |id| JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(id)),
            method: "sessions.importCommit".into(),
            params: Some(json!({
                "planId": plan["planId"],
                "idempotencyKey": "import-once"
            })),
        };
        let first = plane.dispatch(commit(2));
        assert_eq!(first.result.as_ref().unwrap()["state"], "stopped");
        let replay = plane.dispatch(commit(3));
        assert_eq!(replay.result.unwrap(), first.result.unwrap());
        let duplicate = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "sessions.importPlan".into(),
            params: Some(json!({"session": duplicate_candidate})),
        });
        assert!(duplicate.error.is_some());
    }

    #[test]
    fn processors_response_uses_bounded_shared_eq_coefficients() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "processors.response".into(),
            params: Some(json!({
                "sampleRateHz": 48_000.0,
                "bands": [{"enabled": true, "type": "peaking", "frequencyHz": 1000.0, "q": 1.0, "gainDb": 6.0}],
                "frequenciesHz": [100.0, 1000.0, 10_000.0]
            })),
        });
        let result = response.result.unwrap();
        assert_eq!(result["frequenciesHz"], json!([100.0, 1000.0, 10000.0]));
        assert_eq!(result["magnitudeDb"].as_array().unwrap().len(), 3);
        assert!(result["magnitudeDb"][1].as_f64().unwrap() > 5.0);
        let invalid = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "processors.response".into(),
            params: Some(json!({"sampleRateHz": 48_000.0, "bands": [], "frequenciesHz": []})),
        });
        assert!(invalid.error.is_some());
    }
}
