import { optionalLocalStorage } from "./presentationStorage";
export const CLIENT_DIAGNOSTICS_STORAGE_KEY = "audiorouter.client-diagnostics.v1";
const MAX_ROWS = 80;
const MAX_ROW_CHARS = 320;

export type DiagnosticStorage = Pick<Storage, "getItem" | "setItem">;

export function browserDiagnosticStorage(): DiagnosticStorage | null {
  return optionalLocalStorage();
}

export function readClientDiagnostics(storage: DiagnosticStorage | null): string[] {
  if (!storage) return [];
  try {
    const parsed: unknown = JSON.parse(storage.getItem(CLIENT_DIAGNOSTICS_STORAGE_KEY) ?? "[]");
    return Array.isArray(parsed)
      ? parsed
          .filter((row): row is string => typeof row === "string")
          // Rows from older versions that say nothing: they were harmless browser resize notes.
          .filter((row) => !row.endsWith("UI error (unknown)"))
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

/** The parts of a window error event the log reads. */
export type WindowErrorDetails = {
  message: string;
  error: unknown;
  filename: string;
  lineno: number;
  colno: number;
};

/** Fixed descriptions of errors browsers word predictably. The error's own text never reaches the log. */
const KNOWN_ERROR_REASONS: readonly (readonly [RegExp, string])[] = [
  [/Cannot (read|set) propert|undefined is not an object|null is not an object/, "used a value that was missing"],
  [/is not a function/, "called something that is not a function"],
  [/is not defined/, "used a name that is not defined"],
  [/Maximum update depth|Too many re-renders/, "a screen kept redrawing without stopping"],
  [/Maximum call stack/, "ran out of call stack"],
  [/out of memory/i, "ran out of memory"],
  [
    /dynamically imported module|Importing a module script failed|Loading chunk/,
    "a part of the interface failed to load",
  ],
  [/^(Uncaught )?Script error\.?$/, "the browser hid the details"],
];

function knownErrorReason(text: string): string | null {
  const react = /Minified React error #(\d{1,5})/.exec(text);
  if (react) return `React error ${react[1]}`;
  return KNOWN_ERROR_REASONS.find(([pattern]) => pattern.test(text))?.[1] ?? null;
}

const errorName = (value: unknown): string | null => {
  if (!(value instanceof Error)) return null;
  return /^[A-Za-z][A-Za-z0-9_.-]{0,47}$/.test(value.name) ? value.name : "Error";
};

/**
 * A log row for an uncaught error, or null for the browser's harmless note that it moved a resize
 * update to the next frame (it has no error object and fires in bursts while resizing). The row
 * holds the error type, its place in the bundle and a fixed description, never the raw text.
 */
export function describeWindowError(details: WindowErrorDetails): string | null {
  if (/ResizeObserver loop/.test(details.message)) return null;
  const name = errorName(details.error);
  const script = (details.filename.split(/[?#]/, 1)[0].split(/[\\/]/).pop() ?? "")
    .replace(/[^A-Za-z0-9_.-]/g, "_")
    .slice(0, 64);
  const location = details.lineno > 0 ? ` at ${script || "page"}:${details.lineno}:${details.colno}` : "";
  const reason = knownErrorReason(details.message) ?? (name ? null : "the browser gave no details");
  return `UI error${name ? ` (${name})` : ""}${location}${reason ? `: ${reason}` : ""}`;
}

/** A log row for an unhandled promise rejection, without the rejection's own text. */
export function describePromiseRejection(reason: unknown): string {
  const known = reason instanceof Error ? knownErrorReason(reason.message) : null;
  const type = errorName(reason) ?? (/^[a-z]+$/.test(typeof reason) ? typeof reason : "Error");
  return `Unhandled promise rejection (${type})${known ? `: ${known}` : ""}`;
}

/** True the first time a row appears in the repeat window, so one fault cannot fill the 80-row log. */
export function isNewUiErrorRow(row: string, now = Date.now()): boolean {
  const key = `ui|${row}`;
  const last = recentFailures.get(key);
  if (last !== undefined && now - last < REPEAT_WINDOW_MS) return false;
  recentFailures.set(key, now);
  if (recentFailures.size > 64) {
    const oldest = recentFailures.keys().next().value;
    if (oldest !== undefined) recentFailures.delete(oldest);
  }
  return true;
}

/** Test hook: forget collapsed repeats. */
export function resetRpcFailureRepeats(): void {
  recentFailures.clear();
}
