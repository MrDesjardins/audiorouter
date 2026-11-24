// Live Soundboard state and triggers, kept out of App state (like liveTelemetry.tsx):
// polling runs several times a second while a board plays, and only the cards and
// the Properties editor that show it subscribe.
import { createContext, useContext, useEffect, useEffectEvent, useRef, useState, useSyncExternalStore } from "react";
import type { Session } from "@audiorouter/contracts";
import { say, type UiMessage } from "./actionMessage";
import { tm } from "./actionText";
import { type UiBackend, uiError } from "./backend";
import { importSoundFile, type ImportedSound } from "./soundImport";

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
  /** Whether a user file is being imported; one import runs at a time. */
  importing: boolean;
  /**
   * Imports the owner's file onto `slot` of the draft (same checks as the picker) and
   * resolves with the message to show. `keepCaption` when replacing a slot's sound.
   */
  importFile: (
    nodeId: string,
    slot: number,
    file: File,
    keepCaption: boolean,
  ) => Promise<{ ok: boolean; message: UiMessage }>;
  /** Shows a message to the owner in the app's status line. */
  announce: (message: UiMessage) => void;
  /** Puts a sound on a slot of the draft as one change, clearing the slot's random set. */
  assign: (nodeId: string, slot: number, sound: { mediaId: string; fileName: string }, keepCaption: boolean) => void;
};

const idleStore = createSoundboardStore();
const idleController: SoundboardController = {
  store: idleStore,
  ready: false,
  trigger: () => {},
  stop: () => {},
  importing: false,
  importFile: async () => ({ ok: false, message: say("h1.importUnavailable") }),
  announce: () => {},
  assign: () => {},
};
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
  assign = () => {},
  announce = () => {},
}: {
  backend: UiBackend;
  session: Session;
  sessionRunning: boolean;
  startSession: () => Promise<boolean>;
  report: (message: string) => void;
  /** Puts an imported sound on a slot of the draft as one change. */
  assign?: (
    nodeId: string,
    slot: number,
    sound: Pick<ImportedSound, "mediaId" | "fileName">,
    keepCaption: boolean,
  ) => void;
  announce?: (message: UiMessage) => void;
}): SoundboardController {
  const [store] = useState(createSoundboardStore);
  const [importing, setImporting] = useState(false);
  const pendingPresses = useRef(new Map<string, { released: boolean }>());
  const controlActivity = useRef({ version: 0, pending: 0 });
  // The ref closes the gap between a second drop and the re-render that disables it.
  const importBusy = useRef(false);
  const importFile = async (nodeId: string, slot: number, file: File, keepCaption: boolean) => {
    if (importBusy.current) return { ok: false, message: say("h1.importBusy") };
    importBusy.current = true;
    setImporting(true);
    try {
      const sound = await importSoundFile(backend, file);
      assign(nodeId, slot, sound, keepCaption);
      return {
        ok: true,
        message: say("h1.importDone", {
          file: sound.fileName,
          slot,
          seconds: Math.round(sound.durationMs / 1000),
        }),
      };
    } catch (error) {
      return { ok: false, message: uiError(error, tm("h1.importFailed")) };
    } finally {
      importBusy.current = false;
      setImporting(false);
    }
  };
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
    let active = true;
    let failing = false;
    const read = backend.soundboardStatus.bind(backend);
    const timer = window.setInterval(() => {
      if (polling || controlActivity.current.pending > 0) return;
      polling = true;
      const version = controlActivity.current.version;
      void Promise.all(ids.map(async (id) => [id, await read(sessionId, id)] as const))
        .then((results) => {
          if (!active || controlActivity.current.pending > 0 || controlActivity.current.version !== version) return;
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
          if (active && !failing) reportFailure(tm("h1.statusUnavailable"));
          failing = true;
        })
        .finally(() => {
          polling = false;
        });
    }, POLL_MS);
    return () => {
      active = false;
      window.clearInterval(timer);
      store.set({});
    };
  }, [backend, nodeIds, sessionId, sessionRunning, store]);
  const trigger = (nodeId: string, slot: number, action: "press" | "release" = "press") => {
    const key = JSON.stringify([sessionId, nodeId, slot]);
    const pending = pendingPresses.current.get(key);
    if (action === "release" && pending) {
      pending.released = true;
      return;
    }
    const send = backend.soundboardTrigger;
    if (!send || !backend.connected) return;
    const press = action === "press" ? { released: false } : null;
    if (press) pendingPresses.current.set(key, press);
    controlActivity.current.version += 1;
    controlActivity.current.pending += 1;
    void (async () => {
      try {
        if (action === "press" && !sessionRunning && !(await startSession())) return;
        // A key/pointer can be released while starting the route. Do not
        // send release first and then start a held voice after the user lets go.
        if (press?.released) return;
        await send.call(backend, sessionId, nodeId, slot, action);
        if (press?.released) await send.call(backend, sessionId, nodeId, slot, "release");
      } catch {
        report(tm("h1.playFailed"));
      } finally {
        controlActivity.current.version += 1;
        controlActivity.current.pending -= 1;
        if (press && pendingPresses.current.get(key) === press) pendingPresses.current.delete(key);
      }
    })();
  };
  const stop = (nodeId: string, slot?: number) => {
    const send = backend.soundboardStop;
    if (!send || !backend.connected || !sessionRunning) return;
    controlActivity.current.version += 1;
    controlActivity.current.pending += 1;
    void send
      .call(backend, sessionId, nodeId, slot)
      .catch(() => report(tm("h1.stopFailed")))
      .finally(() => {
        controlActivity.current.version += 1;
        controlActivity.current.pending -= 1;
      });
  };
  return { store, ready: backend.connected && sessionRunning, trigger, stop, importing, importFile, announce, assign };
}
