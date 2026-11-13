/** @vitest-environment jsdom */
import { cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  APP_VERSION,
  RELEASE_PAGE_PREFIX,
  clearUpdateCache,
  readUpdateChannel,
  setUpdateCheckEnabled,
  setUpdateChannel,
  updateCheckEnabled,
  useUpdateCheck,
} from "./updateCheck";

const key = "audiorouter.ui.update-check";
const fetcher = vi.fn(() => new Promise<Response>(() => {}));
beforeEach(() => {
  localStorage.clear();
  fetcher.mockClear();
});
afterEach(cleanup);

describe("update cache validation", () => {
  it("ignores an update without a tag without crashing startup", () => {
    localStorage.setItem(
      key,
      JSON.stringify({ checkedAt: Date.now(), forVersion: APP_VERSION, channel: "prerelease", update: {} }),
    );
    const { result } = renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    expect(result.current.update).toBeNull();
    expect(result.current.status).toBe("checking");
  });

  it("reconstructs the release version and URL from the validated tag", () => {
    localStorage.setItem(
      key,
      JSON.stringify({
        checkedAt: Date.now(),
        forVersion: APP_VERSION,
        channel: "prerelease",
        update: { tag: "v999.0.0", version: "injected", url: "javascript:alert(1)" },
        failureCount: 0,
      }),
    );
    const { result } = renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    expect(result.current.update).toEqual({
      tag: "v999.0.0",
      version: "999.0.0",
      url: `${RELEASE_PAGE_PREFIX}v999.0.0`,
    });
    expect(fetcher).not.toHaveBeenCalled();
  });

  it("clamps a cached time that is in the future instead of trusting it", () => {
    const now = Date.now();
    localStorage.setItem(
      key,
      JSON.stringify({
        checkedAt: now + 48 * 60 * 60 * 1000,
        forVersion: APP_VERSION,
        channel: "prerelease",
        update: null,
        failureCount: 0,
      }),
    );
    const { result } = renderHook(() => useUpdateCheck(true, "prerelease", fetcher));
    expect(result.current.status).toBe("current");
    expect(fetcher).not.toHaveBeenCalled();
  });

  it("forgets the cached result when checking is turned off", () => {
    localStorage.setItem(key, JSON.stringify({ checkedAt: Date.now(), forVersion: APP_VERSION, update: null }));
    setUpdateCheckEnabled(false);
    expect(updateCheckEnabled()).toBe(false);
    expect(localStorage.getItem(key)).toBeNull();
    setUpdateCheckEnabled(true);
    expect(updateCheckEnabled()).toBe(true);
  });

  it("stores a channel preference and defaults to prereleases", () => {
    expect(readUpdateChannel()).toBe("prerelease");
    setUpdateChannel("stable");
    expect(readUpdateChannel()).toBe("stable");
  });

  it("clears the cache explicitly", () => {
    localStorage.setItem(key, JSON.stringify({ checkedAt: Date.now(), forVersion: APP_VERSION, update: null }));
    clearUpdateCache();
    expect(localStorage.getItem(key)).toBeNull();
  });
});
