import { describe, expect, it } from "vitest";
import { filterLibraryEntries, libraryEntries, libraryEntryAccessibleLabel } from "./library";

describe("node library search", () => {
  it("matches labels, categories, and availability reasons", () => {
    expect(filterLibraryEntries(libraryEntries, "effect").map((entry) => entry.id)).toEqual(["gain", "mute", "parametric-eq", "compressor", "gate", "limiter", "delay", "graphic-eq", "pitch"]);
    expect(filterLibraryEntries(libraryEntries, "verified running application").map((entry) => entry.id)).toEqual([
      "application-capture",
    ]);
  });

  it("keeps identity-bound sources discoverable with actionable prerequisites", () => {
    const entries = libraryEntries.filter((entry) => entry.id === "application-capture" || entry.id === "endpoint-loopback");
    expect(entries.map((entry) => entry.id)).toEqual(["application-capture", "endpoint-loopback"]);
    expect(entries.map((entry) => entry.unavailableReason)).toEqual([
      "Select a verified running application in Audio sources",
      "Select an exact active render endpoint in Endpoint binding",
    ]);
    expect(libraryEntries.find((entry) => entry.id === "physical-input")?.kind).toBe("physicalInput");
    expect(libraryEntries.find((entry) => entry.id === "physical-output")?.kind).toBe("physicalOutput");
    expect(libraryEntries.find((entry) => entry.id === "existing-virtual-output")?.kind).toBe("physicalOutput");
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
    expect(captureSink?.unavailableReason).toBe("Requires the deferred AudioRouter-managed signed driver");
  });

  it("returns all entries for blank queries", () => {
    expect(filterLibraryEntries(libraryEntries, "  ")).toEqual(libraryEntries);
  });

  it("includes unavailable reasons in accessible library labels", () => {
    expect(libraryEntryAccessibleLabel(libraryEntries.find((entry) => entry.id === "application-capture")!)).toBe(
      "Application capture, Source, unavailable: Select a verified running application in Audio sources",
    );
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
