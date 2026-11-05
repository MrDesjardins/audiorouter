import { describe, expect, it } from "vitest";
import type { DiscoveryDocument } from "@audiorouter/contracts";
import {
  pluginNodeParameterError,
  processorAvailabilityText,
  processorLatencyText,
  processorParameterError,
  processorParametersText,
  type ProcessorDescriptor,
} from "./processorCatalog";

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
    expect(
      processorParametersText({
        ...pitch,
        parameters: [{ name: "semitones", type: "number", unit: "st", minimum: -12, maximum: 12, default: 0 }],
      }),
    ).toBe("semitones: number st, -12..12");
    expect(processorParametersText(pitch)).toBe("no parameters");
  });

  // A free text setting (device ID, address, node reference) has no choices;
  // the backend validates it. Rejecting it blocked the device picker once
  // the catalog described endpointId, and blocked the Duck trigger node.
  it("accepts free text settings and still checks their length", () => {
    const nodeTypes: DiscoveryDocument["nodeTypes"] = [
      {
        type: "physicalInput@1",
        availability: { status: "available" },
        realtimeCostClass: "io",
        latencySamples: 0,
        parameters: [
          { name: "endpointId", type: "string", maxLength: 128, description: "Device ID from devices.list." },
          { name: "channelMode", type: "string", enum: ["stereo", "mono", "left", "right"] },
        ],
      },
    ];
    const duck: ProcessorDescriptor = {
      ...pitch,
      id: "duck",
      parameters: [{ name: "keyNodeId", type: "string", reference: "node" }],
    };
    expect(processorParameterError(null, "physicalInput", "endpointId", "{0.0.1.00000000}.{abc}", nodeTypes)).toBe(
      null,
    );
    expect(processorParameterError(null, "physicalInput", "endpointId", "x".repeat(129), nodeTypes)).toBe(
      "endpointId must be at most 128 characters",
    );
    expect(processorParameterError(null, "physicalInput", "channelMode", "surround", nodeTypes)).toBe(
      "channelMode must be one of the advertised values",
    );
    expect(processorParameterError([duck], "duck", "keyNodeId", "mic")).toBe(null);
  });

  it("validates and presents enumerated string parameters", () => {
    const descriptor = { ...pitch, parameters: [{ name: "filter", type: "string", enum: ["peaking", "notch"] }] };
    expect(processorParameterError([descriptor], "pitch", "filter", "notch")).toBeNull();
    expect(processorParameterError([descriptor], "pitch", "filter", "lowPass")).toContain("advertised");
    expect(processorParametersText(descriptor)).toContain("peaking/notch");
  });

  it("validates inspector values against the authoritative descriptor", () => {
    const descriptor = {
      ...pitch,
      parameters: [{ name: "semitones", type: "number", minimum: -12, maximum: 12, default: 0 }],
    };
    expect(processorParameterError([descriptor], "pitch", "semitones", 12.1)).toContain("at most 12");
    expect(processorParameterError([descriptor], "pitch", "semitones", 12)).toBeNull();
    expect(processorParameterError([descriptor], "pitch", "semitones", Number.NaN)).toContain("finite");
    expect(processorParameterError([descriptor], "pitch", "unknown", 1)).toBeNull();
  });

  it("uses the discovered node catalog for source parameters", () => {
    const nodeType: DiscoveryDocument["nodeTypes"][number] = {
      type: "testSignal@1",
      availability: { status: "available" },
      realtimeCostClass: "low",
      latencySamples: 0,
      parameters: [{ name: "frequencyHz", type: "number", unit: "Hz", minimum: 20, maximum: 20_000, default: 440 }],
    };
    expect(processorParameterError(null, "testSignal", "frequencyHz", 440, [nodeType])).toBeNull();
    expect(processorParameterError(null, "testSignal", "frequencyHz", 1, [nodeType])).toContain("at least 20");
  });

  it("enforces advertised integer steps without rejecting floating-point roundoff", () => {
    const descriptor = {
      ...pitch,
      parameters: [{ name: "harmonics", type: "number", minimum: 1, maximum: 8, step: 1 }],
    };
    expect(processorParameterError([descriptor], "pitch", "harmonics", 4)).toBeNull();
    expect(processorParameterError([descriptor], "pitch", "harmonics", 4.1)).toContain("steps of 1");
    const fractional = { ...pitch, parameters: [{ name: "mix", type: "number", minimum: 0, maximum: 1, step: 0.1 }] };
    expect(processorParameterError([fractional], "pitch", "mix", 0.1 + 0.2)).toBeNull();
  });
});

describe("pluginNodeParameterError", () => {
  const plugin = {
    parameters: [{ parameterId: 3, title: "Gain", defaultValue: 1, minimum: 0, maximum: 1 }],
  } as unknown as import("@audiorouter/contracts").PluginParametersResult;
  const catalog: DiscoveryDocument["nodeTypes"] = [
    {
      type: "plugin@1",
      parameters: [{ name: "passThroughAllowed", type: "boolean" }],
    } as unknown as DiscoveryDocument["nodeTypes"][number],
  ];
  const check = (name: string, value: boolean | number | string) =>
    pluginNodeParameterError(name, value, plugin, () => processorParameterError(null, "plugin", name, value, catalog));

  // Owner report 2026-10-31: ticking "Allow unchanged audio" said "Plugin
  // parameter value is outside the worker-provided range".
  it("checks AudioRouter's own plugin settings against the catalog, not the plugin's sliders", () => {
    expect(check("passThroughAllowed", true)).toBeNull();
    expect(check("passThroughAllowed", "yes")).toBe("passThroughAllowed must be boolean");
    expect(check("stateId", "state-1")).toBeNull();
  });

  it("keeps the plugin's own sliders inside the ranges its worker reported", () => {
    expect(check("pluginParameter:3", 0.5)).toBeNull();
    expect(check("pluginParameter:3", 1.5)).toBe("Plugin parameter value is outside the worker-provided range");
    expect(check("pluginParameter:9", 0.5)).toBe("Plugin parameter value is outside the worker-provided range");
  });
});
