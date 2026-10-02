import { describe, expect, it } from "vitest";
import { filterLibraryEntries, libraryEntries, libraryEntryAccessibleLabel } from "./library";

describe("node library search", () => {
  it("matches labels, categories, and availability reasons", () => {
    expect(filterLibraryEntries(libraryEntries, "effect").map((entry) => entry.id)).toEqual(["volume", "gain", "bass-treble", "fir-filter", "time-shift", "duck", "mute", "parametric-eq", "compressor", "gate", "limiter", "delay", "graphic-eq", "pitch"]);
    expect(filterLibraryEntries(libraryEntries, "verified running application").map((entry) => entry.id)).toEqual([]);
  });

  it("keeps endpoint loopback discoverable with its actionable prerequisite", () => {
    const entries = libraryEntries.filter((entry) => entry.id === "endpoint-loopback");
    expect(entries.map((entry) => entry.id)).toEqual(["endpoint-loopback"]);
    expect(entries.map((entry) => entry.unavailableReason)).toEqual([
      "Select an exact active render endpoint in Endpoint binding",
    ]);
    expect(libraryEntries.some((entry) => entry.id === "application-capture")).toBe(false);
    expect(libraryEntries.find((entry) => entry.id === "physical-input")?.kind).toBe("physicalInput");
    expect(libraryEntries.find((entry) => entry.id === "physical-output")?.kind).toBe("physicalOutput");
    expect(libraryEntries.filter((entry) => entry.kind === "physicalInput")).toHaveLength(1);
    expect(libraryEntries.filter((entry) => entry.kind === "physicalOutput")).toHaveLength(1);
  });

  it("keeps virtual bus entries discoverable and typed for drag-and-drop", () => {
    expect(filterLibraryEntries(libraryEntries, "virtual bus").map((entry) => entry.id)).toEqual([
      "virtual-render-source",
      "virtual-capture-sink",
    ]);
    const renderSource = libraryEntries.find((entry) => entry.id === "virtual-render-source");
    const captureSink = libraryEntries.find((entry) => entry.id === "virtual-capture-sink");
    expect(renderSource?.virtualKind).toBe("virtualRenderSource");
    expect(captureSink?.virtualKind).toBe("virtualCaptureSink");
    expect(renderSource?.kind).toBeUndefined();
    expect(captureSink?.unavailableReason).toContain("does not install its own virtual devices");
    expect(captureSink?.unavailableReason).toContain("Output device");
  });

  it("points unavailable virtual devices at the existing virtual-cable alternative in short notes", () => {
    const renderSource = libraryEntries.find((entry) => entry.id === "virtual-render-source");
    expect(renderSource?.unavailableReason).toContain("Input device");
    const existingInput = libraryEntries.find((entry) => entry.id === "physical-input");
    const existingOutput = libraryEntries.find((entry) => entry.id === "physical-output");
    expect(existingInput?.kind).toBe("physicalInput");
    expect(existingInput?.note).toMatch(/virtual cable/i);
    expect(existingOutput?.note).toMatch(/virtual cable/i);
    // Tool notes stay short and product-neutral (user preference, 2026-09-27).
    for (const entry of libraryEntries) {
      expect(`${entry.note ?? ""} ${entry.unavailableReason ?? ""}`).not.toMatch(/voicemeeter/i);
      expect((entry.note ?? "").length, entry.id).toBeLessThanOrEqual(180);
    }
  });

  it("returns all entries for blank queries", () => {
    expect(filterLibraryEntries(libraryEntries, "  ")).toEqual(libraryEntries);
  });

  it("includes unavailable reasons in accessible library labels", () => {
    expect(libraryEntryAccessibleLabel(libraryEntries.find((entry) => entry.id === "gain")!)).toBe("Gain, Effect");
  });

  it("exposes recorder graph placement while keeping actions in the recorder panel", () => {
    const recorder = libraryEntries.find((entry) => entry.id === "recorder");
    expect(recorder?.kind).toBe("recorder");
    expect(recorder?.unavailableReason).toBeUndefined();
  });

  it("exposes the graph-native Test Signal as an available source", () => {
    expect(libraryEntries.find((entry) => entry.id === "test-signal")).toMatchObject({
      label: "Test Signal",
      category: "Source",
      kind: "testSignal",
    });
  });
});
