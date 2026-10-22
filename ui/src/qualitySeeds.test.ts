import { afterEach, expect, it, vi } from "vitest";
import { qualitySeeds } from "./qualitySeeds";
afterEach(() => vi.unstubAllEnvs());
it("keeps fixed seeds and adds a reproducible bounded nightly seed", () => {
  vi.stubEnv("AUDIOROUTER_TEST_SEED", undefined);
  expect(qualitySeeds([47, 997])).toEqual([47, 997]);
  vi.stubEnv("AUDIOROUTER_TEST_SEED", "39710446637");
  expect(qualitySeeds([47, 997])).toEqual([47, 997, 39710446637 >>> 0]);
  vi.stubEnv("AUDIOROUTER_TEST_SEED", "47");
  expect(qualitySeeds([47, 997])).toEqual([47, 997]);
});
it("rejects invalid seed configuration rather than claiming a generated run", () => {
  for (const seed of ["", "oops", "-1", "1.2", "9007199254740992"]) {
    vi.stubEnv("AUDIOROUTER_TEST_SEED", seed);
    expect(() => qualitySeeds([47])).toThrow("Invalid AUDIOROUTER_TEST_SEED");
  }
});
