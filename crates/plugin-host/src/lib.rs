//! Portable plugin inspection and failure policy.
//!
//! This crate intentionally does not load or execute plugin code. Discovery
//! produces identity evidence for a later disposable worker boundary.

use memmap2::{MmapMut, MmapOptions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::fs;
use std::io::{BufReader, BufWriter, Read, Write};
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver},
    Arc,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
pub const WORKER_RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

/// Milliseconds since the Unix epoch used for cross-process frame deadlines.
pub fn worker_clock_tick() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
}

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
    let root_metadata = fs::symlink_metadata(root).map_err(|_| StateFileError::InvalidRoot)?;
    if !root_metadata.is_dir() || is_reparse_point(&root_metadata) {
        return Err(StateFileError::InvalidRoot);
    }
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
    let root_metadata = fs::symlink_metadata(root).map_err(|_| StateFileError::InvalidRoot)?;
    if !root_metadata.is_dir() || is_reparse_point(&root_metadata) {
        return Err(StateFileError::InvalidRoot);
    }
    let canonical_root = fs::canonicalize(root).map_err(|_| StateFileError::InvalidRoot)?;
    let path_metadata =
        fs::symlink_metadata(path).map_err(|error| StateFileError::Io(error.to_string()))?;
    if path_has_reparse_component(path, &canonical_root)
        .map_err(|error| StateFileError::Io(error.to_string()))?
        || is_reparse_point(&path_metadata)
    {
        return Err(StateFileError::OutsideRoot);
    }
    let canonical_path =
        fs::canonicalize(path).map_err(|error| StateFileError::Io(error.to_string()))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(StateFileError::OutsideRoot);
    }
    if canonical_path.parent() != Some(canonical_root.as_path()) {
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

fn path_has_reparse_component(path: &Path, root: &Path) -> std::io::Result<bool> {
    let mut current = path.to_path_buf();
    loop {
        if current == root {
            return Ok(false);
        }
        if current.exists() && is_reparse_point(&fs::symlink_metadata(&current)?) {
            return Ok(true);
        }
        if !current.pop() {
            return Ok(false);
        }
    }
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
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
    AlreadyRunning,
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
        if self.state == WorkerState::Running {
            return Err(WorkerStartError::AlreadyRunning);
        }
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

    /// Record an immediate worker failure reported by the process adapter.
    /// This shares the heartbeat timeout's bounded quarantine policy and
    /// never starts or restarts a process by itself.
    pub fn record_failure(&mut self, now: Instant) -> WorkerState {
        if self.state == WorkerState::Running {
            self.failures.record_failure_at(now);
            self.state = if self.failures.quarantined() {
                WorkerState::Quarantined
            } else {
                WorkerState::Failed
            };
            self.last_heartbeat = None;
        }
        self.state
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
    ProcessShared {
        sequence: u64,
        deadline_tick: u64,
        channels: u16,
        frames: u32,
        parameters: Vec<ParameterEvent>,
    },
    Processed {
        frame: WorkerFrame,
    },
    ProcessedShared {
        sequence: u64,
        deadline_tick: u64,
        channels: u16,
        frames: u32,
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

    pub fn hello_sent(&mut self) -> Result<(), WorkerSessionError> {
        if self.state != WorkerSessionState::AwaitingHello {
            return Err(WorkerSessionError::UnexpectedMessage);
        }
        self.state = WorkerSessionState::AwaitingReady;
        Ok(())
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
            (
                WorkerSessionState::Active,
                WorkerMessage::ProcessShared {
                    sequence,
                    deadline_tick,
                    channels,
                    frames: _,
                    parameters: _,
                },
            ) => {
                if *channels != self.channels {
                    return Err(WorkerSessionError::Frame(WorkerFrameError::InvalidChannels));
                }
                self.frame_guard
                    .accept_metadata(*sequence, *deadline_tick, now_tick)
                    .map_err(WorkerSessionError::Frame)?;
                Ok(None)
            }
            (
                WorkerSessionState::Active,
                WorkerMessage::Latency(_) | WorkerMessage::ProcessedShared { .. },
            ) => Ok(None),
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

#[derive(Debug)]
pub enum WorkerProcessError {
    Spawn(String),
    Message(WorkerMessageError),
    Protocol(String),
    Exited,
    Timeout,
}

#[cfg(windows)]
struct WorkerSandbox {
    handle: *mut std::ffi::c_void,
}

#[cfg(not(windows))]
struct WorkerSandbox;

#[cfg(windows)]
impl WorkerSandbox {
    fn attach(child: &Child) -> Result<Self, String> {
        let handle = unsafe { create_job_object() };
        if handle.is_null() {
            return Err("CreateJobObjectW failed".into());
        }
        let mut limits = JobObjectExtendedLimitInformation::default();
        limits.basic.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
        limits.basic.active_process_limit = WORKER_MAX_ACTIVE_PROCESSES;
        limits.process_memory_limit = WORKER_MAX_PROCESS_MEMORY_BYTES;
        let configured = unsafe {
            set_information_job_object(
                handle,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                &limits as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JobObjectExtendedLimitInformation>() as u32,
            )
        };
        let assigned =
            configured && unsafe { assign_process_to_job_object(handle, child.as_raw_handle()) };
        if !assigned {
            unsafe { close_handle(handle) };
            return Err("could not assign worker to a kill-on-close job".into());
        }
        Ok(Self { handle })
    }
}

#[cfg(not(windows))]
impl WorkerSandbox {
    fn attach(_: &Child) -> Result<Self, String> {
        Ok(Self)
    }
}

#[cfg(windows)]
impl Drop for WorkerSandbox {
    fn drop(&mut self) {
        unsafe { close_handle(self.handle) };
    }
}

#[cfg(windows)]
const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: u32 = 9;
#[cfg(windows)]
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;
#[cfg(windows)]
const JOB_OBJECT_LIMIT_ACTIVE_PROCESS: u32 = 0x0000_0008;
#[cfg(windows)]
const JOB_OBJECT_LIMIT_PROCESS_MEMORY: u32 = 0x0000_0100;
#[cfg(windows)]
const WORKER_MAX_ACTIVE_PROCESSES: u32 = 1;
#[cfg(windows)]
const WORKER_MAX_PROCESS_MEMORY_BYTES: usize = 512 * 1024 * 1024;

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct JobObjectBasicLimitInformation {
    per_process_user_time_limit: i64,
    per_job_user_time_limit: i64,
    limit_flags: u32,
    minimum_working_set_size: usize,
    maximum_working_set_size: usize,
    active_process_limit: u32,
    affinity: usize,
    priority_class: u32,
    scheduling_class: u32,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct IoCounters {
    read_operation_count: u64,
    write_operation_count: u64,
    other_operation_count: u64,
    read_transfer_count: u64,
    write_transfer_count: u64,
    other_transfer_count: u64,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Default)]
struct JobObjectExtendedLimitInformation {
    basic: JobObjectBasicLimitInformation,
    io: IoCounters,
    process_memory_limit: usize,
    job_memory_limit: usize,
    peak_process_memory_used: usize,
    peak_job_memory_used: usize,
}

#[cfg(windows)]
unsafe extern "system" {
    fn CreateJobObjectW(
        attributes: *const std::ffi::c_void,
        name: *const u16,
    ) -> *mut std::ffi::c_void;
    fn SetInformationJobObject(
        job: *mut std::ffi::c_void,
        class: u32,
        information: *const std::ffi::c_void,
        length: u32,
    ) -> i32;
    fn AssignProcessToJobObject(job: *mut std::ffi::c_void, process: *mut std::ffi::c_void) -> i32;
    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
}

#[cfg(windows)]
unsafe fn create_job_object() -> *mut std::ffi::c_void {
    CreateJobObjectW(std::ptr::null(), std::ptr::null())
}

#[cfg(windows)]
unsafe fn set_information_job_object(
    job: *mut std::ffi::c_void,
    class: u32,
    information: *const std::ffi::c_void,
    length: u32,
) -> bool {
    SetInformationJobObject(job, class, information, length) != 0
}

#[cfg(windows)]
unsafe fn assign_process_to_job_object(
    job: *mut std::ffi::c_void,
    process: *mut std::ffi::c_void,
) -> bool {
    AssignProcessToJobObject(job, process) != 0
}

#[cfg(windows)]
unsafe fn close_handle(handle: *mut std::ffi::c_void) {
    let _ = CloseHandle(handle);
}

/// Control-plane client for one disposable worker process. This owns the
/// process and pipes; realtime callers must exchange frames through a
/// preallocated transport rather than calling these blocking methods.
pub struct WorkerProcess {
    child: Child,
    _sandbox: WorkerSandbox,
    writer: BufWriter<ChildStdin>,
    reader: Receiver<Result<WorkerMessage, WorkerMessageError>>,
    channels: u16,
    shared: Option<SharedAudioTransport>,
}

impl WorkerProcess {
    pub fn spawn(
        executable: impl AsRef<Path>,
        plugin_sha256: &str,
        channels: u16,
    ) -> Result<Self, WorkerProcessError> {
        Self::spawn_inner(executable, plugin_sha256, channels, None)
    }

    pub fn spawn_shared(
        executable: impl AsRef<Path>,
        plugin_sha256: &str,
        channels: u16,
        transport: SharedAudioTransport,
    ) -> Result<Self, WorkerProcessError> {
        Self::spawn_inner(executable, plugin_sha256, channels, Some(transport))
    }

    fn spawn_inner(
        executable: impl AsRef<Path>,
        plugin_sha256: &str,
        channels: u16,
        mut shared: Option<SharedAudioTransport>,
    ) -> Result<Self, WorkerProcessError> {
        let executable =
            validate_worker_executable(executable.as_ref()).map_err(WorkerProcessError::Spawn)?;
        if !is_sha256(plugin_sha256) {
            return Err(WorkerProcessError::Protocol("invalid plugin hash".into()));
        }
        if !matches!(channels, 1 | 2) {
            return Err(WorkerProcessError::Protocol("invalid channel count".into()));
        }
        let mut command = Command::new(&executable);
        // Plugin workers must not inherit backend credentials or unrelated
        // environment configuration. The executable is absolute and the
        // worker protocol passes all required configuration explicitly.
        command.env_clear();
        command.args([
            "--plugin-sha256",
            plugin_sha256,
            "--channels",
            &channels.to_string(),
        ]);
        if let Some(transport) = shared.as_ref() {
            command.args([
                "--input-path",
                &transport.input_path().to_string_lossy(),
                "--output-path",
                &transport.output_path().to_string_lossy(),
            ]);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| WorkerProcessError::Spawn(error.to_string()))?;
        let sandbox = match WorkerSandbox::attach(&child) {
            Ok(sandbox) => sandbox,
            Err(error) => {
                terminate_child(&mut child);
                return Err(WorkerProcessError::Spawn(error));
            }
        };
        let stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                terminate_child(&mut child);
                return Err(WorkerProcessError::Exited);
            }
        };
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                terminate_child(&mut child);
                return Err(WorkerProcessError::Exited);
            }
        };
        let (reader_sender, reader) = mpsc::sync_channel(4);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let result = read_worker_message(&mut reader);
                let finished = result.is_err();
                if reader_sender.send(result).is_err() || finished {
                    break;
                }
            }
        });
        let mut process = Self {
            child,
            _sandbox: sandbox,
            writer: BufWriter::new(stdin),
            reader,
            channels,
            shared: shared.take(),
        };
        let hello = process.read().map_err(WorkerProcessError::Message)?;
        match hello {
            WorkerMessage::Hello {
                protocol_version,
                plugin_sha256: actual,
                channels: actual_channels,
            } if protocol_version == WORKER_PROTOCOL_VERSION
                && actual == plugin_sha256
                && actual_channels == channels => {}
            _ => {
                return Err(WorkerProcessError::Protocol(
                    "worker identity negotiation failed".into(),
                ))
            }
        }
        process
            .write(&WorkerMessage::Ready)
            .map_err(WorkerProcessError::Message)?;
        Ok(process)
    }

    pub fn process_shared(
        &mut self,
        frame: WorkerFrame,
        parameters: Vec<ParameterEvent>,
    ) -> Result<WorkerFrame, WorkerProcessError> {
        if frame.channels != self.channels {
            return Err(WorkerProcessError::Protocol(
                "frame channel count mismatch".into(),
            ));
        }
        {
            let transport = self.shared.as_mut().ok_or_else(|| {
                WorkerProcessError::Protocol("shared transport was not configured".into())
            })?;
            transport.write_input(&frame).map_err(|error| {
                WorkerProcessError::Protocol(format!("input write failed: {error:?}"))
            })?;
        }
        self.write(&WorkerMessage::ProcessShared {
            sequence: frame.sequence,
            deadline_tick: frame.deadline_tick,
            channels: frame.channels,
            frames: frame.frame_count() as u32,
            parameters,
        })
        .map_err(WorkerProcessError::Message)?;
        let response = self.read().map_err(WorkerProcessError::Message)?;
        match response {
            WorkerMessage::ProcessedShared {
                sequence,
                deadline_tick,
                channels,
                frames,
            } if sequence == frame.sequence
                && deadline_tick == frame.deadline_tick
                && channels == frame.channels
                && frames == frame.frame_count() as u32 =>
            {
                self.shared
                    .as_ref()
                    .expect("shared transport was checked before sending")
                    .read_output()
                    .map_err(|error| {
                        WorkerProcessError::Protocol(format!("output read failed: {error:?}"))
                    })
            }
            WorkerMessage::Failure { code } => Err(WorkerProcessError::Protocol(code)),
            _ => Err(WorkerProcessError::Protocol(
                "unexpected shared process response".into(),
            )),
        }
    }

    pub fn process(
        &mut self,
        frame: WorkerFrame,
        parameters: Vec<ParameterEvent>,
    ) -> Result<WorkerFrame, WorkerProcessError> {
        if frame.channels != self.channels {
            return Err(WorkerProcessError::Protocol(
                "frame channel count mismatch".into(),
            ));
        }
        self.write(&WorkerMessage::Process {
            frame: frame.clone(),
            parameters,
        })
        .map_err(WorkerProcessError::Message)?;
        match self.read().map_err(WorkerProcessError::Message)? {
            WorkerMessage::Processed { frame: processed }
                if processed_frame_matches(&frame, &processed) =>
            {
                Ok(processed)
            }
            WorkerMessage::Failure { code } => Err(WorkerProcessError::Protocol(code)),
            _ => Err(WorkerProcessError::Protocol(
                "unexpected process response".into(),
            )),
        }
    }

    pub fn report_latency(
        &mut self,
        latency: WorkerLatency,
    ) -> Result<WorkerLatency, WorkerProcessError> {
        self.write(&WorkerMessage::Latency(latency))
            .map_err(WorkerProcessError::Message)?;
        match self.read().map_err(WorkerProcessError::Message)? {
            WorkerMessage::Latency(actual) => Ok(actual),
            _ => Err(WorkerProcessError::Protocol(
                "unexpected latency response".into(),
            )),
        }
    }

    pub fn shutdown(self) -> Result<ExitStatus, WorkerProcessError> {
        self.shutdown_with_timeout(Duration::from_secs(5))
    }

    pub fn shutdown_with_timeout(
        mut self,
        timeout: Duration,
    ) -> Result<ExitStatus, WorkerProcessError> {
        self.write(&WorkerMessage::Shutdown)
            .map_err(WorkerProcessError::Message)?;
        let started = Instant::now();
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) if started.elapsed() < timeout => {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Ok(None) => {
                    self.child
                        .kill()
                        .map_err(|error| WorkerProcessError::Spawn(error.to_string()))?;
                    let _ = self.child.wait();
                    return Err(WorkerProcessError::Timeout);
                }
                Err(error) => return Err(WorkerProcessError::Spawn(error.to_string())),
            }
        }
    }

    fn read(&mut self) -> Result<WorkerMessage, WorkerMessageError> {
        receive_worker_message(&self.reader, WORKER_RESPONSE_TIMEOUT)
    }
    fn write(&mut self, message: &WorkerMessage) -> Result<(), WorkerMessageError> {
        write_worker_message(&mut self.writer, message)
    }
}

fn receive_worker_message(
    reader: &Receiver<Result<WorkerMessage, WorkerMessageError>>,
    timeout: Duration,
) -> Result<WorkerMessage, WorkerMessageError> {
    reader
        .recv_timeout(timeout)
        .map_err(|error| WorkerMessageError::Io(error.to_string()))?
}

fn processed_frame_matches(expected: &WorkerFrame, actual: &WorkerFrame) -> bool {
    actual.sequence == expected.sequence
        && actual.deadline_tick == expected.deadline_tick
        && actual.channels == expected.channels
        && actual.samples.len() == expected.samples.len()
}

fn validate_worker_executable(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("worker executable path must be absolute".into());
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("worker executable metadata failed: {error}"))?;
    if is_reparse_point(&metadata) || !metadata.is_file() {
        return Err("worker executable must be a regular non-reparse file".into());
    }
    fs::canonicalize(path)
        .map_err(|error| format!("worker executable canonicalization failed: {error}"))
}

fn terminate_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Drop for WorkerProcess {
    fn drop(&mut self) {
        terminate_child(&mut self.child);
    }
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
        WorkerMessage::ProcessShared {
            sequence,
            deadline_tick,
            channels,
            frames,
            parameters,
        } => {
            validate_shared_frame_shape(*sequence, *deadline_tick, *channels, *frames)?;
            validate_parameters(parameters)?;
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
        WorkerMessage::ProcessedShared {
            sequence,
            deadline_tick,
            channels,
            frames,
        } => validate_shared_frame_shape(*sequence, *deadline_tick, *channels, *frames)?,
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

fn validate_parameters(parameters: &[ParameterEvent]) -> Result<(), WorkerMessageError> {
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
    Ok(())
}

fn validate_shared_frame_shape(
    sequence: u64,
    deadline_tick: u64,
    channels: u16,
    frames: u32,
) -> Result<(), WorkerMessageError> {
    if !matches!(channels, 1 | 2) {
        return Err(WorkerMessageError::InvalidFrame(
            WorkerFrameError::InvalidChannels,
        ));
    }
    if frames == 0 || frames as usize > MAX_WORKER_FRAMES {
        return Err(WorkerMessageError::InvalidFrame(
            WorkerFrameError::InvalidFrameCount,
        ));
    }
    let _ = (sequence, deadline_tick);
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

pub const SHARED_AUDIO_HEADER_BYTES: usize = 40;
const SHARED_AUDIO_STATE_OFFSET: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SharedAudioError {
    InvalidChannels,
    AliasedPaths,
    BufferTooSmall,
    InvalidMagic,
    InvalidVersion,
    InvalidFrameCount,
    InvalidFrame(WorkerFrameError),
    InvalidPath,
    Exists,
    Empty,
    Busy,
    TornRead,
    SequenceRegression,
    Io(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SharedAudioMetadata {
    pub sequence: u64,
    pub deadline_tick: u64,
    pub channels: u16,
    pub frames: usize,
}

/// File-backed shared memory for one bounded audio slot. The file is an
/// explicit caller-owned IPC endpoint; this type never chooses a machine-wide
/// name and never touches audio or plugin state.
pub struct SharedAudioRegion {
    file: fs::File,
    map: MmapMut,
    layout: SharedAudioLayout,
}

impl SharedAudioRegion {
    pub fn create(
        path: impl AsRef<Path>,
        layout: SharedAudioLayout,
    ) -> Result<Self, SharedAudioError> {
        let path = path.as_ref();
        if !path.is_absolute() {
            return Err(SharedAudioError::InvalidPath);
        }
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    SharedAudioError::Exists
                } else {
                    SharedAudioError::Io(error.to_string())
                }
            })?;
        file.set_len(layout.buffer_len() as u64)
            .map_err(|error| SharedAudioError::Io(error.to_string()))?;
        let map = unsafe { MmapOptions::new().len(layout.buffer_len()).map_mut(&file) }
            .map_err(|error| SharedAudioError::Io(error.to_string()))?;
        Ok(Self { file, map, layout })
    }

    pub fn open(
        path: impl AsRef<Path>,
        layout: SharedAudioLayout,
    ) -> Result<Self, SharedAudioError> {
        let path = path.as_ref();
        if !path.is_absolute() {
            return Err(SharedAudioError::InvalidPath);
        }
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|error| SharedAudioError::Io(error.to_string()))?;
        if file
            .metadata()
            .map_err(|error| SharedAudioError::Io(error.to_string()))?
            .len()
            < layout.buffer_len() as u64
        {
            return Err(SharedAudioError::BufferTooSmall);
        }
        let map = unsafe { MmapOptions::new().len(layout.buffer_len()).map_mut(&file) }
            .map_err(|error| SharedAudioError::Io(error.to_string()))?;
        Ok(Self { file, map, layout })
    }

    pub fn write(&mut self, frame: &WorkerFrame) -> Result<(), SharedAudioError> {
        let state = self.state() as *const std::sync::atomic::AtomicU64;
        let current = unsafe { (*state).load(std::sync::atomic::Ordering::Acquire) };
        if current & 1 != 0 {
            return Err(SharedAudioError::Busy);
        }
        unsafe {
            (*state).compare_exchange(
                current,
                current.saturating_add(1),
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Relaxed,
            )
        }
        .map_err(|_| SharedAudioError::Busy)?;
        if current != 0 {
            let previous = u64::from_le_bytes(self.map[12..20].try_into().unwrap());
            if frame.sequence <= previous {
                unsafe { (*state).store(current, std::sync::atomic::Ordering::Release) };
                return Err(SharedAudioError::SequenceRegression);
            }
        }
        if let Err(error) = self.layout.write(&mut self.map, frame) {
            unsafe { (*state).store(current, std::sync::atomic::Ordering::Release) };
            return Err(error);
        }
        unsafe {
            (*state).store(
                current.saturating_add(2),
                std::sync::atomic::Ordering::Release,
            )
        };
        Ok(())
    }

    pub fn read(&self) -> Result<WorkerFrame, SharedAudioError> {
        let state = self.state();
        let before = state.load(std::sync::atomic::Ordering::Acquire);
        if before == 0 {
            return Err(SharedAudioError::Empty);
        }
        if before & 1 != 0 {
            return Err(SharedAudioError::Busy);
        }
        let frame = self.layout.read(&self.map)?;
        if state.load(std::sync::atomic::Ordering::Acquire) != before {
            return Err(SharedAudioError::TornRead);
        }
        Ok(frame)
    }

    pub fn read_into(&self, samples: &mut [f32]) -> Result<SharedAudioMetadata, SharedAudioError> {
        let state = self.state();
        let before = state.load(std::sync::atomic::Ordering::Acquire);
        if before == 0 {
            return Err(SharedAudioError::Empty);
        }
        if before & 1 != 0 {
            return Err(SharedAudioError::Busy);
        }
        let metadata = self.layout.read_into(&self.map, samples)?;
        if state.load(std::sync::atomic::Ordering::Acquire) != before {
            return Err(SharedAudioError::TornRead);
        }
        Ok(metadata)
    }

    pub fn flush(&mut self) -> Result<(), SharedAudioError> {
        self.map
            .flush()
            .map_err(|error| SharedAudioError::Io(error.to_string()))
    }

    pub fn layout(&self) -> SharedAudioLayout {
        self.layout
    }

    pub fn file(&self) -> &fs::File {
        &self.file
    }

    fn state(&self) -> &std::sync::atomic::AtomicU64 {
        // The mapping starts page-aligned and the state offset is 8-byte
        // aligned. The region is created/opened at the exact layout length.
        unsafe {
            &*(self.map.as_ptr().add(SHARED_AUDIO_STATE_OFFSET)
                as *const std::sync::atomic::AtomicU64)
        }
    }
}

/// A caller-owned pair of mapped audio slots for a host/worker exchange.
/// `input_path` is written by the host and read by the worker; the output
/// direction is the reverse. The paths and lifecycle remain explicit so this
/// transport never creates a machine-wide IPC name or accesses audio devices.
pub struct SharedAudioTransport {
    input: SharedAudioRegion,
    output: SharedAudioRegion,
    input_path: PathBuf,
    output_path: PathBuf,
}

impl SharedAudioTransport {
    pub fn create(
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
        layout: SharedAudioLayout,
    ) -> Result<Self, SharedAudioError> {
        let input_path = input_path.as_ref();
        let output_path = output_path.as_ref();
        if input_path == output_path {
            return Err(SharedAudioError::AliasedPaths);
        }
        let input = SharedAudioRegion::create(input_path, layout)?;
        match SharedAudioRegion::create(output_path, layout) {
            Ok(output) => Ok(Self {
                input,
                output,
                input_path: input_path.to_path_buf(),
                output_path: output_path.to_path_buf(),
            }),
            Err(error) => {
                drop(input);
                let _ = fs::remove_file(input_path);
                Err(error)
            }
        }
    }

    pub fn open(
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
        layout: SharedAudioLayout,
    ) -> Result<Self, SharedAudioError> {
        if input_path.as_ref() == output_path.as_ref() {
            return Err(SharedAudioError::AliasedPaths);
        }
        Ok(Self {
            input: SharedAudioRegion::open(input_path.as_ref(), layout)?,
            output: SharedAudioRegion::open(output_path.as_ref(), layout)?,
            input_path: input_path.as_ref().to_path_buf(),
            output_path: output_path.as_ref().to_path_buf(),
        })
    }

    pub fn write_input(&mut self, frame: &WorkerFrame) -> Result<(), SharedAudioError> {
        self.input.write(frame)
    }

    pub fn read_input(&self) -> Result<WorkerFrame, SharedAudioError> {
        self.input.read()
    }

    pub fn read_input_into(
        &self,
        samples: &mut [f32],
    ) -> Result<SharedAudioMetadata, SharedAudioError> {
        self.input.read_into(samples)
    }

    pub fn write_output(&mut self, frame: &WorkerFrame) -> Result<(), SharedAudioError> {
        self.output.write(frame)
    }

    pub fn read_output(&self) -> Result<WorkerFrame, SharedAudioError> {
        self.output.read()
    }

    pub fn read_output_into(
        &self,
        samples: &mut [f32],
    ) -> Result<SharedAudioMetadata, SharedAudioError> {
        self.output.read_into(samples)
    }

    pub fn flush(&mut self) -> Result<(), SharedAudioError> {
        self.input.flush()?;
        self.output.flush()
    }

    pub fn input_path(&self) -> &Path {
        &self.input_path
    }

    pub fn output_path(&self) -> &Path {
        &self.output_path
    }
}

/// Describes one fixed-capacity audio slot for a future OS shared mapping.
/// The slot itself is fixed-size and endian-stable; decoding returns an owned
/// validated frame for the control boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SharedAudioLayout {
    channels: u16,
}

impl SharedAudioLayout {
    pub fn new(channels: u16) -> Result<Self, SharedAudioError> {
        if !matches!(channels, 1 | 2) {
            return Err(SharedAudioError::InvalidChannels);
        }
        Ok(Self { channels })
    }

    pub fn channels(self) -> u16 {
        self.channels
    }

    pub fn buffer_len(self) -> usize {
        SHARED_AUDIO_HEADER_BYTES + MAX_WORKER_FRAMES * self.channels as usize * 4
    }

    pub fn write(
        &self,
        destination: &mut [u8],
        frame: &WorkerFrame,
    ) -> Result<(), SharedAudioError> {
        if destination.len() < self.buffer_len() {
            return Err(SharedAudioError::BufferTooSmall);
        }
        WorkerFrame::new(
            frame.sequence,
            frame.deadline_tick,
            frame.channels,
            frame.samples.clone(),
        )
        .map_err(SharedAudioError::InvalidFrame)?;
        if frame.channels != self.channels {
            return Err(SharedAudioError::InvalidChannels);
        }
        let frame_count = frame.frame_count();
        destination[..SHARED_AUDIO_STATE_OFFSET].fill(0);
        destination[SHARED_AUDIO_HEADER_BYTES..self.buffer_len()].fill(0);
        destination[..4].copy_from_slice(b"ARSH");
        destination[4..6].copy_from_slice(&1u16.to_le_bytes());
        destination[6..8].copy_from_slice(&self.channels.to_le_bytes());
        destination[8..12].copy_from_slice(&(frame_count as u32).to_le_bytes());
        destination[12..20].copy_from_slice(&frame.sequence.to_le_bytes());
        destination[20..28].copy_from_slice(&frame.deadline_tick.to_le_bytes());
        for (index, sample) in frame.samples.iter().enumerate() {
            let offset = SHARED_AUDIO_HEADER_BYTES + index * 4;
            destination[offset..offset + 4].copy_from_slice(&sample.to_le_bytes());
        }
        Ok(())
    }

    pub fn read(&self, source: &[u8]) -> Result<WorkerFrame, SharedAudioError> {
        if source.len() < self.buffer_len() {
            return Err(SharedAudioError::BufferTooSmall);
        }
        if &source[..4] != b"ARSH" {
            return Err(SharedAudioError::InvalidMagic);
        }
        if u16::from_le_bytes(source[4..6].try_into().unwrap()) != 1 {
            return Err(SharedAudioError::InvalidVersion);
        }
        if u16::from_le_bytes(source[6..8].try_into().unwrap()) != self.channels {
            return Err(SharedAudioError::InvalidChannels);
        }
        let frame_count = u32::from_le_bytes(source[8..12].try_into().unwrap()) as usize;
        if frame_count == 0 || frame_count > MAX_WORKER_FRAMES {
            return Err(SharedAudioError::InvalidFrameCount);
        }
        let sample_count = frame_count * self.channels as usize;
        let sequence = u64::from_le_bytes(source[12..20].try_into().unwrap());
        let deadline_tick = u64::from_le_bytes(source[20..28].try_into().unwrap());
        let samples = (0..sample_count)
            .map(|index| {
                let offset = SHARED_AUDIO_HEADER_BYTES + index * 4;
                f32::from_le_bytes(source[offset..offset + 4].try_into().unwrap())
            })
            .collect::<Vec<_>>();
        WorkerFrame::new(sequence, deadline_tick, self.channels, samples)
            .map_err(SharedAudioError::InvalidFrame)
    }

    pub fn read_into(
        &self,
        source: &[u8],
        samples: &mut [f32],
    ) -> Result<SharedAudioMetadata, SharedAudioError> {
        if source.len() < self.buffer_len() {
            return Err(SharedAudioError::BufferTooSmall);
        }
        if &source[..4] != b"ARSH" {
            return Err(SharedAudioError::InvalidMagic);
        }
        if u16::from_le_bytes(source[4..6].try_into().unwrap()) != 1 {
            return Err(SharedAudioError::InvalidVersion);
        }
        if u16::from_le_bytes(source[6..8].try_into().unwrap()) != self.channels {
            return Err(SharedAudioError::InvalidChannels);
        }
        let frames = u32::from_le_bytes(source[8..12].try_into().unwrap()) as usize;
        if frames == 0 || frames > MAX_WORKER_FRAMES {
            return Err(SharedAudioError::InvalidFrameCount);
        }
        let sample_count = frames * self.channels as usize;
        if samples.len() < sample_count {
            return Err(SharedAudioError::BufferTooSmall);
        }
        for (index, destination) in samples[..sample_count].iter_mut().enumerate() {
            let offset = SHARED_AUDIO_HEADER_BYTES + index * 4;
            *destination = f32::from_le_bytes(source[offset..offset + 4].try_into().unwrap());
            if !destination.is_finite() {
                return Err(SharedAudioError::InvalidFrame(
                    WorkerFrameError::NonFiniteSample,
                ));
            }
        }
        Ok(SharedAudioMetadata {
            sequence: u64::from_le_bytes(source[12..20].try_into().unwrap()),
            deadline_tick: u64::from_le_bytes(source[20..28].try_into().unwrap()),
            channels: self.channels,
            frames,
        })
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
        self.accept_metadata(frame.sequence, frame.deadline_tick, now_tick)
    }

    pub fn accept_metadata(
        &mut self,
        sequence: u64,
        deadline_tick: u64,
        now_tick: u64,
    ) -> Result<(), WorkerFrameError> {
        if self.last_sequence.is_some_and(|last| sequence <= last) {
            return Err(WorkerFrameError::SequenceRegression);
        }
        if deadline_tick < now_tick {
            return Err(WorkerFrameError::DeadlineExpired);
        }
        self.last_sequence = Some(sequence);
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
    fn processed_frames_must_preserve_transport_identity_and_shape() {
        let expected = WorkerFrame::new(7, 20, 2, vec![0.0; 4]).unwrap();
        assert!(processed_frame_matches(&expected, &expected));
        for actual in [
            WorkerFrame::new(8, 20, 2, vec![0.0; 4]).unwrap(),
            WorkerFrame::new(7, 21, 2, vec![0.0; 4]).unwrap(),
            WorkerFrame::new(7, 20, 1, vec![0.0; 4]).unwrap(),
            WorkerFrame::new(7, 20, 2, vec![0.0; 2]).unwrap(),
        ] {
            assert!(!processed_frame_matches(&expected, &actual));
        }
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
            supervisor.start(&identity, now + Duration::from_secs(1)),
            Err(WorkerStartError::AlreadyRunning)
        );
        assert_eq!(
            supervisor.poll(now + WORKER_HEARTBEAT_TIMEOUT + Duration::from_millis(1)),
            WorkerState::Failed
        );
        assert!(!supervisor.heartbeat(now));
        supervisor.deliberate_retry();
        assert_eq!(supervisor.state(), WorkerState::Stopped);
    }

    #[test]
    fn worker_supervisor_accepts_immediate_failures_and_quarantines_repeated_faults() {
        let identity = PluginIdentity {
            path: PathBuf::from("effect.vst3"),
            binary_path: PathBuf::from("effect.vst3"),
            format: PluginFormat::Vst3,
            architecture: PeArchitecture::X64,
            file_bytes: 1,
            sha256: "0".repeat(64),
        };
        let start = Instant::now();
        let mut supervisor = WorkerSupervisor::new();
        for expected in [WorkerState::Failed, WorkerState::Failed] {
            assert_eq!(supervisor.start(&identity, start), Ok(()));
            assert_eq!(supervisor.record_failure(start), expected);
        }
        assert_eq!(supervisor.start(&identity, start), Ok(()));
        assert_eq!(supervisor.record_failure(start), WorkerState::Quarantined);
        assert_eq!(supervisor.record_failure(start), WorkerState::Quarantined);
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
        let link = root.join("state-link.bin");
        #[cfg(windows)]
        let link_result = std::os::windows::fs::symlink_file(&path, &link);
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&path, &link);
        if link_result.is_ok() {
            assert_eq!(
                read_state_asset(&root, &link, 4, &asset.sha256),
                Err(StateFileError::OutsideRoot)
            );
            fs::remove_file(&link).unwrap();
        }
        let target_dir = root.join("state-target");
        fs::create_dir(&target_dir).unwrap();
        let nested_target = target_dir.join("nested.bin");
        fs::write(&nested_target, &asset.bytes).unwrap();
        let nested_link = root.join("state-directory-link");
        #[cfg(windows)]
        let nested_link_result = std::os::windows::fs::symlink_dir(&target_dir, &nested_link);
        #[cfg(unix)]
        let nested_link_result = std::os::unix::fs::symlink(&target_dir, &nested_link);
        if nested_link_result.is_ok() {
            let nested_path = nested_link.join("nested.bin");
            assert_eq!(
                read_state_asset(&root, &nested_path, 4, &asset.sha256),
                Err(StateFileError::OutsideRoot)
            );
            fs::remove_dir(&nested_link).unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn worker_executable_requires_an_absolute_regular_path() {
        assert!(validate_worker_executable(Path::new("worker.exe"))
            .unwrap_err()
            .contains("must be absolute"));
        let root = temp_root();
        let executable = root.join("worker.exe");
        fs::write(&executable, b"fixture").unwrap();
        assert_eq!(
            validate_worker_executable(&executable).unwrap(),
            executable.canonicalize().unwrap()
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
    fn worker_response_reader_has_a_bounded_wait() {
        let (_sender, receiver) = mpsc::sync_channel(1);
        let started = Instant::now();
        let result = receive_worker_message(&receiver, Duration::from_millis(5));
        assert!(
            matches!(result, Err(WorkerMessageError::Io(message)) if message.contains("timed out"))
        );
        assert!(started.elapsed() < Duration::from_secs(1));
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

    #[test]
    fn shared_audio_layout_round_trips_and_rejects_corruption() {
        let layout = SharedAudioLayout::new(2).unwrap();
        let frame = WorkerFrame::new(9, 100, 2, vec![0.25, -0.5, 1.0, 0.0]).unwrap();
        let mut slot = vec![0u8; layout.buffer_len()];
        layout.write(&mut slot, &frame).unwrap();
        assert_eq!(layout.read(&slot).unwrap(), frame);
        let mut samples = [0.0; 4];
        assert_eq!(
            layout.read_into(&slot, &mut samples).unwrap(),
            SharedAudioMetadata {
                sequence: 9,
                deadline_tick: 100,
                channels: 2,
                frames: 2
            }
        );
        assert_eq!(&samples, &frame.samples[..]);
        slot[0] = b'X';
        assert_eq!(layout.read(&slot), Err(SharedAudioError::InvalidMagic));
        assert_eq!(
            layout.write(&mut slot[..31], &frame),
            Err(SharedAudioError::BufferTooSmall)
        );
    }

    #[test]
    fn shared_audio_region_is_reopenable_and_refuses_relative_paths() {
        let layout = SharedAudioLayout::new(1).unwrap();
        let path =
            std::env::temp_dir().join(format!("audiorouter-shared-audio-{}", std::process::id()));
        let _ = fs::remove_file(&path);
        let frame = WorkerFrame::new(4, 90, 1, vec![0.125, -0.25]).unwrap();
        let mut writer = SharedAudioRegion::create(&path, layout).unwrap();
        assert!(matches!(writer.read(), Err(SharedAudioError::Empty)));
        writer.write(&frame).unwrap();
        writer.flush().unwrap();
        assert_eq!(
            writer.write(&frame),
            Err(SharedAudioError::SequenceRegression)
        );
        drop(writer);
        let reader = SharedAudioRegion::open(&path, layout).unwrap();
        assert_eq!(reader.read().unwrap(), frame);
        let mut samples = [0.0; 2];
        assert_eq!(reader.read_into(&mut samples).unwrap().frames, 2);
        assert_eq!(&samples, &frame.samples[..]);
        assert!(matches!(
            SharedAudioRegion::open("relative-slot", layout),
            Err(SharedAudioError::InvalidPath)
        ));
        drop(reader);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn shared_audio_transport_exchanges_input_and_output_slots() {
        let layout = SharedAudioLayout::new(2).unwrap();
        let stem = format!("audiorouter-shared-transport-{}", std::process::id());
        let input_path = std::env::temp_dir().join(format!("{}-input", stem));
        let output_path = std::env::temp_dir().join(format!("{}-output", stem));
        let _ = fs::remove_file(&input_path);
        let _ = fs::remove_file(&output_path);

        fs::write(&output_path, b"occupied").unwrap();
        assert!(matches!(
            SharedAudioTransport::create(&input_path, &output_path, layout),
            Err(SharedAudioError::Exists)
        ));
        assert!(!input_path.exists());
        fs::remove_file(&output_path).unwrap();

        let mut host = SharedAudioTransport::create(&input_path, &output_path, layout).unwrap();
        let mut worker = SharedAudioTransport::open(&input_path, &output_path, layout).unwrap();
        let input = WorkerFrame::new(1, 100, 2, vec![0.25, -0.25, 0.0, 0.1]).unwrap();
        host.write_input(&input).unwrap();

        let mut input_samples = [0.0; 4];
        let input_metadata = worker.read_input_into(&mut input_samples).unwrap();
        assert_eq!(input_metadata.sequence, input.sequence);
        assert_eq!(&input_samples, &input.samples[..]);

        let output = WorkerFrame::new(1, 100, 2, vec![0.5, -0.5, 0.0, 0.2]).unwrap();
        worker.write_output(&output).unwrap();
        let mut output_samples = [0.0; 4];
        let output_metadata = host.read_output_into(&mut output_samples).unwrap();
        assert_eq!(output_metadata.sequence, output.sequence);
        assert_eq!(&output_samples, &output.samples[..]);

        assert!(matches!(
            SharedAudioTransport::create(&input_path, &input_path, layout),
            Err(SharedAudioError::AliasedPaths)
        ));
        drop(worker);
        drop(host);
        fs::remove_file(input_path).unwrap();
        fs::remove_file(output_path).unwrap();
    }
}
