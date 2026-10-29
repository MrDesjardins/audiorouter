// Live Soundboard state and triggers, kept out of App state (like liveTelemetry.tsx):
// polling runs several times a second while a board plays, and only the cards and
// the Properties editor that show it subscribe.
import { createContext, useContext, useEffect, useEffectEvent, useState, useSyncExternalStore } from "react";
import type { Session } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";

export type SoundboardSlotLive = { slot: number; loaded: boolean; playing: boolean; progress: number };
export type SoundboardLive = { slots: SoundboardSlotLive[]; ignoredTriggers: number };

export type SoundboardStore = {
  get: (nodeId: string) => SoundboardLive | null;
  set: (values: Record<string, SoundboardLive>) => void;
  subscribe: (listener: () => void) => () => void;
};

export function createSoundboardStore(): SoundboardStore {
  let values: Record<string, SoundboardLive> = {};
  const listeners = new Set<() => void>();
  return {
    get: (nodeId) => values[nodeId] ?? null,
    set: (next) => {
      values = next;
      for (const listener of listeners) listener();
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

export type SoundboardController = {
  store: SoundboardStore;
  /** Whether a trigger can reach a running board right now. */
  ready: boolean;
  trigger: (nodeId: string, slot: number, action?: "press" | "release") => void;
  stop: (nodeId: string, slot?: number) => void;
};

const idleStore = createSoundboardStore();
const idleController: SoundboardController = { store: idleStore, ready: false, trigger: () => {}, stop: () => {} };
export const SoundboardContext = createContext<SoundboardController>(idleController);

/** The live state of one board, or null when it is not running. */
export function useSoundboardLive(nodeId: string): SoundboardLive | null {
  const { store } = useContext(SoundboardContext);
  return useSyncExternalStore(
    store.subscribe,
    () => store.get(nodeId),
    () => store.get(nodeId),
  );
}

const POLL_MS = 150;

/**
 * Builds the controller for the App: while the session runs it reads the status of every
 * enabled Soundboard, and its triggers go straight to the backend (start the session
 * first if the owner pressed a button before pressing Start, like the Audio File Play button).
 */
export function useSoundboardController({
  backend,
  session,
  sessionRunning,
  startSession,
  report,
}: {
  backend: UiBackend;
  session: Session;
  sessionRunning: boolean;
  startSession: () => Promise<boolean>;
  report: (message: string) => void;
}): SoundboardController {
  const [store] = useState(createSoundboardStore);
  const nodeIds = session.nodes
    .filter((node) => node.kind === "soundboard" && node.enabled)
    .map((node) => node.id)
    .join("|");
  const sessionId = session.id;
  const reportFailure = useEffectEvent((message: string) => report(message));
  useEffect(() => {
    const ids = nodeIds === "" ? [] : nodeIds.split("|");
    if (!sessionRunning || !backend.connected || !backend.soundboardStatus || ids.length === 0) {
      store.set({});
      return;
    }
    let polling = false;
    let failing = false;
    const read = backend.soundboardStatus.bind(backend);
    const timer = window.setInterval(() => {
      if (polling) return;
      polling = true;
      void Promise.all(ids.map(async (id) => [id, await read(sessionId, id)] as const))
        .then((results) => {
          failing = false;
          store.set(
            Object.fromEntries(
              results.map(([id, status]) => [
                id,
                { slots: status.slots, ignoredTriggers: status.ignoredTriggers } satisfies SoundboardLive,
              ]),
            ),
          );
        })
        .catch(() => {
          // A board that is not prepared yet is normal right after Start; report a real outage once.
          if (!failing) reportFailure("Soundboard status is unavailable.");
          failing = true;
        })
        .finally(() => {
          polling = false;
        });
    }, POLL_MS);
    return () => {
      window.clearInterval(timer);
      store.set({});
    };
  }, [backend, nodeIds, sessionId, sessionRunning, store]);
  const trigger = (nodeId: string, slot: number, action: "press" | "release" = "press") => {
    const send = backend.soundboardTrigger;
    if (!send || !backend.connected) return;
    void (async () => {
      try {
        if (action === "press" && !sessionRunning && !(await startSession())) return;
        await send.call(backend, sessionId, nodeId, slot, action);
      } catch {
        report("The sound could not be played. Check that the session is running.");
      }
    })();
  };
  const stop = (nodeId: string, slot?: number) => {
    const send = backend.soundboardStop;
    if (!send || !backend.connected || !sessionRunning) return;
    void send.call(backend, sessionId, nodeId, slot).catch(() => report("The sounds could not be stopped."));
  };
  return { store, ready: backend.connected && sessionRunning, trigger, stop };
}
