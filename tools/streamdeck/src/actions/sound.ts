import { action, type KeyDownEvent, type KeyUpEvent } from "@elgato/streamdeck";
import { idempotencyKey } from "../api.js";
import { fit, messageFace, soundFace } from "../faces.js";
import type { SummaryNode } from "../store.js";
import { LiveKeyAction, type Face, type KeyTitle } from "./live-key.js";

export type SoundSettings = {
  /** The Soundboard tool, by ID, with its name to find it again if it is re-created. */
  nodeId?: string;
  nodeName?: string;
  /** 1 to 9: the button on the Soundboard. */
  slot?: number;
};

/** The slot number, or null when the key has none chosen. */
export function soundSlot(settings: SoundSettings): number | null {
  const slot = Number(settings.slot);
  return Number.isInteger(slot) && slot >= 1 && slot <= 9 ? slot : null;
}

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
    return soundFace({ label: this.label(title, this.caption(node, slot)), slot, pending: this.pending.has(actionId) });
  }

  /** What the button shows in AudioRouter: its caption, else the file name. */
  private caption(node: SummaryNode, slot: number): string {
    const caption = this.store.setting(node, `slot${slot}Caption`);
    if (typeof caption === "string" && caption.trim()) return caption.trim();
    const file = this.store.setting(node, `slot${slot}FileName`);
    return typeof file === "string" ? file.replace(/\.[A-Za-z0-9]{1,5}$/, "") : `Sound ${slot}`;
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
    await this.run(ev.action, () => this.store.call("soundboard.trigger", {
      sessionId,
      nodeId: node.id,
      slot,
      action: press ? "press" : "release",
    }));
  }

  override async onKeyDown(ev: KeyDownEvent<SoundSettings>) {
    await this.send(ev, true);
  }

  /** A release only matters for "play while held" slots; the backend ignores it for the others. */
  override async onKeyUp(ev: KeyUpEvent<SoundSettings>) {
    const node = this.resolve(ev.payload.settings);
    const slot = soundSlot(ev.payload.settings);
    if (!node || slot === null || this.store.setting(node, `slot${slot}Mode`) !== "hold") return;
    await this.send(ev, false);
  }
}
