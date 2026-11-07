import type { Session } from "@audiorouter/contracts";
import { useCallback, useEffect, useEffectEvent, useRef, useState } from "react";
import { sameSessionDraft } from "./sessionInventory";

/**
 * Unsaved route edits survive quitting. Tray Quit and the window's Quit both
 * exit without asking the page, and a crash cannot ask at all, so the page keeps
 * a copy in a file beside the database while edits are unsaved, and offers it
 * back on the next launch. Only the desktop shell has that file; elsewhere this
 * does nothing.
 */

const WRITE_DELAY_MS = 400;

export interface StoredDraft {
  version: 1;
  sessionId: string;
  /** The saved revision these edits started from. */
  baseRevision: number;
  draft: Session;
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

/** Read a stored value back as a route, keeping only the fields a route has. */
export function decodeStoredDraft(value: unknown): StoredDraft | null {
  if (!isRecord(value) || value.version !== 1) return null;
  const { sessionId, baseRevision, draft } = value;
  if (typeof sessionId !== "string" || !sessionId) return null;
  if (typeof baseRevision !== "number" || !Number.isSafeInteger(baseRevision) || baseRevision < 0) return null;
  if (!isRecord(draft) || draft.id !== sessionId || draft.schemaVersion !== 1) return null;
  if (typeof draft.name !== "string" || !Array.isArray(draft.nodes) || !Array.isArray(draft.edges)) return null;
  const nodesLookValid = draft.nodes.every(
    (node) =>
      isRecord(node) &&
      typeof node.id === "string" &&
      typeof node.kind === "string" &&
      typeof node.name === "string" &&
      Array.isArray(node.ports) &&
      isRecord(node.parameters),
  );
  if (!nodesLookValid || !draft.edges.every(isRecord)) return null;
  return {
    version: 1,
    sessionId,
    baseRevision,
    draft: {
      id: sessionId,
      name: draft.name,
      schemaVersion: 1,
      revision: baseRevision,
      nodes: draft.nodes as unknown as Session["nodes"],
      edges: draft.edges as unknown as Session["edges"],
    },
  };
}

export type StoredDraftAction = "restore" | "already-saved" | "older" | "other-route" | "route-gone";

/** What a stored copy means now, given the route the backend has open. */
export function storedDraftAction(
  stored: StoredDraft,
  session: Session,
  knownIds: readonly string[],
): StoredDraftAction {
  if (stored.sessionId !== session.id) {
    return knownIds.length > 0 && !knownIds.includes(stored.sessionId) ? "route-gone" : "other-route";
  }
  if (stored.baseRevision !== session.revision) return "older";
  return sameSessionDraft(stored.draft, session) ? "already-saved" : "restore";
}

const shellInvoke = () => (typeof window === "undefined" ? undefined : window.__TAURI_INTERNALS__?.invoke);

export function useDraftRecovery({
  ready,
  draft,
  session,
  unsaved,
  knownSessionIds,
  onRestore,
  onOlder,
  onOtherRoute,
  onFailure,
}: {
  /** The backend's real route is open, not the startup placeholder. */
  ready: boolean;
  draft: Session;
  session: Session;
  unsaved: boolean;
  knownSessionIds: readonly string[];
  onRestore: (draft: Session) => void;
  /** The route was saved after these edits; the user decides which one wins. */
  onOlder: (draft: Session) => void;
  onOtherRoute: (sessionId: string) => void;
  onFailure: (message: string) => void;
}) {
  const [stored, setStored] = useState<StoredDraft | null>(null);
  const [loaded, setLoaded] = useState(false);
  /** A copy of this page's edits is on disk, so a clean page must remove it. */
  const wroteCopy = useRef(false);
  const timer = useRef<number | null>(null);
  const latest = useRef({ draft, session, unsaved });
  latest.current = { draft, session, unsaved };

  const reportFailure = useEffectEvent((error: unknown) => {
    const type = error instanceof Error ? error.name.slice(0, 48) : typeof error;
    onFailure(`Keeping unsaved edits failed (${type})`);
  });
  const write = useCallback(async (value: StoredDraft | null) => {
    const invoke = shellInvoke();
    if (!invoke) return;
    await invoke("set_ui_draft", { draft: value });
  }, []);
  const writeLatest = useCallback(async () => {
    const current = latest.current;
    if (!current.unsaved) return;
    wroteCopy.current = true;
    await write({
      version: 1,
      sessionId: current.session.id,
      baseRevision: current.session.revision,
      draft: current.draft,
    });
  }, [write]);
  const cancelTimer = () => {
    if (timer.current !== null) window.clearTimeout(timer.current);
    timer.current = null;
  };

  useEffect(() => {
    const invoke = shellInvoke();
    if (!invoke) return;
    let active = true;
    // Inside an async function, so a bridge that throws is reported like one that rejects.
    void (async () => invoke("get_ui_draft"))()
      .then((value) => {
        if (!active) return;
        setStored(decodeStoredDraft(value));
        setLoaded(true);
      })
      .catch((error: unknown) => {
        if (!active) return;
        reportFailure(error);
        setLoaded(true);
      });
    return () => {
      active = false;
    };
  }, []);

  // Decide once what the stored copy means. A copy for another route waits
  // until that route opens; a copy that cannot be used is removed.
  const toldOtherRoute = useRef(false);
  const answer = useEffectEvent((action: StoredDraftAction, copy: StoredDraft) => {
    if (action === "other-route") {
      if (!toldOtherRoute.current) onOtherRoute(copy.sessionId);
      toldOtherRoute.current = true;
      return;
    }
    setStored(null);
    if (action === "restore") {
      // The copy on disk now belongs to this page: discarding must remove it,
      // even before the next write.
      wroteCopy.current = true;
      onRestore(copy.draft);
    } else if (action === "older") onOlder(copy.draft);
    else void write(null).catch((error: unknown) => reportFailure(error));
  });
  const sessionId = session.id;
  const sessionRevision = session.revision;
  const knownIdsKey = knownSessionIds.join("\n");
  useEffect(() => {
    if (!ready || !stored) return;
    answer(storedDraftAction(stored, latest.current.session, knownIdsKey ? knownIdsKey.split("\n") : []), stored);
  }, [ready, stored, sessionId, sessionRevision, knownIdsKey]);

  // Keep the copy current. A copy waiting for another route stays until this
  // page has edits of its own to keep: the file holds one route's edits.
  const waitingForDecision = stored !== null && stored.sessionId === sessionId;
  useEffect(() => {
    if (!loaded || !ready || waitingForDecision) return;
    if (!unsaved) {
      if (wroteCopy.current) {
        wroteCopy.current = false;
        void write(null).catch((error: unknown) => reportFailure(error));
      }
      return;
    }
    cancelTimer();
    timer.current = window.setTimeout(() => {
      timer.current = null;
      setStored(null);
      void writeLatest().catch((error: unknown) => reportFailure(error));
    }, WRITE_DELAY_MS);
    return cancelTimer;
  }, [loaded, ready, waitingForDecision, unsaved, draft, write, writeLatest]);

  return {
    /** Write the current edits now, for a Quit that is about to exit. */
    flush: useCallback(async () => {
      cancelTimer();
      await writeLatest();
    }, [writeLatest]),
    /** Forget the stored copy: the user chose not to keep it. */
    forget: useCallback(async () => {
      setStored(null);
      wroteCopy.current = false;
      await write(null);
    }, [write]),
  };
}
