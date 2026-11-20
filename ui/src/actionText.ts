import { i18n, tx } from "./i18n";
import type { MessageKey } from "./i18n/generated/messageKeys";

/**
 * Status-line messages are classified (error, warning, success, progress) and
 * matched by their English wording. A translated message therefore remembers
 * its English rendering, so the classification and the "offer a button" checks
 * work in every language.
 */
const englishByText = new Map<string, string>();
const MAX_REMEMBERED = 400;

/** Translates a message and remembers its English rendering. */
export function tm(key: MessageKey, options?: Record<string, unknown>): string {
  const text = tx(key, options);
  if (i18n.language !== "en") {
    const english = (i18n.t as unknown as (key: string, options?: Record<string, unknown>) => string)(key, {
      ...options,
      lng: "en",
    });
    if (english !== text) {
      if (englishByText.size >= MAX_REMEMBERED) englishByText.delete(englishByText.keys().next().value as string);
      englishByText.set(text, english);
    }
  }
  return text;
}

/** The English wording of a message made by `tm`, or the message itself. */
export function englishOf(message: string | null | undefined): string {
  if (!message) return "";
  return englishByText.get(message) ?? message;
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
