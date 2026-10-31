import { action, type KeyDownEvent, type KeyUpEvent } from "@elgato/streamdeck";
import { fit, messageFace, soundFace } from "../faces.js";
import type { SummaryNode } from "../store.js";
import { keyName, releasesOnKeyUp, soundBehavior, soundSlot, triggerParams, type SoundSettings } from "../sound-settings.js";
import { LiveKeyAction, type Face, type KeyTitle } from "./live-key.js";

/** Play one sound of a Soundboard tool, like pressing its button. */
@action({ UUID: "com.mrdesjardins.audiorouter.sound" })
export class SoundAction extends LiveKeyAction<SoundSettings> {
  protected face(settings: SoundSettings, actionId: string, title: KeyTitle): Face {
    const node = this.resolve(settings);
    if (!node) return messageFace(fit(settings.nodeName || "Choose"), settings.nodeName ? "not found" : "a Soundboard");
    const slot = soundSlot(settings);
    if (slot === null) return messageFace(fit(node.name), "choose a sound");
    const mediaId = this.store.setting(node, `slot${slot}MediaId`);
    if (typeof mediaId !== "string" || mediaId === "") return messageFace(fit(node.name), `slot ${slot} is empty`);
    const name = keyName(title.text, settings, (field) => this.store.setting(node, field), slot);
    return soundFace({
      label: name === null ? null : this.label(title, name),
      named: name !== null,
      slot,
      pending: this.pending.has(actionId),
      behavior: soundBehavior(settings),
    });
  }

  private resolve(settings: SoundSettings): SummaryNode | undefined {
    return settings.nodeId || settings.nodeName ? this.store.node(settings.nodeId ?? "", settings.nodeName ?? "") : undefined;
  }

  private async send(ev: KeyDownEvent<SoundSettings> | KeyUpEvent<SoundSettings>, press: boolean) {
    const sessionId = this.store.summary?.sessionId;
    const node = this.resolve(ev.payload.settings);
    const slot = soundSlot(ev.payload.settings);
    if (!sessionId || !node || slot === null || this.store.state !== "online") {
      if (press) await ev.action.showAlert();
      return;
    }
    await this.run(ev.action, () =>
      this.store.call("soundboard.trigger", triggerParams(sessionId, node.id, slot, ev.payload.settings, press)),
    );
  }

  override async onKeyDown(ev: KeyDownEvent<SoundSettings>) {
    await this.send(ev, true);
  }

  /** A release only matters for "play while held"; the backend ignores it for the other modes. */
  override async onKeyUp(ev: KeyUpEvent<SoundSettings>) {
    const node = this.resolve(ev.payload.settings);
    const slot = soundSlot(ev.payload.settings);
    if (!node || slot === null) return;
    if (!releasesOnKeyUp(ev.payload.settings, this.store.setting(node, `slot${slot}Mode`))) return;
    await this.send(ev, false);
  }
}
