import { describe, expect, it } from "vitest";
import { createDisconnectedBackend, createLiveBackend, createLiveBackendFromTransport, SnapshotCache, type UiBackend } from "./backend";
import { demoSession } from "./fixtures";
import { applyGraphDraft, describeDraftChanges, setNodeDraftFlag, setNodeDraftParameter } from "./draft";

describe("disconnected backend", () => {
  it("returns safe local state and an empty event cursor", async () => {
    const backend = createDisconnectedBackend();
    const snapshot = await backend.snapshot();
    expect(backend.connected).toBe(false);
    expect(snapshot.session.id).toBe(demoSession.id);
    expect(await backend.listRecordings()).toEqual([]);
    await expect(backend.previewRecording("recording")).rejects.toThrow("recording preview is unavailable");
    await expect(backend.clearRecoverySafeMode()).rejects.toThrow("recovery safe-mode clearing is unavailable");
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
      listRecordings: async () => [],
      previewRecording: async () => { throw new Error("not connected"); },
      setPrivacyMute: async () => { throw new Error("not connected"); },
      clearRecoverySafeMode: async () => { throw new Error("not connected"); },
      removeRecordingEntry: async () => { throw new Error("not connected"); },
      createSession: async () => { throw new Error("not connected"); },
      duplicateSession: async () => { throw new Error("not connected"); },
      deleteSession: async () => { throw new Error("not connected"); },
      getRecordingRecovery: async () => { throw new Error("not connected"); },
      setRecordingMetadata: async () => { throw new Error("not connected"); },
    };
    const second = await cache.refresh(failing);
    expect(first.snapshot?.session.id).toBe(demoSession.id);
    expect(second.snapshot?.session.id).toBe(demoSession.id);
    expect(second.stale).toBe(true);
    expect(second.error).toBe("pipe closed");
  });
});

describe("live event cursor", () => {
  it("forwards the backend epoch and bounded cursor to the shared client", async () => {
    let received: unknown;
    const client = {
      request: async (method: string, params: unknown) => {
        received = { method, params };
        return { backendEpoch: 8, events: [], nextSequence: 12 };
      },
    } as never;
    const backend = createLiveBackend(client, demoSession.id);
    await backend.subscribe(7, demoSession.id, 8);
    expect(received).toEqual({
      method: "events.subscribe",
      params: { afterSequence: 7, limit: 500, backendEpoch: 8, sessionId: demoSession.id },
    });
  });

  it("constructs the typed live backend from a host transport", async () => {
    let method = "";
    const backend = createLiveBackendFromTransport({
      send: async request => {
        method = request.method;
        return { jsonrpc: "2.0", id: request.id ?? null, result: { backendEpoch: 1, events: [], nextSequence: 0 } };
      },
    }, demoSession.id);
    await backend.subscribe();
    expect(method).toBe("events.subscribe");
  });

  it("lists recordings through the authorized session-scoped API", async () => {
    let received: unknown;
    const client = {
      request: async (method: string, params: unknown) => {
        received = { method, params };
        return [];
      },
    } as never;
    const backend = createLiveBackend(client, demoSession.id);
    await backend.listRecordings();
    expect(received).toEqual({ method: "recordings.list", params: { sessionId: demoSession.id } });
  });

  it("forwards recording preview through the read-only API", async () => {
    let received: unknown;
    const client = {
      request: async (method: string, params: unknown) => {
        received = { method, params };
        return { recordingId: "take-1", status: "present" };
      },
    } as never;
    const backend = createLiveBackend(client, demoSession.id);
    await expect(backend.previewRecording("take-1")).resolves.toEqual({ recordingId: "take-1", status: "present" });
    expect(received).toEqual({ method: "recordings.preview", params: { recordingId: "take-1" } });
  });

  it("forwards the privacy safety latch through the live API", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { muted: true }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).setPrivacyMute(true)).resolves.toEqual({ muted: true });
    expect(received).toEqual({ method: "safety.setPrivacyMute", params: { muted: true } });
  });

  it("forwards authorized recovery safe-mode clearing through the live API", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { safeMode: false, recentCrashes: 0, persistence: "durable" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).clearRecoverySafeMode()).resolves.toMatchObject({ safeMode: false });
    expect(received).toEqual({ method: "recovery.clearSafeMode", params: undefined });
  });

  it("forwards metadata-only recording entry removal", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { fileAction: "none" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).removeRecordingEntry("take-1")).resolves.toEqual({ fileAction: "none" });
    expect(received).toEqual({ method: "recordings.removeEntry", params: { recordingId: "take-1" } });
  });

  it("forwards recording recovery inspection without file actions", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { present: false }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).getRecordingRecovery("take-1")).resolves.toEqual({ present: false });
    expect(received).toEqual({ method: "recordings.recovery", params: { recordingId: "take-1" } });
  });

  it("forwards metadata edits without changing the recording path", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { recordingId: "take-1", path: "C:\\approved\\take.wav", title: "Edited" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).setRecordingMetadata("take-1", { title: "Edited" })).resolves.toMatchObject({ title: "Edited" });
    expect(received).toEqual({ method: "recordings.setMetadata", params: { recordingId: "take-1", title: "Edited" } });
  });

  it("creates a stopped session through the shared API", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { session: { ...demoSession, id: "new-session", name: "New", revision: 0 }, state: "stopped" }; } } as never;
    const candidate = { ...demoSession, id: "new-session", name: "New", revision: 0 };
    await expect(createLiveBackend(client, demoSession.id).createSession(candidate)).resolves.toMatchObject({ state: "stopped" });
    expect(received).toEqual({ method: "sessions.create", params: { session: candidate } });
  });

  it("duplicates a stopped session through the shared API", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { session: { ...demoSession, id: "copy", name: "Copy", revision: 0 }, state: "stopped" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).duplicateSession(demoSession.id, "copy", "Copy")).resolves.toMatchObject({ state: "stopped" });
    expect(received).toEqual({ method: "sessions.duplicate", params: { sourceSessionId: demoSession.id, sessionId: "copy", name: "Copy" } });
  });

  it("deletes a session through the shared API", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { deleted: true, state: "stopped" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).deleteSession("copy")).resolves.toEqual({ deleted: true, state: "stopped" });
    expect(received).toEqual({ method: "sessions.delete", params: { sessionId: "copy" } });
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

  it("edits a processor parameter and includes it in the deterministic plan", () => {
    const candidate = setNodeDraftParameter(demoSession, "voice", "gainDb", -6);
    expect(candidate.revision).toBe(demoSession.revision);
    expect(candidate.nodes[1].parameters.gainDb).toBe(-6);
    expect(describeDraftChanges(demoSession, candidate)).toEqual([
      { path: "/nodes/1/parameters/gainDb", value: -6 },
    ]);
  });

  it("edits a boolean Mute parameter without changing the revision", () => {
    const muteSession = {
      ...demoSession,
      nodes: demoSession.nodes.map((node) => node.id === "voice"
        ? { ...node, kind: "mute" as const, parameters: {} }
        : node),
    };
    const candidate = setNodeDraftParameter(muteSession, "voice", "muted", true);
    expect(candidate.revision).toBe(muteSession.revision);
    expect(candidate.nodes[1].parameters.muted).toBe(true);
    expect(describeDraftChanges(muteSession, candidate)).toEqual([
      { path: "/nodes/1/parameters/muted", value: true },
    ]);
  });

  it("plans before committing and forwards the revision/key", async () => {
    const calls: string[] = [];
    const backend: Pick<UiBackend, "planGraph" | "commitGraph"> = {
      planGraph: async candidate => {
        calls.push(`plan:${candidate.revision}`);
        return {
          planId: "plan-ui",
          baseRevision: candidate.revision,
          expiresInMs: 30_000,
          diff: [],
          affectedDestinations: [],
          warnings: [],
          requiredScopes: ["graph.write"],
        };
      },
      commitGraph: async (planId, revision, key) => {
        calls.push(`commit:${planId}:${revision}:${key}`);
        return { sessionId: demoSession.id, revision: revision + 1 };
      },
    };
    await expect(applyGraphDraft(backend, demoSession, "ui-operation")).resolves.toEqual({
      sessionId: demoSession.id,
      revision: demoSession.revision + 1,
    });
    expect(calls).toEqual([
      `plan:${demoSession.revision}`,
      `commit:plan-ui:${demoSession.revision}:ui-operation`,
    ]);
  });

  it("rejects a plan whose base revision does not match the draft", async () => {
    const backend: Pick<UiBackend, "planGraph" | "commitGraph"> = {
      planGraph: async () => ({
        planId: "plan-stale",
        baseRevision: demoSession.revision + 1,
        expiresInMs: 30_000,
        diff: [],
        affectedDestinations: [],
        warnings: [],
        requiredScopes: ["graph.write"],
      }),
      commitGraph: async () => ({ sessionId: demoSession.id, revision: 1 }),
    };
    await expect(applyGraphDraft(backend, demoSession, "stale-operation")).rejects.toThrow(
      "different session revision",
    );
  });

  it("requires and forwards explicit acknowledgments for warning plans", async () => {
    const calls: unknown[][] = [];
    const backend: Pick<UiBackend, "planGraph" | "commitGraph"> = {
      planGraph: async () => ({ planId: "warn-plan", baseRevision: demoSession.revision, expiresInMs: 30_000, diff: [], affectedDestinations: [], warnings: ["audible change"], requiredScopes: ["graph.write"] }),
      commitGraph: async (...args) => { calls.push(args); return { sessionId: demoSession.id, revision: 2 }; },
    };
    await expect(applyGraphDraft(backend, demoSession, "warning-operation")).rejects.toThrow("requires acknowledgment");
    await expect(applyGraphDraft(backend, demoSession, "warning-operation", ["audible change"])).resolves.toMatchObject({ revision: 2 });
    expect(calls).toEqual([["warn-plan", demoSession.revision, "warning-operation", ["audible change"]]]);
  });
});
