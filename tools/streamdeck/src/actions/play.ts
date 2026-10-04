import { action, type KeyDownEvent } from "@elgato/streamdeck";
import { idempotencyKey } from "../api.js";
import { playFace } from "../faces.js";
import { LiveKeyAction, type Face, type KeyTitle } from "./live-key.js";

export type PlaySettings = {
  /** toggle: Play or Stop; play / stop: the key only does that. */
  press?: "toggle" | "play" | "stop";
};

/** Play or stop the selected session. */
@action({ UUID: "com.mrdesjardins.audiorouter.play" })
export class PlayAction extends LiveKeyAction<PlaySettings> {
  protected face(_settings: PlaySettings, actionId: string, title: KeyTitle): Face {
    const playing = this.store.summary?.playing ?? false;
    // State 0 = stopped, 1 = playing: users can pick their own image for each.
    return { svg: playFace({ label: this.label(title, this.store.summary?.name ?? ""), playing, pending: this.pending.has(actionId) }), state: playing ? 1 : 0 };
  }

  override async onKeyDown(ev: KeyDownEvent<PlaySettings>) {
    const sessionId = this.store.summary?.sessionId;
    if (!sessionId || this.store.state !== "online") {
      await ev.action.showAlert();
      return;
    }
    const press = ev.payload.settings.press ?? "toggle";
    const playing = this.store.summary?.playing ?? false;
    if ((press === "play" && playing) || (press === "stop" && !playing)) return;
    await this.run(ev.action, () => press === "stop" || (press === "toggle" && playing)
      ? this.store.call("session.stop", { sessionId, idempotencyKey: idempotencyKey("stop") })
      : this.store.call("sessions.play", { sessionId, idempotencyKey: idempotencyKey("play") }));
  }
}
