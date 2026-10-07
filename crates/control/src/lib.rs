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

use os_transition::{plan_os_transition, OsTransition};
mod native_paths;
pub use native_paths::*;
mod audio_service;
pub use audio_service::*;
mod native_application;
pub use native_application::*;
mod native_bindings;
mod native_dispatch;
mod native_workers;
mod persistence;
use persistence::*;
mod virtual_devices;
use virtual_devices::*;
mod authorization;
pub use authorization::*;
mod recording;
use recording::*;
mod recording_library;
mod sessions;
use sessions::*;
mod graph;
use graph::*;
mod plugins;
#[cfg(test)]
use plugins::*;
mod catalog;
use catalog::*;
mod network;
mod safety;
mod status;
use network::*;
mod audio_media;
use audio_media::*;
mod recorder_workers;
pub use recorder_workers::*;
mod api_schema;
pub use api_schema::*;
mod api_output_schema;
use api_output_schema::*;

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
    use audiorouter_domain::{Edge, Node, NodeKind, Port, PortDirection};
    use audiorouter_engine::{RuntimeGeneration, RuntimeGraph, RuntimeProcessor};

    #[test]
    fn long_handler_work_keeps_audio_service_passes_running() {
        let mut passes = 0_u32;
        let value = run_while_servicing(
            || {
                std::thread::sleep(std::time::Duration::from_millis(40));
                7
            },
            || passes += 1,
        );
        assert_eq!(value, 7);
        // 40 ms of work at a 1 ms cadence; a loaded machine still manages a few.
        assert!(passes >= 5, "only {passes} service passes during the work");
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_while_servicing(|| -> u8 { panic!("decoder failed") }, || ())
        }));
        assert!(panicked.is_err());
    }

    fn feedback_fixture() -> Session {
        let port = |name: &str, direction| Port {
            name: name.into(),
            direction,
            channels: 2,
        };
        let node = |id: &str, kind, endpoint: &str| Node {
            id: EntityId::new(id),
            kind,
            type_version: 1,
            name: id.into(),
            enabled: true,
            bypass: false,
            parameters: if matches!(kind, NodeKind::PhysicalInput | NodeKind::PhysicalOutput) {
                serde_json::from_value(json!({"endpointId": endpoint})).unwrap()
            } else {
                Default::default()
            },
            ports: match kind {
                NodeKind::PhysicalInput => vec![port("out", PortDirection::Output)],
                NodeKind::PhysicalOutput => vec![port("in", PortDirection::Input)],
                _ => vec![
                    port("in", PortDirection::Input),
                    port("out", PortDirection::Output),
                ],
            },
        };
        Session {
            id: EntityId::new("feedback"),
            name: "feedback".into(),
            schema_version: 1,
            revision: 0,
            nodes: vec![
                node("game", NodeKind::PhysicalInput, "cable-b-output"),
                node("eq", NodeKind::Gain, ""),
                node("mix", NodeKind::Mixer, ""),
                node("recording", NodeKind::PhysicalOutput, "cable-b-input"),
            ],
            edges: [("game", "eq"), ("eq", "mix"), ("mix", "recording")]
                .into_iter()
                .enumerate()
                .map(|(index, (source, destination))| Edge {
                    id: EntityId::new(format!("e{index}")),
                    source_node: EntityId::new(source),
                    source_port: "out".into(),
                    destination_node: EntityId::new(destination),
                    destination_port: "in".into(),
                    matrix: vec![1.0, 0.0, 0.0, 1.0],
                    enabled: true,
                })
                .collect(),
        }
    }

    #[test]
    fn endpoint_feedback_names_the_return_and_accepts_a_separate_recording_cable() {
        let mut session = feedback_fixture();
        let returns = vec![("cable-b-input".into(), "cable-b-output".into())];
        for bypass in [false, true] {
            session.nodes[1].bypass = bypass;
            let error = reject_endpoint_feedback(&session, &returns).unwrap_err();
            let message = control_error_message(&error);
            assert!(
                message.contains("game")
                    && message.contains("recording")
                    && message.contains("different output cable")
            );
        }
        session.nodes[3]
            .parameters
            .insert("endpointId".into(), json!("cable-c-input"));
        reject_endpoint_feedback(&session, &returns).unwrap();
        session.nodes[3]
            .parameters
            .insert("endpointId".into(), json!("cable-b-input"));
        session.edges[1].enabled = false;
        reject_endpoint_feedback(&session, &returns).unwrap();
    }

    #[test]
    fn surround_loopback_input_cannot_play_back_into_its_own_device() {
        let mut session = feedback_fixture();
        // A 7.1 playback device captured by loopback and rendered for headphones.
        session.nodes[0]
            .parameters
            .insert("endpointId".into(), json!("cable-b-input"));
        session.nodes[0]
            .parameters
            .insert("spatialMode".into(), json!("headphones"));
        assert!(reject_endpoint_feedback(&session, &[]).is_err());
        session.nodes[3]
            .parameters
            .insert("endpointId".into(), json!("headphones"));
        reject_endpoint_feedback(&session, &[]).unwrap();
        // An ordinary input naming the same ID is a recording device, not a loop.
        session.nodes[0]
            .parameters
            .insert("spatialMode".into(), json!("off"));
        session.nodes[3]
            .parameters
            .insert("endpointId".into(), json!("cable-b-input"));
        reject_endpoint_feedback(&session, &[]).unwrap();
    }

    #[test]
    fn automatic_route_restart_requires_device_and_lifecycle_authority() {
        assert!(!caller_can_restart_devices(
            &ClientGrant::with_scopes([
                PermissionScope::GraphWrite,
                PermissionScope::SessionControl,
            ]),
            true
        ));
        assert!(!caller_can_restart_devices(
            &ClientGrant::with_scopes([PermissionScope::DeviceAdministration,]),
            false
        ));
        assert!(caller_can_restart_devices(
            &ClientGrant::with_scopes([
                PermissionScope::DeviceAdministration,
                PermissionScope::SessionControl,
            ]),
            false
        ));
        let desktop = ClientGrant::for_desktop_shell();
        assert!(!caller_can_restart_devices(&desktop, false));
        assert!(caller_can_restart_devices(&desktop, true));
        let mut plane = ControlPlane {
            active_device_restart_allowed: Some(false),
            ..Default::default()
        };
        let response = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "system.diagnostics".into(),
                params: None,
            },
            &ClientGrant::with_scopes([PermissionScope::Read]),
        );
        assert!(response.error.is_none(), "{:?}", response.error);
        assert_eq!(plane.active_device_restart_allowed, Some(false));
    }

    #[test]
    fn feedback_selection_can_be_saved_only_after_review_but_not_prepared() {
        let mut candidate = feedback_fixture();
        candidate.nodes[0].kind = NodeKind::EndpointLoopback;
        candidate.nodes[0]
            .parameters
            .insert("endpointId".into(), json!("same-render-endpoint"));
        candidate.nodes[3]
            .parameters
            .insert("endpointId".into(), json!("same-render-endpoint"));
        let mut original = candidate.clone();
        original.edges[2].enabled = false;
        let mut plane = ControlPlane::new("feedback-review");
        plane.insert_session(original).unwrap();
        let request = |method: &str, params| JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(params),
        };
        let result = plane
            .dispatch(request(
                "graph.plan",
                json!({"sessionId":"feedback","baseRevision":0,"candidate":candidate}),
            ))
            .result
            .unwrap();
        let warning = result["warnings"][0].as_str().unwrap();
        assert!(
            warning.contains("game")
                && warning.contains("recording")
                && warning.contains("playback is blocked")
        );
        let mut commit =
            json!({"planId":result["planId"],"baseRevision":0,"idempotencyKey":"review-feedback"});
        assert!(plane
            .dispatch(request("graph.commit", commit.clone()))
            .error
            .unwrap()
            .message
            .contains("acknowledge"));
        commit["acknowledgments"] = result["warnings"].clone();
        let saved = plane.dispatch(request("graph.commit", commit));
        assert!(saved.error.is_none(), "{saved:?}");
        assert_eq!(
            plane
                .get_session(&EntityId::new("feedback"))
                .unwrap()
                .revision,
            1
        );
        assert!(plane
            .validate_endpoint_feedback(plane.get_session(&EntityId::new("feedback")).unwrap())
            .is_err());
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "requires installed VB-Cable B; reads endpoint metadata only"]
    fn installed_cable_feedback_is_rejected_before_opening_audio() {
        let endpoints =
            audiorouter_windows_audio::enumerate_active_endpoint_display_info().unwrap();
        let cable = |direction| {
            endpoints
                .iter()
                .find(|endpoint| {
                    endpoint.direction == direction
                        && audiorouter_windows_audio::known_virtual_cable_key(
                            &endpoint.device_description,
                            &endpoint.driver_inf_section,
                        )
                        .as_deref()
                            == Some("b")
                })
                .expect("VB-Cable B endpoint")
        };
        let mut candidate = feedback_fixture();
        candidate.nodes[0].parameters.insert(
            "endpointId".into(),
            json!(cable(audiorouter_windows_audio::EndpointDirection::Capture).id),
        );
        candidate.nodes[3].parameters.insert(
            "endpointId".into(),
            json!(cable(audiorouter_windows_audio::EndpointDirection::Render).id),
        );
        let plane = ControlPlane::default();
        assert!(
            control_error_message(&plane.validate_endpoint_feedback(&candidate).unwrap_err())
                .contains("Audio feedback loop")
        );
    }

    #[test]
    fn live_source_bypass_retains_shape_and_silences_every_output() {
        let mut session = feedback_fixture();
        session.nodes[3]
            .parameters
            .insert("endpointId".into(), json!("other-output"));
        let mut direct = session.nodes[3].clone();
        direct.id = EntityId::new("direct");
        session.nodes.push(direct);
        let mut edge = session.edges[1].clone();
        edge.id = EntityId::new("direct-branch");
        edge.destination_node = EntityId::new("direct");
        session.edges.push(edge);
        let inputs = vec![session.nodes[0].id.clone()];
        let outputs = vec![session.nodes[3].id.clone(), EntityId::new("direct")];
        for bypass in [true, false, true, false] {
            let mut candidate = session.clone();
            candidate.nodes[0].bypass = bypass;
            normalize_live_path_flags(&mut candidate, &inputs, &outputs);
            let set = audiorouter_engine::compile_native_paths_with_plugins_and_audio(
                &candidate,
                RuntimeGeneration::new(1),
                &Default::default(),
                &Default::default(),
            )
            .unwrap();
            assert_eq!(set.input_node_ids(), inputs);
            assert_eq!(set.output_node_ids(), outputs);
            assert!(candidate.nodes[0].enabled && !candidate.nodes[0].bypass);
            assert_eq!(
                candidate.edges[0].matrix,
                if bypass {
                    vec![0.0; 4]
                } else {
                    vec![1.0, 0.0, 0.0, 1.0]
                }
            );
            let frames = audiorouter_engine::PROCESSING_QUANTUM_FRAMES;
            let mut runtime =
                audiorouter_engine::RealtimeMixerFanout::from_paths(set, 4, &[2], frames).unwrap();
            let mut block = audiorouter_engine::AudioBlock::new(2, frames).unwrap();
            for channel in 0..2 {
                block.channel_mut(channel).unwrap().fill(0.5);
            }
            let rings = (0..2)
                .map(|_| audiorouter_engine::AudioBlockRing::new(4, 2, frames).unwrap())
                .collect::<Vec<_>>();
            runtime
                .try_submit_input(0, RuntimeGeneration::new(1), &block)
                .unwrap();
            assert_eq!(
                runtime
                    .process_once(&rings.iter().collect::<Vec<_>>())
                    .unwrap(),
                2
            );
            for ring in &rings {
                let output = ring.try_receive().unwrap();
                assert!(output
                    .channel(0)
                    .unwrap()
                    .iter()
                    .all(|sample| *sample == if bypass { 0.0 } else { 0.5 }));
            }
        }
    }

    #[test]
    fn plugin_worker_resolves_from_the_packaged_resource_directory() {
        let root = std::env::temp_dir().join(format!(
            "audiorouter-worker-resource-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let resources = root.join("resources");
        std::fs::create_dir_all(&resources).unwrap();
        let shell = root.join(if cfg!(windows) {
            "audiorouter-shell.exe"
        } else {
            "audiorouter-shell"
        });
        let worker_name = if cfg!(windows) {
            "audiorouter-plugin-worker.exe"
        } else {
            "audiorouter-plugin-worker"
        };
        let worker = resources.join(worker_name);
        std::fs::write(&worker, b"test executable placeholder").unwrap();
        assert_eq!(packaged_plugin_worker_path(&shell), Some(worker.clone()));
        let adjacent = root.join(worker_name);
        std::fs::write(&adjacent, b"adjacent executable placeholder").unwrap();
        assert_eq!(packaged_plugin_worker_path(&shell), Some(adjacent));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn missing_plugin_worker_error_names_search_paths_and_repair_action() {
        let root = std::env::temp_dir().join("audiorouter-worker-error");
        let shell = root.join(if cfg!(windows) {
            "audiorouter-shell.exe"
        } else {
            "audiorouter-shell"
        });
        let message = plugin_worker_unavailable_message(Some(&shell));
        let worker = if cfg!(windows) {
            "audiorouter-plugin-worker.exe"
        } else {
            "audiorouter-plugin-worker"
        };
        assert!(message.contains(&root.join(worker).display().to_string()));
        assert!(message.contains(&root.join("resources").join(worker).display().to_string()));
        assert!(message.contains("rebuild the complete AudioRouter package"));
        assert!(message.contains("AUDIOROUTER_PLUGIN_WORKER_PATH"));
    }

    fn encode_test_base64(bytes: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut output = String::new();
        for chunk in bytes.chunks(3) {
            let a = chunk[0];
            let b = *chunk.get(1).unwrap_or(&0);
            let c = *chunk.get(2).unwrap_or(&0);
            output.push(TABLE[(a >> 2) as usize] as char);
            output.push(TABLE[(((a & 3) << 4) | (b >> 4)) as usize] as char);
            output.push(if chunk.len() > 1 {
                TABLE[(((b & 15) << 2) | (c >> 6)) as usize] as char
            } else {
                '='
            });
            output.push(if chunk.len() > 2 {
                TABLE[(c & 63) as usize] as char
            } else {
                '='
            });
        }
        output
    }

    fn tiny_test_wav() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&38u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&8_000u32.to_le_bytes());
        bytes.extend_from_slice(&16_000u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&4096i16.to_le_bytes());
        bytes
    }

    #[derive(Default)]
    struct CountingTap(std::sync::atomic::AtomicUsize);

    impl AudioTap for CountingTap {
        fn on_processed_block(&self, _start_frame: u64, _block: &AudioBlock) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    #[test]
    fn recorder_branch_plays_without_a_file_and_records_once_one_is_attached() {
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
        let node_id = EntityId::new("recorder-node");
        let mut plane = ControlPlane::default();
        plane.insert_session(graph).unwrap();
        // Play binds the branch before anyone pressed Record: no error, and
        // the audio has nowhere to go yet.
        let bound = plane
            .recorder_tap_set_for_node(&session_id, &node_id)
            .expect("recorder branch binds without a file");
        let block = AudioBlock::new(1, 128).unwrap();
        bound.on_processed_block(0, &block);
        // Record attaches a file worker behind the same, already bound tap.
        let counter = Arc::new(CountingTap::default());
        plane
            .attach_recorder_worker_to_node(
                &session_id,
                node_id.clone(),
                Box::new(SuccessfulTapRecorderWorker {
                    tap: counter.clone(),
                }),
            )
            .unwrap();
        bound.on_processed_block(128, &block);
        bound.on_processed_block(256, &block);
        assert_eq!(
            counter.0.load(std::sync::atomic::Ordering::Relaxed),
            2,
            "blocks after Record reach the file"
        );
        // The endpoint-scheduler binding path also succeeds.
        assert!(plane
            .recorder_tap_bindings(&session_id, RuntimeGeneration::new(3))
            .is_ok());
    }

    #[test]
    fn recording_folder_is_chosen_in_the_app_and_unlocks_one_click_recording() {
        let base = std::env::temp_dir().join(format!(
            "audiorouter-root-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut graph = session();
        graph.nodes.push(Node {
            id: EntityId::new("rec"),
            kind: NodeKind::Recorder,
            type_version: 1,
            name: "Recorder".into(),
            enabled: true,
            bypass: false,
            parameters: serde_json::from_value(json!({ "format": "wavPcm16" })).unwrap(),
            ports: vec![Port {
                name: "in".into(),
                direction: PortDirection::Input,
                channels: 2,
            }],
        });
        let session_id = graph.id.clone();
        let mut plane = ControlPlane::default();
        plane.insert_session(graph).unwrap();
        let request = |plane: &mut ControlPlane, method: &str, params: Value| {
            plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params: Some(params),
            })
        };
        // Nothing approved yet: a suggestion is offered, never applied.
        let root = request(&mut plane, "recordings.getRoot", json!({}))
            .result
            .unwrap();
        assert!(root["root"].is_null());
        assert!(root["suggestedRoot"]
            .as_str()
            .is_some_and(|path| path.ends_with("AudioRouter Recordings")));
        // Record explains where to choose the folder.
        let refused = request(
            &mut plane,
            "recorders.startRecording",
            json!({ "sessionId": session_id, "nodeId": "rec", "idempotencyKey": "early" }),
        );
        assert!(refused.error.unwrap().message.contains("Recording folder"));
        // Unusable folders are refused with plain reasons and change nothing.
        for (path, reason) in [
            ("relative\\folder", "full folder path"),
            ("\\\\server\\share\\rec", "network share"),
        ] {
            let error = request(
                &mut plane,
                "recordings.setRoot",
                json!({ "root": path, "create": true, "idempotencyKey": path }),
            )
            .error
            .unwrap();
            assert!(error.message.contains(reason), "{path}: {}", error.message);
        }
        let missing = base.join("missing");
        let error = request(
            &mut plane,
            "recordings.setRoot",
            json!({ "root": missing, "idempotencyKey": "no-create" }),
        )
        .error
        .unwrap();
        assert!(
            error.message.contains("does not exist"),
            "{}",
            error.message
        );
        assert!(!missing.exists());
        std::fs::create_dir_all(&base).unwrap();
        let file = base.join("file.txt");
        std::fs::write(&file, b"x").unwrap();
        let error = request(
            &mut plane,
            "recordings.setRoot",
            json!({ "root": file, "idempotencyKey": "file" }),
        )
        .error
        .unwrap();
        assert!(error.message.contains("not a folder"), "{}", error.message);
        assert!(request(&mut plane, "recordings.getRoot", json!({}))
            .result
            .unwrap()["root"]
            .is_null());
        // Approve (and create) a folder; a retry with the same key replays.
        let chosen = base.join("My Recordings");
        let set = request(
            &mut plane,
            "recordings.setRoot",
            json!({ "root": chosen, "create": true, "idempotencyKey": "choose" }),
        )
        .result
        .unwrap();
        assert_eq!(set["created"], true);
        assert!(chosen.is_dir());
        let again = request(
            &mut plane,
            "recordings.setRoot",
            json!({ "root": chosen, "create": true, "idempotencyKey": "choose" }),
        )
        .result
        .unwrap();
        assert_eq!(again, set);
        let shown = request(&mut plane, "recordings.getRoot", json!({}))
            .result
            .unwrap();
        assert_eq!(shown["root"], set["root"]);
        assert!(!shown["root"].as_str().unwrap().starts_with("\\\\?\\"));
        // Record now writes into the chosen folder.
        let started = request(
            &mut plane,
            "recorders.startRecording",
            json!({ "sessionId": session_id, "nodeId": "rec", "idempotencyKey": "now" }),
        )
        .result
        .unwrap();
        assert_eq!(started["state"], "recording");
        let path = std::path::PathBuf::from(started["path"].as_str().unwrap());
        assert_eq!(
            path.parent().unwrap().canonicalize().unwrap(),
            chosen.canonicalize().unwrap()
        );
        request(
            &mut plane,
            "recorders.stopRecording",
            json!({ "sessionId": session_id, "nodeId": "rec", "idempotencyKey": "stop" }),
        );
        drop(plane);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn one_click_recording_writes_splits_stops_and_never_blocks_stopping_playback() {
        let root = std::env::temp_dir().join(format!(
            "audiorouter-one-click-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut graph = session();
        graph.nodes.push(Node {
            id: EntityId::new("rec"),
            kind: NodeKind::Recorder,
            type_version: 1,
            name: "Recorder".into(),
            enabled: true,
            bypass: false,
            parameters: serde_json::from_value(json!({ "format": "wavPcm16", "splitMinutes": 10 }))
                .unwrap(),
            ports: vec![Port {
                name: "in".into(),
                direction: PortDirection::Input,
                channels: 2,
            }],
        });
        let session_id = graph.id.clone();
        let node_id = EntityId::new("rec");
        let mut plane = ControlPlane::default();
        plane.insert_session(graph).unwrap();
        plane.configure_recording_root(&root).unwrap();
        let call = |plane: &mut ControlPlane, method: &str, key: &str| {
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params: Some(
                    json!({ "sessionId": session_id, "nodeId": "rec", "idempotencyKey": key }),
                ),
            });
            assert!(response.error.is_none(), "{method}: {:?}", response.error);
            response.result.unwrap()
        };
        // Stop with nothing recording is a harmless no-op (StreamDeck safe).
        assert_eq!(
            call(&mut plane, "recorders.stopRecording", "stop-0")["state"],
            "idle"
        );
        let tap = plane
            .recorder_tap_set_for_node(&session_id, &node_id)
            .unwrap();
        let started = call(&mut plane, "recorders.startRecording", "start-1");
        assert_eq!(started["state"], "recording");
        assert_eq!(started["splitMinutes"], 10);
        // A second Record press while recording keeps the same take.
        assert_eq!(
            call(&mut plane, "recorders.startRecording", "start-again")["alreadyRecording"],
            true
        );
        let mut block = AudioBlock::new(2, 128).unwrap();
        block.channel_mut(0).unwrap().fill(0.25);
        block.channel_mut(1).unwrap().fill(-0.25);
        let mut frame = 1_000_000_u64;
        let mut feed = |plane: &mut ControlPlane, blocks: usize| {
            for _ in 0..blocks {
                tap.on_processed_block(frame, &block);
                frame += 128;
                plane.drain_attached_recorders().unwrap();
            }
        };
        feed(&mut plane, 20);
        // Automatic split (shortened for the test): the WAV splits in place.
        plane
            .recording_splits
            .get_mut(&node_id)
            .unwrap()
            .every_frames = 128 * 10;
        plane.maintain_node_recordings(std::time::Instant::now());
        feed(&mut plane, 20);
        plane.recording_maintained_at = None;
        plane.maintain_node_recordings(std::time::Instant::now());
        feed(&mut plane, 5);
        let stopped = call(&mut plane, "recorders.stopRecording", "stop-1");
        assert_eq!(stopped["state"], "completed");
        assert!(
            stopped["parts"].as_array().unwrap().len() >= 2,
            "split produced parts: {stopped}"
        );
        // Every split part must open in a player, not just be non-empty.
        for entry in std::fs::read_dir(&root).unwrap() {
            assert_playable_wav(&entry.unwrap().path());
        }
        // A new take is a new file; stopping playback finalizes it rather
        // than refusing to stop.
        let second = call(&mut plane, "recorders.startRecording", "start-2");
        assert_ne!(second["path"], started["path"]);
        feed(&mut plane, 4);
        let stop_error = plane
            .session_stop(&session_id)
            .err()
            .map(|error| format!("{error:?}"));
        assert!(
            !stop_error
                .as_deref()
                .unwrap_or("")
                .contains("must be finalized"),
            "{stop_error:?}"
        );
        assert!(
            !plane.recorder_node_workers.contains_key(&node_id),
            "recording finalized by Stop"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A WAV a player accepts: RIFF size matches the file, and a `data`
    /// chunk with audio ends inside the file. Returns the data byte count.
    fn assert_playable_wav(path: &std::path::Path) -> u64 {
        let bytes = std::fs::read(path).unwrap();
        assert!(
            bytes.len() > 44 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
            "{path:?} is not a WAV"
        );
        let riff = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        assert_eq!(riff, bytes.len() - 8, "{path:?}: RIFF size not finalized");
        let mut offset = 12;
        while offset + 8 <= bytes.len() {
            let id = &bytes[offset..offset + 4];
            let size =
                u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
            if id == b"data" {
                assert!(size > 0, "{path:?}: empty data chunk");
                assert!(
                    offset + 8 + size <= bytes.len(),
                    "{path:?}: data chunk overruns the file"
                );
                return size as u64;
            }
            offset += 8 + size + (size & 1);
        }
        panic!("{path:?}: no data chunk");
    }

    /// Real playback is not a perfect stream. A service stall shorter than
    /// the two-second queue (REC-08) must not lose anything. A real gap, a
    /// repeated block or a longer stall fails the take, but Stop still
    /// succeeds, keeps a playable file up to that point and says why.
    #[test]
    fn one_click_recording_survives_dropped_and_repeated_blocks_and_stays_playable() {
        // Stalling the control thread no longer stalls the encoder. Deliberate
        // blocked-storage overflow is qualified in threaded_recorder tests.
        for case in ["short stall", "dropped block", "repeated block"] {
            let root = std::env::temp_dir().join(format!(
                "audiorouter-gap-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&root).unwrap();
            let mut graph = session();
            graph.nodes.push(Node {
                id: EntityId::new("rec"),
                kind: NodeKind::Recorder,
                type_version: 1,
                name: "Recorder".into(),
                enabled: true,
                bypass: false,
                parameters: serde_json::from_value(json!({ "format": "wavPcm16" })).unwrap(),
                ports: vec![Port {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    channels: 2,
                }],
            });
            let session_id = graph.id.clone();
            let node_id = EntityId::new("rec");
            let mut plane = ControlPlane::default();
            plane.insert_session(graph).unwrap();
            plane.configure_recording_root(&root).unwrap();
            let tap = plane
                .recorder_tap_set_for_node(&session_id, &node_id)
                .unwrap();
            let request = |plane: &mut ControlPlane, method: &str, key: &str| {
                plane.dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(1)),
                    method: method.into(),
                    params: Some(
                        json!({ "sessionId": session_id, "nodeId": "rec", "idempotencyKey": key }),
                    ),
                })
            };
            let started = request(&mut plane, "recorders.startRecording", "start")
                .result
                .unwrap();
            let path = std::path::PathBuf::from(started["path"].as_str().unwrap());
            let mut block = AudioBlock::new(2, 128).unwrap();
            block.channel_mut(0).unwrap().fill(0.25);
            block.channel_mut(1).unwrap().fill(-0.25);
            let mut frame = 5_000_u64;
            let send = |plane: &mut ControlPlane, frame: &mut u64, blocks: usize, drain: bool| {
                for _ in 0..blocks {
                    tap.on_processed_block(*frame, &block);
                    *frame += 128;
                    if drain {
                        let _ = plane.drain_attached_recorders();
                    }
                }
            };
            send(&mut plane, &mut frame, 10, true);
            match case {
                // ~1 s with no service pass: within the queue.
                "short stall" => send(&mut plane, &mut frame, 375, false),
                "dropped block" => frame += 128,
                _ => {
                    frame -= 128;
                    send(&mut plane, &mut frame, 1, true);
                }
            }
            send(&mut plane, &mut frame, 10, true);
            let stopped = request(&mut plane, "recorders.stopRecording", "stop");
            assert!(
                stopped.error.is_none(),
                "{case}: Stop failed: {:?}",
                stopped.error
            );
            let stopped = stopped.result.unwrap();
            let data = assert_playable_wav(&path);
            if case == "short stall" {
                assert_eq!(stopped["state"], "completed", "{case}: {stopped}");
                // Every block arrived: 10 + 375 + 10 (2 ch × 16-bit).
                assert_eq!(data, 395 * 128 * 4, "{case}");
            } else {
                assert_eq!(stopped["state"], "failed", "{case}: {stopped}");
                assert!(
                    stopped["reason"]
                        .as_str()
                        .is_some_and(|reason| reason.contains("audio was lost")
                            && reason.contains("keeps everything")),
                    "{case}: {stopped}"
                );
                assert_eq!(stopped["paths"][0], json!(path.to_str().unwrap()), "{case}");
                // The ten blocks before the problem are kept and playable.
                assert!(data >= 10 * 128 * 4, "{case}: only {data} bytes of audio");
            }
            // The node is free for a fresh take.
            let again = request(&mut plane, "recorders.startRecording", "again")
                .result
                .unwrap();
            assert_eq!(again["state"], "recording", "{case}");
            request(&mut plane, "recorders.stopRecording", "again-stop");
            drop(plane);
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn one_click_flac_and_mp3_keep_a_finished_file_after_lost_audio() {
        for format in ["flac24", "mp3"] {
            let root = std::env::temp_dir().join(format!(
                "audiorouter-gap-{format}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&root).unwrap();
            let mut graph = session();
            graph.nodes.push(Node {
                id: EntityId::new("rec"),
                kind: NodeKind::Recorder,
                type_version: 1,
                name: "Recorder".into(),
                enabled: true,
                bypass: false,
                parameters: serde_json::from_value(json!({ "format": format })).unwrap(),
                ports: vec![Port {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    channels: 2,
                }],
            });
            let session_id = graph.id.clone();
            let node_id = EntityId::new("rec");
            let mut plane = ControlPlane::default();
            plane.insert_session(graph).unwrap();
            plane.configure_recording_root(&root).unwrap();
            let tap = plane
                .recorder_tap_set_for_node(&session_id, &node_id)
                .unwrap();
            let request = |plane: &mut ControlPlane, method: &str, key: &str| {
                plane.dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(1)),
                    method: method.into(),
                    params: Some(
                        json!({ "sessionId": session_id, "nodeId": "rec", "idempotencyKey": key }),
                    ),
                })
            };
            let started = request(&mut plane, "recorders.startRecording", "start")
                .result
                .unwrap();
            let path = std::path::PathBuf::from(started["path"].as_str().unwrap());
            let mut block = AudioBlock::new(2, 128).unwrap();
            block.channel_mut(0).unwrap().fill(0.25);
            block.channel_mut(1).unwrap().fill(-0.25);
            let mut frame = 9_000_u64;
            for index in 0..200 {
                if index == 150 {
                    frame += 128; // one dropped block
                }
                tap.on_processed_block(frame, &block);
                frame += 128;
                plane.drain_attached_recorders().unwrap();
            }
            let stopped = request(&mut plane, "recorders.stopRecording", "stop");
            assert!(
                stopped.error.is_none(),
                "{format}: Stop failed: {:?}",
                stopped.error
            );
            let stopped = stopped.result.unwrap();
            assert_eq!(stopped["state"], "failed", "{format}: {stopped}");
            assert!(
                std::fs::metadata(&path).unwrap().len() > 1_000,
                "{format}: file too small"
            );
            if format == "flac24" {
                let info = audiorouter_recording::inspect_flac_file(&path).unwrap();
                assert_eq!(info.frames, 150 * 128, "{format}: STREAMINFO frames");
            }
            drop(plane);
            let _ = std::fs::remove_dir_all(&root);
        }
    }

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

    struct FailingTapRecorderWorker {
        tap: Arc<dyn AudioTap>,
    }

    struct SuccessfulTapRecorderWorker {
        tap: Arc<dyn AudioTap>,
    }

    impl RecorderWorker for FailingTapRecorderWorker {
        fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
            Some(self.tap.clone())
        }

        fn finalize(&mut self, _frame: u64) -> Result<RecorderFinalizationOutcome, String> {
            Err("synthetic encoder failure".into())
        }
    }

    impl RecorderWorker for SuccessfulTapRecorderWorker {
        fn shared_audio_tap(&self) -> Option<Arc<dyn AudioTap>> {
            Some(self.tap.clone())
        }

        fn finalize(&mut self, _frame: u64) -> Result<RecorderFinalizationOutcome, String> {
            Ok(RecorderFinalizationOutcome {
                state: "completed".into(),
                file_finalized: true,
                recoverable: false,
            })
        }
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

    #[cfg(windows)]
    #[test]
    fn managed_virtual_device_operations_validate_before_native_access() {
        let mut plane = ControlPlane::default();
        let missing = EntityId::new("missing");
        assert!(matches!(
            plane.provision_virtual_bus_device(&missing, "bus"),
            Err(ControlError::InvalidRequest(message)) if message == "virtual bus not found"
        ));

        let id = EntityId::new("bus");
        plane.create_virtual_bus(id.clone(), "Bus").unwrap();
        plane.set_virtual_bus_enabled(&id, false).unwrap();
        assert!(matches!(
            plane.provision_virtual_bus_device(&id, "bus"),
            Err(ControlError::InvalidRequest(message)) if message == "virtual bus must be enabled before device provisioning"
        ));
        assert!(matches!(
            plane.remove_virtual_bus_device(&id),
            Err(ControlError::InvalidRequest(message)) if message.contains("not tracked")
        ));
    }

    #[test]
    fn virtual_routes_list_exposes_only_explicit_durable_routes() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(8)),
            method: "virtualRoutes.list".into(),
            params: None,
        });
        assert_eq!(
            response.result.unwrap(),
            json!({ "revision": 0, "routes": [] })
        );
        assert!(plane.describe()["methods"]
            .as_array()
            .unwrap()
            .iter()
            .any(|method| method["name"] == "virtualRoutes.list"));
    }

    #[test]
    fn virtual_routes_replace_is_revision_checked_and_idempotent() {
        let mut plane = ControlPlane::default();
        let request = |key: &str, base_revision| JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "virtualRoutes.replace".into(),
            params: Some(json!({
                "baseRevision": base_revision,
                "routes": [],
                "idempotencyKey": key
            })),
        };
        let first = plane.dispatch(request("route-1", 0));
        assert_eq!(
            first.result,
            Some(json!({ "state": "applied", "revision": 1, "routes": [] }))
        );
        assert_eq!(plane.dispatch(request("route-1", 0)).result, first.result);
        assert!(plane.dispatch(request("route-2", 0)).error.is_some());
        let listed = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(2)),
                method: "virtualRoutes.list".into(),
                params: None,
            })
            .result
            .unwrap();
        assert_eq!(listed["revision"], 1);
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
    fn session_file_export_imports_on_another_database_without_replacing_sessions() {
        let root =
            std::env::temp_dir().join(format!("audiorouter-session-rpc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a")).unwrap();
        std::fs::create_dir_all(root.join("b")).unwrap();
        let call = |plane: &mut ControlPlane, method: &str, params: Value| {
            plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params: Some(params),
            })
        };
        let file = root.join("setup.audiorouter");
        let mut source =
            ControlPlane::with_storage("export", Storage::open(root.join("a/db.sqlite")).unwrap());
        source.create_session(session()).unwrap();
        let exported = call(
            &mut source,
            "sessions.exportFile",
            json!({ "sessionId": "session", "path": file }),
        );
        assert_eq!(exported.result.unwrap()["sessionId"], "session");
        let refused = call(
            &mut source,
            "sessions.exportFile",
            json!({ "sessionId": "session", "path": file }),
        );
        assert!(
            refused.error.unwrap().message.contains("already exists"),
            "never overwrites unasked"
        );
        let before = std::fs::metadata(&file).unwrap().len();
        let replaced = call(
            &mut source,
            "sessions.exportFile",
            json!({ "sessionId": "session", "path": file, "replace": true }),
        );
        assert!(replaced.error.is_none(), "{:?}", replaced.error);
        assert_eq!(std::fs::metadata(&file).unwrap().len(), before);
        assert_eq!(
            std::fs::read_dir(&root).unwrap().count(),
            3,
            "no staged file left behind"
        );
        assert!(call(
            &mut source,
            "sessions.exportFile",
            json!({ "sessionId": "session", "path": root.join("a"), "replace": true })
        )
        .error
        .is_some());
        assert!(call(
            &mut source,
            "sessions.exportFile",
            json!({ "sessionId": "session", "path": root.join("x.txt") })
        )
        .error
        .is_some());
        assert!(call(
            &mut source,
            "sessions.exportFile",
            json!({ "sessionId": "session", "path": "relative.audiorouter" })
        )
        .error
        .is_some());

        let mut target =
            ControlPlane::with_storage("import", Storage::open(root.join("b/db.sqlite")).unwrap());
        let first = call(&mut target, "sessions.importFile", json!({ "path": file }))
            .result
            .unwrap();
        assert_eq!(first["session"]["id"], "session");
        assert_eq!(first["renamed"], false);
        let second = call(&mut target, "sessions.importFile", json!({ "path": file }))
            .result
            .unwrap();
        assert_eq!(second["session"]["id"], "session-imported-1");
        assert_eq!(second["session"]["name"], "test (imported)");
        assert_eq!(second["renamed"], true);
        assert_eq!(second["session"]["nodes"], first["session"]["nodes"]);
        drop((source, target));
        let _ = std::fs::remove_dir_all(root);
    }

    /// A fresh install: Play needs device administration, which the desktop
    /// grant lacks until the user consents once in the app. Consent persists
    /// across restarts, only the desktop window's grant may give it, and it
    /// can be withdrawn. The release 0.0.1 shipped without this path.
    #[test]
    fn desktop_consent_lets_a_fresh_install_open_devices_and_persists() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-device-consent-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let shell = ClientGrant::for_desktop_shell();
        let cli = ClientGrant::for_role(ClientRole::Operator);
        let call = |plane: &mut ControlPlane, grant: &ClientGrant, method: &str, params: Value| {
            plane.dispatch_authorized_for_client(
                JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(1)),
                    method: method.into(),
                    params: Some(params),
                },
                "consent-test",
                grant,
            )
        };
        let denied = |response: &JsonRpcResponse| {
            response
                .error
                .as_ref()
                .is_some_and(|error| error.message.contains("permission denied"))
        };
        {
            let mut plane =
                ControlPlane::with_storage("consent-first", Storage::open(&path).unwrap());
            // Before consent: Play's device preparation is refused.
            assert_eq!(
                call(&mut plane, &shell, "devices.getAccess", json!({}))
                    .result
                    .unwrap()["allowed"],
                false
            );
            let prepare = call(
                &mut plane,
                &shell,
                "nativePaths.prepare",
                json!({ "sessionId": "missing" }),
            );
            assert!(denied(&prepare), "{prepare:?}");
            // A CLI/MCP operator cannot give consent, even for itself.
            let refused = call(
                &mut plane,
                &cli,
                "devices.setAccess",
                json!({ "allowed": true, "idempotencyKey": "cli" }),
            );
            assert!(denied(&refused), "{refused:?}");
            // The desktop window gives it.
            let allowed = call(
                &mut plane,
                &shell,
                "devices.setAccess",
                json!({ "allowed": true, "idempotencyKey": "allow" }),
            );
            assert_eq!(allowed.result.unwrap()["allowed"], true);
            let prepare = call(
                &mut plane,
                &shell,
                "nativePaths.prepare",
                json!({ "sessionId": "missing" }),
            );
            assert!(
                !denied(&prepare),
                "now authorized; fails only on the missing session: {prepare:?}"
            );
            // Consent never widens other grants.
            let cli_prepare = call(
                &mut plane,
                &cli,
                "nativePaths.prepare",
                json!({ "sessionId": "missing" }),
            );
            assert!(denied(&cli_prepare), "{cli_prepare:?}");
        }
        {
            // After a restart (and a fresh grant object) the consent holds.
            let mut plane =
                ControlPlane::with_storage("consent-second", Storage::open(&path).unwrap());
            assert_eq!(
                call(&mut plane, &shell, "devices.getAccess", json!({}))
                    .result
                    .unwrap()["allowed"],
                true
            );
            let prepare = call(
                &mut plane,
                &shell,
                "nativePaths.prepare",
                json!({ "sessionId": "missing" }),
            );
            assert!(!denied(&prepare), "{prepare:?}");
            // Withdrawing it refuses device preparation again.
            call(
                &mut plane,
                &shell,
                "devices.setAccess",
                json!({ "allowed": false, "idempotencyKey": "withdraw" }),
            );
            let prepare = call(
                &mut plane,
                &shell,
                "nativePaths.prepare",
                json!({ "sessionId": "missing" }),
            );
            assert!(denied(&prepare), "{prepare:?}");
        }
        let _ = std::fs::remove_file(&path);
    }

    #[cfg(windows)]
    #[test]
    fn network_log_keeps_only_the_windows_socket_error_code() {
        assert_eq!(os_error_code("network I/O failed: Only one usage of each socket address is normally permitted. (os error 10048)"), Some(10048));
        assert_eq!(os_error_code("no code here"), None);
        assert_eq!(os_error_code("(os error x)"), None);
    }

    #[test]
    fn the_selected_session_survives_a_backend_restart() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-active-session-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let shell = ClientGrant::for_desktop_shell();
        let call = |plane: &mut ControlPlane, method: &str, params: Value| {
            plane.dispatch_authorized_for_client(
                JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(1)),
                    method: method.into(),
                    params: Some(params),
                },
                "active-session-test",
                &shell,
            )
        };
        let session = |id: &str| json!({ "id": id, "name": id, "schemaVersion": 1, "revision": 0, "nodes": [], "edges": [] });
        {
            let mut plane =
                ControlPlane::with_storage("active-first", Storage::open(&path).unwrap());
            for id in ["a-first", "z-mine"] {
                let created = call(
                    &mut plane,
                    "sessions.create",
                    json!({ "session": session(id), "idempotencyKey": format!("create-{id}") }),
                );
                assert!(created.error.is_none(), "{created:?}");
            }
            let selected = call(
                &mut plane,
                "sessions.active.set",
                json!({ "sessionId": "z-mine", "idempotencyKey": "select" }),
            );
            assert!(selected.error.is_none(), "{selected:?}");
        }
        {
            // Tray Play and autoplay at sign-in play this session, not the first one.
            let mut plane =
                ControlPlane::with_storage("active-second", Storage::open(&path).unwrap());
            assert_eq!(
                call(&mut plane, "sessions.active.get", json!({}))
                    .result
                    .unwrap()["sessionId"],
                "z-mine"
            );
        }
        let _ = std::fs::remove_file(&path);
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
    fn native_worker_lifecycle_cannot_bypass_running_session_boundary() {
        let mut plane = ControlPlane::default();
        let mut owned = session();
        owned.id = EntityId::new("native-boundary");
        plane.insert_session(owned.clone()).unwrap();
        plane.session_start(&owned.id).unwrap();
        plane.native_endpoint_session = Some(owned.id.clone());

        assert!(matches!(
            plane.stop_native_endpoint_worker(),
            Err(ControlError::InvalidRequest(message))
                if message == "stop the session before stopping its native endpoint worker"
        ));
        assert!(matches!(
            plane.detach_native_endpoint_worker(),
            Err(ControlError::InvalidRequest(message))
                if message == "stop the session before detaching its native endpoint worker"
        ));
    }

    #[test]
    fn native_graph_activation_rejects_missing_worker_before_graph_work() {
        let mut plane = ControlPlane::default();
        let mut owned = session();
        owned.id = EntityId::new("native-graph-without-worker");
        plane.create_session(owned.clone()).unwrap();
        let started = plane.session_start(&owned.id).unwrap();
        let generation = started["generation"].as_u64().unwrap();
        plane.native_endpoint_session = Some(owned.id.clone());

        let error = plane
            .activate_native_graph(&owned.id, generation, 48_000)
            .unwrap_err();
        assert!(matches!(
            error,
            ControlError::InvalidRequest(message)
                if message == "native endpoint worker is not attached"
        ));
        assert!(plane.native_endpoint_taps.is_none());
    }

    #[cfg(windows)]
    #[test]
    fn native_capture_sink_binding_rejects_mismatched_hello_before_driver_open() {
        let mut plane = ControlPlane::default();
        let error = plane
            .prepare_native_bridge(
                EntityId::new("bus-guard"),
                r"\\.\NotAudioRouter",
                r"C:\Temp\render.slot",
                r"C:\Temp\capture.slot",
                1,
                1_000,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ControlError::InvalidRequest(message)
                if message.contains("native bridge device path is not the AudioRouter broker")
        ));
        let mut plane = ControlPlane::default();
        let bus_id = EntityId::new("capture-bus");
        plane.create_virtual_bus(bus_id.clone(), "Capture").unwrap();
        let hello = audiorouter_protocol::AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: "different-bus".into(),
            direction: audiorouter_protocol::AudioBridgeDirection::CaptureSink,
            generation: 1,
            sample_rate_hz: 48_000,
            channels: 2,
            frames_per_quantum: 128,
            lease_ms: 1_000,
        };
        let error = plane
            .prepare_native_capture_sink_binding(
                bus_id,
                "\\\\.\\AudioRouterVirtualBridge",
                "C:\\missing-capture.slot",
                hello,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ControlError::InvalidRequest(message)
                if message == "native capture sink hello does not match the requested bus"
        ));
    }

    #[cfg(windows)]
    #[test]
    fn native_bridge_preparation_rejects_unbounded_or_relative_inputs_before_driver_open() {
        let mut plane = ControlPlane::default();
        let error = plane
            .prepare_native_bridge(
                EntityId::new("bounds-bus"),
                r"\\.\AudioRouterVirtualBridge",
                r"relative-render.slot",
                r"C:\capture.slot",
                1,
                0,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ControlError::InvalidRequest(message)
                if message == "native bridge paths, lease, and generation are invalid"
        ));

        let mut plane = ControlPlane::default();
        let error = plane
            .prepare_native_bridge(
                EntityId::new("bounds-bus"),
                r"\\.\AudioRouterVirtualBridge",
                r"C:\render.slot",
                r"C:\capture.slot",
                0,
                1_000,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ControlError::InvalidRequest(message)
                if message == "native bridge paths, lease, and generation are invalid"
        ));
    }

    #[cfg(windows)]
    #[test]
    fn native_render_source_binding_rejects_mismatched_hello_before_driver_open() {
        let mut plane = ControlPlane::default();
        let bus_id = EntityId::new("render-bus");
        plane.create_virtual_bus(bus_id.clone(), "Render").unwrap();
        let hello = audiorouter_protocol::AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: "different-bus".into(),
            direction: audiorouter_protocol::AudioBridgeDirection::RenderSource,
            generation: 1,
            sample_rate_hz: 48_000,
            channels: 2,
            frames_per_quantum: 128,
            lease_ms: 1_000,
        };
        let error = plane
            .prepare_native_render_source_binding(
                bus_id,
                "\\\\.\\AudioRouterVirtualBridge",
                "C:\\missing-render.slot",
                hello,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ControlError::InvalidRequest(message)
                if message == "native render source hello does not match the requested bus"
        ));
    }

    #[cfg(windows)]
    #[test]
    fn native_duplex_binding_rejects_mismatched_hellos_before_driver_open() {
        let mut plane = ControlPlane::default();
        let bus_id = EntityId::new("duplex-bus");
        plane.create_virtual_bus(bus_id.clone(), "Duplex").unwrap();
        let render_hello = audiorouter_protocol::AudioBridgeHello {
            protocol_major: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MAJOR,
            protocol_minor: audiorouter_protocol::AUDIO_BRIDGE_PROTOCOL_MINOR,
            bus_id: bus_id.as_str().into(),
            direction: audiorouter_protocol::AudioBridgeDirection::RenderSource,
            generation: 2,
            sample_rate_hz: 48_000,
            channels: 2,
            frames_per_quantum: 128,
            lease_ms: 1_000,
        };
        let mut capture_hello = render_hello.clone();
        capture_hello.direction = audiorouter_protocol::AudioBridgeDirection::CaptureSink;
        capture_hello.generation = 3;
        let error = plane
            .prepare_native_duplex_binding(
                bus_id,
                "\\\\.\\AudioRouterVirtualBridge",
                "C:\\missing-render.slot",
                "C:\\missing-capture.slot",
                render_hello,
                capture_hello,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ControlError::InvalidRequest(message)
                if message == "native duplex hellos do not match the requested bus"
        ));
    }

    #[test]
    fn native_worker_preparation_rejects_mismatched_endpoint_shapes_before_opening() {
        let mut plane = ControlPlane::default();
        let mut owned = session();
        owned.id = EntityId::new("native-worker-shape-mismatch");
        plane.create_session(owned).unwrap();
        let endpoint = |direction, channels| audiorouter_windows_audio::EndpointInfo {
            id: format!("{direction:?}-{channels}"),
            direction,
            default_period_100ns: 100_000,
            minimum_period_100ns: 30_000,
            sample_rate_hz: 48_000,
            channels,
            bits_per_sample: 32,
            format_tag: 3,
            channel_mask: 3,
            subformat_guid: "00000003-0000-0010-8000-00aa00389b71".into(),
        };
        // Mono capture is intentionally supported for microphone routes and
        // duplicated into the stereo graph. Three channels remain invalid.
        let capture = endpoint(audiorouter_windows_audio::EndpointDirection::Capture, 3);
        let render = endpoint(audiorouter_windows_audio::EndpointDirection::Render, 2);

        let error = plane
            .prepare_native_endpoint_worker(
                EntityId::new("native-worker-shape-mismatch"),
                &capture,
                &render,
                0,
                1,
                0,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ControlError::Audio {
                code: "invalidArgument",
                ..
            }
        ));
        assert!(plane.native_endpoint_worker.is_none());
        assert!(plane.endpoint_monitor.is_none());
    }

    #[test]
    fn recovery_endpoint_resnapshot_publishes_a_bounded_device_change_event() {
        let mut plane = ControlPlane::default();
        let endpoint = audiorouter_windows_audio::EndpointInfo {
            id: "recovery-endpoint".into(),
            direction: audiorouter_windows_audio::EndpointDirection::Render,
            default_period_100ns: 100_000,
            minimum_period_100ns: 30_000,
            sample_rate_hz: 48_000,
            channels: 2,
            bits_per_sample: 32,
            format_tag: 3,
            channel_mask: 3,
            subformat_guid: "00000003-0000-0010-8000-00aa00389b71".into(),
        };
        let before = plane.events.latest_sequence();

        plane
            .retain_endpoint_changes(&[audiorouter_windows_audio::EndpointChange::Added(endpoint)]);

        assert_eq!(plane.events.latest_sequence(), before + 1);
        assert_eq!(
            plane.events.since(before, 1).unwrap()[0].category,
            "devices.changed"
        );
    }

    #[test]
    fn native_application_preparation_requires_device_administration_before_platform_access() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(91)),
                method: "nativeApplications.prepare".into(),
                params: Some(json!({ "processId": 1 })),
            },
            &ClientGrant::read_only(),
        );
        assert_eq!(response.error.unwrap().code, -32001);
        assert!(plane.endpoint_monitor.is_none());
        assert!(plane.native_endpoint_worker.is_none());
    }

    #[test]
    fn native_application_worker_rejects_missing_graph_source_before_platform_access() {
        let mut plane = ControlPlane::default();
        let mut owned = session();
        owned.id = EntityId::new("application-worker-source-required");
        plane.create_session(owned).unwrap();
        let render = audiorouter_windows_audio::EndpointInfo {
            id: "render".into(),
            direction: audiorouter_windows_audio::EndpointDirection::Render,
            default_period_100ns: 100_000,
            minimum_period_100ns: 30_000,
            sample_rate_hz: 48_000,
            channels: 2,
            bits_per_sample: 32,
            format_tag: 3,
            channel_mask: 3,
            subformat_guid: "00000003-0000-0010-8000-00aa00389b71".into(),
        };
        let error = plane
            .prepare_native_application_worker(
                EntityId::new("application-worker-source-required"),
                NativeApplicationWorkerConfig {
                    process_id: 12_345,
                    expected_executable: "probe.exe",
                    expected_executable_path: None,
                    expected_creation_time_100ns: 1,
                    mode: audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
                    render: &render,
                    buffer_duration_100ns: 0,
                    max_attempts: 1,
                    retry_delay_ms: 0,
                },
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ControlError::InvalidRequest(message)
                if message == "application worker requires a matching enabled applicationCapture node"
        ));
        assert!(plane.native_endpoint_worker.is_none());
        assert!(plane.endpoint_monitor.is_none());
    }

    #[test]
    fn native_application_worker_rejects_graph_identity_mismatch_before_platform_access() {
        let mut owned = session();
        owned.id = EntityId::new("application-worker-identity-required");
        owned.nodes[0] = Node {
            id: EntityId::new("application"),
            kind: NodeKind::ApplicationCapture,
            type_version: 1,
            name: "Application capture".into(),
            enabled: true,
            bypass: false,
            parameters: [
                ("executable".into(), json!("actual.exe")),
                ("processPolicy".into(), json!("selectedInstance")),
                ("processId".into(), json!(42)),
                ("creationTime100ns".into(), json!("100")),
            ]
            .into_iter()
            .collect(),
            ports: vec![Port {
                name: "main".into(),
                direction: PortDirection::Output,
                channels: 2,
            }],
        };
        owned.nodes[1].ports[0].channels = 2;
        owned.edges[0].source_node = EntityId::new("application");
        owned.edges[0].matrix = vec![1.0, 0.0, 0.0, 1.0];
        let mut plane = ControlPlane::default();
        plane.create_session(owned).unwrap();
        let render = audiorouter_windows_audio::EndpointInfo {
            id: "render".into(),
            direction: audiorouter_windows_audio::EndpointDirection::Render,
            default_period_100ns: 100_000,
            minimum_period_100ns: 30_000,
            sample_rate_hz: 48_000,
            channels: 2,
            bits_per_sample: 32,
            format_tag: 3,
            channel_mask: 3,
            subformat_guid: "00000003-0000-0010-8000-00aa00389b71".into(),
        };
        let error = plane
            .prepare_native_application_worker(
                EntityId::new("application-worker-identity-required"),
                NativeApplicationWorkerConfig {
                    process_id: 42,
                    expected_executable: "ACTUAL.EXE",
                    expected_executable_path: None,
                    expected_creation_time_100ns: 101,
                    mode: audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
                    render: &render,
                    buffer_duration_100ns: 0,
                    max_attempts: 1,
                    retry_delay_ms: 0,
                },
            )
            .unwrap_err();
        assert!(
            matches!(error, ControlError::InvalidRequest(message) if message.contains("matching enabled"))
        );
        assert!(plane.endpoint_monitor.is_none());
        assert!(plane.native_endpoint_worker.is_none());
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires explicit application identity, render endpoint, and AUDIOROUTER_ALLOW_LIVE_AUDIO=1"]
    fn guarded_live_native_application_worker_lifecycle_uses_one_control_plane() {
        if std::env::var("AUDIOROUTER_ALLOW_LIVE_AUDIO").as_deref() != Ok("1") {
            return;
        }
        let process_id = std::env::var("AUDIOROUTER_APPLICATION_PROCESS_ID")
            .expect("AUDIOROUTER_APPLICATION_PROCESS_ID is required")
            .parse::<u32>()
            .expect("application process ID must be numeric");
        let executable = std::env::var("AUDIOROUTER_APPLICATION_EXECUTABLE")
            .expect("AUDIOROUTER_APPLICATION_EXECUTABLE is required");
        let creation_time = std::env::var("AUDIOROUTER_APPLICATION_CREATION_TIME_100NS")
            .expect("AUDIOROUTER_APPLICATION_CREATION_TIME_100NS is required")
            .parse::<u64>()
            .expect("application creation time must be numeric");
        let mode = match std::env::var("AUDIOROUTER_APPLICATION_MODE")
            .unwrap_or_else(|_| "include".into())
            .as_str()
        {
            "include" => audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
            "exclude" => audiorouter_windows_audio::ProcessLoopbackMode::ExcludeTargetTree,
            value => panic!("application mode must be include or exclude, got {value}"),
        };
        let render_id = std::env::var("AUDIOROUTER_RENDER_ENDPOINT_ID")
            .expect("AUDIOROUTER_RENDER_ENDPOINT_ID is required");
        let render = audiorouter_windows_audio::enumerate_active_endpoints()
            .unwrap()
            .into_iter()
            .find(|endpoint| {
                endpoint.id == render_id
                    && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Render
            })
            .expect("configured render endpoint is not active");
        let mut owned = session();
        owned.id = EntityId::new("guarded-live-application");
        owned.nodes[0] = Node {
            id: EntityId::new("application"),
            kind: NodeKind::ApplicationCapture,
            type_version: 1,
            name: "Application capture".into(),
            enabled: true,
            bypass: false,
            parameters: [
                ("executable".into(), json!(executable)),
                ("processPolicy".into(), json!("selectedInstance")),
                ("processId".into(), json!(process_id)),
                ("creationTime100ns".into(), json!(creation_time.to_string())),
            ]
            .into_iter()
            .collect(),
            ports: vec![Port {
                name: "main".into(),
                direction: PortDirection::Output,
                channels: 2,
            }],
        };
        owned.nodes[1].ports[0].channels = 2;
        owned.edges[0].source_node = EntityId::new("application");
        owned.edges[0].matrix = vec![1.0, 0.0, 0.0, 1.0];
        let mut plane = ControlPlane::default();
        plane.create_session(owned).unwrap();
        plane
            .prepare_native_application_worker(
                EntityId::new("guarded-live-application"),
                NativeApplicationWorkerConfig {
                    process_id,
                    expected_executable: &executable,
                    expected_executable_path: std::env::var("AUDIOROUTER_APPLICATION_PATH")
                        .ok()
                        .as_deref(),
                    expected_creation_time_100ns: creation_time,
                    mode,
                    render: &render,
                    buffer_duration_100ns: 0,
                    max_attempts: 3,
                    retry_delay_ms: 100,
                },
            )
            .unwrap();
        let started = plane
            .session_start(&EntityId::new("guarded-live-application"))
            .unwrap();
        let generation = started["generation"].as_u64().unwrap();
        let started_at = Instant::now();
        let mut packets = 0_u64;
        let mut processed_quanta = 0_u64;
        let mut rendered_frames = 0_u64;
        while started_at.elapsed() < Duration::from_millis(500) {
            let result = plane
                .pump_native_endpoint_worker_with_bound_taps(
                    &EntityId::new("guarded-live-application"),
                    generation,
                    64,
                )
                .unwrap();
            packets = packets.saturating_add(result["packets"].as_u64().unwrap_or(0));
            processed_quanta =
                processed_quanta.saturating_add(result["processedQuanta"].as_u64().unwrap_or(0));
            rendered_frames =
                rendered_frames.saturating_add(result["renderedFrames"].as_u64().unwrap_or(0));
            std::thread::sleep(Duration::from_millis(1));
        }
        plane
            .session_stop(&EntityId::new("guarded-live-application"))
            .unwrap();
        let restarted = plane
            .session_start(&EntityId::new("guarded-live-application"))
            .unwrap();
        let restarted_generation = restarted["generation"].as_u64().unwrap();
        let restarted_at = Instant::now();
        while restarted_at.elapsed() < Duration::from_millis(500) {
            let result = plane
                .pump_native_endpoint_worker_with_bound_taps(
                    &EntityId::new("guarded-live-application"),
                    restarted_generation,
                    64,
                )
                .unwrap();
            packets = packets.saturating_add(result["packets"].as_u64().unwrap_or(0));
            processed_quanta =
                processed_quanta.saturating_add(result["processedQuanta"].as_u64().unwrap_or(0));
            rendered_frames =
                rendered_frames.saturating_add(result["renderedFrames"].as_u64().unwrap_or(0));
            std::thread::sleep(Duration::from_millis(1));
        }
        plane
            .session_stop(&EntityId::new("guarded-live-application"))
            .unwrap();
        plane.detach_native_endpoint_worker().unwrap();
        assert!(packets > 0);
        assert!(processed_quanta > 0);
        assert!(rendered_frames > 0);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires explicit live endpoint IDs and AUDIOROUTER_ALLOW_LIVE_AUDIO=1"]
    fn guarded_live_native_endpoint_session_lifecycle_uses_one_control_plane() {
        if std::env::var("AUDIOROUTER_ALLOW_LIVE_AUDIO").as_deref() != Ok("1") {
            return;
        }
        let capture_id = std::env::var("AUDIOROUTER_CAPTURE_ENDPOINT_ID")
            .expect("AUDIOROUTER_CAPTURE_ENDPOINT_ID is required");
        let render_id = std::env::var("AUDIOROUTER_RENDER_ENDPOINT_ID")
            .expect("AUDIOROUTER_RENDER_ENDPOINT_ID is required");
        let fanout_endpoint_ids = std::env::var("AUDIOROUTER_OUTPUT_FANOUT_ENDPOINT_IDS")
            .unwrap_or_default()
            .split('|')
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let endpoints = audiorouter_windows_audio::enumerate_active_endpoints().unwrap();
        let capture = endpoints
            .iter()
            .find(|endpoint| {
                endpoint.id == capture_id
                    && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Capture
            })
            .expect("configured capture endpoint is not an active exact match")
            .clone();
        let render = endpoints
            .iter()
            .find(|endpoint| {
                endpoint.id == render_id
                    && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Render
            })
            .expect("configured render endpoint is not an active exact match")
            .clone();
        let mut plane = ControlPlane::default();
        let mut owned = session();
        owned.id = EntityId::new("guarded-live-native");
        owned.nodes.insert(
            1,
            Node {
                id: EntityId::new("eq"),
                kind: NodeKind::ParametricEq,
                type_version: 1,
                name: "Live EQ".into(),
                enabled: true,
                bypass: false,
                parameters: [
                    ("band0Enabled".into(), json!(true)),
                    ("band0Type".into(), json!("peaking")),
                    ("band0FrequencyHz".into(), json!(1000.0)),
                    ("band0Q".into(), json!(1.0)),
                    ("band0GainDb".into(), json!(-6.0)),
                ]
                .into_iter()
                .collect(),
                ports: vec![
                    Port {
                        name: "in".into(),
                        direction: PortDirection::Input,
                        channels: 1,
                    },
                    Port {
                        name: "out".into(),
                        direction: PortDirection::Output,
                        channels: 1,
                    },
                ],
            },
        );
        owned.nodes.insert(
            2,
            Node {
                id: EntityId::new("gate"),
                kind: NodeKind::Gate,
                type_version: 1,
                name: "Live Gate".into(),
                enabled: true,
                bypass: false,
                parameters: [
                    ("thresholdDb".into(), json!(-45.0)),
                    ("rangeDb".into(), json!(60.0)),
                    ("hysteresisDb".into(), json!(3.0)),
                    ("ratio".into(), json!(4.0)),
                    ("attackMs".into(), json!(5.0)),
                    ("holdMs".into(), json!(50.0)),
                    ("releaseMs".into(), json!(150.0)),
                ]
                .into_iter()
                .collect(),
                ports: vec![
                    Port {
                        name: "in".into(),
                        direction: PortDirection::Input,
                        channels: 1,
                    },
                    Port {
                        name: "out".into(),
                        direction: PortDirection::Output,
                        channels: 1,
                    },
                ],
            },
        );
        owned.nodes.insert(
            3,
            Node {
                id: EntityId::new("compressor"),
                kind: NodeKind::Compressor,
                type_version: 1,
                name: "Live Compressor".into(),
                enabled: true,
                bypass: false,
                parameters: [
                    ("thresholdDb".into(), json!(-18.0)),
                    ("ratio".into(), json!(3.0)),
                    ("attackMs".into(), json!(10.0)),
                    ("releaseMs".into(), json!(150.0)),
                    ("kneeDb".into(), json!(6.0)),
                    ("makeupDb".into(), json!(0.0)),
                ]
                .into_iter()
                .collect(),
                ports: vec![
                    Port {
                        name: "in".into(),
                        direction: PortDirection::Input,
                        channels: 1,
                    },
                    Port {
                        name: "out".into(),
                        direction: PortDirection::Output,
                        channels: 1,
                    },
                ],
            },
        );
        owned.nodes.insert(
            4,
            Node {
                id: EntityId::new("pitch"),
                kind: NodeKind::Pitch,
                type_version: 1,
                name: "Live Pitch".into(),
                enabled: true,
                bypass: false,
                parameters: [
                    ("semitones".into(), json!(2.0)),
                    ("cents".into(), json!(0.0)),
                ]
                .into_iter()
                .collect(),
                ports: vec![
                    Port {
                        name: "in".into(),
                        direction: PortDirection::Input,
                        channels: 1,
                    },
                    Port {
                        name: "out".into(),
                        direction: PortDirection::Output,
                        channels: 1,
                    },
                ],
            },
        );
        owned.nodes.insert(
            5,
            Node {
                id: EntityId::new("limiter"),
                kind: NodeKind::Limiter,
                type_version: 1,
                name: "Live Limiter".into(),
                enabled: true,
                bypass: false,
                parameters: [
                    ("ceilingDb".into(), json!(-1.0)),
                    ("lookaheadMs".into(), json!(5.0)),
                    ("releaseMs".into(), json!(100.0)),
                ]
                .into_iter()
                .collect(),
                ports: vec![
                    Port {
                        name: "in".into(),
                        direction: PortDirection::Input,
                        channels: 1,
                    },
                    Port {
                        name: "out".into(),
                        direction: PortDirection::Output,
                        channels: 1,
                    },
                ],
            },
        );
        owned.edges = vec![
            Edge {
                id: EntityId::new("edge-in-eq"),
                source_node: EntityId::new("in"),
                source_port: "main".into(),
                destination_node: EntityId::new("eq"),
                destination_port: "in".into(),
                matrix: vec![1.0],
                enabled: true,
            },
            Edge {
                id: EntityId::new("edge-eq-gate"),
                source_node: EntityId::new("eq"),
                source_port: "out".into(),
                destination_node: EntityId::new("gate"),
                destination_port: "in".into(),
                matrix: vec![1.0],
                enabled: true,
            },
            Edge {
                id: EntityId::new("edge-gate-compressor"),
                source_node: EntityId::new("gate"),
                source_port: "out".into(),
                destination_node: EntityId::new("compressor"),
                destination_port: "in".into(),
                matrix: vec![1.0],
                enabled: true,
            },
            Edge {
                id: EntityId::new("edge-compressor-pitch"),
                source_node: EntityId::new("compressor"),
                source_port: "out".into(),
                destination_node: EntityId::new("pitch"),
                destination_port: "in".into(),
                matrix: vec![1.0],
                enabled: true,
            },
            Edge {
                id: EntityId::new("edge-pitch-limiter"),
                source_node: EntityId::new("pitch"),
                source_port: "out".into(),
                destination_node: EntityId::new("limiter"),
                destination_port: "in".into(),
                matrix: vec![1.0],
                enabled: true,
            },
            Edge {
                id: EntityId::new("edge-limiter-out"),
                source_node: EntityId::new("limiter"),
                source_port: "out".into(),
                destination_node: EntityId::new("out"),
                destination_port: "main".into(),
                matrix: vec![1.0],
                enabled: true,
            },
        ];
        plane.insert_session(owned.clone()).unwrap();
        plane
            .prepare_native_endpoint_worker(owned.id.clone(), &capture, &render, 0, 3, 100)
            .unwrap();
        if !fanout_endpoint_ids.is_empty() {
            plane
                .prepare_native_output_fanout(owned.id.clone(), 1, &fanout_endpoint_ids)
                .unwrap();
        }
        let started = plane.session_start(&owned.id).unwrap();
        assert_eq!(started["runtime"], "native");
        assert_eq!(
            plane.native_endpoint_lifecycle_telemetry()["successfulStarts"],
            1
        );
        let generation = started["generation"].as_u64().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        let mut captured_frames = 0_u64;
        let mut processed_quanta = 0_u64;
        let mut rendered_frames = 0_u64;
        let mut fanout_packets = 0_u64;
        let mut fanout_rendered_frames = 0_u64;
        while std::time::Instant::now() < deadline {
            let pump = plane
                .pump_native_endpoint_worker_with_bound_taps(&owned.id, generation, 64)
                .unwrap();
            captured_frames =
                captured_frames.saturating_add(pump["capturedFrames"].as_u64().unwrap());
            processed_quanta =
                processed_quanta.saturating_add(pump["processedQuanta"].as_u64().unwrap());
            rendered_frames =
                rendered_frames.saturating_add(pump["renderedFrames"].as_u64().unwrap());
            if let Some(fanout) = pump.get("outputFanout") {
                fanout_packets = fanout_packets.saturating_add(fanout["packets"].as_u64().unwrap());
                fanout_rendered_frames = fanout_rendered_frames
                    .saturating_add(fanout["renderedFrames"].as_u64().unwrap());
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(captured_frames > 0, "native worker did not capture frames");
        assert!(
            processed_quanta > 0,
            "native worker did not process graph quanta"
        );
        assert!(rendered_frames > 0, "native worker did not render frames");
        if !fanout_endpoint_ids.is_empty() {
            assert!(fanout_packets > 0, "output fan-out did not drain packets");
            assert!(
                fanout_rendered_frames > 0,
                "output fan-out did not render frames"
            );
        }
        eprintln!(
            "guarded_native_lifecycle capture_frames={captured_frames} processed_quanta={processed_quanta} rendered_frames={rendered_frames} fanout_packets={fanout_packets} fanout_rendered_frames={fanout_rendered_frames}"
        );
        let privacy_enabled = plane
            .dispatch_privacy_mute(Some(json!({
                "muted": true,
                "idempotencyKey": "guarded-live-privacy-enable"
            })))
            .unwrap();
        assert_eq!(privacy_enabled["muted"], true);
        assert_eq!(
            plane.status_snapshot().unwrap()["privacyMute"]["muted"],
            true
        );
        let mut mute_dispatch_to_processed_block = Vec::with_capacity(8);
        for sample in 0..8 {
            let before = plane
                .native_scheduler_telemetry()
                .get("processedQuanta")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let started = std::time::Instant::now();
            let response = plane
                .dispatch_privacy_mute(Some(json!({
                    "muted": true,
                    "idempotencyKey": format!("guarded-live-privacy-sample-{sample}"),
                })))
                .unwrap();
            assert_eq!(response["muted"], true);
            let deadline = started + std::time::Duration::from_millis(100);
            let mut observed = false;
            while std::time::Instant::now() < deadline {
                let pump = plane
                    .pump_native_endpoint_worker_with_bound_taps(&owned.id, generation, 64)
                    .unwrap();
                if pump["processedQuanta"].as_u64().unwrap_or(0) > 0 {
                    observed = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            assert!(observed, "privacy mute did not reach a processed block");
            let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
            mute_dispatch_to_processed_block.push(elapsed_ms);
            let response = plane
                .dispatch_privacy_mute(Some(json!({
                    "muted": false,
                    "idempotencyKey": format!("guarded-live-privacy-clear-{sample}"),
                })))
                .unwrap();
            assert_eq!(response["muted"], false);
            assert!(
                plane
                    .native_scheduler_telemetry()
                    .get("processedQuanta")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    >= before
            );
        }
        mute_dispatch_to_processed_block.sort_by(f64::total_cmp);
        let p95_index = (mute_dispatch_to_processed_block.len() * 95).div_ceil(100) - 1;
        let p95_ms = mute_dispatch_to_processed_block[p95_index];
        assert!(
            p95_ms <= 100.0,
            "privacy mute dispatch-to-processed p95 exceeded 100 ms: {p95_ms:.3} ms"
        );
        eprintln!("guarded_native_privacy_mute_dispatch_to_processed_p95_ms={p95_ms:.3}");
        let privacy_disabled = plane
            .dispatch_privacy_mute(Some(json!({
                "muted": false,
                "idempotencyKey": "guarded-live-privacy-disable"
            })))
            .unwrap();
        assert_eq!(privacy_disabled["muted"], false);
        assert_eq!(
            plane.status_snapshot().unwrap()["privacyMute"]["muted"],
            false
        );
        let stopped = plane.session_stop(&owned.id).unwrap();
        assert_eq!(stopped["runtime"], "native");
        assert_eq!(
            plane.native_endpoint_lifecycle_telemetry()["successfulStops"],
            1
        );
        if !fanout_endpoint_ids.is_empty() {
            plane.detach_native_output_fanout().unwrap();
        }
        plane
            .rebind_native_endpoint_worker(&owned.id, &capture_id, &render_id, 0, 3, 100)
            .unwrap();
        assert_eq!(
            plane.native_endpoint_lifecycle_telemetry()["successfulStarts"],
            1
        );
        let rebound = plane.session_start(&owned.id).unwrap();
        assert_eq!(rebound["runtime"], "native");
        plane.session_stop(&owned.id).unwrap();
        assert_eq!(
            plane.native_endpoint_lifecycle_telemetry()["successfulStops"],
            2
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires explicit live endpoint IDs and AUDIOROUTER_ALLOW_LIVE_AUDIO=1"]
    fn guarded_live_test_signal_reaches_destination_meter() {
        if std::env::var("AUDIOROUTER_ALLOW_LIVE_AUDIO").as_deref() != Ok("1") {
            return;
        }
        let capture_id = std::env::var("AUDIOROUTER_CAPTURE_ENDPOINT_ID")
            .expect("AUDIOROUTER_CAPTURE_ENDPOINT_ID is required");
        let render_id = std::env::var("AUDIOROUTER_RENDER_ENDPOINT_ID")
            .expect("AUDIOROUTER_RENDER_ENDPOINT_ID is required");
        let endpoints = audiorouter_windows_audio::enumerate_active_endpoints().unwrap();
        let capture = endpoints
            .iter()
            .find(|endpoint| {
                endpoint.id == capture_id
                    && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Capture
            })
            .expect("configured capture endpoint is not an active exact match")
            .clone();
        let render = endpoints
            .iter()
            .find(|endpoint| {
                endpoint.id == render_id
                    && endpoint.direction == audiorouter_windows_audio::EndpointDirection::Render
            })
            .expect("configured render endpoint is not an active exact match")
            .clone();

        let session_id = EntityId::new("guarded-live-test-signal");
        let mut owned = session();
        owned.id = session_id.clone();
        owned.nodes = vec![
            Node {
                id: EntityId::new("test-signal"),
                kind: NodeKind::TestSignal,
                type_version: 1,
                name: "Test Signal".into(),
                enabled: true,
                bypass: false,
                parameters: [
                    ("frequencyHz".into(), json!(440.0)),
                    ("levelDb".into(), json!(-18.0)),
                    ("durationMs".into(), json!(600_000.0)),
                ]
                .into_iter()
                .collect(),
                ports: vec![Port {
                    name: "main".into(),
                    direction: PortDirection::Output,
                    channels: 2,
                }],
            },
            Node {
                id: EntityId::new("out"),
                kind: NodeKind::PhysicalOutput,
                type_version: 1,
                name: "Destination".into(),
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![Port {
                    name: "main".into(),
                    direction: PortDirection::Input,
                    channels: 2,
                }],
            },
        ];
        owned.edges = vec![Edge {
            id: EntityId::new("edge-test-signal-output"),
            source_node: EntityId::new("test-signal"),
            source_port: "main".into(),
            destination_node: EntityId::new("out"),
            destination_port: "main".into(),
            matrix: vec![1.0, 0.0, 0.0, 1.0],
            enabled: true,
        }];

        let mut plane = ControlPlane::default();
        let mut initial = session();
        initial.id = session_id.clone();
        plane.insert_session(initial).unwrap();
        let plan_id = plane.plan_graph(&session_id, 0, owned).unwrap();
        let committed = plane
            .commit_graph(&plan_id, 0, "guarded-test-signal")
            .unwrap();
        assert_eq!(committed["revision"], 1);
        plane
            .prepare_native_endpoint_worker(session_id.clone(), &capture, &render, 0, 3, 100)
            .unwrap();
        let started = plane.session_start(&session_id).unwrap();
        let generation = started["generation"].as_u64().unwrap();
        plane
            .dispatch_audio_source_transport(Some(json!({
                "sessionId": session_id,
                "nodeId": "test-signal",
                "action": "play"
            })))
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        let mut processed_quanta = 0_u64;
        while std::time::Instant::now() < deadline {
            let pump = plane
                .pump_native_endpoint_worker_with_bound_taps(&session_id, generation, 64)
                .unwrap();
            processed_quanta =
                processed_quanta.saturating_add(pump["processedQuanta"].as_u64().unwrap_or(0));
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            processed_quanta > 0,
            "Test Signal graph did not process quanta"
        );
        let telemetry = plane.native_node_telemetry();
        let destination = telemetry
            .as_array()
            .and_then(|nodes| nodes.iter().find(|node| node["nodeId"] == "out"))
            .expect("destination node telemetry is missing");
        assert!(
            destination["meter"]["peakDb"].as_f64().unwrap_or(-120.0) > -120.0,
            "destination meter remained at the finite silence floor: {destination}"
        );
        eprintln!(
            "guarded_test_signal_meter processed_quanta={} destination_peak_db={:.3}",
            processed_quanta,
            destination["meter"]["peakDb"].as_f64().unwrap_or(-120.0)
        );
        plane.session_stop(&session_id).unwrap();
        plane.detach_native_endpoint_worker().unwrap();
    }

    #[test]
    fn native_multi_inputs_prepare_param_allowlist_matches_the_real_request_shape() {
        let real_request = json!({
            "sessionId": "demo-session",
            "generation": 1,
            "sources": [
                { "kind": "physical", "endpointId": "capture-1" },
                {
                    "kind": "application",
                    "processId": 4242,
                    "executable": "Zoom.exe",
                    "executablePath": null,
                    "creationTime100ns": "999999999",
                    "mode": "include",
                },
            ],
        });
        assert!(validate_method_params("nativeMultiInputs.prepare", Some(&real_request)).is_ok());

        let stale_shape = json!({
            "sessionId": "demo-session",
            "generation": 1,
            "captureEndpointIds": ["capture-1", "capture-2"],
        });
        let error =
            validate_method_params("nativeMultiInputs.prepare", Some(&stale_shape)).unwrap_err();
        assert!(matches!(
            error,
            ControlError::InvalidRequest(message) if message.contains("unknown parameter")
        ));
    }

    #[test]
    fn native_paths_prepare_takes_only_the_session_and_needs_device_administration() {
        let request = json!({ "sessionId": "patrick-main", "generation": 2 });
        assert!(validate_method_params("nativePaths.prepare", Some(&request)).is_ok());
        let with_sources = json!({ "sessionId": "patrick-main", "sources": [] });
        assert!(validate_method_params("nativePaths.prepare", Some(&with_sources)).is_err());
        let spec = API_METHODS
            .iter()
            .find(|spec| spec.name == "nativePaths.prepare")
            .expect("nativePaths.prepare is discoverable");
        assert_eq!(spec.permission, PermissionScope::DeviceAdministration);
        let response = ControlPlane::default().dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "nativePaths.prepare".into(),
                params: Some(request),
            },
            &ClientGrant::for_desktop_shell(),
        );
        assert_eq!(response.error.map(|error| error.code), Some(-32001));
    }

    /// Live check of a saved multi-path session on this machine's devices:
    /// prepare, start (with plugins), pump, read signal timing, stop. Privacy
    /// mute is set first so nothing is audible. Point
    /// `AUDIOROUTER_LIVE_PATHS_DATABASE` at a COPY of a database holding the
    /// session (`AUDIOROUTER_LIVE_PATHS_SESSION`, default
    /// `patrick-main-session`) and `AUDIOROUTER_PLUGIN_WORKER_PATH` at a built
    /// plugin worker.
    /// The whole product path of the network tools, with no audio device:
    /// Test Signal -> Network Send -> UDP (127.0.0.1) -> Network Receive ->
    /// Recorder, prepared and started like Play, serviced by the backend
    /// audio loop, reported in telemetry, and recorded as a continuous tone.
    #[cfg(windows)]
    #[test]
    fn network_send_and_receive_carry_a_continuous_tone_through_the_backend() {
        let root = std::env::temp_dir().join(format!(
            "audiorouter-net-route-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let port_number = std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let port = |name: &str, direction| Port {
            name: name.into(),
            direction,
            channels: 2,
        };
        let node = |id: &str, kind, ports, parameters: Value| Node {
            id: EntityId::new(id),
            kind,
            type_version: 1,
            name: id.into(),
            enabled: true,
            bypass: false,
            parameters: serde_json::from_value(parameters).unwrap(),
            ports,
        };
        let edge = |from: &str, to: &str| audiorouter_domain::Edge {
            id: EntityId::new(format!("{from}-{to}")),
            source_node: EntityId::new(from),
            source_port: "out".into(),
            destination_node: EntityId::new(to),
            destination_port: "in".into(),
            matrix: vec![1.0, 0.0, 0.0, 1.0],
            enabled: true,
        };
        let tone = node(
            "tone",
            NodeKind::TestSignal,
            vec![port("out", PortDirection::Output)],
            json!({ "frequencyHz": 997.0, "levelDb": -12.0, "durationMs": 600000.0 }),
        );
        let send = node(
            "net-send",
            NodeKind::NetworkSend,
            vec![port("in", PortDirection::Input)],
            json!({ "host": "127.0.0.1", "port": port_number }),
        );
        let receive = node(
            "net-receive",
            NodeKind::NetworkReceive,
            vec![port("out", PortDirection::Output)],
            json!({ "sender": "127.0.0.1", "port": port_number, "bufferMs": 40.0 }),
        );
        let rec = node(
            "rec",
            NodeKind::Recorder,
            vec![port("in", PortDirection::Input)],
            json!({ "format": "wavFloat32" }),
        );
        let session = |id: &str, nodes: Vec<Node>, edges| Session {
            id: EntityId::new(id),
            name: id.into(),
            schema_version: 1,
            revision: 0,
            nodes,
            edges,
        };
        // One session holding both ends, then two separate backends (two
        // computers: each its own worker, clock and timeline).
        for two_computers in [false, true] {
            let sessions = if two_computers {
                vec![
                    session(
                        "sending-pc",
                        vec![tone.clone(), send.clone()],
                        vec![edge("tone", "net-send")],
                    ),
                    session(
                        "receiving-pc",
                        vec![receive.clone(), rec.clone()],
                        vec![edge("net-receive", "rec")],
                    ),
                ]
            } else {
                vec![session(
                    "network-route",
                    vec![tone.clone(), send.clone(), receive.clone(), rec.clone()],
                    vec![edge("tone", "net-send"), edge("net-receive", "rec")],
                )]
            };
            let mut planes = sessions
                .into_iter()
                .map(|session| {
                    let id = session.id.clone();
                    let mut plane = ControlPlane::default();
                    plane.insert_session(session).unwrap();
                    plane.configure_recording_root(&root).unwrap();
                    (plane, id)
                })
                .collect::<Vec<_>>();
            let call = |plane: &mut ControlPlane, method: &str, params: Value| {
                let response = plane.dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(1)),
                    method: method.into(),
                    params: Some(params),
                });
                assert!(response.error.is_none(), "{method}: {:?}", response.error);
                response.result.unwrap()
            };
            // Receiver first, as on two computers it is usually already playing.
            for (plane, id) in planes.iter_mut().rev() {
                call(plane, "nativePaths.prepare", json!({ "sessionId": id }));
                call(
                    plane,
                    "session.start",
                    json!({ "sessionId": id, "idempotencyKey": "net-start" }),
                );
            }
            let service = |planes: &mut Vec<(ControlPlane, EntityId)>, seconds: f64| {
                let until = Instant::now() + Duration::from_secs_f64(seconds);
                while Instant::now() < until {
                    for (plane, _) in planes.iter_mut() {
                        plane.service_running_native_audio(Instant::now());
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
            };
            // Play the tone (as the node's Play button does), let the jitter
            // buffer fill, then record three seconds.
            let sender = 0;
            let receiver = planes.len() - 1;
            let (plane, id) = &mut planes[sender];
            call(
                plane,
                "audioSources.transport",
                json!({ "sessionId": id, "nodeId": "tone", "action": "play" }),
            );
            service(&mut planes, 0.5);
            let (plane, id) = &mut planes[receiver];
            let started = call(
                plane,
                "recorders.startRecording",
                json!({ "sessionId": id, "nodeId": "rec", "idempotencyKey": "net-rec" }),
            );
            let path = std::path::PathBuf::from(started["path"].as_str().unwrap());
            service(&mut planes, 3.0);
            let telemetry = |plane: &mut ControlPlane, node_id: &str| {
                let diagnostics = call(plane, "system.diagnostics", json!({}));
                diagnostics["nodeTelemetry"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|item| item["nodeId"] == node_id)
                    .map(|item| item["network"].clone())
                    .unwrap_or(Value::Null)
            };
            let send = telemetry(&mut planes[sender].0, "net-send");
            let receive = telemetry(&mut planes[receiver].0, "net-receive");
            // The window names the addresses each computer must use.
            assert_eq!(receive["thisAddress"], "127.0.0.1", "{receive}");
            assert_eq!(send["localAddress"], "127.0.0.1", "{send}");
            assert!(send.get("lastErrorCode").is_none(), "{send}");
            // The network log summarizes both sides from the same counters.
            let summaries = planes
                .iter()
                .flat_map(|(plane, id)| {
                    plane.write_network_summaries(
                        id,
                        &mut network_log::Sampler::default(),
                        Instant::now(),
                    )
                })
                .collect::<Vec<_>>();
            let summary = |role: &str| {
                summaries
                    .iter()
                    .find(|record| record["role"] == role)
                    .cloned()
                    .unwrap_or(Value::Null)
            };
            let (send_summary, receive_summary) = (summary("send"), summary("receive"));
            assert!(
                send_summary["sentPackets"].as_u64().unwrap_or(0) > 100,
                "{send_summary}"
            );
            assert_eq!(send_summary["sendErrors"], 0, "{send_summary}");
            assert!(
                send_summary["localAddress"]
                    .as_str()
                    .is_some_and(|address| address.contains(':')),
                "{send_summary}"
            );
            assert!(
                receive_summary["receivedPackets"].as_u64().unwrap_or(0) > 100,
                "{receive_summary}"
            );
            assert_eq!(receive_summary["rejectedDatagrams"], 0, "{receive_summary}");
            assert!(
                receive_summary["hint"].is_null(),
                "a healthy stream has no hint: {receive_summary}"
            );
            let (plane, id) = &mut planes[receiver];
            let stopped = call(
                plane,
                "recorders.stopRecording",
                json!({ "sessionId": id, "nodeId": "rec", "idempotencyKey": "net-rec-stop" }),
            );
            for (plane, id) in planes.iter_mut() {
                call(
                    plane,
                    "session.stop",
                    json!({ "sessionId": id, "idempotencyKey": "net-stop" }),
                );
            }
            let layout = if two_computers {
                "two backends"
            } else {
                "one session"
            };
            eprintln!("{layout}: send {send}\nreceive {receive}\nstopped {stopped}");
            assert_eq!(stopped["state"], "completed", "{layout}: {stopped}");
            assert!(
                send["sentPackets"].as_u64().unwrap_or(0) > 1_000,
                "{layout}: sender telemetry: {send}"
            );
            assert_eq!(send["droppedPackets"], 0, "{layout}: {send}");
            assert!(
                receive["receivedPackets"].as_u64().unwrap_or(0) > 1_000,
                "{layout}: receiver telemetry: {receive}"
            );
            assert_eq!(receive["rejectedDatagrams"], 0, "{layout}: {receive}");
            assert_eq!(receive["lostPackets"], 0, "{layout}: {receive}");
            // The recorded tone: ~3 s, continuous (no gap, no repeat, no step).
            assert_playable_wav(&path);
            let bytes = std::fs::read(&path).unwrap();
            let data = bytes
                .windows(4)
                .position(|window| window == b"data")
                .unwrap()
                + 8;
            let left = bytes[data..]
                .chunks_exact(8)
                .map(|frame| f32::from_le_bytes(frame[..4].try_into().unwrap()))
                .collect::<Vec<_>>();
            assert!(
                left.len() > 48_000 * 2,
                "{layout}: recorded {} frames",
                left.len()
            );
            let peak = left
                .iter()
                .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
            assert!(
                (peak - 10_f32.powf(-12.0 / 20.0)).abs() < 0.02,
                "{layout}: level preserved over the network: {peak}"
            );
            let start = left
                .iter()
                .position(|sample| sample.abs() > peak / 2.0)
                .unwrap();
            let coefficient = (2.0 * (2.0 * std::f64::consts::PI * 997.0 / 48_000.0).cos()) as f32;
            let steps = left[start..]
                .windows(3)
                .filter(|window| {
                    (window[2] - (coefficient * window[1] - window[0])).abs() > peak * 0.05
                })
                .count();
            assert_eq!(steps, 0, "{layout}: the received tone is continuous");
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Two computers set up with mistakes, corrected while playing: the
    /// receiver expects the wrong sending computer and the sender uses the
    /// wrong port. Telemetry names the real sender, and fixing either
    /// setting (as the UI's auto-save or a StreamDeck `nodes.set` does)
    /// takes effect at once, without Stop/Play.
    #[cfg(windows)]
    #[test]
    fn network_settings_corrected_while_playing_take_effect_without_restart() {
        let send_port = std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let wrong_port = std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let port = |name: &str, direction| Port {
            name: name.into(),
            direction,
            channels: 2,
        };
        let node = |id: &str, kind, ports, parameters: Value| Node {
            id: EntityId::new(id),
            kind,
            type_version: 1,
            name: id.into(),
            enabled: true,
            bypass: false,
            parameters: serde_json::from_value(parameters).unwrap(),
            ports,
        };
        let edge = |from: &str, to: &str| audiorouter_domain::Edge {
            id: EntityId::new(format!("{from}-{to}")),
            source_node: EntityId::new(from),
            source_port: "out".into(),
            destination_node: EntityId::new(to),
            destination_port: "in".into(),
            matrix: vec![1.0, 0.0, 0.0, 1.0],
            enabled: true,
        };
        let session = |id: &str, nodes, edges| Session {
            id: EntityId::new(id),
            name: id.into(),
            schema_version: 1,
            revision: 0,
            nodes,
            edges,
        };
        let sending = session(
            "sending-pc",
            vec![
                node(
                    "tone",
                    NodeKind::TestSignal,
                    vec![port("out", PortDirection::Output)],
                    json!({ "frequencyHz": 997.0, "levelDb": -12.0, "durationMs": 600000.0 }),
                ),
                // Mistake 1: the receiving computer listens on another port.
                node(
                    "net-send",
                    NodeKind::NetworkSend,
                    vec![port("in", PortDirection::Input)],
                    json!({ "host": "127.0.0.1", "port": wrong_port }),
                ),
            ],
            vec![edge("tone", "net-send")],
        );
        let receiving = session(
            "receiving-pc",
            vec![
                // Mistake 2: the wrong sending computer's address.
                node(
                    "net-receive",
                    NodeKind::NetworkReceive,
                    vec![port("out", PortDirection::Output)],
                    json!({ "sender": "127.0.0.2", "port": send_port, "bufferMs": 40.0 }),
                ),
                node(
                    "rec",
                    NodeKind::Recorder,
                    vec![port("in", PortDirection::Input)],
                    json!({ "format": "wavFloat32" }),
                ),
            ],
            vec![edge("net-receive", "rec")],
        );
        let mut planes = [sending, receiving].map(|session| {
            let id = session.id.clone();
            let mut plane = ControlPlane::default();
            plane.insert_session(session).unwrap();
            (plane, id)
        });
        let call = |plane: &mut ControlPlane, method: &str, params: Value| {
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params: Some(params),
            });
            assert!(response.error.is_none(), "{method}: {:?}", response.error);
            response.result.unwrap()
        };
        for (plane, id) in planes.iter_mut() {
            call(plane, "nativePaths.prepare", json!({ "sessionId": id }));
            call(
                plane,
                "session.start",
                json!({ "sessionId": id, "idempotencyKey": "start" }),
            );
            call(
                plane,
                "sessions.active.set",
                json!({ "sessionId": id, "idempotencyKey": "active" }),
            );
        }
        let (plane, id) = &mut planes[0];
        call(
            plane,
            "audioSources.transport",
            json!({ "sessionId": id, "nodeId": "tone", "action": "play" }),
        );
        let service = |planes: &mut [(ControlPlane, EntityId); 2], seconds: f64| {
            let until = Instant::now() + Duration::from_secs_f64(seconds);
            while Instant::now() < until {
                for (plane, _) in planes.iter_mut() {
                    plane.service_running_native_audio(Instant::now());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        };
        let receive_telemetry = |planes: &mut [(ControlPlane, EntityId); 2]| {
            let diagnostics = call(&mut planes[1].0, "system.diagnostics", json!({}));
            diagnostics["nodeTelemetry"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["nodeId"] == "net-receive")
                .unwrap()["network"]
                .clone()
        };
        service(&mut planes, 0.5);
        assert_eq!(
            receive_telemetry(&mut planes)["receivedPackets"],
            0,
            "nothing reaches the wrong port"
        );
        // Fix the sender's port while playing.
        let (plane, _) = &mut planes[0];
        let fixed = call(
            plane,
            "nodes.set",
            json!({ "node": "net-send", "parameters": { "port": send_port }, "idempotencyKey": "fix-port" }),
        );
        assert_ne!(
            fixed["activation"]["native"]["state"], "restartRequired",
            "{fixed}"
        );
        service(&mut planes, 0.5);
        let waiting = receive_telemetry(&mut planes);
        assert_eq!(
            waiting["receivedPackets"], 0,
            "still the wrong sender address: {waiting}"
        );
        assert!(
            waiting["rejectedDatagrams"].as_u64().unwrap_or(0) > 50,
            "{waiting}"
        );
        assert_eq!(
            waiting["rejectedFrom"], "127.0.0.1",
            "the real sender is named: {waiting}"
        );
        // Use the named address while playing (the UI's one-click fix).
        let (plane, _) = &mut planes[1];
        let fixed = call(
            plane,
            "nodes.set",
            json!({ "node": "net-receive", "parameters": { "sender": "127.0.0.1" }, "idempotencyKey": "fix-sender" }),
        );
        assert_ne!(
            fixed["activation"]["native"]["state"], "restartRequired",
            "{fixed}"
        );
        service(&mut planes, 1.0);
        let receiving = receive_telemetry(&mut planes);
        eprintln!("after both fixes: {receiving}");
        assert!(
            receiving["receivedPackets"].as_u64().unwrap_or(0) > 200,
            "audio flows after the live fix: {receiving}"
        );
        assert!(
            receiving.get("rejectedFrom").is_none(),
            "the old hint is cleared: {receiving}"
        );
        // A port change on the receiver opens a new socket, also live.
        let new_port = std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let (plane, _) = &mut planes[1];
        call(
            plane,
            "nodes.set",
            json!({ "node": "net-receive", "parameters": { "port": new_port }, "idempotencyKey": "move-port" }),
        );
        let (plane, _) = &mut planes[0];
        call(
            plane,
            "nodes.set",
            json!({ "node": "net-send", "parameters": { "port": new_port }, "idempotencyKey": "follow-port" }),
        );
        service(&mut planes, 1.0);
        let moved = receive_telemetry(&mut planes);
        assert!(
            moved["receivedPackets"].as_u64().unwrap_or(0) > 200,
            "audio follows the new port: {moved}"
        );
        // P2-5: pair the receiver while playing. The unpaired sender is now
        // refused (counted, never played) until it gets the same key.
        const PAIRING_KEY: &str = "K7QW2X9MPAIRSTUDIO4HJ8NV";
        let (plane, _) = &mut planes[1];
        call(
            plane,
            "nodes.set",
            json!({ "node": "net-receive", "parameters": { "pairingKey": PAIRING_KEY }, "idempotencyKey": "pair-receive" }),
        );
        service(&mut planes, 0.3);
        let before = receive_telemetry(&mut planes)["receivedPackets"]
            .as_u64()
            .unwrap_or(0);
        service(&mut planes, 0.5);
        let refused = receive_telemetry(&mut planes);
        assert_eq!(refused["paired"], true, "{refused}");
        assert_eq!(
            refused["receivedPackets"].as_u64().unwrap_or(0),
            before,
            "nothing unpaired is played: {refused}"
        );
        assert!(
            refused["authFailures"].as_u64().unwrap_or(0) > 100,
            "{refused}"
        );
        assert_eq!(refused["authProblem"], "senderNotPaired", "{refused}");
        let (plane, _) = &mut planes[0];
        call(
            plane,
            "nodes.set",
            json!({ "node": "net-send", "parameters": { "pairingKey": PAIRING_KEY }, "idempotencyKey": "pair-send" }),
        );
        service(&mut planes, 1.0);
        let paired = receive_telemetry(&mut planes);
        assert!(
            paired["receivedPackets"].as_u64().unwrap_or(0) > before + 200,
            "paired audio flows: {paired}"
        );
        assert_eq!(paired["replayedPackets"], 0, "{paired}");
        for (plane, _) in planes.iter_mut() {
            let diagnostics = call(plane, "system.diagnostics", json!({}));
            assert!(
                !diagnostics.to_string().contains(PAIRING_KEY),
                "diagnostics never carry the key"
            );
        }
        for (plane, id) in planes.iter_mut() {
            call(
                plane,
                "session.stop",
                json!({ "sessionId": id, "idempotencyKey": "stop" }),
            );
        }
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "live Windows audio devices and plugins"]
    fn live_native_paths_start_pump_and_report_signal_timing() {
        let Some(database) = std::env::var_os("AUDIOROUTER_LIVE_PATHS_DATABASE") else {
            return;
        };
        let session_id = std::env::var("AUDIOROUTER_LIVE_PATHS_SESSION")
            .unwrap_or_else(|_| "patrick-main-session".into());
        let storage = audiorouter_storage::Storage::open(std::path::Path::new(&database)).unwrap();
        let mut plane = ControlPlane::with_storage("live-paths", storage);
        // Idempotency survives database copies and backend restart. Each
        // qualification is a new operation, rather than replaying a previous
        // test's successful start response on a currently stopped backend.
        let run_id = format!(
            "live-paths-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let mut call = |method: &str, params: Value| {
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params: (!params.is_null()).then_some(params),
            });
            assert!(response.error.is_none(), "{method}: {:?}", response.error);
            response.result.unwrap()
        };
        call(
            "safety.setPrivacyMute",
            json!({ "muted": true, "idempotencyKey": format!("{run_id}-mute") }),
        );
        let prepared = call("nativePaths.prepare", json!({ "sessionId": session_id }));
        eprintln!("prepared: {prepared}");
        let started = call(
            "session.start",
            json!({ "sessionId": session_id, "idempotencyKey": format!("{run_id}-start") }),
        );
        eprintln!("started: {started}");
        assert_eq!(started["runtime"], "native");
        let generation = started["generation"].as_u64().unwrap();
        let until = Instant::now() + Duration::from_secs(4);
        let mut delivered = 0_u64;
        while Instant::now() < until {
            let pump = call(
                "nativeMultiInputs.pump",
                json!({ "sessionId": session_id, "generation": generation, "maxPackets": 64 }),
            );
            delivered += pump["deliveredQuanta"].as_u64().unwrap_or(0);
            std::thread::sleep(Duration::from_millis(10));
        }
        let diagnostics = call("system.diagnostics", Value::Null);
        if let Some(path) = std::env::var_os("AUDIOROUTER_LIVE_DIAGNOSTICS_DUMP") {
            std::fs::write(path, serde_json::to_vec_pretty(&diagnostics).unwrap()).unwrap();
        }
        for item in diagnostics["nodeTelemetry"].as_array().unwrap() {
            eprintln!(
                "{} timing={} plugin={}",
                item["nodeId"], item["timing"], item["plugin"]
            );
            if let Some(levels) = item["spectrum"]["levelsDb"].as_array() {
                eprintln!(
                    "{} spectrum (first 8 of {} bands, dB): {:?}",
                    item["nodeId"],
                    levels.len(),
                    &levels[..8.min(levels.len())]
                );
            }
        }
        eprintln!("delivered branch blocks: {delivered}");
        call(
            "session.stop",
            json!({ "sessionId": session_id, "idempotencyKey": format!("{run_id}-stop") }),
        );
        let timed = diagnostics["nodeTelemetry"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["timing"]["delayMs"].is_number())
            .count();
        assert!(
            timed > 0,
            "signal timing is reported while the session runs"
        );
        let failed_plugins = diagnostics["nodeTelemetry"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["plugin"].is_object() && item["plugin"]["state"] != "running")
            .map(|item| item["nodeId"].clone())
            .collect::<Vec<_>>();
        assert!(
            failed_plugins.is_empty(),
            "plugins failed while running: {failed_plugins:?}"
        );
        assert!(delivered > 0, "both paths must deliver audio blocks");
        if session_id == "patrick-main-session" {
            // The session's consecutive ReaPlugs share one worker.
            let session = EntityId::new(&session_id);
            let plugin_ids = plane
                .get_session(&session)
                .unwrap()
                .nodes
                .iter()
                .filter(|node| node.kind == NodeKind::Plugin && node.enabled)
                .map(|node| node.id.clone())
                .collect::<Vec<_>>();
            let members = plugin_ids
                .iter()
                .map(|id| plane.plugin_bridge(&session, id).unwrap())
                .collect::<Vec<_>>();
            assert!(
                members
                    .iter()
                    .all(|member| member.shares_worker_with(&members[0])),
                "Patrick's ReaPlugs must share one worker"
            );
            eprintln!(
                "verified shared plugin worker: {}",
                plugin_ids
                    .iter()
                    .map(EntityId::as_str)
                    .collect::<Vec<_>>()
                    .join(" -> ")
            );
        }
    }

    /// Qualify the exact saved Meter topology without writing the user DB or
    /// recording audio. All test-owned output is privacy-muted. The caller
    /// supplies an API session snapshot and must ensure endpoints are free.
    #[cfg(windows)]
    #[test]
    #[ignore = "live exact Windows endpoints; privacy-muted isolated session"]
    fn live_native_meter_saved_route_starts_pumps_and_resets() {
        let Some(path) = std::env::var_os("AUDIOROUTER_LIVE_METER_SESSION_JSON") else {
            return;
        };
        let text = std::fs::read_to_string(path).unwrap();
        let session: Session = serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap();
        assert!(
            session
                .nodes
                .iter()
                .all(|node| node.kind != NodeKind::Plugin),
            "this fixture is native-only"
        );
        let session_id = session.id.clone();
        let meter_id = session
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Meter)
            .expect("fixture contains a Meter")
            .id
            .clone();
        let mut plane = ControlPlane::default();
        plane.insert_session(session.clone()).unwrap();
        let key = format!("meter-{}", std::process::id());
        let request = |method: &str, params: Value| JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params: Some(params),
        };
        assert!(plane
            .dispatch(request(
                "safety.setPrivacyMute",
                json!({"muted":true,"idempotencyKey":format!("{key}-mute")})
            ))
            .error
            .is_none());
        let prepared = plane.dispatch(request(
            "nativePaths.prepare",
            json!({"sessionId":session_id}),
        ));
        assert!(prepared.error.is_none(), "prepare: {:?}", prepared.error);
        let started = plane.dispatch(request(
            "session.start",
            json!({"sessionId":session_id,"idempotencyKey":format!("{key}-start")}),
        ));
        assert!(started.error.is_none(), "start: {:?}", started.error);
        let generation = started.result.unwrap()["generation"].as_u64().unwrap();
        let deadline = Instant::now() + Duration::from_secs(4);
        let mut delivered = 0;
        while Instant::now() < deadline {
            let pump = plane.dispatch(request(
                "nativeMultiInputs.pump",
                json!({"sessionId":session_id,"generation":generation,"maxPackets":64}),
            ));
            assert!(pump.error.is_none(), "pump: {:?}", pump.error);
            delivered += pump.result.unwrap()["deliveredQuanta"]
                .as_u64()
                .unwrap_or(0);
            std::thread::sleep(Duration::from_millis(2));
        }
        let diagnostics = plane
            .dispatch(request("system.diagnostics", json!({})))
            .result
            .unwrap();
        let meter = diagnostics["nodeTelemetry"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["nodeId"] == json!(meter_id))
            .expect("native Meter telemetry");
        assert!(meter["meter"]["observedFrames"].as_u64().unwrap() > 0);
        let reset = plane.dispatch(request(
            "meters.reset",
            json!({"sessionId":session_id,"nodeId":meter_id}),
        ));
        assert!(reset.error.is_none(), "reset: {:?}", reset.error);
        assert_eq!(reset.result.unwrap()["reset"], true);
        let after = plane
            .dispatch(request("system.diagnostics", json!({})))
            .result
            .unwrap();
        let meter = after["nodeTelemetry"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["nodeId"] == json!(meter_id))
            .unwrap();
        assert_eq!(meter["meter"]["observedFrames"], 0);
        assert_eq!(
            plane.get_session(&session_id).unwrap(),
            &session,
            "runtime reset does not save or alter graph"
        );
        assert!(plane
            .dispatch(request(
                "session.stop",
                json!({"sessionId":session_id,"idempotencyKey":format!("{key}-stop")})
            ))
            .error
            .is_none());
        assert!(delivered > 0);
        eprintln!("native Meter route prepared, started, delivered {delivered} branch blocks, exposed frames, reset and stopped; saved graph unchanged");
    }

    /// A plugin setting changed while playing (as in its editor) must come
    /// back after Stop and Play. Same environment as the test above; the
    /// session must contain a plugin node `AUDIOROUTER_LIVE_PLUGIN_NODE`
    /// (default `reaeq`). Privacy mute keeps it silent.
    #[cfg(windows)]
    #[test]
    #[ignore = "live Windows audio devices and plugins"]
    fn live_plugin_settings_survive_stop_and_play() {
        let Some(database) = std::env::var_os("AUDIOROUTER_LIVE_PATHS_DATABASE") else {
            return;
        };
        let session_id = std::env::var("AUDIOROUTER_LIVE_PATHS_SESSION")
            .unwrap_or_else(|_| "patrick-main-session".into());
        let node_id = EntityId::new(
            std::env::var("AUDIOROUTER_LIVE_PLUGIN_NODE").unwrap_or_else(|_| "reaeq".into()),
        );
        let storage = audiorouter_storage::Storage::open(std::path::Path::new(&database)).unwrap();
        let mut plane = ControlPlane::with_storage("live-plugin-state", storage);
        let run_id = format!(
            "live-plugin-state-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let call = |plane: &mut ControlPlane, method: &str, params: Value| {
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params: (!params.is_null()).then_some(params),
            });
            assert!(response.error.is_none(), "{method}: {:?}", response.error);
            response.result.unwrap()
        };
        let play = |plane: &mut ControlPlane, run: &str| {
            call(
                plane,
                "nativePaths.prepare",
                json!({ "sessionId": session_id }),
            );
            let started = call(
                plane,
                "session.start",
                json!({ "sessionId": session_id, "idempotencyKey": format!("{run_id}-{run}-start") }),
            );
            let generation = started["generation"].as_u64().unwrap();
            let until = Instant::now() + Duration::from_millis(800);
            while Instant::now() < until {
                call(
                    plane,
                    "nativeMultiInputs.pump",
                    json!({ "sessionId": session_id, "generation": generation, "maxPackets": 64 }),
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            generation
        };
        call(
            &mut plane,
            "safety.setPrivacyMute",
            json!({ "muted": true, "idempotencyKey": format!("{run_id}-mute") }),
        );
        let generation = play(&mut plane, "first");
        let session = EntityId::new(&session_id);
        let bridge = plane.plugin_bridge(&session, &node_id).unwrap();
        let before = bridge.save_state().unwrap();
        // Change settings the way an editor does: on the running instance only.
        // The values differ on every run, so a restored earlier run's values
        // never turn the change into a no-op.
        let seed = 0.1
            + std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_millis() as f32
                / 1000.0
                * 0.6;
        bridge
            .set_parameters(
                (0..4)
                    .map(|parameter_id| audiorouter_plugin_host::ParameterEvent {
                        parameter_id,
                        normalized_value: (seed + parameter_id as f32 * 0.05).min(0.95),
                        sample_offset: 0,
                    })
                    .collect(),
            )
            .unwrap();
        let until = Instant::now() + Duration::from_millis(500);
        while Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
            call(
                &mut plane,
                "nativeMultiInputs.pump",
                json!({ "sessionId": session_id, "generation": generation, "maxPackets": 64 }),
            );
        }
        let changed = bridge.save_state().unwrap();
        assert_ne!(
            changed.bytes, before.bytes,
            "the test must actually change the plugin's state"
        );
        drop(bridge);
        call(
            &mut plane,
            "session.stop",
            json!({ "sessionId": session_id, "idempotencyKey": format!("{run_id}-first-stop") }),
        );
        call(
            &mut plane,
            "nativeEndpoints.detach",
            json!({ "sessionId": session_id }),
        );
        play(&mut plane, "second");
        let restored = plane
            .plugin_bridge(&session, &node_id)
            .unwrap()
            .save_state()
            .unwrap();
        call(
            &mut plane,
            "session.stop",
            json!({ "sessionId": session_id, "idempotencyKey": format!("{run_id}-second-stop") }),
        );
        // Some plugins re-round a stored double when reloading (ReaEQ moves
        // one frequency by its last bit), so allow a byte or two of drift but
        // require the restore to match the change, not the original state.
        let differing = |left: &[u8], right: &[u8]| {
            left.iter().zip(right).filter(|(a, b)| a != b).count()
                + left.len().abs_diff(right.len())
        };
        assert!(
            differing(&restored.bytes, &changed.bytes) <= 2
                && differing(&restored.bytes, &before.bytes)
                    > differing(&restored.bytes, &changed.bytes),
            "Stop then Play must restore the plugin's settings: restored {:?}, changed {:?}",
            restored.bytes,
            changed.bytes
        );
        eprintln!(
            "verified {} settings survive Stop and Play ({} state bytes)",
            node_id.as_str(),
            restored.bytes.len()
        );
    }

    #[cfg(windows)]
    #[test]
    fn native_paths_read_a_mono_endpoint_through_a_stereo_device_node() {
        let stereo = |name: &str, direction| Port {
            name: name.into(),
            direction,
            channels: 2,
        };
        let node = |id: &str, kind, ports: Vec<Port>| Node {
            id: EntityId::new(id),
            kind,
            type_version: 1,
            name: id.into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports,
        };
        let session_id = EntityId::new("mono-mic");
        let mut plane = ControlPlane::default();
        plane
            .insert_session(Session {
                id: session_id.clone(),
                name: "mono mic".into(),
                schema_version: 1,
                revision: 0,
                nodes: vec![
                    node(
                        "mic",
                        NodeKind::PhysicalInput,
                        vec![stereo("out", PortDirection::Output)],
                    ),
                    node(
                        "cable-a",
                        NodeKind::PhysicalOutput,
                        vec![stereo("in", PortDirection::Input)],
                    ),
                ],
                edges: vec![Edge {
                    id: EntityId::new("mic-cable-a"),
                    source_node: EntityId::new("mic"),
                    source_port: "out".into(),
                    destination_node: EntityId::new("cable-a"),
                    destination_port: "in".into(),
                    matrix: vec![1.0, 0.0, 0.0, 1.0],
                    enabled: true,
                }],
            })
            .unwrap();
        plane.native_multi_input_mono_nodes = vec![EntityId::new("mic")];
        let adapted = plane.native_paths_session(&session_id).unwrap();
        assert_eq!(adapted.nodes[0].ports[0].channels, 1);
        assert_eq!(adapted.edges[0].matrix, vec![1.0, 1.0]);
        assert!(audiorouter_domain::validate_session(&adapted).is_ok());
        // The saved session keeps its stereo device node.
        assert_eq!(
            plane.get_session(&session_id).unwrap().nodes[0].ports[0].channels,
            2
        );
    }

    #[cfg(windows)]
    #[test]
    fn native_multi_input_preparation_validates_mixed_physical_and_application_sources() {
        let session_id = EntityId::new("mixed-multi-input");
        let session = Session {
            id: session_id.clone(),
            name: "mixed multi-input".into(),
            schema_version: 1,
            revision: 0,
            nodes: vec![
                Node {
                    id: EntityId::new("mic"),
                    kind: NodeKind::PhysicalInput,
                    type_version: 1,
                    name: "Microphone".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Output,
                        channels: 2,
                    }],
                },
                Node {
                    id: EntityId::new("app"),
                    kind: NodeKind::ApplicationCapture,
                    type_version: 1,
                    name: "Zoom capture".into(),
                    enabled: true,
                    bypass: false,
                    parameters: [
                        ("executable".to_string(), json!("Zoom.exe")),
                        ("processPolicy".to_string(), json!("selectedInstance")),
                        ("processId".to_string(), json!(4242u64)),
                        ("creationTime100ns".to_string(), json!("999999999")),
                    ]
                    .into_iter()
                    .collect(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Output,
                        channels: 2,
                    }],
                },
                Node {
                    id: EntityId::new("mixer"),
                    kind: NodeKind::Mixer,
                    type_version: 1,
                    name: "Mixer".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![
                        Port {
                            name: "in".into(),
                            direction: PortDirection::Input,
                            channels: 2,
                        },
                        Port {
                            name: "out".into(),
                            direction: PortDirection::Output,
                            channels: 2,
                        },
                    ],
                },
                Node {
                    id: EntityId::new("output-a"),
                    kind: NodeKind::PhysicalOutput,
                    type_version: 1,
                    name: "Output A".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Input,
                        channels: 2,
                    }],
                },
                Node {
                    id: EntityId::new("output-b"),
                    kind: NodeKind::PhysicalOutput,
                    type_version: 1,
                    name: "Output B".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Input,
                        channels: 2,
                    }],
                },
            ],
            edges: vec![
                Edge {
                    id: EntityId::new("mic-mixer"),
                    source_node: EntityId::new("mic"),
                    source_port: "main".into(),
                    destination_node: EntityId::new("mixer"),
                    destination_port: "in".into(),
                    matrix: vec![1.0, 0.0, 0.0, 1.0],
                    enabled: true,
                },
                Edge {
                    id: EntityId::new("app-mixer"),
                    source_node: EntityId::new("app"),
                    source_port: "main".into(),
                    destination_node: EntityId::new("mixer"),
                    destination_port: "in".into(),
                    matrix: vec![1.0, 0.0, 0.0, 1.0],
                    enabled: true,
                },
                Edge {
                    id: EntityId::new("mixer-output-a"),
                    source_node: EntityId::new("mixer"),
                    source_port: "out".into(),
                    destination_node: EntityId::new("output-a"),
                    destination_port: "main".into(),
                    matrix: vec![1.0, 0.0, 0.0, 1.0],
                    enabled: true,
                },
                Edge {
                    id: EntityId::new("mixer-output-b"),
                    source_node: EntityId::new("mixer"),
                    source_port: "out".into(),
                    destination_node: EntityId::new("output-b"),
                    destination_port: "main".into(),
                    matrix: vec![1.0, 0.0, 0.0, 1.0],
                    enabled: true,
                },
            ],
        };
        let mic_endpoint = audiorouter_windows_audio::EndpointInfo {
            id: "mic-endpoint".into(),
            direction: audiorouter_windows_audio::EndpointDirection::Capture,
            default_period_100ns: 100_000,
            minimum_period_100ns: 30_000,
            sample_rate_hz: audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ,
            channels: 2,
            bits_per_sample: 32,
            format_tag: 3,
            channel_mask: 3,
            subformat_guid: "00000003-0000-0010-8000-00aa00389b71".into(),
        };

        // A binding kind that does not match the node kind at that graph
        // position is rejected before any device or process is opened.
        let mut plane = ControlPlane::default();
        plane.insert_session(session.clone()).unwrap();
        let swapped_kind_error = plane
            .prepare_native_multi_input_worker(
                session_id.clone(),
                1,
                &[
                    NativeMultiInputSourceBinding::Application {
                        process_id: 4242,
                        expected_executable: "Zoom.exe",
                        expected_executable_path: None,
                        expected_creation_time_100ns: 999_999_999,
                        mode: audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
                    },
                    NativeMultiInputSourceBinding::Physical(&mic_endpoint),
                ],
                0,
                1,
                0,
            )
            .unwrap_err();
        assert!(matches!(
            swapped_kind_error,
            ControlError::InvalidRequest(message) if message.contains("physical inputs or application captures")
        ));

        // An application binding with an identity that does not match the
        // enabled node's persisted identity is rejected the same way.
        let mut plane = ControlPlane::default();
        plane.insert_session(session.clone()).unwrap();
        let mismatched_identity_error = plane
            .prepare_native_multi_input_worker(
                session_id.clone(),
                1,
                &[
                    NativeMultiInputSourceBinding::Physical(&mic_endpoint),
                    NativeMultiInputSourceBinding::Application {
                        process_id: 9999,
                        expected_executable: "Zoom.exe",
                        expected_executable_path: None,
                        expected_creation_time_100ns: 999_999_999,
                        mode: audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
                    },
                ],
                0,
                1,
                0,
            )
            .unwrap_err();
        assert!(matches!(
            mismatched_identity_error,
            ControlError::InvalidRequest(message) if message.contains("does not match the enabled node's identity")
        ));

        // A correctly matching application binding passes identity
        // validation and proceeds to open a real process-loopback capture,
        // which fails in this test environment for audio/process reasons
        // rather than an identity or kind mismatch.
        let mut plane = ControlPlane::default();
        plane.insert_session(session).unwrap();
        let progressed_error = plane
            .prepare_native_multi_input_worker(
                session_id,
                1,
                &[
                    NativeMultiInputSourceBinding::Physical(&mic_endpoint),
                    NativeMultiInputSourceBinding::Application {
                        process_id: 4242,
                        expected_executable: "Zoom.exe",
                        expected_executable_path: None,
                        expected_creation_time_100ns: 999_999_999,
                        mode: audiorouter_windows_audio::ProcessLoopbackMode::IncludeTargetTree,
                    },
                ],
                0,
                1,
                0,
            )
            .unwrap_err();
        assert!(
            !matches!(progressed_error, ControlError::InvalidRequest(ref message) if message.contains("does not match")
                || message.contains("physical inputs or application captures")),
            "expected validation to pass and fail later while opening the real source, got {progressed_error:?}"
        );
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "requires explicit multi-capture/render endpoint IDs and AUDIOROUTER_ALLOW_LIVE_AUDIO=1"]
    fn guarded_live_native_multi_input_many_output_lifecycle() {
        if std::env::var("AUDIOROUTER_ALLOW_LIVE_AUDIO").as_deref() != Ok("1") {
            return;
        }
        let capture_ids = std::env::var("AUDIOROUTER_MULTI_CAPTURE_ENDPOINT_IDS")
            .expect("AUDIOROUTER_MULTI_CAPTURE_ENDPOINT_IDS is required")
            .split('|')
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let render_ids = std::env::var("AUDIOROUTER_MULTI_RENDER_ENDPOINT_IDS")
            .expect("AUDIOROUTER_MULTI_RENDER_ENDPOINT_IDS is required")
            .split('|')
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert!(capture_ids.len() >= 2 && capture_ids.len() <= 8);
        assert!(render_ids.len() >= 2 && render_ids.len() <= 8);
        let endpoints = audiorouter_windows_audio::enumerate_active_endpoints().unwrap();
        let captures = capture_ids
            .iter()
            .map(|id| {
                endpoints
                    .iter()
                    .find(|endpoint| {
                        endpoint.id == *id
                            && endpoint.direction
                                == audiorouter_windows_audio::EndpointDirection::Capture
                    })
                    .expect("configured capture endpoint is not an active exact match")
                    .clone()
            })
            .collect::<Vec<_>>();
        let _renders = render_ids
            .iter()
            .map(|id| {
                endpoints
                    .iter()
                    .find(|endpoint| {
                        endpoint.id == *id
                            && endpoint.direction
                                == audiorouter_windows_audio::EndpointDirection::Render
                    })
                    .expect("configured render endpoint is not an active exact match")
                    .clone()
            })
            .collect::<Vec<_>>();
        let mut plane = ControlPlane::default();
        let session_id = EntityId::new("guarded-live-native-multi");
        let owned = Session {
            id: session_id.clone(),
            name: "guarded multi-input many-output".into(),
            schema_version: 1,
            revision: 0,
            nodes: vec![
                Node {
                    id: EntityId::new("input-a"),
                    kind: NodeKind::PhysicalInput,
                    type_version: 1,
                    name: "Input A".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Output,
                        channels: 2,
                    }],
                },
                Node {
                    id: EntityId::new("input-b"),
                    kind: NodeKind::PhysicalInput,
                    type_version: 1,
                    name: "Input B".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Output,
                        channels: 2,
                    }],
                },
                Node {
                    id: EntityId::new("mixer"),
                    kind: NodeKind::Mixer,
                    type_version: 1,
                    name: "Mixer".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![
                        Port {
                            name: "in".into(),
                            direction: PortDirection::Input,
                            channels: 2,
                        },
                        Port {
                            name: "out".into(),
                            direction: PortDirection::Output,
                            channels: 2,
                        },
                    ],
                },
                Node {
                    id: EntityId::new("output-a"),
                    kind: NodeKind::PhysicalOutput,
                    type_version: 1,
                    name: "Output A".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Input,
                        channels: 2,
                    }],
                },
                Node {
                    id: EntityId::new("output-b"),
                    kind: NodeKind::PhysicalOutput,
                    type_version: 1,
                    name: "Output B".into(),
                    enabled: true,
                    bypass: false,
                    parameters: Default::default(),
                    ports: vec![Port {
                        name: "main".into(),
                        direction: PortDirection::Input,
                        channels: 2,
                    }],
                },
            ],
            edges: vec![
                Edge {
                    id: EntityId::new("input-a-mixer"),
                    source_node: EntityId::new("input-a"),
                    source_port: "main".into(),
                    destination_node: EntityId::new("mixer"),
                    destination_port: "in".into(),
                    matrix: vec![1.0, 0.0, 0.0, 1.0],
                    enabled: true,
                },
                Edge {
                    id: EntityId::new("input-b-mixer"),
                    source_node: EntityId::new("input-b"),
                    source_port: "main".into(),
                    destination_node: EntityId::new("mixer"),
                    destination_port: "in".into(),
                    matrix: vec![1.0, 0.0, 0.0, 1.0],
                    enabled: true,
                },
                Edge {
                    id: EntityId::new("mixer-output-a"),
                    source_node: EntityId::new("mixer"),
                    source_port: "out".into(),
                    destination_node: EntityId::new("output-a"),
                    destination_port: "main".into(),
                    matrix: vec![1.0, 0.0, 0.0, 1.0],
                    enabled: true,
                },
                Edge {
                    id: EntityId::new("mixer-output-b"),
                    source_node: EntityId::new("mixer"),
                    source_port: "out".into(),
                    destination_node: EntityId::new("output-b"),
                    destination_port: "main".into(),
                    matrix: vec![1.0, 0.0, 0.0, 1.0],
                    enabled: true,
                },
            ],
        };
        plane.insert_session(owned.clone()).unwrap();
        let bindings = captures
            .iter()
            .map(NativeMultiInputSourceBinding::Physical)
            .collect::<Vec<_>>();
        plane
            .prepare_native_multi_input_worker(session_id.clone(), 1, &bindings, 0, 3, 100)
            .unwrap();
        plane
            .prepare_native_output_fanout(session_id.clone(), 1, &render_ids)
            .unwrap();
        let started = plane.session_start(&session_id).unwrap();
        let generation = started["generation"].as_u64().unwrap();
        assert_eq!(generation, 1);
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        let mut captured_frames = 0_u64;
        let mut delivered_quanta = 0_u64;
        let mut rendered_frames = 0_u64;
        while std::time::Instant::now() < deadline {
            let pump = plane
                .pump_native_multi_input_worker(&session_id, generation, 64)
                .unwrap();
            captured_frames =
                captured_frames.saturating_add(pump["capturedFrames"].as_u64().unwrap());
            delivered_quanta =
                delivered_quanta.saturating_add(pump["deliveredQuanta"].as_u64().unwrap());
            rendered_frames =
                rendered_frames.saturating_add(pump["renderedFrames"].as_u64().unwrap());
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            captured_frames > 0,
            "multi-input worker did not capture frames"
        );
        assert!(
            delivered_quanta > 0,
            "multi-input worker delivered no quanta"
        );
        assert!(rendered_frames > 0, "multi-input worker rendered no frames");
        eprintln!(
            "guarded_native_multi capture_frames={captured_frames} delivered_quanta={delivered_quanta} rendered_frames={rendered_frames}"
        );
        let stopped = plane.session_stop(&session_id).unwrap();
        assert_eq!(stopped["state"], "stopped");
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
    fn test_signal_transport_controls_only_the_prepared_source() {
        let mut plane = ControlPlane::default();
        let owned = session();
        plane.insert_session(owned.clone()).unwrap();
        plane.session_start(&owned.id).unwrap();
        let node_id = EntityId::new("tone");
        let source = std::sync::Arc::new(audiorouter_engine::TestSignalSource::new(
            440.0, -18.0, 1_000.0, 48_000,
        ));
        plane.test_signal_sources.insert(
            (owned.id.clone(), node_id.clone()),
            std::sync::Arc::clone(&source),
        );
        let request =
            |action| json!({ "sessionId": owned.id, "nodeId": node_id, "action": action });
        assert_eq!(
            plane
                .dispatch_audio_source_transport(Some(request("status")))
                .unwrap()["state"],
            "stopped"
        );
        assert_eq!(
            plane
                .dispatch_audio_source_transport(Some(request("play")))
                .unwrap()["state"],
            "playing"
        );
        assert_eq!(
            plane
                .dispatch_audio_source_transport(Some(request("stop")))
                .unwrap()["state"],
            "stopped"
        );
        assert!(plane
            .dispatch_audio_source_transport(Some(request("pause")))
            .is_err());
        assert_eq!(
            plane.status_snapshot().unwrap()["activeSessionIds"][0],
            owned.id.as_str()
        );
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
        let events_subscribe_schema = method_input_schema("events.subscribe");
        assert_eq!(
            events_subscribe_schema["properties"]["categories"]["items"]["enum"],
            json!(STATE_CATEGORIES)
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
                && node["availability"]["status"] == "available"));
        assert!(description["nodeTypes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["type"] == "application-capture@1"
                && node["availability"]["status"] == "available"));
        assert!(description["nodeTypes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["type"] == "endpoint-loopback@1"
                && node["availability"]["status"] == "available"));
        assert!(description["nodeTypes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["type"] == "virtual-render-source@1"
                && node["availability"]["status"] == "unavailable"));
        assert_eq!(description["processors"].as_array().unwrap().len(), 18);
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
        assert_eq!(result.as_array().unwrap().len(), 18);
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
    fn plugin_placeholder_plan_requires_current_scan_evidence() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        let mut candidate = original.clone();
        candidate.nodes.push(Node {
            id: EntityId::new("plugin"),
            kind: NodeKind::Plugin,
            type_version: 1,
            name: "Unbound plugin".into(),
            enabled: false,
            bypass: false,
            parameters: [
                ("path".into(), json!("C:\\Plugins\\effect.dll")),
                ("format".into(), json!("vst2")),
                (
                    "fingerprint".into(),
                    json!("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"),
                ),
                ("classId".into(), json!("effect-class")),
            ]
            .into_iter()
            .collect(),
            ports: vec![
                Port {
                    name: "in".into(),
                    direction: PortDirection::Input,
                    channels: 1,
                },
                Port {
                    name: "out".into(),
                    direction: PortDirection::Output,
                    channels: 1,
                },
            ],
        });
        let error = plane
            .plan_graph(&original.id, original.revision, candidate)
            .unwrap_err();
        assert!(matches!(
            error,
            ControlError::InvalidRequest(message)
                if message.contains("current explicit scan")
        ));
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
            status["outputSchema"]["properties"]["audio"]["enum"],
            json!(["available", "unavailable"])
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
            "devices.bindingInvalidated",
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
            json!(["wav", "flac", "mp3"])
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
            id: Some(json!(8)),
            method: "events.subscribe".into(),
            params: Some(json!({ "categories": ["not-a-real-event"] })),
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
        assert_eq!(result["nativeAdapterKind"], Value::Null);
        assert_eq!(result["nativeSessionId"], Value::Null);
        assert_eq!(result["schedulerTelemetry"], Value::Null);
        assert_eq!(result["nodeTelemetry"], json!([]));
        assert_eq!(result["redacted"], true);
        assert!(result.get("path").is_none());
    }

    #[test]
    fn audio_status_names_the_attached_worker_kind() {
        assert_eq!(
            ControlPlane::audio_status_for("running", Some("endpoint")),
            ("available", "native endpoint audio is running")
        );
        assert_eq!(
            ControlPlane::audio_status_for("configured-stopped", Some("duplex")),
            (
                "unavailable",
                "native duplex worker is prepared but stopped; start a session explicitly"
            )
        );
        assert_eq!(
            ControlPlane::audio_status_for("running", Some("multi-input")),
            ("available", "native multi-input audio is running")
        );
        assert_eq!(
            ControlPlane::audio_status_for("unknown", None),
            (
                "unavailable",
                "audio is not prepared; in Devices, select exact capture and render endpoints, then Prepare native endpoints and Start session"
            )
        );
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
            "audio is not prepared; in Devices, select exact capture and render endpoints, then Prepare native endpoints and Start session"
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
    fn sleep_stops_fake_sessions_and_resume_requires_revalidation() {
        let mut plane = ControlPlane::default();
        let mut value = session();
        value.id = EntityId::new("sleep-route");
        let session_id = value.id.clone();
        plane.insert_session(value).unwrap();
        plane.session_start(&session_id).unwrap();

        let stopped = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!("sleep")),
                method: "system.osTransition".into(),
                params: Some(json!({
                    "transition": "sleep",
                    "idempotencyKey": "os-sleep-1"
                })),
            })
            .result
            .unwrap();
        assert_eq!(stopped["action"], "stopAndRelease");
        assert_eq!(
            plane.status_snapshot().unwrap()["activeSessionIds"],
            json!([])
        );

        let resumed = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!("resume")),
                method: "system.osTransition".into(),
                params: Some(json!({
                    "transition": "resume",
                    "idempotencyKey": "os-resume-1"
                })),
            })
            .result
            .unwrap();
        assert_eq!(resumed["action"], "revalidateBeforeRestart");
        assert_eq!(resumed["sessionIds"], json!(["sleep-route"]));
        assert_eq!(
            plane.status_snapshot().unwrap()["activeSessionIds"],
            json!([])
        );
    }

    #[test]
    fn os_transition_idempotency_replays_and_rejects_key_reuse() {
        let mut plane = ControlPlane::default();
        let mut value = session();
        value.id = EntityId::new("idempotent-os-transition");
        let session_id = value.id.clone();
        plane.insert_session(value).unwrap();
        plane.session_start(&session_id).unwrap();

        let request = |transition: &str| JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(transition)),
            method: "system.osTransition".into(),
            params: Some(json!({
                "transition": transition,
                "idempotencyKey": "os-transition-replay"
            })),
        };
        let first = plane.dispatch(request("sleep"));
        let first_result = first.result.clone().expect("initial transition succeeds");
        let replay = plane.dispatch(request("sleep"));
        assert_eq!(replay.result, Some(first_result));
        assert_eq!(
            plane.status_snapshot().unwrap()["activeSessionIds"],
            json!([])
        );

        let conflict = plane.dispatch(request("resume"));
        assert_eq!(
            conflict.error.as_ref().map(|error| error.code),
            Some(-32602)
        );
        assert!(conflict
            .error
            .as_ref()
            .is_some_and(|error| error.message.contains("idempotency")));
    }

    #[test]
    fn os_transition_refuses_to_stop_an_active_recording_implicitly() {
        let mut plane = ControlPlane::default();
        let value = session();
        let session_id = value.id.clone();
        plane.insert_session(value).unwrap();
        plane.session_start(&session_id).unwrap();
        let recorder = RecorderController::new();
        plane.recorders.insert(session_id.clone(), recorder);
        let recorder = plane.recorders.get_mut(&session_id).unwrap();
        recorder.arm().unwrap();
        recorder.start(0).unwrap();

        let result = plane.handle_os_transition(os_transition::OsTransition::Sleep);
        assert!(result.is_err());
        assert_eq!(
            plane.status_snapshot().unwrap()["activeSessionIds"],
            json!([session_id])
        );
    }

    #[test]
    fn sleep_preserves_native_identity_for_explicit_resume_revalidation() {
        let mut plane = ControlPlane::default();
        let value = session();
        let session_id = value.id.clone();
        plane.insert_session(value).unwrap();
        plane.session_start(&session_id).unwrap();
        // A marker is sufficient to model a native-owned route in this
        // portable test; no endpoint or driver is opened.
        plane.native_endpoint_session = Some(session_id.clone());

        plane
            .handle_os_transition(os_transition::OsTransition::Sleep)
            .unwrap();
        let resumed = plane
            .handle_os_transition(os_transition::OsTransition::Resume)
            .unwrap();
        assert_eq!(resumed["action"], "revalidateBeforeRestart");
        assert_eq!(resumed["nativeSessionIds"], json!([session_id]));
        assert_eq!(resumed["sessionIds"], json!([]));
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
    fn runtime_crash_recovery_does_not_restart_native_owned_sessions() {
        let mut plane = ControlPlane::default();
        let mut value = session();
        value.id = EntityId::new("native-owned");
        let session_id = value.id.clone();
        plane.insert_session(value).unwrap();
        plane.session_start(&session_id).unwrap();

        // A native worker is not constructible in this portable test, but the
        // ownership marker is enough to model the supervisor handoff. A stale
        // marker must fail closed too: recovery must not restart a route that
        // could still require native endpoint ownership.
        plane.native_endpoint_session = Some(session_id.clone());
        let decision = plane.recover_after_runtime_crash(100).unwrap();

        assert_eq!(decision.mode, RecoveryMode::RestoreEligible);
        assert!(decision.session_ids.is_empty());
        assert_eq!(
            plane.status_snapshot().unwrap()["activeSessionIds"],
            json!([])
        );
        assert!(plane.native_endpoint_session.is_none());
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
    fn memory_recovery_status_exposes_recent_crashes_and_safe_mode() {
        let mut plane = ControlPlane::default();
        let now = unix_epoch_seconds() as u64;
        plane.record_runtime_crash(now).unwrap();
        plane.record_runtime_crash(now + 1).unwrap();
        plane.record_runtime_crash(now + 2).unwrap();

        let status = plane.status_snapshot().unwrap();
        assert_eq!(status["recovery"]["recentCrashes"], 3);
        assert_eq!(status["recovery"]["safeMode"], true);
        assert_eq!(status["recovery"]["persistence"], "memory");
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
    fn recovery_clear_rpc_preserves_durable_latch_when_journal_is_full() {
        let storage = Storage::open_memory().unwrap();
        for index in 0..audiorouter_storage::MAX_OPERATION_JOURNAL_ENTRIES {
            storage
                .journal_commit(&format!("existing-recovery-{index}"), "test", "{}", 0)
                .unwrap();
        }
        let mut plane = ControlPlane::with_storage("recovery-full-journal", storage);
        let now = unix_epoch_seconds() as u64;
        plane.record_runtime_crash(now).unwrap();
        plane.record_runtime_crash(now + 1).unwrap();
        plane.record_runtime_crash(now + 2).unwrap();

        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(21)),
            method: "recovery.clearSafeMode".into(),
            params: Some(json!({ "idempotencyKey": "recovery-clear-full-journal" })),
        });
        assert!(response.error.is_some());
        let status = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(22)),
                method: "status.get".into(),
                params: None,
            })
            .result
            .unwrap();
        assert_eq!(status["recovery"]["safeMode"], true);
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
        // Regression for a desktop-shell tray/UI toggle that was silently
        // refused: the desktop shell grant must be sufficient on its own,
        // without also requiring Capture (which it never holds).
        let shell_toggled = first.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(20)),
                method: "safety.setPrivacyMute".into(),
                params: Some(json!({ "muted": false, "idempotencyKey": "privacy-2" })),
            },
            &ClientGrant::for_desktop_shell(),
        );
        assert_eq!(shell_toggled.result.unwrap()["muted"], false);
        let shell_relatched = first.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(20)),
                method: "safety.setPrivacyMute".into(),
                params: Some(json!({ "muted": true, "idempotencyKey": "privacy-3" })),
            },
            &ClientGrant::for_desktop_shell(),
        );
        assert_eq!(shell_relatched.result.unwrap()["muted"], true);
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
            "native sign-in registration is owned by the desktop shell"
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
        assert_eq!(plan["requiredScopes"], json!(["startupWrite"]));
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
            "native sign-in registration is owned by the desktop shell"
        );
        assert!(
            plane
                .dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(5)),
                    method: "startup.get".into(),
                    params: None,
                })
                .result
                .unwrap()["enabled"]
                .as_bool()
                == Some(true)
        );
        let replay = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "startup.apply".into(),
            params: Some(json!({ "planId": plan_id, "idempotencyKey": "startup-2" })),
        });
        assert!(replay.error.is_some());
        let retry = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "startup.apply".into(),
            params: Some(json!({ "planId": plan_id, "idempotencyKey": "startup-1" })),
        });
        assert_eq!(retry.result.unwrap()["state"], "unavailable");
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
        assert!(
            restarted
                .dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(5)),
                    method: "startup.get".into(),
                    params: None,
                })
                .result
                .unwrap()["enabled"]
                .as_bool()
                == Some(true)
        );
        assert!(restarted
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(3)),
                method: "startup.apply".into(),
                params: Some(json!({ "planId": plan_id, "idempotencyKey": "startup-replay" })),
            })
            .error
            .is_some());
        let retry = restarted.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "startup.apply".into(),
            params: Some(json!({ "planId": plan_id, "idempotencyKey": "startup-restart" })),
        });
        assert_eq!(retry.result.unwrap()["state"], "unavailable");
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
    fn one_hundred_durable_reconnects_advance_backend_epoch_monotonically() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-control-reconnect-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let mut previous_epoch = None;

        for index in 0..100 {
            let plane = ControlPlane::with_storage(
                format!("reconnect-{index}"),
                Storage::open(&path).unwrap(),
            );
            let epoch = plane.events.backend_epoch();
            if let Some(previous) = previous_epoch {
                assert_eq!(epoch, previous + 1);
            }
            previous_epoch = Some(epoch);
        }

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
                dither: false,
                conversion: "unknown".into(),
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
                dither: true,
                conversion: "targetSampleRate=44100;channels=2;bitsPerSample=16".into(),
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
        assert_eq!(result[0]["dither"], false);
        assert_eq!(result[0]["conversion"], "unknown");
        assert_eq!(result[1]["dither"], true);
        assert_eq!(
            result[1]["conversion"],
            "targetSampleRate=44100;channels=2;bitsPerSample=16"
        );
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
                dither: false,
                conversion: "unknown".into(),
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
                dither: false,
                conversion: "unknown".into(),
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
    fn verbose_diagnostics_switch_needs_session_control_and_reports_the_window() {
        let call = |method: &str, params: Option<Value>| JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: method.into(),
            params,
        };
        let mut plane = ControlPlane::default();
        let now = audiorouter_protocol::diagnostics::unix_time_ms();
        assert!(!plane.verbose_diagnostics_active(now));
        let observer = ClientGrant::for_role(ClientRole::Observer);
        let status = plane
            .dispatch_authorized(call("diagnostics.getVerbose", None), &observer)
            .result
            .unwrap();
        assert_eq!(status["enabled"], false);
        let denied = plane.dispatch_authorized(
            call("diagnostics.setVerbose", Some(json!({"enabled": true}))),
            &observer,
        );
        assert!(denied.error.is_some());
        assert!(!plane.verbose_diagnostics_active(now));
        let on = plane
            .dispatch_authorized(
                call("diagnostics.setVerbose", Some(json!({"enabled": true}))),
                &ClientGrant::for_desktop_shell(),
            )
            .result
            .unwrap();
        assert_eq!(on["enabled"], true);
        assert!(on["remainingSeconds"].as_u64().unwrap() > 3_590);
        assert_eq!(on["maxSeconds"], 3_600);
        assert!(plane.verbose_diagnostics_active(audiorouter_protocol::diagnostics::unix_time_ms()));
        assert!(!plane.verbose_diagnostics_active(on["expiresAtUnixMs"].as_u64().unwrap()));
        for bad in [
            json!({}),
            json!({"enabled": "yes"}),
            json!({"enabled": true, "path": "x"}),
        ] {
            assert!(plane
                .dispatch(call("diagnostics.setVerbose", Some(bad)))
                .error
                .is_some());
        }
        let off = plane
            .dispatch(call(
                "diagnostics.setVerbose",
                Some(json!({"enabled": false})),
            ))
            .result
            .unwrap();
        assert_eq!(off["enabled"], false);
        assert_eq!(off["expiresAtUnixMs"], Value::Null);
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
    fn native_pump_is_not_counted_as_a_user_mutation() {
        assert!(!rate_limit_method("nativeEndpoints.pump"));
        assert!(!rate_limit_method("nativeDuplex.pump"));
        assert!(rate_limit_method("graph.commit"));
        assert!(is_mutating_method("nativeEndpoints.pump"));
        assert!(is_mutating_method("nativeDuplex.pump"));
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
        let replay = plane.session_start(&original.id).unwrap();
        assert_eq!(replay["generation"], 1);
        assert_eq!(replay["runtime"], "fake");
        assert_eq!(
            plane.session_stop(&original.id).unwrap()["state"],
            "stopped"
        );
        let events = plane.events.since(0, 10).unwrap();
        assert_eq!(events.last().unwrap().resource_revision, original.revision);
        assert_eq!(plane.session_start(&original.id).unwrap()["generation"], 2);
    }

    #[test]
    fn system_quit_stops_running_sessions_and_replays_idempotently() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        plane.session_start(&original.id).unwrap();
        let request = || JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!("quit")),
            method: "system.quit".into(),
            params: Some(json!({ "idempotencyKey": "quit-once" })),
        };
        let grant = ClientGrant::for_role(ClientRole::Operator);
        let first = plane.dispatch_authorized_for_client(request(), "shell", &grant);
        assert_eq!(first.result.as_ref().unwrap()["state"], "stopped");
        assert_eq!(
            first.result.as_ref().unwrap()["sessions"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(plane.runtimes[&original.id].state(), RuntimeState::Stopped);
        let replay = plane.dispatch_authorized_for_client(request(), "shell", &grant);
        assert_eq!(replay.result, first.result);
    }

    #[test]
    fn recorder_api_reports_failed_finalization_without_stopping_audio() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        plane.session_start(&original.id).unwrap();
        let queue = Arc::new(RecordingQueue::new(4).unwrap());
        plane
            .attach_recorder_worker(
                original.id.clone(),
                Box::new(FailingTapRecorderWorker {
                    tap: Arc::new(RecorderAudioTap::new(queue)),
                }),
            )
            .unwrap();
        for (index, method) in ["recorders.arm", "recorders.start", "recorders.stop"]
            .iter()
            .enumerate()
        {
            let mut params = json!({"sessionId": original.id,
                "idempotencyKey": format!("failed-finalization-{index}")});
            if *method != "recorders.arm" {
                params["frame"] = json!(0);
            }
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(index)),
                method: (*method).into(),
                params: Some(params),
            });
            assert_eq!(
                response.error.is_some(),
                *method == "recorders.stop",
                "{response:?}"
            );
        }
        assert_eq!(plane.recorders[&original.id].state(), RecorderState::Failed);
        assert_eq!(plane.runtimes[&original.id].state(), RuntimeState::Running);
    }

    #[test]
    fn system_quit_keeps_session_running_when_recorder_finalization_fails() {
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        plane.session_start(&original.id).unwrap();
        let mut recorder = RecorderController::new();
        recorder.arm().unwrap();
        recorder.start(128).unwrap();
        plane.recorders.insert(original.id.clone(), recorder);
        let request = || JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!("quit")),
            method: "system.quit".into(),
            params: Some(json!({ "idempotencyKey": "quit-retry" })),
        };
        let grant = ClientGrant::for_role(ClientRole::Operator);
        let failed = plane.dispatch_authorized_for_client(request(), "shell", &grant);
        assert!(failed.result.is_none());
        assert_eq!(plane.runtimes[&original.id].state(), RuntimeState::Running);

        plane
            .attach_recorder_worker(original.id.clone(), Box::new(TestRecorderWorker))
            .unwrap();
        let recovered = plane.dispatch_authorized_for_client(request(), "shell", &grant);
        assert_eq!(recovered.result.as_ref().unwrap()["state"], "stopped");
        assert_eq!(plane.runtimes[&original.id].state(), RuntimeState::Stopped);
    }

    #[test]
    fn system_quit_finalizes_all_active_node_recorders_before_stopping_session() {
        let mut plane = ControlPlane::default();
        let mut original = session();
        for node_id in ["quit-recorder-a", "quit-recorder-b"] {
            original.nodes.push(Node {
                id: EntityId::new(node_id),
                kind: NodeKind::Recorder,
                type_version: 1,
                name: node_id.into(),
                enabled: true,
                bypass: false,
                parameters: Default::default(),
                ports: vec![],
            });
        }
        let session_id = original.id.clone();
        plane.insert_session(original).unwrap();
        plane.session_start(&session_id).unwrap();
        let queue = Arc::new(RecordingQueue::new(4).unwrap());
        for node_id in ["quit-recorder-a", "quit-recorder-b"] {
            plane
                .attach_recorder_worker_to_node(
                    &session_id,
                    EntityId::new(node_id),
                    Box::new(SuccessfulTapRecorderWorker {
                        tap: Arc::new(RecorderAudioTap::new(queue.clone())),
                    }),
                )
                .unwrap();
            plane
                .control_recorder_node(&EntityId::new(node_id), "recorders.arm", None)
                .unwrap();
            plane
                .control_recorder_node(&EntityId::new(node_id), "recorders.start", Some(0))
                .unwrap();
        }

        let response = plane.dispatch_authorized_for_client(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!("quit")),
                method: "system.quit".into(),
                params: Some(json!({ "idempotencyKey": "quit-two-node-recorders" })),
            },
            "shell",
            &ClientGrant::for_role(ClientRole::Operator),
        );

        assert_eq!(response.result.as_ref().unwrap()["state"], "stopped");
        assert_eq!(
            response.result.as_ref().unwrap()["recorders"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(plane.recorder_node_workers.is_empty());
        assert!(plane.recorder_node_states.is_empty());
        assert!(plane.recorder_node_sessions.is_empty());
        assert_eq!(plane.runtimes[&session_id].state(), RuntimeState::Stopped);
    }

    #[test]
    fn session_runtime_label_distinguishes_native_attachment() {
        assert_eq!(session_runtime_label(false), "fake");
        assert_eq!(session_runtime_label(true), "native");
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

        assert!(plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "recorders.arm".into(),
                params: Some(json!({
                    "sessionId": original.id,
                    "idempotencyKey": "hook-arm"
                })),
            })
            .result
            .is_some());
        let repeated_arm = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "recorders.arm".into(),
            params: Some(json!({
                "sessionId": original.id,
                "idempotencyKey": "hook-repeated-arm"
            })),
        });
        assert!(repeated_arm.result.is_none());
        assert!(repeated_arm.error.is_some());
        assert_eq!(*hooks.lock().unwrap(), vec!["arm"]);

        for (id, method, frame) in [
            (3, "recorders.start", Some(0)),
            (4, "recorders.split", Some(128)),
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
    fn file_recorder_factory_removes_file_when_attachment_is_rejected() {
        let root = std::env::temp_dir().join(format!(
            "audiorouter-control-factory-rollback-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        let policy = RecordingPathPolicy::new(&root).unwrap();
        let mut plane = ControlPlane::default();
        let original = session();
        plane.insert_session(original.clone()).unwrap();
        plane
            .attach_recorder_worker(
                original.id.clone(),
                Box::new(HookRecorderWorker {
                    hooks: Arc::new(std::sync::Mutex::new(Vec::new())),
                }),
            )
            .unwrap();

        let result = plane.create_and_attach_file_recorder(
            &policy,
            original.id,
            "voice",
            0,
            FileRecorderFormat::Wav(WavFormat::Pcm16),
            1,
            48_000,
            false,
            8,
            1,
        );

        assert!(matches!(
            result,
            Err(ControlError::InvalidRequest(message))
                if message == "recorder worker is already attached"
        ));
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(root);
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
        let taps = plane.recorder_tap_set(&original.id).unwrap();
        let mut first_block = AudioBlock::new(1, 2).unwrap();
        first_block
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[0.1, 0.2]);
        taps.on_processed_block(0, &first_block);

        // Idle audio is not retained. Start is the admission boundary.
        let queue = plane.recorder_workers[&original.id]
            .shared_recording_queue()
            .unwrap();
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.overruns(), 0);

        for (id, method, frame) in [
            (1, "recorders.arm", None),
            (2, "recorders.start", Some(0)),
            (3, "recorders.split", Some(2)),
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
            if method == "recorders.start" {
                taps.on_processed_block(0, &first_block);
            }
        }
        let mut second_block = AudioBlock::new(1, 2).unwrap();
        second_block
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[0.3, 0.4]);
        taps.on_processed_block(2, &second_block);
        let response = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "recorders.stop".into(),
            params: Some(json!({
                "sessionId": original.id,
                "frame": 4,
                "idempotencyKey": "factory-4"
            })),
        });
        assert!(response.result.is_some(), "recorders.stop: {response:?}");
        let rows = plane
            .storage
            .as_ref()
            .unwrap()
            .list_recordings(Some(original.id.as_str()))
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows
            .iter()
            .all(|row| row.format == "wav" && row.frames == 2));
        assert_eq!(rows[0].path, path.to_str().unwrap());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn all_file_formats_drain_queued_audio_before_pause_and_drop_paused_taps() {
        let root = std::env::temp_dir().join(format!(
            "audiorouter-e2e-recorders-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let policy = RecordingPathPolicy::new(&root).unwrap();
        for (index, format) in [
            FileRecorderFormat::Wav(WavFormat::Pcm16),
            FileRecorderFormat::Wav(WavFormat::Pcm24),
            FileRecorderFormat::Wav(WavFormat::Float32),
            FileRecorderFormat::Flac {
                bits_per_sample: 16,
            },
            FileRecorderFormat::Flac {
                bits_per_sample: 24,
            },
            FileRecorderFormat::Mp3,
        ]
        .into_iter()
        .enumerate()
        {
            let config = FileRecorderConfig {
                version: FILE_RECORDER_CONFIG_VERSION,
                session_id: "synthetic",
                recorder_id: "take",
                sequence: index as u64,
                format,
                channels: 2,
                sample_rate: 48_000,
                dither: false,
                queue_capacity: 4,
                maximum_chunks_per_pass: 1,
            };
            let (path, mut worker) = create_file_recorder_with_config(&policy, &config).unwrap();
            worker.arm().unwrap();
            worker.start(0).unwrap();
            let tap = worker.shared_audio_tap().unwrap();
            let mut block = AudioBlock::new(2, 128).unwrap();
            block.channel_mut(0).unwrap().fill(0.1);
            block.channel_mut(1).unwrap().fill(-0.1);
            tap.on_processed_block(0, &block);
            // No writer pump between queued input and the lifecycle commands.
            worker.pause(128).unwrap();
            for frame in 128..1152 {
                tap.on_processed_block(frame, &block);
            }
            worker.resume(256).unwrap();
            tap.on_processed_block(256, &block);
            let outcome = worker.finalize(384).unwrap();
            assert_eq!(outcome.state, "completed", "{format:?}");
            assert!(outcome.file_finalized && !outcome.recoverable, "{format:?}");
            let recordings = worker.finalized_recordings();
            assert_eq!(
                recordings
                    .iter()
                    .map(|recording| recording.frames)
                    .sum::<u64>(),
                256,
                "{format:?}"
            );
            assert!(std::fs::metadata(&path).unwrap().len() > 0);
            drop(tap);
            drop(worker);
        }
        assert!(
            root.is_absolute()
                && root
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("audiorouter-e2e-recorders-")
        );
        std::fs::remove_dir_all(root).unwrap();
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
            let arm = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(format!("{node_id}-arm"))),
                method: "recorders.arm".into(),
                params: Some(json!({
                    "sessionId": session_id,
                    "nodeId": node_id,
                    "idempotencyKey": format!("node-{node_id}-arm"),
                })),
            });
            assert!(arm.error.is_none(), "recorders.arm: {arm:?}");
            let repeated_arm = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(format!("{node_id}-repeated-arm"))),
                method: "recorders.arm".into(),
                params: Some(json!({
                    "sessionId": session_id,
                    "nodeId": node_id,
                    "idempotencyKey": format!("node-{node_id}-repeated-arm"),
                })),
            });
            assert!(repeated_arm.result.is_none());
            assert!(repeated_arm.error.is_some());
            for (index, method, frame) in [
                (2, "recorders.start", Some(0)),
                (3, "recorders.stop", Some(0)),
            ] {
                let mut params = json!({
                    "sessionId": session_id,
                    "nodeId": node_id,
                    "idempotencyKey": format!("node-{node_id}-{index}"),
                });
                if let Some(frame) = frame {
                    params["frame"] = json!(frame);
                }
                let response = plane.dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(format!("{node_id}-{index}"))),
                    method: method.into(),
                    params: Some(params),
                });
                assert!(response.error.is_none(), "{method}: {response:?}");
                if method == "recorders.stop" {
                    assert_eq!(response.result.unwrap()["state"], "completed");
                }
            }
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
    fn failed_node_recorder_does_not_stop_healthy_sibling() {
        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let healthy_path =
            std::env::temp_dir().join(format!("audiorouter-control-isolation-{run_id}.wav"));
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&healthy_path)
            .unwrap();
        let healthy = WavRecorderWorker::new(file, WavFormat::Pcm16, 1, 48_000, 4, 1).unwrap();
        let queue = Arc::new(RecordingQueue::new(4).unwrap());
        let failing = FailingTapRecorderWorker {
            tap: Arc::new(RecorderAudioTap::new(queue)),
        };
        let mut graph = session();
        for id in ["failed-recorder", "healthy-recorder"] {
            graph.nodes.push(Node {
                id: EntityId::new(id),
                kind: NodeKind::Recorder,
                type_version: 1,
                name: id.into(),
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
                EntityId::new("failed-recorder"),
                Box::new(failing),
            )
            .unwrap();
        plane
            .attach_recorder_worker_to_node(
                &session_id,
                EntityId::new("healthy-recorder"),
                Box::new(healthy),
            )
            .unwrap();
        for node_id in ["failed-recorder", "healthy-recorder"] {
            plane
                .control_recorder_node(&EntityId::new(node_id), "recorders.arm", None)
                .unwrap();
            plane
                .control_recorder_node(&EntityId::new(node_id), "recorders.start", Some(0))
                .unwrap();
        }
        assert!(plane
            .control_recorder_node(&EntityId::new("failed-recorder"), "recorders.stop", Some(0),)
            .is_err());
        assert_eq!(
            plane.recorder_node_states[&EntityId::new("failed-recorder")].state(),
            RecorderState::Failed
        );
        let listed = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "recorders.list".into(),
                params: None,
            })
            .result
            .unwrap();
        assert!(listed
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["nodeId"] == "failed-recorder" && entry["state"] == "failed"));
        assert!(plane
            .control_recorder_node(
                &EntityId::new("healthy-recorder"),
                "recorders.stop",
                Some(0),
            )
            .is_ok());
        assert!(!plane
            .recorder_node_workers
            .contains_key(&EntityId::new("healthy-recorder")));
        assert!(plane
            .recorder_node_workers
            .contains_key(&EntityId::new("failed-recorder")));
        plane.delete_session(&session_id).unwrap();
        let _ = std::fs::remove_file(healthy_path);
    }

    #[test]
    fn mixed_legacy_and_node_recorders_share_the_active_capacity_limit() {
        let mut graph = session();
        graph.nodes.push(Node {
            id: EntityId::new("capacity-recorder"),
            kind: NodeKind::Recorder,
            type_version: 1,
            name: "Capacity recorder".into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: vec![],
        });
        let session_id = graph.id.clone();
        let mut plane = ControlPlane::default();
        plane.insert_session(graph).unwrap();
        for index in 0..MAX_ACTIVE_RECORDERS {
            let mut recorder = RecorderController::new();
            recorder.arm().unwrap();
            recorder.start(0).unwrap();
            plane
                .recorders
                .insert(EntityId::new(format!("legacy-{index}")), recorder);
        }
        let queue = Arc::new(RecordingQueue::new(4).unwrap());
        plane
            .attach_recorder_worker_to_node(
                &session_id,
                EntityId::new("capacity-recorder"),
                Box::new(FailingTapRecorderWorker {
                    tap: Arc::new(RecorderAudioTap::new(queue)),
                }),
            )
            .unwrap();
        assert!(plane
            .control_recorder_node(&EntityId::new("capacity-recorder"), "recorders.arm", None,)
            .is_err());
        assert_eq!(
            plane.recorder_node_states[&EntityId::new("capacity-recorder")].state(),
            RecorderState::Idle
        );
        plane.delete_session(&session_id).unwrap();
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
    fn recorder_create_schema_allows_format_aware_dither_default() {
        let schema = method_input_schema("recorders.create");
        let required = schema["required"].as_array().unwrap();
        assert!(!required.iter().any(|value| value == "dither"));
        assert_eq!(
            schema["properties"]["dither"]["description"],
            "Optional; defaults to TPDF for integer WAV/FLAC and false for WAV Float32 or MP3."
        );
    }

    #[test]
    fn recorder_create_api_can_target_a_recorder_node_and_replay_idempotently() {
        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("audiorouter-control-node-create-{run_id}"));
        std::fs::create_dir_all(&root).unwrap();
        let mut plane = ControlPlane::with_storage("node-create", Storage::open_memory().unwrap());
        plane.configure_recording_root(&root).unwrap();
        let mut original = session();
        original.nodes.push(Node {
            id: EntityId::new("capture-recorder"),
            kind: NodeKind::Recorder,
            type_version: 1,
            name: "Capture recorder".into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: vec![],
        });
        original.nodes.push(Node {
            id: EntityId::new("desktop-recorder"),
            kind: NodeKind::Recorder,
            type_version: 1,
            name: "Desktop recorder".into(),
            enabled: true,
            bypass: false,
            parameters: Default::default(),
            ports: vec![],
        });
        plane.insert_session(original.clone()).unwrap();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "recorders.create".into(),
            params: Some(json!({
                "sessionId": original.id,
                "nodeId": "capture-recorder",
                "recorderId": "capture",
                "format": "wavPcm16",
                "sequence": 0,
                "channels": 1,
                "sampleRate": 48000,
                "queueCapacity": 8,
                "maximumChunksPerPass": 1,
                "idempotencyKey": "node-create-1"
            })),
        };
        let first = plane.dispatch(request.clone());
        let first_result = first.result.clone().unwrap();
        assert_eq!(first_result["nodeId"], "capture-recorder");
        assert_eq!(first_result["state"], "idle");
        assert_eq!(plane.recorder_node_workers.len(), 1);
        assert_eq!(plane.dispatch(request).result.unwrap(), first_result);
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        let second = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "recorders.create".into(),
            params: Some(json!({
                "sessionId": original.id,
                "nodeId": "desktop-recorder",
                "recorderId": "desktop",
                "format": "wavFloat32",
                "sequence": 0,
                "channels": 1,
                "sampleRate": 48000,
                "queueCapacity": 8,
                "maximumChunksPerPass": 1,
                "idempotencyKey": "node-create-2"
            })),
        });
        assert!(second.error.is_none(), "{second:?}");
        assert_eq!(second.result.unwrap()["nodeId"], "desktop-recorder");
        assert_eq!(plane.recorder_node_workers.len(), 2);
        let listed = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(10)),
                method: "recorders.list".into(),
                params: None,
            })
            .result
            .unwrap();
        let listed = listed.as_array().unwrap();
        assert_eq!(listed.len(), 2);
        assert!(listed
            .iter()
            .any(|entry| entry["nodeId"] == "capture-recorder"));
        assert!(listed
            .iter()
            .any(|entry| entry["nodeId"] == "desktop-recorder"));

        for (node_id, recorder_id) in [
            ("capture-recorder", "capture"),
            ("desktop-recorder", "desktop"),
        ] {
            for (index, method, frame) in [
                (2, "recorders.arm", None),
                (3, "recorders.start", Some(0)),
                (4, "recorders.stop", Some(0)),
            ] {
                let mut params = json!({
                    "sessionId": original.id,
                    "nodeId": node_id,
                    "idempotencyKey": format!("node-create-{recorder_id}-{index}"),
                });
                if let Some(frame) = frame {
                    params["frame"] = json!(frame);
                }
                let response = plane.dispatch(JsonRpcRequest {
                    jsonrpc: "2.0".into(),
                    id: Some(json!(index)),
                    method: method.into(),
                    params: Some(params),
                });
                assert!(response.error.is_none(), "{method}: {response:?}");
            }
        }
        assert!(plane.recorder_node_workers.is_empty());
        let recordings = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(20)),
                method: "recordings.list".into(),
                params: Some(json!({ "sessionId": original.id })),
            })
            .result
            .unwrap();
        let recordings = recordings.as_array().unwrap();
        assert_eq!(recordings.len(), 2);
        assert!(recordings
            .iter()
            .all(|recording| recording["state"] == "completed"));
        assert!(recordings
            .iter()
            .any(|recording| recording["recorderId"] == "capture"));
        assert!(recordings
            .iter()
            .any(|recording| recording["recorderId"] == "desktop"));
        assert!(recordings
            .iter()
            .any(|recording| recording["nodeId"] == "capture-recorder"));
        assert!(recordings
            .iter()
            .any(|recording| recording["nodeId"] == "desktop-recorder"));
        assert_eq!(
            recordings
                .iter()
                .find(|recording| recording["recorderId"] == "capture")
                .unwrap()["dither"],
            true
        );
        assert_eq!(
            recordings
                .iter()
                .find(|recording| recording["recorderId"] == "desktop")
                .unwrap()["dither"],
            false
        );
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
    fn float_wav_metadata_does_not_claim_requested_dither() {
        let path = std::env::temp_dir().join(format!(
            "audiorouter-control-float-dither-{}.wav",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut worker =
            WavRecorderWorker::new_with_dither(file, WavFormat::Float32, 1, 48_000, true, 8, 1)
                .unwrap();
        worker.set_library_identity(FileRecordingIdentity {
            session_id: "float-session".into(),
            recorder_id: "float-recorder".into(),
            path: path.clone(),
        });
        worker.arm().unwrap();
        worker.start(0).unwrap();
        worker
            .try_push(RecordingChunk {
                start_frame: 0,
                samples: vec![0.25],
            })
            .unwrap();
        worker.finalize(1).unwrap();

        let recordings = worker.finalized_recordings();
        assert_eq!(recordings.len(), 1);
        assert!(!recordings[0].dither);
        assert_eq!(
            recordings[0].conversion,
            "targetSampleRate=48000;channels=1;format=Float32"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn recorder_format_defaults_enable_dither_only_for_integer_output() {
        assert!(default_dither_for_format(FileRecorderFormat::Wav(
            WavFormat::Pcm16
        )));
        assert!(default_dither_for_format(FileRecorderFormat::Wav(
            WavFormat::Pcm24
        )));
        assert!(default_dither_for_format(FileRecorderFormat::Flac {
            bits_per_sample: 16,
        }));
        assert!(!default_dither_for_format(FileRecorderFormat::Wav(
            WavFormat::Float32
        )));
        assert!(!default_dither_for_format(FileRecorderFormat::Mp3));
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
                && !recording.dither
                && recording.conversion == "targetSampleRate=48000;channels=1;format=wav"
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
        let mut worker =
            BufferedFlacRecorderWorker::new_with_dither(file, 1, 48_000, 16, true, 8, 1).unwrap();
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

        let mut plane =
            ControlPlane::with_storage("buffered-flac-metadata", Storage::open_memory().unwrap());
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
        let finalized = plane
            .storage
            .as_ref()
            .unwrap()
            .list_recordings(Some("session"))
            .unwrap();
        assert_eq!(finalized.len(), 1);
        assert!(finalized[0].dither);
        assert_eq!(
            finalized[0].conversion,
            "targetSampleRate=48000;channels=1;bitsPerSample=16"
        );
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
        let mut worker =
            StreamingFlacRecorderWorker::new(file, 1, 48_000, 16, false, 8, 1).unwrap();
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
        assert_eq!(worker.drain_pending(1).unwrap(), 1);
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
    fn recorder_factory_selects_incremental_flac_worker() {
        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("audiorouter-control-streaming-factory-{run_id}"));
        std::fs::create_dir_all(&root).unwrap();
        let policy = RecordingPathPolicy::new(&root).unwrap();
        let config = FileRecorderConfig {
            version: FILE_RECORDER_CONFIG_VERSION,
            session_id: "session",
            recorder_id: "voice",
            sequence: 0,
            format: FileRecorderFormat::Flac {
                bits_per_sample: 16,
            },
            channels: 1,
            sample_rate: 48_000,
            dither: true,
            queue_capacity: 8,
            maximum_chunks_per_pass: 1,
        };
        let (path, mut worker) = create_file_recorder_with_config(&policy, &config).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() >= 4);
        worker.arm().unwrap();
        worker.start(0).unwrap();
        let tap = worker.shared_audio_tap().unwrap();
        let mut block = AudioBlock::new(1, 2).unwrap();
        block
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[0.25, -0.25]);
        tap.on_processed_block(0, &block);
        // Service reads progress without waiting; Finalize is the durable barrier.
        worker.drain_pending(1).unwrap();
        assert_eq!(worker.finalize(2).unwrap().state, "completed");
        assert_eq!(
            audiorouter_recording::inspect_flac_file(&path)
                .unwrap()
                .frames,
            2
        );
        let finalized = worker.finalized_recordings();
        assert_eq!(finalized.len(), 1);
        assert!(finalized[0].dither);
        assert_eq!(
            finalized[0].conversion,
            "targetSampleRate=48000;channels=1;bitsPerSample=16"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn recorder_factory_selects_mp3_worker_and_persists_metadata() {
        let run_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("audiorouter-control-mp3-factory-{run_id}"));
        std::fs::create_dir_all(&root).unwrap();
        let policy = RecordingPathPolicy::new(&root).unwrap();
        let config = FileRecorderConfig {
            version: FILE_RECORDER_CONFIG_VERSION,
            session_id: "session",
            recorder_id: "voice",
            sequence: 0,
            format: FileRecorderFormat::Mp3,
            channels: 1,
            sample_rate: 48_000,
            dither: false,
            queue_capacity: 8,
            maximum_chunks_per_pass: 1,
        };
        let (path, mut worker) = create_file_recorder_with_config(&policy, &config).unwrap();
        worker.arm().unwrap();
        worker.start(0).unwrap();
        let tap = worker.shared_audio_tap().unwrap();
        let mut block = AudioBlock::new(1, 4).unwrap();
        block
            .channel_mut(0)
            .unwrap()
            .copy_from_slice(&[0.25, -0.25, 0.1, -0.1]);
        tap.on_processed_block(0, &block);
        worker.drain_pending(1).unwrap();
        assert_eq!(worker.finalize(4).unwrap().state, "completed");
        assert_eq!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("mp3")
        );
        assert!(std::fs::metadata(&path).unwrap().len() > 128);
        let finalized = worker.finalized_recordings();
        assert_eq!(finalized.len(), 1);
        assert_eq!(finalized[0].format, "mp3");
        assert_eq!(finalized[0].frames, 4);
        assert!(!finalized[0].dither);
        assert!(finalized[0].conversion.contains("bitrateKbps=192"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn draft_preview_requires_prepared_native_route_and_does_not_save_candidate() {
        let mut plane = ControlPlane::default();
        let saved = session();
        plane.insert_session(saved.clone()).unwrap();
        let mut candidate = saved.clone();
        candidate.name = "temporary audition".into();
        let error = plane
            .session_preview_start(&saved.id, &candidate)
            .unwrap_err();
        let ControlError::InvalidRequest(message) = error else {
            panic!("expected native preview preparation guidance");
        };
        assert!(message.contains("prepared single-endpoint audio route"));
        assert_eq!(plane.get_session(&saved.id).unwrap(), &saved);
        assert!(!plane
            .runtimes
            .get(&saved.id)
            .is_some_and(|runtime| runtime.state() == RuntimeState::Running));

        let request = json!({
            "sessionId": saved.id,
            "candidate": candidate,
            "idempotencyKey": "preview-contract"
        });
        assert!(validate_method_params("session.start", Some(&request)).is_ok());
    }

    #[test]
    fn meter_reset_requires_session_control_and_a_prepared_meter() {
        let mut plane = ControlPlane::default();
        let mut saved = session();
        saved.nodes[1].kind = NodeKind::Meter;
        let node_id = saved.nodes[1].id.clone();
        let id = saved.id.clone();
        plane.insert_session(saved.clone()).unwrap();
        let request = || JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "meters.reset".into(),
            params: Some(json!({"sessionId":id,"nodeId":node_id})),
        };
        let read_only = ClientGrant::read_only();
        assert_eq!(
            plane
                .dispatch_authorized(request(), &read_only)
                .error
                .unwrap()
                .code,
            -32001
        );
        assert!(plane
            .dispatch(request())
            .error
            .unwrap()
            .message
            .contains("not prepared"));
        assert_eq!(plane.get_session(&id).unwrap(), &saved);
        assert!(validate_method_params(
            "meters.reset",
            Some(&json!({"sessionId":id,"nodeId":node_id,"unexpected":true}))
        )
        .is_err());
    }

    #[test]
    fn unprepared_plugin_start_explains_http_preparation_for_both_aliases() {
        for method in ["session.start", "sessions.start"] {
            let mut plane = ControlPlane::default();
            let mut saved = session();
            saved.nodes[0].kind = NodeKind::Plugin;
            saved.nodes[0].enabled = true;
            let id = saved.id.clone();
            plane.insert_session(saved.clone()).unwrap();
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: method.into(),
                params: Some(json!({ "sessionId": id, "idempotencyKey": "unprepared-start" })),
            });
            let error = response.error.expect("unprepared plugins must not start");
            assert_eq!(error.code, -32602);
            assert!(error.message.contains("POST /api/v1/nativePaths/prepare"));
            assert!(error.message.contains("DeviceAdministration"));
            assert_eq!(plane.get_session(&id).unwrap(), &saved);
            assert!(!plane
                .runtimes
                .get(&id)
                .is_some_and(|runtime| runtime.state() == RuntimeState::Running));
        }
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
    fn virtual_bridge_failure_event_is_discoverable_and_bus_scoped() {
        let mut plane = ControlPlane::default();
        let bus_id = EntityId::new("failed-bus");
        plane.publish_virtual_bridge_failure(&bus_id);
        let event = plane.events.since(0, 10).unwrap().pop().unwrap();
        assert_eq!(event.category, "virtualBridge.failed");
        assert_eq!(event.operation_id.as_deref(), Some("failed-bus"));
        assert!(event.session_id.is_none());
        let description = plane.describe();
        let categories = description["events"]["stateCategories"].as_array().unwrap();
        assert!(categories
            .iter()
            .any(|value| value == "virtualBridge.failed"));
    }

    #[test]
    fn virtual_bridge_lease_expiry_silences_and_drains_managed_route() {
        let mut plane = ControlPlane::default();
        let id = EntityId::new("expiring-bus");
        plane
            .create_virtual_bus(id.clone(), "Expiring bus")
            .unwrap();
        let bridge = plane.virtual_bridges.get(&id).unwrap();
        bridge.activate(1).unwrap();
        bridge.renew_lease(1, 100).unwrap();

        assert_eq!(plane.expire_virtual_bridge_leases(99), 0);
        assert!(bridge.is_active());
        assert_eq!(plane.expire_virtual_bridge_leases(100), 1);
        assert!(!bridge.is_active());
        assert!(bridge.try_receive_capture().is_none());
        let event = plane.events.since(0, 10).unwrap().pop().unwrap();
        assert_eq!(event.category, "virtualBridge.expired");
        assert_eq!(event.operation_id.as_deref(), Some("expiring-bus"));
    }

    #[test]
    fn native_detach_cleanup_deactivates_the_portable_bridge() {
        let mut plane = ControlPlane::default();
        let id = EntityId::new("detach-bus");
        plane.create_virtual_bus(id.clone(), "Detach bus").unwrap();
        let bridge = plane.virtual_bridges.get(&id).unwrap();
        bridge.activate(1).unwrap();
        assert!(bridge.is_active());

        plane.deactivate_virtual_bridge(&id);

        assert!(!bridge.is_active());
        assert!(bridge.try_receive_capture().is_none());
    }

    #[test]
    fn virtual_route_taps_require_an_explicit_sink_and_select_only_matching_enabled_buses() {
        let mut plane = ControlPlane::default();
        let first_bus = EntityId::new("route-bus-1");
        let second_bus = EntityId::new("route-bus-2");
        let third_bus = EntityId::new("route-bus-3");
        plane
            .create_virtual_bus(first_bus.clone(), "First")
            .unwrap();
        plane
            .create_virtual_bus(second_bus.clone(), "Second")
            .unwrap();
        plane
            .create_virtual_bus(third_bus.clone(), "Third")
            .unwrap();
        let routes = VirtualBusRouteRegistry::new(vec![
            audiorouter_domain::VirtualBusRoute {
                bus_id: first_bus.clone(),
                producer_session_id: EntityId::new("producer"),
                consumer_session_id: EntityId::new("consumer"),
            },
            audiorouter_domain::VirtualBusRoute {
                bus_id: second_bus.clone(),
                producer_session_id: EntityId::new("other-producer"),
                consumer_session_id: EntityId::new("consumer"),
            },
            audiorouter_domain::VirtualBusRoute {
                bus_id: third_bus.clone(),
                producer_session_id: EntityId::new("producer"),
                consumer_session_id: EntityId::new("consumer"),
            },
        ])
        .unwrap();
        plane.virtual_bus_routes = routes;
        plane.virtual_bus_route_revision = 1;
        assert!(plane
            .virtual_route_tap_set(&EntityId::new("producer"), &[], 1)
            .unwrap()
            .is_empty());
        plane
            .virtual_bridges
            .get(&first_bus)
            .unwrap()
            .activate(1)
            .unwrap();
        plane
            .virtual_bridges
            .get(&third_bus)
            .unwrap()
            .activate(1)
            .unwrap();
        assert_eq!(
            plane
                .virtual_route_tap_set(
                    &EntityId::new("producer"),
                    &[first_bus.clone(), third_bus.clone()],
                    1,
                )
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            plane
                .virtual_route_tap_set(
                    &EntityId::new("producer"),
                    std::slice::from_ref(&first_bus),
                    1,
                )
                .unwrap()
                .len(),
            1
        );
        assert!(plane
            .virtual_route_tap_set(
                &EntityId::new("producer"),
                std::slice::from_ref(&second_bus),
                1,
            )
            .unwrap()
            .is_empty());
    }

    #[test]
    fn virtual_route_bridge_activation_follows_sink_generation_and_stop_clears_it() {
        let mut plane = ControlPlane::default();
        let bus_id = EntityId::new("generation-bus");
        let producer = EntityId::new("generation-producer");
        let mut producer_session = session();
        producer_session.id = producer.clone();
        let mut consumer_session = session();
        consumer_session.id = EntityId::new("generation-consumer");
        plane.insert_session(producer_session).unwrap();
        plane.insert_session(consumer_session).unwrap();
        plane
            .create_virtual_bus(bus_id.clone(), "Generation")
            .unwrap();
        plane
            .replace_virtual_bus_routes(
                VirtualBusRouteRegistry::new(vec![audiorouter_domain::VirtualBusRoute {
                    bus_id: bus_id.clone(),
                    producer_session_id: producer.clone(),
                    consumer_session_id: EntityId::new("generation-consumer"),
                }])
                .unwrap(),
            )
            .unwrap();

        let bridge = plane.virtual_bridges.get(&bus_id).unwrap();
        assert!(!bridge.is_active());
        plane
            .prepare_virtual_route_bridges(&producer, 7, std::slice::from_ref(&bus_id))
            .unwrap();
        assert!(bridge.is_active());
        assert_eq!(bridge.generation(), 7);

        plane
            .prepare_virtual_route_bridges(&producer, 8, &[])
            .unwrap();
        assert!(!bridge.is_active());
        assert_eq!(bridge.generation(), 7);

        plane
            .prepare_virtual_route_bridges(&producer, 8, std::slice::from_ref(&bus_id))
            .unwrap();
        assert!(bridge.is_active());
        plane.delete_session(&producer).unwrap();
        assert!(!bridge.is_active());
    }

    #[test]
    fn virtual_route_bridge_rejects_stale_reactivation_generation() {
        let mut plane = ControlPlane::default();
        let bus_id = EntityId::new("stale-generation-bus");
        let producer = EntityId::new("stale-generation-producer");
        let mut producer_session = session();
        producer_session.id = producer.clone();
        let mut consumer_session = session();
        consumer_session.id = EntityId::new("stale-generation-consumer");
        plane.insert_session(producer_session).unwrap();
        plane.insert_session(consumer_session).unwrap();
        plane.create_virtual_bus(bus_id.clone(), "Stale").unwrap();
        plane
            .replace_virtual_bus_routes(
                VirtualBusRouteRegistry::new(vec![audiorouter_domain::VirtualBusRoute {
                    bus_id: bus_id.clone(),
                    producer_session_id: producer.clone(),
                    consumer_session_id: EntityId::new("stale-generation-consumer"),
                }])
                .unwrap(),
            )
            .unwrap();
        plane
            .prepare_virtual_route_bridges(&producer, 4, std::slice::from_ref(&bus_id))
            .unwrap();
        plane
            .prepare_virtual_route_bridges(&producer, 4, &[])
            .unwrap();
        let error = plane.prepare_virtual_route_bridges(&producer, 3, &[bus_id]);
        assert!(
            matches!(error, Err(ControlError::InvalidRequest(message)) if message.contains("stale"))
        );
    }

    #[test]
    fn virtual_bus_routes_require_known_buses_and_protect_referenced_deletion() {
        let mut plane = ControlPlane::default();
        plane.insert_session(session()).unwrap();
        let mut consumer = session();
        consumer.id = EntityId::new("consumer");
        plane.insert_session(consumer).unwrap();
        let bus_id = EntityId::new("bus-route");
        plane
            .create_virtual_bus(bus_id.clone(), "Route bus")
            .unwrap();
        let route = audiorouter_domain::VirtualBusRoute {
            bus_id: bus_id.clone(),
            producer_session_id: EntityId::new("session"),
            consumer_session_id: EntityId::new("consumer"),
        };
        plane
            .replace_virtual_bus_routes(
                audiorouter_domain::VirtualBusRouteRegistry::new(vec![route]).unwrap(),
            )
            .unwrap();
        assert_eq!(plane.virtual_bus_routes().list().len(), 1);
        assert!(plane.delete_virtual_bus(&bus_id).is_err());
        assert!(plane
            .replace_virtual_bus_routes(
                audiorouter_domain::VirtualBusRouteRegistry::new(vec![
                    audiorouter_domain::VirtualBusRoute {
                        bus_id: EntityId::new("missing"),
                        producer_session_id: EntityId::new("session"),
                        consumer_session_id: EntityId::new("consumer"),
                    },
                ])
                .unwrap(),
            )
            .is_err());
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
            let provision = plane.dispatch_authorized(
                request(
                    "virtualDevices.provision",
                    Some(json!({ "busId": "bus-1", "instanceId": "instance-1", "idempotencyKey": "key-2" })),
                ),
                &grant,
            );
            assert_eq!(provision.error.unwrap().code, -32001);
            let remove = plane.dispatch_authorized(
                request(
                    "virtualDevices.remove",
                    Some(json!({ "busId": "bus-1", "idempotencyKey": "key-3" })),
                ),
                &grant,
            );
            assert_eq!(remove.error.unwrap().code, -32001);
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
        assert!(!ClientGrant::for_role(ClientRole::Operator).allows(PermissionScope::StartupWrite));
        assert!(ClientGrant::for_desktop_shell().allows(PermissionScope::StartupWrite));
        assert!(ClientGrant::for_desktop_shell().allows(PermissionScope::Record));
        assert!(ClientGrant::for_desktop_shell().allows(PermissionScope::PluginScan));
        assert!(!ClientGrant::for_desktop_shell().allows(PermissionScope::Capture));
        assert!(!ClientGrant::for_desktop_shell().allows(PermissionScope::DeviceAdministration));
        assert!(!ClientGrant::for_role(ClientRole::Operator)
            .allows(PermissionScope::DeviceAdministration));
        assert!(!ClientGrant::read_only().allows(PermissionScope::PluginScan));
        assert!(ClientGrant::with_scopes([PermissionScope::PluginScan])
            .allows(PermissionScope::PluginScan));
    }

    #[test]
    fn native_endpoint_preparation_requires_device_administration_before_parameters() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(91)),
                method: "nativeEndpoints.prepare".into(),
                params: None,
            },
            &ClientGrant::for_role(ClientRole::Operator),
        );
        assert_eq!(response.error.unwrap().code, -32001);
        assert!(plane.native_endpoint_worker.is_none());
        assert!(plane.endpoint_monitor.is_none());
    }

    #[test]
    fn native_bridge_preparation_requires_device_administration_before_parameters() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(96)),
                method: "nativeBridges.prepare".into(),
                params: Some(json!({
                    "busId": "bus-guard",
                    "generation": 1,
                    "devicePath": "\\\\.\\AudioRouterVirtualBridge",
                    "renderMappingPath": "C:\\render.slot",
                    "captureMappingPath": "C:\\capture.slot",
                })),
            },
            &ClientGrant::for_role(ClientRole::Operator),
        );
        assert_eq!(response.error.unwrap().code, -32001);
        assert!(plane.native_duplex_bindings.is_empty());
        assert!(plane.native_duplex_worker.is_none());
    }

    #[test]
    fn native_endpoint_detachment_requires_device_administration_and_exact_binding() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(92)),
                method: "nativeEndpoints.detach".into(),
                params: Some(json!({ "sessionId": "missing" })),
            },
            &ClientGrant::for_role(ClientRole::Operator),
        );
        assert_eq!(response.error.unwrap().code, -32001);

        let response = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(93)),
                method: "nativeEndpoints.detach".into(),
                params: Some(json!({ "sessionId": "missing" })),
            },
            &ClientGrant::with_scopes([PermissionScope::DeviceAdministration]),
        );
        assert_eq!(response.error.unwrap().code, -32602);
        assert!(plane.native_endpoint_worker.is_none());
    }

    #[test]
    fn native_duplex_detachment_requires_device_administration_and_windows() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(94)),
                method: "nativeDuplex.detach".into(),
                params: Some(json!({ "sessionId": "missing" })),
            },
            &ClientGrant::for_role(ClientRole::Operator),
        );
        assert_eq!(response.error.unwrap().code, -32001);

        let response = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(95)),
                method: "nativeDuplex.detach".into(),
                params: Some(json!({ "sessionId": "missing" })),
            },
            &ClientGrant::with_scopes([PermissionScope::DeviceAdministration]),
        );
        assert_eq!(response.error.unwrap().code, -32602);
    }

    #[test]
    fn startup_mutations_require_the_dedicated_startup_write_scope() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "startup.plan".into(),
            params: Some(json!({ "enabled": true })),
        };
        let denied = ControlPlane::default().dispatch_authorized(
            request.clone(),
            &ClientGrant::for_role(ClientRole::Operator),
        );
        assert_eq!(denied.error.unwrap().code, -32001);

        let allowed = ControlPlane::default().dispatch_authorized(
            request,
            &ClientGrant::with_scopes([PermissionScope::StartupWrite]),
        );
        assert!(allowed.result.is_some());
    }

    #[test]
    fn draft_preview_requires_graph_write_in_addition_to_session_control() {
        let saved = session();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(94)),
            method: "session.start".into(),
            params: Some(json!({
                "sessionId": saved.id,
                "candidate": saved,
                "idempotencyKey": "preview-permission"
            })),
        };
        let denied = ControlPlane::default().dispatch_authorized(
            request,
            &ClientGrant::with_scopes([PermissionScope::SessionControl]),
        );
        let error = denied.error.unwrap();
        assert_eq!(error.code, -32001);
        assert_eq!(error.data.unwrap()["code"], "permissionDenied");
    }

    #[test]
    fn native_endpoint_pump_requires_session_control_before_parameters() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(92)),
                method: "nativeEndpoints.pump".into(),
                params: None,
            },
            &ClientGrant::read_only(),
        );
        assert_eq!(response.error.unwrap().code, -32001);
        assert_eq!(plane.native_endpoint_rejections, 0);
    }

    #[test]
    fn native_endpoint_pump_input_schema_bounds_packet_budget() {
        let description = ControlPlane::default().describe();
        let method = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "nativeEndpoints.pump")
            .unwrap();
        assert_eq!(
            method["inputSchema"]["properties"]["maxPackets"]["maximum"],
            json!(audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE)
        );
    }

    #[test]
    fn native_duplex_pump_input_schema_bounds_both_budgets() {
        let description = ControlPlane::default().describe();
        let method = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "nativeDuplex.pump")
            .unwrap();
        for property in ["maxInputQuanta", "maxOutputPackets"] {
            assert_eq!(
                method["inputSchema"]["properties"][property]["maximum"],
                json!(audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE)
            );
        }
    }

    #[test]
    fn native_render_source_pump_requires_session_control_before_parameters() {
        let mut plane = ControlPlane::default();
        let response = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(93)),
                method: "nativeRenderSources.pump".into(),
                params: None,
            },
            &ClientGrant::read_only(),
        );
        assert_eq!(response.error.unwrap().code, -32001);
        assert_eq!(plane.native_endpoint_rejections, 0);
    }

    #[test]
    fn native_render_source_pump_input_schema_bounds_quanta_budget() {
        let description = ControlPlane::default().describe();
        let method = description["methods"]
            .as_array()
            .unwrap()
            .iter()
            .find(|method| method["name"] == "nativeRenderSources.pump")
            .unwrap();
        assert_eq!(
            method["inputSchema"]["properties"]["maxQuanta"]["maximum"],
            json!(audiorouter_windows_audio::MAX_ENDPOINT_WORKER_PACKETS_PER_WAKE)
        );
        assert_eq!(
            method["inputSchema"]["required"],
            json!(["sessionId", "generation"])
        );
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
        for kind in ["bandPass", "allPass"] {
            let response = plane.dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(), id: Some(json!(5)), method: "processors.response".into(),
                params: Some(json!({ "sampleRateHz": 48000.0,
                    "bands": [{ "enabled": true, "type": kind, "frequencyHz": 1000.0, "q": 1.0, "gainDb": 24.0 }],
                    "frequenciesHz": [100.0, 1000.0, 10000.0] })),
            });
            let result = response.result.unwrap();
            assert!(result["magnitudeDb"][1].as_f64().unwrap().abs() < 0.02);
            if kind == "bandPass" {
                assert!(result["magnitudeDb"][0].as_f64().unwrap() < -10.0);
                assert!(result["magnitudeDb"][2].as_f64().unwrap() < -10.0);
            } else {
                assert!(result["magnitudeDb"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|value| value.as_f64().unwrap().abs() < 0.02));
            }
        }
        let flat_band = json!({"enabled": false, "type": "peaking", "frequencyHz": 1000.0, "q": 1.0, "gainDb": 0.0});
        let bounded_bands = vec![flat_band.clone(); MAX_RESPONSE_BANDS];
        let maximum = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "processors.response".into(),
            params: Some(json!({"sampleRateHz": 48_000.0, "bands": bounded_bands, "frequenciesHz": [1000.0]})),
        });
        assert!(maximum.error.is_none());
        let too_many = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(4)),
            method: "processors.response".into(),
            params: Some(json!({"sampleRateHz": 48_000.0, "bands": vec![flat_band; MAX_RESPONSE_BANDS + 1], "frequenciesHz": [1000.0]})),
        });
        assert!(too_many.error.is_some());
        let invalid = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "processors.response".into(),
            params: Some(json!({"sampleRateHz": 48_000.0, "bands": [], "frequenciesHz": []})),
        });
        assert!(invalid.error.is_some());
    }

    #[test]
    fn audio_media_api_uploads_and_decodes_a_bounded_wav() {
        let storage = Storage::open_memory().unwrap();
        let mut plane = ControlPlane::with_storage("audio-upload", storage);
        let wav = tiny_test_wav();
        let begin = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "audioMedia.beginUpload".into(),
                params: Some(json!({"fileName":"voice.wav", "sizeBytes":wav.len()})),
            })
            .result
            .unwrap();
        let upload_id = begin["uploadId"].as_str().unwrap();
        let chunk = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(), id: Some(json!(2)), method: "audioMedia.uploadChunk".into(),
            params: Some(json!({"uploadId":upload_id, "chunkIndex":0, "dataBase64":encode_test_base64(&wav)})),
        });
        assert_eq!(chunk.result.unwrap()["receivedBytes"], json!(wav.len()));
        let finish = plane
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(3)),
                method: "audioMedia.finishUpload".into(),
                params: Some(json!({"uploadId":upload_id})),
            })
            .result
            .unwrap();
        assert_eq!(finish["format"], "wav");
        assert_eq!(finish["channels"], 1);
        assert_eq!(finish["durationMs"], 1);
        assert!(finish["mediaId"]
            .as_str()
            .unwrap()
            .starts_with("audio-media-"));
    }

    #[test]
    fn temporary_take_import_is_bounded_expires_and_removes_only_its_own_file() {
        let storage = Storage::open_memory().unwrap();
        let mut plane = ControlPlane::with_storage("temporary-take", storage);
        let wav = {
            let samples = [0i16; 48];
            let mut bytes = Vec::with_capacity(36 + samples.len() * 2);
            bytes.extend_from_slice(b"RIFF");
            bytes.extend_from_slice(&(36u32 + (samples.len() * 2) as u32).to_le_bytes());
            bytes.extend_from_slice(b"WAVEfmt ");
            bytes.extend_from_slice(&16u32.to_le_bytes());
            bytes.extend_from_slice(&1u16.to_le_bytes());
            bytes.extend_from_slice(&1u16.to_le_bytes());
            bytes.extend_from_slice(&48_000u32.to_le_bytes());
            bytes.extend_from_slice(&96_000u32.to_le_bytes());
            bytes.extend_from_slice(&2u16.to_le_bytes());
            bytes.extend_from_slice(&16u16.to_le_bytes());
            bytes.extend_from_slice(b"data");
            bytes.extend_from_slice(&((samples.len() * 2) as u32).to_le_bytes());
            for sample in samples {
                bytes.extend_from_slice(&sample.to_le_bytes());
            }
            bytes
        };
        let path = std::env::temp_dir().join(format!(
            "audiorouter-temporary-take-{}.wav",
            std::process::id()
        ));
        std::fs::write(&path, &wav).unwrap();
        plane
            .storage
            .as_ref()
            .unwrap()
            .save_recording(&audiorouter_storage::RecordingRecord {
                id: "session-audio-file-take-test-run".into(),
                session_id: "session".into(),
                recorder_id: "audio-file-take-test".into(),
                path: path.to_string_lossy().into_owned(),
                format: "wav".into(),
                channels: 1,
                sample_rate: 48_000,
                frames: 48,
                file_bytes: wav.len() as u64,
                start_time: "2026-09-22T00:00:00Z".into(),
                state: "completed".into(),
                missing: false,
                title: None,
                artist: None,
                comment: None,
                dither: false,
                conversion: "temporary take".into(),
            })
            .unwrap();
        let denied = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(0)),
                method: "audioMedia.importTemporaryRecording".into(),
                params: Some(json!({"recordingId":"session-audio-file-take-test-run"})),
            },
            &ClientGrant::read_only(),
        );
        assert_eq!(denied.error.unwrap().code, -32001);
        assert!(
            path.exists(),
            "a denied import must not touch the temporary file"
        );
        let result = plane.dispatch_authorized(
            JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "audioMedia.importTemporaryRecording".into(),
                params: Some(json!({"recordingId":"session-audio-file-take-test-run"})),
            },
            &ClientGrant::with_scopes([PermissionScope::Read, PermissionScope::Record]),
        );
        assert!(
            result.error.is_none(),
            "temporary take import error: {:?}",
            result.error
        );
        let result = result.result.unwrap();
        assert_eq!(result["format"], "wav");
        assert_eq!(result["sourceRemoved"], true);
        assert!(result["expiresAt"].as_i64().unwrap() > unix_epoch_seconds());
        assert!(!path.exists());
        let media_id = result["mediaId"].as_str().unwrap();
        assert!(plane
            .storage
            .as_ref()
            .unwrap()
            .load_audio_media(media_id)
            .unwrap()
            .is_some());
        assert!(plane
            .storage
            .as_ref()
            .unwrap()
            .get_recording("session-audio-file-take-test-run")
            .unwrap()
            .is_none());

        let ordinary = path.with_file_name("audiorouter-ordinary-recording.wav");
        std::fs::write(&ordinary, &wav).unwrap();
        plane
            .storage
            .as_ref()
            .unwrap()
            .save_recording(&audiorouter_storage::RecordingRecord {
                id: "ordinary-recording-run".into(),
                session_id: "session".into(),
                recorder_id: "normal-recorder".into(),
                path: ordinary.to_string_lossy().into_owned(),
                format: "wav".into(),
                channels: 1,
                sample_rate: 48_000,
                frames: 48,
                file_bytes: wav.len() as u64,
                start_time: "2026-09-22T00:00:00Z".into(),
                state: "completed".into(),
                missing: false,
                title: None,
                artist: None,
                comment: None,
                dither: false,
                conversion: "test".into(),
            })
            .unwrap();
        let rejected = plane.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "audioMedia.importTemporaryRecording".into(),
            params: Some(json!({"recordingId":"ordinary-recording-run"})),
        });
        assert!(rejected.error.is_some());
        assert!(ordinary.exists(), "ordinary recordings are not consumed");
        let _ = std::fs::remove_file(ordinary);
    }
}
