import { describe, expect, it } from "vitest";
import type { Node } from "@audiorouter/contracts";
import {
  compressorOutputDb,
  compressorReductionDb,
  gateOutputDb,
  levelStatistics,
  nextFallingLevel,
  nextPeakHold,
  ratioForOutputAtFullScale,
  responseSketch,
  suggestCompressorThreshold,
  suggestGateThreshold,
  PEAK_FALL_DB_PER_SECOND,
  PEAK_HOLD_MS,
  type LevelSample,
} from "./dynamics";

const node = (kind: Node["kind"], parameters: Node["parameters"]): Node => ({
  id: kind,
  kind,
  typeVersion: 1,
  name: kind,
  enabled: true,
  bypass: false,
  parameters,
  ports: [
    { name: "in", direction: "input", channels: 1 },
    { name: "out", direction: "output", channels: 1 },
  ],
});

describe("dynamics curves mirror the DSP reference vectors", () => {
  it("compressor reduction with knee (crates/dsp compressor_transfer_curve_reference_vectors)", () => {
    const settings = { thresholdDb: -18, ratio: 3, kneeDb: 6 };
    expect(compressorReductionDb(-30, settings)).toBe(0);
    expect(compressorReductionDb(-18, settings)).toBeCloseTo(0.5, 6);
    expect(compressorReductionDb(-19.5, settings)).toBeCloseTo(0.125, 6);
    expect(compressorReductionDb(-12, settings)).toBeCloseTo(4, 6);
    expect(compressorReductionDb(0, settings)).toBeCloseTo(12, 6);
    expect(compressorOutputDb(0, { ...settings, attackMs: 10, releaseMs: 100, makeupDb: 3 })).toBeCloseTo(-9, 6);
  });
  it("gate static curve (crates/dsp gate_transfer_curve_reference_vectors)", () => {
    const settings = { thresholdDb: -45, ratio: 4, rangeDb: 60 };
    expect(gateOutputDb(-45, settings)).toBe(-45);
    expect(gateOutputDb(-50, settings)).toBeCloseTo(-65, 6);
    expect(gateOutputDb(-100, settings)).toBeCloseTo(-160, 6);
  });
  it("converts a dragged 0 dBFS output point into a ratio", () => {
    expect(ratioForOutputAtFullScale(-20, -15)).toBeCloseTo(4, 6);
    expect(ratioForOutputAtFullScale(-20, 0)).toBe(1);
    expect(ratioForOutputAtFullScale(-20, -40)).toBeCloseTo(20, 6);
  });
});

describe("response sketch", () => {
  it("shows a longer release as a slower return to unity", () => {
    const fast = responseSketch(
      "compressor",
      node("compressor", { thresholdDb: -20, ratio: 4, attackMs: 1, releaseMs: 20 }),
    );
    const slow = responseSketch(
      "compressor",
      node("compressor", { thresholdDb: -20, ratio: 4, attackMs: 1, releaseMs: 400 }),
    );
    const at = (sketch: typeof fast, ms: number) => sketch.points.find((point) => point.ms >= ms)!.gainDb;
    expect(at(fast, 390)).toBeLessThan(-8); // 12 dB over at 4:1 → 9 dB reduction
    expect(at(fast, 520)).toBeGreaterThan(-0.5);
    expect(at(slow, 520)).toBeLessThan(at(fast, 520) - 1.5);
  });
  it("keeps a gate open through Hold, then releases toward Range", () => {
    const sketch = responseSketch(
      "gate",
      node("gate", {
        thresholdDb: -40,
        rangeDb: 40,
        hysteresisDb: 3,
        ratio: 20,
        attackMs: 1,
        holdMs: 200,
        releaseMs: 20,
      }),
    );
    const at = (ms: number) => sketch.points.find((point) => point.ms >= ms)!.gainDb;
    expect(at(80)).toBeCloseTo(-40, 0);
    expect(at(300)).toBeGreaterThan(-0.1);
    expect(at(550)).toBeGreaterThan(-0.1); // hold ends at 600 ms
    expect(at(750)).toBeLessThan(-30);
  });
});

describe("level statistics and suggestions", () => {
  const samples = (levels: number[]): LevelSample[] =>
    levels.map((inputDb, index) => ({ at: index * 50, inputDb, outputDb: inputDb, reductionDb: 0, open: null }));
  it("needs enough material and a usable gap", () => {
    expect(levelStatistics(samples(Array(20).fill(-30)))).toBeNull();
    expect(levelStatistics(samples(Array(100).fill(-30)))).toBeNull();
  });
  it("finds room noise and voice and suggests thresholds between them", () => {
    const stats = levelStatistics(samples([...Array(60).fill(-62), ...Array(40).fill(-18)]))!;
    expect(stats).toEqual({ noiseDb: -62, voiceDb: -18 });
    expect(suggestGateThreshold(stats)).toBe(-47);
    const compressor = suggestCompressorThreshold(stats, 3); // 9 dB over at 3:1
    expect(compressor.thresholdDb).toBe(-27);
    expect(compressor.reductionDb).toBeCloseTo(6);
    expect(suggestGateThreshold({ noiseDb: -90, voiceDb: -20 })).toBeGreaterThanOrEqual(-80);
  });
  it("keeps the gate's closing point clear of room noise without shutting out quiet words", () => {
    expect(suggestGateThreshold({ noiseDb: -62, voiceDb: -18 }, 3)).toBe(-47); // closes at −50, 12 dB above noise
    expect(suggestGateThreshold({ noiseDb: -60, voiceDb: -30 }, 6)).toBe(-48); // a third would close only 4 dB above noise
    expect(suggestGateThreshold({ noiseDb: -40, voiceDb: -28 }, 12)).toBe(-34); // never above the noise-to-voice midpoint
  });
  it("measures the voice from speech alone, however briefly the person talks", () => {
    const stats = levelStatistics(samples([...Array(92).fill(-62), ...Array(8).fill(-18)]))!;
    expect(stats).toEqual({ noiseDb: -62, voiceDb: -18 });
    expect(levelStatistics(samples([...Array(95).fill(-62), ...Array(5).fill(-18)]))).toBeNull();
  });
  it("keeps a compressor threshold near the voice peaks instead of chasing a gentle ratio into the noise", () => {
    // 1.5:1 would need 18 dB over for 6 dB; the suggestion stops 12 dB under the peaks and reports 4 dB.
    const gentle = suggestCompressorThreshold({ noiseDb: -62, voiceDb: -18 }, 1.5);
    expect(gentle.thresholdDb).toBe(-30);
    expect(gentle.reductionDb).toBeCloseTo(4);
    // A small noise-to-voice gap keeps the threshold in its upper half.
    expect(suggestCompressorThreshold({ noiseDb: -40, voiceDb: -24 }, 2).thresholdDb).toBe(-32);
    expect(suggestCompressorThreshold({ noiseDb: -62, voiceDb: -18 }, 1).reductionDb).toBe(0);
  });
});

describe("display ballistics", () => {
  it("holds a peak, then falls at a fixed rate but not below the reading", () => {
    let hold = nextPeakHold(null, -6, 0);
    hold = nextPeakHold(hold, -30, 500);
    expect(hold.db).toBe(-6);
    hold = nextPeakHold(hold, -30, PEAK_HOLD_MS + 500);
    expect(hold.db).toBeCloseTo(-6 - PEAK_FALL_DB_PER_SECOND * 0.5, 6);
    hold = nextPeakHold(hold, -30, PEAK_HOLD_MS + 5000);
    expect(hold.db).toBe(-30);
    expect(nextPeakHold(hold, -3, PEAK_HOLD_MS + 5050).db).toBe(-3);
  });
  it("lets a level bar rise at once and fall smoothly", () => {
    const level = nextFallingLevel(nextFallingLevel(null, -10, 0), -40, 100, 30);
    expect(level.db).toBeCloseTo(-13, 6);
    expect(nextFallingLevel(level, -5, 150).db).toBe(-5);
  });
});
