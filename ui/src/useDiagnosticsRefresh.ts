// Live diagnostics and meter refresh while audio plays (moved from App.tsx).
import { useEffect, useEffectEvent, type Dispatch, type RefObject, type SetStateAction } from "react";
import type { DiagnosticsSnapshot } from "@audiorouter/contracts";
import type { UiBackend, UiSnapshotState } from "./backend";
import { differsOnlyInTelemetry, type TelemetryStore } from "./liveTelemetry";
import { safeErrorName } from "./ErrorBoundary";

/** Default diagnostics/meter refresh: 20 Hz, below the API's 30 Hz ceiling. */
export const DIAGNOSTICS_REFRESH_INTERVAL_MS = 50;

/** While the session runs, refreshes diagnostics every
 * DIAGNOSTICS_REFRESH_INTERVAL_MS; meter-only changes go to the telemetry
 * store instead of re-rendering the app. */
export function useDiagnosticsRefresh({
  backend,
  sessionRunning,
  telemetryStore,
  diagnosticsRef,
  setSnapshotState,
  recordUiDiagnostic,
}: {
  backend: UiBackend;
  sessionRunning: boolean;
  telemetryStore: TelemetryStore;
  diagnosticsRef: RefObject<DiagnosticsSnapshot | null>;
  setSnapshotState: Dispatch<SetStateAction<UiSnapshotState>>;
  recordUiDiagnostic: (message: string) => void;
}) {
  // Callbacks from the caller: always the latest, never a reason to restart the 20 Hz loop.
  const reportFailure = useEffectEvent((message: string) => recordUiDiagnostic(message));
  const updateSnapshot = useEffectEvent((update: SetStateAction<UiSnapshotState>) => setSnapshotState(update));
  useEffect(() => {
    if (!backend.connected || !sessionRunning) return;
    let active = true;
    let refreshing = false;
    let failing = false;
    const refreshDiagnostics = async () => {
      if (!active || refreshing) return;
      refreshing = true;
      try {
        const diagnostics = await backend.refreshDiagnostics();
        if (!active) return;
        // Meters alone change between ticks: publish them to the live views
        // only, instead of re-rendering the whole app 20 times a second.
        const basis = diagnosticsRef.current;
        if (basis && differsOnlyInTelemetry(basis, diagnostics))
          telemetryStore.set({ basis, nodeTelemetry: diagnostics.nodeTelemetry });
        else
          updateSnapshot((current) =>
            current.snapshot ? { ...current, snapshot: { ...current.snapshot, diagnostics } } : current,
          );
        failing = false;
      } catch (error) {
        // Keep the last known diagnostics; the event/snapshot path reports
        // connection failures and never replaces observations with guesses.
        // Record the first failure of a run, not one row per 50 ms tick.
        if (!failing) reportFailure(`Live diagnostics refresh failed (${safeErrorName(error)})`);
        failing = true;
      } finally {
        refreshing = false;
      }
    };
    void refreshDiagnostics();
    const timer = window.setInterval(() => void refreshDiagnostics(), DIAGNOSTICS_REFRESH_INTERVAL_MS);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [backend, sessionRunning, telemetryStore, diagnosticsRef]);
}
