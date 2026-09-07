import type { DiscoveryDocument } from "@audiorouter/contracts";

export type ProcessorDescriptor = DiscoveryDocument["processors"][number];

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
