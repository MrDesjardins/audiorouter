import { describe, expect, it } from "vitest";
import { appendLibraryNode } from "./draft";
import { demoSession } from "./fixtures";

describe("appendLibraryNode", () => {
  it("adds a valid built-in processor without changing revision or edges", () => {
    const next = appendLibraryNode(demoSession, "gain");
    const added = next.nodes.at(-1);
    expect(added).toMatchObject({
      id: "gain-1",
      kind: "gain",
      name: "Gain 1",
      enabled: true,
      bypass: false,
      parameters: { gainDb: 0 },
    });
    expect(added?.ports).toEqual([
      { name: "in", direction: "input", channels: 1 },
      { name: "out", direction: "output", channels: 1 },
    ]);
    expect(next.revision).toBe(demoSession.revision);
    expect(next.edges).toEqual(demoSession.edges);
  });

  it("chooses the next deterministic id when a processor already exists", () => {
    const once = appendLibraryNode(demoSession, "meter");
    const twice = appendLibraryNode(once, "meter");
    expect(twice.nodes.slice(-2).map((node) => node.id)).toEqual(["meter-1", "meter-2"]);
  });
});
