import { action, type KeyDownEvent } from "@elgato/streamdeck";
import { idempotencyKey } from "../api.js";
import { privacyFace } from "../faces.js";
import { LiveKeyAction } from "./live-key.js";

export type PrivacySettings = {
  /** toggle: press mutes or unmutes; mute / unmute: the key only does that. */
  mode?: "toggle" | "mute" | "unmute";
};

/** AudioRouter's privacy mute: the global microphone kill switch. */
@action({ UUID: "com.mrdesjardins.audiorouter.privacy" })
export class PrivacyAction extends LiveKeyAction<PrivacySettings> {
  protected face(settings: PrivacySettings, actionId: string): string {
    return privacyFace({ muted: this.store.summary?.privacyMuted ?? true, mode: settings.mode ?? "toggle", pending: this.pending.has(actionId) });
  }

  override async onKeyDown(ev: KeyDownEvent<PrivacySettings>) {
    if (this.store.state !== "online") {
      await ev.action.showAlert();
      return;
    }
    const mode = ev.payload.settings.mode ?? "toggle";
    await this.run(ev.action, () => mode === "toggle"
      ? this.store.call("safety.togglePrivacyMute", { idempotencyKey: idempotencyKey("privacy") })
      : this.store.call("safety.setPrivacyMute", { muted: mode === "mute", idempotencyKey: idempotencyKey("privacy") }));
  }
}
