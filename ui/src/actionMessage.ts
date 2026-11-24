/**
 * Status-line messages carry their own severity (L10N-06). A message is made
 * from a message key through `say`, whose severity, progress state and button
 * come from the typed table in `actionMessageTones.ts`, or from an explicit
 * helper for text that is not a catalog message (a caught backend error, a
 * text a plugin supplies). Nothing here reads the wording of the text, so a
 * translated message keeps the same colour, expiry and buttons.
 */
import { tx } from "./i18n";
import type { MessageKey } from "./i18n/generated/messageKeys";
import { type MessageAction, MESSAGE_TONES, type ToneKey } from "./actionMessageTones";

export type ActionMessageTone = "error" | "warning" | "success" | "info";
export type { MessageAction, ToneKey };

export interface UiMessage {
  readonly text: string;
  readonly tone: ActionMessageTone;
  /** Work still in progress ("Saving…"): stays until it changes, whatever its tone. */
  readonly progress: boolean;
  /** The buttons this message offers beside its text, when it offers any. */
  readonly actions?: readonly MessageAction[];
  /** The catalog key the text was made from, when it was made from one. */
  readonly key?: MessageKey;
}

/** A catalog message with the severity declared for its key. */
export function say(key: ToneKey, options?: Record<string, unknown>): UiMessage {
  const declared: { tone: ActionMessageTone; progress?: true; actions?: readonly MessageAction[] } = MESSAGE_TONES[key];
  return {
    text: tx(key, options),
    tone: declared.tone,
    progress: declared.progress === true,
    ...(declared.actions ? { actions: declared.actions } : {}),
    key,
  };
}

/**
 * The same message shown with another severity, for a catalog message whose
 * tone depends on a structured value it reports (a recorder that `failed`).
 */
export function withTone(message: UiMessage, tone: ActionMessageTone): UiMessage {
  return { ...message, tone };
}

/** Text that describes a failure, such as a caught backend error. */
export function errorMessage(text: string): UiMessage {
  return { text, tone: "error", progress: false };
}

/** Plain information that is neither a failure, a warning nor a success. */
export function infoMessage(text: string): UiMessage {
  return { text, tone: "info", progress: false };
}

/**
 * A blank message that reserves the status line's space (UI-17) while the
 * caller shows the real text somewhere else.
 */
export const BLANK_MESSAGE: UiMessage = infoMessage(" ");

export const INFO_LIFETIME_MS = 8_000;
export const WARNING_LIFETIME_MS = 30_000;

/**
 * How long a message stays before closing itself (owner, 2026-11-07), or null
 * to keep it until dismissed. Errors stay so a failure is never missed;
 * progress text and messages offering a button stay until they change.
 */
export function actionMessageLifetimeMs(message: UiMessage | null | undefined, hasAction: boolean): number | null {
  if (!message || hasAction || message.progress) return null;
  switch (message.tone) {
    case "error":
      return null;
    case "warning":
      return WARNING_LIFETIME_MS;
    case "success":
    case "info":
      return INFO_LIFETIME_MS;
  }
}

/** The tone to draw a message with; no message reads as plain information. */
export function actionMessageTone(message: UiMessage | null | undefined): ActionMessageTone {
  return message?.tone ?? "info";
}

/** Whether a message offers the given button. */
export function offersAction(message: UiMessage | null | undefined, action: MessageAction): boolean {
  return message?.actions?.includes(action) === true;
}
