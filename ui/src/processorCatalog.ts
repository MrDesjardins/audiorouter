import type { DiscoveryDocument, PluginParametersResult } from "@audiorouter/contracts";

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

export function processorAvailabilityText(processor: ProcessorDescriptor): string {
  return processor.availability.status === "available"
    ? "available"
    : `unavailable: ${processor.availability.reason ?? "capability unavailable"}`;
}

export function processorLatencyText(processor: ProcessorDescriptor): string {
  return processor.latencySamples === 0 ? "no declared latency" : `${processor.latencySamples} samples latency`;
}

export function processorParametersText(processor: ProcessorDescriptor): string {
  if (processor.parameters.length === 0) return "no parameters";
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
