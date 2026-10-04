import { action } from "@elgato/streamdeck";
import { fit, messageFace, meterFace } from "../faces.js";
import type { AudioRouterStore } from "../store.js";
import { LiveKeyAction } from "./live-key.js";

export type MeterSettings = { nodeId?: string; nodeName?: string };

/** A live level meter for one tool of the selected session. */
@action({ UUID: "com.mrdesjardins.audiorouter.meter" })
export class MeterAction extends LiveKeyAction<MeterSettings> {
  constructor(store: AudioRouterStore) {
    super(store, true);
  }

  protected face(settings: MeterSettings): string {
    const node = settings.nodeId || settings.nodeName ? this.store.node(settings.nodeId ?? "", settings.nodeName ?? "") : undefined;
    if (!node) return messageFace(fit(settings.nodeName || "Choose"), settings.nodeName ? "not found" : "a tool");
    const level = this.store.levels.get(node.id);
    return meterFace({
      label: node.name,
      peakDb: level?.peakDb ?? null,
      rmsDb: level?.rmsDb ?? null,
      clipped: level?.clipped ?? false,
      muted: !node.enabled,
      playing: this.store.summary?.playing ?? false,
    });
  }
}
