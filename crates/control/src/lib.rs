//! Portable control-plane façade for M01.
//!
//! Transport, authorization, and durable storage are deliberately separate
//! follow-up layers. This façade proves that all adapters can share one domain
//! authority and that unsupported audio capabilities are discoverable.

// The discovery document's JSON schemas are large `json!` literals.
#![recursion_limit = "256"]

use audiorouter_domain::{
    format_validation_errors, inspect_routes, node_registry, validate_session, ApiMethodSpec,
    CrashRecoveryTracker, EntityId, EventLog, EventReplayError, FakeRuntime, GraphStore, NodeKind,
    PermissionScope, PortDirection, RecoveryDecision, RecoveryMode, RuntimeError, RuntimeState,
    Session, VirtualBusRegistry, VirtualBusRouteRegistry, API_METHODS,
};
use audiorouter_engine::{
    AudioBlock, AudioTap, AudioTapSet, DecodedAudio, RealtimePluginProcessor, RecorderTapBindings,
    RuntimeGeneration, VirtualBusBridgeSet, VirtualBusBridgeSetError,
};
use audiorouter_protocol::{
    decode_rpc_frame, encode_frame, FrameError, JsonRpcRequest, JsonRpcResponse, RpcMessage,
    MAX_METHOD_NAME_BYTES, MAX_REQUEST_ID_BYTES,
};
use audiorouter_recording::{
    BufferedFlacRecorder, Mp3Recorder, PathPolicyError, RecorderController, RecorderState,
    RecordingChunk, RecordingError, RecordingPathPolicy, RecordingQueue, SegmentedWavRecorder,
    StreamingFlacRecorder, StreamingFlacWriter, WavFormat, WavRecorder, WavWriter,
};
use audiorouter_storage::{
    GraphPlanRecord, RecordingRecord, Storage, StorageError, GRAPH_PLAN_RETENTION_SECONDS,
    MAX_PENDING_PLAN_RECORDS, MAX_RECORDING_ID_BYTES, MAX_RECORDING_LIST_ITEMS,
};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

mod network_log;
pub mod os_transition;
mod siege_round;
mod simple;
mod threaded_recorder;

// Control-plane domains. Each module adds its own `impl ControlPlane` block.
// The globs re-export public items at the crate root and let sibling
// modules reach crate-internal helpers through their `use super::*`.
mod api_output_schema;
mod api_schema;
mod audio_media;
mod audio_service;
mod authorization;
mod catalog;
mod graph;
mod native_application;
mod native_bindings;
mod native_dispatch;
mod native_paths;
mod native_workers;
mod network;
mod persistence;
mod plugins;
mod recorder_workers;
mod recording;
mod recording_library;
mod safety;
mod sessions;
mod status;
#[cfg(test)]
mod test_support;
mod virtual_devices;

use api_output_schema::*;
pub use api_schema::*;
use audio_media::*;
pub use audio_service::*;
pub use authorization::*;
use catalog::*;
use graph::*;
pub use native_application::*;
pub use native_paths::*;
use network::*;
use persistence::*;
pub use recorder_workers::*;
use recording::*;
use sessions::*;
use virtual_devices::*;

use os_transition::{plan_os_transition, OsTransition};

/// Pause between audio service passes while a long handler's work runs on a
/// helper thread (see `ControlPlane::while_servicing_audio`). Matches the
/// backend loop's 1 ms interval, well below a 10 ms WASAPI period.
const AUDIO_SERVICE_PASS_DURING_WORK: std::time::Duration = std::time::Duration::from_millis(1);

const MUTATION_RATE_PER_SECOND: f64 = 20.0;
const MUTATION_BURST: f64 = 40.0;
const MAX_MUTATION_BUCKETS: usize = 256;
const MUTATION_BUCKET_RETENTION: Duration = Duration::from_secs(10 * 60);
const MAX_CONTROL_VALUE_DEPTH: usize = 32;
/// State IDs of plugin states captured automatically (editor close, Stop);
/// only these are replaced by the next capture of the same node.
const AUTOMATIC_PLUGIN_STATE_PREFIX: &str = "plugin-autostate-";
pub const MAX_CONTROL_STRING_BYTES: usize = 4096;
const MAX_CONTROL_VALUE_COUNT: usize = 8192;
const MAX_EVENT_SUBSCRIPTION_ITEMS: usize = 500;
const MAX_SESSION_LIST_ITEMS: usize = 500;
const MAX_GRAPH_HISTORY_ITEMS: usize = 100;
const MAX_REVISION_CURSOR_BYTES: usize = 20;
const MAX_GRAPH_DIFF_ITEMS: usize = 3;
const MAX_GRAPH_AFFECTED_DESTINATIONS: usize = audiorouter_domain::MAX_NODES_PER_SESSION;
const MAX_DEVICE_LIST_ITEMS: usize = 500;
const APPLICATION_CAPTURE_LIVENESS_POLL: Duration = Duration::from_secs(1);
const APPLICATION_CAPTURE_RETRY_MIN: Duration = Duration::from_secs(1);
const APPLICATION_CAPTURE_RETRY_MAX: Duration = Duration::from_secs(5);
const MAX_VIRTUAL_DEVICE_LIST_ITEMS: usize = 500;
const MAX_PROCESSOR_CATALOG_ITEMS: usize = 32;

/// Maximum simultaneously armed/active recorder controllers across sessions.
const MAX_ACTIVE_RECORDERS: usize = audiorouter_engine::MAX_AUDIO_TAPS;
/// Maximum number of bounded queue-drain passes a recorder finalization may
/// perform. A producer that keeps refilling a queue must not make a stop
/// operation loop forever; the caller receives a recoverable finalization
/// error and the recorder remains owned by the worker.
const MAX_RECORDER_FINALIZATION_PASSES: usize = 4096;
/// Two seconds of 128-frame quanta at 48 kHz (REC-08 default queue depth).
const ONE_CLICK_RECORDER_QUEUE_CHUNKS: usize = {
    let chunks = 2 * 48_000 / audiorouter_engine::PROCESSING_QUANTUM_FRAMES;
    if chunks > audiorouter_recording::MAX_RECORDING_QUEUE_CHUNKS {
        audiorouter_recording::MAX_RECORDING_QUEUE_CHUNKS
    } else {
        chunks
    }
};
/// Maximum number of queued recorder chunks drained per native pump and per
/// attached worker. This keeps recording I/O bounded per control request.
const MAX_RECORDER_PUMP_CHUNKS: usize = 4;
const MAX_RESPONSE_BANDS: usize = audiorouter_dsp::PARAMETRIC_EQ_BANDS;
const MAX_RESPONSE_FREQUENCIES: usize = 256;
const MAX_MEMORY_OPERATION_OUTCOMES: usize = 100;
/// Maximum number of distinct plugin scan roots retained for `plugins.list`.
const MAX_PLUGIN_INVENTORY_ROOTS: usize = 64;
const MAX_PLAN_REQUIRED_SCOPES: usize = 1;
const MAX_PLAN_WARNINGS: usize = 1;
const AUDIO_UPLOAD_CHUNK_BYTES: usize = 192 * 1024;
const AUDIO_UPLOAD_TTL: Duration = Duration::from_secs(30 * 60);
const STATE_CATEGORIES: [&str; 22] = [
    "session.created",
    "session.deleted",
    "session.selectionChanged",
    "graph.committed",
    "runtime.crashed",
    "runtime.started",
    "runtime.activated",
    "runtime.stopped",
    "devices.changed",
    "devices.bindingInvalidated",
    "recovery.safeModeCleared",
    "privacy.muteEnabled",
    "privacy.muteDisabled",
    "virtualDevice.changed",
    "virtualBridge.failed",
    "virtualBridge.expired",
    "recorder.changed",
    "recording.metadataChanged",
    "recording.renamed",
    "recording.entryRemoved",
    "recording.recycled",
    "application.captureStateChanged",
];
const APPLICATION_SNAPSHOT_TTL: std::time::Duration = std::time::Duration::from_millis(100);
const VIRTUAL_DEVICE_PLAN_TTL: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Eq, PartialEq)]
pub enum ControlError {
    InvalidRequest(String),
    Audio {
        code: &'static str,
        operation: Option<&'static str>,
        hresult: u32,
        retryable: bool,
        remediation: &'static str,
        message: String,
        /// Exact endpoint IDs the failed operation was opening, reported as
        /// `resourceIds` so clients can name the device (never logged).
        resource_ids: Vec<String>,
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
        operation: error.operation(),
        hresult: error.hresult(),
        retryable: error.is_retryable(),
        remediation: error.remediation(),
        message: error.to_string(),
        resource_ids: Vec::new(),
    }
}

/// Device rates the multi-path engine accepts. Its graph runs at
/// `INTERNAL_SAMPLE_RATE_HZ`; other rates are opened at that rate and
/// resampled by the Windows audio engine (CAP-14, 44.1/96 kHz devices).
fn multi_path_rate_supported(sample_rate_hz: u32) -> bool {
    (8_000..=192_000).contains(&sample_rate_hz)
}

/// An audio failure while opening one exact endpoint, identified so the
/// client can say which device was refused (for example held exclusively).
fn endpoint_audio_control_error(
    error: audiorouter_windows_audio::AudioError,
    endpoint_id: &str,
) -> ControlError {
    match audio_control_error(error) {
        ControlError::Audio {
            code,
            operation,
            hresult,
            retryable,
            remediation,
            message,
            ..
        } => ControlError::Audio {
            code,
            operation,
            hresult,
            retryable,
            remediation,
            message,
            resource_ids: vec![endpoint_id.to_owned()],
        },
        other => other,
    }
}

impl From<audiorouter_domain::StoreError> for ControlError {
    fn from(error: audiorouter_domain::StoreError) -> Self {
        Self::Store(error)
    }
}

/// Device buffer requested for multi-input physical outputs (100 ns units).
pub const MULTI_INPUT_RENDER_HEADROOM_100NS: i64 = 500_000;

/// Service gaps longer than this can empty a minimal shared-mode render
/// buffer (one 10 ms engine period plus margin) and are counted as late.
pub const AUDIO_SERVICE_LATE_GAP: std::time::Duration = std::time::Duration::from_millis(8);

pub struct ControlPlane {
    store: GraphStore,
    /// Opt-in verbose logging window (`diagnostics.setVerbose`). Read by the
    /// transport's logger; never touches audio.
    verbose_diagnostics: audiorouter_protocol::diagnostics::VerboseDiagnostics,
    active_session_id: Option<EntityId>,
    build: String,
    runtimes: HashMap<EntityId, FakeRuntime>,
    recorders: HashMap<EntityId, RecorderController>,
    recorder_workers: HashMap<EntityId, Box<dyn RecorderWorker>>,
    recorder_node_workers: HashMap<EntityId, Box<dyn RecorderWorker>>,
    /// Stable realtime inlet per Recorder node. Playback binds the inlet, so
    /// a route with a Recorder always plays; Record attaches a file worker
    /// behind it at any time. Control thread only (never the audio callback).
    recorder_inlets:
        std::sync::Mutex<HashMap<EntityId, Arc<audiorouter_engine::SwitchableAudioTap>>>,
    recorder_node_states: HashMap<EntityId, RecorderController>,
    recorder_node_sessions: HashMap<EntityId, EntityId>,
    /// One-click recordings: per Recorder node, the automatic split interval
    /// and the frame the current file began.
    recording_splits: HashMap<EntityId, RecordingSplit>,
    recording_sequence: u64,
    recording_maintained_at: Option<std::time::Instant>,
    /// Why a one-click recording stopped saving (first failure per node);
    /// reported once at Stop (REC-08: mark the gap and its cause).
    recorder_node_failures: HashMap<EntityId, String>,
    recording_policy: Option<RecordingPathPolicy>,
    storage: Option<Storage>,
    audio_upload: Option<AudioMediaUpload>,
    next_audio_upload: u64,
    next_audio_media: u64,
    audio_file_sources:
        HashMap<(EntityId, EntityId), std::sync::Arc<audiorouter_engine::AudioFileSource>>,
    test_signal_sources:
        HashMap<(EntityId, EntityId), std::sync::Arc<audiorouter_engine::TestSignalSource>>,
    enrollments: HashMap<String, (ClientRole, bool)>,
    events: EventLog,
    mutation_limiter: MutationRateLimiter,
    operation_outcomes: HashMap<String, Value>,
    operation_names: HashMap<String, String>,
    operation_order: VecDeque<String>,
    idempotency_hashes: HashMap<String, String>,
    application_snapshot: Option<(Instant, Value)>,
    application_capture_runtime: Option<ApplicationCaptureRuntime>,
    plugin_inventories: HashMap<String, Value>,
    /// Live plugin runtime bridges by (session, node), for state capture and
    /// the native editor. Weak: a bridge lives only as long as its graph.
    plugin_bridges: std::sync::Mutex<
        HashMap<
            (EntityId, EntityId),
            std::sync::Weak<audiorouter_plugin_host::PluginRuntimeBridge>,
        >,
    >,
    plugin_inventory_order: VecDeque<String>,
    privacy_muted: bool,
    startup_enabled: bool,
    /// The user's persisted consent for the desktop app to open audio
    /// devices on Play (see `ClientGrant::accepts_device_consent`).
    device_access_allowed: bool,
    recovery_tracker: CrashRecoveryTracker,
    os_suspended_sessions: Vec<EntityId>,
    os_suspended_native_sessions: Vec<EntityId>,
    virtual_buses: VirtualBusRegistry,
    virtual_bus_routes: VirtualBusRouteRegistry,
    virtual_bus_route_revision: u64,
    virtual_bridges: VirtualBusBridgeSet,
    virtual_bus_plans: HashMap<EntityId, VirtualBusPlan>,
    next_virtual_bus_plan: u64,
    startup_plans: HashMap<EntityId, (bool, Instant)>,
    next_startup_plan: u64,
    session_import_plans: HashMap<EntityId, (Session, Instant)>,
    next_session_import_plan: u64,
    active_idempotency_scope: Option<String>,
    /// Authority for nested device preparation in a graph edit. None denotes
    /// a trusted internal call; adapter dispatch always supplies the grant.
    active_device_restart_allowed: Option<bool>,
    endpoint_monitor: Option<audiorouter_windows_audio::EndpointMonitor>,
    /// Changes observed by read-only inventory but not yet handled at a
    /// mutating native lifecycle boundary. Keeping these pending prevents a
    /// `devices.list` call from consuming the invalidation signal before the
    /// audio pump can fail closed.
    pending_endpoint_changes: Vec<audiorouter_windows_audio::EndpointChange>,
    /// Continuity statistics for the backend-owned native audio service.
    audio_service: AudioServiceStats,
    native_endpoint_worker: Option<audiorouter_windows_audio::NativeAudioWorker>,
    native_endpoint_session: Option<EntityId>,
    native_endpoint_worker_secondary: Option<audiorouter_windows_audio::NativeAudioWorker>,
    native_endpoint_session_secondary: Option<EntityId>,
    #[cfg(windows)]
    native_multi_input_worker: Option<audiorouter_windows_audio::NativeMultiInputWorker>,
    #[cfg(windows)]
    native_multi_input_worker_session: Option<EntityId>,
    #[cfg(windows)]
    native_multi_input_worker_generation: Option<u64>,
    /// Runtime generation a live graph update (flag/parameter commit while
    /// playing) was applied to. The running worker keeps its prepared
    /// generation, so it serves both until it is replaced or detached.
    #[cfg(windows)]
    native_multi_input_applied_generation: Option<u64>,
    #[cfg(windows)]
    multi_input_application_sources: Vec<MultiInputApplicationSource>,
    /// Stereo device nodes the prepared multi-input worker reads from a
    /// mono endpoint (see `native_paths_session`).
    #[cfg(windows)]
    native_multi_input_mono_nodes: Vec<EntityId>,
    /// Paces `network.jsonl` summaries of playing network nodes.
    network_log: network_log::Sampler,
    /// Stats.cc client for Ducks following the Siege round (started on demand).
    siege_round_feed: siege_round::SiegeRoundFeed,
    native_endpoint_taps: Option<AudioTapSet>,
    native_endpoint_taps_secondary: Option<AudioTapSet>,
    native_endpoint_rejections: u64,
    #[cfg(windows)]
    native_output_fanout: Option<audiorouter_windows_audio::WasapiOutputFanout>,
    #[cfg(windows)]
    native_output_fanout_session: Option<EntityId>,
    #[cfg(windows)]
    native_output_fanout_generation: Option<u64>,
    #[cfg(windows)]
    native_capture_sink_bindings:
        HashMap<EntityId, audiorouter_windows_audio::NativeBridgeCaptureSinkBinding>,
    #[cfg(windows)]
    native_render_source_bindings:
        HashMap<EntityId, audiorouter_windows_audio::NativeBridgeRenderSourceBinding>,
    #[cfg(windows)]
    native_duplex_bindings: HashMap<EntityId, audiorouter_windows_audio::NativeBridgeDuplexBinding>,
    #[cfg(windows)]
    native_duplex_worker: Option<audiorouter_windows_audio::NativeBridgeDuplexWorker>,
    #[cfg(windows)]
    native_duplex_worker_session: Option<EntityId>,
    #[cfg(windows)]
    native_duplex_worker_generation: Option<u64>,
    #[cfg(windows)]
    native_render_source_worker: Option<audiorouter_windows_audio::NativeBridgeInputWorker>,
    #[cfg(windows)]
    native_render_source_worker_session: Option<EntityId>,
    #[cfg(windows)]
    native_render_source_worker_generation: Option<u64>,
    #[cfg(windows)]
    native_render_source_taps: Option<AudioTapSet>,
    #[cfg(windows)]
    managed_software_devices: audiorouter_windows_audio::ManagedSoftwareDeviceInventory,
}

impl Default for ControlPlane {
    fn default() -> Self {
        Self::new("dev")
    }
}

/// A short, bounded human-readable reason for a control error.
fn control_error_message(error: &ControlError) -> String {
    let text = match error {
        ControlError::InvalidRequest(message) => message.clone(),
        ControlError::Audio { message, .. } => message.clone(),
        other => format!("{other:?}"),
    };
    text.chars().take(240).collect()
}

impl ControlPlane {
    pub fn new(build: impl Into<String>) -> Self {
        Self {
            store: GraphStore::default(),
            verbose_diagnostics: Default::default(),
            active_session_id: None,
            build: build.into(),
            runtimes: HashMap::new(),
            recorders: HashMap::new(),
            recorder_workers: HashMap::new(),
            recorder_node_workers: HashMap::new(),
            recorder_inlets: Default::default(),
            recorder_node_states: HashMap::new(),
            recorder_node_sessions: HashMap::new(),
            recording_splits: HashMap::new(),
            recording_sequence: 0,
            recording_maintained_at: None,
            recorder_node_failures: HashMap::new(),
            recording_policy: None,
            storage: None,
            audio_upload: None,
            next_audio_upload: 1,
            next_audio_media: 1,
            audio_file_sources: HashMap::new(),
            test_signal_sources: HashMap::new(),
            enrollments: HashMap::new(),
            events: EventLog::new(1),
            mutation_limiter: MutationRateLimiter::default(),
            operation_outcomes: HashMap::new(),
            operation_names: HashMap::new(),
            operation_order: VecDeque::new(),
            idempotency_hashes: HashMap::new(),
            application_snapshot: None,
            application_capture_runtime: None,
            plugin_inventories: HashMap::new(),
            plugin_bridges: Default::default(),
            plugin_inventory_order: VecDeque::new(),
            privacy_muted: false,
            startup_enabled: false,
            device_access_allowed: false,
            recovery_tracker: CrashRecoveryTracker::default(),
            os_suspended_sessions: Vec::new(),
            os_suspended_native_sessions: Vec::new(),
            virtual_buses: VirtualBusRegistry::default(),
            virtual_bus_routes: VirtualBusRouteRegistry::default(),
            virtual_bus_route_revision: 0,
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
            active_device_restart_allowed: None,
            endpoint_monitor: None,
            pending_endpoint_changes: Vec::new(),
            audio_service: AudioServiceStats::default(),
            native_endpoint_worker: None,
            native_endpoint_session: None,
            native_endpoint_worker_secondary: None,
            native_endpoint_session_secondary: None,
            #[cfg(windows)]
            native_multi_input_worker: None,
            #[cfg(windows)]
            native_multi_input_worker_session: None,
            #[cfg(windows)]
            native_multi_input_worker_generation: None,
            native_multi_input_applied_generation: None,
            #[cfg(windows)]
            multi_input_application_sources: Vec::new(),
            #[cfg(windows)]
            native_multi_input_mono_nodes: Vec::new(),
            network_log: network_log::Sampler::default(),
            siege_round_feed: Default::default(),
            native_endpoint_taps: None,
            native_endpoint_taps_secondary: None,
            native_endpoint_rejections: 0,
            #[cfg(windows)]
            native_output_fanout: None,
            #[cfg(windows)]
            native_output_fanout_session: None,
            #[cfg(windows)]
            native_output_fanout_generation: None,
            #[cfg(windows)]
            native_capture_sink_bindings: HashMap::new(),
            #[cfg(windows)]
            native_render_source_bindings: HashMap::new(),
            #[cfg(windows)]
            native_duplex_bindings: HashMap::new(),
            #[cfg(windows)]
            native_duplex_worker: None,
            #[cfg(windows)]
            native_duplex_worker_session: None,
            #[cfg(windows)]
            native_duplex_worker_generation: None,
            #[cfg(windows)]
            native_render_source_worker: None,
            #[cfg(windows)]
            native_render_source_worker_session: None,
            #[cfg(windows)]
            native_render_source_worker_generation: None,
            #[cfg(windows)]
            native_render_source_taps: None,
            #[cfg(windows)]
            managed_software_devices:
                audiorouter_windows_audio::ManagedSoftwareDeviceInventory::default(),
        }
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
                    "diagnostics.getVerbose" => self.dispatch_verbose_diagnostics(None),
                    "diagnostics.setVerbose" => self.dispatch_verbose_diagnostics(Some(
                        request.params.clone().unwrap_or(Value::Null),
                    )),
                    "system.osTransition" => self.dispatch_os_transition(request.params),
                    "system.diagnostics" => {
                        let (recent_recovery_crashes, recovery_safe_mode) =
                            self.recovery_status()?;
                        let (audio, audio_reason) = self.audio_status();
                        let mut node_telemetry = self
                            .native_node_telemetry()
                            .as_array()
                            .cloned()
                            .unwrap_or_default();
                        node_telemetry.extend(
                            self.native_multi_input_node_telemetry()
                                .as_array()
                                .cloned()
                                .unwrap_or_default(),
                        );
                        Ok(json!({
                        "build": self.build,
                        "backend": "control-plane",
                        "storage": if self.storage.is_some() { "sqlite" } else { "memory" },
                        "audio": {
                            "state": audio,
                            "reason": audio_reason
                        },
                        "nativeAdapter": self.native_adapter_state(),
                        "nativeAdapterKind": self.native_adapter_kind(),
                        "nativeSessionId": self.native_session_id().map(EntityId::as_str),
                        "schedulerTelemetry": self.native_scheduler_telemetry(),
                        "nodeTelemetry": node_telemetry,
                        "applicationCaptureStates": self.application_capture_states(),
                        "gameRound": self.siege_round_feed.status_json(),
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
                    "system.quit" => self.dispatch_system_quit(request.params),
                    "clients.list" => self.dispatch_clients_list(),
                    "clients.authorize" => self.dispatch_client_authorize(request.params),
                    "clients.revoke" => self.dispatch_client_revoke(request.params),
                    "operations.get" => self.dispatch_operation_get(request.params),
                    "operations.cancel" => self.dispatch_operation_cancel(request.params),
                    "recordings.list" => self.dispatch_recordings_list(request.params),
                    "audioMedia.beginUpload" => {
                        self.dispatch_audio_media_begin_upload(request.params)
                    }
                    "audioMedia.uploadChunk" => {
                        self.dispatch_audio_media_upload_chunk(request.params)
                    }
                    "audioMedia.finishUpload" => {
                        self.dispatch_audio_media_finish_upload(request.params)
                    }
                    "audioMedia.importTemporaryRecording" => {
                        self.dispatch_audio_media_import_temporary_recording(request.params)
                    }
                    "audioMedia.delete" => self.dispatch_audio_media_delete(request.params),
                    "audioSources.transport" => {
                        self.dispatch_audio_source_transport(request.params)
                    }
                    "timeShift.transport" => self.dispatch_time_shift_transport(request.params),
                    "meters.reset" => self.dispatch_meter_reset(request.params),
                    "recorders.list" => self.dispatch_recorders_list(request.params),
                    "recorders.create" => self.dispatch_recorder_create(request.params),
                    "recorders.arm" | "recorders.start" | "recorders.pause"
                    | "recorders.resume" | "recorders.split" | "recorders.stop" => {
                        self.dispatch_recorder(request.method.as_str(), request.params)
                    }
                    "recorders.startRecording" | "recorders.stopRecording" => {
                        self.dispatch_one_click_recording(request.method.as_str(), request.params)
                    }
                    method
                        if simple::SIMPLE_METHODS
                            .iter()
                            .any(|(name, _)| *name == method) =>
                    {
                        self.dispatch_simple(method, request.params)
                    }
                    "devices.getAccess" => Ok(json!({ "allowed": self.device_access_allowed })),
                    "devices.setAccess" => self.dispatch_device_access_set(request.params),
                    "recordings.getRoot" => self.dispatch_recording_root_get(),
                    "recordings.setRoot" => self.dispatch_recording_root_set(request.params),
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
                        "enabled": self.startup_enabled,
                        "registration": "unavailable",
                        "reason": "native sign-in registration is owned by the desktop shell"
                    })),
                    "startup.plan" => self.dispatch_startup_plan(request.params),
                    "startup.apply" => self.dispatch_startup_apply(request.params),
                    "devices.list" => self.dispatch_devices_list(request.params),
                    "nativeEndpoints.prepare" => {
                        self.dispatch_native_endpoints_prepare(request.params)
                    }
                    "nativeOutputs.prepare" => self.dispatch_native_outputs_prepare(request.params),
                    "nativeMultiInputs.prepare" => {
                        self.dispatch_native_multi_inputs_prepare(request.params)
                    }
                    "nativePaths.prepare" => self.dispatch_native_paths_prepare(request.params),
                    "nativeBridges.prepare" => self.dispatch_native_bridges_prepare(request.params),
                    "nativeBridges.detach" => self.dispatch_native_bridges_detach(request.params),
                    "nativeBridges.heartbeat" => {
                        self.dispatch_native_bridges_heartbeat(request.params)
                    }
                    "nativeEndpoints.rebind" => {
                        self.dispatch_native_endpoints_rebind(request.params)
                    }
                    "nativeEndpoints.detach" => {
                        self.dispatch_native_endpoints_detach(request.params)
                    }
                    "nativeApplications.prepare" => {
                        self.dispatch_native_applications_prepare(request.params)
                    }
                    "nativeDuplex.detach" => self.dispatch_native_duplex_detach(request.params),
                    "nativeEndpoints.pump" => self.dispatch_native_endpoints_pump(request.params),
                    "nativeDuplex.pump" => self.dispatch_native_duplex_pump(request.params),
                    "nativeRenderSources.pump" => {
                        self.dispatch_native_render_source_pump(request.params)
                    }
                    "nativeMultiInputs.pump" => {
                        self.dispatch_native_multi_input_pump(request.params)
                    }
                    "nativeMultiInputs.bindBranches" => {
                        self.dispatch_native_multi_input_bind_branches(request.params)
                    }
                    "plugins.scan" => self.dispatch_plugins_scan(request.params),
                    "plugins.list" => self.dispatch_plugins_list(request.params),
                    "plugins.inventory" => {
                        Ok(json!({ "inventories": self.remembered_plugin_inventories() }))
                    }
                    "plugins.retry" => self.dispatch_plugins_retry(request.params),
                    "plugins.inspect" => self.dispatch_plugins_inspect(request.params),
                    "plugins.parameters" => self.dispatch_plugins_parameters(request.params),
                    "plugins.saveState" => self.dispatch_plugins_save_state(request.params),
                    "plugins.openEditor" => self.dispatch_plugins_editor(request.params, true),
                    "plugins.closeEditor" => self.dispatch_plugins_editor(request.params, false),
                    "virtualDevices.list" => self.dispatch_virtual_devices_list(request.params),
                    "virtualDevices.plan" => self.dispatch_virtual_devices_plan(request.params),
                    "virtualDevices.apply" => self.dispatch_virtual_devices_apply(request.params),
                    "virtualDevices.provision" => {
                        self.dispatch_virtual_devices_provision(request.params)
                    }
                    "virtualDevices.remove" => self.dispatch_virtual_devices_remove(request.params),
                    "virtualRoutes.list" => self.dispatch_virtual_routes_list(),
                    "virtualRoutes.replace" => self.dispatch_virtual_routes_replace(request.params),
                    "apps.list" | "applications.list" => self.dispatch_apps_list(),
                    "nodes.types" => Ok(self.describe()["nodeTypes"].clone()),
                    "nodes.describe" => Ok(self.describe()["nodeTypes"].clone()),
                    "presets.list" => Ok(self.describe()["presets"].clone()),
                    "processors.list" => Ok(self.describe()["processors"].clone()),
                    "processors.response" => self.dispatch_processors_response(request.params),
                    "sessions.get" => self.dispatch_session_get(request.params),
                    "sessions.export" => self.dispatch_session_export(request.params),
                    "sessions.exportFile" => self.dispatch_session_export_file(request.params),
                    "sessions.importFile" => self.dispatch_session_import_file(request.params),
                    "sessions.importPlan" => self.dispatch_session_import_plan(request.params),
                    "sessions.importCommit" => self.dispatch_session_import_commit(request.params),
                    "sessions.list" => self.dispatch_sessions_list(request.params),
                    "sessions.active.get" => Ok(self.dispatch_active_session_get()),
                    "sessions.active.set" => self.dispatch_active_session_set(request.params),
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
        if mutating {
            self.refresh_siege_round_feed();
        }
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
        let consented = spec.permission == PermissionScope::DeviceAdministration
            && grant.accepts_device_consent()
            && self.device_access_allowed;
        // Only the desktop window may give or withdraw that consent.
        let consent_method_refused =
            spec.name == "devices.setAccess" && !grant.accepts_device_consent();
        if (!grant.allows(spec.permission) && !consented) || consent_method_refused {
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
        if matches!(request.method.as_str(), "session.start" | "sessions.start")
            && request
                .params
                .as_ref()
                .is_some_and(|params| params.get("candidate").is_some())
            && !grant.allows(PermissionScope::GraphWrite)
        {
            let mut response = JsonRpcResponse::failure(
                id,
                -32001,
                "permission denied: GraphWrite is required to preview an edited route",
            );
            if let Some(error) = response.error.as_mut() {
                error.data = Some(application_error_data("permissionDenied"));
            }
            return response;
        }
        // Native endpoint pumping is a scheduler tick, not a user mutation;
        // applying the ordinary 20/sec mutation bucket would stop a running
        // stream after its initial burst. It remains permission-gated and
        // generation-bound, but is intentionally outside mutation throttling.
        if rate_limit_method(&request.method) {
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
        let previous_restart =
            self.active_device_restart_allowed
                .replace(caller_can_restart_devices(
                    grant,
                    self.device_access_allowed,
                ));
        let response = self.dispatch(request);
        self.active_device_restart_allowed = previous_restart;
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
            operation,
            resource_ids,
            ..
        } => Some((
            *hresult,
            *retryable,
            *remediation,
            *operation,
            resource_ids.clone(),
        )),
        _ => None,
    };
    let mut response = JsonRpcResponse::failure(id, -32000, message);
    if let Some(error) = response.error.as_mut() {
        let mut data = application_error_data(code);
        if let Some((hresult, retryable, remediation, operation, resource_ids)) = audio_details {
            data["hresult"] = json!(hresult);
            data["retryable"] = json!(retryable);
            data["remediation"] = json!(remediation);
            data["operation"] = json!(operation);
            data["resourceIds"] = json!(resource_ids);
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
        "invalidRoot" | "tooManyCandidates" | "tooManyDirectories" | "cancelled" => (
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
    use crate::test_support::*;

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
    fn endpoint_audio_errors_name_the_endpoint_being_opened() {
        let plain = audio_control_error(audiorouter_windows_audio::AudioError::InvalidUtf16);
        let named = endpoint_audio_control_error(
            audiorouter_windows_audio::AudioError::InvalidUtf16,
            "{0.0.0.00000000}.{cable-input}",
        );
        let data = |error| {
            application_error_response(Some(json!(1)), error)
                .error
                .unwrap()
                .data
                .unwrap()
        };
        let (plain, named) = (data(plain), data(named));
        assert_eq!(plain["resourceIds"], json!([]));
        assert_eq!(
            named["resourceIds"],
            json!(["{0.0.0.00000000}.{cable-input}"])
        );
        assert_eq!(named["code"], plain["code"]);
        assert_eq!(named["hresult"], plain["hresult"]);
    }

    #[test]
    fn audio_error_response_preserves_hresult_and_contention_guidance() {
        let response = application_error_response(
            Some(json!(1)),
            ControlError::Audio {
                code: "deviceInUse",
                operation: Some("IAudioClient::Initialize(render)"),
                hresult: 0x8889_000A,
                retryable: true,
                remediation:
                    "identify the owning stream, select another endpoint, or close it and retry",
                message: "audio endpoint is busy".into(),
                resource_ids: vec!["{0.0.0.00000000}.{busy-render}".into()],
            },
        );
        let error = response.error.unwrap();
        assert_eq!(error.message, "audio endpoint is busy");
        let data = error.data.unwrap();
        assert_eq!(data["code"], "deviceInUse");
        assert_eq!(
            data["resourceIds"],
            json!(["{0.0.0.00000000}.{busy-render}"])
        );
        assert_eq!(data["hresult"], 0x8889_000A_u32);
        assert_eq!(data["operation"], "IAudioClient::Initialize(render)");
        assert_eq!(data["retryable"], true);
        assert_eq!(
            data["remediation"],
            "identify the owning stream, select another endpoint, or close it and retry"
        );
    }
}
