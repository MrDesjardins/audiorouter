// Audio File / Test Signal transport status polling (moved from App.tsx).
import { useEffect, useEffectEvent, type Dispatch, type SetStateAction, type RefObject } from "react";
import type { Session } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";
import { safeErrorName } from "./ErrorBoundary";

type AudioSourceStates = Record<string, "playing" | "paused" | "stopped">;
export type SourceTransportActivity = Map<string, { version: number; pending: number }>;

/** Discover external playback too. Rotate through all sources in batches of at most 16. */
export function useAudioFileStatus({
  backend,
  draft,
  session,
  sessionRunning,
  audioSourceStates,
  setAudioSourceStates,
  recordUiDiagnostic,
  transportActivity,
}: {
  backend: UiBackend;
  draft: Session;
  session: Session;
  sessionRunning: boolean;
  audioSourceStates: AudioSourceStates;
  setAudioSourceStates: Dispatch<SetStateAction<AudioSourceStates>>;
  recordUiDiagnostic: (message: string) => void;
  transportActivity?: RefObject<SourceTransportActivity>;
}) {
  // Callbacks from the caller: always the latest, never a reason to restart polling.
  const reportFailure = useEffectEvent((message: string) => recordUiDiagnostic(message));
  const updateStates = useEffectEvent((update: SetStateAction<AudioSourceStates>) => setAudioSourceStates(update));
  const clearStates = useEffectEvent(() => {
    if (Object.keys(audioSourceStates).length > 0) setAudioSourceStates({});
  });
  const sourceIds = JSON.stringify(
    draft.nodes.filter((node) => node.kind === "audioFile" || node.kind === "testSignal").map((node) => node.id),
  );
  const sessionId = session.id;
  useEffect(() => {
    const ids = JSON.parse(sourceIds) as string[];
    if (!sessionRunning) {
      clearStates();
      return;
    }
    if (!backend.connected || ids.length === 0) return;
    let active = true;
    let offset = 0;
    let polling = false;
    let statusFailing = false;
    const timer = window.setInterval(() => {
      if (polling) return;
      polling = true;
      const batch = Array.from({ length: Math.min(16, ids.length) }, (_, index) => ids[(offset + index) % ids.length]);
      offset = (offset + batch.length) % ids.length;
      void Promise.allSettled(
        batch.map(async (nodeId) => {
          const key = JSON.stringify([sessionId, nodeId]);
          const before = transportActivity?.current.get(key);
          if (before?.pending) return null;
          const version = before?.version ?? 0;
          const result = await backend.transportAudioSource(sessionId, nodeId, "status");
          return { nodeId, result, key, version };
        }),
      )
        .then((states) => {
          if (!active) return;
          const failure = states.find((item) => item.status === "rejected");
          if (failure?.status === "rejected" && !statusFailing)
            reportFailure(`Audio file status unavailable (${safeErrorName(failure.reason)})`);
          statusFailing = Boolean(failure);
          updateStates((current) => {
            if (!active) return current;
            let changed = false;
            const next = { ...current };
            for (const item of states) {
              if (item.status !== "fulfilled" || !item.value) continue;
              const { nodeId, result, key, version } = item.value;
              const activity = transportActivity?.current.get(key);
              if (activity?.pending || (activity?.version ?? 0) !== version) continue;
              if (next[nodeId] !== result.state) {
                next[nodeId] = result.state;
                changed = true;
              }
            }
            return changed ? next : current;
          });
        })
        .catch((error: unknown) => {
          // Keep the last states shown; note the first failure of a run.
          if (active && !statusFailing) reportFailure(`Audio file status unavailable (${safeErrorName(error)})`);
          statusFailing = true;
        })
        .finally(() => {
          polling = false;
        });
    }, 750);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [backend, sourceIds, sessionId, sessionRunning, transportActivity]);
}
