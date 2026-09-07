import type { DiscoveryDocument } from "@audiorouter/contracts";

export type ProcessorDescriptor = DiscoveryDocument["processors"][number];

export function processorParameterError(
  processors: ProcessorDescriptor[] | null,
  nodeKind: string,
  name: string,
  value: boolean | number,
): string | null {
  const parameter = processors?.find((processor) => processor.id === nodeKind)?.parameters
    .find((candidate) => candidate.name === name);
  if (!parameter) return null;
  if (parameter.type === "number") {
    if (typeof value !== "number" || !Number.isFinite(value)) return `${name} must be finite`;
    if (parameter.minimum !== undefined && value < parameter.minimum) return `${name} must be at least ${parameter.minimum}`;
    if (parameter.maximum !== undefined && value > parameter.maximum) return `${name} must be at most ${parameter.maximum}`;
  } else if (parameter.type === "boolean" && typeof value !== "boolean") {
    return `${name} must be boolean`;
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
  return processor.parameters.map((parameter) => {
    const range = parameter.minimum !== undefined && parameter.maximum !== undefined
      ? `, ${parameter.minimum}..${parameter.maximum}`
      : "";
    const unit = parameter.unit ? ` ${parameter.unit}` : "";
    return `${parameter.name}: ${parameter.type}${unit}${range}`;
  }).join("; ");
}
