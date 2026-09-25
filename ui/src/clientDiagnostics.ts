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
      ? parsed.filter((row): row is string => typeof row === "string").slice(0, MAX_ROWS).map((row) => row.slice(0, MAX_ROW_CHARS))
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
