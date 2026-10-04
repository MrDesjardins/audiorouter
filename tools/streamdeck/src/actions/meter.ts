import { action, type KeyDownEvent } from "@elgato/streamdeck";
import { idempotencyKey } from "../api.js";
import { fit, messageFace, meterFace, type MeterScheme } from "../faces.js";
import type { AudioRouterStore } from "../store.js";
import { LiveKeyAction, type Face, type KeyTitle } from "./live-key.js";

export type MeterSettings = {
  nodeId?: string;
  nodeName?: string;
  /** Show the dB number beside the bar. */
  showValue?: boolean;
  /** canvas: AudioRouter's colours; classic: green, yellow, red. */
  scheme?: MeterScheme;
  /** What a press does: nothing, switch the tool off/on, or bypass it. */
  press?: "none" | "enabled" | "bypass";
};

/** A live level meter for one tool of the selected session. */
@action({ UUID: "com.mrdesjardins.audiorouter.meter" })
export class MeterAction extends LiveKeyAction<MeterSettings> {
  constructor(store: AudioRouterStore) {
    super(store, { levels: true });
  }

  protected face(settings: MeterSettings, _actionId: string, title: KeyTitle): Face {
    const node = settings.nodeId || settings.nodeName ? this.store.node(settings.nodeId ?? "", settings.nodeName ?? "") : undefined;
    if (!node) return messageFace(fit(settings.nodeName || "Choose"), settings.nodeName ? "not found" : "a tool");
    const level = this.store.levels.get(node.id);
    return meterFace({
      label: this.label(title, node.name),
      peakDb: level?.peakDb ?? null,
      rmsDb: level?.rmsDb ?? null,
      clipped: level?.clipped ?? false,
      muted: !node.enabled || (settings.press === "bypass" && node.bypass),
      playing: this.store.summary?.playing ?? false,
      showValue: settings.showValue ?? true,
      scheme: settings.scheme ?? "canvas",
    });
  }

  override async onKeyDown(ev: KeyDownEvent<MeterSettings>) {
    const press = ev.payload.settings.press ?? "none";
    if (press === "none") return;
    const node = this.store.node(ev.payload.settings.nodeId ?? "", ev.payload.settings.nodeName ?? "");
    if (!node || this.store.state !== "online") {
      await ev.action.showAlert();
      return;
    }
    await this.run(ev.action, () => this.store.call("nodes.toggle", { node: node.id, target: press, idempotencyKey: idempotencyKey("meter") }));
  }
}
