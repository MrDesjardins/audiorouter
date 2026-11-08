/** @vitest-environment jsdom */
import { readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { scrollBehavior } from "./reducedMotion";

afterEach(() => vi.unstubAllGlobals());

describe("reduced motion", () => {
  it("scrolls instantly when the reader asks for reduced motion", () => {
    vi.stubGlobal("matchMedia", (query: string) => ({ matches: query.includes("reduce") }));
    expect(scrollBehavior()).toBe("auto");
    vi.stubGlobal("matchMedia", () => ({ matches: false }));
    expect(scrollBehavior()).toBe("smooth");
  });

  it("never hard-codes a smooth scripted scroll, which overrides the CSS rule", () => {
    const dir = resolve(__dirname);
    const offenders = readdirSync(dir)
      .filter((file) => /\.tsx?$/.test(file) && !/\.test\./.test(file))
      .filter((file) => /behavior:\s*"smooth"/.test(readFileSync(resolve(dir, file), "utf8")));
    expect(offenders).toEqual([]);
  });
});
