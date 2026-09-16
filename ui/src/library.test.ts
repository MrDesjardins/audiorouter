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
  });

  it("keeps virtual bus entries discoverable but unavailable", () => {
    expect(filterLibraryEntries(libraryEntries, "virtual bus").map((entry) => entry.id)).toEqual([
      "virtual-render-source",
      "virtual-capture-sink",
    ]);
    expect(
      libraryEntries.filter((entry) => entry.id.startsWith("virtual-")).every(
        (entry) => entry.kind === undefined && entry.unavailableReason?.includes("M03") === true,
      ),
    ).toBe(true);
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

  it("directs recorder actions to the dedicated recorder panel", () => {
    const recorder = libraryEntries.find((entry) => entry.id === "recorder");
    expect(recorder?.unavailableReason).toBe("Use the Recorder panel to control recording");
    expect(recorder?.kind).toBeUndefined();
  });
});
