// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { createDisconnectedBackend, SnapshotCache } from "./backend";
import { demoSession } from "./fixtures";
import { applyLanguage } from "./i18n";
import en from "./i18n/locales/en.json";
import { useWorkspaceEvents } from "./useWorkspaceEvents";

it("refreshes after a failed subscription even when recovery has no events", async () => {
  vi.useFakeTimers();
  const base = createDisconnectedBackend();
  const snapshot = await base.snapshot();
  const backend = {
    ...base,
    connected: true,
    snapshot: vi.fn().mockResolvedValue(snapshot),
    subscribe: vi
      .fn()
      .mockRejectedValueOnce(new Error("temporarily unavailable"))
      .mockResolvedValue({ backendEpoch: 1, nextSequence: 0, events: [], resyncRequired: false }),
  };
  const cache = new SnapshotCache();
  await cache.refresh(backend);
  backend.snapshot.mockClear();
  const update = vi.fn();
  const hook = renderHook(() =>
    useWorkspaceEvents({
      backend,
      hasSnapshot: true,
      session: demoSession,
      snapshotCache: cache,
      eventCursor: { current: { backendEpoch: 1, sequence: 0 } },
      setSnapshotState: update,
      followBackendActiveSession: () => false,
      setActionMessage: vi.fn(),
      refreshApplications: vi.fn(),
      refreshDevices: vi.fn(),
      refreshSessions: async () => [],
      setRecordings: vi.fn(),
      setRecordingsError: vi.fn(),
      setRecorderStatuses: vi.fn(),
      setRecorderStatusAvailable: vi.fn(),
    }),
  );
  try {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(update).toHaveBeenCalledWith(expect.any(Function));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(backend.snapshot).toHaveBeenCalledTimes(1);
    expect(update).toHaveBeenLastCalledWith({ snapshot, stale: false, error: null });
  } finally {
    hook.unmount();
    vi.useRealTimers();
  }
});

it("reports an automatic route recovery and its failure", async () => {
  vi.useFakeTimers();
  const base = createDisconnectedBackend();
  const snapshot = await base.snapshot();
  const event = (category: string) => ({ sequence: 1, category, revision: 0 });
  const backend = {
    ...base,
    connected: true,
    snapshot: vi.fn().mockResolvedValue(snapshot),
    subscribe: vi
      .fn()
      .mockResolvedValueOnce({
        backendEpoch: 1,
        nextSequence: 2,
        events: [event("devices.bindingInvalidated"), event("runtime.recovered")],
        resyncRequired: false,
      })
      .mockResolvedValueOnce({
        backendEpoch: 1,
        nextSequence: 3,
        events: [event("runtime.recoveryFailed")],
        resyncRequired: false,
      })
      .mockResolvedValueOnce({
        backendEpoch: 1,
        nextSequence: 5,
        events: [event("runtime.recoveryFailed"), event("runtime.recovered")],
        resyncRequired: false,
      })
      .mockResolvedValue({ backendEpoch: 1, nextSequence: 6, events: [], resyncRequired: false }),
  };
  const cache = new SnapshotCache();
  await cache.refresh(backend);
  const message = vi.fn();
  const hook = renderHook(() =>
    useWorkspaceEvents({
      backend,
      hasSnapshot: true,
      session: demoSession,
      snapshotCache: cache,
      eventCursor: { current: { backendEpoch: 1, sequence: 0 } },
      setSnapshotState: vi.fn(),
      followBackendActiveSession: () => false,
      setActionMessage: message,
      refreshApplications: vi.fn(),
      refreshDevices: vi.fn(),
      refreshSessions: async () => [],
      setRecordings: vi.fn(),
      setRecordingsError: vi.fn(),
      setRecorderStatuses: vi.fn(),
      setRecorderStatusAvailable: vi.fn(),
    }),
  );
  try {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(message).toHaveBeenLastCalledWith(
      expect.objectContaining({ text: expect.stringContaining("restarted the route"), tone: "info" }),
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(message).toHaveBeenLastCalledWith(
      expect.objectContaining({ text: expect.stringContaining("could not restart the route"), tone: "error" }),
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(message).toHaveBeenLastCalledWith(
      expect.objectContaining({ text: expect.stringContaining("restarted the route"), tone: "info" }),
    );
  } finally {
    hook.unmount();
    vi.useRealTimers();
  }
});

it("names the bus in a virtual cable notice, keeps its severity and follows the language", async () => {
  vi.useFakeTimers();
  const base = createDisconnectedBackend();
  const snapshot = await base.snapshot();
  const event = (category: string, operationId?: string) => ({ sequence: 1, category, revision: 0, operationId });
  const backend = {
    ...base,
    connected: true,
    snapshot: vi.fn().mockResolvedValue(snapshot),
    subscribe: vi
      .fn()
      .mockResolvedValueOnce({
        backendEpoch: 1,
        nextSequence: 2,
        events: [event("virtualBridge.expired", "voice-bus")],
        resyncRequired: false,
      })
      .mockResolvedValueOnce({
        backendEpoch: 1,
        nextSequence: 3,
        events: [event("virtualBridge.failed")],
        resyncRequired: false,
      })
      .mockResolvedValueOnce({
        backendEpoch: 1,
        nextSequence: 4,
        events: [event("devices.bindingInvalidated")],
        resyncRequired: false,
      })
      .mockResolvedValue({ backendEpoch: 1, nextSequence: 5, events: [], resyncRequired: false }),
  };
  const cache = new SnapshotCache();
  await cache.refresh(backend);
  const message = vi.fn();
  const hook = renderHook(() =>
    useWorkspaceEvents({
      backend,
      hasSnapshot: true,
      session: demoSession,
      snapshotCache: cache,
      eventCursor: { current: { backendEpoch: 1, sequence: 0 } },
      setSnapshotState: vi.fn(),
      followBackendActiveSession: () => false,
      setActionMessage: message,
      refreshApplications: vi.fn(),
      refreshDevices: vi.fn(),
      refreshSessions: async () => [],
      setRecordings: vi.fn(),
      setRecordingsError: vi.fn(),
      setRecorderStatuses: vi.fn(),
      setRecorderStatusAvailable: vi.fn(),
    }),
  );
  try {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(message).toHaveBeenLastCalledWith(
      expect.objectContaining({
        text: expect.stringContaining("link for voice-bus timed out"),
        tone: "error",
        key: "workspaceEvents.bridgeTimedOut",
      }),
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(message).toHaveBeenLastCalledWith(
      expect.objectContaining({
        text: expect.stringContaining("link for an affected bus failed"),
        tone: "error",
        key: "workspaceEvents.bridgeFailedUnnamed",
      }),
    );
    await applyLanguage("fr");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    const device = message.mock.lastCall?.[0] as { text: string; tone: string } | undefined;
    expect(device?.tone).toBe("error");
    expect(device?.text).toContain("appuyez sur Lecture");
    expect(device?.text).not.toBe(en.messages.workspaceEvents.deviceChanged);
  } finally {
    await applyLanguage("en");
    hook.unmount();
    vi.useRealTimers();
  }
});

it("does not poll while a user reconnect is running, so it cannot supersede that snapshot", async () => {
  vi.useFakeTimers();
  const base = createDisconnectedBackend();
  const snapshot = await base.snapshot();
  const backend = {
    ...base,
    connected: true,
    snapshot: vi.fn().mockResolvedValue(snapshot),
    subscribe: vi.fn().mockResolvedValue({ backendEpoch: 1, nextSequence: 0, events: [], resyncRequired: true }),
  };
  const cache = new SnapshotCache();
  const hook = renderHook(
    ({ reconnecting }) =>
      useWorkspaceEvents({
        backend,
        hasSnapshot: true,
        session: demoSession,
        snapshotCache: cache,
        eventCursor: { current: { backendEpoch: 1, sequence: 0 } },
        setSnapshotState: vi.fn(),
        followBackendActiveSession: () => false,
        setActionMessage: vi.fn(),
        refreshApplications: vi.fn(),
        refreshDevices: vi.fn(),
        refreshSessions: async () => [],
        setRecordings: vi.fn(),
        setRecordingsError: vi.fn(),
        setRecorderStatuses: vi.fn(),
        setRecorderStatusAvailable: vi.fn(),
        reconnecting,
      }),
    { initialProps: { reconnecting: true } },
  );
  try {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3000);
    });
    expect(backend.subscribe).not.toHaveBeenCalled();
    hook.rerender({ reconnecting: false });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(backend.subscribe).toHaveBeenCalled();
  } finally {
    hook.unmount();
    vi.useRealTimers();
  }
});

it("discards an in-flight subscription when reconnect starts before its response", async () => {
  vi.useFakeTimers();
  const base = createDisconnectedBackend();
  const snapshot = await base.snapshot();
  let finish!: (value: Awaited<ReturnType<typeof base.subscribe>>) => void;
  const backend = {
    ...base,
    connected: true,
    snapshot: vi.fn().mockResolvedValue(snapshot),
    subscribe: vi.fn(
      () =>
        new Promise<Awaited<ReturnType<typeof base.subscribe>>>((resolve) => {
          finish = resolve;
        }),
    ),
  };
  const update = vi.fn();
  const cache = new SnapshotCache();
  const cursor = { current: { backendEpoch: 1, sequence: 0 } };
  const hook = renderHook(
    ({ reconnecting }) =>
      useWorkspaceEvents({
        backend,
        hasSnapshot: true,
        session: demoSession,
        snapshotCache: cache,
        eventCursor: cursor,
        setSnapshotState: update,
        followBackendActiveSession: () => false,
        setActionMessage: vi.fn(),
        refreshApplications: vi.fn(),
        refreshDevices: vi.fn(),
        refreshSessions: async () => [],
        setRecordings: vi.fn(),
        setRecordingsError: vi.fn(),
        setRecorderStatuses: vi.fn(),
        setRecorderStatusAvailable: vi.fn(),
        reconnecting,
      }),
    { initialProps: { reconnecting: false } },
  );
  try {
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(backend.subscribe).toHaveBeenCalledTimes(1);
    hook.rerender({ reconnecting: true });
    await act(async () => {
      finish({ backendEpoch: 1, nextSequence: 1, events: [], resyncRequired: true });
    });
    expect(backend.snapshot).not.toHaveBeenCalled();
    expect(update).not.toHaveBeenCalled();
  } finally {
    hook.unmount();
    vi.useRealTimers();
  }
});
