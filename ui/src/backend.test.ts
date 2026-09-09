import { describe, expect, it } from "vitest";
import { createDisconnectedBackend, createLiveBackend, createLiveBackendFromTransport, formatUiError, SnapshotCache, type UiBackend } from "./backend";
import { AudioRouterRpcError } from "@audiorouter/contracts";
import { demoSession } from "./fixtures";
import { applyGraphDraft, describeDraftChanges, setNodeDraftFlag, setNodeDraftParameter } from "./draft";

describe("disconnected backend", () => {
  it("returns safe local state and an empty event cursor", async () => {
    const backend = createDisconnectedBackend();
    const snapshot = await backend.snapshot();
    expect(backend.connected).toBe(false);
    expect(snapshot.status.privacyMute.muted).toBe(true);
    expect(snapshot.session.id).toBe(demoSession.id);
    expect(await backend.listRecordings()).toEqual([]);
    expect(await backend.listSessions()).toEqual(expect.arrayContaining([demoSession]));
    expect(await backend.listApplications()).toEqual([]);
    await expect(backend.previewRecording("recording")).rejects.toThrow("recording preview is unavailable");
    await expect(backend.clearRecoverySafeMode()).rejects.toThrow("recovery safe-mode clearing is unavailable");
    expect(await backend.subscribe()).toEqual({ backendEpoch: 0, events: [], nextSequence: 0 });
    await expect(backend.planGraph(demoSession)).rejects.toThrow("backend is disconnected");
    await expect(backend.commitGraph("plan-1", demoSession.revision, "ui-op")).rejects.toThrow(
      "backend is disconnected",
    );
  });
});

describe("UI error formatting", () => {
  it("keeps audio category, HRESULT, remediation, and retry guidance", () => {
    const error = new AudioRouterRpcError({
      code: -32010,
      message: "Windows audio endpoint enumeration failed.",
      data: {
        code: "deviceInUse",
        fieldPath: null,
        resourceIds: [],
        retryable: true,
        remediation: "Retry after the owning stream releases the endpoint.",
        hresult: 0x8889000A,
      },
    });
    expect(formatUiError(error, "fallback")).toBe(
      "Windows audio endpoint enumeration failed. [deviceInUse, HRESULT 0x8889000A] Retry after the owning stream releases the endpoint. Retry may succeed.",
    );
  });

  it("uses the fallback for non-error failures", () => {
    expect(formatUiError("bad failure", "Inventory unavailable")).toBe("Inventory unavailable");
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
      listSessions: async () => [],
      listApplications: async () => [],
      listDevices: async () => [],
      listProcessors: async () => [],
      listPresets: async () => { throw new Error("not connected"); },
      scanPlugins: async () => { throw new Error("not connected"); },
      listPlugins: async () => { throw new Error("not connected"); },
      retryPlugins: async () => { throw new Error("not connected"); },
      inspectPlugin: async () => { throw new Error("not connected"); },
      listVirtualDevices: async () => [],
      planVirtualDevice: async () => { throw new Error("not connected"); },
      applyVirtualDevice: async () => { throw new Error("not connected"); },
      previewRecording: async () => { throw new Error("not connected"); },
      setPrivacyMute: async () => { throw new Error("not connected"); },
      clearRecoverySafeMode: async () => { throw new Error("not connected"); },
      removeRecordingEntry: async () => { throw new Error("not connected"); },
      recycleRecording: async () => { throw new Error("not connected"); },
      armRecorder: async () => { throw new Error("not connected"); },
      startRecorder: async () => { throw new Error("not connected"); },
      pauseRecorder: async () => { throw new Error("not connected"); },
      resumeRecorder: async () => { throw new Error("not connected"); },
      splitRecorder: async () => { throw new Error("not connected"); },
      stopRecorder: async () => { throw new Error("not connected"); },
      createSession: async () => { throw new Error("not connected"); },
      duplicateSession: async () => { throw new Error("not connected"); },
      deleteSession: async () => { throw new Error("not connected"); },
      startSession: async () => { throw new Error("not connected"); },
      stopSession: async () => { throw new Error("not connected"); },
      exportSession: async () => { throw new Error("not connected"); },
      planSessionImport: async () => { throw new Error("not connected"); },
      commitSessionImport: async () => { throw new Error("not connected"); },
      getStartup: async () => ({ enabled: false, registration: "unavailable", reason: "not connected" }),
      planStartup: async () => { throw new Error("not connected"); },
      applyStartup: async () => { throw new Error("not connected"); },
      getRecordingRecovery: async () => { throw new Error("not connected"); },
      revealRecording: async () => { throw new Error("not connected"); },
      setRecordingMetadata: async () => { throw new Error("not connected"); },
      renameRecording: async () => { throw new Error("not connected"); },
    };
    const second = await cache.refresh(failing);
    expect(first.snapshot?.session.id).toBe(demoSession.id);
    expect(second.snapshot?.session.id).toBe(demoSession.id);
    expect(second.stale).toBe(true);
    expect(second.error).toBe("pipe closed");
  });

  it("retains structured audio diagnostics when the status refresh fails", async () => {
    const cache = new SnapshotCache();
    const failing: UiBackend = {
      ...createDisconnectedBackend(),
      connected: true,
      snapshot: async () => {
        throw new AudioRouterRpcError({
          code: -32010,
          message: "Audio status unavailable.",
          data: {
            code: "accessDenied",
            fieldPath: null,
            resourceIds: [],
            retryable: false,
            remediation: "Check endpoint access and microphone privacy permissions.",
            hresult: 0x80070005,
          },
        });
      },
    };
    await expect(cache.refresh(failing)).resolves.toMatchObject({
      stale: true,
      error: "Audio status unavailable. [accessDenied, HRESULT 0x80070005] Check endpoint access and microphone privacy permissions.",
    });
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
    await backend.subscribe(7, demoSession.id, 8, ["graph.committed"]);
    expect(received).toEqual({
      method: "events.subscribe",
      params: { afterSequence: 7, limit: 500, backendEpoch: 8, sessionId: demoSession.id, categories: ["graph.committed"] },
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
    expect(received).toEqual({ method: "recordings.list", params: { sessionId: demoSession.id, limit: 500 } });
  });

  it("normalizes a paged recording response for the existing UI row contract", async () => {
    const row = { id: "take-1" };
    const client = {
      request: async () => ({ items: [row], nextCursor: null }),
    } as never;
    await expect(createLiveBackend(client, demoSession.id).listRecordings()).resolves.toEqual([row]);
  });

  it("follows inventory cursors instead of dropping later pages", async () => {
    const requests: unknown[] = [];
    const client = {
      request: async (method: string, params: unknown) => {
        requests.push({ method, params });
        return requests.length === 1
          ? { items: [demoSession], nextCursor: "session-1" }
          : { items: [{ ...demoSession, id: "session-2" }], nextCursor: null };
      },
    } as never;
    await expect(createLiveBackend(client, demoSession.id).listSessions()).resolves.toEqual([
      demoSession,
      { ...demoSession, id: "session-2" },
    ]);
    expect(requests).toEqual([
      { method: "sessions.list", params: { limit: 500 } },
      { method: "sessions.list", params: { limit: 500, cursor: "session-1" } },
    ]);
  });

  it("rejects a non-advancing inventory cursor", async () => {
    const client = {
      request: async () => ({ items: [demoSession], nextCursor: "same" }),
    } as never;
    await expect(createLiveBackend(client, demoSession.id).listDevices()).rejects.toThrow(
      "non-advancing inventory cursor",
    );
  });

  it("normalizes the bounded session inventory through the shared API", async () => {
    let received: unknown;
    const client = {
      request: async (method: string, params: unknown) => {
        received = { method, params };
        return { items: [demoSession], nextCursor: null };
      },
    } as never;
    await expect(createLiveBackend(client, demoSession.id).listSessions()).resolves.toEqual([demoSession]);
    expect(received).toEqual({ method: "sessions.list", params: { limit: 500 } });
  });

  it("forwards the dedicated processor catalog method", async () => {
    let received: unknown;
    const client = {
      request: async (method: string, params: unknown) => {
        received = { method, params };
        return [];
      },
    } as never;
    await expect(createLiveBackend(client, demoSession.id).listProcessors()).resolves.toEqual([]);
    expect(received).toEqual({ method: "processors.list", params: undefined });
  });

  it("forwards the dedicated preset catalog method", async () => {
    let received: unknown;
    const client = {
      request: async (method: string, params: unknown) => {
        received = { method, params };
        return { voiceChains: [], eq: [] };
      },
    } as never;
    await expect(createLiveBackend(client, demoSession.id).listPresets()).resolves.toEqual({ voiceChains: [], eq: [] });
    expect(received).toEqual({ method: "presets.list", params: undefined });
  });

  it("maps application inventory to the canonical read method", async () => {
    let received: unknown;
    const client = {
      request: async (method: string, params: unknown) => {
        received = { method, params };
        return [];
      },
    } as never;
    const backend = createLiveBackend(client, demoSession.id);
    expect(await backend.listApplications()).toEqual([]);
    expect(received).toEqual({ method: "applications.list", params: undefined });
  });

  it("normalizes a paged device response through the bounded discovery API", async () => {
    const device = { id: "device-1", direction: "render", state: "active", format: { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 65534, bytesPerFrame: 8 }, periods: { default100ns: 100000, minimum100ns: 20000 } };
    let received: unknown;
    const client = {
      request: async (method: string, params: unknown) => { received = { method, params }; return { items: [device], nextCursor: null }; },
    } as never;
    await expect(createLiveBackend(client, demoSession.id).listDevices()).resolves.toEqual([device]);
    expect(received).toEqual({ method: "devices.list", params: { limit: 500 } });
  });

  it("forwards explicit plugin scan and inspection requests", async () => {
    const requests: unknown[] = [];
    const client = {
      request: async (method: string, params: unknown) => {
        requests.push({ method, params });
        return method === "plugins.scan" ? { directory: "C:\\Plugins", entries: [] } : { path: "C:\\Plugins\\demo.vst3", identity: null, error: "NotPe", errorCode: "notPe" };
      },
    } as never;
    const backend = createLiveBackend(client, demoSession.id);
    await backend.scanPlugins("C:\\Plugins");
    await backend.listPlugins("C:\\Plugins");
    await backend.retryPlugins("C:\\Plugins", "retry-key");
    await backend.inspectPlugin("C:\\Plugins\\demo.vst3");
    expect(requests).toEqual([
      { method: "plugins.scan", params: { directory: "C:\\Plugins" } },
      { method: "plugins.list", params: { directory: "C:\\Plugins" } },
      { method: "plugins.retry", params: { directory: "C:\\Plugins", idempotencyKey: "retry-key" } },
      { method: "plugins.inspect", params: { path: "C:\\Plugins\\demo.vst3" } },
    ]);
  });

  it("normalizes the managed virtual-device inventory through the shared API", async () => {
    const bus = { id: "bus-1", name: "Desktop", direction: "bidirectional", channels: 2, enabled: true };
    let received: unknown;
    const client = {
      request: async (method: string, params: unknown) => {
        received = { method, params };
        return { items: [bus], nextCursor: null };
      },
    } as never;
    await expect(createLiveBackend(client, demoSession.id).listVirtualDevices()).resolves.toEqual([bus]);
    expect(received).toEqual({ method: "virtualDevices.list", params: { limit: 500 } });
  });

  it("forwards virtual-device lifecycle plans and applies through typed methods", async () => {
    const operation = { action: "create", id: "bus-1", name: "Desktop In" } as const;
    const calls: unknown[] = [];
    const client = {
      request: async (method: string, params: unknown) => {
        calls.push({ method, params });
        return method === "virtualDevices.plan"
          ? { planId: "plan-1", expiresInMs: 300000, operation, availability: { status: "unavailable", reason: "driver" }, requiredScopes: ["deviceAdministration"], warnings: [] }
          : { planId: "plan-1", state: "applied", operation, availability: { status: "unavailable", reason: "driver" } };
      },
    } as never;
    const backend = createLiveBackend(client, demoSession.id);
    await expect(backend.planVirtualDevice(operation)).resolves.toMatchObject({ planId: "plan-1" });
    await expect(backend.applyVirtualDevice("plan-1", "key-1")).resolves.toMatchObject({ state: "applied" });
    expect(calls).toEqual([
      { method: "virtualDevices.plan", params: { operation } },
      { method: "virtualDevices.apply", params: { planId: "plan-1", idempotencyKey: "key-1" } },
    ]);
  });

  it("forwards recording preview through the read-only API", async () => {
    let received: unknown;
    const client = {
      request: async (method: string, params: unknown) => {
        received = { method, params };
        return { recordingId: "take-1", preview: { status: "missing" } };
      },
    } as never;
    const backend = createLiveBackend(client, demoSession.id);
    await expect(backend.previewRecording("take-1")).resolves.toEqual({ recordingId: "take-1", preview: { status: "missing" } });
    expect(received).toEqual({ method: "recordings.preview", params: { recordingId: "take-1" } });
  });

  it("forwards the privacy safety latch through the live API", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { muted: true, persistence: "memory", audioEffect: "process-local" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).setPrivacyMute(true)).resolves.toEqual({ muted: true, persistence: "memory", audioEffect: "process-local" });
    expect(received).toEqual({ method: "safety.setPrivacyMute", params: { muted: true } });
  });

  it("forwards authorized recovery safe-mode clearing through the live API", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { safeMode: false, recentCrashes: 0, persistence: "durable" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).clearRecoverySafeMode()).resolves.toMatchObject({ safeMode: false });
    expect(received).toEqual({ method: "recovery.clearSafeMode", params: undefined });
  });

  it("forwards the explicit startup status and plan/apply boundary", async () => {
    const calls: unknown[] = [];
    const client = {
      request: async (method: string, params: unknown) => {
        calls.push({ method, params });
        if (method === "startup.get") return { enabled: false, registration: "unavailable", reason: "native registration unavailable" };
        if (method === "startup.plan") return { planId: "startup-plan", enabled: true, registration: "unavailable", reason: "native registration unavailable", requiredScopes: ["sessionControl"], warnings: [] };
        return { planId: "startup-plan", state: "unavailable", registration: "unavailable", reason: "native registration unavailable" };
      },
    } as never;
    const backend = createLiveBackend(client, demoSession.id);
    await expect(backend.getStartup()).resolves.toMatchObject({ registration: "unavailable" });
    await expect(backend.planStartup(true)).resolves.toMatchObject({ planId: "startup-plan" });
    await expect(backend.applyStartup("startup-plan", "startup-key")).resolves.toMatchObject({ state: "unavailable" });
    expect(calls).toEqual([
      { method: "startup.get", params: undefined },
      { method: "startup.plan", params: { enabled: true } },
      { method: "startup.apply", params: { planId: "startup-plan", idempotencyKey: "startup-key" } },
    ]);
  });

  it("forwards metadata-only recording entry removal", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { recordingId: "take-1", removed: true, fileAction: "none" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).removeRecordingEntry("take-1", "remove-key")).resolves.toEqual({ recordingId: "take-1", removed: true, fileAction: "none" });
    expect(received).toEqual({ method: "recordings.removeEntry", params: { recordingId: "take-1", idempotencyKey: "remove-key" } });
  });

  it("forwards recording recovery inspection without file actions", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { recordingId: "take-1", status: "missing" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).getRecordingRecovery("take-1")).resolves.toEqual({ recordingId: "take-1", status: "missing" });
    expect(received).toEqual({ method: "recordings.recovery", params: { recordingId: "take-1" } });
  });

  it("forwards recording reveal without opening the path in the UI adapter", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { recordingId: "take-1", path: "C:\\approved\\take.wav", fileAction: "none", reason: "missing" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).revealRecording("take-1")).resolves.toMatchObject({ reason: "missing" });
    expect(received).toEqual({ method: "recordings.reveal", params: { recordingId: "take-1" } });
  });

  it("forwards explicit recording recycle confirmation through the API", async () => {
    let received: unknown;
    const client = {
      request: async (method: string, params: unknown) => {
        received = { method, params };
        return { recordingId: "take-1", path: "C:\\approved\\take.wav", fileAction: "recycle", preview: true };
      },
    } as never;
    await expect(createLiveBackend(client, demoSession.id).recycleRecording("take-1", false)).resolves.toMatchObject({ preview: true });
    expect(received).toEqual({ method: "recordings.recycle", params: { recordingId: "take-1", confirm: false } });
  });

  it("forwards metadata edits without changing the recording path", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { recordingId: "take-1", path: "C:\\approved\\take.wav", title: "Edited" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).setRecordingMetadata("take-1", { title: "Edited" })).resolves.toMatchObject({ title: "Edited" });
    expect(received).toEqual({ method: "recordings.setMetadata", params: { recordingId: "take-1", title: "Edited" } });
  });

  it("rejects oversized recording metadata before dispatch", async () => {
    let dispatched = false;
    const client = { request: async () => { dispatched = true; return { recordingId: "take-1", updated: true }; } } as never;
    const title = "😀".repeat(257);
    await expect(createLiveBackend(client, demoSession.id).setRecordingMetadata("take-1", { title })).rejects.toThrow("256 characters");
    expect(dispatched).toBe(false);
  });

  it("forwards recording renames as an explicit file operation", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { recordingId: "take-1", oldPath: "C:\\approved\\take.wav", newPath: "C:\\approved\\renamed.wav", renamed: true, fileAction: "renamed" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).renameRecording("take-1", "C:\\approved\\renamed.wav")).resolves.toMatchObject({ renamed: true });
    expect(received).toEqual({ method: "recordings.rename", params: { recordingId: "take-1", newPath: "C:\\approved\\renamed.wav" } });
  });

  it("forwards session lifecycle actions through the shared API", async () => {
    const received: unknown[] = [];
    const client = {
      request: async (method: string, params: unknown) => {
        received.push({ method, params });
        return method === "session.start"
          ? { sessionId: demoSession.id, state: "running", generation: 1, runtime: "fake" }
          : { sessionId: demoSession.id, state: "stopped", runtime: "fake" };
      },
    } as never;
    const backend = createLiveBackend(client, demoSession.id);
    await expect(backend.startSession(demoSession.id)).resolves.toMatchObject({ state: "running" });
    await expect(backend.stopSession(demoSession.id)).resolves.toMatchObject({ state: "stopped" });
    expect(received).toEqual([
      { method: "session.start", params: { sessionId: demoSession.id } },
      { method: "session.stop", params: { sessionId: demoSession.id } },
    ]);
  });

  it("forwards frame-accurate recorder actions with retry keys", async () => {
    const received: unknown[] = [];
    const client = {
      request: async (method: string, params: unknown) => {
        received.push({ method, params });
        return { sessionId: demoSession.id, state: method.endsWith("arm") ? "armed" : "recording", parts: [], pauses: [], lastFrame: 12 };
      },
    } as never;
    const backend = createLiveBackend(client, demoSession.id);
    await backend.armRecorder(demoSession.id, "arm-key");
    await backend.startRecorder(demoSession.id, 12, "start-key");
    expect(received).toEqual([
      { method: "recorders.arm", params: { sessionId: demoSession.id, idempotencyKey: "arm-key" } },
      { method: "recorders.start", params: { sessionId: demoSession.id, frame: 12, idempotencyKey: "start-key" } },
    ]);
  });

  it("forwards session export and stopped import planning through the shared API", async () => {
    const received: unknown[] = [];
    const client = {
      request: async (method: string, params: unknown) => {
        received.push({ method, params });
        if (method === "sessions.export") return demoSession;
        if (method === "sessions.importPlan") return { planId: "import-plan-1", expiresInMs: 300000, session: demoSession };
        return { session: demoSession, state: "stopped", imported: true };
      },
    } as never;
    const backend = createLiveBackend(client, demoSession.id);
    await expect(backend.exportSession(demoSession.id)).resolves.toEqual(demoSession);
    await expect(backend.planSessionImport(demoSession)).resolves.toMatchObject({ planId: "import-plan-1" });
    await expect(backend.commitSessionImport("import-plan-1", "import-key")).resolves.toMatchObject({ imported: true });
    expect(received).toEqual([
      { method: "sessions.export", params: { sessionId: demoSession.id } },
      { method: "sessions.importPlan", params: { session: demoSession } },
      { method: "sessions.importCommit", params: { planId: "import-plan-1", idempotencyKey: "import-key" } },
    ]);
  });

  it("creates a stopped session through the shared API", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { session: { ...demoSession, id: "new-session", name: "New", revision: 0 }, state: "stopped" }; } } as never;
    const candidate = { ...demoSession, id: "new-session", name: "New", revision: 0 };
    await expect(createLiveBackend(client, demoSession.id).createSession(candidate, "create-key")).resolves.toMatchObject({ state: "stopped" });
    expect(received).toEqual({ method: "sessions.create", params: { session: candidate, idempotencyKey: "create-key" } });
  });

  it("duplicates a stopped session through the shared API", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { session: { ...demoSession, id: "copy", name: "Copy", revision: 0 }, state: "stopped" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).duplicateSession(demoSession.id, "copy", "Copy", "duplicate-key")).resolves.toMatchObject({ state: "stopped" });
    expect(received).toEqual({ method: "sessions.duplicate", params: { sourceSessionId: demoSession.id, sessionId: "copy", name: "Copy", idempotencyKey: "duplicate-key" } });
  });

  it("deletes a session through the shared API", async () => {
    let received: unknown;
    const client = { request: async (method: string, params: unknown) => { received = { method, params }; return { deleted: true, state: "stopped" }; } } as never;
    await expect(createLiveBackend(client, demoSession.id).deleteSession("copy", "delete-key")).resolves.toEqual({ deleted: true, state: "stopped" });
    expect(received).toEqual({ method: "sessions.delete", params: { sessionId: "copy", idempotencyKey: "delete-key" } });
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
