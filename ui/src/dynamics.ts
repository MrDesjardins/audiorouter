/**
 * Presentation math for the Compressor, Gate and Limiter editors. Static
 * curves mirror `crates/dsp` (`compression_reduction`, `gate_target_gain_db`
 * and the peak limiter) so what is drawn is what the backend applies. The
 * response sketch reruns the same detector equations on a synthetic word.
 */
import type { Node } from "@audiorouter/contracts";

export type DynamicsKind = "compressor" | "gate" | "limiter";
export const isDynamicsKind = (kind: string): kind is DynamicsKind => kind === "compressor" || kind === "gate" || kind === "limiter";

export type CompressorSettings = { thresholdDb: number; ratio: number; attackMs: number; releaseMs: number; kneeDb: number; makeupDb: number };
export type GateSettings = { thresholdDb: number; rangeDb: number; hysteresisDb: number; ratio: number; attackMs: number; holdMs: number; releaseMs: number };
export type LimiterSettings = { ceilingDb: number; lookaheadMs: number; releaseMs: number };

/** Backend defaults (`crates/control` node catalog) for unset parameters. */
const DEFAULTS = {
  compressor: { thresholdDb: -18, ratio: 3, attackMs: 10, releaseMs: 150, kneeDb: 6, makeupDb: 0 },
  gate: { thresholdDb: -45, rangeDb: 60, hysteresisDb: 3, ratio: 4, attackMs: 5, holdMs: 50, releaseMs: 150 },
  limiter: { ceilingDb: -1, lookaheadMs: 5, releaseMs: 100 },
} as const;

function read<T extends Record<string, number>>(node: Node, defaults: T): T {
  const result = { ...defaults } as Record<string, number>;
  for (const name of Object.keys(defaults)) {
    const value = node.parameters[name];
    if (typeof value === "number" && Number.isFinite(value)) result[name] = value;
  }
  return result as T;
}
export const compressorSettings = (node: Node): CompressorSettings => read(node, DEFAULTS.compressor);
export const gateSettings = (node: Node): GateSettings => read(node, DEFAULTS.gate);
export const limiterSettings = (node: Node): LimiterSettings => read(node, DEFAULTS.limiter);

/** Gain reduction (positive dB) for a detector level, as `compression_reduction`. */
export function compressorReductionDb(levelDb: number, { thresholdDb, ratio, kneeDb }: Pick<CompressorSettings, "thresholdDb" | "ratio" | "kneeDb">): number {
  const over = levelDb - thresholdDb;
  if (kneeDb > 0 && over > -kneeDb / 2 && over < kneeDb / 2) {
    const distance = over + kneeDb / 2;
    return distance * distance * (1 - 1 / ratio) / (2 * kneeDb);
  }
  return over > 0 ? over * (1 - 1 / ratio) : 0;
}

export function compressorOutputDb(levelDb: number, settings: CompressorSettings): number {
  return levelDb - compressorReductionDb(levelDb, settings) + settings.makeupDb;
}

/** Static gate curve for a steady level: open at or above the threshold. */
export function gateOutputDb(levelDb: number, { thresholdDb, ratio, rangeDb }: Pick<GateSettings, "thresholdDb" | "ratio" | "rangeDb">): number {
  if (levelDb >= thresholdDb) return levelDb;
  return levelDb - Math.min(Math.max((thresholdDb - levelDb) * (ratio - 1), 0), rangeDb);
}

export const limiterOutputDb = (levelDb: number, { ceilingDb }: Pick<LimiterSettings, "ceilingDb">) => Math.min(levelDb, ceilingDb);

/** The ratio that places the curve's 0 dBFS input at `outputDb` (compressor, no knee, no makeup). */
export function ratioForOutputAtFullScale(thresholdDb: number, outputDb: number): number {
  const over = -thresholdDb;
  const outOver = Math.min(Math.max(outputDb - thresholdDb, over / 20), over);
  return over <= 0 ? 1 : over / outOver;
}

export type ResponsePoint = { ms: number; inputDb: number; gainDb: number };
export type ResponseSketch = { points: ResponsePoint[]; wordStartMs: number; wordEndMs: number; totalMs: number; quietDb: number; loudDb: number };

const SKETCH_RATE = 16_000;
const SKETCH_STEP_MS = 4;

/**
 * Run a synthetic word through the detector: quiet room noise, a word for
 * 300 ms, then quiet again. The word is set 12 dB over the threshold (or
 * ceiling), the room noise well below it, so attack, hold and release show
 * as they would on speech. Uses the backend's one-pole coefficients.
 */
export function responseSketch(kind: DynamicsKind, node: Node): ResponseSketch {
  const totalMs = 900;
  const wordStartMs = 100;
  const wordEndMs = 400;
  const points: ResponsePoint[] = [];
  const coefficient = (ms: number) => Math.exp(-1 / (Math.max(ms, 0.01) * 0.001 * SKETCH_RATE));
  const frames = Math.round(totalMs * SKETCH_RATE / 1000);
  const every = Math.round(SKETCH_STEP_MS * SKETCH_RATE / 1000);
  if (kind === "compressor") {
    const settings = compressorSettings(node);
    const loudDb = Math.min(settings.thresholdDb + 12, 0);
    const quietDb = settings.thresholdDb - 24;
    const attack = coefficient(settings.attackMs);
    const release = coefficient(settings.releaseMs);
    let envelope = quietDb;
    for (let frame = 0; frame < frames; frame += 1) {
      const ms = frame * 1000 / SKETCH_RATE;
      const inputDb = ms >= wordStartMs && ms < wordEndMs ? loudDb : quietDb;
      const c = inputDb > envelope ? attack : release;
      envelope = c * envelope + (1 - c) * inputDb;
      if (frame % every === 0) points.push({ ms, inputDb, gainDb: -compressorReductionDb(envelope, settings) });
    }
    return { points, wordStartMs, wordEndMs, totalMs, quietDb, loudDb };
  }
  if (kind === "gate") {
    const settings = gateSettings(node);
    const loudDb = Math.min(settings.thresholdDb + 12, 0);
    const quietDb = settings.thresholdDb - Math.max(settings.hysteresisDb + 12, 18);
    const attack = coefficient(settings.attackMs);
    const release = coefficient(settings.releaseMs);
    const hold = Math.floor(settings.holdMs * 0.001 * SKETCH_RATE);
    let gainDb = -settings.rangeDb;
    let open = false;
    let holdFrames = 0;
    for (let frame = 0; frame < frames; frame += 1) {
      const ms = frame * 1000 / SKETCH_RATE;
      const inputDb = ms >= wordStartMs && ms < wordEndMs ? loudDb : quietDb;
      if (open) {
        if (inputDb < settings.thresholdDb - settings.hysteresisDb) {
          if (holdFrames > 0) holdFrames -= 1; else open = false;
        } else holdFrames = hold;
      } else if (inputDb >= settings.thresholdDb) {
        open = true;
        holdFrames = hold;
      }
      const target = open ? 0 : -Math.min(Math.max((settings.thresholdDb - inputDb) * (settings.ratio - 1), 0), settings.rangeDb);
      const c = target > gainDb ? attack : release;
      gainDb = c * gainDb + (1 - c) * target;
      if (frame % every === 0) points.push({ ms, inputDb, gainDb });
    }
    return { points, wordStartMs, wordEndMs, totalMs, quietDb, loudDb };
  }
  const settings = limiterSettings(node);
  const loudDb = settings.ceilingDb + 6; // the float path carries overs above 0 dBFS
  const quietDb = settings.ceilingDb - 18;
  const release = coefficient(settings.releaseMs);
  let gain = 1;
  for (let frame = 0; frame < frames; frame += 1) {
    const ms = frame * 1000 / SKETCH_RATE;
    const inputDb = ms >= wordStartMs && ms < wordEndMs ? loudDb : quietDb;
    const desired = inputDb > settings.ceilingDb ? 10 ** ((settings.ceilingDb - inputDb) / 20) : 1;
    gain = Math.min(gain, desired);
    if (desired >= gain) gain += (1 - gain) * (1 - release);
    if (frame % every === 0) points.push({ ms, inputDb, gainDb: 20 * Math.log10(Math.max(gain, 1e-6)) });
  }
  return { points, wordStartMs, wordEndMs, totalMs, quietDb, loudDb };
}

export type LevelSample = { at: number; inputDb: number; outputDb: number; reductionDb: number; open: boolean | null };

/**
 * Estimate the room-noise floor and the voice level from recent input
 * levels: the 15th and 90th percentiles of the readings above silence.
 * Returns null until there is enough material and a usable gap.
 */
export function levelStatistics(samples: LevelSample[]): { noiseDb: number; voiceDb: number } | null {
  const levels = samples.map((sample) => sample.inputDb).filter((db) => Number.isFinite(db) && db > -110).sort((a, b) => a - b);
  if (levels.length < 40) return null;
  const at = (fraction: number) => levels[Math.min(levels.length - 1, Math.floor(fraction * levels.length))];
  const noiseDb = at(0.15);
  const voiceDb = at(0.9);
  return voiceDb - noiseDb >= 10 ? { noiseDb, voiceDb } : null;
}

const round = (value: number, step: number) => Math.round(value / step) * step;

/** Gate threshold a third of the way from the noise floor to the voice, at least 6 dB above the noise. */
export function suggestGateThreshold({ noiseDb, voiceDb }: { noiseDb: number; voiceDb: number }): number {
  return Math.min(0, Math.max(-80, round(Math.max(noiseDb + 6, noiseDb + (voiceDb - noiseDb) / 3), 1)));
}

/** Compressor threshold that takes about `targetDb` off typical voice peaks at the current ratio. */
export function suggestCompressorThreshold(voiceDb: number, ratio: number, targetDb = 6): number {
  const over = ratio > 1.01 ? targetDb / (1 - 1 / ratio) : targetDb;
  return Math.min(0, Math.max(-60, round(voiceDb - over, 1)));
}

export const PEAK_HOLD_MS = 1200;
export const PEAK_FALL_DB_PER_SECOND = 20;
export type PeakHold = { db: number; heldAt: number; updatedAt: number };

/**
 * Display ballistics: a new higher peak is taken at once and held for
 * `PEAK_HOLD_MS`, then the marker falls at a fixed rate but never below
 * the current reading. This only slows the picture; values stay exact.
 */
export function nextPeakHold(previous: PeakHold | null, valueDb: number, now: number, holdMs = PEAK_HOLD_MS, fallDbPerSecond = PEAK_FALL_DB_PER_SECOND): PeakHold {
  if (!previous || valueDb >= previous.db) return { db: valueDb, heldAt: now, updatedAt: now };
  const held = now - previous.heldAt < holdMs;
  if (held) return { ...previous, updatedAt: now };
  const fallStart = Math.max(previous.updatedAt, previous.heldAt + holdMs);
  const fallen = previous.db - fallDbPerSecond * Math.max(0, now - fallStart) / 1000;
  return { db: Math.max(valueDb, fallen), heldAt: previous.heldAt, updatedAt: now };
}

/** Level bar ballistics: rises at once, falls at `fallDbPerSecond`. */
export function nextFallingLevel(previous: { db: number; at: number } | null, valueDb: number, now: number, fallDbPerSecond = 30): { db: number; at: number } {
  if (!previous || valueDb >= previous.db) return { db: valueDb, at: now };
  return { db: Math.max(valueDb, previous.db - fallDbPerSecond * Math.max(0, now - previous.at) / 1000), at: now };
}
