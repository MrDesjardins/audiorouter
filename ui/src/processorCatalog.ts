import type { DiscoveryDocument, PluginParametersResult } from "@audiorouter/contracts";
import type { MessageKey } from "./i18n/generated/messageKeys";

export type ProcessorDescriptor = DiscoveryDocument["processors"][number];

/**
 * A plugin node's own sliders (`pluginParameter:<id>`) are checked against the
 * ranges its worker reported. Every other setting on the node is AudioRouter's
 * (`stateId`, `passThroughAllowed`) and follows the catalog like any tool.
 */
export function pluginNodeParameterError(
  name: string,
  value: boolean | number | string,
  pluginParameters: PluginParametersResult | null,
  catalogError: () => string | null,
): string | null {
  if (!name.startsWith("pluginParameter:")) return catalogError();
  const id = Number(name.slice("pluginParameter:".length));
  const descriptor = pluginParameters?.parameters.find((parameter) => parameter.parameterId === id);
  return !descriptor ||
    typeof value !== "number" ||
    !Number.isFinite(value) ||
    value < descriptor.minimum ||
    value > descriptor.maximum
    ? "Plugin parameter value is outside the worker-provided range"
    : null;
}

export function processorParameterError(
  processors: ProcessorDescriptor[] | null,
  nodeKind: string,
  name: string,
  value: boolean | number | string,
  nodeTypes: DiscoveryDocument["nodeTypes"] | null = null,
): string | null {
  const wireKind = nodeKind.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
  const parameter =
    processors
      ?.find((processor) => processor.id === nodeKind)
      ?.parameters.find((candidate) => candidate.name === name) ??
    nodeTypes
      ?.find((node) => node.type === `${wireKind}@1` || node.type === `${nodeKind}@1`)
      ?.parameters.find((candidate) => candidate.name === name);
  if (!parameter) return null;
  if (parameter.type === "number") {
    if (typeof value !== "number" || !Number.isFinite(value)) return `${name} must be finite`;
    if (parameter.minimum !== undefined && value < parameter.minimum)
      return `${name} must be at least ${parameter.minimum}`;
    if (parameter.maximum !== undefined && value > parameter.maximum)
      return `${name} must be at most ${parameter.maximum}`;
    if (parameter.step !== undefined && parameter.step > 0 && Number.isFinite(parameter.step)) {
      const steps = (value - (parameter.minimum ?? 0)) / parameter.step;
      if (Math.abs(steps - Math.round(steps)) > 1e-7) return `${name} must use steps of ${parameter.step}`;
    }
  } else if (parameter.type === "boolean" && typeof value !== "boolean") {
    return `${name} must be boolean`;
  } else if (parameter.type === "string") {
    if (typeof value !== "string") return `${name} must be text`;
    // Free text (a device ID, an address, a node reference) has no choices;
    // the backend validates its content.
    if (parameter.enum && !parameter.enum.includes(value)) return `${name} must be one of the advertised values`;
    if (parameter.maxLength !== undefined && value.length > parameter.maxLength)
      return `${name} must be at most ${parameter.maxLength} characters`;
  }
  return null;
}

/** Component translator (`useMessages()`); without one the English text is returned. */
export type ProcessorTranslator = (key: MessageKey, options?: Record<string, unknown>) => string;

export function processorAvailabilityText(processor: ProcessorDescriptor, tr?: ProcessorTranslator): string {
  if (processor.availability.status === "available") return tr ? tr("g3Adv.procAvailable") : "available";
  const reason = processor.availability.reason ?? (tr ? tr("g3Adv.procNoCapability") : "capability unavailable");
  return tr ? tr("g3Adv.procUnavailableReason", { reason }) : `unavailable: ${reason}`;
}

export function processorLatencyText(processor: ProcessorDescriptor, tr?: ProcessorTranslator): string {
  if (processor.latencySamples === 0) return tr ? tr("g3Adv.procNoLatency") : "no declared latency";
  return tr ? tr("g3Adv.procLatency", { n: processor.latencySamples }) : `${processor.latencySamples} samples latency`;
}

export function processorParametersText(processor: ProcessorDescriptor, tr?: ProcessorTranslator): string {
  if (processor.parameters.length === 0) return tr ? tr("g3Adv.procNoParams") : "no parameters";
  return processor.parameters
    .map((parameter) => {
      const range =
        parameter.minimum !== undefined && parameter.maximum !== undefined
          ? `, ${parameter.minimum}..${parameter.maximum}`
          : "";
      const unit = parameter.unit ? ` ${parameter.unit}` : "";
      const choices = parameter.enum ? ` (${parameter.enum.join("/")})` : "";
      return `${parameter.name}: ${parameter.type}${unit}${range}${choices}`;
    })
    .join("; ");
}
