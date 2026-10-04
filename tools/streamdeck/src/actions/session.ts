import { action, type KeyDownEvent } from "@elgato/streamdeck";
import { idempotencyKey } from "../api.js";
import { messageFace, sessionFace } from "../faces.js";
import type { AudioRouterStore } from "../store.js";
import { LiveKeyAction, type Face, type KeyTitle } from "./live-key.js";

export type SessionSettings = {
  /** select: this key selects one session; cycle: each press selects the next. */
  mode?: "select" | "cycle";
  sessionId?: string;
  sessionName?: string;
  /** Also start playing the session after selecting it. */
  play?: boolean;
};

/**
 * Select a session (what every "selected session" key then follows), or
 * step through all sessions. Optionally starts it playing.
 */
@action({ UUID: "com.mrdesjardins.audiorouter.session" })
export class SessionAction extends LiveKeyAction<SessionSettings> {
  constructor(store: AudioRouterStore) {
    super(store, { sessions: true });
  }

  protected face(settings: SessionSettings, actionId: string, title: KeyTitle): Face {
    const summary = this.store.summary;
    const cycle = settings.mode === "cycle";
    const target = cycle ? { id: summary?.sessionId ?? "", name: summary?.name ?? "" } : this.store.sessions.find((item) => item.id === settings.sessionId) ?? (settings.sessionId ? { id: settings.sessionId, name: settings.sessionName ?? settings.sessionId } : null);
    if (!target) return messageFace("Choose", "a session");
    const selected = target.id === summary?.sessionId;
    const name = this.label(title, target.name) ?? "";
    return { svg: sessionFace({ name, selected, playing: selected && (summary?.playing ?? false), cycle, pending: this.pending.has(actionId) }), state: selected ? 1 : 0 };
  }

  override async onKeyDown(ev: KeyDownEvent<SessionSettings>) {
    const settings = ev.payload.settings;
    const sessions = this.store.sessions;
    const current = this.store.summary?.sessionId;
    let next: string | undefined;
    if (settings.mode === "cycle") {
      const index = sessions.findIndex((item) => item.id === current);
      next = sessions.length > 0 ? sessions[(index + 1) % sessions.length].id : undefined;
    } else {
      next = settings.sessionId;
    }
    if (!next || this.store.state !== "online") {
      await ev.action.showAlert();
      return;
    }
    const sessionId = next;
    await this.run(ev.action, async () => {
      if (sessionId !== current) await this.store.call("sessions.active.set", { sessionId, idempotencyKey: idempotencyKey("session") });
      if (settings.play) await this.store.call("sessions.play", { sessionId, idempotencyKey: idempotencyKey("play") });
    });
  }
}
