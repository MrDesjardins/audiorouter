/** @vitest-environment jsdom */
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ObsStatus } from "@audiorouter/contracts";
import { OBS_STATUS_BUSY_POLL_MS, useObsStatus, type ObsBackend } from "./useObsStatus";

const status: ObsStatus = {
  installed: true,
  connection: "connected",
  source: "live",
  sceneCollection: "Streaming",
  inputs: [],
  passwordSource: null,
  lastChange: { state: "none", changed: 0, restorable: false, error: null },
};

function backend(): Required<ObsBackend> {
  return { obsStatus: vi.fn(async () => status), obsRefresh: vi.fn(async () => ({ queued: true as const })) };
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.useRealTimers();
});

describe("useObsStatus", () => {
  it("asks OBS for fresh facts periodically without a focus change", async () => {
    vi.useFakeTimers();
    const obs = backend();
    renderHook(() => useObsStatus(obs, { active: true }));
    await act(async () => {});
    expect(obs.obsRefresh).toHaveBeenCalledTimes(1);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10_000);
    });
    expect(vi.mocked(obs.obsRefresh).mock.calls.length).toBeGreaterThan(1);
  });

  it("keeps one status request in flight", async () => {
    vi.useFakeTimers();
    const obs = backend();
    let resolve!: (value: ObsStatus) => void;
    vi.mocked(obs.obsStatus).mockReturnValue(
      new Promise((done) => {
        resolve = done;
      }),
    );
    renderHook(() => useObsStatus(obs, { active: true }));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(8_000);
    });
    expect(obs.obsStatus).toHaveBeenCalledTimes(1);
    await act(async () => {
      resolve(status);
    });
  });

  it("cancels delayed polls on unmount", async () => {
    vi.useFakeTimers();
    const obs = backend();
    const { result, unmount } = renderHook(() => useObsStatus(obs, { active: true }));
    await act(async () => {});
    const calls = vi.mocked(obs.obsStatus).mock.calls.length;
    const oldPoll = result.current.poll;
    unmount();
    oldPoll();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(OBS_STATUS_BUSY_POLL_MS);
    });
    expect(obs.obsStatus).toHaveBeenCalledTimes(calls);
  });

  it("retains the last good facts but marks a failed read stale", async () => {
    const obs = backend();
    const { result } = renderHook(() => useObsStatus(obs, { active: false }));
    await act(async () => {});
    vi.mocked(obs.obsStatus).mockRejectedValueOnce(new Error("offline"));
    await act(async () => {
      result.current.poll();
    });
    expect(result.current.status).toBe(status);
    expect(result.current.stale).toBe(true);
  });

  it("does not poll or inspect while the window is hidden", async () => {
    vi.useFakeTimers();
    const obs = backend();
    renderHook(() => useObsStatus(obs, { active: true }));
    await act(async () => {});
    const calls = vi.mocked(obs.obsStatus).mock.calls.length;
    vi.spyOn(document, "visibilityState", "get").mockReturnValue("hidden");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(30_000);
    });
    expect(obs.obsStatus).toHaveBeenCalledTimes(calls);
    expect(obs.obsRefresh).toHaveBeenCalledTimes(1);
  });

  it("ignores late answers and old callbacks after switching backend", async () => {
    const old = backend();
    let resolve!: (value: ObsStatus) => void;
    vi.mocked(old.obsStatus).mockReturnValue(
      new Promise((done) => {
        resolve = done;
      }),
    );
    const { result, rerender } = renderHook(({ obs }) => useObsStatus(obs, { active: false }), {
      initialProps: { obs: old },
    });
    const oldPoll = result.current.poll;
    const next = backend();
    vi.mocked(next.obsStatus).mockResolvedValue({ ...status, sceneCollection: "New" });
    rerender({ obs: next });
    await act(async () => {
      resolve({ ...status, sceneCollection: "Old" });
    });
    oldPoll();
    expect(result.current.status?.sceneCollection).toBe("New");
    expect(old.obsStatus).toHaveBeenCalledTimes(1);
  });
});
