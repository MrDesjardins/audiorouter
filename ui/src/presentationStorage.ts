/** Far above 64 nodes/128 edges/64 groups, but bounded before JSON parsing. */
export const MAX_PRESENTATION_JSON_CHARS = 1_048_576;
export function optionalLocalStorage(): Storage | null {
  try {
    return typeof window === "undefined" ? null : window.localStorage;
  } catch {
    return null;
  }
}
export function readPresentationJson(storage: Pick<Storage, "getItem"> | null, key: string): unknown {
  try {
    const raw = storage?.getItem(key);
    return raw && raw.length <= MAX_PRESENTATION_JSON_CHARS ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}
export function writePresentationJson(storage: Pick<Storage, "setItem"> | null, key: string, value: unknown): void {
  try {
    if (!storage) return;
    const raw = JSON.stringify(value);
    if (raw && raw.length <= MAX_PRESENTATION_JSON_CHARS) storage.setItem(key, raw);
  } catch {
    /* Optional preferences cannot interrupt the in-memory editor. */
  }
}
