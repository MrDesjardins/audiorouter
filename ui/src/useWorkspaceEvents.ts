// Backend state-event subscription of the workspace (moved from App.tsx).
import { useEffect, useEffectEvent, type Dispatch, type RefObject, type SetStateAction } from "react";
import type { RecordingRow, Session, StateEventCategory } from "@audiorouter/contracts";
import {
  formatUiError,
  type RecorderStatus,
  type SnapshotCache,
  type UiBackend,
  type UiSnapshotState,
} from "./backend";
import { markErrorMessage } from "./actionMessage";

/** State categories that can invalidate the workspace snapshot. Meter events
 * are intentionally excluded; diagnostics use the bounded snapshot path. */
export const WORKSPACE_EVENT_CATEGORIES = [
  "session.selectionChanged",
  "graph.committed",
  "runtime.crashed",
  "runtime.started",
  "runtime.activated",
  "runtime.stopped",
  "devices.changed",
  "devices.bindingInvalidated",
  "recovery.safeModeCleared",
  "virtualDevice.changed",
  "virtualBridge.failed",
  "virtualBridge.expired",
  "recorder.changed",
  "recording.metadataChanged",
  "recording.renamed",
  "recording.entryRemoved",
  "recording.recycled",
  "application.captureStateChanged",
] as const satisfies readonly StateEventCategory[];

/** Polls the backend's state events once a second; an event or a resync
 * refreshes the snapshot and the inventories it may have changed. */
export function useWorkspaceEvents({
  backend,
  hasSnapshot,
  session,
  snapshotCache,
  eventCursor,
  setSnapshotState,
  followBackendActiveSession,
  setActionMessage,
  refreshApplications,
  refreshDevices,
  refreshSessions,
  setRecordings,
  setRecordingsError,
  setRecorderStatuses,
  setRecorderStatusAvailable,
}: {
  backend: UiBackend;
  hasSnapshot: boolean;
  session: Session;
  snapshotCache: SnapshotCache;
  eventCursor: RefObject<{ backendEpoch: number; sequence: number }>;
  setSnapshotState: Dispatch<SetStateAction<UiSnapshotState>>;
  /** Adopts an active-session change made elsewhere (MCP, Stream Deck). */
  followBackendActiveSession: (sessionId: string) => boolean;
  setActionMessage: Dispatch<SetStateAction<string | null>>;
  refreshApplications: () => void;
  refreshDevices: (options?: { announce?: boolean }) => void;
  refreshSessions: () => Promise<unknown>;
  setRecordings: Dispatch<SetStateAction<RecordingRow[]>>;
  setRecordingsError: Dispatch<SetStateAction<string | null>>;
  setRecorderStatuses: Dispatch<SetStateAction<RecorderStatus[]>>;
  setRecorderStatusAvailable: Dispatch<SetStateAction<boolean>>;
}) {
  // Callbacks from the caller: always the latest, never a reason to restart polling.
  const follow = useEffectEvent((sessionId: string) => followBackendActiveSession(sessionId));
  const showMessage = useEffectEvent((message: string) => setActionMessage(message));
  const updateSnapshot = useEffectEvent((update: SetStateAction<UiSnapshotState>) => setSnapshotState(update));
  const refreshInventories = useEffectEvent(() => {
    refreshApplications();
    refreshDevices({ announce: false });
    void refreshSessions();
  });
  const showRecordings = useEffectEvent((items: RecordingRow[]) => {
    setRecordings(items);
    setRecordingsError(null);
  });
  const showRecordingsError = useEffectEvent((message: string) => setRecordingsError(message));
  const showRecorders = useEffectEvent((items: RecorderStatus[] | null) => {
    if (items) setRecorderStatuses(items);
    setRecorderStatusAvailable(items !== null);
  });
  useEffect(() => {
    if (!backend.connected || !hasSnapshot) return;
    let active = true;
    let polling = false;
    const poll = async () => {
      if (!active || polling) return;
      polling = true;
      try {
        const result = await backend.subscribe(
          eventCursor.current.sequence,
          session.id,
          eventCursor.current.backendEpoch,
          [...WORKSPACE_EVENT_CATEGORIES],
        );
        if (!active) return;
        const bindingInvalidatedEvent = result.events.find((event) => event.category === "devices.bindingInvalidated");
        const selectionChangedEvent = result.events.find((event) => event.category === "session.selectionChanged");
        const activeSession =
          selectionChangedEvent && backend.getActiveSession ? await backend.getActiveSession() : null;
        if (!active) return;
        const adoptedSessionId =
          activeSession?.sessionId && follow(activeSession.sessionId) ? activeSession.sessionId : session.id;
        const bridgeEvent = result.events.find(
          (event) => event.category === "virtualBridge.failed" || event.category === "virtualBridge.expired",
        );
        // Marked as errors: "stopped" otherwise reads as a success, which
        // showed a green check and closed itself after 8 s (owner, 2026-11-07).
        if (bindingInvalidatedEvent) {
          showMessage(
            markErrorMessage(
              "An audio device this route uses was disconnected or changed, so audio stopped. Reconnect it, then press Stop and Play.",
            ),
          );
        } else if (bridgeEvent) {
          const bus = bridgeEvent.operationId ?? "an affected bus";
          showMessage(
            markErrorMessage(
              bridgeEvent.category === "virtualBridge.expired"
                ? `The virtual cable link for ${bus} timed out, so that audio is silent. Press Stop and Play to restart it.`
                : `The virtual cable link for ${bus} failed, so that audio is silent. Press Stop and Play to restart it.`,
            ),
          );
        }
        if (result.resyncRequired || result.events.length > 0) {
          const nextState = await snapshotCache.refresh(backend, adoptedSessionId);
          if (active) {
            updateSnapshot(nextState);
            refreshInventories();
            void backend
              .listRecordings(session.id)
              .then((items) => {
                if (active) showRecordings(items);
              })
              .catch((error) => {
                if (active) showRecordingsError(formatUiError(error, "Recording library unavailable"));
              });
            void backend
              .listRecorders()
              .then((items) => {
                if (active) showRecorders(items);
              })
              .catch(() => {
                if (active) showRecorders(null);
              });
            if (!nextState.stale)
              eventCursor.current = { backendEpoch: result.backendEpoch, sequence: result.nextSequence };
          }
        } else {
          eventCursor.current = { backendEpoch: result.backendEpoch, sequence: result.nextSequence };
        }
      } catch (error) {
        if (active)
          updateSnapshot((current) => ({
            ...current,
            stale: true,
            error: formatUiError(error, "Event subscription failed"),
          }));
      } finally {
        polling = false;
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 1000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [backend, session.id, snapshotCache, hasSnapshot, eventCursor]);
}
