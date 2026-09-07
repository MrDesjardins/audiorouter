import { describe, expect, it } from "vitest";
import { processorAvailabilityText, processorLatencyText, processorParametersText, type ProcessorDescriptor } from "./processorCatalog";

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

  it("shows typed parameter ranges without inventing empty parameters", () => {
    expect(processorParametersText({
      ...pitch,
      parameters: [{ name: "semitones", type: "number", unit: "st", minimum: -12, maximum: 12, default: 0 }],
    })).toBe("semitones: number st, -12..12");
    expect(processorParametersText(pitch)).toBe("no parameters");
  });
});
