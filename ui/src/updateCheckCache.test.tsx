/** @vitest-environment jsdom */
import { cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { APP_VERSION, RELEASE_PAGE_PREFIX, useUpdateCheck } from "./updateCheck";

const key = "audiorouter.ui.update-check";
const fetcher = vi.fn(() => new Promise<Response>(() => {}));
beforeEach(() => {
  localStorage.clear();
  fetcher.mockClear();
});
afterEach(cleanup);

describe("update cache validation", () => {
  it("ignores an update without a tag without crashing startup", () => {
    localStorage.setItem(key, JSON.stringify({ checkedAt: Date.now(), forVersion: APP_VERSION, update: {} }));
    const { result } = renderHook(() => useUpdateCheck(true, fetcher));
    expect(result.current).toBeNull();
  });

  it("reconstructs the release version and URL from the validated tag", () => {
    localStorage.setItem(
      key,
      JSON.stringify({
        checkedAt: Date.now(),
        forVersion: APP_VERSION,
        update: { tag: "v999.0.0", version: "injected", url: "javascript:alert(1)" },
      }),
    );
    const { result } = renderHook(() => useUpdateCheck(true, fetcher));
    expect(result.current).toEqual({ tag: "v999.0.0", version: "999.0.0", url: `${RELEASE_PAGE_PREFIX}v999.0.0` });
    expect(fetcher).not.toHaveBeenCalled();
  });
});
