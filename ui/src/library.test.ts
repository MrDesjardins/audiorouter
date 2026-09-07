import { describe, expect, it } from "vitest";
import { filterLibraryEntries, libraryEntries, libraryEntryAccessibleLabel } from "./library";

describe("node library search", () => {
  it("matches labels, categories, and unavailable reasons", () => {
    expect(filterLibraryEntries(libraryEntries, "effect").map((entry) => entry.id)).toEqual(["gain", "mute", "parametric-eq", "compressor", "gate"]);
    expect(filterLibraryEntries(libraryEntries, "M02").map((entry) => entry.id)).toEqual([
      "physical-input",
      "application-capture",
      "endpoint-loopback",
      "physical-output",
    ]);
  });

  it("keeps every M02 source and destination discoverable but unavailable", () => {
    const entries = libraryEntries.filter((entry) => entry.unavailableReason?.includes("M02") === true);
    expect(entries.map((entry) => entry.id)).toEqual([
      "physical-input",
      "application-capture",
      "endpoint-loopback",
      "physical-output",
    ]);
    expect(entries.every((entry) => entry.kind === undefined)).toBe(true);
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
    expect(libraryEntryAccessibleLabel(libraryEntries.find((entry) => entry.id === "physical-output")!)).toBe(
      "Physical output, Destination, unavailable: Requires the M02 Windows audio adapter",
    );
    expect(libraryEntryAccessibleLabel(libraryEntries.find((entry) => entry.id === "gain")!)).toBe("Gain, Effect");
  });
});
