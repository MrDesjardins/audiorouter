/**
 * Classifies a global action message so failures are visually distinct from
 * progress and success text. Messages are produced by many UI paths as plain
 * strings, so the classification relies on their stable failure phrasing.
 */
export type ActionMessageTone = "error" | "info";

const ERROR_PATTERNS = [
  /^no audio (started|played)/i,
  /^draft rejected/i,
  /^(could not|unable to|cannot)\b/i,
  /\b(permission denied|not permitted|access denied)\b/i,
  /\b(failed|rejected|unavailable|cannot play|cannot prepare|is not attached|not changed)\b/i,
];

export function actionMessageTone(message: string | null | undefined): ActionMessageTone {
  if (!message) return "info";
  return ERROR_PATTERNS.some((pattern) => pattern.test(message)) ? "error" : "info";
}
