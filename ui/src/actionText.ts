import { tx } from "./i18n";
import type { MessageKey } from "./i18n/generated/messageKeys";

/**
 * Translates a catalog message made outside React. A status message that needs
 * a severity is made with `say` (actionMessage.ts) instead; nothing reads the
 * wording of the text to decide how it is shown (L10N-06).
 */
export function tm(key: MessageKey, options?: Record<string, unknown>): string {
  return tx(key, options);
}

const BACKEND_TEXTS: Record<string, MessageKey> = {
  "The control backend is disconnected.": "h3.backendDisconnected",
  "The backend is disconnected; processor catalog is unavailable.": "h3.backendNoProcessorCatalog",
  "The backend is disconnected; preset catalog is unavailable.": "h3.backendNoPresetCatalog",
};

/** Translates the fixed disconnected-backend texts made in `backend.ts` when they are shown. */
export function backendText<T extends string | null | undefined>(text: T): T | string {
  const key = text ? BACKEND_TEXTS[text] : undefined;
  return key ? tm(key) : text;
}
