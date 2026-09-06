//! Portable plugin inspection and failure policy.
//!
//! This crate intentionally does not load or execute plugin code. Discovery
//! produces identity evidence for a later disposable worker boundary.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

pub const MAX_PLUGIN_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_FAILURES_BEFORE_QUARANTINE: u32 = 3;
pub const FAILURE_WINDOW: Duration = Duration::from_secs(10 * 60);
pub const MAX_WORKER_FRAMES: usize = 2048;
pub const MAX_SCAN_CANDIDATES: usize = 256;
pub const DEFAULT_SCAN_DEADLINE: Duration = Duration::from_secs(10);
pub const MAX_PLUGIN_STATE_BYTES: usize = 16 * 1024 * 1024;
pub const WORKER_HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(100);
pub const MAX_PARAMETER_EVENTS: usize = 128;
pub const MAX_WORKER_MESSAGE_BYTES: usize = 1_024 * 1_024;
pub const WORKER_PROTOCOL_VERSION: u16 = 1;
pub const MAX_WORKER_LATENCY_MS: u32 = 10_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginFormat {
    Vst3,
    Vst2,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginCompatibility {
    SupportedVst3X64,
    UnsupportedFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PeArchitecture {
    X64,
    X86,
    Arm64,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InspectionError {
    OutsideConfiguredRoot,
    UnsupportedExtension,
    Missing,
    TooLarge,
    NotPe,
    UnsupportedArchitecture,
    Io(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateError {
    Empty,
    TooLarge,
    VersionMismatch,
    IntegrityMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateFileError {
    InvalidRoot,
    InvalidAssetId,
    OutsideRoot,
    Exists,
    TooLarge,
    Io(String),
    InvalidState(StateError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginStateAsset {
    pub version: u32,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

impl PluginStateAsset {
    pub fn new(version: u32, bytes: Vec<u8>) -> Result<Self, StateError> {
        if bytes.is_empty() {
            return Err(StateError::Empty);
        }
        if bytes.len() > MAX_PLUGIN_STATE_BYTES {
            return Err(StateError::TooLarge);
        }
        let sha256 = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Ok(Self {
            version,
            bytes,
            sha256,
        })
    }

    pub fn verify_for_restore(&self, expected_version: u32) -> Result<&[u8], StateError> {
        if self.version != expected_version {
            return Err(StateError::VersionMismatch);
        }
        let digest: String = Sha256::digest(&self.bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if digest != self.sha256 {
            return Err(StateError::IntegrityMismatch);
        }
        Ok(&self.bytes)
    }
}

pub fn write_state_asset(
    root: &Path,
    asset_id: &str,
    asset: &PluginStateAsset,
) -> Result<PathBuf, StateFileError> {
    asset
        .verify_for_restore(asset.version)
        .map_err(StateFileError::InvalidState)?;
    let canonical_root = fs::canonicalize(root).map_err(|_| StateFileError::InvalidRoot)?;
    if !canonical_root.is_dir() {
        return Err(StateFileError::InvalidRoot);
    }
    if !is_safe_asset_id(asset_id) {
        return Err(StateFileError::InvalidAssetId);
    }
    let path = canonical_root.join(format!("{asset_id}.bin"));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                StateFileError::Exists
            } else {
                StateFileError::Io(error.to_string())
            }
        })?;
    file.write_all(&asset.bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| StateFileError::Io(error.to_string()))?;
    Ok(path)
}

pub fn read_state_asset(
    root: &Path,
    path: &Path,
    version: u32,
    expected_sha256: &str,
) -> Result<PluginStateAsset, StateFileError> {
    let canonical_root = fs::canonicalize(root).map_err(|_| StateFileError::InvalidRoot)?;
    let canonical_path =
        fs::canonicalize(path).map_err(|error| StateFileError::Io(error.to_string()))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(StateFileError::OutsideRoot);
    }
    let metadata =
        fs::metadata(&canonical_path).map_err(|error| StateFileError::Io(error.to_string()))?;
    if metadata.len() > MAX_PLUGIN_STATE_BYTES as u64 {
        return Err(StateFileError::TooLarge);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(canonical_path)
        .map_err(|error| StateFileError::Io(error.to_string()))?
        .read_to_end(&mut bytes)
        .map_err(|error| StateFileError::Io(error.to_string()))?;
    let asset = PluginStateAsset::new(version, bytes).map_err(StateFileError::InvalidState)?;
    if !is_sha256(expected_sha256) || asset.sha256 != expected_sha256 {
        return Err(StateFileError::InvalidState(StateError::IntegrityMismatch));
    }
    asset
        .verify_for_restore(version)
        .map_err(StateFileError::InvalidState)?;
    Ok(asset)
}

fn is_safe_asset_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginIdentity {
    pub path: PathBuf,
    pub binary_path: PathBuf,
    pub format: PluginFormat,
    pub architecture: PeArchitecture,
    pub file_bytes: u64,
    pub sha256: String,
}

impl PluginIdentity {
    pub fn compatibility(&self) -> PluginCompatibility {
        if self.format == PluginFormat::Vst3 && self.architecture == PeArchitecture::X64 {
            PluginCompatibility::SupportedVst3X64
        } else {
            PluginCompatibility::UnsupportedFormat
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanEntry {
    pub path: PathBuf,
    pub identity: Option<PluginIdentity>,
    pub error: Option<InspectionError>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScanError {
    InvalidRoot,
    TooManyCandidates,
    Cancelled,
    DeadlineExceeded,
    Io(String),
}

#[derive(Clone, Debug)]
pub struct ScanControl {
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
}

impl ScanControl {
    pub fn with_deadline(deadline: Instant) -> Self {
        Self {
            deadline,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn default_deadline() -> Self {
        Self::with_deadline(Instant::now() + DEFAULT_SCAN_DEADLINE)
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    fn check(&self) -> Result<(), ScanError> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(ScanError::Cancelled);
        }
        if Instant::now() >= self.deadline {
            return Err(ScanError::DeadlineExceeded);
        }
        Ok(())
    }
}

pub fn inspect_binary(
    path: &Path,
    configured_roots: &[PathBuf],
) -> Result<PluginIdentity, InspectionError> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            InspectionError::Missing
        } else {
            InspectionError::Io(error.to_string())
        }
    })?;
    if !configured_roots
        .iter()
        .any(|root| root_contains(root, &canonical))
    {
        return Err(InspectionError::OutsideConfiguredRoot);
    }
    let format = match canonical
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("vst3") => PluginFormat::Vst3,
        // A DLL extension alone does not prove VST2; retain the binary as an
        // inspected unknown candidate rather than advertising compatibility.
        Some("dll") => PluginFormat::Unknown,
        _ => return Err(InspectionError::UnsupportedExtension),
    };
    let binary_path = resolve_binary_path(&canonical)?;
    if !configured_roots
        .iter()
        .any(|root| root_contains(root, &binary_path))
    {
        return Err(InspectionError::OutsideConfiguredRoot);
    }
    let metadata =
        fs::metadata(&binary_path).map_err(|error| InspectionError::Io(error.to_string()))?;
    if metadata.len() > MAX_PLUGIN_BYTES {
        return Err(InspectionError::TooLarge);
    }
    let bytes = fs::read(&binary_path).map_err(|error| InspectionError::Io(error.to_string()))?;
    let architecture = parse_pe_architecture(&bytes).ok_or(InspectionError::NotPe)?;
    if architecture != PeArchitecture::X64 {
        return Err(InspectionError::UnsupportedArchitecture);
    }
    let digest = Sha256::digest(&bytes);
    Ok(PluginIdentity {
        path: canonical,
        binary_path,
        format,
        architecture,
        file_bytes: metadata.len(),
        sha256: digest.iter().map(|byte| format!("{byte:02x}")).collect(),
    })
}

fn resolve_binary_path(path: &Path) -> Result<PathBuf, InspectionError> {
    if !path.is_dir() {
        return Ok(path.to_path_buf());
    }
    let contents = path.join("Contents").join("x86_64-win");
    let mut binaries = fs::read_dir(&contents)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                InspectionError::Missing
            } else {
                InspectionError::Io(error.to_string())
            }
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|candidate| candidate.is_file())
        .collect::<Vec<_>>();
    binaries.sort();
    if binaries.len() != 1 {
        return Err(InspectionError::NotPe);
    }
    fs::canonicalize(&binaries[0]).map_err(|error| InspectionError::Io(error.to_string()))
}

/// Enumerates one explicitly selected directory without executing its files.
/// Every candidate is returned, including unsupported/error entries.
pub fn scan_directory(root: &Path) -> Result<Vec<ScanEntry>, ScanError> {
    scan_directory_with_control(root, &ScanControl::default_deadline())
}

pub fn scan_directory_with_control(
    root: &Path,
    control: &ScanControl,
) -> Result<Vec<ScanEntry>, ScanError> {
    if !root.is_dir() {
        return Err(ScanError::InvalidRoot);
    }
    control.check()?;
    let mut candidates = Vec::new();
    for entry in fs::read_dir(root).map_err(|error| ScanError::Io(error.to_string()))? {
        control.check()?;
        let entry = entry.map_err(|error| ScanError::Io(error.to_string()))?;
        let path = entry.path();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase());
        if matches!(extension.as_deref(), Some("vst3") | Some("dll")) {
            candidates.push(path);
            if candidates.len() > MAX_SCAN_CANDIDATES {
                return Err(ScanError::TooManyCandidates);
            }
        }
    }
    candidates.sort();
    let mut entries = Vec::with_capacity(candidates.len());
    for path in candidates {
        control.check()?;
        entries.push(match inspect_binary(&path, &[root.to_path_buf()]) {
            Ok(identity) => ScanEntry {
                path,
                identity: Some(identity),
                error: None,
            },
            Err(error) => ScanEntry {
                path,
                identity: None,
                error: Some(error),
            },
        });
    }
    Ok(entries)
}

fn root_contains(root: &Path, candidate: &Path) -> bool {
    fs::canonicalize(root)
        .map(|root| candidate.starts_with(root))
        .unwrap_or(false)
}

fn parse_pe_architecture(bytes: &[u8]) -> Option<PeArchitecture> {
    if bytes.len() < 0x40 || bytes.get(0..2) != Some(b"MZ") {
        return None;
    }
    let pe_offset = u32::from_le_bytes(bytes.get(0x3c..0x40)?.try_into().ok()?) as usize;
    let signature_end = pe_offset.checked_add(4)?;
    if bytes.get(pe_offset..signature_end)? != b"PE\0\0" {
        return None;
    }
    match u16::from_le_bytes(bytes.get(pe_offset + 4..pe_offset + 6)?.try_into().ok()?) {
        0x8664 => Some(PeArchitecture::X64),
        0x014c => Some(PeArchitecture::X86),
        0xaa64 => Some(PeArchitecture::Arm64),
        _ => Some(PeArchitecture::Unknown),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureLedger {
    failures: u32,
    quarantined: bool,
    window_started: Option<Instant>,
}

impl FailureLedger {
    pub fn new() -> Self {
        Self {
            failures: 0,
            quarantined: false,
            window_started: None,
        }
    }
    pub fn failures(&self) -> u32 {
        self.failures
    }
    pub fn quarantined(&self) -> bool {
        self.quarantined
    }
    pub fn record_failure(&mut self) {
        self.record_failure_at(Instant::now());
    }

    pub fn record_failure_at(&mut self, now: Instant) {
        if self
            .window_started
            .is_some_and(|started| now.saturating_duration_since(started) > FAILURE_WINDOW)
        {
            self.failures = 0;
            self.quarantined = false;
            self.window_started = None;
        }
        if self.window_started.is_none() {
            self.window_started = Some(now);
        }
        self.failures = self.failures.saturating_add(1);
        if self.failures >= MAX_FAILURES_BEFORE_QUARANTINE {
            self.quarantined = true;
        }
    }
    pub fn deliberate_retry(&mut self) {
        self.failures = 0;
        self.quarantined = false;
        self.window_started = None;
    }
}

impl Default for FailureLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerFrameError {
    InvalidChannels,
    InvalidFrameCount,
    WrongSampleCount,
    NonFiniteSample,
    SequenceRegression,
    DeadlineExpired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkerMessageError {
    TooShort,
    TooLarge { length: usize, maximum: usize },
    LengthMismatch { declared: usize, actual: usize },
    Json(String),
    InvalidFrame(WorkerFrameError),
    InvalidParameter(ParameterEventError),
    InvalidProtocolVersion,
    InvalidPluginHash,
    InvalidFailureCode,
    InvalidLatency,
    Io(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerSessionState {
    AwaitingHello,
    AwaitingReady,
    Active,
    Closed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerSessionError {
    InvalidExpectedHash,
    InvalidChannels,
    UnexpectedMessage,
    IdentityMismatch,
    Frame(WorkerFrameError),
    InvalidLatency,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerFailureAction {
    Silence,
    DryFallback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkerFailurePolicy {
    protected_path: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerState {
    Stopped,
    Running,
    Failed,
    Quarantined,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkerStartError {
    UnsupportedPlugin,
    Quarantined,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerSupervisor {
    state: WorkerState,
    last_heartbeat: Option<Instant>,
    failures: FailureLedger,
}

impl WorkerSupervisor {
    pub fn new() -> Self {
        Self {
            state: WorkerState::Stopped,
            last_heartbeat: None,
            failures: FailureLedger::new(),
        }
    }

    pub fn state(&self) -> WorkerState {
        self.state
    }

    /// Records lifecycle policy only; process creation belongs to the native worker adapter.
    pub fn start(
        &mut self,
        identity: &PluginIdentity,
        now: Instant,
    ) -> Result<(), WorkerStartError> {
        if self.failures.quarantined() {
            self.state = WorkerState::Quarantined;
            return Err(WorkerStartError::Quarantined);
        }
        if identity.format != PluginFormat::Vst3 || identity.architecture != PeArchitecture::X64 {
            return Err(WorkerStartError::UnsupportedPlugin);
        }
        self.state = WorkerState::Running;
        self.last_heartbeat = Some(now);
        Ok(())
    }

    pub fn heartbeat(&mut self, now: Instant) -> bool {
        if self.state != WorkerState::Running {
            return false;
        }
        self.last_heartbeat = Some(now);
        true
    }

    pub fn poll(&mut self, now: Instant) -> WorkerState {
        if self.state == WorkerState::Running
            && self
                .last_heartbeat
                .is_some_and(|last| now.saturating_duration_since(last) > WORKER_HEARTBEAT_TIMEOUT)
        {
            self.failures.record_failure_at(now);
            self.state = if self.failures.quarantined() {
                WorkerState::Quarantined
            } else {
                WorkerState::Failed
            };
        }
        self.state
    }

    pub fn deliberate_retry(&mut self) {
        self.failures.deliberate_retry();
        self.state = WorkerState::Stopped;
        self.last_heartbeat = None;
    }
}

impl Default for WorkerSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerFailurePolicy {
    pub fn new(protected_path: bool) -> Self {
        Self { protected_path }
    }

    pub fn on_failure(self) -> WorkerFailureAction {
        if self.protected_path {
            WorkerFailureAction::Silence
        } else {
            WorkerFailureAction::DryFallback
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerFrame {
    pub sequence: u64,
    pub deadline_tick: u64,
    pub channels: u16,
    pub samples: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedFrameQueue {
    frames: VecDeque<WorkerFrame>,
    capacity: usize,
    overflow_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParameterEvent {
    pub parameter_id: u32,
    pub normalized_value: f32,
    pub sample_offset: usize,
}

/// Control messages for the future disposable native worker. Audio payloads
/// are bounded here for testability; the production transport may replace the
/// samples with shared-memory handles without changing lifecycle semantics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WorkerMessage {
    Hello {
        protocol_version: u16,
        plugin_sha256: String,
        channels: u16,
    },
    Ready,
    Process {
        frame: WorkerFrame,
        parameters: Vec<ParameterEvent>,
    },
    Processed {
        frame: WorkerFrame,
    },
    Latency(WorkerLatency),
    Shutdown,
    Failure {
        code: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerLatency {
    pub samples: u32,
    pub sample_rate_hz: u32,
}

impl WorkerLatency {
    pub fn new(samples: u32, sample_rate_hz: u32) -> Result<Self, WorkerMessageError> {
        if !(8_000..=192_000).contains(&sample_rate_hz)
            || samples > sample_rate_hz.saturating_mul(MAX_WORKER_LATENCY_MS) / 1_000
        {
            return Err(WorkerMessageError::InvalidLatency);
        }
        Ok(Self {
            samples,
            sample_rate_hz,
        })
    }

    pub fn milliseconds(self) -> f32 {
        self.samples as f32 * 1_000.0 / self.sample_rate_hz as f32
    }
}

/// Stateful handshake and frame gate for one disposable worker instance.
/// Process creation and OS-level isolation remain outside this portable type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerSession {
    expected_plugin_sha256: String,
    channels: u16,
    state: WorkerSessionState,
    frame_guard: WorkerFrameGuard,
}

impl WorkerSession {
    pub fn new(
        expected_plugin_sha256: impl Into<String>,
        channels: u16,
    ) -> Result<Self, WorkerSessionError> {
        let expected_plugin_sha256 = expected_plugin_sha256.into();
        if !is_sha256(&expected_plugin_sha256) {
            return Err(WorkerSessionError::InvalidExpectedHash);
        }
        if !matches!(channels, 1 | 2) {
            return Err(WorkerSessionError::InvalidChannels);
        }
        Ok(Self {
            expected_plugin_sha256,
            channels,
            state: WorkerSessionState::AwaitingHello,
            frame_guard: WorkerFrameGuard::new(),
        })
    }

    pub fn state(&self) -> WorkerSessionState {
        self.state
    }

    pub fn accept(
        &mut self,
        message: &WorkerMessage,
        now_tick: u64,
    ) -> Result<Option<WorkerFrame>, WorkerSessionError> {
        validate_worker_message(message).map_err(|error| match error {
            WorkerMessageError::InvalidFrame(error) => WorkerSessionError::Frame(error),
            WorkerMessageError::InvalidLatency => WorkerSessionError::InvalidLatency,
            _ => WorkerSessionError::UnexpectedMessage,
        })?;
        match (&self.state, message) {
            (
                WorkerSessionState::AwaitingHello,
                WorkerMessage::Hello {
                    protocol_version: _,
                    plugin_sha256,
                    channels,
                },
            ) if plugin_sha256 == &self.expected_plugin_sha256 && *channels == self.channels => {
                self.state = WorkerSessionState::AwaitingReady;
                Ok(None)
            }
            (WorkerSessionState::AwaitingHello, WorkerMessage::Hello { .. }) => {
                Err(WorkerSessionError::IdentityMismatch)
            }
            (WorkerSessionState::AwaitingReady, WorkerMessage::Ready) => {
                self.state = WorkerSessionState::Active;
                Ok(None)
            }
            (WorkerSessionState::Active, WorkerMessage::Process { frame, .. }) => {
                if frame.channels != self.channels {
                    return Err(WorkerSessionError::Frame(WorkerFrameError::InvalidChannels));
                }
                self.frame_guard
                    .accept(frame, now_tick)
                    .map_err(WorkerSessionError::Frame)?;
                Ok(Some(frame.clone()))
            }
            (WorkerSessionState::Active, WorkerMessage::Latency(_)) => Ok(None),
            (
                WorkerSessionState::Active,
                WorkerMessage::Shutdown | WorkerMessage::Failure { .. },
            ) => {
                self.state = WorkerSessionState::Closed;
                Ok(None)
            }
            (WorkerSessionState::Closed, _) => Err(WorkerSessionError::UnexpectedMessage),
            _ => Err(WorkerSessionError::UnexpectedMessage),
        }
    }
}

pub fn encode_worker_message(message: &WorkerMessage) -> Result<Vec<u8>, WorkerMessageError> {
    let payload =
        serde_json::to_vec(message).map_err(|error| WorkerMessageError::Json(error.to_string()))?;
    if payload.len() > MAX_WORKER_MESSAGE_BYTES {
        return Err(WorkerMessageError::TooLarge {
            length: payload.len(),
            maximum: MAX_WORKER_MESSAGE_BYTES,
        });
    }
    let length = u32::try_from(payload.len()).map_err(|_| WorkerMessageError::TooLarge {
        length: payload.len(),
        maximum: MAX_WORKER_MESSAGE_BYTES,
    })?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_worker_message(frame: &[u8]) -> Result<WorkerMessage, WorkerMessageError> {
    if frame.len() < 4 {
        return Err(WorkerMessageError::TooShort);
    }
    let declared = u32::from_le_bytes(frame[..4].try_into().unwrap()) as usize;
    if declared > MAX_WORKER_MESSAGE_BYTES {
        return Err(WorkerMessageError::TooLarge {
            length: declared,
            maximum: MAX_WORKER_MESSAGE_BYTES,
        });
    }
    let actual = frame.len() - 4;
    if declared != actual {
        return Err(WorkerMessageError::LengthMismatch { declared, actual });
    }
    let message: WorkerMessage = serde_json::from_slice(&frame[4..])
        .map_err(|error| WorkerMessageError::Json(error.to_string()))?;
    validate_worker_message(&message)?;
    Ok(message)
}

/// Reads one worker frame from a stream, handling partial pipe reads and
/// rejecting its declared size before allocating the payload buffer.
pub fn read_worker_message<R: Read>(reader: &mut R) -> Result<WorkerMessage, WorkerMessageError> {
    let mut header = [0u8; 4];
    reader
        .read_exact(&mut header)
        .map_err(|error| WorkerMessageError::Io(error.to_string()))?;
    let declared = u32::from_le_bytes(header) as usize;
    if declared > MAX_WORKER_MESSAGE_BYTES {
        return Err(WorkerMessageError::TooLarge {
            length: declared,
            maximum: MAX_WORKER_MESSAGE_BYTES,
        });
    }
    let mut frame = Vec::with_capacity(4 + declared);
    frame.extend_from_slice(&header);
    frame.resize(4 + declared, 0);
    reader
        .read_exact(&mut frame[4..])
        .map_err(|error| WorkerMessageError::Io(error.to_string()))?;
    decode_worker_message(&frame)
}

/// Writes one complete worker frame to a stream. This is a control-plane
/// operation; realtime audio never waits on this helper.
pub fn write_worker_message<W: Write>(
    writer: &mut W,
    message: &WorkerMessage,
) -> Result<(), WorkerMessageError> {
    let frame = encode_worker_message(message)?;
    writer
        .write_all(&frame)
        .and_then(|_| writer.flush())
        .map_err(|error| WorkerMessageError::Io(error.to_string()))
}

fn validate_worker_message(message: &WorkerMessage) -> Result<(), WorkerMessageError> {
    match message {
        WorkerMessage::Hello {
            protocol_version,
            plugin_sha256,
            channels,
        } => {
            if *protocol_version != WORKER_PROTOCOL_VERSION {
                return Err(WorkerMessageError::InvalidProtocolVersion);
            }
            if !is_sha256(plugin_sha256) {
                return Err(WorkerMessageError::InvalidPluginHash);
            }
            if !matches!(channels, 1 | 2) {
                return Err(WorkerMessageError::InvalidFrame(
                    WorkerFrameError::InvalidChannels,
                ));
            }
        }
        WorkerMessage::Process { frame, parameters } => {
            WorkerFrame::new(
                frame.sequence,
                frame.deadline_tick,
                frame.channels,
                frame.samples.clone(),
            )
            .map_err(WorkerMessageError::InvalidFrame)?;
            if parameters.len() > MAX_PARAMETER_EVENTS {
                return Err(WorkerMessageError::InvalidParameter(
                    ParameterEventError::OffsetOutOfRange,
                ));
            }
            for parameter in parameters {
                ParameterEvent::new(
                    parameter.parameter_id,
                    parameter.normalized_value,
                    parameter.sample_offset,
                )
                .map_err(WorkerMessageError::InvalidParameter)?;
            }
        }
        WorkerMessage::Processed { frame } => {
            WorkerFrame::new(
                frame.sequence,
                frame.deadline_tick,
                frame.channels,
                frame.samples.clone(),
            )
            .map_err(WorkerMessageError::InvalidFrame)?;
        }
        WorkerMessage::Latency(latency) => {
            WorkerLatency::new(latency.samples, latency.sample_rate_hz)?;
        }
        WorkerMessage::Failure { code } if code.is_empty() => {
            return Err(WorkerMessageError::InvalidFailureCode);
        }
        _ => {}
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterEventError {
    NonFiniteValue,
    ValueOutOfRange,
    OffsetOutOfRange,
}

impl ParameterEvent {
    pub fn new(
        parameter_id: u32,
        normalized_value: f32,
        sample_offset: usize,
    ) -> Result<Self, ParameterEventError> {
        if !normalized_value.is_finite() {
            return Err(ParameterEventError::NonFiniteValue);
        }
        if !(0.0..=1.0).contains(&normalized_value) {
            return Err(ParameterEventError::ValueOutOfRange);
        }
        if sample_offset >= MAX_WORKER_FRAMES {
            return Err(ParameterEventError::OffsetOutOfRange);
        }
        Ok(Self {
            parameter_id,
            normalized_value,
            sample_offset,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedParameterQueue {
    events: VecDeque<ParameterEvent>,
    capacity: usize,
    overflow_count: u64,
}

impl BoundedParameterQueue {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "parameter queue capacity must be positive");
        Self {
            events: VecDeque::with_capacity(capacity),
            capacity,
            overflow_count: 0,
        }
    }

    pub fn push(&mut self, event: ParameterEvent) -> Result<(), ParameterEvent> {
        if self.events.len() >= self.capacity {
            self.overflow_count = self.overflow_count.saturating_add(1);
            return Err(event);
        }
        self.events.push_back(event);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<ParameterEvent> {
        self.events.pop_front()
    }
    pub fn overflow_count(&self) -> u64 {
        self.overflow_count
    }
}

impl BoundedFrameQueue {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "worker frame queue capacity must be positive");
        Self {
            frames: VecDeque::with_capacity(capacity),
            capacity,
            overflow_count: 0,
        }
    }

    pub fn push(&mut self, frame: WorkerFrame) -> Result<(), WorkerFrame> {
        if self.frames.len() >= self.capacity {
            self.overflow_count = self.overflow_count.saturating_add(1);
            return Err(frame);
        }
        self.frames.push_back(frame);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<WorkerFrame> {
        self.frames.pop_front()
    }
    pub fn len(&self) -> usize {
        self.frames.len()
    }
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
    pub fn overflow_count(&self) -> u64 {
        self.overflow_count
    }
}

impl WorkerFrame {
    pub fn new(
        sequence: u64,
        deadline_tick: u64,
        channels: u16,
        samples: Vec<f32>,
    ) -> Result<Self, WorkerFrameError> {
        if !matches!(channels, 1 | 2) {
            return Err(WorkerFrameError::InvalidChannels);
        }
        if samples.is_empty() || samples.len() > MAX_WORKER_FRAMES * channels as usize {
            return Err(WorkerFrameError::InvalidFrameCount);
        }
        if samples.len() % channels as usize != 0 {
            return Err(WorkerFrameError::WrongSampleCount);
        }
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err(WorkerFrameError::NonFiniteSample);
        }
        Ok(Self {
            sequence,
            deadline_tick,
            channels,
            samples,
        })
    }

    pub fn frame_count(&self) -> usize {
        self.samples.len() / self.channels as usize
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerFrameGuard {
    last_sequence: Option<u64>,
}

impl WorkerFrameGuard {
    pub fn new() -> Self {
        Self {
            last_sequence: None,
        }
    }

    pub fn accept(&mut self, frame: &WorkerFrame, now_tick: u64) -> Result<(), WorkerFrameError> {
        if self
            .last_sequence
            .is_some_and(|last| frame.sequence <= last)
        {
            return Err(WorkerFrameError::SequenceRegression);
        }
        if frame.deadline_tick < now_tick {
            return Err(WorkerFrameError::DeadlineExpired);
        }
        self.last_sequence = Some(frame.sequence);
        Ok(())
    }
}

impl Default for WorkerFrameGuard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct ChunkedReader {
        bytes: Vec<u8>,
        offset: usize,
        chunk_size: usize,
    }

    impl Read for ChunkedReader {
        fn read(&mut self, destination: &mut [u8]) -> std::io::Result<usize> {
            if self.offset == self.bytes.len() {
                return Ok(0);
            }
            let count = self
                .chunk_size
                .min(destination.len())
                .min(self.bytes.len() - self.offset);
            destination[..count].copy_from_slice(&self.bytes[self.offset..self.offset + count]);
            self.offset += count;
            Ok(count)
        }
    }

    fn temp_root() -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("audiorouter-plugin-{id}"));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn pe_x64() -> Vec<u8> {
        let mut bytes = vec![0; 0x100];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&(0x80u32).to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes
    }

    #[test]
    fn inspects_x64_vst3_and_fingerprints_it() {
        let root = temp_root();
        let path = root.join("effect.vst3");
        fs::write(&path, pe_x64()).unwrap();
        let identity = inspect_binary(&path, std::slice::from_ref(&root)).unwrap();
        assert_eq!(identity.format, PluginFormat::Vst3);
        assert_eq!(identity.architecture, PeArchitecture::X64);
        assert_eq!(identity.sha256.len(), 64);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_escape_legacy_and_non_x64_binaries() {
        let root = temp_root();
        let outside = temp_root().join("legacy.dll");
        fs::write(&outside, pe_x64()).unwrap();
        assert_eq!(
            inspect_binary(&outside, std::slice::from_ref(&root)),
            Err(InspectionError::OutsideConfiguredRoot)
        );
        let vst2 = root.join("legacy.dll");
        fs::write(&vst2, pe_x64()).unwrap();
        assert_eq!(
            inspect_binary(&vst2, std::slice::from_ref(&root))
                .unwrap()
                .format,
            PluginFormat::Unknown
        );
        let invalid = root.join("bad.vst3");
        fs::write(invalid, b"not a pe").unwrap();
        assert_eq!(
            inspect_binary(&root.join("bad.vst3"), std::slice::from_ref(&root)),
            Err(InspectionError::NotPe)
        );
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside.parent().unwrap()).unwrap();
    }

    #[test]
    fn quarantines_after_three_failures_until_deliberate_retry() {
        let mut ledger = FailureLedger::new();
        ledger.record_failure();
        ledger.record_failure();
        assert!(!ledger.quarantined());
        ledger.record_failure();
        assert!(ledger.quarantined());
        ledger.deliberate_retry();
        assert_eq!(ledger, FailureLedger::new());
    }

    #[test]
    fn validates_bounded_worker_frames_and_deadlines() {
        let mut guard = WorkerFrameGuard::new();
        let frame = WorkerFrame::new(1, 10, 2, vec![0.25, -0.25, 0.0, 0.1]).unwrap();
        assert_eq!(frame.frame_count(), 2);
        assert!(guard.accept(&frame, 9).is_ok());
        assert_eq!(
            guard.accept(&frame, 9),
            Err(WorkerFrameError::SequenceRegression)
        );
        let late = WorkerFrame::new(2, 10, 1, vec![0.0]).unwrap();
        assert_eq!(
            guard.accept(&late, 11),
            Err(WorkerFrameError::DeadlineExpired)
        );
        assert_eq!(
            WorkerFrame::new(3, 20, 2, vec![f32::NAN, 0.0]),
            Err(WorkerFrameError::NonFiniteSample)
        );
    }

    #[test]
    fn scans_explicit_root_and_keeps_invalid_entries_visible() {
        let root = temp_root();
        fs::write(root.join("good.vst3"), pe_x64()).unwrap();
        fs::write(root.join("bad.vst3"), b"not a pe").unwrap();
        fs::write(root.join("notes.txt"), b"ignored").unwrap();
        let entries = scan_directory(&root).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|entry| entry.identity.is_some()));
        assert!(entries
            .iter()
            .any(|entry| entry.error == Some(InspectionError::NotPe)));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inspects_a_vst3_bundle_binary_under_x64_contents() {
        let root = temp_root();
        let bundle = root.join("effect.vst3");
        let binary_dir = bundle.join("Contents").join("x86_64-win");
        fs::create_dir_all(&binary_dir).unwrap();
        let binary = binary_dir.join("effect.vst3");
        fs::write(&binary, pe_x64()).unwrap();
        let identity = inspect_binary(&bundle, std::slice::from_ref(&root)).unwrap();
        assert_eq!(identity.path, fs::canonicalize(bundle).unwrap());
        assert_eq!(identity.binary_path, fs::canonicalize(binary).unwrap());
        assert_eq!(
            identity.compatibility(),
            PluginCompatibility::SupportedVst3X64
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn controlled_scan_honors_cancel_and_deadline() {
        let root = temp_root();
        let cancelled = ScanControl::default_deadline();
        cancelled.cancel();
        assert_eq!(
            scan_directory_with_control(&root, &cancelled),
            Err(ScanError::Cancelled)
        );
        let expired = ScanControl::with_deadline(Instant::now());
        assert_eq!(
            scan_directory_with_control(&root, &expired),
            Err(ScanError::DeadlineExceeded)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_more_than_the_candidate_budget() {
        let root = temp_root();
        for index in 0..=MAX_SCAN_CANDIDATES {
            fs::write(root.join(format!("effect-{index}.vst3")), b"not inspected").unwrap();
        }
        assert_eq!(scan_directory(&root), Err(ScanError::TooManyCandidates));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn protected_worker_failures_never_choose_dry_fallback() {
        assert_eq!(
            WorkerFailurePolicy::new(true).on_failure(),
            WorkerFailureAction::Silence
        );
        assert_eq!(
            WorkerFailurePolicy::new(false).on_failure(),
            WorkerFailureAction::DryFallback
        );
    }

    #[test]
    fn plugin_state_is_versioned_and_integrity_checked() {
        let asset = PluginStateAsset::new(2, vec![1, 2, 3]).unwrap();
        assert_eq!(asset.verify_for_restore(2), Ok(&[1, 2, 3][..]));
        assert_eq!(
            asset.verify_for_restore(1),
            Err(StateError::VersionMismatch)
        );
        let mut corrupt = asset.clone();
        corrupt.bytes[0] = 9;
        assert_eq!(
            corrupt.verify_for_restore(2),
            Err(StateError::IntegrityMismatch)
        );
        assert_eq!(PluginStateAsset::new(1, Vec::new()), Err(StateError::Empty));
    }

    #[test]
    fn quarantine_failures_expire_after_the_ten_minute_window() {
        let start = Instant::now();
        let mut ledger = FailureLedger::new();
        ledger.record_failure_at(start);
        ledger.record_failure_at(start + Duration::from_secs(1));
        ledger.record_failure_at(start + Duration::from_secs(FAILURE_WINDOW.as_secs() + 1));
        assert_eq!(ledger.failures(), 1);
        assert!(!ledger.quarantined());
    }

    #[test]
    fn worker_supervisor_requires_vst3_x64_and_expires_heartbeats() {
        let identity = PluginIdentity {
            path: PathBuf::from("effect.vst3"),
            binary_path: PathBuf::from("effect.vst3"),
            format: PluginFormat::Vst3,
            architecture: PeArchitecture::X64,
            file_bytes: 1,
            sha256: "0".repeat(64),
        };
        let now = Instant::now();
        let mut supervisor = WorkerSupervisor::new();
        assert_eq!(supervisor.start(&identity, now), Ok(()));
        assert_eq!(
            supervisor.poll(now + WORKER_HEARTBEAT_TIMEOUT + Duration::from_millis(1)),
            WorkerState::Failed
        );
        assert!(!supervisor.heartbeat(now));
        supervisor.deliberate_retry();
        assert_eq!(supervisor.state(), WorkerState::Stopped);
    }

    #[test]
    fn bounded_frame_queue_returns_overflow_ownership_without_waiting() {
        let first = WorkerFrame::new(1, 10, 1, vec![0.0]).unwrap();
        let second = WorkerFrame::new(2, 10, 1, vec![0.1]).unwrap();
        let mut queue = BoundedFrameQueue::new(1);
        assert!(queue.push(first.clone()).is_ok());
        assert_eq!(queue.push(second.clone()), Err(second));
        assert_eq!(queue.overflow_count(), 1);
        assert_eq!(queue.pop(), Some(first));
        assert!(queue.is_empty());
    }

    #[test]
    fn state_assets_use_exclusive_safe_paths_and_verify_on_read() {
        let root = temp_root();
        let asset = PluginStateAsset::new(4, vec![7, 8, 9]).unwrap();
        let path = write_state_asset(&root, "plugin_state-1", &asset).unwrap();
        assert_eq!(
            read_state_asset(&root, &path, 4, &asset.sha256).unwrap(),
            asset
        );
        assert_eq!(
            read_state_asset(&root, &path, 4, &"00".repeat(32)),
            Err(StateFileError::InvalidState(StateError::IntegrityMismatch))
        );
        assert_eq!(
            write_state_asset(&root, "plugin_state-1", &asset),
            Err(StateFileError::Exists)
        );
        assert_eq!(
            write_state_asset(&root, "..\\escape", &asset),
            Err(StateFileError::InvalidAssetId)
        );

        let mut corrupted = asset.clone();
        corrupted.bytes[0] = 0;
        assert_eq!(
            write_state_asset(&root, "corrupted", &corrupted),
            Err(StateFileError::InvalidState(StateError::IntegrityMismatch))
        );

        fs::write(&path, [9_u8, 8, 7]).unwrap();
        assert_eq!(
            read_state_asset(&root, &path, 4, &asset.sha256),
            Err(StateFileError::InvalidState(StateError::IntegrityMismatch))
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compatibility_requires_vst3_x64_identity() {
        let mut identity = PluginIdentity {
            path: PathBuf::from("effect.vst3"),
            binary_path: PathBuf::from("effect.vst3"),
            format: PluginFormat::Vst3,
            architecture: PeArchitecture::X64,
            file_bytes: 1,
            sha256: "0".repeat(64),
        };
        assert_eq!(
            identity.compatibility(),
            PluginCompatibility::SupportedVst3X64
        );
        identity.format = PluginFormat::Unknown;
        assert_eq!(
            identity.compatibility(),
            PluginCompatibility::UnsupportedFormat
        );
    }

    #[test]
    fn parameter_events_are_bounded_finite_and_nonblocking() {
        let event = ParameterEvent::new(4, 0.5, 127).unwrap();
        assert_eq!(
            ParameterEvent::new(4, f32::NAN, 0),
            Err(ParameterEventError::NonFiniteValue)
        );
        assert_eq!(
            ParameterEvent::new(4, 1.1, 0),
            Err(ParameterEventError::ValueOutOfRange)
        );
        assert_eq!(
            ParameterEvent::new(4, 0.5, MAX_WORKER_FRAMES),
            Err(ParameterEventError::OffsetOutOfRange)
        );
        let mut queue = BoundedParameterQueue::new(1);
        assert!(queue.push(event).is_ok());
        assert_eq!(queue.push(event), Err(event));
        assert_eq!(queue.overflow_count(), 1);
        assert_eq!(queue.pop(), Some(event));
    }

    #[test]
    fn worker_messages_round_trip_and_revalidate_audio_payloads() {
        let message = WorkerMessage::Process {
            frame: WorkerFrame::new(7, 100, 2, vec![0.25, -0.25, 0.0, 0.1]).unwrap(),
            parameters: vec![ParameterEvent::new(3, 0.75, 2).unwrap()],
        };
        let encoded = encode_worker_message(&message).unwrap();
        assert_eq!(decode_worker_message(&encoded).unwrap(), message);

        let mut invalid = Vec::new();
        let payload = serde_json::to_vec(&WorkerMessage::Process {
            frame: WorkerFrame {
                sequence: 7,
                deadline_tick: 100,
                channels: 3,
                samples: vec![0.0, 0.0, 0.0],
            },
            parameters: Vec::new(),
        })
        .unwrap();
        invalid.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        invalid.extend_from_slice(&payload);
        assert!(matches!(
            decode_worker_message(&invalid),
            Err(WorkerMessageError::InvalidFrame(
                WorkerFrameError::InvalidChannels
            ))
        ));
    }

    #[test]
    fn worker_messages_reject_truncated_and_oversized_frames() {
        assert_eq!(
            decode_worker_message(&[]),
            Err(WorkerMessageError::TooShort)
        );
        let mut oversized = Vec::new();
        oversized.extend_from_slice(&((MAX_WORKER_MESSAGE_BYTES as u32) + 1).to_le_bytes());
        assert!(matches!(
            decode_worker_message(&oversized),
            Err(WorkerMessageError::TooLarge { .. })
        ));
    }

    #[test]
    fn worker_stream_helpers_handle_fragmented_reads_and_flush_writes() {
        let message = WorkerMessage::Ready;
        let mut encoded = Cursor::new(Vec::new());
        write_worker_message(&mut encoded, &message).unwrap();
        let mut reader = ChunkedReader {
            bytes: encoded.into_inner(),
            offset: 0,
            chunk_size: 1,
        };
        assert_eq!(read_worker_message(&mut reader).unwrap(), message);
    }

    #[test]
    fn worker_hello_validates_negotiated_capabilities() {
        let valid = WorkerMessage::Hello {
            protocol_version: WORKER_PROTOCOL_VERSION,
            plugin_sha256: "a".repeat(64),
            channels: 2,
        };
        assert_eq!(
            decode_worker_message(&encode_worker_message(&valid).unwrap()),
            Ok(valid)
        );

        for message in [
            WorkerMessage::Hello {
                protocol_version: WORKER_PROTOCOL_VERSION + 1,
                plugin_sha256: "a".repeat(64),
                channels: 2,
            },
            WorkerMessage::Hello {
                protocol_version: WORKER_PROTOCOL_VERSION,
                plugin_sha256: "not-a-hash".into(),
                channels: 2,
            },
            WorkerMessage::Hello {
                protocol_version: WORKER_PROTOCOL_VERSION,
                plugin_sha256: "a".repeat(64),
                channels: 4,
            },
        ] {
            let encoded = encode_worker_message(&message).unwrap();
            assert!(decode_worker_message(&encoded).is_err());
        }
        let failure = WorkerMessage::Failure {
            code: String::new(),
        };
        assert!(decode_worker_message(&encode_worker_message(&failure).unwrap()).is_err());
    }

    #[test]
    fn worker_session_requires_handshake_identity_and_monotonic_frames() {
        let hash = "b".repeat(64);
        let frame = WorkerFrame::new(1, 20, 2, vec![0.0, 0.1]).unwrap();
        let mut session = WorkerSession::new(&hash, 2).unwrap();
        assert_eq!(session.state(), WorkerSessionState::AwaitingHello);
        assert_eq!(
            session.accept(
                &WorkerMessage::Hello {
                    protocol_version: WORKER_PROTOCOL_VERSION,
                    plugin_sha256: "c".repeat(64),
                    channels: 2,
                },
                0,
            ),
            Err(WorkerSessionError::IdentityMismatch)
        );
        assert_eq!(
            session.accept(
                &WorkerMessage::Hello {
                    protocol_version: WORKER_PROTOCOL_VERSION,
                    plugin_sha256: hash,
                    channels: 2,
                },
                0,
            ),
            Ok(None)
        );
        assert_eq!(session.accept(&WorkerMessage::Ready, 0), Ok(None));
        assert_eq!(session.state(), WorkerSessionState::Active);
        assert_eq!(
            session.accept(
                &WorkerMessage::Process {
                    frame: frame.clone(),
                    parameters: Vec::new(),
                },
                10,
            ),
            Ok(Some(frame.clone()))
        );
        assert_eq!(
            session.accept(
                &WorkerMessage::Process {
                    frame,
                    parameters: Vec::new(),
                },
                10,
            ),
            Err(WorkerSessionError::Frame(
                WorkerFrameError::SequenceRegression
            ))
        );
        assert_eq!(session.accept(&WorkerMessage::Shutdown, 10), Ok(None));
        assert_eq!(session.state(), WorkerSessionState::Closed);
    }

    #[test]
    fn worker_latency_reports_bounded_sample_rate_and_dynamic_updates() {
        let first = WorkerLatency::new(240, 48_000).unwrap();
        assert!((first.milliseconds() - 5.0).abs() < f32::EPSILON);
        let encoded = encode_worker_message(&WorkerMessage::Latency(first)).unwrap();
        assert_eq!(
            decode_worker_message(&encoded).unwrap(),
            WorkerMessage::Latency(first)
        );

        assert_eq!(
            WorkerLatency::new(0, 7_999),
            Err(WorkerMessageError::InvalidLatency)
        );
        assert_eq!(
            WorkerLatency::new(480_001, 48_000),
            Err(WorkerMessageError::InvalidLatency)
        );
    }
}
