import { describe, expect, it } from "vitest";
import { libraryEntries } from "./library";
import {
  COMMON_TOOL_IDS,
  libraryEntryKind,
  needsSetup,
  rememberRecent,
  SHELF_ROWS,
  shelfRows,
  togglePin,
  TOOL_GROUP_OF,
} from "./toolGroups";

const usable = new Set(libraryEntries.filter((entry) => !needsSetup(entry)).map((entry) => entry.id));

describe("Tools list", () => {
  it("places every library tool in a group", () => {
    for (const entry of libraryEntries) expect(TOOL_GROUP_OF[libraryEntryKind(entry)], entry.id).toBeTruthy();
  });

  it("has common tools that really exist and work on a new PC", () => {
    for (const id of COMMON_TOOL_IDS) expect(usable.has(id), id).toBe(true);
    expect(shelfRows([], [], usable)).toEqual([...COMMON_TOOL_IDS]);
  });

  it("always shows five rows, recent first, without repeats", () => {
    const rows = shelfRows([], ["gain", "duck"], usable);
    expect(rows).toHaveLength(SHELF_ROWS);
    expect(rows.slice(0, 2)).toEqual(["gain", "duck"]);
    expect(new Set(rows).size).toBe(SHELF_ROWS);
  });

  it("keeps pins above recents, and keeps every pin", () => {
    expect(shelfRows(["mute"], ["gain"], usable).slice(0, 2)).toEqual(["mute", "gain"]);
    const pins = ["mute", "gain", "duck", "gate", "limiter", "meter"];
    expect(shelfRows(pins, ["delay"], usable)).toEqual(pins);
  });

  it("skips a remembered tool that cannot be added now and still fills the rows", () => {
    const rows = shelfRows(["virtual-render-source"], ["endpoint-loopback"], usable);
    expect(rows).not.toContain("virtual-render-source");
    expect(rows).not.toContain("endpoint-loopback");
    expect(rows).toHaveLength(SHELF_ROWS);
  });

  it("moves a repeated tool to the front and bounds the list", () => {
    expect(rememberRecent(["a", "b"], "b")).toEqual(["b", "a"]);
    expect(rememberRecent(["a", "b", "c", "d", "e"], "f")).toEqual(["f", "a", "b", "c", "d"]);
    expect(togglePin(["a"], "a")).toEqual([]);
    expect(togglePin(["a"], "b")).toEqual(["a", "b"]);
  });
});
