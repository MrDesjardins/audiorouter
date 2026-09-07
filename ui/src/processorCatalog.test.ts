import { describe, expect, it } from "vitest";
import { processorAvailabilityText, processorLatencyText, type ProcessorDescriptor } from "./processorCatalog";

const pitch: ProcessorDescriptor = {
  id: "pitch",
  version: 1,
  category: "pitch",
  availability: { status: "unavailable", reason: "requires M04 graph integration" },
  latencySamples: 1024,
  parameters: [],
};

describe("processor catalog presentation", () => {
  it("preserves explicit unavailable reasons and latency", () => {
    expect(processorAvailabilityText(pitch)).toBe("unavailable: requires M04 graph integration");
    expect(processorLatencyText(pitch)).toBe("1024 samples latency");
  });

  it("does not invent latency for zero-latency processors", () => {
    expect(processorLatencyText({ ...pitch, latencySamples: 0 })).toBe("no declared latency");
  });
});
