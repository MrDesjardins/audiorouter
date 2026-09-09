import { AudioRouterRpcError, createAudioRouterClient } from "@audiorouter/contracts";
import type {
  AudioRouterClient,
  ApplicationInfo,
  DeviceInfo,
  VirtualDeviceInfo,
  VirtualDeviceApplyResult,
  VirtualDeviceOperation,
  VirtualDevicePlanResult,
  DiscoveryDocument,
  EventsSubscribeResult,
  GraphCommitResult,
  GraphPlanResult,
  PrivacyMuteResult,
  PluginScanEntry,
  PluginScanResult,
  RecoveryClearResult,
  RecordingMetadataResult,
  RecordingRenameResult,
  RecordingPreviewResult,
  RecordingRecoveryResult,
  RecordingRevealResult,
  RecordingRecycleResult,
  RecordingRemoveResult,
  RecordingRow,
  RecorderLifecycleResult,
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
  StatusSnapshot,
  RpcTransport,
} from "@audiorouter/contracts";
import type { MethodParams, MethodResult } from "@audiorouter/contracts";
import { demoSession, demoSessions } from "./fixtures";

export type ApplicationRow = ApplicationInfo;
export type ProcessorResponseParams = NonNullable<MethodParams["processors.response"]>;
export type ProcessorResponse = MethodResult["processors.response"];

/** Formats structured backend failures without losing actionable audio guidance. */
export function formatUiError(error: unknown, fallback: string): string {
  if (!(error instanceof Error)) return fallback;
  if (!(error instanceof AudioRouterRpcError) || !error.data) return error.message;
  const { code, hresult, remediation, retryable } = error.data;
  const hresultText = typeof hresult === "number"
    ? `, HRESULT 0x${(hresult >>> 0).toString(16).padStart(8, "0").toUpperCase()}`
    : "";
  const retryText = retryable ? " Retry may succeed." : "";
  return `${error.message} [${code}${hresultText}] ${remediation}${retryText}`;
}

export type UiBackendSnapshot = {
  status: StatusSnapshot;
  session: Session;
  discovery: DiscoveryDocument | null;
};

/** The UI consumes snapshots, keeping protocol and native transport details out of React. */
export interface UiBackend {
  readonly connected: boolean;
  snapshot(): Promise<UiBackendSnapshot>;
  subscribe(afterSequence?: number, sessionId?: string, backendEpoch?: number, categories?: string[]): Promise<EventsSubscribeResult>;
  inspectRoute(destinationNode: string): Promise<RouteInspection | null>;
  planGraph(candidate: Session): Promise<GraphPlanResult>;
  commitGraph(planId: string, baseRevision: number, idempotencyKey: string, acknowledgments?: string[]): Promise<GraphCommitResult>;
  listRecordings(sessionId?: string): Promise<RecordingRow[]>;
  listSessions(): Promise<Session[]>;
  listApplications(): Promise<ApplicationRow[]>;
  listDevices(): Promise<DeviceInfo[]>;
  listProcessors(): Promise<DiscoveryDocument["processors"]>;
  processorResponse(params: ProcessorResponseParams): Promise<ProcessorResponse>;
  listPresets(): Promise<DiscoveryDocument["presets"]>;
  scanPlugins(directory: string): Promise<PluginScanResult>;
  listPlugins(directory: string): Promise<PluginScanResult>;
  retryPlugins(directory: string, idempotencyKey: string): Promise<PluginScanResult>;
  inspectPlugin(path: string): Promise<PluginScanEntry>;
  listVirtualDevices(): Promise<VirtualDeviceInfo[]>;
  planVirtualDevice(operation: VirtualDeviceOperation): Promise<VirtualDevicePlanResult>;
  applyVirtualDevice(planId: string, idempotencyKey: string): Promise<VirtualDeviceApplyResult>;
  previewRecording(recordingId: string): Promise<RecordingPreviewResult>;
  getRecordingRecovery(recordingId: string): Promise<RecordingRecoveryResult>;
  revealRecording(recordingId: string): Promise<RecordingRevealResult>;
  setRecordingMetadata(recordingId: string, metadata: { title?: string | null; artist?: string | null; comment?: string | null; idempotencyKey?: string }): Promise<RecordingMetadataResult>;
  renameRecording(recordingId: string, newPath: string, idempotencyKey?: string): Promise<RecordingRenameResult>;
  setPrivacyMute(muted: boolean, idempotencyKey?: string): Promise<PrivacyMuteResult>;
  clearRecoverySafeMode(idempotencyKey?: string): Promise<RecoveryClearResult>;
  removeRecordingEntry(recordingId: string, idempotencyKey?: string): Promise<RecordingRemoveResult>;
  recycleRecording(recordingId: string, confirm: boolean, idempotencyKey?: string): Promise<RecordingRecycleResult>;
  armRecorder(sessionId: string, idempotencyKey?: string): Promise<RecorderLifecycleResult>;
  startRecorder(sessionId: string, frame: number, idempotencyKey?: string): Promise<RecorderLifecycleResult>;
  pauseRecorder(sessionId: string, frame: number, idempotencyKey?: string): Promise<RecorderLifecycleResult>;
  resumeRecorder(sessionId: string, frame: number, idempotencyKey?: string): Promise<RecorderLifecycleResult>;
  splitRecorder(sessionId: string, frame: number, idempotencyKey?: string): Promise<RecorderLifecycleResult>;
  stopRecorder(sessionId: string, frame: number, idempotencyKey?: string): Promise<RecorderLifecycleResult>;
  createSession(session: Session, idempotencyKey?: string): Promise<SessionCreateResult>;
  duplicateSession(sourceSessionId: string, sessionId: string, name?: string, idempotencyKey?: string): Promise<SessionCreateResult>;
  deleteSession(sessionId: string, idempotencyKey?: string): Promise<SessionDeleteResult>;
  startSession(sessionId: string, idempotencyKey?: string): Promise<SessionStartResult>;
  stopSession(sessionId: string, idempotencyKey?: string): Promise<SessionStopResult>;
  exportSession(sessionId: string): Promise<Session>;
  planSessionImport(session: Session): Promise<SessionImportPlanResult>;
  commitSessionImport(planId: string, idempotencyKey: string): Promise<SessionImportCommitResult>;
  getStartup(): Promise<StartupStatus>;
  planStartup(enabled: boolean): Promise<StartupPlanResult>;
  applyStartup(planId: string, idempotencyKey: string): Promise<StartupApplyResult>;
}

export type UiSnapshotState = {
  snapshot: UiBackendSnapshot | null;
  stale: boolean;
  error: string | null;
};

/** Keeps the last known state visible across a failed refresh or reconnect. */
export class SnapshotCache {
  private state: UiSnapshotState = { snapshot: null, stale: true, error: null };

  current(): UiSnapshotState {
    return this.state;
  }

  async refresh(backend: UiBackend): Promise<UiSnapshotState> {
    try {
      const snapshot = await backend.snapshot();
      this.state = { snapshot, stale: false, error: null };
    } catch (error) {
      this.state = {
        ...this.state,
        stale: true,
        error: formatUiError(error, "Backend refresh failed"),
      };
    }
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

/** Safe startup backend: it only returns local fixture data and has no mutation methods. */
export function createDisconnectedBackend(session: Session = demoSession): UiBackend {
  return {
    connected: false,
    async snapshot() {
      return { status: disconnectedStatus, session, discovery: null };
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
    async listRecordings() {
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
    async previewRecording() {
      throw new Error("The backend is disconnected; recording preview is unavailable.");
    },
    async getRecordingRecovery() {
      throw new Error("The backend is disconnected; recording recovery is unavailable.");
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
    async removeRecordingEntry() {
      throw new Error("The backend is disconnected; recording removal is unavailable.");
    },
    async recycleRecording() {
      throw new Error("The backend is disconnected; recording recycle is unavailable.");
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
export function createLiveBackend(client: AudioRouterClient, sessionId: string): UiBackend {
  return {
    connected: true,
    async snapshot() {
      const [status, discovery, session] = await Promise.all([
        client.request("status.get", undefined),
        client.request("system.describe", undefined),
        client.request("sessions.get", { sessionId }),
      ]);
      return { status, discovery, session };
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
    async inspectRoute(destinationNode) {
      return client.request("routes.inspect", { sessionId, destinationNode });
    },
    async planGraph(candidate) {
      return client.request("graph.plan", {
        sessionId,
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
    async listRecordings(recordingSessionId = sessionId) {
      return collectPagedRows(
        (cursor) => client.request("recordings.list", cursor === null
          ? { sessionId: recordingSessionId, limit: 500 }
          : { sessionId: recordingSessionId, limit: 500, cursor }),
      );
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
    async listPlugins(directory) {
      return client.request("plugins.list", { directory });
    },
    async retryPlugins(directory, idempotencyKey) {
      return client.request("plugins.retry", { directory, idempotencyKey });
    },
    async inspectPlugin(path) {
      return client.request("plugins.inspect", { path });
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
    async previewRecording(recordingId) {
      return client.request("recordings.preview", { recordingId });
    },
    async getRecordingRecovery(recordingId) {
      return client.request("recordings.recovery", { recordingId });
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
    async removeRecordingEntry(recordingId, idempotencyKey) {
      return client.request("recordings.removeEntry", { recordingId, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async recycleRecording(recordingId, confirm, idempotencyKey) {
      return client.request("recordings.recycle", { recordingId, confirm, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async armRecorder(recorderSessionId, idempotencyKey) {
      return client.request("recorders.arm", { sessionId: recorderSessionId, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async startRecorder(recorderSessionId, frame, idempotencyKey) {
      return client.request("recorders.start", { sessionId: recorderSessionId, frame, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async pauseRecorder(recorderSessionId, frame, idempotencyKey) {
      return client.request("recorders.pause", { sessionId: recorderSessionId, frame, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async resumeRecorder(recorderSessionId, frame, idempotencyKey) {
      return client.request("recorders.resume", { sessionId: recorderSessionId, frame, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async splitRecorder(recorderSessionId, frame, idempotencyKey) {
      return client.request("recorders.split", { sessionId: recorderSessionId, frame, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
    },
    async stopRecorder(recorderSessionId, frame, idempotencyKey) {
      return client.request("recorders.stop", { sessionId: recorderSessionId, frame, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
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
    async startSession(startSessionId, idempotencyKey) {
      return client.request("session.start", { sessionId: startSessionId, ...(idempotencyKey === undefined ? {} : { idempotencyKey }) });
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
  };
}

/** Build the live UI backend directly from the host-provided framed transport. */
export function createLiveBackendFromTransport(transport: RpcTransport, sessionId: string): UiBackend {
  return createLiveBackend(createAudioRouterClient(transport), sessionId);
}
