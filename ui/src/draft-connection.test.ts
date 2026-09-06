import { describe, expect, it } from "vitest";
import { appendDraftConnection, removeDraftConnection, setDraftConnectionEnabled } from "./draft";
import { demoSession } from "./fixtures";

describe("appendDraftConnection", () => {
  it("adds a deterministic identity matrix without changing revision", () => {
    const next = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    expect(next.edges).toEqual([{
      id: "edge-1",
      sourceNode: "mic",
      sourcePort: "out",
      destinationNode: "voice",
      destinationPort: "in",
      matrix: [1],
      enabled: true,
    }]);
    expect(next.revision).toBe(demoSession.revision);
  });

  it("creates a bounded mono-to-stereo map and rejects duplicate inputs", () => {
    const next = appendDraftConnection(demoSession, "mic", "out", "headphones", "in");
    expect(next.edges[0].matrix).toEqual([1, 1]);
    expect(() => appendDraftConnection(next, "mic", "out", "headphones", "in")).toThrow("already in the draft");
    expect(() => appendDraftConnection(next, "voice", "out", "headphones", "in")).toThrow("already has a connection");
  });

  it("removes only the requested draft edge", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const restored = removeDraftConnection(connected, "edge-1");
    expect(restored.edges).toEqual([]);
    expect(restored.revision).toBe(demoSession.revision);
    expect(() => removeDraftConnection(restored, "edge-1")).toThrow("Unknown draft connection");
  });

  it("toggles edge state without changing topology or revision", () => {
    const connected = appendDraftConnection(demoSession, "mic", "out", "voice", "in");
    const disabled = setDraftConnectionEnabled(connected, "edge-1", false);
    expect(disabled.edges[0]).toMatchObject({ id: "edge-1", enabled: false, sourceNode: "mic", destinationNode: "voice" });
    expect(disabled.revision).toBe(demoSession.revision);
    expect(() => setDraftConnectionEnabled(disabled, "missing", true)).toThrow("Unknown draft connection");
  });
});
