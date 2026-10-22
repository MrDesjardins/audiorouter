/** Test-only seed selection. Fixed PR seeds remain even when nightly adds one. */
export function qualitySeeds(fixed: number[]): number[] {
  const value = process.env.AUDIOROUTER_TEST_SEED;
  if (value === undefined) return fixed;
  if (!/^\d+$/.test(value) || !Number.isSafeInteger(Number(value))) throw Error("Invalid AUDIOROUTER_TEST_SEED");
  return [...new Set([...fixed, Number(value) >>> 0])];
}
