import type {
  AudioRouterClient,
  DiscoveryDocument,
  EventsSubscribeResult,
  Session,
  StatusSnapshot,
} from "@audiorouter/contracts";
import { demoSession } from "./fixtures";

export type UiBackendSnapshot = {
  status: StatusSnapshot;
  session: Session;
  discovery: DiscoveryDocument | null;
};

/** The UI consumes snapshots, keeping protocol and native transport details out of React. */
export interface UiBackend {
  readonly connected: boolean;
  snapshot(): Promise<UiBackendSnapshot>;
  subscribe(afterSequence?: number, sessionId?: string): Promise<EventsSubscribeResult>;
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
    async subscribe(afterSequence = 0, sessionId) {
      return client.request("events.subscribe", {
        afterSequence,
        limit: 500,
        ...(sessionId === undefined ? {} : { sessionId }),
      });
    },
  };
}
