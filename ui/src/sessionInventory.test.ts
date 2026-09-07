import { describe, expect, it } from "vitest";
import { demoSession } from "./fixtures";
import { mergeSessionInventory } from "./sessionInventory";

describe("session inventory merge", () => {
  it("prefers the point-in-time snapshot over a stale listed copy", () => {
    const listed = { ...demoSession, name: "stale", revision: 2 };
    const snapshot = { ...demoSession, name: "current", revision: 3 };
    expect(mergeSessionInventory([listed], snapshot, [])).toEqual([snapshot]);
  });

  it("preserves other listed sessions and gives local creations precedence", () => {
    const other = { ...demoSession, id: "other", name: "Other" };
    const created = { ...demoSession, id: "created", name: "Created" };
    expect(mergeSessionInventory([demoSession, other], null, [created])).toEqual([demoSession, other, created]);
  });
});
