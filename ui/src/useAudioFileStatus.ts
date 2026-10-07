// Audio File / Test Signal transport status polling (moved from App.tsx).
import { useEffect, type Dispatch, type SetStateAction } from "react";
import type { Session } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";
import { safeErrorName } from "./ErrorBoundary";

type AudioSourceStates = Record<string, "playing" | "paused" | "stopped">;

/** While the session plays, reads the transport state of up to 16 playing
 * Audio File and Test Signal nodes every 750 ms. */
export function useAudioFileStatus({
  backend,
  draft,
  session,
  sessionRunning,
  audioSourceStates,
  setAudioSourceStates,
  recordUiDiagnostic,
}: {
  backend: UiBackend;
  draft: Session;
  session: Session;
  sessionRunning: boolean;
  audioSourceStates: AudioSourceStates;
  setAudioSourceStates: Dispatch<SetStateAction<AudioSourceStates>>;
  recordUiDiagnostic: (message: string) => void;
}) {
  useEffect(() => {
    const playingNodes = draft.nodes
      .filter(
        (node) => (node.kind === "audioFile" || node.kind === "testSignal") && audioSourceStates[node.id] === "playing",
      )
      .slice(0, 16);
    if (!sessionRunning) {
      if (Object.keys(audioSourceStates).length > 0) setAudioSourceStates({});
      return;
    }
    if (!backend.connected || playingNodes.length === 0) return;
    let polling = false;
    let statusFailing = false;
    const timer = window.setInterval(() => {
      if (polling) return;
      polling = true;
      void Promise.all(
        playingNodes.map(
          async (node) => [node.id, await backend.transportAudioSource(session.id, node.id, "status")] as const,
        ),
      )
        .then((states) =>
          setAudioSourceStates((current) => {
            let changed = false;
            const next = { ...current };
            for (const [nodeId, result] of states)
              if (next[nodeId] !== result.state) {
                next[nodeId] = result.state;
                changed = true;
              }
            return changed ? next : current;
          }),
        )
        .then(() => {
          statusFailing = false;
        })
        .catch((error: unknown) => {
          // Keep the last states shown; note the first failure of a run.
          if (!statusFailing) recordUiDiagnostic(`Audio file status unavailable (${safeErrorName(error)})`);
          statusFailing = true;
        })
        .finally(() => {
          polling = false;
        });
    }, 750);
    return () => window.clearInterval(timer);
  }, [backend, draft.nodes, session.id, sessionRunning, audioSourceStates]);
}
