import { action, type KeyDownEvent } from "@elgato/streamdeck";
import { idempotencyKey } from "../api.js";
import { fit, messageFace, stepFace, targetLabel } from "../faces.js";
import type { SummaryNode } from "../store.js";
import { formatValue, stepValue } from "../values.js";
import { LiveKeyAction, type Face, type KeyTitle } from "./live-key.js";

export type StepSettings = {
  nodeId?: string;
  nodeName?: string;
  /** A numeric setting, for example gainDb, percent or a Mixer input level. */
  parameter?: string;
  /** Added on each press; negative to go down. */
  step?: number;
  /** set: jump to exactly `step` instead of adding it (a preset value). */
  mode?: "add" | "set";
  showDetail?: boolean;
};

/** Nudge a numeric setting up or down (or set it), showing its value. */
@action({ UUID: "com.mrdesjardins.audiorouter.step" })
export class StepAction extends LiveKeyAction<StepSettings> {
  protected face(settings: StepSettings, actionId: string, title: KeyTitle): Face {
    const node = this.resolve(settings);
    if (!node) return messageFace(fit(settings.nodeName || "Choose"), settings.nodeName ? "not found" : "a tool");
    if (!settings.parameter) return messageFace(fit(node.name), "choose a setting");
    const spec = this.store.spec(node, settings.parameter);
    const current = Number(this.store.setting(node, settings.parameter) ?? 0);
    const step = Number(settings.step ?? 1);
    const mode = settings.mode ?? "add";
    const next = stepValue(current, step, mode, spec);
    const stepText = mode === "set" ? `SET ${formatValue(step, spec?.unit)}` : `${step > 0 ? "+" : ""}${formatValue(step, spec?.unit)}`;
    return stepFace({
      label: this.label(title, node.name),
      detail: settings.showDetail === false ? null : targetLabel(settings.parameter),
      valueText: formatValue(current, spec?.unit),
      step: stepText,
      pending: this.pending.has(actionId),
      atLimit: next === current,
    });
  }

  private resolve(settings: StepSettings): SummaryNode | undefined {
    return settings.nodeId || settings.nodeName ? this.store.node(settings.nodeId ?? "", settings.nodeName ?? "") : undefined;
  }

  override async onKeyDown(ev: KeyDownEvent<StepSettings>) {
    const settings = ev.payload.settings;
    const node = this.resolve(settings);
    if (!node || !settings.parameter || this.store.state !== "online") {
      await ev.action.showAlert();
      return;
    }
    const parameter = settings.parameter;
    const next = stepValue(Number(this.store.setting(node, parameter) ?? 0), Number(settings.step ?? 1), settings.mode ?? "add", this.store.spec(node, parameter));
    await this.run(ev.action, () => this.store.call("nodes.set", { node: node.id, parameters: { [parameter]: next }, idempotencyKey: idempotencyKey("step") }));
  }
}
