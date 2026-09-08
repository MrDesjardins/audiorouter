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
pub const MAX_PLUGIN_METADATA_BYTES: u64 = 1024 * 1024;
pub const MAX_FAILURES_BEFORE_QUARANTINE: u32 = 3;
pub const FAILURE_WINDOW: Duration = Duration::from_secs(10 * 60);
pub const MAX_WORKER_FRAMES: usize = 2048;
pub const MAX_SCAN_CANDIDATES: usize = 256;
pub const DEFAULT_SCAN_DEADLINE: Duration = Duration::from_secs(10);
pub const MAX_PLUGIN_STATE_BYTES: usize = 16 * 1024 * 1024;
pub const WORKER_HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(100);
pub const MAX_PARAMETER_EVENTS: usize = 128;
pub const MAX_PARAMETER_DESCRIPTORS: usize = 256;
pub const MAX_PARAMETER_TITLE_BYTES: usize = 128;
pub const MAX_WORKER_MESSAGE_BYTES: usize = 1_024 * 1_024;
pub const MAX_WORKER_FAILURE_CODE_BYTES: usize = 128;
pub const MAX_WORKER_STATE_BYTES: usize = 512 * 1024;
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
    Cancelled,
    DeadlineExceeded,
    Io(String),
}

impl InspectionError {
    /// Stable machine-readable diagnostic for API/CLI consumers.
    pub fn code(&self) -> &'static str {
        match self {
            Self::OutsideConfiguredRoot => "outsideConfiguredRoot",
            Self::UnsupportedExtension => "unsupportedExtension",
            Self::Missing => "missing",
            Self::TooLarge => "tooLarge",
            Self::NotPe => "notPe",
            Self::UnsupportedArchitecture => "unsupportedArchitecture",
            Self::Cancelled => "cancelled",
            Self::DeadlineExceeded => "deadlineExceeded",
            Self::Io(_) => "io",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdentityVerificationError {
    Inspection(InspectionError),
    Changed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateError {
    Empty,
    InvalidVersion,
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PluginStateAsset {
    pub version: u32,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

impl PluginStateAsset {
    pub fn new(version: u32, bytes: Vec<u8>) -> Result<Self, StateError> {
        if version == 0 {
            return Err(StateError::InvalidVersion);
        }
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
        if self.version == 0 || expected_version == 0 {
            return Err(StateError::InvalidVersion);
        }
        if self.bytes.is_empty() {
            return Err(StateError::Empty);
        }
        if self.bytes.len() > MAX_PLUGIN_STATE_BYTES {
            return Err(StateError::TooLarge);
        }
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
    let file =
        fs::File::open(canonical_path).map_err(|error| StateFileError::Io(error.to_string()))?;
    let mut bytes = Vec::with_capacity(metadata.len().min(MAX_PLUGIN_STATE_BYTES as u64) as usize);
    file.take(MAX_PLUGIN_STATE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| StateFileError::Io(error.to_string()))?;
    if bytes.len() > MAX_PLUGIN_STATE_BYTES {
        return Err(StateFileError::TooLarge);
    }
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PluginMetadata {
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub class_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginIdentity {
    pub path: PathBuf,
    pub binary_path: PathBuf,
    pub format: PluginFormat,
    pub architecture: PeArchitecture,
    pub file_bytes: u64,
    pub sha256: String,
    pub metadata: PluginMetadata,
}

impl PluginIdentity {
    pub fn compatibility(&self) -> PluginCompatibility {
        if self.format == PluginFormat::Vst3 && self.architecture == PeArchitecture::X64 {
            PluginCompatibility::SupportedVst3X64
        } else {
            PluginCompatibility::UnsupportedFormat
        }
    }

    /// Reinspect the exact selected path before a worker is launched. This
    /// prevents a discovery result from authorizing a replacement binary or
    /// reparse-point path. Optional module metadata is intentionally excluded:
    /// execution authorization is bound to the canonical binary fingerprint.
    pub fn verify_current(
        &self,
        configured_roots: &[PathBuf],
    ) -> Result<(), IdentityVerificationError> {
        let current = inspect_binary(&self.path, configured_roots)
            .map_err(IdentityVerificationError::Inspection)?;
        if current.path != self.path
            || current.binary_path != self.binary_path
            || current.format != self.format
            || current.architecture != self.architecture
            || current.file_bytes != self.file_bytes
            || current.sha256 != self.sha256
        {
            return Err(IdentityVerificationError::Changed);
        }
        Ok(())
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

impl ScanError {
    /// Stable machine-readable diagnostic for a directory scan failure.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidRoot => "invalidRoot",
            Self::TooManyCandidates => "tooManyCandidates",
            Self::Cancelled => "cancelled",
            Self::DeadlineExceeded => "deadlineExceeded",
            Self::Io(_) => "io",
        }
    }
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

    fn for_candidate(&self) -> Self {
        let candidate_deadline = Instant::now()
            .checked_add(DEFAULT_SCAN_DEADLINE)
            .unwrap_or(self.deadline);
        Self {
            deadline: self.deadline.min(candidate_deadline),
            cancelled: Arc::clone(&self.cancelled),
        }
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
    inspect_binary_with_control(path, configured_roots, None)
}

fn inspect_binary_with_control(
    path: &Path,
    configured_roots: &[PathBuf],
    control: Option<&ScanControl>,
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
    let mut file =
        fs::File::open(&binary_path).map_err(|error| InspectionError::Io(error.to_string()))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut chunk = [0_u8; 64 * 1024];
    loop {
        if let Some(control) = control {
            match control.check() {
                Ok(()) => {}
                Err(ScanError::Cancelled) => return Err(InspectionError::Cancelled),
                Err(ScanError::DeadlineExceeded) => return Err(InspectionError::DeadlineExceeded),
                Err(error) => {
                    return Err(InspectionError::Io(format!(
                        "scan control failed: {error:?}"
                    )))
                }
            }
        }
        let read = file
            .read(&mut chunk)
            .map_err(|error| InspectionError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() as u64 > MAX_PLUGIN_BYTES {
            return Err(InspectionError::TooLarge);
        }
    }
    let architecture = parse_pe_architecture(&bytes).ok_or(InspectionError::NotPe)?;
    if architecture != PeArchitecture::X64 {
        return Err(InspectionError::UnsupportedArchitecture);
    }
    let digest = Sha256::digest(&bytes);
    let plugin_metadata = if format == PluginFormat::Vst3 && canonical.is_dir() {
        read_plugin_metadata(&canonical)
    } else {
        PluginMetadata::default()
    };
    Ok(PluginIdentity {
        path: canonical,
        binary_path,
        format,
        architecture,
        file_bytes: metadata.len(),
        sha256: digest.iter().map(|byte| format!("{byte:02x}")).collect(),
        metadata: plugin_metadata,
    })
}

fn read_plugin_metadata(bundle: &Path) -> PluginMetadata {
    let path = bundle
        .join("Contents")
        .join("Resources")
        .join("moduleinfo.json");
    let Ok(file) = fs::File::open(path) else {
        return PluginMetadata::default();
    };
    let mut bytes = Vec::new();
    if file
        .take(MAX_PLUGIN_METADATA_BYTES + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() as u64 > MAX_PLUGIN_METADATA_BYTES
    {
        return PluginMetadata::default();
    }
    let normalized = strip_json_trailing_commas(&bytes);
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&normalized) else {
        return PluginMetadata::default();
    };
    let vendor = value
        .get("Factory Info")
        .and_then(|info| info.get("Vendor"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .map(str::to_owned);
    let version = value
        .get("Version")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .map(str::to_owned);
    let mut class_ids = Vec::new();
    if let Some(entries) = value
        .get("Compatibility")
        .and_then(serde_json::Value::as_array)
    {
        for entry in entries {
            let Some(id) = entry.get("New").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if id.len() <= 32
                && class_ids.len() < 256
                && !class_ids.iter().any(|existing| existing == id)
            {
                class_ids.push(id.to_owned());
            }
        }
    }
    PluginMetadata {
        vendor,
        version,
        class_ids,
    }
}

fn strip_json_trailing_commas(bytes: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(bytes.len());
    let mut in_string = false;
    let mut escaped = false;
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if in_string {
            result.push(byte);
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if byte == b'"' {
            in_string = true;
            result.push(byte);
            index += 1;
            continue;
        }
        if byte == b',' {
            let mut next = index + 1;
            while next < bytes.len() && bytes[next].is_ascii_whitespace() {
                next += 1;
            }
            if next < bytes.len() && matches!(bytes[next], b'}' | b']') {
                index += 1;
                continue;
            }
        }
        result.push(byte);
        index += 1;
    }
    result
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
    let root_metadata = fs::symlink_metadata(root).map_err(|_| ScanError::InvalidRoot)?;
    if !root_metadata.is_dir() || is_reparse_point(&root_metadata) {
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
        let candidate_control = control.for_candidate();
        entries.push(
            match inspect_binary_with_control(
                &path,
                &[root.to_path_buf()],
                Some(&candidate_control),
            ) {
                Ok(identity) => ScanEntry {
                    path,
                    identity: Some(identity),
                    error: None,
                },
                Err(InspectionError::Cancelled) => return Err(ScanError::Cancelled),
                Err(InspectionError::DeadlineExceeded) => return Err(ScanError::DeadlineExceeded),
                Err(error) => ScanEntry {
                    path,
                    identity: None,
                    error: Some(error),
                },
            },
        );
    }
    Ok(entries)
}

fn root_contains(root: &Path, candidate: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(root) else {
        return false;
    };
    if !metadata.is_dir() || path_has_reparse_ancestor(root) {
        return false;
    }
    fs::canonicalize(root)
        .map(|root| candidate.starts_with(root))
        .unwrap_or(false)
}

fn path_has_reparse_ancestor(path: &Path) -> bool {
    let mut current = Some(path);
    while let Some(component) = current {
        if let Ok(metadata) = fs::symlink_metadata(component) {
            if is_reparse_point(&metadata) {
                return true;
            }
        }
        current = component.parent();
    }
    false
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
    InvalidParameterDescriptor(ParameterDescriptorError),
    InvalidProtocolVersion,
    InvalidPluginHash,
    InvalidFailureCode,
    InvalidState,
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
    InvalidState,
    InvalidParameterDescriptor(ParameterDescriptorError),
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

/// Control-plane state for an optional worker-owned native editor. The editor
/// lifecycle is deliberately separate from processing generation ownership:
/// opening, closing, or failing an editor must not restart audio processing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorState {
    Closed,
    Open,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorError {
    AlreadyOpen,
    AlreadyClosed,
    NotOpen,
    RetryRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EditorLifecycle {
    state: EditorState,
    processing_generation: u64,
}

impl EditorLifecycle {
    pub fn new(processing_generation: u64) -> Self {
        Self {
            state: EditorState::Closed,
            processing_generation,
        }
    }

    pub fn state(self) -> EditorState {
        self.state
    }

    pub fn processing_generation(self) -> u64 {
        self.processing_generation
    }

    pub fn open(&mut self) -> Result<(), EditorError> {
        match self.state {
            EditorState::Closed => {
                self.state = EditorState::Open;
                Ok(())
            }
            EditorState::Open => Err(EditorError::AlreadyOpen),
            EditorState::Failed => Err(EditorError::RetryRequired),
        }
    }

    pub fn close(&mut self) -> Result<(), EditorError> {
        if self.state == EditorState::Closed {
            return Err(EditorError::AlreadyClosed);
        }
        self.state = EditorState::Closed;
        Ok(())
    }

    pub fn fail(&mut self) -> Result<(), EditorError> {
        if self.state != EditorState::Open {
            return Err(EditorError::NotOpen);
        }
        self.state = EditorState::Failed;
        Ok(())
    }

    pub fn retry(&mut self) -> Result<(), EditorError> {
        if self.state != EditorState::Failed {
            return Err(EditorError::NotOpen);
        }
        self.state = EditorState::Closed;
        Ok(())
    }
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

    pub fn failure_count(&self) -> u32 {
        self.failures.failures()
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParameterDescriptor {
    pub parameter_id: u32,
    pub title: String,
    pub default_value: f32,
    pub minimum: f32,
    pub maximum: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParameterDescriptorError {
    TooMany,
    EmptyTitle,
    TitleTooLong,
    NonFiniteValue,
    InvalidRange,
    DuplicateId,
}

impl ParameterDescriptor {
    pub fn new(
        parameter_id: u32,
        title: impl Into<String>,
        default_value: f32,
        minimum: f32,
        maximum: f32,
    ) -> Result<Self, ParameterDescriptorError> {
        let title = title.into();
        if title.is_empty() {
            return Err(ParameterDescriptorError::EmptyTitle);
        }
        if title.len() > MAX_PARAMETER_TITLE_BYTES {
            return Err(ParameterDescriptorError::TitleTooLong);
        }
        if !default_value.is_finite() || !minimum.is_finite() || !maximum.is_finite() {
            return Err(ParameterDescriptorError::NonFiniteValue);
        }
        if !(0.0..=1.0).contains(&minimum)
            || !(0.0..=1.0).contains(&maximum)
            || minimum >= maximum
            || !(minimum..=maximum).contains(&default_value)
        {
            return Err(ParameterDescriptorError::InvalidRange);
        }
        Ok(Self {
            parameter_id,
            title,
            default_value,
            minimum,
            maximum,
        })
    }
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
    DescribeParameters,
    Parameters {
        descriptors: Vec<ParameterDescriptor>,
    },
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
    StateSave,
    StateRestore {
        asset: PluginStateAsset,
    },
    State {
        asset: PluginStateAsset,
    },
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
/// Process creation and OS-level isolation are provided by `WorkerProcess`;
/// this portable state machine remains independent of the operating system.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerSession {
    expected_plugin_sha256: String,
    channels: u16,
    state: WorkerSessionState,
    frame_guard: WorkerFrameGuard,
    latency: Option<(u32, u32)>,
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
            latency: None,
        })
    }

    pub fn state(&self) -> WorkerSessionState {
        self.state
    }

    /// Return the most recent worker-reported latency. The sample rate is
    /// fixed for a session; plugins may change their sample count dynamically
    /// but may not change the graph's negotiated rate underneath it.
    pub fn latency(&self) -> Option<WorkerLatency> {
        self.latency.map(|(samples, sample_rate_hz)| WorkerLatency {
            samples,
            sample_rate_hz,
        })
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
            WorkerMessageError::InvalidState => WorkerSessionError::InvalidState,
            WorkerMessageError::InvalidParameterDescriptor(error) => {
                WorkerSessionError::InvalidParameterDescriptor(error)
            }
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
            (WorkerSessionState::Active, WorkerMessage::Latency(latency)) => {
                if self
                    .latency
                    .is_some_and(|(_, sample_rate_hz)| sample_rate_hz != latency.sample_rate_hz)
                {
                    return Err(WorkerSessionError::InvalidLatency);
                }
                self.latency = Some((latency.samples, latency.sample_rate_hz));
                Ok(None)
            }
            (
                WorkerSessionState::Active,
                WorkerMessage::ProcessedShared { .. }
                | WorkerMessage::StateSave
                | WorkerMessage::StateRestore { .. },
            ) => Ok(None),
            (WorkerSessionState::Active, WorkerMessage::DescribeParameters) => Ok(None),
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
    validate_worker_message(message)?;
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
    PluginIdentity(IdentityVerificationError),
    State(StateError),
    /// The bounded failure policy has latched quarantine; replacement is not
    /// permitted until an explicit operator retry clears that policy.
    Quarantined,
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

/// A worker process coupled to the bounded lifecycle policy. Successful
/// protocol exchanges refresh the heartbeat; spawn, processing, and latency
/// failures are recorded immediately. This wrapper deliberately does not
/// restart a process or open an audio device: an outer runtime supervisor
/// still owns restart and route-recovery decisions.
pub struct SupervisedWorkerProcess {
    process: WorkerProcess,
    supervisor: WorkerSupervisor,
    executable: PathBuf,
    identity: PluginIdentity,
    channels: u16,
    shared_transport: bool,
}

impl SupervisedWorkerProcess {
    pub fn spawn(
        executable: impl AsRef<Path>,
        identity: &PluginIdentity,
        channels: u16,
        now: Instant,
    ) -> Result<Self, WorkerProcessError> {
        let supervisor = WorkerSupervisor::new();
        Self::spawn_with_supervisor(executable, identity, channels, supervisor, now)
            .map_err(|(error, _)| error)
    }

    /// Spawn only after revalidating the exact scanned plugin identity against
    /// the caller's configured roots. This is the safe discovery-to-worker
    /// launch path; it never substitutes a different plugin path.
    pub fn spawn_verified(
        executable: impl AsRef<Path>,
        identity: &PluginIdentity,
        configured_roots: &[PathBuf],
        channels: u16,
        now: Instant,
    ) -> Result<Self, WorkerProcessError> {
        identity
            .verify_current(configured_roots)
            .map_err(WorkerProcessError::PluginIdentity)?;
        Self::spawn(executable, identity, channels, now)
    }

    #[cfg(feature = "test-fixtures")]
    pub fn spawn_fixture(
        executable: impl AsRef<Path>,
        identity: &PluginIdentity,
        channels: u16,
        mode: &str,
        now: Instant,
    ) -> Result<Self, WorkerProcessError> {
        let executable =
            validate_worker_executable(executable.as_ref()).map_err(WorkerProcessError::Spawn)?;
        let mut supervisor = WorkerSupervisor::new();
        supervisor.start(identity, now).map_err(|error| {
            WorkerProcessError::Protocol(format!("worker start rejected: {error:?}"))
        })?;
        let process = WorkerProcess::spawn_fixture(&executable, &identity.sha256, channels, mode)?;
        Ok(Self {
            process,
            supervisor,
            executable,
            identity: identity.clone(),
            channels,
            shared_transport: false,
        })
    }

    /// Spawn a replacement while preserving the caller-owned failure ledger.
    /// The ledger is returned with an error so an outer supervisor cannot lose
    /// quarantine history when a replacement fails to launch.
    pub fn spawn_with_supervisor(
        executable: impl AsRef<Path>,
        identity: &PluginIdentity,
        channels: u16,
        mut supervisor: WorkerSupervisor,
        now: Instant,
    ) -> Result<Self, (WorkerProcessError, WorkerSupervisor)> {
        let executable = match validate_worker_executable(executable.as_ref()) {
            Ok(path) => path,
            Err(error) => {
                supervisor.record_failure(now);
                return Err((WorkerProcessError::Spawn(error), supervisor));
            }
        };
        if let Err(error) = supervisor.start(identity, now) {
            if error == WorkerStartError::Quarantined {
                return Err((WorkerProcessError::Quarantined, supervisor));
            }
            return Err((
                WorkerProcessError::Protocol(format!("worker start rejected: {error:?}")),
                supervisor,
            ));
        }
        match WorkerProcess::spawn(&executable, &identity.sha256, channels) {
            Ok(process) => Ok(Self {
                process,
                supervisor,
                executable,
                identity: identity.clone(),
                channels,
                shared_transport: false,
            }),
            Err(error) => {
                supervisor.record_failure(now);
                Err((error, supervisor))
            }
        }
    }

    pub fn spawn_shared(
        executable: impl AsRef<Path>,
        identity: &PluginIdentity,
        channels: u16,
        transport: SharedAudioTransport,
        now: Instant,
    ) -> Result<Self, WorkerProcessError> {
        Self::spawn_shared_with_supervisor(
            executable,
            identity,
            channels,
            transport,
            WorkerSupervisor::new(),
            now,
        )
        .map_err(|(error, _)| error)
    }

    fn spawn_shared_with_supervisor(
        executable: impl AsRef<Path>,
        identity: &PluginIdentity,
        channels: u16,
        transport: SharedAudioTransport,
        mut supervisor: WorkerSupervisor,
        now: Instant,
    ) -> Result<Self, (WorkerProcessError, WorkerSupervisor)> {
        let executable = match validate_worker_executable(executable.as_ref()) {
            Ok(path) => path,
            Err(error) => {
                supervisor.record_failure(now);
                return Err((WorkerProcessError::Spawn(error), supervisor));
            }
        };
        if let Err(error) = supervisor.start(identity, now) {
            if error == WorkerStartError::Quarantined {
                return Err((WorkerProcessError::Quarantined, supervisor));
            }
            return Err((
                WorkerProcessError::Protocol(format!("worker start rejected: {error:?}")),
                supervisor,
            ));
        }
        match WorkerProcess::spawn_shared(&executable, &identity.sha256, channels, transport) {
            Ok(process) => Ok(Self {
                process,
                supervisor,
                executable,
                identity: identity.clone(),
                channels,
                shared_transport: true,
            }),
            Err(error) => {
                supervisor.record_failure(now);
                Err((error, supervisor))
            }
        }
    }

    pub fn state(&self) -> WorkerState {
        self.supervisor.state()
    }

    pub fn poll(&mut self, now: Instant) -> WorkerState {
        if self.supervisor.state() == WorkerState::Running {
            match self.process.child.try_wait() {
                Ok(Some(_)) | Err(_) => {
                    self.supervisor.record_failure(now);
                }
                Ok(None) => {}
            }
        }
        let state = self.supervisor.poll(now);
        if matches!(state, WorkerState::Failed | WorkerState::Quarantined) {
            terminate_child(&mut self.process.child);
        }
        state
    }

    /// Poll the worker once and perform at most one replacement when the
    /// bounded supervisor marks it failed. Quarantine is never bypassed, and
    /// this method does not loop or silently restore a route; the owning
    /// runtime decides when to call the next recovery attempt.
    pub fn poll_and_restart(
        mut self,
        now: Instant,
    ) -> Result<Self, (WorkerProcessError, WorkerSupervisor)> {
        if self.poll(now) == WorkerState::Running {
            return Ok(self);
        }
        self.restart(now)
    }

    /// Record a process exit or other failure observed by an outer runtime
    /// adapter. The process is not restarted here; callers must discard this
    /// wrapper and deliberately construct a replacement after policy allows.
    pub fn record_failure(&mut self, now: Instant) -> WorkerState {
        let state = self.supervisor.record_failure(now);
        if matches!(state, WorkerState::Failed | WorkerState::Quarantined) {
            terminate_child(&mut self.process.child);
        }
        state
    }

    pub fn process(
        &mut self,
        frame: WorkerFrame,
        parameters: Vec<ParameterEvent>,
        now: Instant,
    ) -> Result<WorkerFrame, WorkerProcessError> {
        self.ensure_running()?;
        match self.process.process(frame, parameters) {
            Ok(frame) => {
                self.supervisor.heartbeat(now);
                Ok(frame)
            }
            Err(error) => {
                terminate_child(&mut self.process.child);
                self.supervisor.record_failure(now);
                Err(error)
            }
        }
    }

    pub fn process_shared(
        &mut self,
        frame: WorkerFrame,
        parameters: Vec<ParameterEvent>,
        now: Instant,
    ) -> Result<WorkerFrame, WorkerProcessError> {
        self.ensure_running()?;
        match self.process.process_shared(frame, parameters) {
            Ok(frame) => {
                self.supervisor.heartbeat(now);
                Ok(frame)
            }
            Err(error) => {
                terminate_child(&mut self.process.child);
                self.supervisor.record_failure(now);
                Err(error)
            }
        }
    }

    pub fn report_latency(
        &mut self,
        latency: WorkerLatency,
        now: Instant,
    ) -> Result<WorkerLatency, WorkerProcessError> {
        self.ensure_running()?;
        match self.process.report_latency(latency) {
            Ok(latency) => {
                self.supervisor.heartbeat(now);
                Ok(latency)
            }
            Err(error) => {
                terminate_child(&mut self.process.child);
                self.supervisor.record_failure(now);
                Err(error)
            }
        }
    }

    pub fn restore_state(
        &mut self,
        asset: PluginStateAsset,
        now: Instant,
    ) -> Result<(), WorkerProcessError> {
        self.ensure_running()?;
        match self.process.restore_state(asset) {
            Ok(()) => {
                self.supervisor.heartbeat(now);
                Ok(())
            }
            Err(error) => {
                terminate_child(&mut self.process.child);
                self.supervisor.record_failure(now);
                Err(error)
            }
        }
    }

    pub fn describe_parameters(
        &mut self,
        now: Instant,
    ) -> Result<Vec<ParameterDescriptor>, WorkerProcessError> {
        self.ensure_running()?;
        match self.process.describe_parameters() {
            Ok(descriptors) => {
                self.supervisor.heartbeat(now);
                Ok(descriptors)
            }
            Err(error) => {
                terminate_child(&mut self.process.child);
                self.supervisor.record_failure(now);
                Err(error)
            }
        }
    }

    pub fn restore_state_for_version(
        &mut self,
        asset: PluginStateAsset,
        expected_version: u32,
        now: Instant,
    ) -> Result<(), WorkerProcessError> {
        asset
            .verify_for_restore(expected_version)
            .map_err(WorkerProcessError::State)?;
        self.restore_state(asset, now)
    }

    pub fn save_state(&mut self, now: Instant) -> Result<PluginStateAsset, WorkerProcessError> {
        self.ensure_running()?;
        match self.process.save_state() {
            Ok(asset) => {
                self.supervisor.heartbeat(now);
                Ok(asset)
            }
            Err(error) => {
                terminate_child(&mut self.process.child);
                self.supervisor.record_failure(now);
                Err(error)
            }
        }
    }

    pub fn shutdown(self) -> Result<ExitStatus, WorkerProcessError> {
        self.process.shutdown()
    }

    pub fn shutdown_with_timeout(
        self,
        timeout: Duration,
    ) -> Result<ExitStatus, WorkerProcessError> {
        self.process.shutdown_with_timeout(timeout)
    }

    /// Consume the wrapper and return its policy state for a deliberate
    /// replacement. Dropping the process closes the current worker; the
    /// returned ledger remains available for the next spawn attempt.
    pub fn into_supervisor(self) -> WorkerSupervisor {
        let Self {
            process: _process,
            supervisor,
            executable: _executable,
            identity: _identity,
            channels: _channels,
            shared_transport: _shared_transport,
        } = self;
        supervisor
    }

    /// Deliberately replace a failed worker while retaining its failure ledger.
    /// Shared-memory workers carry their caller-owned transport into the
    /// replacement. The current process is dropped before the replacement is
    /// attempted, so this operation never leaves two workers for one slot.
    pub fn restart(self, now: Instant) -> Result<Self, (WorkerProcessError, WorkerSupervisor)> {
        let Self {
            mut process,
            supervisor,
            executable,
            identity,
            channels,
            shared_transport,
        } = self;
        let state = supervisor.state();
        if state == WorkerState::Running {
            return Err((
                WorkerProcessError::Protocol(
                    "worker restart requires a failed or stopped worker".into(),
                ),
                supervisor,
            ));
        }
        let transport = process.take_shared_transport();
        drop(process);
        if shared_transport {
            let Some(transport) = transport else {
                return Err((
                    WorkerProcessError::Protocol(
                        "shared worker transport was unavailable for restart".into(),
                    ),
                    supervisor,
                ));
            };
            Self::spawn_shared_with_supervisor(
                executable, &identity, channels, transport, supervisor, now,
            )
        } else {
            Self::spawn_with_supervisor(executable, &identity, channels, supervisor, now)
        }
    }

    fn ensure_running(&self) -> Result<(), WorkerProcessError> {
        if self.supervisor.state() == WorkerState::Running {
            Ok(())
        } else {
            Err(WorkerProcessError::Protocol(format!(
                "worker is not running under supervision: {:?}",
                self.supervisor.state()
            )))
        }
    }
}

impl WorkerProcess {
    pub fn spawn(
        executable: impl AsRef<Path>,
        plugin_sha256: &str,
        channels: u16,
    ) -> Result<Self, WorkerProcessError> {
        Self::spawn_inner(executable, plugin_sha256, channels, None, None)
    }

    #[cfg(feature = "test-fixtures")]
    pub fn spawn_fixture(
        executable: impl AsRef<Path>,
        plugin_sha256: &str,
        channels: u16,
        mode: &str,
    ) -> Result<Self, WorkerProcessError> {
        if !matches!(mode, "crash" | "hang" | "invalid-output") {
            return Err(WorkerProcessError::Protocol(
                "invalid worker fixture mode".into(),
            ));
        }
        Self::spawn_inner(executable, plugin_sha256, channels, None, Some(mode))
    }

    pub fn spawn_shared(
        executable: impl AsRef<Path>,
        plugin_sha256: &str,
        channels: u16,
        transport: SharedAudioTransport,
    ) -> Result<Self, WorkerProcessError> {
        Self::spawn_inner(executable, plugin_sha256, channels, Some(transport), None)
    }

    fn spawn_inner(
        executable: impl AsRef<Path>,
        plugin_sha256: &str,
        channels: u16,
        mut shared: Option<SharedAudioTransport>,
        fixture_mode: Option<&str>,
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
        if let Some(mode) = fixture_mode {
            command.args(["--fixture-mode", mode]);
        }
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

    pub fn restore_state(&mut self, asset: PluginStateAsset) -> Result<(), WorkerProcessError> {
        self.write(&WorkerMessage::StateRestore {
            asset: asset.clone(),
        })
        .map_err(WorkerProcessError::Message)?;
        match self.read().map_err(WorkerProcessError::Message)? {
            WorkerMessage::State { asset: actual } if actual == asset => Ok(()),
            WorkerMessage::Failure { code } => Err(WorkerProcessError::Protocol(code)),
            _ => Err(WorkerProcessError::Protocol(
                "unexpected state restore response".into(),
            )),
        }
    }

    pub fn describe_parameters(&mut self) -> Result<Vec<ParameterDescriptor>, WorkerProcessError> {
        self.write(&WorkerMessage::DescribeParameters)
            .map_err(WorkerProcessError::Message)?;
        match self.read().map_err(WorkerProcessError::Message)? {
            WorkerMessage::Parameters { descriptors } => Ok(descriptors),
            WorkerMessage::Failure { code } => Err(WorkerProcessError::Protocol(code)),
            _ => Err(WorkerProcessError::Protocol(
                "unexpected parameter description response".into(),
            )),
        }
    }

    pub fn restore_state_for_version(
        &mut self,
        asset: PluginStateAsset,
        expected_version: u32,
    ) -> Result<(), WorkerProcessError> {
        asset
            .verify_for_restore(expected_version)
            .map_err(WorkerProcessError::State)?;
        self.restore_state(asset)
    }

    pub fn save_state(&mut self) -> Result<PluginStateAsset, WorkerProcessError> {
        self.write(&WorkerMessage::StateSave)
            .map_err(WorkerProcessError::Message)?;
        match self.read().map_err(WorkerProcessError::Message)? {
            WorkerMessage::State { asset } => Ok(asset),
            WorkerMessage::Failure { code } => Err(WorkerProcessError::Protocol(code)),
            _ => Err(WorkerProcessError::Protocol(
                "unexpected state save response".into(),
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
            WorkerMessage::Failure { code } => Err(WorkerProcessError::Protocol(code)),
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
        if let Some(status) = self
            .child
            .try_wait()
            .map_err(|error| WorkerProcessError::Spawn(error.to_string()))?
        {
            return Ok(status);
        }
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

    fn take_shared_transport(&mut self) -> Option<SharedAudioTransport> {
        self.shared.take()
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
    if path_has_reparse_ancestor(path) {
        return Err("worker executable path must not have a reparse-point ancestor".into());
    }
    fs::canonicalize(path)
        .map_err(|error| format!("worker executable canonicalization failed: {error}"))
}

fn same_file_identity(left: &Path, right: &Path) -> Result<bool, SharedAudioError> {
    #[cfg(windows)]
    {
        windows_file_identity(left, right)
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let left = fs::metadata(left).map_err(|error| SharedAudioError::Io(error.to_string()))?;
        let right = fs::metadata(right).map_err(|error| SharedAudioError::Io(error.to_string()))?;
        Ok(left.dev() == right.dev() && left.ino() == right.ino())
    }
    #[cfg(not(any(windows, unix)))]
    {
        Ok(false)
    }
}

#[cfg(windows)]
#[repr(C)]
struct NativeFileInformation {
    attributes: u32,
    creation_time_low: u32,
    creation_time_high: u32,
    last_access_time_low: u32,
    last_access_time_high: u32,
    last_write_time_low: u32,
    last_write_time_high: u32,
    volume_serial_number: u32,
    file_size_high: u32,
    file_size_low: u32,
    number_of_links: u32,
    file_index_high: u32,
    file_index_low: u32,
}

#[cfg(windows)]
unsafe extern "system" {
    fn GetFileInformationByHandle(
        handle: *mut std::ffi::c_void,
        information: *mut NativeFileInformation,
    ) -> i32;
}

#[cfg(windows)]
fn windows_file_identity(left: &Path, right: &Path) -> Result<bool, SharedAudioError> {
    use std::os::windows::io::AsRawHandle;

    fn read(path: &Path) -> Result<(u32, u64), SharedAudioError> {
        let file = fs::File::open(path).map_err(|error| SharedAudioError::Io(error.to_string()))?;
        let mut information = NativeFileInformation {
            attributes: 0,
            creation_time_low: 0,
            creation_time_high: 0,
            last_access_time_low: 0,
            last_access_time_high: 0,
            last_write_time_low: 0,
            last_write_time_high: 0,
            volume_serial_number: 0,
            file_size_high: 0,
            file_size_low: 0,
            number_of_links: 0,
            file_index_high: 0,
            file_index_low: 0,
        };
        let succeeded =
            unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) };
        if succeeded == 0 {
            return Err(SharedAudioError::Io(
                std::io::Error::last_os_error().to_string(),
            ));
        }
        Ok((
            information.volume_serial_number,
            (u64::from(information.file_index_high) << 32) | u64::from(information.file_index_low),
        ))
    }

    Ok(read(left)? == read(right)?)
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
            validate_parameters_for_frame(parameters, frame.frame_count())?;
        }
        WorkerMessage::ProcessShared {
            sequence,
            deadline_tick,
            channels,
            frames,
            parameters,
        } => {
            validate_shared_frame_shape(*sequence, *deadline_tick, *channels, *frames)?;
            validate_parameters_for_frame(parameters, *frames as usize)?;
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
        WorkerMessage::Failure { code }
            if code.is_empty() || code.len() > MAX_WORKER_FAILURE_CODE_BYTES =>
        {
            return Err(WorkerMessageError::InvalidFailureCode);
        }
        WorkerMessage::StateSave => {}
        WorkerMessage::StateRestore { asset } | WorkerMessage::State { asset } => {
            validate_worker_state(asset)?;
        }
        WorkerMessage::Parameters { descriptors } => {
            validate_parameter_descriptors(descriptors)?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_worker_state(asset: &PluginStateAsset) -> Result<(), WorkerMessageError> {
    if asset.version == 0
        || asset.bytes.is_empty()
        || asset.bytes.len() > MAX_WORKER_STATE_BYTES
        || !is_sha256(&asset.sha256)
    {
        return Err(WorkerMessageError::InvalidState);
    }
    let digest = Sha256::digest(&asset.bytes);
    if asset.sha256
        != digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    {
        return Err(WorkerMessageError::InvalidState);
    }
    Ok(())
}

fn validate_parameter_descriptors(
    descriptors: &[ParameterDescriptor],
) -> Result<(), WorkerMessageError> {
    if descriptors.len() > MAX_PARAMETER_DESCRIPTORS {
        return Err(WorkerMessageError::InvalidParameterDescriptor(
            ParameterDescriptorError::TooMany,
        ));
    }
    for (index, descriptor) in descriptors.iter().enumerate() {
        ParameterDescriptor::new(
            descriptor.parameter_id,
            descriptor.title.clone(),
            descriptor.default_value,
            descriptor.minimum,
            descriptor.maximum,
        )
        .map_err(WorkerMessageError::InvalidParameterDescriptor)?;
        if descriptors[..index]
            .iter()
            .any(|previous| previous.parameter_id == descriptor.parameter_id)
        {
            return Err(WorkerMessageError::InvalidParameterDescriptor(
                ParameterDescriptorError::DuplicateId,
            ));
        }
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

fn validate_parameters_for_frame(
    parameters: &[ParameterEvent],
    frame_count: usize,
) -> Result<(), WorkerMessageError> {
    validate_parameters(parameters)?;
    if parameters
        .iter()
        .any(|parameter| parameter.sample_offset >= frame_count)
    {
        return Err(WorkerMessageError::InvalidParameter(
            ParameterEventError::OffsetOutOfRange,
        ));
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
        let capacity = capacity.min(MAX_PARAMETER_EVENTS);
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
        let capacity = capacity.min(MAX_WORKER_FRAMES);
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
        if !path.is_absolute() || path_has_reparse_ancestor(path) {
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
        if !path.is_absolute() || path_has_reparse_ancestor(path) {
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
        let input_path = input_path.as_ref();
        let output_path = output_path.as_ref();
        if input_path == output_path {
            return Err(SharedAudioError::AliasedPaths);
        }
        let input_canonical = fs::canonicalize(input_path)
            .map_err(|error| SharedAudioError::Io(error.to_string()))?;
        let output_canonical = fs::canonicalize(output_path)
            .map_err(|error| SharedAudioError::Io(error.to_string()))?;
        if input_canonical == output_canonical || same_file_identity(input_path, output_path)? {
            return Err(SharedAudioError::AliasedPaths);
        }
        Ok(Self {
            input: SharedAudioRegion::open(input_path, layout)?,
            output: SharedAudioRegion::open(output_path, layout)?,
            input_path: input_path.to_path_buf(),
            output_path: output_path.to_path_buf(),
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
    fn plugin_identity_revalidation_rejects_changed_binary_bytes() {
        let root = temp_root();
        let path = root.join("effect.vst3");
        fs::write(&path, pe_x64()).unwrap();
        let identity = inspect_binary(&path, std::slice::from_ref(&root)).unwrap();
        assert_eq!(identity.verify_current(std::slice::from_ref(&root)), Ok(()));

        let mut changed = pe_x64();
        changed[0x90] = 1;
        fs::write(&path, changed).unwrap();
        assert_eq!(
            identity.verify_current(std::slice::from_ref(&root)),
            Err(IdentityVerificationError::Changed)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plugin_identity_revalidation_keeps_root_containment_fail_closed() {
        let root = temp_root();
        let outside = temp_root();
        let path = outside.join("effect.vst3");
        fs::write(&path, pe_x64()).unwrap();
        let identity = inspect_binary(&path, std::slice::from_ref(&outside)).unwrap();
        assert_eq!(
            identity.verify_current(std::slice::from_ref(&root)),
            Err(IdentityVerificationError::Inspection(
                InspectionError::OutsideConfiguredRoot
            ))
        );
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
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
        let resources = bundle.join("Contents").join("Resources");
        fs::create_dir_all(&resources).unwrap();
        fs::write(
            resources.join("moduleinfo.json"),
            br#"{
                "Name": "fixture",
                "Version": "1.2.3",
                "Factory Info": { "Vendor": "Example Vendor", },
                "Compatibility": [
                    { "New": "ABCDEF0123456789ABCDEF0123456789", },
                    { "New": "ABCDEF0123456789ABCDEF0123456789", },
                ],
            }"#,
        )
        .unwrap();
        let identity = inspect_binary(&bundle, std::slice::from_ref(&root)).unwrap();
        assert_eq!(identity.path, fs::canonicalize(bundle).unwrap());
        assert_eq!(identity.binary_path, fs::canonicalize(binary).unwrap());
        assert_eq!(
            identity.compatibility(),
            PluginCompatibility::SupportedVst3X64
        );
        assert_eq!(identity.metadata.vendor.as_deref(), Some("Example Vendor"));
        assert_eq!(identity.metadata.version.as_deref(), Some("1.2.3"));
        assert_eq!(identity.metadata.class_ids.len(), 1);
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
    fn refuses_a_reparse_point_scan_root() {
        let root = temp_root();
        let target = root.join("target");
        fs::create_dir(&target).unwrap();
        let link = root.join("scan-link");
        #[cfg(windows)]
        let link_result = std::os::windows::fs::symlink_dir(&target, &link);
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&target, &link);

        if link_result.is_ok() {
            assert_eq!(scan_directory(&link), Err(ScanError::InvalidRoot));
            fs::remove_dir(&link).unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inspection_rejects_a_reparse_configured_root() {
        let root = temp_root();
        let target = root.join("target");
        let link = root.join("configured-link");
        fs::create_dir(&target).unwrap();
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&target, &link);
        #[cfg(windows)]
        let link_result = std::os::windows::fs::symlink_dir(&target, &link);
        if link_result.is_ok() {
            let candidate = target.join("effect.vst3");
            fs::write(&candidate, pe_x64()).unwrap();
            assert_eq!(
                inspect_binary(&candidate, std::slice::from_ref(&link)),
                Err(InspectionError::OutsideConfiguredRoot)
            );
            fs::remove_dir(&link).unwrap();
        }

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inspection_rejects_a_root_under_a_reparse_parent() {
        let root = temp_root();
        let target = root.join("target");
        let link = root.join("redirected-parent");
        let configured = link.join("allowed");
        fs::create_dir(&target).unwrap();
        fs::create_dir(target.join("allowed")).unwrap();
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&target, &link);
        #[cfg(windows)]
        let link_result = std::os::windows::fs::symlink_dir(&target, &link);

        if link_result.is_ok() {
            let candidate = configured.join("effect.vst3");
            fs::write(&candidate, pe_x64()).unwrap();
            assert_eq!(
                inspect_binary(&candidate, std::slice::from_ref(&configured)),
                Err(InspectionError::OutsideConfiguredRoot)
            );
        }

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn controlled_binary_inspection_propagates_cancel_during_read() {
        let root = temp_root();
        let candidate = root.join("effect.vst3");
        fs::write(&candidate, pe_x64()).unwrap();
        let cancelled = ScanControl::default_deadline();
        cancelled.cancel();
        assert_eq!(
            inspect_binary_with_control(&candidate, std::slice::from_ref(&root), Some(&cancelled)),
            Err(InspectionError::Cancelled)
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
        assert_eq!(asset.verify_for_restore(0), Err(StateError::InvalidVersion));
        assert_eq!(PluginStateAsset::new(1, Vec::new()), Err(StateError::Empty));
        assert_eq!(
            PluginStateAsset::new(0, vec![1]),
            Err(StateError::InvalidVersion)
        );
    }

    #[test]
    fn state_restore_rejects_oversized_files() {
        let root = temp_root();
        let path = root.join("oversized.bin");
        fs::write(&path, vec![0u8; MAX_PLUGIN_STATE_BYTES + 1]).unwrap();
        assert_eq!(
            read_state_asset(&root, &path, 1, &"0".repeat(64)),
            Err(StateFileError::TooLarge)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn state_restore_verification_rejects_directly_constructed_oversized_assets() {
        let bytes = vec![0u8; MAX_PLUGIN_STATE_BYTES + 1];
        let sha256 = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let asset = PluginStateAsset {
            version: 1,
            bytes,
            sha256,
        };
        assert_eq!(asset.verify_for_restore(1), Err(StateError::TooLarge));
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
            metadata: Default::default(),
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
    fn editor_lifecycle_is_independent_of_processing_generation() {
        let mut editor = EditorLifecycle::new(42);
        assert_eq!(editor.state(), EditorState::Closed);
        assert_eq!(editor.processing_generation(), 42);
        assert_eq!(editor.close(), Err(EditorError::AlreadyClosed));

        editor.open().unwrap();
        assert_eq!(editor.open(), Err(EditorError::AlreadyOpen));
        editor.fail().unwrap();
        assert_eq!(editor.state(), EditorState::Failed);
        assert_eq!(editor.processing_generation(), 42);
        assert_eq!(editor.open(), Err(EditorError::RetryRequired));

        editor.retry().unwrap();
        editor.open().unwrap();
        editor.close().unwrap();
        assert_eq!(editor.state(), EditorState::Closed);
        assert_eq!(editor.processing_generation(), 42);
    }

    #[test]
    fn editor_failure_and_retry_are_deliberate_transitions() {
        let mut editor = EditorLifecycle::new(7);
        assert_eq!(editor.fail(), Err(EditorError::NotOpen));
        assert_eq!(editor.retry(), Err(EditorError::NotOpen));
        editor.open().unwrap();
        editor.fail().unwrap();
        editor.close().unwrap();
        assert_eq!(editor.retry(), Err(EditorError::NotOpen));
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
            metadata: Default::default(),
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
    fn worker_executable_rejects_a_reparse_point_ancestor() {
        let root = temp_root();
        let target = root.join("target");
        fs::create_dir(&target).unwrap();
        let executable = target.join("worker.exe");
        fs::write(&executable, b"fixture").unwrap();
        let link = root.join("redirected");
        #[cfg(windows)]
        let link_result = std::os::windows::fs::symlink_dir(&target, &link);
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&target, &link);
        if link_result.is_ok() {
            let redirected_executable = link.join("worker.exe");
            assert!(validate_worker_executable(&redirected_executable)
                .unwrap_err()
                .contains("reparse-point ancestor"));
            fs::remove_dir(&link).unwrap();
        }
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
            metadata: Default::default(),
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
    fn parameter_descriptors_are_bounded_and_normalized() {
        let descriptor = ParameterDescriptor::new(4, "Mix", 0.5, 0.0, 1.0).unwrap();
        let message = WorkerMessage::Parameters {
            descriptors: vec![descriptor.clone()],
        };
        assert_eq!(
            decode_worker_message(&encode_worker_message(&message).unwrap()).unwrap(),
            message
        );
        assert_eq!(
            ParameterDescriptor::new(4, "", 0.5, 0.0, 1.0),
            Err(ParameterDescriptorError::EmptyTitle)
        );
        assert_eq!(
            ParameterDescriptor::new(4, "Mix", 1.1, 0.0, 1.0),
            Err(ParameterDescriptorError::InvalidRange)
        );
        let duplicate = WorkerMessage::Parameters {
            descriptors: vec![descriptor.clone(), descriptor],
        };
        assert_eq!(
            encode_worker_message(&duplicate),
            Err(WorkerMessageError::InvalidParameterDescriptor(
                ParameterDescriptorError::DuplicateId
            ))
        );
    }

    #[test]
    fn queue_constructors_cap_requested_capacity_before_allocation() {
        assert_eq!(
            BoundedFrameQueue::new(usize::MAX).capacity,
            MAX_WORKER_FRAMES
        );
        assert_eq!(
            BoundedParameterQueue::new(usize::MAX).capacity,
            MAX_PARAMETER_EVENTS
        );
    }

    #[test]
    fn worker_messages_round_trip_and_revalidate_audio_payloads() {
        let message = WorkerMessage::Process {
            frame: WorkerFrame::new(7, 100, 2, vec![0.25, -0.25, 0.0, 0.1]).unwrap(),
            parameters: vec![ParameterEvent::new(3, 0.75, 1).unwrap()],
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
    fn worker_parameter_offsets_cannot_escape_the_current_frame() {
        let inline = WorkerMessage::Process {
            frame: WorkerFrame::new(1, 100, 1, vec![0.0; 4]).unwrap(),
            parameters: vec![ParameterEvent::new(1, 0.5, 4).unwrap()],
        };
        assert!(matches!(
            validate_worker_message(&inline),
            Err(WorkerMessageError::InvalidParameter(
                ParameterEventError::OffsetOutOfRange
            ))
        ));
        let shared = WorkerMessage::ProcessShared {
            sequence: 1,
            deadline_tick: 100,
            channels: 1,
            frames: 4,
            parameters: vec![ParameterEvent::new(1, 0.5, 4).unwrap()],
        };
        assert!(matches!(
            validate_worker_message(&shared),
            Err(WorkerMessageError::InvalidParameter(
                ParameterEventError::OffsetOutOfRange
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
        assert_eq!(
            encode_worker_message(&WorkerMessage::Failure {
                code: "x".repeat(MAX_WORKER_FAILURE_CODE_BYTES + 1),
            }),
            Err(WorkerMessageError::InvalidFailureCode)
        );
    }

    #[test]
    fn worker_state_messages_require_bounded_integrity_checked_assets() {
        let asset = PluginStateAsset::new(4, vec![7, 8, 9]).unwrap();
        let message = WorkerMessage::StateRestore {
            asset: asset.clone(),
        };
        assert_eq!(
            decode_worker_message(&encode_worker_message(&message).unwrap()).unwrap(),
            message
        );
        let oversized = PluginStateAsset {
            version: 4,
            bytes: vec![0; MAX_WORKER_STATE_BYTES + 1],
            sha256: "0".repeat(64),
        };
        assert_eq!(
            encode_worker_message(&WorkerMessage::StateRestore { asset: oversized }),
            Err(WorkerMessageError::InvalidState)
        );
        let invalid_version = PluginStateAsset {
            version: 0,
            bytes: vec![1],
            sha256: Sha256::digest([1u8])
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
        };
        assert_eq!(
            encode_worker_message(&WorkerMessage::StateRestore {
                asset: invalid_version,
            }),
            Err(WorkerMessageError::InvalidState)
        );
        let mut corrupt = asset;
        corrupt.sha256 = "0".repeat(64);
        assert_eq!(
            encode_worker_message(&WorkerMessage::State { asset: corrupt }),
            Err(WorkerMessageError::InvalidState)
        );
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
            assert!(encode_worker_message(&message).is_err());
        }
        let failure = WorkerMessage::Failure {
            code: String::new(),
        };
        assert!(encode_worker_message(&failure).is_err());
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
    fn worker_session_retains_dynamic_latency_at_one_sample_rate() {
        let hash = "d".repeat(64);
        let mut session = WorkerSession::new(hash.clone(), 2).unwrap();
        session
            .accept(
                &WorkerMessage::Hello {
                    protocol_version: WORKER_PROTOCOL_VERSION,
                    plugin_sha256: hash,
                    channels: 2,
                },
                0,
            )
            .unwrap();
        session.accept(&WorkerMessage::Ready, 0).unwrap();

        let first = WorkerLatency::new(128, 48_000).unwrap();
        let updated = WorkerLatency::new(256, 48_000).unwrap();
        session.accept(&WorkerMessage::Latency(first), 1).unwrap();
        assert_eq!(session.latency(), Some(first));
        session.accept(&WorkerMessage::Latency(updated), 2).unwrap();
        assert_eq!(session.latency(), Some(updated));
        assert_eq!(
            session.accept(
                &WorkerMessage::Latency(WorkerLatency::new(256, 44_100).unwrap()),
                3,
            ),
            Err(WorkerSessionError::InvalidLatency)
        );
        assert_eq!(session.latency(), Some(updated));
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
    fn shared_audio_region_rejects_a_reparse_point_parent() {
        let layout = SharedAudioLayout::new(1).unwrap();
        let root = temp_root();
        let target = root.join("target");
        fs::create_dir(&target).unwrap();
        let link = root.join("redirected");
        #[cfg(windows)]
        let link_result = std::os::windows::fs::symlink_dir(&target, &link);
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&target, &link);
        if link_result.is_ok() {
            let path = link.join("slot");
            assert!(matches!(
                SharedAudioRegion::create(&path, layout),
                Err(SharedAudioError::InvalidPath)
            ));
            fs::remove_dir(&link).unwrap();
        }
        fs::remove_dir_all(root).unwrap();
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
        let hard_link = std::env::temp_dir().join(format!("{}-alias", stem));
        let _ = fs::remove_file(&hard_link);
        if fs::hard_link(&input_path, &hard_link).is_ok() {
            assert!(matches!(
                SharedAudioTransport::open(&input_path, &hard_link, layout),
                Err(SharedAudioError::AliasedPaths)
            ));
            fs::remove_file(&hard_link).unwrap();
        }
        drop(worker);
        drop(host);
        fs::remove_file(input_path).unwrap();
        fs::remove_file(output_path).unwrap();
    }
}
