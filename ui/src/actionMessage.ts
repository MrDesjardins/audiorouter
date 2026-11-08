/**
 * Classifies a message so errors, warnings, successes and plain progress are
 * visually distinct (red, orange, green, blue). Text produced from a caught
 * error by `formatUiError` is always an error, whatever its wording; other
 * messages are plain strings from many UI paths, so they are classified by
 * their stable phrasing.
 */
export type ActionMessageTone = "error" | "warning" | "success" | "info";

const ERROR_PATTERNS = [
  /^no audio (started|played)/i,
  /^change rejected/i,
  /^(could not|unable to|cannot)\b/i,
  /\b(permission denied|not permitted|access denied)\b/i,
  /\b(failed|rejected|unavailable|cannot play|cannot prepare|is not attached|not changed)\b/i,
];

const WARNING_PATTERNS = [
  /\bwarnings?\b/i,
  /\b(press play again|stop and press play|stop the session|changed while|not measured|is missing)\b/i,
  /^review\b/i,
];

const SUCCESS_PATTERNS = [
  /\b(is running|was saved|route saved|saved\b|started|stopped|prepared|restored|applied|copied|connected|deleted|created)\b/i,
];

/** Texts produced from caught errors; bounded, most recent kept. */
const errorTexts: string[] = [];
const MAX_ERROR_TEXTS = 64;

/** Remember that `text` describes a failure, then return it unchanged. */
export function markErrorMessage(text: string): string {
  if (!errorTexts.includes(text)) {
    errorTexts.push(text);
    if (errorTexts.length > MAX_ERROR_TEXTS) errorTexts.shift();
  }
  return text;
}

export function actionMessageTone(message: string | null | undefined): ActionMessageTone {
  if (!message) return "info";
  if (errorTexts.includes(message) || ERROR_PATTERNS.some((pattern) => pattern.test(message))) return "error";
  if (WARNING_PATTERNS.some((pattern) => pattern.test(message))) return "warning";
  // Progress text ("Preparing…", "Starting…") stays informational even when
  // it names a later outcome.
  if (/(\.\.\.|…)$/.test(message.trim())) return "info";
  if (SUCCESS_PATTERNS.some((pattern) => pattern.test(message))) return "success";
  return "info";
}

export const INFO_LIFETIME_MS = 8_000;
export const WARNING_LIFETIME_MS = 30_000;

/**
 * How long a message stays before closing itself (owner, 2026-11-07), or null
 * to keep it until dismissed. Errors stay so a failure is never missed;
 * progress text and messages offering a button stay until they change.
 */
export function actionMessageLifetimeMs(message: string | null | undefined, hasAction: boolean): number | null {
  if (!message || hasAction || /(\.\.\.|…)$/.test(message.trim())) return null;
  switch (actionMessageTone(message)) {
    case "error":
      return null;
    case "warning":
      return WARNING_LIFETIME_MS;
    case "success":
    case "info":
      return INFO_LIFETIME_MS;
  }
}
