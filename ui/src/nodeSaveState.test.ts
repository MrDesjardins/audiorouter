import { describe, expect, it } from "vitest";
import { demoSession } from "./fixtures";
import { appendDraftConnection } from "./draft";
import { unsavedNodeIds } from "./nodeSaveState";

describe("canvas save indicators", () => {
  it("marks parameter and name drafts and clears them after authoritative save", () => {
    const draft = structuredClone(demoSession);
    draft.nodes[1].parameters.gainDb = -6;
    draft.nodes[0].name = "Voice input";
    expect([...unsavedNodeIds(demoSession, draft)].sort()).toEqual(["mic", "voice"]);
    expect(unsavedNodeIds({ ...draft, revision: draft.revision + 1 }, draft).size).toBe(0);
  });

  it("marks both ends of added, changed and removed wiring", () => {
    const routed = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    expect([...unsavedNodeIds(demoSession, routed)].sort()).toEqual(["mic", "voice"]);
    expect([...unsavedNodeIds(routed, demoSession)].sort()).toEqual(["mic", "voice"]);
    const changed = structuredClone(routed);
    changed.edges[0].enabled = false;
    expect([...unsavedNodeIds(routed, changed)].sort()).toEqual(["mic", "voice"]);
  });

  it("does not mark backend key reordering as an unsaved edit", () => {
    const saved = structuredClone(demoSession);
    saved.nodes[1].parameters = { gainDb: 0, pairingKey: "private-value" };
    const draft = structuredClone(saved);
    draft.nodes[1].parameters = { pairingKey: "private-value", gainDb: 0 };
    expect(unsavedNodeIds(saved, draft).size).toBe(0);
  });
});
