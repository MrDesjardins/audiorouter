import { describe, expect, it } from "vitest";
import { filterLibraryEntries, libraryEntries } from "./library";

describe("node library search", () => {
  it("matches labels, categories, and unavailable reasons", () => {
    expect(filterLibraryEntries(libraryEntries, "effect").map((entry) => entry.id)).toEqual(["gain", "mute"]);
    expect(filterLibraryEntries(libraryEntries, "M02").map((entry) => entry.id)).toEqual(["physical-input"]);
  });

  it("returns all entries for blank queries", () => {
    expect(filterLibraryEntries(libraryEntries, "  ")).toEqual(libraryEntries);
  });
});
