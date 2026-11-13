/** @vitest-environment jsdom */
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  APP_VERSION,
  CHECK_INTERVAL_MS,
  RELEASES_API,
  RETRY_BASE_MS,
  RETRY_WINDOW_MS,
  useUpdateCheck,
} from "./updateCheck";

const cacheKey = "audiorouter.ui.update-check";
const start = Date.UTC(2026, 10, 11, 12, 0, 0);

function response(payload: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => payload,
    headers: { get: () => null },
  } as unknown as Response;
}

function release(version: string, prerelease = true) {
  return {
    tag_name: `v${version}`,
    draft: false,
    prerelease,
    published_at: "2026-11-10T00:00:00Z",
    assets: [{ name: `AudioRouter_${version}_x64-setup.exe` }],
  };
}

function writeCache(values: Record<string, unknown> = {}) {
  localStorage.setItem(
    cacheKey,
    JSON.stringify({
      checkedAt: start,
      forVersion: APP_VERSION,
      channel: "prerelease",
      update: null,
      lastAttemptAt: start,
      failureCount: 0,
      windowStartedAt: null,
      failure: null,
      retryAfterMs: 0,
      ...values,
    }),
  );
}

async function settle() {
  await act(async () => {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
  });
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(start));
  localStorage.clear();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("useUpdateCheck", () => {
  it("uses a fresh cached answer without a request", async () => {
    writeCache({ checkedAt: start });
    const fetcher = vi.fn(async () => response([]));
    const { result } = renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    expect(result.current.status).toBe("current");
    expect(result.current.nextCheckAt).toBe(start + CHECK_INTERVAL_MS);
    await settle();
    expect(fetcher).not.toHaveBeenCalled();
  });

  it("reads the public API without credentials and pins its version", async () => {
    const fetcher = vi.fn(async () => response([]));
    renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    await settle();
    expect(fetcher).toHaveBeenCalledWith(
      RELEASES_API,
      expect.objectContaining({
        credentials: "omit",
        cache: "no-store",
        referrerPolicy: "no-referrer",
        headers: expect.objectContaining({
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
        }),
      }),
    );
  });

  it("checks a stale cache and shows the newest installer release", async () => {
    writeCache({ checkedAt: start - CHECK_INTERVAL_MS - 1 });
    const fetcher = vi.fn(async () => response([release("0.0.28"), release("0.0.27")]));
    const { result } = renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    expect(result.current.status).toBe("checking");
    await settle();
    expect(result.current.status).toBe("available");
    expect(result.current.update?.version).toBe("0.0.28");
    expect(result.current.nextCheckAt).toBe(start + CHECK_INTERVAL_MS);
    expect(result.current.failure).toBeNull();
    expect(fetcher).toHaveBeenCalledTimes(1);
  });

  it("retries a failed check with exponential backoff and then skips a day", async () => {
    const fetcher = vi.fn(async () => {
      throw new TypeError("Failed to fetch");
    });
    const { result } = renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    await settle();
    expect(result.current.status).toBe("failed");
    expect(result.current.failure).toBe("failed");
    expect(result.current.retrying).toBe(true);
    expect(result.current.nextCheckAt).toBe(start + RETRY_BASE_MS);

    await act(async () => {
      vi.advanceTimersByTime(RETRY_BASE_MS);
    });
    await settle();
    expect(fetcher).toHaveBeenCalledTimes(2);
    expect(result.current.nextCheckAt).toBe(start + RETRY_BASE_MS + 2 * RETRY_BASE_MS);

    // A later attempt after the 12-hour window waits for the next day.
    vi.setSystemTime(new Date(start + RETRY_WINDOW_MS));
    await act(async () => {
      window.dispatchEvent(new Event("focus"));
    });
    await settle();
    expect(result.current.retrying).toBe(false);
    expect(result.current.nextCheckAt).toBe(start + CHECK_INTERVAL_MS);
  });

  it("clears the failure after a later success", async () => {
    const fetcher = vi
      .fn()
      .mockRejectedValueOnce(new TypeError("Failed to fetch"))
      .mockResolvedValueOnce(response([release("0.0.28")]));
    const { result } = renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    await settle();
    expect(result.current.status).toBe("failed");
    await act(async () => {
      vi.advanceTimersByTime(RETRY_BASE_MS);
    });
    await settle();
    expect(result.current.status).toBe("available");
    expect(result.current.failure).toBeNull();
    expect(result.current.failureCount).toBe(0);
    expect(result.current.nextCheckAt).toBe(start + RETRY_BASE_MS + CHECK_INTERVAL_MS);
  });

  it("checks immediately on demand, even with a fresh cache", async () => {
    writeCache({ checkedAt: start });
    const fetcher = vi.fn(async () => response([release("0.0.28")]));
    const { result } = renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    expect(fetcher).not.toHaveBeenCalled();
    act(() => result.current.checkNow());
    await settle();
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(result.current.status).toBe("available");
  });

  it("checks again after the connection returns", async () => {
    const fetcher = vi
      .fn()
      .mockRejectedValueOnce(new TypeError("Failed to fetch"))
      .mockResolvedValueOnce(response([release("0.0.28")]));
    const { result } = renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    await settle();
    expect(result.current.status).toBe("failed");
    await act(async () => {
      window.dispatchEvent(new Event("online"));
    });
    await settle();
    expect(fetcher).toHaveBeenCalledTimes(2);
    expect(result.current.status).toBe("available");
  });

  it("checks on focus when the day is due even if the timer was suspended", async () => {
    writeCache({ checkedAt: start });
    const fetcher = vi.fn(async () => response([]));
    const { result } = renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    expect(fetcher).not.toHaveBeenCalled();
    vi.setSystemTime(new Date(start + CHECK_INTERVAL_MS + 1));
    await act(async () => {
      window.dispatchEvent(new Event("focus"));
    });
    await settle();
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(result.current.status).toBe("current");
  });

  it("checks every day while the window stays open", async () => {
    writeCache({ checkedAt: start });
    const fetcher = vi.fn(async () => response([]));
    renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    expect(fetcher).not.toHaveBeenCalled();
    await act(async () => {
      vi.advanceTimersByTime(CHECK_INTERVAL_MS);
    });
    await settle();
    expect(fetcher).toHaveBeenCalledTimes(1);
  });

  it("applies the release-channel preference and rechecks on change", async () => {
    const fetcher = vi.fn(async () => response([release("0.0.28", true), release("0.0.27", false)]));
    const { result, rerender } = renderHook(
      ({ channel }: { channel: "prerelease" | "stable" }) => useUpdateCheck(true, channel, fetcher),
      { initialProps: { channel: "prerelease" } },
    );
    await settle();
    expect(result.current.update?.version).toBe("0.0.28");
    rerender({ channel: "stable" });
    await settle();
    expect(fetcher).toHaveBeenCalledTimes(2);
    expect(result.current.update?.version).toBe("0.0.27");
  });

  it("resumes a saved retry window without an extra request", async () => {
    writeCache({
      checkedAt: start - 25 * 60 * 60 * 1000,
      failureCount: 2,
      windowStartedAt: start - 2 * 60 * 1000,
      lastAttemptAt: start - 10_000,
      failure: "failed",
    });
    const fetcher = vi.fn(async () => response([]));
    const { result } = renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    await settle();
    expect(fetcher).not.toHaveBeenCalled();
    expect(result.current.status).toBe("failed");
    expect(result.current.retrying).toBe(true);
    expect(result.current.nextCheckAt).toBe(start + 2 * 60 * 1000 - 10_000);
  });

  it("abandons an in-flight result when checking is turned off", async () => {
    let resolveFetch!: (value: Response) => void;
    const fetcher = vi.fn(
      () =>
        new Promise<Response>((resolve) => {
          resolveFetch = resolve;
        }),
    );
    const { result, rerender } = renderHook(
      ({ enabled }: { enabled: boolean }) => useUpdateCheck(enabled, "prerelease", fetcher),
      { initialProps: { enabled: true } },
    );
    await settle();
    expect(result.current.status).toBe("checking");
    rerender({ enabled: false });
    expect(result.current.status).toBe("off");
    await act(async () => {
      resolveFetch(response([release("0.0.28")]));
      await Promise.resolve();
    });
    await settle();
    expect(result.current.status).toBe("off");
    expect(localStorage.getItem(cacheKey)).toBeNull();
  });

  it("starts a fresh check after a quick off/on toggle", async () => {
    let resolveFirst!: (value: Response) => void;
    const fetcher = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise<Response>((resolve) => {
            resolveFirst = resolve;
          }),
      )
      .mockResolvedValueOnce(response([release("0.0.28")]));
    const { result, rerender } = renderHook(
      ({ enabled }: { enabled: boolean }) => useUpdateCheck(enabled, "prerelease", fetcher),
      { initialProps: { enabled: true } },
    );
    await settle();
    expect(result.current.status).toBe("checking");
    rerender({ enabled: false });
    rerender({ enabled: true });
    await settle();
    expect(fetcher).toHaveBeenCalledTimes(2);
    expect(result.current.status).toBe("available");
    await act(async () => {
      resolveFirst(response([release("0.0.29")]));
      await Promise.resolve();
    });
    await settle();
    expect(result.current.update?.version).toBe("0.0.28");
  });

  it("honours a rate-limit retry delay", async () => {
    const fetcher = vi.fn(
      async () =>
        ({
          ok: false,
          status: 429,
          json: async () => ({}),
          headers: {
            get: (name: string) => (name === "retry-after" ? "3600" : null),
          },
        }) as unknown as Response,
    );
    const { result } = renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    await settle();
    expect(result.current.status).toBe("failed");
    expect(result.current.failure).toBe("rate-limited");
    expect(result.current.nextCheckAt).toBe(start + 60 * 60 * 1000);
  });

  it("reports an offline failure without a request", async () => {
    const onLine = vi.spyOn(navigator, "onLine", "get").mockReturnValue(false);
    try {
      const fetcher = vi.fn(async () => response([]));
      const { result } = renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
      await settle();
      expect(fetcher).not.toHaveBeenCalled();
      expect(result.current.status).toBe("failed");
      expect(result.current.failure).toBe("offline");
      expect(result.current.nextCheckAt).toBe(start + RETRY_BASE_MS);
    } finally {
      onLine.mockRestore();
    }
  });
});
