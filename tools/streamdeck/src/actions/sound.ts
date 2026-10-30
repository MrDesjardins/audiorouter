import { action, type KeyDownEvent, type KeyUpEvent } from "@elgato/streamdeck";
import { fit, messageFace, soundFace } from "../faces.js";
import type { SummaryNode } from "../store.js";
import { LiveKeyAction, type Face, type KeyTitle } from "./live-key.js";

export type SoundSettings = {
  /** The Soundboard tool, by ID, with its name to find it again if it is re-created. */
  nodeId?: string;
  nodeName?: string;
  /** 1 to 9: the button on the Soundboard. */
  slot?: number;
  /** How a press plays: the slot's own setting, or once, loop until pressed again, or while held. */
  behavior?: SoundBehavior;
  /** A name for the key; empty shows the sound's caption from AudioRouter. */
  label?: string;
};

export type SoundBehavior = "slot" | "once" | "loop" | "hold";

const BEHAVIORS: readonly SoundBehavior[] = ["slot", "once", "loop", "hold"];

/** The chosen behavior, "slot" when none or an unknown one is saved. */
export function soundBehavior(settings: SoundSettings): SoundBehavior {
  return BEHAVIORS.find((behavior) => behavior === settings.behavior) ?? "slot";
}

/** What the button shows in AudioRouter: its caption, else the file name. */
export function slotCaption(read: (name: string) => unknown, slot: number): string {
  const caption = read(`slot${slot}Caption`);
  if (typeof caption === "string" && caption.trim()) return caption.trim();
  const file = read(`slot${slot}FileName`);
  return typeof file === "string" && file.trim() ? file.replace(/\.[A-Za-z0-9]{1,5}$/, "") : `Sound ${slot}`;
}

/** The filled slots of a Soundboard, for the settings panel. */
export function soundSlotChoices(read: (name: string) => unknown): { slot: number; label: string; filled: boolean }[] {
  return Array.from({ length: 9 }, (_, index) => {
    const slot = index + 1;
    const media = read(`slot${slot}MediaId`);
    const filled = typeof media === "string" && media !== "";
    return { slot, filled, label: filled ? slotCaption(read, slot) : "(empty)" };
  });
}

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
    const name = settings.label?.trim() || slotCaption((field) => this.store.setting(node, field), slot);
    return soundFace({
      label: this.label(title, name),
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
    const behavior = soundBehavior(ev.payload.settings);
    await this.run(ev.action, () => this.store.call("soundboard.trigger", {
      sessionId,
      nodeId: node.id,
      slot,
      action: press ? "press" : "release",
      ...(behavior === "slot" ? {} : { mode: behavior }),
    }));
  }

  override async onKeyDown(ev: KeyDownEvent<SoundSettings>) {
    await this.send(ev, true);
  }

  /** A release only matters for "play while held"; the backend ignores it for the other modes. */
  override async onKeyUp(ev: KeyUpEvent<SoundSettings>) {
    const node = this.resolve(ev.payload.settings);
    const slot = soundSlot(ev.payload.settings);
    if (!node || slot === null) return;
    const behavior = soundBehavior(ev.payload.settings);
    const mode = behavior === "slot" ? this.store.setting(node, `slot${slot}Mode`) : behavior;
    if (mode !== "hold") return;
    await this.send(ev, false);
  }
}
