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
