import { describe, expect, it } from "vitest";
import { processorAvailabilityText, processorLatencyText, processorParameterError, processorParametersText, type ProcessorDescriptor } from "./processorCatalog";

const pitch: ProcessorDescriptor = {
  id: "pitch",
  version: 1,
  category: "pitch",
  availability: { status: "unavailable", reason: "synthetic test fixture" },
  latencySamples: 1024,
  parameters: [],
};

describe("processor catalog presentation", () => {
  it("preserves explicit unavailable reasons and latency", () => {
    expect(processorAvailabilityText(pitch)).toBe("unavailable: synthetic test fixture");
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

  it("validates and presents enumerated string parameters", () => {
    const descriptor = { ...pitch, parameters: [{ name: "filter", type: "string", enum: ["peaking", "notch"] }] };
    expect(processorParameterError([descriptor], "pitch", "filter", "notch")).toBeNull();
    expect(processorParameterError([descriptor], "pitch", "filter", "lowPass")).toContain("advertised");
    expect(processorParametersText(descriptor)).toContain("peaking/notch");
  });

  it("validates inspector values against the authoritative descriptor", () => {
    const descriptor = { ...pitch, parameters: [{ name: "semitones", type: "number", minimum: -12, maximum: 12, default: 0 }] };
    expect(processorParameterError([descriptor], "pitch", "semitones", 12.1)).toContain("at most 12");
    expect(processorParameterError([descriptor], "pitch", "semitones", 12)).toBeNull();
    expect(processorParameterError([descriptor], "pitch", "semitones", Number.NaN)).toContain("finite");
    expect(processorParameterError([descriptor], "pitch", "unknown", 1)).toBeNull();
  });
});
