import { describe, expect, it } from "vitest";
import { appendDraftConnection, appendLibraryNode, duplicateDraftNode, GAIN_MAX_DB, GAIN_MIN_DB, removeDraftNode, resetNodeDraftParameters, setNodeDraftName, setNodeDraftParameter, setSessionDraftName } from "./draft";
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

  it("duplicates a node without copying edges or changing the revision", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const duplicated = duplicateDraftNode(connected, "voice");
    const copy = duplicated.nodes.at(-1);
    expect(copy).toMatchObject({ id: "voice-copy-1", name: "Voice gain copy 1", kind: "gain", parameters: { gainDb: 0 } });
    expect(duplicated.edges).toEqual(connected.edges);
    expect(duplicated.revision).toBe(demoSession.revision);
    const twice = duplicateDraftNode(duplicated, "voice");
    expect(twice.nodes.at(-1)?.id).toBe("voice-copy-2");
  });

  it("resets supported parameters without changing topology", () => {
    const changed = { ...demoSession, nodes: demoSession.nodes.map((node) => node.id === "voice" ? { ...node, parameters: { gainDb: 8 } } : node) };
    const reset = resetNodeDraftParameters(changed, "voice");
    expect(reset.nodes.find((node) => node.id === "voice")?.parameters).toEqual({ gainDb: 0 });
    expect(reset.revision).toBe(demoSession.revision);
    expect(() => resetNodeDraftParameters(demoSession, "mic")).not.toThrow();
  });

  it("keeps gain drafts inside the documented range", () => {
    expect(setNodeDraftParameter(demoSession, "voice", "gainDb", GAIN_MAX_DB).nodes[1].parameters.gainDb).toBe(24);
    expect(setNodeDraftParameter(demoSession, "voice", "gainDb", GAIN_MIN_DB).nodes[1].parameters.gainDb).toBe(-60);
    expect(() => setNodeDraftParameter(demoSession, "voice", "gainDb", 24.1)).toThrow("between -60 and 24");
  });

  it("normalizes and bounds session names", () => {
    expect(setSessionDraftName(demoSession, "  Streaming setup  ").name).toBe("Streaming setup");
    expect(() => setSessionDraftName(demoSession, " ")).toThrow("cannot be empty");
    expect(() => setSessionDraftName(demoSession, "x".repeat(121))).toThrow("120");
  });
});
