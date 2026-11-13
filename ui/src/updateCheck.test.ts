import { describe, expect, it } from "vitest";
import {
  isNewer,
  newestUpdate,
  nextRetryPlan,
  parseVersion,
  resumeRetryPlan,
  retryDelayMs,
  sortReleasesNewestFirst,
  RETRY_BASE_MS,
  RETRY_WINDOW_MS,
  RELEASE_PAGE_PREFIX,
  CHECK_INTERVAL_MS,
} from "./updateCheck";

const installer = (version: string) => [{ name: `AudioRouter_${version}_x64-setup.exe` }];

describe("update check", () => {
  it("parses and compares three-part versions numerically", () => {
    expect(parseVersion("v0.0.10")).toEqual([0, 0, 10]);
    expect(parseVersion("0.1.0")).toEqual([0, 1, 0]);
    expect(parseVersion("v0.1.0-beta")).toBeNull();
    expect(isNewer("v0.0.10", "0.0.9")).toBe(true);
    expect(isNewer("v0.1.0", "0.0.99")).toBe(true);
    expect(isNewer("v0.0.8", "0.0.8")).toBe(false);
    expect(isNewer("v0.0.7", "0.0.8")).toBe(false);
    expect(isNewer("garbage", "0.0.8")).toBe(false);
  });

  it("picks the newest published release above the current version, prereleases included", () => {
    const releases = [
      { tag_name: "v0.0.9", draft: false, prerelease: true, assets: installer("0.0.9") },
      { tag_name: "v0.0.11", draft: true, prerelease: true, assets: installer("0.0.11") },
      { tag_name: "v0.0.10", draft: false, prerelease: true, assets: installer("0.0.10") },
      { tag_name: "v0.0.7", draft: false, prerelease: true, assets: installer("0.0.7") },
      { tag_name: "nightly", draft: false },
      null,
    ];
    expect(newestUpdate(releases, "0.0.8")).toEqual({
      version: "0.0.10",
      tag: "v0.0.10",
      url: `${RELEASE_PAGE_PREFIX}v0.0.10`,
    });
    expect(newestUpdate(releases, "0.0.10")).toBeNull();
    expect(newestUpdate({ message: "API rate limit exceeded" }, "0.0.8")).toBeNull();
  });

  it("ignores a release without the expected installer", () => {
    const releases = [
      { tag_name: "v0.0.12", draft: false, assets: [] },
      { tag_name: "v0.0.10", draft: false, assets: installer("0.0.10") },
    ];
    expect(newestUpdate(releases, "0.0.8")?.version).toBe("0.0.10");
    expect(newestUpdate(releases, "0.0.8", { requireInstaller: false })?.version).toBe("0.0.12");
  });

  it("can restrict the check to stable releases", () => {
    const releases = [
      { tag_name: "v0.0.11", draft: false, prerelease: true, assets: installer("0.0.11") },
      { tag_name: "v0.0.10", draft: false, prerelease: false, assets: installer("0.0.10") },
    ];
    expect(newestUpdate(releases, "0.0.8")?.version).toBe("0.0.11");
    expect(newestUpdate(releases, "0.0.8", { channel: "stable" })?.version).toBe("0.0.10");
  });

  it("looks at the newest releases first and only keeps the first page", () => {
    const dated = (version: string, day: number) => ({
      tag_name: `v0.0.${version}`,
      draft: false,
      published_at: `2026-10-${String(day).padStart(2, "0")}T00:00:00Z`,
      assets: installer(`0.0.${version}`),
    });
    const releases = [dated("10", 1), dated("9", 3), dated("11", 2)];
    expect(sortReleasesNewestFirst(releases).map((release) => (release as { tag_name: string }).tag_name)).toEqual([
      "v0.0.9",
      "v0.0.11",
      "v0.0.10",
    ]);
    // The highest tag still wins while it is inside the newest 20.
    expect(newestUpdate(releases, "0.0.8")?.version).toBe("0.0.11");

    const many = [...Array.from({ length: 20 }, (_, index) => dated(String(20 + index), 10)), dated("99", 1)];
    expect(newestUpdate(many, "0.0.8")?.version).toBe("0.0.39");
  });

  it("backs off exponentially for at most twelve hours", () => {
    expect(retryDelayMs(1)).toBe(RETRY_BASE_MS);
    expect(retryDelayMs(2)).toBe(2 * RETRY_BASE_MS);
    expect(retryDelayMs(3)).toBe(4 * RETRY_BASE_MS);
    expect(retryDelayMs(20)).toBe(RETRY_WINDOW_MS);
    expect(retryDelayMs(1, 60 * 60 * 1000)).toBe(60 * 60 * 1000);
  });

  it("plans retries and skips to the next day after the window", () => {
    const now = Date.UTC(2026, 10, 11, 12, 0, 0);
    const first = nextRetryPlan({ now, failureCount: 0, windowStartedAt: null });
    expect(first).toEqual({
      failureCount: 1,
      windowStartedAt: now,
      nextAttemptAt: now + RETRY_BASE_MS,
      retrying: true,
    });
    const later = nextRetryPlan({ now: now + 2 * RETRY_BASE_MS, failureCount: 1, windowStartedAt: now });
    expect(later.nextAttemptAt).toBe(now + 2 * RETRY_BASE_MS + 2 * RETRY_BASE_MS);
    expect(later.retrying).toBe(true);

    const ending = nextRetryPlan({
      now: now + RETRY_WINDOW_MS - 30_000,
      failureCount: 8,
      windowStartedAt: now,
    });
    expect(ending.retrying).toBe(true);
    expect(ending.nextAttemptAt).toBe(now + RETRY_WINDOW_MS);

    const afterWindow = nextRetryPlan({
      now: now + RETRY_WINDOW_MS,
      failureCount: 9,
      windowStartedAt: now,
    });
    expect(afterWindow.retrying).toBe(false);
    expect(afterWindow.nextAttemptAt).toBe(now + CHECK_INTERVAL_MS);
  });

  it("resumes a saved retry window without counting twice", () => {
    const now = Date.UTC(2026, 10, 11, 12, 0, 0);
    const resumed = resumeRetryPlan({
      now: now + 90_000,
      failureCount: 2,
      windowStartedAt: now,
      lastAttemptAt: now,
    });
    expect(resumed.failureCount).toBe(2);
    expect(resumed.retrying).toBe(true);
    const skipped = resumeRetryPlan({
      now: now + RETRY_WINDOW_MS,
      failureCount: 2,
      windowStartedAt: now,
      lastAttemptAt: now,
    });
    expect(skipped.retrying).toBe(false);
    expect(skipped.nextAttemptAt).toBe(now + CHECK_INTERVAL_MS);
  });
});
