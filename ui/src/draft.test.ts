import { describe, expect, it } from "vitest";
import { appendDraftConnection, appendLibraryNode, removeDraftNode, setNodeDraftName } from "./draft";
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

  it("renames without changing identity, revision, or topology", () => {
    const renamed = setNodeDraftName(demoSession, "voice", "  Voice processing  ");
    expect(renamed.nodes.find((node) => node.id === "voice")?.name).toBe("Voice processing");
    expect(renamed.id).toBe(demoSession.id);
    expect(renamed.revision).toBe(demoSession.revision);
    expect(renamed.edges).toEqual(demoSession.edges);
    expect(() => setNodeDraftName(demoSession, "voice", " ")).toThrow("cannot be empty");
    expect(() => setNodeDraftName(demoSession, "voice", "x".repeat(121))).toThrow("120");
  });

  it("removes a node and only its incident edges", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const reduced = removeDraftNode(connected, "voice");
    expect(reduced.nodes.map((node) => node.id)).toEqual(["mic", "headphones"]);
    expect(reduced.edges).toEqual([]);
    expect(reduced.revision).toBe(demoSession.revision);
    expect(() => removeDraftNode(reduced, "voice")).toThrow("Unknown draft node");
  });
});
