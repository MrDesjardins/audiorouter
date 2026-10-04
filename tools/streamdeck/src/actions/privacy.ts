import { action, type KeyDownEvent } from "@elgato/streamdeck";
import { idempotencyKey } from "../api.js";
import { privacyFace } from "../faces.js";
import { LiveKeyAction, type Face } from "./live-key.js";

export type PrivacySettings = {
  /** toggle: press mutes or unmutes; mute / unmute: the key only does that. */
  mode?: "toggle" | "mute" | "unmute";
  /** Words for each state and whether to show the action line. */
  mutedText?: string;
  liveText?: string;
  showDetail?: boolean;
};

/** AudioRouter's privacy mute: the global microphone kill switch. */
@action({ UUID: "com.mrdesjardins.audiorouter.privacy" })
export class PrivacyAction extends LiveKeyAction<PrivacySettings> {
  protected face(settings: PrivacySettings, actionId: string): Face {
    const muted = this.store.summary?.privacyMuted ?? true;
    const svg = privacyFace({ muted, mode: settings.mode ?? "toggle", pending: this.pending.has(actionId), mutedText: settings.mutedText, liveText: settings.liveText, showDetail: settings.showDetail });
    // State 0 = live, 1 = muted: users can pick their own image for each.
    return { svg, state: muted ? 1 : 0 };
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
