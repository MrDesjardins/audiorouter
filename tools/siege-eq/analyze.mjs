// Spectral analysis of labelled game-audio reference takes (Siege footstep EQ plan,
// docs/plans/future/siege-footstep-eq.md). Local only: reads WAV files, prints
// 1/3-octave band levels and writes a JSON summary. No audio leaves the machine.
//
// Usage: node tools/siege-eq/analyze.mjs <folder> [out.json]
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const FRAME = 4096;
const HOP = 1024;
export const BANDS = [20, 25, 31.5, 40, 50, 63, 80, 100, 125, 160, 200, 250, 315, 400, 500, 630, 800, 1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300, 8000, 10000, 12500, 16000, 20000];

export function readWav(path) {
  const buf = readFileSync(path);
  if (buf.toString("ascii", 0, 4) !== "RIFF" || buf.toString("ascii", 8, 12) !== "WAVE") throw new Error(`${path}: not a WAV file`);
  let offset = 12, fmt = null, data = null;
  while (offset + 8 <= buf.length) {
    const id = buf.toString("ascii", offset, offset + 4);
    const size = buf.readUInt32LE(offset + 4);
    const body = offset + 8;
    if (id === "fmt ") {
      let format = buf.readUInt16LE(body);
      if (format === 0xfffe) format = buf.readUInt16LE(body + 24); // WAVE_FORMAT_EXTENSIBLE subformat
      fmt = { format, channels: buf.readUInt16LE(body + 2), rate: buf.readUInt32LE(body + 4), bits: buf.readUInt16LE(body + 14) };
    } else if (id === "data") data = { start: body, size: Math.min(size, buf.length - body) };
    offset = body + size + (size & 1);
  }
  if (!fmt || !data) throw new Error(`${path}: missing fmt or data chunk`);
  const bytes = fmt.bits / 8;
  const frames = Math.floor(data.size / (bytes * fmt.channels));
  const channels = Array.from({ length: fmt.channels }, () => new Float32Array(frames));
  for (let i = 0; i < frames; i += 1) {
    for (let c = 0; c < fmt.channels; c += 1) {
      const at = data.start + (i * fmt.channels + c) * bytes;
      let value;
      if (fmt.format === 3 && fmt.bits === 32) value = buf.readFloatLE(at);
      else if (fmt.format === 1 && fmt.bits === 16) value = buf.readInt16LE(at) / 32768;
      else if (fmt.format === 1 && fmt.bits === 24) value = buf.readIntLE(at, 3) / 8388608;
      else if (fmt.format === 1 && fmt.bits === 32) value = buf.readInt32LE(at) / 2147483648;
      else throw new Error(`${path}: unsupported format ${fmt.format}/${fmt.bits}-bit`);
      channels[c][i] = value;
    }
  }
  return { rate: fmt.rate, channels };
}

function fft(re, im) {
  const n = re.length;
  for (let i = 1, j = 0; i < n; i += 1) {
    let bit = n >> 1;
    for (; j & bit; bit >>= 1) j ^= bit;
    j ^= bit;
    if (i < j) { [re[i], re[j]] = [re[j], re[i]]; [im[i], im[j]] = [im[j], im[i]]; }
  }
  for (let size = 2; size <= n; size <<= 1) {
    const step = -2 * Math.PI / size;
    for (let start = 0; start < n; start += size) {
      for (let k = 0; k < size / 2; k += 1) {
        const wr = Math.cos(step * k), wi = Math.sin(step * k);
        const a = start + k, b = a + size / 2;
        const tr = re[b] * wr - im[b] * wi, ti = re[b] * wi + im[b] * wr;
        re[b] = re[a] - tr; im[b] = im[a] - ti;
        re[a] += tr; im[a] += ti;
      }
    }
  }
}

/** Per-frame 1/3-octave band power (linear, summed over channels / channel count). */
export function bandFrames({ rate, channels }) {
  const window = Float64Array.from({ length: FRAME }, (_, i) => 0.5 - 0.5 * Math.cos(2 * Math.PI * i / FRAME));
  const norm = 1 / (window.reduce((sum, w) => sum + w * w, 0) * FRAME / 2);
  const binHz = rate / FRAME;
  const bandBins = BANDS.map((center) => {
    const low = center / 2 ** (1 / 6), high = center * 2 ** (1 / 6);
    const bins = [];
    for (let k = 1; k < FRAME / 2; k += 1) if (k * binHz >= low && k * binHz < high) bins.push(k);
    if (!bins.length) bins.push(Math.max(1, Math.round(center / binHz)));
    return bins;
  });
  const frames = [];
  const length = channels[0].length;
  for (let start = 0; start + FRAME <= length; start += HOP) {
    const power = new Float64Array(BANDS.length);
    for (const channel of channels) {
      const re = new Float64Array(FRAME), im = new Float64Array(FRAME);
      for (let i = 0; i < FRAME; i += 1) re[i] = channel[start + i] * window[i];
      fft(re, im);
      bandBins.forEach((bins, band) => { for (const k of bins) power[band] += (re[k] * re[k] + im[k] * im[k]) * norm / channels.length; });
    }
    frames.push(power);
  }
  return frames;
}

const dB = (power) => 10 * Math.log10(Math.max(power, 1e-14));
const percentile = (values, fraction) => { const sorted = [...values].sort((a, b) => a - b); return sorted[Math.min(sorted.length - 1, Math.floor(fraction * sorted.length))]; };

/**
 * Split a take into event frames (≥ 6 dB above its own quiet floor, 100 Hz–10 kHz)
 * and floor frames, and report per-band levels. "far" uses the quieter half of the events.
 */
export function analyzeTake(frames) {
  const broadband = frames.map((power) => dB(power.slice(BANDS.indexOf(100), BANDS.indexOf(10000) + 1).reduce((a, b) => a + b, 0)));
  const floorDb = percentile(broadband, 0.2);
  const eventIndexes = broadband.map((level, index) => [level, index]).filter(([level]) => level >= floorDb + 6);
  const quietIndexes = broadband.map((level, index) => [level, index]).filter(([level]) => level < floorDb + 3).map(([, index]) => index);
  const median = (indexes, band) => indexes.length ? percentile(indexes.map((index) => frames[index][band]), 0.5) : 0;
  const mean = (indexes, band) => indexes.length ? indexes.reduce((sum, index) => sum + frames[index][band], 0) / indexes.length : 0;
  const sortedEvents = [...eventIndexes].sort((a, b) => a[0] - b[0]).map(([, index]) => index);
  const far = sortedEvents.slice(0, Math.max(1, Math.floor(sortedEvents.length / 2)));
  const all = frames.map((_, index) => index);
  return {
    seconds: frames.length * HOP / 48000,
    eventFraction: eventIndexes.length / frames.length,
    floorDb, peakDb: Math.max(...broadband),
    bands: BANDS.map((hz, band) => ({
      hz,
      averageDb: dB(mean(all, band)),
      eventDb: dB(mean(sortedEvents, band)),
      farEventDb: dB(mean(far, band)),
      floorDb: dB(median(quietIndexes, band)),
    })),
  };
}

if (import.meta.url === `file:///${process.argv[1].replace(/\\/g, "/")}` || process.argv[1]?.endsWith("analyze.mjs")) {
  const folder = process.argv[2];
  const out = process.argv[3];
  if (!folder) { console.error("usage: node tools/siege-eq/analyze.mjs <folder> [out.json]"); process.exit(2); }
  const files = readdirSync(folder).filter((name) => /^\d+-.*\.wav$/i.test(name)).sort((a, b) => parseInt(a) - parseInt(b));
  const result = {};
  for (const name of files) {
    const take = analyzeTake(bandFrames(readWav(join(folder, name))));
    result[name] = take;
    console.log(`${name.padEnd(42)} ${take.seconds.toFixed(1).padStart(5)} s  events ${(take.eventFraction * 100).toFixed(0).padStart(3)} %  floor ${take.floorDb.toFixed(1)} dB  peak ${take.peakDb.toFixed(1)} dB`);
  }
  if (out) writeFileSync(out, JSON.stringify(result, null, 1));
}
