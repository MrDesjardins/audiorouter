import { action, type KeyDownEvent, type KeyUpEvent } from "@elgato/streamdeck";
import { idempotencyKey } from "../api.js";
import { fit, messageFace, targetLabel, toggleFace } from "../faces.js";
import type { SummaryNode } from "../store.js";
import { LiveKeyAction, type Face, type KeyTitle } from "./live-key.js";

export type ToggleSettings = {
  /** The tool, by ID, with its name to find it again if it is re-created. */
  nodeId?: string;
  nodeName?: string;
  /** "enabled", "bypass", or an on/off or two-choice setting. */
  target?: string;
  /** latch: each press flips; momentary: on while held, back on release. */
  mode?: "latch" | "momentary";
  /** Look: colours from the palette, words for each state, the second line. */
  onColor?: string;
  offColor?: string;
  onText?: string;
  offText?: string;
  showDetail?: boolean;
};

/**
 * Toggle any tool's Enabled, Bypass, or on/off / two-choice setting in the
 * selected session. The key shows the backend's value, not a local guess.
 */
@action({ UUID: "com.mrdesjardins.audiorouter.toggle" })
export class ToggleAction extends LiveKeyAction<ToggleSettings> {
  /** Momentary keys: the value to restore on release. */
  private readonly held = new Map<string, unknown>();

  protected face(settings: ToggleSettings, actionId: string, title: KeyTitle): Face {
    const node = this.resolve(settings);
    if (!node) return messageFace(fit(settings.nodeName || "Choose"), settings.nodeName ? "not found" : "a tool");
    const target = settings.target || "enabled";
    const value = this.store.setting(node, target);
    const choices = this.store.spec(node, target)?.enum;
    const twoChoice = Array.isArray(choices) && choices.length === 2;
    const on = twoChoice ? value === choices[1] : value === true;
    const svg = toggleFace({
      label: this.label(title, node.name),
      detail: settings.showDetail === false ? null : targetLabel(target),
      target,
      on,
      pending: this.pending.has(actionId),
      valueText: twoChoice ? String(value ?? "").toUpperCase() : undefined,
      onColor: settings.onColor,
      offColor: settings.offColor,
      onText: settings.onText,
      offText: settings.offText,
    });
    return { svg, state: on ? 1 : 0 };
  }

  private resolve(settings: ToggleSettings): SummaryNode | undefined {
    return settings.nodeId || settings.nodeName ? this.store.node(settings.nodeId ?? "", settings.nodeName ?? "") : undefined;
  }

  override async onKeyDown(ev: KeyDownEvent<ToggleSettings>) {
    const settings = ev.payload.settings;
    const node = this.resolve(settings);
    if (!node || this.store.state !== "online") {
      await ev.action.showAlert();
      return;
    }
    const target = settings.target || "enabled";
    if (settings.mode === "momentary") {
      const original = this.store.setting(node, target);
      this.held.set(ev.action.id, original);
      await this.run(ev.action, () => this.setValue(node.id, target, this.opposite(node, target, original)));
      return;
    }
    await this.run(ev.action, () => this.store.call("nodes.toggle", { node: node.id, target, idempotencyKey: idempotencyKey("toggle") }));
  }

  override async onKeyUp(ev: KeyUpEvent<ToggleSettings>) {
    if (!this.held.has(ev.action.id)) return;
    const original = this.held.get(ev.action.id);
    this.held.delete(ev.action.id);
    const node = this.resolve(ev.payload.settings);
    if (!node) return;
    await this.run(ev.action, () => this.setValue(node.id, ev.payload.settings.target || "enabled", original));
  }

  /** The other value of a switch (boolean or two-choice setting). */
  private opposite(node: SummaryNode, target: string, value: unknown): unknown {
    const choices = this.store.spec(node, target)?.enum;
    if (Array.isArray(choices) && choices.length === 2) return value === choices[0] ? choices[1] : choices[0];
    return !(value === true);
  }

  /** Set an explicit value (momentary keys never rely on flipping twice). */
  private setValue(nodeId: string, target: string, value: unknown) {
    const change = target === "enabled" ? { enabled: value } : target === "bypass" ? { bypass: value } : { parameters: { [target]: value } };
    return this.store.call("nodes.set", { node: nodeId, ...change, idempotencyKey: idempotencyKey("set") });
  }
}
