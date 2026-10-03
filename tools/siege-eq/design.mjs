// Evaluate a candidate Advanced EQ on the analysed reference takes (see analyze.mjs).
// Filter responses use the same RBJ formulas as crates/dsp `coefficients`.
//
// Usage: node tools/siege-eq/design.mjs <analysis.json> [preset-out.json]
import { readFileSync, writeFileSync } from "node:fs";

const RATE = 48000;

/** Headphone layer: broad corrections all three AutoEq fits of the KZ ZS10 Pro agree on. */
export const HEADPHONE = [
  { type: "peaking", frequencyHz: 165, q: 0.65, gainDb: -3.5, why: "ZS10 Pro bass hump" },
  { type: "peaking", frequencyHz: 770, q: 1.0, gainDb: 3.0, why: "ZS10 Pro mid recess" },
  { type: "peaking", frequencyHz: 2100, q: 2.5, gainDb: -2.0, why: "ZS10 Pro 2 kHz peak (half: full cut sounded muffled)" },
  { type: "peaking", frequencyHz: 3200, q: 2.8, gainDb: 3.5, why: "ZS10 Pro 3 kHz dip" },
];

/** Game layer: derived from the Siege reference takes. */
export const GAME = [
  { type: "highPass", frequencyHz: 50, q: 0.707, gainDb: 0, why: "rumble only below 50 Hz" },
  { type: "peaking", frequencyHz: 200, q: 1.0, gainDb: 4.0, why: "steps through floors 100–400 Hz" },
  { type: "highShelf", frequencyHz: 3000, q: 0.7, gainDb: 3.0, why: "clarity; a gunfire cut at 1.6 kHz sounded muffled" },
  { type: "peaking", frequencyHz: 7000, q: 0.9, gainDb: 6.0, why: "step scuffs, crouch and drone 4–12 kHz" },
];

export function responseDb(band, hz) {
  const omega = 2 * Math.PI * band.frequencyHz / RATE;
  const sin = Math.sin(omega), cos = Math.cos(omega);
  const alpha = sin / (2 * band.q);
  const A = 10 ** (band.gainDb / 40);
  let b0, b1, b2, a0, a1, a2;
  switch (band.type) {
    case "peaking": [b0, b1, b2, a0, a1, a2] = [1 + alpha * A, -2 * cos, 1 - alpha * A, 1 + alpha / A, -2 * cos, 1 - alpha / A]; break;
    case "highPass": [b0, b1, b2, a0, a1, a2] = [(1 + cos) / 2, -(1 + cos), (1 + cos) / 2, 1 + alpha, -2 * cos, 1 - alpha]; break;
    case "lowPass": [b0, b1, b2, a0, a1, a2] = [(1 - cos) / 2, 1 - cos, (1 - cos) / 2, 1 + alpha, -2 * cos, 1 - alpha]; break;
    case "lowShelf": case "highShelf": {
      const beta = 2 * Math.sqrt(A) * alpha;
      [b0, b1, b2, a0, a1, a2] = band.type === "lowShelf"
        ? [A * ((A + 1) - (A - 1) * cos + beta), 2 * A * ((A - 1) - (A + 1) * cos), A * ((A + 1) - (A - 1) * cos - beta), (A + 1) + (A - 1) * cos + beta, -2 * ((A - 1) + (A + 1) * cos), (A + 1) + (A - 1) * cos - beta]
        : [A * ((A + 1) + (A - 1) * cos + beta), -2 * A * ((A - 1) + (A + 1) * cos), A * ((A + 1) + (A - 1) * cos - beta), (A + 1) - (A - 1) * cos + beta, 2 * ((A - 1) - (A + 1) * cos), (A + 1) - (A - 1) * cos - beta];
      break;
    }
    default: throw new Error(`unsupported ${band.type}`);
  }
  const w = 2 * Math.PI * hz / RATE;
  const mag2 = (c0, c1, c2) => {
    const re = c0 + c1 * Math.cos(-w) + c2 * Math.cos(-2 * w);
    const im = c1 * Math.sin(-w) + c2 * Math.sin(-2 * w);
    return re * re + im * im;
  };
  return 10 * Math.log10(mag2(b0, b1, b2) / mag2(a0, a1, a2));
}

export const totalDb = (bands, hz) => bands.reduce((sum, band) => sum + responseDb(band, hz), 0);

/** IEC 61672 A-weighting, as a rough loudness weight per band. */
function aWeightDb(f) {
  const f2 = f * f;
  const ra = (12194 ** 2 * f2 * f2) / ((f2 + 20.6 ** 2) * Math.sqrt((f2 + 107.7 ** 2) * (f2 + 737.9 ** 2)) * (f2 + 12194 ** 2));
  return 20 * Math.log10(ra) + 2.0;
}

const power = (db) => 10 ** (db / 10);
const level = (bands, key, eqDb, weight = true) => 10 * Math.log10(bands.reduce((sum, band, i) => sum + power(band[key] + eqDb[i] + (weight ? aWeightDb(band.hz) : 0)), 0));

if (process.argv[1]?.endsWith("design.mjs")) {
  const analysis = JSON.parse(readFileSync(process.argv[2], "utf8"));
  const eq = [...GAME, ...HEADPHONE];
  const hz = analysis[Object.keys(analysis)[0]].bands.map((band) => band.hz);
  const curve = hz.map((f) => totalDb(eq, f));
  const flat = hz.map(() => 0);
  const gameOnly = hz.map((f) => totalDb(GAME, f));
  console.log("Response (dB):  Hz  game  +headphone");
  hz.forEach((f, i) => console.log(`${String(f).padStart(6)} ${gameOnly[i].toFixed(1).padStart(6)} ${curve[i].toFixed(1).padStart(6)}`));
  const fine = []; for (let f = 20; f <= 20000; f *= 1.01) fine.push(totalDb(eq, f));
  console.log(`Peak boost ${Math.max(...fine).toFixed(1)} dB → preamp ${(-Math.max(...fine)).toFixed(1)} dB`);

  const gun = analysis["12-ShootinAbout18meterNextRoom.wav"].bands;
  console.log("\nA-weighted, game layer only vs flat (headphone layer only flattens the earphones):");
  console.log("take".padEnd(42) + "  vs gunfire before→after   own SNR before→after");
  for (const [name, take] of Object.entries(analysis)) {
    if (/^1[234]-/.test(name)) continue;
    const isDrone = /Drone/.test(name);
    const key = isDrone ? "averageDb" : "farEventDb";
    const vsGun = (eqDb) => level(take.bands, key, eqDb) - level(gun, "averageDb", eqDb);
    const snr = (eqDb) => level(take.bands, key, eqDb) - level(take.bands, "floorDb", eqDb);
    console.log(`${name.padEnd(42)} ${vsGun(flat).toFixed(1).padStart(6)} → ${vsGun(gameOnly).toFixed(1).padStart(5)}  (${(vsGun(gameOnly) - vsGun(flat) >= 0 ? "+" : "")}${(vsGun(gameOnly) - vsGun(flat)).toFixed(1)})   ${snr(flat).toFixed(1).padStart(5)} → ${snr(gameOnly).toFixed(1).padStart(5)}`);
  }
  if (process.argv[3]) {
    const parameters = {};
    eq.forEach((band, index) => Object.assign(parameters, {
      [`band${index}Enabled`]: true, [`band${index}Type`]: band.type, [`band${index}FrequencyHz`]: band.frequencyHz,
      [`band${index}Q`]: band.q, [`band${index}GainDb`]: band.gainDb,
    }));
    writeFileSync(process.argv[3], JSON.stringify({ bands: eq, preampDb: -Math.ceil(Math.max(...fine)), parameters }, null, 1));
  }
}
