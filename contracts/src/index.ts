// Generated contract surface for the shared AudioRouter JSON-RPC API.
// Keep this file aligned with crates/domain and crates/protocol; UI/CLI/MCP
// adapters must use these shapes rather than private request paths.

export type EntityId = string;

export type NodeKind =
  | "physicalInput"
  | "applicationCapture"
  | "endpointLoopback"
  | "physicalOutput"
  | "virtualRenderSource"
  | "virtualCaptureSink"
  | "testSignal"
  | "mixer"
  | "gain"
  | "mute"
  | "meter"
  | "parametricEq"
  | "compressor"
  | "gate"
  | "limiter"
  | "delay"
  | "graphicEq"
  | "pitch"
  | "recorder"
  | "plugin"
  | "audioFile";

export type PortDirection = "input" | "output";

export interface Port {
  name: string;
  direction: PortDirection;
  channels: 1 | 2;
}

export interface Node {
  id: EntityId;
  kind: NodeKind;
  typeVersion: 1;
  name: string;
  enabled: boolean;
  bypass: boolean;
  parameters: Record<string, boolean | number | string>;
  ports: Port[];
}

export interface Edge {
  id: EntityId;
  sourceNode: EntityId;
  sourcePort: string;
  destinationNode: EntityId;
  destinationPort: string;
  matrix: number[];
  enabled: boolean;
}

export interface Session {
  id: EntityId;
  name: string;
  schemaVersion: 1;
  revision: number;
  nodes: Node[];
  edges: Edge[];
}

export interface SessionListPage {
  items: Session[];
  nextCursor: EntityId | null;
}

export interface RoutePath {
  nodes: EntityId[];
  edges: EntityId[];
  channelMaps: number[][];
  latencySamples: number;
}

export interface RouteInspection {
  destinationNode: EntityId;
  reachable: boolean;
  complete: boolean;
  paths: RoutePath[];
}

export interface PluginScanEntry {
  path: string;
  identity: {
    path: string;
    binaryPath: string;
    format: "vst3" | "vst2" | "unknown";
    architecture: "x64" | "x86" | "arm64" | "unknown";
    fileBytes: number;
    sha256: string;
    vendor: string | null;
    version: string | null;
    classIds: string[];
    compatibility:
      | "supportedVst3X64"
      | "supportedVst2X64Gated"
      | "unsupportedFormat";
  } | null;
  error: string | null;
  errorCode:
    | "outsideConfiguredRoot"
    | "unsupportedExtension"
    | "missing"
    | "tooLarge"
    | "notPe"
    | "unsupportedArchitecture"
    | "cancelled"
    | "deadlineExceeded"
    | "io"
    | null;
}

export interface PluginScanResult {
  directory: string;
  entries: PluginScanEntry[];
}

export interface PluginParameterDescriptor {
  parameterId: number;
  title: string;
  defaultValue: number;
  minimum: number;
  maximum: number;
}

export interface PluginParametersResult {
  path: string;
  sha256: string;
  format: "vst3" | "vst2";
  parameters: PluginParameterDescriptor[];
}

export interface GraphPlanResult {
  planId: EntityId;
  baseRevision: number;
  expiresInMs: number;
  diff: unknown[];
  affectedDestinations: string[];
  warnings: string[];
  requiredScopes: string[];
}

export interface GraphCommitResult {
  sessionId: EntityId;
  revision: number;
  idempotentReplay?: boolean;
  activation?:
    | { state: "pending"; runtime: "fake" }
    | { state: "running"; generation: number; runtime: "fake" };
}

export interface OperationCompleted {
  operationId: EntityId;
  operation: string;
  status: "completed";
  durable: boolean;
  revision: number;
  createdAt: number | null;
  result: Record<string, unknown>;
}

export interface OperationUnknown {
  operationId: EntityId;
  status: "unknown";
  durable: false;
}

export interface OperationCancelled {
  operationId: EntityId;
  status: "completed";
  cancelled: false;
  reason: "alreadyCompleted";
}

export type RecordingState = "armed" | "recording" | "paused" | "completed" | "failed";

export interface RecordingMetadata {
  title: string | null;
  artist: string | null;
  comment: string | null;
  dither: boolean;
  conversion: string;
}

export interface RecordingRow extends RecordingMetadata {
  id: EntityId;
  sessionId: EntityId;
  recorderId: EntityId;
  path: string;
  format: "wav" | "flac";
  channels: 1 | 2;
  sampleRate: 44100 | 48000;
  frames: number;
  fileBytes: number;
  startTime: string;
  state: RecordingState;
  missing: boolean;
}

export interface RecorderLifecycleResult {
  sessionId: EntityId;
  state: "idle" | "armed" | "recording" | "paused" | "stopping" | "completed" | "failed";
  parts: Array<{ index: number; startFrame: number; endFrame?: number | null }>;
  pauses: Array<{ startFrame: number; endFrame: number }>;
  lastFrame?: number | null;
}

export type RecorderFileFormat = "wavPcm16" | "wavPcm24" | "wavFloat32" | "flac16" | "flac24" | "mp3";

export interface RecorderCreateResult {
  sessionId: EntityId;
  nodeId?: EntityId | null;
  recorderId: EntityId;
  format: RecorderFileFormat;
  path: string;
  state: "idle";
  armed: false;
}

export interface TemporaryAudioImportResult {
  mediaId: EntityId;
  fileName: "Temporary voice take.wav";
  format: "wav";
  durationMs: number;
  channels: 1 | 2;
  sampleRateHz: number;
  expiresAt: number;
  sourceRemoved: true;
}

export interface RecordingListPage {
  items: RecordingRow[];
  nextCursor: string | null;
}

export interface DeviceInfo {
  id: string;
  name: string;
  direction: "capture" | "render";
  state: "active";
  defaultRoles: Array<"console" | "multimedia" | "communications">;
  format: {
    sampleRateHz: number;
    channels: number;
    bitsPerSample: number;
    formatTag: number;
    bytesPerFrame: number;
  };
  periods: {
    default100ns: number;
    minimum100ns: number;
  };
}

export interface NativeEndpointPrepareResult {
  sessionId: EntityId;
  state: "configured-stopped";
  captureEndpointId: string;
  renderEndpointId: string;
}

export interface NativeOutputFanoutPrepareResult {
  sessionId: EntityId;
  generation: number;
  state: "configured-stopped";
  renderEndpointIds: string[];
  outputCount: number;
}

export type NativeMultiInputSourceBinding =
  | { kind: "physical"; endpointId: string }
  | {
      kind: "application";
      processId: number;
      executable: string;
      executablePath: string | null;
      creationTime100ns: string;
      mode: "include" | "exclude";
    };

export interface NativeMultiInputPreparedSource {
  kind: "physical" | "application";
  endpointId?: string;
  processId?: number;
  executable?: string;
}

export interface NativeMultiInputPrepareResult {
  sessionId: EntityId;
  generation: number;
  state: "configured-stopped";
  sources: NativeMultiInputPreparedSource[];
  sourceNodeIds: EntityId[];
  branchNodeIds: EntityId[];
}

export interface NativeBridgePrepareResult {
  busId: EntityId;
  generation: number;
  state: "configured-stopped";
  directions: ["renderSource", "captureSink"];
}

export interface NativeBridgeDetachResult {
  busId: EntityId;
  state: "detached";
}

export interface NativeBridgeHeartbeatResult {
  state: "healthy";
  bindings: number;
}

export type NativeEndpointRebindResult = NativeEndpointPrepareResult;

export interface NativeEndpointDetachResult {
  sessionId: EntityId;
  state: "detached";
}

export interface NativeDuplexDetachResult {
  sessionId: EntityId;
  state: "detached";
}

export interface NativeApplicationPrepareResult {
  sessionId: EntityId;
  state: "configured-stopped";
  processId: number;
  executable: string;
  executablePath: string | null;
  creationTime100ns: string;
  mode: "include" | "exclude";
  renderEndpointId: string;
}

export interface NativeEndpointPumpResult {
  sessionId: EntityId;
  generation: number;
  packets: number;
  capturedFrames: number;
  processedQuanta: number;
  renderedFrames: number;
  droppedRenderFrames: number;
  renderBackpressureEvents: number;
  recorderChunksDrained: number;
}

export interface NativeDuplexPumpResult {
  sessionId: EntityId;
  generation: number;
  input: Omit<NativeEndpointPumpResult, "sessionId" | "generation" | "recorderChunksDrained">;
  output: Omit<NativeEndpointPumpResult, "sessionId" | "generation" | "recorderChunksDrained">;
}

export interface NativeRenderSourcePumpResult {
  sessionId: EntityId;
  generation: number;
  packets: number;
  processedQuanta: number;
  renderedFrames: number;
  droppedRenderFrames: number;
}

export interface NativeMultiInputPumpResult {
  sessionId: EntityId;
  generation: number;
  inputs: number;
  capturedFrames: number;
  submittedQuanta: number;
  outputCount: number;
  deliveredQuanta: number;
  renderedFrames: number;
  renderBackpressureEvents: number;
}

export interface NativeMultiInputBranchBindingResult {
  sessionId: EntityId;
  generation: number;
  branchNodeIds: EntityId[];
  boundBranches: number;
}

export interface InactiveDeviceInfo {
  id: string;
  name: string;
  direction: "capture" | "render";
  state: "disabled" | "unplugged" | "notPresent" | "unknown";
  defaultRoles: Array<"console" | "multimedia" | "communications">;
}

export type DeviceListItem = DeviceInfo | InactiveDeviceInfo;

export interface DeviceListPage {
  items: DeviceListItem[];
  nextCursor: string | null;
}

export interface VirtualDeviceInfo {
  id: string;
  name: string;
  driverInstanceId: string | null;
  direction: "bidirectional";
  channels: 2;
  enabled: boolean;
  availability: {
    status: "unavailable";
    reason: string;
  };
  endpointIds: {
    render: string | null;
    capture: string | null;
  };
  capabilities: {
    render: false;
    capture: false;
    channels: 2;
  };
  privilege: "deviceAdministration";
  restartRequired: false;
  clientImpacts: string[];
  leaseOwner: string | null;
}

export interface VirtualDeviceListPage {
  items: VirtualDeviceInfo[];
  nextCursor: string | null;
}

export interface VirtualBusRoute {
  busId: EntityId;
  producerSessionId: EntityId;
  consumerSessionId: EntityId;
}

export interface VirtualRouteListResult {
  revision: number;
  routes: VirtualBusRoute[];
}

export interface VirtualRouteReplaceResult {
  state: "applied";
  revision: number;
  routes: VirtualBusRoute[];
}

export type VirtualDeviceOperation =
  | { action: "create"; id: EntityId; name: string }
  | { action: "rename"; id: EntityId; name: string }
  | { action: "setEnabled"; id: EntityId; enabled: boolean }
  | { action: "delete"; id: EntityId };

export interface VirtualDevicePlanResult {
  planId: EntityId;
  expiresInMs: number;
  operation: VirtualDeviceOperation;
  availability: { status: "unavailable"; reason: string };
  requiredScopes: string[];
  warnings: string[];
}

export interface VirtualDeviceApplyResult {
  planId: EntityId;
  state: "applied";
  availability: { status: "unavailable"; reason: string };
  operation: VirtualDeviceOperation;
}

export interface VirtualDeviceProvisionResult {
  operationId: EntityId;
  state: "completed";
  busId: EntityId;
  driverInstanceId: string;
  availability: { status: "unavailable"; reason: string };
}

export interface VirtualDeviceRemoveResult {
  operationId: EntityId;
  state: "completed";
  busId: EntityId;
  driverInstanceId: null;
  availability: { status: "unavailable"; reason: string };
}

export interface GraphHistoryPage {
  items: Session[];
  nextCursor: string | null;
}

export type PermissionScope =
  | "read"
  | "graphWrite"
  | "sessionControl"
  | "capture"
  | "record"
  | "startupWrite"
  | "deviceAdministration";

export type SideEffectClass =
  | "readOnly"
  | "planOnly"
  | "mutating"
  | "externalOperation";

export interface MethodDescription {
  name: string;
  description: string;
  permission: PermissionScope;
  sideEffect: SideEffectClass;
  inputSchema: unknown;
  outputSchema: unknown;
}

export interface DiscoveryDocument {
  protocolVersion: { major: number; minor: number };
  schemaVersion: number;
  build: string;
  methods: MethodDescription[];
  nodeTypes: Array<{
    type: `${NodeKind}@${number}`;
    availability: { status: "available" | "unavailable"; reason?: string };
    realtimeCostClass: string;
    latencySamples: number;
    parameters: Array<{
      name: string;
      type: string;
      unit?: string;
      minimum?: number;
      maximum?: number;
      enum?: string[];
      default?: boolean | number | string;
    }>;
  }>;
  processors: Array<{
    id: string;
    version: number;
    category: string;
    availability: { status: "available" | "unavailable"; reason?: string };
    latencySamples: number;
    parameters: Array<{
      name: string;
      type: string;
      unit?: string;
      minimum?: number;
      maximum?: number;
      enum?: string[];
      default?: boolean | number | string;
    }>;
  }>;
  presets: {
    voiceChains: Array<{
      id: string;
      version: number;
      name: string;
      description: string;
    }>;
    eq: Array<{
      id: string;
      version: number;
      name: string;
      description: string;
    }>;
  };
  limits: {
    maxNodesPerSession: number;
    maxEdgesPerSession: number;
    maxNodesGlobal: number;
    maxEdgesGlobal: number;
    maxActiveSessions: number;
    maxVirtualBuses: number;
    maxEntityIdBytes: number;
    maxDisplayNameBytes: number;
    maxPortNameBytes: number;
    maxPortsPerNode: number;
    maxChannelMatrixCoefficients: number;
    maxControlValueDepth: number;
    maxControlStringBytes: number;
    maxControlValueCount: number;
    maxMethodNameBytes: number;
    maxRequestIdBytes: number;
  };
  events: {
    stateCategories: string[];
    meterReplay: false;
    retention: { maxEvents: number; maxAgeSeconds: number };
  };
}

export interface StatusSnapshot {
  build: string;
  audio: "available" | "unavailable";
  deviceDiscovery: "available";
  reason: string;
  storage: "memory" | "sqlite";
  sessionCount: number;
  activeSessionCount: number;
  activeSessionIds: EntityId[];
  privacyMute: {
    muted: boolean;
    persistence: "durable" | "memory";
    audioEffect: string;
  };
  recovery: {
    safeMode: boolean;
    recentCrashes: number;
    persistence: "durable" | "memory";
  };
  eventCursor: { backendEpoch: number; latestSequence: number };
}

export interface DiagnosticsSnapshot {
  build: string;
  backend: "control-plane";
  storage: "memory" | "sqlite";
  audio: { state: "available" | "unavailable"; reason: string };
  nativeAdapter: "implemented-not-activated" | "configured-stopped" | "running";
  nativeAdapterKind: "endpoint" | "duplex" | "render-source" | "multi-input" | null;
  nativeSessionId: EntityId | null;
  schedulerTelemetry: {
    activeGeneration: number | null;
    activeSampleRateHz: number | null;
    inputOverruns: number;
    inputUnderruns: number;
    outputOverruns: number;
    outputUnderruns: number;
    processedQuanta: number;
    repairedSamples: number;
    xruns: number;
    processingTimeNsTotal: number;
    processingTimeNsMax: number;
    deadlineMisses: number;
    deadlineLatenessNsTotal: number;
    deadlineLatenessNsMax: number;
  } | null;
  nodeTelemetry: Array<{
    nodeId: EntityId;
    kind: string;
    meter: {
      peakDb: number;
      rmsDb: number;
      clippedSamples: number;
      channelPeakDb: number[];
      channelRmsDb: number[];
      channelClippedSamples: number[];
    } | null;
    processor: {
      gainReductionDb: number[];
      gateOpen: boolean[];
    } | null;
    plugin: {
      state: "unknown" | "stopped" | "running" | "failed" | "quarantined";
      failureCount: number;
    } | null;
  }>;
  privacyMute: { muted: boolean; persistence: "durable" | "memory" };
  recovery: { safeMode: boolean; recentCrashes: number; persistence: "durable" | "memory" };
  eventLog: { latestSequence: number; retained: number };
  redacted: true;
}

export interface StartupStatus {
  enabled: boolean;
  registration: "unavailable";
  reason: string;
}

export interface StartupPlanResult {
  planId: EntityId;
  enabled: boolean;
  registration: "unavailable";
  reason: string;
  requiredScopes: string[];
  warnings: string[];
}

export interface StartupApplyResult {
  planId: EntityId;
  state: "unavailable";
  registration: "unavailable";
  reason: string;
}

export type OsTransition = "lock" | "signOut" | "sleep" | "resume";

export interface OsTransitionResult {
  transition: OsTransition;
  action: "keepRunning" | "stopAndRelease" | "revalidateBeforeRestart" | "remainStopped";
  endpointInventory: "refreshed" | "notStarted";
  nativeSessionIds: EntityId[];
  sessionIds: EntityId[];
}

export interface RecoveryClearResult {
  safeMode: false;
  recentCrashes: 0;
  persistence: "durable" | "memory";
}

export interface HandshakeResult {
  compatible: true;
  requested: { major: number; minor: number };
  negotiated: { major: 1; minor: 0 };
  schemaVersion: number;
}

export interface SessionCreateResult {
  session: Session;
  state: "stopped";
}

export interface SessionDeleteResult {
  sessionId: EntityId;
  deleted: true;
}

export interface SessionImportPlanResult {
  planId: EntityId;
  expiresInMs: number;
  session: Session;
}

export interface SessionImportCommitResult {
  session: Session;
  state: "stopped";
  imported: true;
}

export interface SessionStartResult {
  sessionId: EntityId;
  state: "running";
  generation: number;
  runtime: "fake" | "native";
  preview?: boolean;
  savedRevision?: number;
}

export interface SessionStopResult {
  sessionId: EntityId;
  state: "stopped";
  runtime: "fake" | "native";
  recorders: Array<{
    sessionId: EntityId;
    state: "completed";
    fileFinalized: true;
    recoverable: false;
  }>;
}

export interface SystemQuitResult {
  state: "stopped";
  sessions: SessionStopResult[];
  recorders: RecorderLifecycleResult[];
}

export interface GraphUndoPlanResult {
  planId: EntityId;
  baseRevision: number;
  expiresInMs: number;
}

export interface PrivacyMuteResult {
  muted: boolean;
  persistence: "durable" | "memory";
  audioEffect: string;
}

export interface ClientAuthorizeResult {
  clientId: EntityId;
  role: "observer" | "editor" | "operator";
  revoked: false;
}

export interface ClientRevokeResult {
  clientId: EntityId;
  revoked: true;
  changed: boolean;
}

export interface RecordingMetadataResult {
  recordingId: EntityId;
  updated: true;
}

export interface RecordingRenameResult {
  recordingId: EntityId;
  renamed: true;
  path: string;
  fileAction: "renamed";
}

export interface RecordingRemoveResult {
  recordingId: EntityId;
  removed: true;
  fileAction: "none";
}

export type RecordingRevealResult =
  | { recordingId: EntityId; path: string; revealed: true }
  | { recordingId: EntityId; path: string; revealed: false; reason: "missing" };

export interface RecordingCheckpoint {
  version: 1;
  state: "Idle" | "Armed" | "Recording" | "Paused" | "Stopping" | "Completed" | "Failed";
  parts: Array<Record<string, unknown>>;
  pauses: Array<Record<string, unknown>>;
  pause_start: number | null;
  last_frame: number | null;
  stop_frame: number | null;
}

export interface RecordingRecoveryItem {
  recordingId: EntityId;
  status: "missing" | "available" | "invalid";
  checkpoint?: RecordingCheckpoint;
}

export interface RecordingRecoveryList {
  items: RecordingRecoveryItem[];
  nextCursor: EntityId | null;
}

export type RecordingRecoverySingleResult =
  | { recordingId: EntityId; status: "missing" }
  | { recordingId: EntityId; status: "available"; checkpoint: RecordingCheckpoint };

export type RecordingRecoveryResult = RecordingRecoverySingleResult | RecordingRecoveryList;

export type RecordingPreview =
  | {
      status: "present";
      format: "wav";
      channels: number;
      sampleRate: number;
      frames: number;
      dataBytes: number;
      fileBytes: number;
    }
  | {
      status: "present";
      format: "flac";
      channels: number;
      sampleRate: number;
      bitsPerSample: number;
      frames: number;
      fileBytes: number;
    }
  | {
      status: "present";
      format: "mp3";
      fileBytes: number;
    }
  | { status: "missing" | "invalid" };

export interface RecordingPreviewResult {
  recordingId: EntityId;
  preview: RecordingPreview;
}

export type RecordingRecycleResult =
  | { recordingId: EntityId; path: string; fileAction: "none"; reason: "missing" | "recycleUnavailable" }
  | { recordingId: EntityId; path: string; fileAction: "recycle"; preview: true }
  | { recordingId: EntityId; path: string; fileAction: "recycled"; missing: true };

export type StateEventCategory =
  | "session.created"
  | "session.deleted"
  | "graph.committed"
  | "runtime.crashed"
  | "runtime.started"
  | "runtime.activated"
  | "runtime.stopped"
  | "devices.changed"
  | "devices.bindingInvalidated"
  | "recovery.safeModeCleared"
  | "privacy.muteEnabled"
  | "privacy.muteDisabled"
  | "virtualDevice.changed"
  | "virtualBridge.failed"
  | "virtualBridge.expired"
  | "recorder.changed"
  | "recording.metadataChanged"
  | "recording.renamed"
  | "recording.entryRemoved"
  | "recording.recycled";

export interface StateEvent {
  sequence: number;
  backendEpoch: number;
  resourceRevision: number;
  operationId: string | null;
  category: StateEventCategory;
  sessionId: EntityId | null;
}

export interface EventsSubscribeResult {
  backendEpoch: number;
  events: StateEvent[];
  nextSequence: number;
  resyncRequired?: boolean;
  reason?: "backendEpochChanged";
  snapshot?: { sessions: SessionListPage };
}

export interface EventsSubscribeParams {
  afterSequence?: number;
  backendEpoch?: number;
  categories?: StateEventCategory[];
  limit?: number;
  sessionId?: EntityId;
}

export interface ApplicationInfo {
  processId: number;
  executable: string;
  executablePath: string | null;
  creationTime100ns: string | null;
  audioActivity: "active" | "inactive" | "none";
  captureCapability: "observed" | "notObserved";
  audioSessionCount: number;
  activeAudioSessionCount: number;
  captureSessionCount: number;
  renderSessionCount: number;
  audioDisplayNames: string[];
}

export interface JsonRpcRequest<Params = unknown> {
  jsonrpc: "2.0";
  id?: string | number | null;
  method: string;
  params?: Params;
}

export interface JsonRpcSuccess<Result = unknown> {
  jsonrpc: "2.0";
  id: string | number | null;
  result: Result;
}

export interface JsonRpcError {
  jsonrpc: "2.0";
  id: string | number | null;
  error: {
    code: number;
    message: string;
    data?: ApplicationErrorData;
  };
}

export interface ApplicationErrorData {
  code: string;
  fieldPath: string | null;
  resourceIds: EntityId[];
  retryable: boolean;
  remediation: string;
  /** Unsigned Windows HRESULT when the failure originated in the audio API. */
  hresult?: number;
  retryAfterMs?: number;
}

export type JsonRpcResponse<Result = unknown> =
  | JsonRpcSuccess<Result>
  | JsonRpcError;

export type ImplementedMethod =
  | "system.describe"
  | "system.handshake"
  | "status.get"
  | "system.diagnostics"
  | "system.quit"
  | "system.osTransition"
  | "clients.list"
  | "clients.authorize"
  | "clients.revoke"
  | "operations.get"
  | "operations.cancel"
  | "recordings.list"
  | "audioMedia.beginUpload"
  | "audioMedia.uploadChunk"
  | "audioMedia.finishUpload"
  | "audioMedia.importTemporaryRecording"
  | "audioMedia.delete"
  | "audioSources.transport"
  | "recorders.list"
  | "recorders.create"
  | "recorders.arm"
  | "recorders.start"
  | "recorders.pause"
  | "recorders.resume"
  | "recorders.split"
  | "recorders.stop"
  | "recordings.get"
  | "recordings.recovery"
  | "recordings.reveal"
  | "recordings.preview"
  | "recordings.setMetadata"
  | "recordings.rename"
  | "recordings.removeEntry"
  | "recordings.recycle"
  | "recovery.clearSafeMode"
  | "safety.setPrivacyMute"
  | "startup.get"
  | "startup.plan"
  | "startup.apply"
  | "devices.list"
  | "nativeEndpoints.prepare"
  | "nativeOutputs.prepare"
  | "nativeMultiInputs.prepare"
  | "nativeBridges.prepare"
  | "nativeBridges.detach"
  | "nativeBridges.heartbeat"
  | "nativeEndpoints.rebind"
  | "nativeEndpoints.detach"
  | "nativeDuplex.detach"
  | "nativeApplications.prepare"
  | "nativeEndpoints.pump"
  | "nativeDuplex.pump"
  | "nativeRenderSources.pump"
  | "nativeMultiInputs.pump"
  | "nativeMultiInputs.bindBranches"
  | "plugins.scan"
  | "plugins.list"
  | "plugins.retry"
  | "plugins.inspect"
  | "plugins.parameters"
  | "virtualDevices.list"
  | "virtualDevices.plan"
  | "virtualDevices.apply"
  | "virtualDevices.provision"
  | "virtualDevices.remove"
  | "virtualRoutes.list"
  | "virtualRoutes.replace"
  | "apps.list"
  | "applications.list"
  | "nodes.types"
  | "routes.inspect"
  | "graph.history"
  | "graph.undoPlan"
  | "events.subscribe"
  | "nodes.describe"
  | "presets.list"
  | "processors.list"
  | "processors.response"
  | "sessions.get"
  | "sessions.export"
  | "sessions.importPlan"
  | "sessions.importCommit"
  | "sessions.list"
  | "sessions.create"
  | "sessions.duplicate"
  | "sessions.delete"
  | "graph.plan"
  | "graph.commit"
  | "session.start"
  | "sessions.start"
  | "session.stop"
  | "sessions.stop";

export type MethodParams = {
  "system.describe": undefined;
  "system.handshake": { protocolVersion: { major: number; minor: number } };
  "status.get": undefined;
  "system.diagnostics": undefined;
  "system.quit": { idempotencyKey: string };
  "system.osTransition": { transition: OsTransition; idempotencyKey: string };
  "clients.list": undefined;
  "clients.authorize": { clientId: string; role: "observer" | "editor" | "operator"; idempotencyKey: string };
  "clients.revoke": { clientId: string; idempotencyKey: string };
  "operations.get": { operationId: string };
  "operations.cancel": { operationId: string; idempotencyKey?: string };
  "recordings.list":
    | { sessionId?: EntityId | null; cursor?: string | null; limit?: number }
    | undefined;
  "audioMedia.beginUpload": { fileName: string; sizeBytes: number };
  "audioMedia.uploadChunk": { uploadId: EntityId; chunkIndex: number; dataBase64: string };
  "audioMedia.finishUpload": { uploadId: EntityId };
  "audioMedia.importTemporaryRecording": { recordingId: EntityId };
  "audioMedia.delete": { mediaId: EntityId };
  "audioSources.transport": { sessionId: EntityId; nodeId: EntityId; action: "play" | "pause" | "stop" | "status" };
  "recorders.list": undefined;
  "recorders.create": {
    sessionId: EntityId;
    nodeId?: EntityId | null;
    recorderId: EntityId;
    format: RecorderFileFormat;
    sequence: number;
    channels: 1 | 2;
    sampleRate: 44100 | 48000;
    dither?: boolean;
    queueCapacity: number;
    maximumChunksPerPass: number;
    idempotencyKey: string;
  };
  "recorders.arm": { sessionId: EntityId; nodeId?: EntityId; idempotencyKey?: string };
  "recorders.start": { sessionId: EntityId; nodeId?: EntityId; frame: number; idempotencyKey?: string };
  "recorders.pause": { sessionId: EntityId; nodeId?: EntityId; frame: number; idempotencyKey?: string };
  "recorders.resume": { sessionId: EntityId; nodeId?: EntityId; frame: number; idempotencyKey?: string };
  "recorders.split": { sessionId: EntityId; nodeId?: EntityId; frame: number; idempotencyKey?: string };
  "recorders.stop": { sessionId: EntityId; nodeId?: EntityId; frame: number; idempotencyKey?: string };
  "recordings.get": { recordingId: EntityId };
  "recordings.recovery": { recordingId?: EntityId; cursor?: EntityId; limit?: number } | undefined;
  "recordings.reveal": { recordingId: EntityId };
  "recordings.preview": { recordingId: EntityId };
  "recordings.setMetadata": {
    recordingId: EntityId;
    title?: string | null;
    artist?: string | null;
    comment?: string | null;
    idempotencyKey?: string;
  };
  "recordings.rename": { recordingId: EntityId; newPath: string; idempotencyKey?: string };
  "recordings.removeEntry": { recordingId: EntityId; idempotencyKey?: string };
  "recordings.recycle": { recordingId: EntityId; confirm?: boolean; idempotencyKey?: string };
  "recovery.clearSafeMode": { idempotencyKey?: string } | undefined;
  "safety.setPrivacyMute": { muted: boolean; idempotencyKey?: string };
  "startup.get": undefined;
  "startup.plan": { enabled: boolean };
  "startup.apply": { planId: EntityId; idempotencyKey: string };
  "devices.list": { cursor?: string; limit?: number; includeInactive?: boolean } | undefined;
  "nativeEndpoints.prepare": { sessionId: EntityId; captureEndpointId: string; renderEndpointId: string };
  "nativeOutputs.prepare": { sessionId: EntityId; generation: number; renderEndpointIds: string[] };
  "nativeMultiInputs.prepare": { sessionId: EntityId; generation: number; sources: NativeMultiInputSourceBinding[] };
  "nativeBridges.prepare": { busId: EntityId; generation: number; devicePath: string; renderMappingPath: string; captureMappingPath: string; leaseMs?: number };
  "nativeBridges.detach": { busId: EntityId };
  "nativeBridges.heartbeat": undefined;
  "nativeEndpoints.rebind": { sessionId: EntityId; captureEndpointId: string; renderEndpointId: string };
  "nativeEndpoints.detach": { sessionId: EntityId };
  "nativeDuplex.detach": { sessionId: EntityId };
  "nativeApplications.prepare": { sessionId: EntityId; processId: number; executable: string; executablePath?: string | null; creationTime100ns: string; mode: "include" | "exclude"; renderEndpointId: string };
  "nativeEndpoints.pump": { sessionId: EntityId; generation: number; maxPackets?: number };
  "nativeDuplex.pump": { sessionId: EntityId; generation: number; maxInputQuanta?: number; maxOutputPackets?: number };
  "nativeRenderSources.pump": { sessionId: EntityId; generation: number; maxQuanta?: number };
  "nativeMultiInputs.pump": { sessionId: EntityId; generation: number; maxPackets?: number };
  "nativeMultiInputs.bindBranches": { sessionId: EntityId; generation: number; branchNodeIds: EntityId[] };
  "plugins.scan": { directory: string };
  "plugins.list": { directory: string };
  "plugins.retry": { directory: string; idempotencyKey: string };
  "plugins.inspect": { path: string };
  "plugins.parameters": { path: string };
  "virtualDevices.list": { cursor?: string; limit?: number } | undefined;
  "virtualDevices.plan": { operation: VirtualDeviceOperation };
  "virtualDevices.apply": { planId: EntityId; idempotencyKey: string };
  "virtualDevices.provision": { busId: EntityId; instanceId: string; idempotencyKey: string };
  "virtualDevices.remove": { busId: EntityId; idempotencyKey: string };
  "virtualRoutes.list": undefined;
  "virtualRoutes.replace": {
    baseRevision: number;
    routes: VirtualBusRoute[];
    idempotencyKey: string;
  };
  "apps.list": undefined;
  "applications.list": undefined;
  "nodes.types": undefined;
  "routes.inspect": { sessionId: EntityId; destinationNode: EntityId };
  "graph.history": { sessionId: EntityId; cursor?: string; limit?: number };
  "graph.undoPlan": { sessionId: EntityId; baseRevision: number };
  "events.subscribe":
    | EventsSubscribeParams
    | undefined;
  "nodes.describe": undefined;
  "presets.list": undefined;
  "processors.list": undefined;
  "processors.response": {
    sampleRateHz: number;
    bands: Array<{
      enabled?: boolean;
      type: "peaking" | "lowShelf" | "highShelf" | "lowPass" | "highPass" | "notch";
      frequencyHz: number;
      q: number;
      gainDb: number;
    }>;
    frequenciesHz: number[];
  };
  "sessions.get": { sessionId: EntityId };
  "sessions.export": { sessionId: EntityId };
  "sessions.importPlan": { session: Session };
  "sessions.importCommit": { planId: EntityId; idempotencyKey: string };
  "sessions.list": { cursor?: string; limit?: number } | undefined;
  "sessions.create": { session: Session; idempotencyKey?: string };
  "sessions.duplicate": {
    sourceSessionId: EntityId;
    sessionId: EntityId;
    name?: string;
    idempotencyKey?: string;
  };
  "sessions.delete": { sessionId: EntityId; idempotencyKey?: string };
  "graph.plan": { sessionId: EntityId; baseRevision: number; candidate: Session };
  "graph.commit": {
    planId: EntityId;
    baseRevision: number;
    idempotencyKey: string;
    acknowledgments?: string[] | null;
  };
  "session.start": { sessionId: EntityId; idempotencyKey?: string; candidate?: Session };
  "sessions.start": { sessionId: EntityId; idempotencyKey?: string; candidate?: Session };
  "session.stop": { sessionId: EntityId; idempotencyKey?: string };
  "sessions.stop": { sessionId: EntityId; idempotencyKey?: string };
};

export type MethodResult = {
  "system.describe": DiscoveryDocument;
  "system.handshake": {
    compatible: true;
    requested: { major: number; minor: number };
    negotiated: { major: 1; minor: 0 };
    schemaVersion: number;
  };
  "status.get": StatusSnapshot;
  "system.diagnostics": DiagnosticsSnapshot;
  "system.quit": SystemQuitResult;
  "system.osTransition": OsTransitionResult;
  "clients.list": Array<{ clientId: string; role: string; revoked: boolean }>;
  "clients.authorize": ClientAuthorizeResult;
  "clients.revoke": ClientRevokeResult;
  "operations.get": OperationCompleted | OperationUnknown;
  "operations.cancel": OperationCancelled;
  "recordings.list": RecordingRow[] | RecordingListPage;
  "audioMedia.beginUpload": { uploadId: EntityId; chunkBytes: number };
  "audioMedia.uploadChunk": { receivedBytes: number; nextChunkIndex: number };
  "audioMedia.finishUpload": { mediaId: EntityId; fileName: string; format: "wav" | "mp3"; durationMs: number; channels: 1 | 2; sampleRateHz: number };
  "audioMedia.importTemporaryRecording": TemporaryAudioImportResult;
  "audioMedia.delete": { deleted: boolean };
  "audioSources.transport": { sessionId: EntityId; nodeId: EntityId; state: "playing" | "paused" | "stopped"; loop: boolean };
  "recorders.list": Array<{ sessionId: EntityId; nodeId?: EntityId | null; state: "idle" | "armed" | "recording" | "paused" | "stopping" | "completed" | "failed"; lastFrame: number | null }>;
  "recorders.create": RecorderCreateResult;
  "recorders.arm": RecorderLifecycleResult;
  "recorders.start": RecorderLifecycleResult;
  "recorders.pause": RecorderLifecycleResult;
  "recorders.resume": RecorderLifecycleResult;
  "recorders.split": RecorderLifecycleResult;
  "recorders.stop": RecorderLifecycleResult;
  "recordings.get": RecordingRow;
  "recordings.recovery": RecordingRecoveryResult;
  "recordings.reveal": RecordingRevealResult;
  "recordings.preview": RecordingPreviewResult;
  "recordings.setMetadata": RecordingMetadataResult;
  "recordings.rename": RecordingRenameResult;
  "recordings.removeEntry": RecordingRemoveResult;
  "recordings.recycle": RecordingRecycleResult;
  "recovery.clearSafeMode": RecoveryClearResult;
  "safety.setPrivacyMute": PrivacyMuteResult;
  "startup.get": StartupStatus;
  "startup.plan": StartupPlanResult;
  "startup.apply": StartupApplyResult;
  "devices.list": DeviceInfo[] | DeviceListPage;
  "nativeEndpoints.prepare": NativeEndpointPrepareResult;
  "nativeOutputs.prepare": NativeOutputFanoutPrepareResult;
  "nativeMultiInputs.prepare": NativeMultiInputPrepareResult;
  "nativeBridges.prepare": NativeBridgePrepareResult;
  "nativeBridges.detach": NativeBridgeDetachResult;
  "nativeBridges.heartbeat": NativeBridgeHeartbeatResult;
  "nativeEndpoints.rebind": NativeEndpointRebindResult;
  "nativeEndpoints.detach": NativeEndpointDetachResult;
  "nativeDuplex.detach": NativeDuplexDetachResult;
  "nativeApplications.prepare": NativeApplicationPrepareResult;
  "nativeEndpoints.pump": NativeEndpointPumpResult;
  "nativeDuplex.pump": NativeDuplexPumpResult;
  "nativeRenderSources.pump": NativeRenderSourcePumpResult;
  "nativeMultiInputs.pump": NativeMultiInputPumpResult;
  "nativeMultiInputs.bindBranches": NativeMultiInputBranchBindingResult;
  "plugins.scan": PluginScanResult;
  "plugins.list": PluginScanResult;
  "plugins.retry": PluginScanResult;
  "plugins.inspect": PluginScanEntry;
  "plugins.parameters": PluginParametersResult;
  "virtualDevices.list": VirtualDeviceInfo[] | VirtualDeviceListPage;
  "virtualDevices.plan": VirtualDevicePlanResult;
  "virtualDevices.apply": VirtualDeviceApplyResult;
  "virtualDevices.provision": VirtualDeviceProvisionResult;
  "virtualDevices.remove": VirtualDeviceRemoveResult;
  "virtualRoutes.list": VirtualRouteListResult;
  "virtualRoutes.replace": VirtualRouteReplaceResult;
  "apps.list": ApplicationInfo[];
  "applications.list": ApplicationInfo[];
  "nodes.types": DiscoveryDocument["nodeTypes"];
  "routes.inspect": RouteInspection;
  "graph.history": GraphHistoryPage;
  "graph.undoPlan": GraphUndoPlanResult;
  "events.subscribe": EventsSubscribeResult;
  "nodes.describe": DiscoveryDocument["nodeTypes"];
  "presets.list": DiscoveryDocument["presets"];
  "processors.list": DiscoveryDocument["processors"];
  "processors.response": { frequenciesHz: number[]; magnitudeDb: number[] };
  "sessions.get": Session;
  "sessions.export": Session;
  "sessions.importPlan": { planId: EntityId; expiresInMs: number; session: Session };
  "sessions.importCommit": { session: Session; state: "stopped"; imported: true };
  "sessions.list": SessionListPage;
  "sessions.create": SessionCreateResult;
  "sessions.duplicate": SessionCreateResult;
  "sessions.delete": SessionDeleteResult;
  "graph.plan": GraphPlanResult;
  "graph.commit": GraphCommitResult;
  "session.start": SessionStartResult;
  "sessions.start": SessionStartResult;
  "session.stop": SessionStopResult;
  "sessions.stop": SessionStopResult;
};

export interface RpcTransport {
  send(request: JsonRpcRequest): Promise<JsonRpcResponse>;
}

export class AudioRouterRpcError extends Error {
  readonly code: number;
  readonly data?: ApplicationErrorData;

  constructor(error: JsonRpcError["error"]) {
    super(error.message);
    this.name = "AudioRouterRpcError";
    this.code = error.code;
    this.data = error.data;
  }
}

export interface AudioRouterClient {
  request<M extends ImplementedMethod>(
    method: M,
    params: MethodParams[M],
  ): Promise<MethodResult[M]>;
}

/** Create the shared typed client over any framed/local transport adapter. */
export function createAudioRouterClient(transport: RpcTransport): AudioRouterClient {
  let nextId = 1;
  return {
    async request(method, params) {
      const response = await transport.send({
        jsonrpc: "2.0",
        id: nextId++,
        method,
        ...(params === undefined ? {} : { params }),
      });
      if ("error" in response) {
        throw new AudioRouterRpcError(response.error);
      }
      return response.result as MethodResult[typeof method];
    },
  };
}
