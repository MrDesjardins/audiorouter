import { describe, expect, it } from "vitest";
import { appendDraftConnection, setDraftConnectionEnabled } from "./draft";
import { demoSession } from "./fixtures";
import { relatedNodeIds } from "./graphView";
import { nodePortLabels } from "./graphView";

describe("graph path highlighting", () => {
  it("describes each port with direction, role, and channels", () => {
    expect(nodePortLabels(demoSession.nodes[1])).toEqual(["input: in (1ch)", "output: out (1ch)"]);
  });

  it("finds all enabled upstream and downstream nodes", () => {
    const connected = appendDraftConnection(appendDraftConnection(demoSession, "mic", "out", "voice", "in"), "voice", "out", "headphones", "in");
    expect([...relatedNodeIds(connected, "voice")]).toEqual(["voice", "mic", "headphones"]);
  });

  it("does not cross disabled edges or unrelated branches", () => {
    const connected = appendDraftConnection(appendDraftConnection(demoSession, "mic", "out", "voice", "in"), "voice", "out", "headphones", "in");
    const disabled = setDraftConnectionEnabled(connected, "edge-1", false);
    expect(relatedNodeIds(disabled, "voice")).toEqual(new Set(["voice", "headphones"]));
    expect(relatedNodeIds(disabled, "unknown")).toEqual(new Set());
  });
});
