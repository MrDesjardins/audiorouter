import { describe, expect, it } from "vitest";
import { filterLibraryEntries, libraryEntries } from "./library";

describe("node library search", () => {
  it("matches labels, categories, and unavailable reasons", () => {
    expect(filterLibraryEntries(libraryEntries, "effect").map((entry) => entry.id)).toEqual(["gain", "mute"]);
    expect(filterLibraryEntries(libraryEntries, "M02").map((entry) => entry.id)).toEqual(["physical-input"]);
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
});
