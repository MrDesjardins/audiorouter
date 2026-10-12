let fallbackIdempotencyCounter = 0;

/** Creates a compact key; the backend separately scopes it by caller and method. */
export function uiIdempotencyKey(_operation: string): string {
  const randomValues = globalThis.crypto?.getRandomValues?.(new Uint32Array(4));
  const nonce =
    globalThis.crypto?.randomUUID?.() ??
    (randomValues
      ? Array.from(randomValues, (value) => value.toString(16).padStart(8, "0")).join("")
      : `${Date.now()}-${fallbackIdempotencyCounter++}`);
  return `ui-${nonce}`;
}
