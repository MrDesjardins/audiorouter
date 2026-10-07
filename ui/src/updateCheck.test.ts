import { describe, expect, it } from "vitest";
import { isNewer, newestUpdate, parseVersion, RELEASE_PAGE_PREFIX } from "./updateCheck";

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
      { tag_name: "v0.0.9", draft: false, prerelease: true },
      { tag_name: "v0.0.11", draft: true, prerelease: true },
      { tag_name: "v0.0.10", draft: false, prerelease: true },
      { tag_name: "v0.0.7", draft: false, prerelease: true },
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
});
