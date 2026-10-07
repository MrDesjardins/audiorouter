export const CLIENT_DIAGNOSTICS_STORAGE_KEY = "audiorouter.client-diagnostics.v1";
const MAX_ROWS = 80;
const MAX_ROW_CHARS = 320;

export type DiagnosticStorage = Pick<Storage, "getItem" | "setItem">;

export function browserDiagnosticStorage(): DiagnosticStorage | null {
  try {
    return typeof window === "undefined" ? null : window.localStorage;
  } catch {
    return null;
  }
}

export function readClientDiagnostics(storage: DiagnosticStorage | null): string[] {
  if (!storage) return [];
  try {
    const parsed: unknown = JSON.parse(storage.getItem(CLIENT_DIAGNOSTICS_STORAGE_KEY) ?? "[]");
    return Array.isArray(parsed)
      ? parsed
          .filter((row): row is string => typeof row === "string")
          .slice(0, MAX_ROWS)
          .map((row) => row.slice(0, MAX_ROW_CHARS))
      : [];
  } catch {
    return [];
  }
}

export function appendClientDiagnostic(
  storage: DiagnosticStorage | null,
  current: readonly string[],
  message: string,
  timestamp = new Date().toLocaleTimeString(),
): string[] {
  const next = [`${timestamp} ${message}`.slice(0, MAX_ROW_CHARS), ...current].slice(0, MAX_ROWS);
  try {
    storage?.setItem(CLIENT_DIAGNOSTICS_STORAGE_KEY, JSON.stringify(next));
  } catch {
    // Storage can be disabled or full; the in-memory diagnostic still works.
  }
  return next;
}

/** A failed backend request as the client diagnostics record it (P2-3). */
export type RpcFailureRow = { method: string; requestId: string; code?: number; kind?: string };

type RpcFailureListener = (message: string) => void;
const rpcFailureListeners = new Set<RpcFailureListener>();
const recentFailures = new Map<string, number>();
/** Identical failures (same method and category) within this window are logged once. */
const REPEAT_WINDOW_MS = 30_000;

const SAFE_TOKEN = /^[A-Za-z0-9.-]{1,96}$/;

/**
 * One short row for a failed request: method, error category and the
 * correlation ID to find it in shell.jsonl and backend.jsonl. Error text and
 * parameters are never included; unexpected tokens are dropped.
 */
export function formatRpcFailure(failure: RpcFailureRow): string {
  const method = SAFE_TOKEN.test(failure.method) ? failure.method : "request";
  const kind = failure.kind !== undefined && SAFE_TOKEN.test(failure.kind) ? failure.kind : undefined;
  const category =
    failure.code === undefined
      ? "no response"
      : (kind ?? `code ${Number.isFinite(failure.code) ? Math.trunc(failure.code) : "?"}`);
  const id = /^[A-Za-z0-9-]{1,32}$/.test(failure.requestId) ? ` [req ${failure.requestId}]` : "";
  return `RPC failed: ${method} (${category})${id}`;
}

/** Report a failed request to the window's client diagnostics, repeats collapsed. */
export function reportRpcFailure(failure: RpcFailureRow, now = Date.now()): void {
  const key = `${failure.method}|${failure.code ?? "transport"}|${failure.kind ?? ""}`;
  const last = recentFailures.get(key);
  if (last !== undefined && now - last < REPEAT_WINDOW_MS) return;
  recentFailures.set(key, now);
  if (recentFailures.size > 64) {
    const oldest = recentFailures.keys().next().value;
    if (oldest !== undefined) recentFailures.delete(oldest);
  }
  const message = formatRpcFailure(failure);
  for (const listener of rpcFailureListeners) listener(message);
}

/** Receive failed-request rows; returns the unsubscribe function. */
export function onRpcFailure(listener: RpcFailureListener): () => void {
  rpcFailureListeners.add(listener);
  return () => {
    rpcFailureListeners.delete(listener);
  };
}

/** Test hook: forget collapsed repeats. */
export function resetRpcFailureRepeats(): void {
  recentFailures.clear();
}
