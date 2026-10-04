import { idempotencyKey } from "./api.js";

type Call = (method: string, params?: Record<string, unknown>) => Promise<unknown>;

/**
 * Select `to` from a Session key so that audio follows the selection.
 * Selecting alone never stops audio, and only one session's audio can be
 * prepared at a time: switching while another session played left it
 * running, and Play on the new one was refused ("Audio is already
 * prepared"). So: stop whatever plays, select, and play the new session
 * when audio was playing before or when the key asks to.
 */
export async function switchSession(call: Call, to: string, current: string | undefined, playAfter: boolean): Promise<void> {
  const status = await call("status.get") as { activeSessionIds?: string[] };
  const playing = status.activeSessionIds ?? [];
  if (to === current && playing.includes(to)) return;
  for (const sessionId of playing.filter((id) => id !== to)) {
    await call("session.stop", { sessionId, idempotencyKey: idempotencyKey("switch-stop") });
  }
  if (to !== current) await call("sessions.active.set", { sessionId: to, idempotencyKey: idempotencyKey("session") });
  if ((playAfter || playing.length > 0) && !playing.includes(to)) {
    await call("sessions.play", { sessionId: to, idempotencyKey: idempotencyKey("play") });
  }
}
