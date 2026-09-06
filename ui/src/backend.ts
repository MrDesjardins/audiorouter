import { createAudioRouterClient } from "@audiorouter/contracts";
import type {
  AudioRouterClient,
  ApplicationInfo,
  DeviceInfo,
  DiscoveryDocument,
  EventsSubscribeResult,
  GraphCommitResult,
  GraphPlanResult,
  RecordingRow,
  RouteInspection,
  Session,
  StatusSnapshot,
  RpcTransport,
} from "@audiorouter/contracts";
import { demoSession } from "./fixtures";

export type ApplicationRow = ApplicationInfo;

export type UiBackendSnapshot = {
  status: StatusSnapshot;
  session: Session;
  discovery: DiscoveryDocument | null;
};

/** The UI consumes snapshots, keeping protocol and native transport details out of React. */
export interface UiBackend {
  readonly connected: boolean;
  snapshot(): Promise<UiBackendSnapshot>;
  subscribe(afterSequence?: number, sessionId?: string, backendEpoch?: number): Promise<EventsSubscribeResult>;
  inspectRoute(destinationNode: string): Promise<RouteInspection | null>;
  planGraph(candidate: Session): Promise<GraphPlanResult>;
  commitGraph(planId: string, baseRevision: number, idempotencyKey: string, acknowledgments?: string[]): Promise<GraphCommitResult>;
  listRecordings(sessionId?: string): Promise<RecordingRow[]>;
  listApplications(): Promise<ApplicationRow[]>;
  listDevices(): Promise<DeviceInfo[]>;
  previewRecording(recordingId: string): Promise<Record<string, unknown>>;
  getRecordingRecovery(recordingId: string): Promise<Record<string, unknown>>;
  setRecordingMetadata(recordingId: string, metadata: { title?: string | null; artist?: string | null; comment?: string | null }): Promise<Record<string, unknown>>;
  setPrivacyMute(muted: boolean): Promise<Record<string, unknown>>;
  clearRecoverySafeMode(): Promise<Record<string, unknown>>;
  removeRecordingEntry(recordingId: string): Promise<Record<string, unknown>>;
  createSession(session: Session): Promise<{ session: Session; state: string }>;
  duplicateSession(sourceSessionId: string, sessionId: string, name?: string): Promise<{ session: Session; state: string }>;
  deleteSession(sessionId: string): Promise<Record<string, unknown>>;
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
        error: error instanceof Error ? error.message : "Backend refresh failed",
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
    async listApplications() {
      return [];
    },
    async listDevices() {
      return [];
    },
    async previewRecording() {
      throw new Error("The backend is disconnected; recording preview is unavailable.");
    },
    async getRecordingRecovery() {
      throw new Error("The backend is disconnected; recording recovery is unavailable.");
    },
    async setRecordingMetadata() {
      throw new Error("The backend is disconnected; recording metadata editing is unavailable.");
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
    async createSession() {
      throw new Error("The backend is disconnected; session creation is unavailable.");
    },
    async duplicateSession() {
      throw new Error("The backend is disconnected; session duplication is unavailable.");
    },
    async deleteSession() {
      throw new Error("The backend is disconnected; session deletion is unavailable.");
    },
  };
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
    async subscribe(afterSequence = 0, sessionId, backendEpoch) {
      return client.request("events.subscribe", {
        afterSequence,
        limit: 500,
        ...(backendEpoch === undefined ? {} : { backendEpoch }),
        ...(sessionId === undefined ? {} : { sessionId }),
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
      const result = await client.request("recordings.list", { sessionId: recordingSessionId });
      return Array.isArray(result) ? result : result.items;
    },
    async listApplications() {
      return client.request("applications.list", undefined);
    },
    async listDevices() {
      const result = await client.request("devices.list", { limit: 500 });
      return Array.isArray(result) ? result : result.items;
    },
    async previewRecording(recordingId) {
      return client.request("recordings.preview", { recordingId });
    },
    async getRecordingRecovery(recordingId) {
      return client.request("recordings.recovery", { recordingId });
    },
    async setRecordingMetadata(recordingId, metadata) {
      return client.request("recordings.setMetadata", { recordingId, ...metadata });
    },
    async setPrivacyMute(muted) {
      return client.request("safety.setPrivacyMute", { muted });
    },
    async clearRecoverySafeMode() {
      return client.request("recovery.clearSafeMode", undefined);
    },
    async removeRecordingEntry(recordingId) {
      return client.request("recordings.removeEntry", { recordingId });
    },
    async createSession(session) {
      return client.request("sessions.create", { session });
    },
    async duplicateSession(sourceSessionId, sessionId, name) {
      return client.request("sessions.duplicate", { sourceSessionId, sessionId, ...(name === undefined ? {} : { name }) });
    },
    async deleteSession(sessionId) {
      return client.request("sessions.delete", { sessionId });
    },
  };
}

/** Build the live UI backend directly from the host-provided framed transport. */
export function createLiveBackendFromTransport(transport: RpcTransport, sessionId: string): UiBackend {
  return createLiveBackend(createAudioRouterClient(transport), sessionId);
}
