// Native route counters, read once a second while routes play (moved from App.tsx).
import { useEffect, type Dispatch, type SetStateAction } from "react";
import type {
  NativeDuplexPumpResult,
  NativeEndpointPumpResult,
  NativeMultiInputPumpResult,
  NativeRenderSourcePumpResult,
  Session,
} from "@audiorouter/contracts";
import type { UiBackend, UiSnapshotState } from "./backend";
import { selectNativePump } from "./nativePump";
import { safeErrorName } from "./ErrorBoundary";

/**
 * How often the UI reads native route counters. The backend pumps native
 * audio itself every 1 ms (its audio service); the UI never paces audio.
 */
export const NATIVE_COUNTERS_REFRESH_MS = 1000;

export type NativePumpStats =
  NativeEndpointPumpResult | NativeDuplexPumpResult | NativeMultiInputPumpResult | NativeRenderSourcePumpResult;

export function formatNativePumpSummary(stats: NativePumpStats | null, running: boolean): string | null {
  const summary = formatNativePumpCounters(stats, running);
  const service = stats?.audioService;
  if (summary === null || !service || service.lateGaps === 0) return summary;
  // Late backend service passes are the audible-continuity risk: report them.
  return `${summary} / ${service.lateGaps} late audio service gap${service.lateGaps === 1 ? "" : "s"} (max ${(service.maxGapMicros / 1000).toFixed(1)} ms)`;
}

export function formatNativePumpCounters(stats: NativePumpStats | null, running: boolean): string | null {
  if (!stats || !running) return null;
  if ("submittedQuanta" in stats) {
    return `native multi-input ${stats.capturedFrames} in / ${stats.renderedFrames} out / ${stats.submittedQuanta} quanta${stats.outputCount > 0 ? ` / ${stats.deliveredQuanta} branches` : ""}${stats.renderBackpressureEvents > 0 ? ` / ${stats.renderBackpressureEvents} backpressure` : ""}${(stats.outputUnderruns ?? 0) > 0 ? ` / ${stats.outputUnderruns} output underrun${stats.outputUnderruns === 1 ? "" : "s"}` : ""}`;
  }
  if (!("capturedFrames" in stats) && !("input" in stats)) {
    return `native render-source ${stats.renderedFrames} out / ${stats.processedQuanta} quanta${stats.droppedRenderFrames > 0 ? ` / ${stats.droppedRenderFrames} dropped` : ""}`;
  }
  const input = "input" in stats ? stats.input : stats;
  const output = "output" in stats ? stats.output : stats;
  const warnings = [
    output.droppedRenderFrames > 0 ? `${output.droppedRenderFrames} dropped` : null,
    output.renderBackpressureEvents > 0 ? `${output.renderBackpressureEvents} backpressure` : null,
  ].filter((value): value is string => value !== null);
  const recorderChunks = "recorderChunksDrained" in stats ? stats.recorderChunksDrained : 0;
  const processedQuanta = "input" in stats ? input.processedQuanta + output.processedQuanta : stats.processedQuanta;
  return `native ${input.capturedFrames} in / ${output.renderedFrames} out / ${processedQuanta} quanta${recorderChunks > 0 ? ` / ${recorderChunks} recorder chunks` : ""}${warnings.length > 0 ? ` / ${warnings.join(" / ")}` : ""}`;
}

/** Reads the counters of every running native route each
 * NATIVE_COUNTERS_REFRESH_MS; the selected session's go to the status line. */
export function useNativeCounters({
  backend,
  nativeGenerations,
  snapshot,
  session,
  setNativePumpStats,
  recordUiDiagnostic,
}: {
  backend: UiBackend;
  nativeGenerations: Record<string, { generation: number; kind: string }>;
  snapshot: UiSnapshotState["snapshot"];
  session: Session;
  setNativePumpStats: Dispatch<SetStateAction<NativePumpStats | null>>;
  recordUiDiagnostic: (message: string) => void;
}) {
  useEffect(() => {
    const pumpNativeEndpoint = backend.pumpNativeEndpoint;
    const pumpNativeDuplex = backend.pumpNativeDuplex;
    const pumpNativeRenderSource = backend.pumpNativeRenderSource;
    const pumpNativeMultiInputs = backend.pumpNativeMultiInputs;
    const activeRoutes = Object.entries(nativeGenerations).filter(([sessionId]) =>
      snapshot?.status.activeSessionIds.includes(sessionId),
    );
    if (!backend.connected || activeRoutes.length === 0 || !pumpNativeEndpoint) return;
    let active = true;
    let pumping = false;
    let failing = false;
    // The backend services native audio itself; this loop only reads each
    // route's counters (the pump call returns them) once a second.
    const pump = async () => {
      if (!active || pumping) return;
      pumping = true;
      try {
        for (const [sessionId, route] of activeRoutes) {
          if (!active) return;
          // Each independently prepared endpoint route has its own scheduler
          // and graph. Service every running route even when another session
          // is selected in the sidebar.
          const pumpKind = selectNativePump(
            route.kind,
            Boolean(pumpNativeEndpoint),
            Boolean(pumpNativeDuplex),
            Boolean(pumpNativeRenderSource),
            Boolean(pumpNativeMultiInputs),
          );
          if (!pumpKind) continue;
          const result =
            pumpKind === "multiInput"
              ? await pumpNativeMultiInputs!(sessionId, route.generation, 64)
              : pumpKind === "duplex"
                ? await pumpNativeDuplex!(sessionId, route.generation, 64, 64)
                : pumpKind === "renderSource"
                  ? await pumpNativeRenderSource!(sessionId, route.generation, 64)
                  : await pumpNativeEndpoint!(sessionId, route.generation, 64);
          if (sessionId === session.id) setNativePumpStats(result);
        }
        failing = false;
      } catch (error) {
        setNativePumpStats(null);
        // Diagnostics remain backend-owned; do not retry or substitute
        // endpoints here. Record the first failure of a run, not every tick.
        if (!failing) recordUiDiagnostic(`Native route counters unavailable (${safeErrorName(error)})`);
        failing = true;
      } finally {
        pumping = false;
      }
    };
    void pump();
    const timer = window.setInterval(() => void pump(), NATIVE_COUNTERS_REFRESH_MS);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [backend, nativeGenerations, session.id, snapshot?.status.activeSessionIds]);
}
