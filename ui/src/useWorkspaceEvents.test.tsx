// @vitest-environment jsdom
import { act, renderHook } from "@testing-library/react";
import { expect, it, vi } from "vitest";
import { createDisconnectedBackend, SnapshotCache } from "./backend";
import { demoSession } from "./fixtures";
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
      .mockResolvedValue({
        backendEpoch: 1,
        nextSequence: 3,
        events: [event("runtime.recoveryFailed")],
        resyncRequired: false,
      }),
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
    expect(message).toHaveBeenLastCalledWith(expect.stringContaining("restarted the route"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1000);
    });
    expect(message).toHaveBeenLastCalledWith(expect.stringContaining("could not restart the route"));
  } finally {
    hook.unmount();
    vi.useRealTimers();
  }
});
