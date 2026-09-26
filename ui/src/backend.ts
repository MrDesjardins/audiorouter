import { AudioRouterRpcError, createAudioRouterClient } from "@audiorouter/contracts";
import type {
  AudioRouterClient,
  ApplicationInfo,
  DeviceListItem,
  VirtualDeviceInfo,
  VirtualDeviceApplyResult,
  VirtualDeviceOperation,
  VirtualDevicePlanResult,
  VirtualDeviceProvisionResult,
  VirtualDeviceRemoveResult,
  VirtualBusRoute,
  VirtualRouteListResult,
  VirtualRouteReplaceResult,
  DiscoveryDocument,
  DiagnosticsSnapshot,
  EventsSubscribeResult,
  StateEventCategory,
  GraphCommitResult,
  GraphPlanResult,
  PrivacyMuteResult,
  PluginScanEntry,
  PluginScanResult,
  PluginParametersResult,
  RecoveryClearResult,
  RecordingMetadataResult,
  RecordingRenameResult,
  RecordingPreviewResult,
  RecordingRecoveryResult,
  RecordingRecoverySingleResult,
  RecordingRecoveryList,
  RecordingRevealResult,
  RecordingRecycleResult,
  RecordingRemoveResult,
  RecordingRow,
  RecorderLifecycleResult,
  RecorderCreateResult,
  RouteInspection,
  Session,
  SessionCreateResult,
  SessionDeleteResult,
  SessionStartResult,
  SessionStopResult,
  SessionImportPlanResult,
  SessionImportCommitResult,
  StartupStatus,
  StartupPlanResult,
  StartupApplyResult,
  OsTransition,
  OsTransitionResult,
  StatusSnapshot,
  RpcTransport,
  NativeOutputFanoutPrepareResult,
  NativeMultiInputPrepareResult,
  NativeRenderSourcePumpResult,
  NativeMultiInputPumpResult,
  NativeMultiInputBranchBindingResult,
} from "@audiorouter/contracts";
import type { MethodParams, MethodResult } from "@audiorouter/contracts";
import { demoSession, demoSessions } from "./fixtures";
import { markErrorMessage } from "./actionMessage";

export type ApplicationRow = ApplicationInfo;
export type ProcessorResponseParams = NonNullable<MethodParams["processors.response"]>;
export type ProcessorResponse = MethodResult["processors.response"];
export type RecorderStatus = MethodResult["recorders.list"][number];
export type ClientRow = MethodResult["clients.list"][number];
export type ClientAuthorizeResult = MethodResult["clients.authorize"];
export type ClientRevokeResult = MethodResult["clients.revoke"];
export type GraphHistoryPage = MethodResult["graph.history"];
export type GraphUndoPlanResult = MethodResult["graph.undoPlan"];

/** Formats structured backend failures without losing actionable audio guidance. */
/** Readable text for a caught error; the message is shown with the error tone. */
export function formatUiError(error: unknown, fallback: string): string {
  return markErrorMessage(formatUiErrorText(error, fallback));
}

function formatUiErrorText(error: unknown, fallback: string): string {
  if (!(error instanceof Error)) return fallback;
  if (/0x88890004/i.test(error.message) || (error instanceof AudioRouterRpcError && (error.data?.code === "deviceInvalidated" || (typeof error.data?.hresult === "number" && (error.data.hresult >>> 0) === 0x88890004)))) {
    return "The selected audio device changed or disconnected. Stop audio, refresh the device list, then select the exact input and output again. Play will reopen those devices. If one is missing, reconnect it before retrying.";
  }
  if (/0x8889000a/i.test(error.message) || (error instanceof AudioRouterRpcError && typeof error.data?.hresult === "number" && (error.data.hresult >>> 0) === 0x8889000A)) {
    return "The selected audio device is in use by another application. Choose a different output in Devices, or release this exact device in the application using it, then prepare it again.";
  }
  if (/native graph rejected: UnsupportedTopology/.test(error.message)) {
    return "This route includes an audio combination the current engine cannot play. Try a separate Test Signal → Physical Output route, or remove one source and connect the remaining source directly to the output. Your saved route was not changed.";
  }
  const permission = /permission denied:\s*([A-Za-z]+)/i.exec(error.message);
  if (permission || (error instanceof AudioRouterRpcError && error.data?.code === "permissionDenied")) {
    const scope = permission?.[1] ?? "the required";
    return `Permission denied: this app is not granted the ${scope} permission needed for this action, so the backend refused it. Nothing was changed.`;
  }
  if (!(error instanceof AudioRouterRpcError) || !error.data) return error.message;
  const { code, hresult, remediation, retryable } = error.data;
  const hresultText = typeof hresult === "number"
    ? `, HRESULT 0x${(hresult >>> 0).toString(16).padStart(8, "0").toUpperCase()}`
    : "";
  const retryText = retryable ? " Retry may succeed." : "";
  return `${error.message} [${code}${hresultText}] ${remediation}${retryText}`;
}

/** Identifies a graph conflict without guessing from localized text. */
export function isRevisionConflict(error: unknown): boolean {
  return error instanceof AudioRouterRpcError && error.data?.code === "revisionConflict";
}

export type UiBackendSnapshot = {
  status: StatusSnapshot;
  diagnostics: DiagnosticsSnapshot;
  session: Session;
  discovery: DiscoveryDocument | null;
};

/** The UI consumes snapshots, keeping protocol and native transport details out of React. */
export interface UiBackend {
  readonly connected: boolean;
  snapshot(sessionId?: string): Promise<UiBackendSnapshot>;
  refreshDiagnostics(): Promise<DiagnosticsSnapshot>;
  subscribe(afterSequence?: number, sessionId?: string, backendEpoch?: number, categories?: StateEventCategory[]): Promise<EventsSubscribeResult>;
  inspectRoute(destinationNode: string, sessionId?: string): Promise<RouteInspection | null>;
  planGraph(candidate: Session): Promise<GraphPlanResult>;
  commitGraph(planId: string, baseRevision: number, idempotencyKey: string, acknowledgments?: string[]): Promise<GraphCommitResult>;
  listGraphHistory(sessionId: string, cursor?: string, limit?: number): Promise<GraphHistoryPage>;
  undoGraphPlan(sessionId: string, baseRevision: number): Promise<GraphUndoPlanResult>;
  beginAudioUpload(fileName: string, sizeBytes: number): Promise<MethodResult["audioMedia.beginUpload"]>;
  uploadAudioChunk(uploadId: string, chunkIndex: number, dataBase64: string): Promise<MethodResult["audioMedia.uploadChunk"]>;
  finishAudioUpload(uploadId: string): Promise<MethodResult["audioMedia.finishUpload"]>;
  importTemporaryRecording(recordingId: string): Promise<MethodResult["audioMedia.importTemporaryRecording"]>;
  transportAudioSource(sessionId: string, nodeId: string, action: "play" | "pause" | "stop" | "status"): Promise<MethodResult["audioSources.transport"]>;
  transportTimeShift?(sessionId: string, nodeId: string, action: MethodParams["timeShift.transport"]["action"]): Promise<MethodResult["timeShift.transport"]>;
  listRecordings(sessionId?: string): Promise<RecordingRow[]>;
  listRecorders(): Promise<RecorderStatus[]>;
  listSessions(): Promise<Session[]>;
  listApplications(): Promise<ApplicationRow[]>;
  listDevices(): Promise<DeviceListItem[]>;
  prepareNativeEndpoint?(sessionId: string, captureEndpointId: string, renderEndpointId: string): Promise<import("@audiorouter/contracts").NativeEndpointPrepareResult>;
  prepareNativeOutputs?(sessionId: string, generation: number | undefined, renderEndpointIds: string[]): Promise<NativeOutputFanoutPrepareResult>;
  prepareNativeMultiInputs?(sessionId: string, generation: number | undefined, sources: import("@audiorouter/contracts").NativeMultiInputSourceBinding[]): Promise<NativeMultiInputPrepareResult>;
  /** Prepare every independent path of the saved session from the devices stored on its nodes. */
  prepareNativePaths?(sessionId: string): Promise<import("@audiorouter/contracts").NativePathsPrepareResult>;
  rebindNativeEndpoint?(sessionId: string, captureEndpointId: string, renderEndpointId: string): Promise<import("@audiorouter/contracts").NativeEndpointRebindResult>;
  detachNativeEndpoint?(sessionId: string): Promise<import("@audiorouter/contracts").NativeEndpointDetachResult>;
  detachNativeDuplex?(sessionId: string): Promise<import("@audiorouter/contracts").NativeDuplexDetachResult>;
  prepareNativeApplication?(params: Omit<import("@audiorouter/contracts").MethodParams["nativeApplications.prepare"], "creationTime100ns"> & { creationTime100ns: string | null }): Promise<import("@audiorouter/contracts").NativeApplicationPrepareResult>;
  pumpNativeEndpoint?(sessionId: string, generation: number, maxPackets?: number): Promise<import("@audiorouter/contracts").NativeEndpointPumpResult>;
  pumpNativeDuplex?(sessionId: string, generation: number, maxInputQuanta?: number, maxOutputPackets?: number): Promise<import("@audiorouter/contracts").NativeDuplexPumpResult>;
  pumpNativeRenderSource?(sessionId: string, generation: number, maxQuanta?: number): Promise<NativeRenderSourcePumpResult>;
  pumpNativeMultiInputs?(sessionId: string, generation: number, maxPackets?: number): Promise<NativeMultiInputPumpResult>;
  bindNativeMultiInputBranches?(sessionId: string, generation: number, branchNodeIds: string[]): Promise<NativeMultiInputBranchBindingResult>;
  listProcessors(): Promise<DiscoveryDocument["processors"]>;
  processorResponse(params: ProcessorResponseParams): Promise<ProcessorResponse>;
  listPresets(): Promise<DiscoveryDocument["presets"]>;
  scanPlugins(directory: string): Promise<PluginScanResult>;
  listPlugins(directory: string): Promise<PluginScanResult>;
  /** Every remembered scan (persisted by the backend); optional for older backends. */
  pluginInventory?(): Promise<MethodResult["plugins.inventory"]>;
  /** Capture a playing plugin node state and store it; returns its stateId. */
  savePluginState?(sessionId: string, nodeId: string): Promise<MethodResult["plugins.saveState"]>;
  retryPlugins(directory: string, idempotencyKey: string): Promise<PluginScanResult>;
  inspectPlugin(path: string): Promise<PluginScanEntry>;
  describePluginParameters(path: string): Promise<PluginParametersResult>;
  listVirtualDevices(): Promise<VirtualDeviceInfo[]>;
  planVirtualDevice(operation: VirtualDeviceOperation): Promise<VirtualDevicePlanResult>;
  applyVirtualDevice(planId: string, idempotencyKey: string): Promise<VirtualDeviceApplyResult>;
  provisionVirtualDevice(busId: string, instanceId: string, idempotencyKey: string): Promise<VirtualDeviceProvisionResult>;
  removeVirtualDevice(busId: string, idempotencyKey: string): Promise<VirtualDeviceRemoveResult>;
  listVirtualRoutes(): Promise<VirtualRouteListResult>;
  replaceVirtualRoutes(baseRevision: number, routes: VirtualBusRoute[], idempotencyKey: string): Promise<VirtualRouteReplaceResult>;
  previewRecording(recordingId: string): Promise<RecordingPreviewResult>;
  getRecordingRecovery(recordingId: string): Promise<RecordingRecoverySingleResult>;
  listRecordingRecovery(): Promise<RecordingRecoveryList>;
  revealRecording(recordingId: string): Promise<RecordingRevealResult>;
  setRecordingMetadata(recordingId: string, metadata: { title?: string | null; artist?: string | null; comment?: string | null; idempotencyKey?: string }): Promise<RecordingMetadataResult>;
  renameRecording(recordingId: string, newPath: string, idempotencyKey?: string): Promise<RecordingRenameResult>;
  setPrivacyMute(muted: boolean, idempotencyKey?: string): Promise<PrivacyMuteResult>;
  clearRecoverySafeMode(idempotencyKey?: string): Promise<RecoveryClearResult>;
  osTransition?(transition: OsTransition, idempotencyKey?: string): Promise<OsTransitionResult>;
  removeRecordingEntry(recordingId: string, idempotencyKey?: string): Promise<RecordingRemoveResult>;
  recycleRecording(recordingId: string, confirm: boolean, idempotencyKey?: string): Promise<RecordingRecycleResult>;
  createRecorder(params: MethodParams["recorders.create"]): Promise<RecorderCreateResult>;
  armRecorder(sessionId: string, idempotencyKey?: string, nodeId?: string): Promise<RecorderLifecycleResult>;
  startRecorder(sessionId: string, frame: number, idempotencyKey?: string, nodeId?: string): Promise<RecorderLifecycleResult>;
  pauseRecorder(sessionId: string, frame: number, idempotencyKey?: string, nodeId?: string): Promise<RecorderLifecycleResult>;
  resumeRecorder(sessionId: string, frame: number, idempotencyKey?: string, nodeId?: string): Promise<RecorderLifecycleResult>;
  splitRecorder(sessionId: string, frame: number, idempotencyKey?: string, nodeId?: string): Promise<RecorderLifecycleResult>;
  stopRecorder(sessionId: string, frame: number, idempotencyKey?: string, nodeId?: string): Promise<RecorderLifecycleResult>;
  createSession(session: Session, idempotencyKey?: string): Promise<SessionCreateResult>;
  duplicateSession(sourceSessionId: string, sessionId: string, name?: string, idempotencyKey?: string): Promise<SessionCreateResult>;
  deleteSession(sessionId: string, idempotencyKey?: string): Promise<SessionDeleteResult>;
  startSession(sessionId: string, idempotencyKey?: string, candidate?: Session): Promise<SessionStartResult>;
  stopSession(sessionId: string, idempotencyKey?: string): Promise<SessionStopResult>;
  exportSession(sessionId: string): Promise<Session>;
  planSessionImport(session: Session): Promise<SessionImportPlanResult>;
  commitSessionImport(planId: string, idempotencyKey: string): Promise<SessionImportCommitResult>;
  getStartup(): Promise<StartupStatus>;
  planStartup(enabled: boolean): Promise<StartupPlanResult>;
  applyStartup(planId: string, idempotencyKey: string): Promise<StartupApplyResult>;
  registerStartup?(enabled: boolean): Promise<string>;
  startupRegistrationStatus?(): Promise<"registered" | "unregistered">;
  listClients(): Promise<ClientRow[]>;
  authorizeClient(clientId: string, role: "observer" | "editor" | "operator", idempotencyKey: string): Promise<ClientAuthorizeResult>;
  revokeClient(clientId: string, idempotencyKey: string): Promise<ClientRevokeResult>;
}

export type UiSnapshotState = {
  snapshot: UiBackendSnapshot | null;
  stale: boolean;
  error: string | null;
};

/** Keeps the last known state visible across a failed refresh or reconnect. */
export class SnapshotCache {
  private state: UiSnapshotState = { snapshot: null, stale: true, error: null };
  private refreshGeneration = 0;

  current(): UiSnapshotState {
    return this.state;
  }

  async refresh(backend: UiBackend, sessionId?: string): Promise<UiSnapshotState> {
    const generation = ++this.refreshGeneration;
    let lastError: unknown = undefined;
    // The native shell may still be creating its per-user pipe/backend when
    // the WebView finishes loading. Retry briefly without blocking the UI or
    // hiding a previously valid snapshot.
    for (let attempt = 0; attempt < 3; attempt += 1) {
      if (generation !== this.refreshGeneration) return this.state;
      try {
        const snapshot = await backend.snapshot(sessionId);
        if (generation !== this.refreshGeneration) return this.state;
        this.state = { snapshot, stale: false, error: null };
        return this.state;
      } catch (error) {
        if (generation !== this.refreshGeneration) return this.state;
        lastError = error;
        if (attempt < 2) {
          await new Promise<void>((resolve) => setTimeout(resolve, 100 * (attempt + 1)));
        }
      }
    }
    if (generation !== this.refreshGeneration) return this.state;
    this.state = {
      ...this.state,
      stale: true,
      error: formatUiError(lastError, "Backend refresh failed"),
    };
    return this.state;
  }
}

const disconnectedStatus: StatusSnapshot = {
  build: "preview",
  audio: "unavailable",
  deviceDiscovery: "available",
  reason: "The control backend is disconnected.",
  storage: "memory",
  sessionCount: 1,
  activeSessionCount: 0,
  activeSessionIds: [],
  privacyMute: {
    muted: true,
    persistence: "memory",
    audioEffect: "process-local-when-realtime-backend-is-available",
  },
  recovery: { safeMode: false, recentCrashes: 0, persistence: "memory" },
  eventCursor: { backendEpoch: 0, latestSequence: 0 },
};

const disconnectedDiagnostics: DiagnosticsSnapshot = {
  build: "preview",
  backend: "control-plane",
  storage: "memory",
  audio: { state: "unavailable", reason: "The control backend is disconnected." },
  nativeAdapter: "implemented-not-activated",
  nativeAdapterKind: null,
  nativeSessionId: null,
  schedulerTelemetry: null,
  nodeTelemetry: [],
  applicationCaptureStates: [],
  privacyMute: { muted: true, persistence: "memory" },
  recovery: { safeMode: false, recentCrashes: 0, persistence: "memory" },
  eventLog: { latestSequence: 0, retained: 0 },
  redacted: true,
};

/** Safe startup backend: it only returns local fixture data and has no mutation methods. */
export function createDisconnectedBackend(session: Session = demoSession): UiBackend {
  return {
    connected: false,
    async snapshot() {
      return { status: disconnectedStatus, diagnostics: disconnectedDiagnostics, session, discovery: null };
    },
    async refreshDiagnostics() {
      return disconnectedDiagnostics;
    },
    async subscribe() {
      return { backendEpoch: 0, events: [], nextSequence: 0 };
    },
    async inspectRoute() {
      return null;
    },
    async planGraph() {
      throw new Error("The backend is disconnected; graph changes are unavailable.");
    },
    async commitGraph() {
      throw new Error("The backend is disconnected; graph changes are unavailable.");
    },
    async listGraphHistory() {
      return { items: [], nextCursor: null };
    },
    async undoGraphPlan() {
      throw new Error("The backend is disconnected; undo is unavailable.");
    },
    async beginAudioUpload() { throw new Error("The backend is disconnected; audio import is unavailable."); },
    async uploadAudioChunk() { throw new Error("The backend is disconnected; audio import is unavailable."); },
    async finishAudioUpload() { throw new Error("The backend is disconnected; audio import is unavailable."); },
    async importTemporaryRecording() { throw new Error("The backend is disconnected; temporary audio import is unavailable."); },
    async transportAudioSource() { throw new Error("The backend is disconnected; audio playback is unavailable."); },
    async listRecordings() {
      return [];
    },
    async listRecorders() {
      return [];
    },
    async listSessions() {
      return demoSessions;
    },
    async listApplications() {
      return [];
    },
    async listDevices() {
      return [];
    },
    async listProcessors() {
      throw new Error("The backend is disconnected; processor catalog is unavailable.");
    },
    async processorResponse() {
      throw new Error("The backend is disconnected; processor response is unavailable.");
    },
    async listPresets() {
      throw new Error("The backend is disconnected; preset catalog is unavailable.");
    },
    async scanPlugins() {
      throw new Error("The backend is disconnected; plugin scanning is unavailable.");
    },
    async listPlugins() {
      throw new Error("The backend is disconnected; plugin inventory is unavailable.");
    },
    async retryPlugins() {
      throw new Error("The backend is disconnected; plugin scan retry is unavailable.");
    },
    async inspectPlugin() {
      throw new Error("The backend is disconnected; plugin inspection is unavailable.");
    },
    async describePluginParameters() {
      throw new Error("The backend is disconnected; plugin parameter discovery is unavailable.");
    },
    async listVirtualDevices() {
      return [];
    },
    async planVirtualDevice(operation) {
      return {
        planId: "unavailable",
        expiresInMs: 1,
        operation,
        availability: { status: "unavailable", reason: "demo backend has no managed virtual driver" },
        requiredScopes: ["deviceAdministration"],
        warnings: ["demo backend cannot provision virtual endpoints"],
      };
    },
    async applyVirtualDevice() {
      throw new Error("demo backend has no managed virtual driver");
    },
    async provisionVirtualDevice() {
      throw new Error("demo backend has no managed virtual driver");
    },
    async removeVirtualDevice() {
      throw new Error("demo backend has no managed virtual driver");
    },
    async listVirtualRoutes() {
      return { revision: 0, routes: [] };
    },
    async replaceVirtualRoutes() {
      throw new Error("demo backend has no managed virtual route control");
    },
    async previewRecording() {
      throw new Error("The backend is disconnected; recording preview is unavailable.");
    },
    async getRecordingRecovery() {
      throw new Error("The backend is disconnected; recording recovery is unavailable.");
    },
    async listRecordingRecovery() {
      return { items: [], nextCursor: null };
    },
    async revealRecording() {
      throw new Error("The backend is disconnected; recording reveal is unavailable.");
    },
    async setRecordingMetadata() {
      throw new Error("The backend is disconnected; recording metadata editing is unavailable.");
    },
    async renameRecording() {
      throw new Error("The backend is disconnected; recording rename is unavailable.");
    },
    async setPrivacyMute() {
      throw new Error("The backend is disconnected; privacy mute is unavailable.");
    },
    async clearRecoverySafeMode() {
      throw new Error("The backend is disconnected; recovery safe-mode clearing is unavailable.");
    },
    async osTransition() {
      throw new Error("The backend is disconnected; OS transition handling is unavailable.");
    },
    async removeRecordingEntry() {
      throw new Error("The backend is disconnected; recording removal is unavailable.");
    },
    async recycleRecording() {
      throw new Error("The backend is disconnected; recording recycle is unavailable.");
    },
    async createRecorder() {
      throw new Error("The backend is disconnected; recorder creation is unavailable.");
    },
    async armRecorder() {
      throw new Error("The backend is disconnected; recorder control is unavailable.");
    },
    async startRecorder() {
      throw new Error("The backend is disconnected; recorder control is unavailable.");
    },
    async pauseRecorder() {
      throw new Error("The backend is disconnected; recorder control is unavailable.");
    },
    async resumeRecorder() {
      throw new Error("The backend is disconnected; recorder control is unavailable.");
    },
    async splitRecorder() {
      throw new Error("The backend is disconnected; recorder control is unavailable.");
    },
    async stopRecorder() {
      throw new Error("The backend is disconnected; recorder control is unavailable.");
    },
    async createSession() {
      throw new Error("The backend is disconnected; session creation is unavailable.");
    },
    async duplicateSession() {
      throw new Error("The backend is disconnected; session duplication is unavailable.");
    },
    async deleteSession() {
      throw new Error("The backend is disconnected; session deletion is unavailable.");
    },
    async startSession() {
      throw new Error("The backend is disconnected; session start is unavailable.");
    },
    async stopSession() {
      throw new Error("The backend is disconnected; session stop is unavailable.");
    },
    async exportSession() {
      throw new Error("The backend is disconnected; session export is unavailable.");
    },
    async planSessionImport() {
      throw new Error("The backend is disconnected; session import is unavailable.");
    },
    async commitSessionImport() {
      throw new Error("The backend is disconnected; session import is unavailable.");
    },
    async getStartup() {
      return { enabled: false, registration: "unavailable", reason: "The control backend is disconnected." };
    },
    async planStartup() {
      throw new Error("The backend is disconnected; startup planning is unavailable.");
    },
    async applyStartup() {
      throw new Error("The backend is disconnected; startup apply is unavailable.");
    },
    async listClients() {
      return [];
    },
    async authorizeClient() {
      throw new Error("The backend is disconnected; client authorization is unavailable.");
    },
    async revokeClient() {
      throw new Error("The backend is disconnected; client revocation is unavailable.");
    },
  };
}

const MAX_UI_PAGES = 10_000;

type PagedRequest = (cursor: string | null) => Promise<unknown>;

/** Collects bounded API pages while rejecting a broken/non-advancing cursor. */
async function collectPagedRows<T>(request: PagedRequest): Promise<T[]> {
  const rows: T[] = [];
  let cursor: string | null = null;
  const seenCursors = new Set<string>();

  for (let pageNumber = 0; pageNumber < MAX_UI_PAGES; pageNumber += 1) {
    const result = await request(cursor);
    if (Array.isArray(result)) return rows.concat(result as T[]);
    if (!result || typeof result !== "object") {
      throw new Error("The backend returned an invalid paged inventory response.");
    }
    const page = result as { items?: unknown; nextCursor?: unknown };
    if (!Array.isArray(page.items)) {
      throw new Error("The backend returned an invalid paged inventory response.");
    }
    rows.push(...(page.items as T[]));
    const nextCursor = page.nextCursor;
    if (nextCursor === null || nextCursor === undefined) return rows;
    if (typeof nextCursor !== "string" || nextCursor.length === 0 || seenCursors.has(nextCursor)) {
      throw new Error("The backend returned a non-advancing inventory cursor.");
    }
    seenCursors.add(nextCursor);
    cursor = nextCursor;
  }
  throw new Error("The backend returned too many inventory pages.");
}

/** Adapter for a future framed/local transport implementation. */
export function createLiveBackend(client: AudioRouterClient, sessionId: string, registerStartup?: (enabled: boolean) => Promise<string>, startupRegistrationStatus?: () => Promise<"registered" | "unregistered">): UiBackend {
  return {
    connected: true,
    async snapshot(selectedSessionId = sessionId) {
      const [status, diagnostics, discovery, session] = await Promise.all([
        client.request("status.get", undefined),
        client.request("system.diagnostics", undefined),
        client.request("system.describe", undefined),
        client.request("sessions.get", { sessionId: selectedSessionId }),
      ]);
      return { status, diagnostics, discovery, session };
    },
    async refreshDiagnostics() {
      return client.request("system.diagnostics", undefined);
    },
    async subscribe(afterSequence = 0, sessionId, backendEpoch, categories) {
      return client.request("events.subscribe", {
        afterSequence,
        limit: 500,
        ...(backendEpoch === undefined ? {} : { backendEpoch }),
        ...(sessionId === undefined ? {} : { sessionId }),
        ...(categories === undefined ? {} : { categories }),
      });
    },
    async inspectRoute(destinationNode, selectedSessionId = sessionId) {
      return client.request("routes.inspect", { sessionId: selectedSessionId, destinationNode });
    },
    async planGraph(candidate) {
      return client.request("graph.plan", {
        sessionId: candidate.id,
        baseRevision: candidate.revision,
        candidate,
      });
    },
    async commitGraph(planId, baseRevision, idempotencyKey, acknowledgments) {
      return client.request("graph.commit", {
        planId,
        baseRevision,
        idempotencyKey,
        ...(acknowledgments === undefined ? {} : { acknowledgments }),
      });
    },
    async listGraphHistory(historySessionId, cursor, limit) {
      return client.request("graph.history", {
        sessionId: historySessionId,
        ...(cursor === undefined ? {} : { cursor }),
        ...(limit === undefined ? {} : { limit }),
      });
    },
    async undoGraphPlan(undoSessionId, baseRevision) {
      return client.request("graph.undoPlan", { sessionId: undoSessionId, baseRevision });
    },
    async beginAudioUpload(fileName, sizeBytes) {
      return client.request("audioMedia.beginUpload", { fileName, sizeBytes });
    },
    async uploadAudioChunk(uploadId, chunkIndex, dataBase64) {
      return client.request("audioMedia.uploadChunk", { uploadId, chunkIndex, dataBase64 });
    },
    async finishAudioUpload(uploadId) {
      return client.request("audioMedia.finishUpload", { uploadId });
    },
    async importTemporaryRecording(recordingId) {
      return client.request("audioMedia.importTemporaryRecording", { recordingId });
    },
    async transportTimeShift(currentSessionId, nodeId, action) {
      return client.request("timeShift.transport", { sessionId: currentSessionId, nodeId, action });
    },
    async transportAudioSource(currentSessionId, nodeId, action) {
      return client.request("audioSources.transport", { sessionId: currentSessionId, nodeId, action });
    },
    async listRecordings(recordingSessionId = sessionId) {
      return collectPagedRows(
        (cursor) => client.request("recordings.list", cursor === null
          ? { sessionId: recordingSessionId, limit: 500 }
          : { sessionId: recordingSessionId, limit: 500, cursor }),
      );
    },
    async listRecorders() {
      return client.request("recorders.list", undefined);
    },
    async listSessions() {
      return collectPagedRows((cursor) => client.request("sessions.list", cursor === null
        ? { limit: 500 }
        : { limit: 500, cursor }));
    },
    async listApplications() {
      return client.request("applications.list", undefined);
    },
    async listDevices() {
      return collectPagedRows((cursor) => client.request("devices.list", cursor === null
        ? { limit: 500 }
        : { limit: 500, cursor }));
    },
    async prepareNativeEndpoint(currentSessionId, captureEndpointId, renderEndpointId) {
      return client.request("nativeEndpoints.prepare", {
        sessionId: currentSessionId,
        captureEndpointId,
        renderEndpointId,
      });
    },
    async prepareNativeOutputs(currentSessionId, generation, renderEndpointIds) {
      return client.request("nativeOutputs.prepare", {
        sessionId: currentSessionId,
        generation,
        renderEndpointIds,
      });
    },
    async prepareNativeMultiInputs(currentSessionId, generation, sources) {
      return client.request("nativeMultiInputs.prepare", {
        sessionId: currentSessionId,
        generation,
        sources,
      });
    },
    async prepareNativePaths(currentSessionId) {
      return client.request("nativePaths.prepare", { sessionId: currentSessionId });
    },
    async rebindNativeEndpoint(currentSessionId, captureEndpointId, renderEndpointId) {
      return client.request("nativeEndpoints.rebind", {
        sessionId: currentSessionId,
        captureEndpointId,
        renderEndpointId,
      });
    },
    async detachNativeEndpoint(currentSessionId) {
      return client.request("nativeEndpoints.detach", { sessionId: currentSessionId });
    },
    async detachNativeDuplex(currentSessionId) {
      return client.request("nativeDuplex.detach", { sessionId: currentSessionId });
    },
    async prepareNativeApplication(params) {
      return client.request("nativeApplications.prepare", params as import("@audiorouter/contracts").MethodParams["nativeApplications.prepare"]);
    },
    async pumpNativeEndpoint(currentSessionId, generation, maxPackets = 64) {
      return client.request("nativeEndpoints.pump", { sessionId: currentSessionId, generation, maxPackets });
    },
    async pumpNativeDuplex(currentSessionId, generation, maxInputQuanta = 64, maxOutputPackets = 64) {
      return client.request("nativeDuplex.pump", {
        sessionId: currentSessionId,
        generation,
        maxInputQuanta,
        maxOutputPackets,
      });
    },
    async pumpNativeRenderSource(currentSessionId, generation, maxQuanta = 64) {
      return client.request("nativeRenderSources.pump", { sessionId: currentSessionId, generation, maxQuanta });
    },
    async pumpNativeMultiInputs(currentSessionId, generation, maxPackets = 64) {
      return client.request("nativeMultiInputs.pump", { sessionId: currentSessionId, generation, maxPackets });
    },
    async bindNativeMultiInputBranches(currentSessionId, generation, branchNodeIds) {
      return client.request("nativeMultiInputs.bindBranches", {
        sessionId: currentSessionId,
        generation,
        branchNodeIds,
      });
    },
    async listProcessors() {
      return client.request("processors.list", undefined);
    },
    async processorResponse(params) {
      return client.request("processors.response", params);
    },
    async listPresets() {
      return client.request("presets.list", undefined);
    },
    async scanPlugins(directory) {
      return client.request("plugins.scan", { directory });
    },
    async savePluginState(currentSessionId, nodeId) {
      return client.request("plugins.saveState", { sessionId: currentSessionId, nodeId });
    },
    async pluginInventory() {
      return client.request("plugins.inventory", {});
    },
    async listPlugins(directory) {
      return client.request("plugins.list", { directory });
    },
    async retryPlugins(directory, idempotencyKey) {
      return client.request("plugins.retry", { directory, idempotencyKey });
    },
    async inspectPlugin(path) {
      return client.request("plugins.inspect", { path });
    },
    async describePluginParameters(path) {
      return client.request("plugins.parameters", { path });
    },
    async listVirtualDevices() {
      return collectPagedRows((cursor) => client.request("virtualDevices.list", cursor === null
        ? { limit: 500 }
        : { limit: 500, cursor }));
    },
    async planVirtualDevice(operation) {
      return client.request("virtualDevices.plan", { operation });
    },
    async applyVirtualDevice(planId, idempotencyKey) {
      return client.request("virtualDevices.apply", { planId, idempotencyKey });
    },
    async provisionVirtualDevice(busId, instanceId, idempotencyKey) {
      return client.request("virtualDevices.provision", { busId, instanceId, idempotencyKey });
    },
    async removeVirtualDevice(busId, idempotencyKey) {
      return client.request("virtualDevices.remove", { busId, idempotencyKey });
    },
    async listVirtualRoutes() {
      return client.request("virtualRoutes.list", undefined);
    },
    async replaceVirtualRoutes(baseRevision, routes, idempotencyKey) {
      return client.request("virtualRoutes.replace", { baseRevision, routes, idempotencyKey });
    },
    async previewRecording(recordingId) {
      return client.request("recordings.preview", { recordingId });
    },
    async getRecordingRecovery(recordingId) {
      return client.request("recordings.recovery", { recordingId }) as Promise<RecordingRecoverySingleResult>;
    },
    async listRecordingRecovery() {
      return client.request("recordings.recovery", { limit: 500 }) as Promise<RecordingRecoveryList>;
    },
    async revealRecording(recordingId) {
      return client.request("recordings.reveal", { recordingId });
    },
    async setRecordingMetadata(recordingId, metadata) {
      for (const value of [metadata.title, metadata.artist, metadata.comment]) {
        if (value !== undefined && value !== null && [...value].length > 256) {
          throw new Error("recording metadata fields are limited to 256 characters");
        }
      }
      return client.request("recordings.setMetadata", { recordingId, ...metadata });
    },
    async renameRecording(recordingId, newPath, idempotencyKey) {
      return client.request("recordings.rename", { recordingId, newPath, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async setPrivacyMute(muted, idempotencyKey) {
      return client.request("safety.setPrivacyMute", { muted, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async clearRecoverySafeMode(idempotencyKey) {
      return client.request("recovery.clearSafeMode", idempotencyKey === undefined ? undefined : { idempotencyKey });
    },
    async osTransition(transition, idempotencyKey) {
      return client.request("system.osTransition", {
        transition,
        idempotencyKey: idempotencyKey ?? `ui-os-transition-${Date.now().toString(36)}`,
      });
    },
    async removeRecordingEntry(recordingId, idempotencyKey) {
      return client.request("recordings.removeEntry", { recordingId, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async recycleRecording(recordingId, confirm, idempotencyKey) {
      return client.request("recordings.recycle", { recordingId, confirm, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async createRecorder(params) {
      return client.request("recorders.create", params);
    },
    async armRecorder(recorderSessionId, idempotencyKey, nodeId) {
      return client.request("recorders.arm", { sessionId: recorderSessionId, ...(nodeId === undefined ? {} : { nodeId }), ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async startRecorder(recorderSessionId, frame, idempotencyKey, nodeId) {
      return client.request("recorders.start", { sessionId: recorderSessionId, frame, ...(nodeId === undefined ? {} : { nodeId }), ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async pauseRecorder(recorderSessionId, frame, idempotencyKey, nodeId) {
      return client.request("recorders.pause", { sessionId: recorderSessionId, frame, ...(nodeId === undefined ? {} : { nodeId }), ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async resumeRecorder(recorderSessionId, frame, idempotencyKey, nodeId) {
      return client.request("recorders.resume", { sessionId: recorderSessionId, frame, ...(nodeId === undefined ? {} : { nodeId }), ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async splitRecorder(recorderSessionId, frame, idempotencyKey, nodeId) {
      return client.request("recorders.split", { sessionId: recorderSessionId, frame, ...(nodeId === undefined ? {} : { nodeId }), ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async stopRecorder(recorderSessionId, frame, idempotencyKey, nodeId) {
      return client.request("recorders.stop", { sessionId: recorderSessionId, frame, ...(nodeId === undefined ? {} : { nodeId }), ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async createSession(session, idempotencyKey) {
      return client.request("sessions.create", { session, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async duplicateSession(sourceSessionId, sessionId, name, idempotencyKey) {
      return client.request("sessions.duplicate", { sourceSessionId, sessionId, ...(name === undefined ? {} : { name }), ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async deleteSession(sessionId, idempotencyKey) {
      return client.request("sessions.delete", { sessionId, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async startSession(startSessionId, idempotencyKey, candidate) {
      return client.request("session.start", { sessionId: startSessionId, ...(idempotencyKey === undefined ? {} : { idempotencyKey }), ...(candidate === undefined ? {} : { candidate }) });
    },
    async stopSession(stopSessionId, idempotencyKey) {
      return client.request("session.stop", { sessionId: stopSessionId, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async exportSession(exportSessionId) {
      return client.request("sessions.export", { sessionId: exportSessionId });
    },
    async planSessionImport(session) {
      return client.request("sessions.importPlan", { session });
    },
    async commitSessionImport(planId, idempotencyKey) {
      return client.request("sessions.importCommit", { planId, idempotencyKey });
    },
    async getStartup() {
      return client.request("startup.get", undefined);
    },
    async planStartup(enabled) {
      return client.request("startup.plan", { enabled });
    },
    async applyStartup(planId, idempotencyKey) {
      return client.request("startup.apply", { planId, idempotencyKey });
    },
    async listClients() {
      return client.request("clients.list", undefined);
    },
    async authorizeClient(clientId, role, idempotencyKey) {
      return client.request("clients.authorize", { clientId, role, idempotencyKey });
    },
    async revokeClient(clientId, idempotencyKey) {
      return client.request("clients.revoke", { clientId, idempotencyKey });
    },
    ...(registerStartup === undefined ? {} : { registerStartup }),
    ...(startupRegistrationStatus === undefined ? {} : { startupRegistrationStatus }),
  };
}

/** Build the live UI backend directly from the host-provided framed transport. */
export function createLiveBackendFromTransport(transport: RpcTransport, sessionId: string, registerStartup?: (enabled: boolean) => Promise<string>, startupRegistrationStatus?: () => Promise<"registered" | "unregistered">): UiBackend {
  return createLiveBackend(createAudioRouterClient(transport), sessionId, registerStartup, startupRegistrationStatus);
}
