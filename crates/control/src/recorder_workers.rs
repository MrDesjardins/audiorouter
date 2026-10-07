//! File recorder workers: WAV, segmented WAV, FLAC and MP3 writers behind the `RecorderWorker` trait.

use super::*;

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

/// Reports the quantization policy that was actually applied to a WAV file.
/// Float32 output is written without quantization dithering even when a
/// caller supplies the generic dither option, so its persisted metadata must
/// not claim that TPDF noise was applied.
pub(crate) fn effective_wav_dither(format: WavFormat, requested: bool) -> bool {
    requested && !matches!(format, WavFormat::Float32)
}

pub(crate) fn default_dither_for_format(format: FileRecorderFormat) -> bool {
    !matches!(
        format,
        FileRecorderFormat::Wav(WavFormat::Float32) | FileRecorderFormat::Mp3
    )
}

/// Explicit identity required before a file worker may publish a library row.
/// The worker never guesses session, recorder, or path ownership from a file
/// handle; callers must provide all three on the lifecycle thread.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileRecordingIdentity {
    pub session_id: String,
    pub recorder_id: String,
    pub path: std::path::PathBuf,
}

pub(crate) fn finalized_flac_recording(
    identity: &FileRecordingIdentity,
    run_id: &str,
    start_time: &str,
    dither: bool,
    conversion: String,
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
        dither,
        conversion,
    })
}

pub(crate) fn finalized_wav_recording(
    identity: &FileRecordingIdentity,
    run_id: &str,
    start_time: &str,
    dither: bool,
    conversion: String,
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
        dither,
        conversion,
    })
}

// Each argument is a distinct field of the finalized recording record.
#[allow(clippy::too_many_arguments)]
pub(crate) fn finalized_mp3_recording(
    identity: &FileRecordingIdentity,
    run_id: &str,
    start_time: &str,
    channels: u16,
    sample_rate: u32,
    frames: u64,
    dither: bool,
    conversion: String,
) -> Result<FinalizedRecording, String> {
    let file_bytes = std::fs::metadata(&identity.path)
        .map_err(|error| format!("MP3 recording metadata failed: {error}"))?
        .len();
    let path = identity
        .path
        .to_str()
        .ok_or_else(|| "MP3 recording path is not valid Unicode".to_owned())?
        .to_owned();
    let id = format!(
        "{}-{}-{}",
        identity.session_id, identity.recorder_id, run_id
    );
    if id.len() > MAX_RECORDING_ID_BYTES
        || identity.session_id.is_empty()
        || identity.recorder_id.is_empty()
    {
        return Err("MP3 recording identity exceeds its bound".into());
    }
    Ok(FinalizedRecording {
        id,
        session_id: identity.session_id.clone(),
        recorder_id: identity.recorder_id.clone(),
        path,
        format: "mp3".into(),
        channels,
        sample_rate,
        frames,
        file_bytes,
        start_time: start_time.to_owned(),
        state: "completed".into(),
        missing: false,
        title: None,
        artist: None,
        comment: None,
        dither,
        conversion,
    })
}

/// Backend-owned recording worker boundary. Implementations own their queue,
/// encoder, and destination handle; the control plane owns the lifecycle
/// decision and will stop a session only after this method reports a finalized
/// file. The frame is the last committed control-plane boundary.
pub trait RecorderWorker: Send {
    /// Shared frame/admission state for off-thread file encoders.
    fn shared_recording_queue(&self) -> Option<Arc<RecordingQueue>> {
        None
    }
    /// Exposes the worker's preallocated queue observer for a prepared engine
    /// tap set. The control plane never invokes it from the audio callback.
    fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
        None
    }

    /// The frame just past the newest audio its tap committed; stopping or
    /// splitting there keeps everything received so far.
    fn committed_end_frame(&self) -> Option<u64> {
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

    /// Drain a bounded amount of queued audio on the lifecycle/control
    /// thread. Implementations return zero while armed or idle; the realtime
    /// tap only enqueues into the preallocated queue and never calls this.
    fn drain_pending(&mut self, _maximum_chunks: usize) -> Result<usize, String> {
        Ok(0)
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
    format: WavFormat,
    channels: u16,
    sample_rate: u32,
    dither: bool,
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
            format,
            channels,
            sample_rate,
            dither,
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
    fn shared_recording_queue(&self) -> Option<Arc<RecordingQueue>> {
        Some(self.queue.clone())
    }
    fn committed_end_frame(&self) -> Option<u64> {
        self.queue.committed_end_frame()
    }

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

    fn drain_pending(&mut self, maximum_chunks: usize) -> Result<usize, String> {
        let Some(recorder) = self.recorder.as_mut() else {
            return Ok(0);
        };
        if !matches!(
            recorder.state(),
            RecorderState::Recording | RecorderState::Stopping
        ) {
            return Ok(0);
        }
        recorder
            .drain_queue(&self.queue, maximum_chunks)
            .map_err(|error| format!("WAV recorder drain failed: {error:?}"))
    }

    fn arm(&mut self) -> Result<(), String> {
        WavRecorderWorker::arm(self)
    }

    fn start(&mut self, frame: u64) -> Result<(), String> {
        WavRecorderWorker::start(self, frame)
    }

    fn pause(&mut self, frame: u64) -> Result<(), String> {
        let queue = Arc::clone(&self.queue);
        drain_before_recorder_pause(self, &queue)?;
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
            .map_err(|error| format!("WAV recorder resume failed: {error:?}"))?;
        self.queue.open_tap_admission();
        Ok(())
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
            self.finalized_recordings = vec![finalized_wav_recording(
                identity,
                run_id,
                start_time,
                effective_wav_dither(self.format, self.dither),
                format!(
                    "targetSampleRate={};channels={};format={:?}",
                    self.sample_rate, self.channels, self.format
                ),
            )?];
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
    Mp3,
}

impl FileRecorderFormat {
    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Wav(_) => "wav",
            Self::Flac { .. } => "flac",
            Self::Mp3 => "mp3",
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
    pub(crate) fn validate(&self) -> Result<(), String> {
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
    let (path, worker): (std::path::PathBuf, Box<dyn RecorderWorker>) = match config.format {
        FileRecorderFormat::Wav(wav_format) => {
            let max_segment_frames = audiorouter_recording::default_wav_segment_frames(
                wav_format,
                config.channels,
                config.sample_rate,
            )
            .map_err(|error| format!("invalid default WAV segment boundary: {error:?}"))?;
            let worker = SegmentedWavRecorderWorker::new_with_dither_and_sequence(
                policy.clone(),
                config.session_id,
                config.recorder_id,
                wav_format,
                config.channels,
                config.sample_rate,
                config.dither,
                config.sequence,
                config.queue_capacity,
                config.maximum_chunks_per_pass,
                max_segment_frames,
            )?;
            let path = worker.initial_path().to_owned();
            (path, Box::new(worker))
        }
        FileRecorderFormat::Flac { bits_per_sample } => {
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
            let mut worker = StreamingFlacRecorderWorker::new(
                file,
                config.channels,
                config.sample_rate,
                bits_per_sample,
                config.dither,
                config.queue_capacity,
                config.maximum_chunks_per_pass,
            )?;
            worker.set_library_identity(identity);
            (path, Box::new(worker))
        }
        FileRecorderFormat::Mp3 => {
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
            let mut worker = Mp3RecorderWorker::new(
                file,
                config.channels,
                config.sample_rate,
                config.dither,
                config.queue_capacity,
                config.maximum_chunks_per_pass,
            )?;
            worker.set_library_identity(identity);
            (path, Box::new(worker))
        }
    };
    let worker =
        threaded_recorder::ThreadedRecorderWorker::new(worker, config.maximum_chunks_per_pass)?;
    Ok((path, Box::new(worker)))
}

pub(crate) type SegmentedWavFactory =
    Box<dyn FnMut(u32) -> Result<std::fs::File, RecordingError> + Send>;

/// Control-plane worker that creates bounded WAV segments through the
/// approved recording path policy. File creation and rotation stay on the
/// worker/lifecycle side; the audio tap only submits pooled chunks.
pub struct SegmentedWavRecorderWorker {
    recorder: Option<SegmentedWavRecorder<std::fs::File, SegmentedWavFactory>>,
    pub(crate) queue: Arc<RecordingQueue>,
    initial_path: std::path::PathBuf,
    maximum_chunks_per_pass: usize,
    session_id: String,
    recorder_id: String,
    format: WavFormat,
    channels: u16,
    sample_rate: u32,
    dither: bool,
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
        Self::new_with_dither(
            policy,
            session,
            recorder_name,
            format,
            channels,
            sample_rate,
            false,
            queue_capacity,
            maximum_chunks_per_pass,
            max_segment_frames,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_dither(
        policy: RecordingPathPolicy,
        session: &str,
        recorder_name: &str,
        format: WavFormat,
        channels: u16,
        sample_rate: u32,
        dither: bool,
        queue_capacity: usize,
        maximum_chunks_per_pass: usize,
        max_segment_frames: u64,
    ) -> Result<Self, String> {
        Self::new_with_dither_and_sequence(
            policy,
            session,
            recorder_name,
            format,
            channels,
            sample_rate,
            dither,
            0,
            queue_capacity,
            maximum_chunks_per_pass,
            max_segment_frames,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_with_dither_and_sequence(
        policy: RecordingPathPolicy,
        session: &str,
        recorder_name: &str,
        format: WavFormat,
        channels: u16,
        sample_rate: u32,
        dither: bool,
        sequence: u64,
        queue_capacity: usize,
        maximum_chunks_per_pass: usize,
        max_segment_frames: u64,
    ) -> Result<Self, String> {
        if maximum_chunks_per_pass == 0 {
            return Err("maximum recorder drain pass must be positive".into());
        }
        let (initial_path, initial_file) = policy
            .create_file(session, recorder_name, sequence, "wav")
            .map_err(format_path_policy_error)?;
        let session_id = session.to_owned();
        let recorder_id = recorder_name.to_owned();
        let first_path = initial_path.clone();
        let paths = Arc::new(std::sync::Mutex::new(vec![initial_path]));
        let factory_paths = paths.clone();
        let factory_session = session_id.clone();
        let factory_recorder = recorder_id.clone();
        let factory: SegmentedWavFactory = Box::new(move |index| {
            let sequence = sequence
                .checked_add(u64::from(index))
                .ok_or(RecordingError::TooManyFrames)?;
            let (path, file) = policy
                .create_file(&factory_session, &factory_recorder, sequence, "wav")
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
            dither,
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
            initial_path: first_path,
            maximum_chunks_per_pass,
            session_id,
            recorder_id,
            format,
            channels,
            sample_rate,
            dither,
            paths,
            started_at: None,
            run_id: None,
            finalized_recordings: Vec::new(),
        })
    }

    pub fn initial_path(&self) -> &std::path::Path {
        &self.initial_path
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
    fn shared_recording_queue(&self) -> Option<Arc<RecordingQueue>> {
        Some(self.queue.clone())
    }
    fn committed_end_frame(&self) -> Option<u64> {
        self.queue.committed_end_frame()
    }

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

    fn drain_pending(&mut self, maximum_chunks: usize) -> Result<usize, String> {
        let Some(recorder) = self.recorder.as_mut() else {
            return Ok(0);
        };
        if !matches!(
            recorder.state(),
            RecorderState::Recording | RecorderState::Stopping
        ) {
            return Ok(0);
        }
        recorder
            .drain_queue(&self.queue, maximum_chunks)
            .map_err(|error| format!("segmented WAV recorder drain failed: {error:?}"))
    }

    fn pause(&mut self, frame: u64) -> Result<(), String> {
        let queue = Arc::clone(&self.queue);
        drain_before_recorder_pause(self, &queue)?;
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
            .map_err(|error| format!("segmented WAV recorder resume failed: {error:?}"))?;
        self.queue.open_tap_admission();
        Ok(())
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
        // REC-08: a recorder that failed mid-take keeps a playable prefix.
        let mut failed = recorder.state() == RecorderState::Failed;
        let outputs = if failed {
            recorder
                .finish_failed()
                .map_err(|error| format!("segmented WAV prefix finalization failed: {error:?}"))?
        } else {
            let mut completed = false;
            for _ in 0..MAX_RECORDER_FINALIZATION_PASSES {
                match recorder.stop_and_drain(&self.queue, frame, self.maximum_chunks_per_pass) {
                    Ok(_) => {
                        completed = true;
                        break;
                    }
                    Err(RecordingError::QueueNotEmpty) => continue,
                    // A gap found in the queued audio: keep the prefix.
                    Err(_) if recorder.state() == RecorderState::Failed => break,
                    Err(error) => {
                        self.recorder = Some(recorder);
                        return Err(format!("segmented WAV finalization failed: {error:?}"));
                    }
                }
            }
            if !completed && recorder.state() != RecorderState::Failed {
                self.recorder = Some(recorder);
                return Err("segmented WAV finalization exceeded its bounded drain budget".into());
            }
            if recorder.state() == RecorderState::Failed {
                failed = true;
                recorder.finish_failed().map_err(|error| {
                    format!("segmented WAV prefix finalization failed: {error:?}")
                })?
            } else {
                recorder
                    .finish()
                    .map_err(|error| format!("segmented WAV file finalization failed: {error:?}"))?
            }
        };
        let final_state = if failed { "failed" } else { "completed" };
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
                state: final_state.into(),
                missing: false,
                title: None,
                artist: None,
                comment: None,
                dither: effective_wav_dither(self.format, self.dither),
                conversion: format!(
                    "targetSampleRate={};channels={};format=wav",
                    self.sample_rate, self.channels
                ),
            });
        }
        self.finalized_recordings = finalized;
        Ok(RecorderFinalizationOutcome {
            state: final_state.into(),
            file_finalized: true,
            recoverable: false,
        })
    }
}

pub(crate) fn format_path_policy_error(error: PathPolicyError) -> String {
    format!("recording path policy rejected file creation: {error:?}")
}

/// Concrete buffered-FLAC worker for offline and compatibility recording
/// integration. The encoder retains compressed output until finalization;
/// the file-recorder factory uses the streaming worker for live attachment.
pub struct BufferedFlacRecorderWorker {
    recorder: Option<BufferedFlacRecorder>,
    queue: Arc<RecordingQueue>,
    output: Option<std::fs::File>,
    maximum_chunks_per_pass: usize,
    channels: usize,
    sample_rate: u32,
    bits_per_sample: u8,
    dither: bool,
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
        Self::new_with_dither(
            output,
            channels,
            sample_rate,
            bits_per_sample,
            false,
            queue_capacity,
            maximum_chunks_per_pass,
        )
    }

    pub fn new_with_dither(
        output: std::fs::File,
        channels: usize,
        sample_rate: u32,
        bits_per_sample: u8,
        dither: bool,
        queue_capacity: usize,
        maximum_chunks_per_pass: usize,
    ) -> Result<Self, String> {
        if maximum_chunks_per_pass == 0 {
            return Err("maximum recorder drain pass must be positive".into());
        }
        let recorder =
            BufferedFlacRecorder::new_with_dither(channels, sample_rate, bits_per_sample, dither)
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
            channels,
            sample_rate,
            bits_per_sample,
            dither,
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
    fn committed_end_frame(&self) -> Option<u64> {
        self.queue.committed_end_frame()
    }

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

    fn drain_pending(&mut self, maximum_chunks: usize) -> Result<usize, String> {
        let Some(recorder) = self.recorder.as_mut() else {
            return Ok(0);
        };
        if !matches!(
            recorder.state(),
            RecorderState::Recording | RecorderState::Stopping
        ) {
            return Ok(0);
        }
        recorder
            .drain_queue(&self.queue, maximum_chunks)
            .map_err(|error| format!("buffered FLAC recorder drain failed: {error:?}"))
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
        let queue = Arc::clone(&self.queue);
        drain_before_recorder_pause(self, &queue)?;
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
            .map_err(|error| format!("FLAC recorder resume failed: {error:?}"))?;
        self.queue.open_tap_admission();
        Ok(())
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
            self.finalized_recordings = vec![finalized_flac_recording(
                identity,
                run_id,
                start_time,
                self.dither,
                format!(
                    "targetSampleRate={};channels={};bitsPerSample={}",
                    self.sample_rate, self.channels, self.bits_per_sample
                ),
            )?];
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
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u8,
    dither: bool,
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
        dither: bool,
        queue_capacity: usize,
        maximum_chunks_per_pass: usize,
    ) -> Result<Self, String> {
        if maximum_chunks_per_pass == 0 {
            return Err("maximum recorder drain pass must be positive".into());
        }
        let writer =
            StreamingFlacWriter::new(output, channels, sample_rate, bits_per_sample, dither)
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
            channels,
            sample_rate,
            bits_per_sample,
            dither,
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
    fn shared_recording_queue(&self) -> Option<Arc<RecordingQueue>> {
        Some(self.queue.clone())
    }
    fn committed_end_frame(&self) -> Option<u64> {
        self.queue.committed_end_frame()
    }

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

    fn drain_pending(&mut self, maximum_chunks: usize) -> Result<usize, String> {
        let Some(recorder) = self.recorder.as_mut() else {
            return Ok(0);
        };
        if !matches!(
            recorder.state(),
            RecorderState::Recording | RecorderState::Stopping
        ) {
            return Ok(0);
        }
        recorder
            .drain_queue(&self.queue, maximum_chunks)
            .map_err(|error| format!("streaming FLAC recorder drain failed: {error:?}"))
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
        let queue = Arc::clone(&self.queue);
        drain_before_recorder_pause(self, &queue)?;
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
            .map_err(|error| format!("streaming FLAC recorder resume failed: {error:?}"))?;
        self.queue.open_tap_admission();
        Ok(())
    }

    fn finalize(&mut self, frame: u64) -> Result<RecorderFinalizationOutcome, String> {
        let mut recorder = self
            .recorder
            .take()
            .ok_or_else(|| "streaming FLAC recorder was finalized more than once".to_owned())?;
        // REC-08: a recorder that failed mid-take keeps a playable prefix.
        let mut failed = recorder.state() == RecorderState::Failed;
        let output = if failed {
            recorder
                .finish_failed()
                .map_err(|error| format!("streaming FLAC prefix finalization failed: {error:?}"))?
        } else {
            let mut completed = false;
            for _ in 0..MAX_RECORDER_FINALIZATION_PASSES {
                match recorder.stop_and_drain(&self.queue, frame, self.maximum_chunks_per_pass) {
                    Ok(_) => {
                        completed = true;
                        break;
                    }
                    Err(RecordingError::QueueNotEmpty) => continue,
                    // A gap found in the queued audio: keep the prefix.
                    Err(_) if recorder.state() == RecorderState::Failed => break,
                    Err(error) => {
                        self.recorder = Some(recorder);
                        return Err(format!("streaming FLAC finalization failed: {error:?}"));
                    }
                }
            }
            if !completed && recorder.state() != RecorderState::Failed {
                self.recorder = Some(recorder);
                return Err(
                    "streaming FLAC recorder finalization exceeded its bounded drain budget".into(),
                );
            }
            if recorder.state() == RecorderState::Failed {
                failed = true;
                recorder.finish_failed().map_err(|error| {
                    format!("streaming FLAC prefix finalization failed: {error:?}")
                })?
            } else {
                recorder.finish().map_err(|error| {
                    format!("streaming FLAC file finalization failed: {error:?}")
                })?
            }
        };
        let final_state = if failed { "failed" } else { "completed" };
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
            let mut recording = finalized_flac_recording(
                identity,
                run_id,
                start_time,
                self.dither,
                format!(
                    "targetSampleRate={};channels={};bitsPerSample={}",
                    self.sample_rate, self.channels, self.bits_per_sample
                ),
            )?;
            recording.state = final_state.into();
            self.finalized_recordings = vec![recording];
        }
        Ok(RecorderFinalizationOutcome {
            state: final_state.into(),
            file_finalized: true,
            recoverable: false,
        })
    }
}

/// Concrete incremental MP3 worker. Encoding and file I/O happen only while
/// the lifecycle thread drains the bounded queue; the realtime tap remains a
/// queue-only operation.
pub struct Mp3RecorderWorker {
    recorder: Option<Mp3Recorder<std::fs::File>>,
    queue: Arc<RecordingQueue>,
    maximum_chunks_per_pass: usize,
    channels: u16,
    sample_rate: u32,
    dither: bool,
    library_identity: Option<FileRecordingIdentity>,
    started_at: Option<String>,
    run_id: Option<String>,
    finalized_recordings: Vec<FinalizedRecording>,
}

impl Mp3RecorderWorker {
    pub fn new(
        output: std::fs::File,
        channels: u16,
        sample_rate: u32,
        dither: bool,
        queue_capacity: usize,
        maximum_chunks_per_pass: usize,
    ) -> Result<Self, String> {
        if maximum_chunks_per_pass == 0 {
            return Err("maximum recorder drain pass must be positive".into());
        }
        let recorder = Mp3Recorder::new(output, channels, sample_rate)
            .map_err(|error| format!("MP3 writer initialization failed: {error:?}"))?;
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
            channels,
            sample_rate,
            dither,
            library_identity: None,
            started_at: None,
            run_id: None,
            finalized_recordings: Vec::new(),
        })
    }

    pub fn set_library_identity(&mut self, identity: FileRecordingIdentity) {
        self.library_identity = Some(identity);
    }
    pub fn try_push(&self, chunk: RecordingChunk) -> Result<(), RecordingChunk> {
        self.queue.try_push(chunk)
    }
    pub fn audio_tap(&self) -> RecorderAudioTap {
        RecorderAudioTap::new(self.queue.clone())
    }
    pub(crate) fn mark_started(&mut self) {
        self.started_at = Some(unix_epoch_seconds().to_string());
        self.run_id = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
                .to_string(),
        );
    }
}

impl RecorderWorker for Mp3RecorderWorker {
    fn shared_recording_queue(&self) -> Option<Arc<RecordingQueue>> {
        Some(self.queue.clone())
    }
    fn committed_end_frame(&self) -> Option<u64> {
        self.queue.committed_end_frame()
    }

    fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
        Some(Arc::new(self.audio_tap()))
    }
    fn set_library_identity(&mut self, identity: FileRecordingIdentity) -> Result<(), String> {
        self.set_library_identity(identity);
        Ok(())
    }
    fn finalized_recordings(&self) -> Vec<FinalizedRecording> {
        self.finalized_recordings.clone()
    }
    fn drain_pending(&mut self, maximum_chunks: usize) -> Result<usize, String> {
        let Some(recorder) = self.recorder.as_mut() else {
            return Ok(0);
        };
        if !matches!(
            recorder.state(),
            RecorderState::Recording | RecorderState::Stopping
        ) {
            return Ok(0);
        }
        recorder
            .drain_queue(&self.queue, maximum_chunks)
            .map_err(|error| format!("MP3 recorder drain failed: {error:?}"))
    }
    fn arm(&mut self) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "MP3 recorder is already finalized".to_owned())?
            .arm()
            .map_err(|error| format!("MP3 recorder arm failed: {error:?}"))
    }
    fn start(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "MP3 recorder is already finalized".to_owned())?
            .start(frame)
            .map_err(|error| format!("MP3 recorder start failed: {error:?}"))?;
        self.mark_started();
        Ok(())
    }
    fn pause(&mut self, frame: u64) -> Result<(), String> {
        let queue = Arc::clone(&self.queue);
        drain_before_recorder_pause(self, &queue)?;
        self.recorder
            .as_mut()
            .ok_or_else(|| "MP3 recorder is already finalized".to_owned())?
            .pause(frame)
            .map_err(|error| format!("MP3 recorder pause failed: {error:?}"))
    }
    fn resume(&mut self, frame: u64) -> Result<(), String> {
        self.recorder
            .as_mut()
            .ok_or_else(|| "MP3 recorder is already finalized".to_owned())?
            .resume(frame)
            .map_err(|error| format!("MP3 recorder resume failed: {error:?}"))?;
        self.queue.open_tap_admission();
        Ok(())
    }
    fn split(&mut self, _frame: u64) -> Result<(), String> {
        Err("MP3 recorder splitting is not supported".into())
    }
    fn finalize(&mut self, frame: u64) -> Result<RecorderFinalizationOutcome, String> {
        let mut recorder = self
            .recorder
            .take()
            .ok_or_else(|| "MP3 recorder was finalized more than once".to_owned())?;
        // REC-08: a recorder that failed mid-take keeps a playable prefix.
        let mut failed = recorder.state() == RecorderState::Failed;
        let (mut output, frames) = if failed {
            recorder
                .finish_failed()
                .map_err(|error| format!("MP3 prefix finalization failed: {error:?}"))?
        } else {
            let mut completed = false;
            for _ in 0..MAX_RECORDER_FINALIZATION_PASSES {
                match recorder.stop_and_drain(&self.queue, frame, self.maximum_chunks_per_pass) {
                    Ok(_) => {
                        completed = true;
                        break;
                    }
                    Err(RecordingError::QueueNotEmpty) => continue,
                    // A gap found in the queued audio: keep the prefix.
                    Err(_) if recorder.state() == RecorderState::Failed => break,
                    Err(error) => {
                        self.recorder = Some(recorder);
                        return Err(format!("MP3 recorder finalization failed: {error:?}"));
                    }
                }
            }
            if !completed && recorder.state() != RecorderState::Failed {
                self.recorder = Some(recorder);
                return Err("MP3 recorder finalization exceeded its bounded drain budget".into());
            }
            if recorder.state() == RecorderState::Failed {
                failed = true;
                recorder
                    .finish_failed()
                    .map_err(|error| format!("MP3 prefix finalization failed: {error:?}"))?
            } else {
                recorder
                    .finish()
                    .map_err(|error| format!("MP3 file finalization failed: {error:?}"))?
            }
        };
        let final_state = if failed { "failed" } else { "completed" };
        std::io::Write::flush(&mut output)
            .and_then(|()| output.sync_all())
            .map_err(|error| format!("MP3 file sync failed: {error}"))?;
        if let Some(identity) = &self.library_identity {
            let start_time = self
                .started_at
                .as_deref()
                .ok_or_else(|| "MP3 finalized before start".to_owned())?;
            let run_id = self
                .run_id
                .as_deref()
                .ok_or_else(|| "MP3 finalized without a run identity".to_owned())?;
            let mut recording = finalized_mp3_recording(
                identity,
                run_id,
                start_time,
                self.channels,
                self.sample_rate,
                frames,
                self.dither,
                format!(
                    "targetSampleRate={};channels={};format=mp3;bitrateKbps=192",
                    self.sample_rate, self.channels
                ),
            )?;
            recording.state = final_state.into();
            self.finalized_recordings = vec![recording];
        }
        Ok(RecorderFinalizationOutcome {
            state: final_state.into(),
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
    pub(crate) fn new(queue: Arc<RecordingQueue>) -> Self {
        Self { queue }
    }
}

impl AudioTap for RecorderAudioTap {
    fn on_processed_block(&self, start_frame: u64, block: &AudioBlock) {
        let Some(_permit) = self.queue.try_begin_tap() else {
            return;
        };
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
        match self.queue.try_commit(chunk) {
            Ok(()) => self
                .queue
                .note_committed_end_frame(start_frame.saturating_add(block.frames() as u64)),
            Err(chunk) => self.queue.recycle(chunk),
        }
    }
}

/// Pause is a control-thread boundary: retire admitted callbacks and write
/// pre-pause quanta before the writer's timeline advances to Resume.
pub(crate) fn drain_before_recorder_pause(
    worker: &mut dyn RecorderWorker,
    queue: &RecordingQueue,
) -> Result<(), String> {
    queue.close_tap_admission();
    let deadline = Instant::now() + Duration::from_millis(100);
    while queue.taps_in_flight() {
        if Instant::now() >= deadline {
            return Err("recorder callback retirement timed out".into());
        }
        std::thread::yield_now();
    }
    let pending = queue.len();
    if pending > 0 && worker.drain_pending(pending)? != pending {
        return Err("recorder pre-pause queue did not drain".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "recorder_workers_tests.rs"]
mod tests;
