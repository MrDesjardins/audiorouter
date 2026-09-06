import { describe, expect, it } from "vitest";
import { createDisconnectedBackend, SnapshotCache, type UiBackend } from "./backend";
import { demoSession } from "./fixtures";
import { describeDraftChanges, setNodeDraftFlag } from "./draft";

describe("disconnected backend", () => {
  it("returns safe local state and an empty event cursor", async () => {
    const backend = createDisconnectedBackend();
    const snapshot = await backend.snapshot();
    expect(backend.connected).toBe(false);
    expect(snapshot.session.id).toBe(demoSession.id);
    expect(await backend.subscribe()).toEqual({ backendEpoch: 0, events: [], nextSequence: 0 });
    await expect(backend.planGraph(demoSession)).rejects.toThrow("backend is disconnected");
    await expect(backend.commitGraph("plan-1", demoSession.revision, "ui-op")).rejects.toThrow(
      "backend is disconnected",
    );
  });
});

describe("snapshot cache", () => {
  it("retains the last snapshot when refresh fails", async () => {
    const cache = new SnapshotCache();
    const first = await cache.refresh(createDisconnectedBackend());
    const failing: UiBackend = {
      connected: true,
      snapshot: async () => { throw new Error("pipe closed"); },
      subscribe: async () => ({ backendEpoch: 0, events: [], nextSequence: 0 }),
      inspectRoute: async () => null,
      planGraph: async () => { throw new Error("not connected"); },
      commitGraph: async () => { throw new Error("not connected"); },
    };
    const second = await cache.refresh(failing);
    expect(first.snapshot?.session.id).toBe(demoSession.id);
    expect(second.snapshot?.session.id).toBe(demoSession.id);
    expect(second.stale).toBe(true);
    expect(second.error).toBe("pipe closed");
  });
});

describe("plan-only drafts", () => {
  it("changes a node flag without changing the revision", () => {
    const candidate = setNodeDraftFlag(demoSession, "voice", "bypass", true);
    expect(candidate.revision).toBe(demoSession.revision);
    expect(describeDraftChanges(demoSession, candidate)).toEqual([{ path: "/nodes/1/bypass", value: true }]);
  });

  it("rejects unknown nodes", () => {
    expect(() => setNodeDraftFlag(demoSession, "missing", "enabled", false)).toThrow("Unknown node");
  });
});
