// Generates the original (project-made) Soundboard library sounds as 16-bit mono WAV files.
// Deterministic: a seeded generator, so the same command writes the same bytes.
// Usage: node tools/soundboard/generate-original.mjs <output-folder>
// These sounds are made for AudioRouter and carry no third-party license. Listen to
// every one before it joins the library; synthesis is a starting point, not a verdict.
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

const RATE = 44100;
const TAU = Math.PI * 2;

function seeded(seed) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const seconds = (s) => new Float32Array(Math.round(s * RATE));
const hz = (note) => 440 * Math.pow(2, (note - 69) / 12);

/** One-pole low-pass, in place. */
function lowpass(buf, cutoff) {
  const a = 1 - Math.exp((-TAU * cutoff) / RATE);
  let y = 0;
  for (let i = 0; i < buf.length; i += 1) {
    y += a * (buf[i] - y);
    buf[i] = y;
  }
  return buf;
}

/** Attack/decay envelope with a held level between, applied in place. */
function envelope(buf, attack, release, level = 1) {
  const a = Math.max(1, Math.round(attack * RATE));
  const r = Math.max(1, Math.round(release * RATE));
  for (let i = 0; i < buf.length; i += 1) {
    const up = Math.min(1, i / a);
    const down = Math.min(1, (buf.length - 1 - i) / r);
    buf[i] *= level * up * down;
  }
  return buf;
}

function mix(target, source, at = 0, gain = 1) {
  const start = Math.round(at * RATE);
  for (let i = 0; i < source.length && start + i < target.length; i += 1) target[start + i] += source[i] * gain;
  return target;
}

function saw(freq, length, vibrato = 0, vibratoHz = 5.5) {
  const buf = seconds(length);
  let phase = 0;
  for (let i = 0; i < buf.length; i += 1) {
    const f = freq * (1 + vibrato * Math.sin((TAU * vibratoHz * i) / RATE));
    phase = (phase + f / RATE) % 1;
    buf[i] = 2 * phase - 1;
  }
  return buf;
}

function square(freq, length, duty = 0.5) {
  const buf = seconds(length);
  let phase = 0;
  for (let i = 0; i < buf.length; i += 1) {
    phase = (phase + freq / RATE) % 1;
    buf[i] = phase < duty ? 0.8 : -0.8;
  }
  return buf;
}

function sweep(from, to, length, wave = "sine", curve = 1) {
  const buf = seconds(length);
  let phase = 0;
  for (let i = 0; i < buf.length; i += 1) {
    const t = i / buf.length;
    const f = from + (to - from) * Math.pow(t, curve);
    phase = (phase + f / RATE) % 1;
    buf[i] = wave === "square" ? (phase < 0.5 ? 0.7 : -0.7) : Math.sin(TAU * phase);
  }
  return buf;
}

function noise(length, rand, cutoff) {
  const buf = seconds(length);
  for (let i = 0; i < buf.length; i += 1) buf[i] = rand() * 2 - 1;
  return cutoff ? lowpass(buf, cutoff) : buf;
}

function bell(freq, length, ratio = 3.5, index = 3) {
  const buf = seconds(length);
  for (let i = 0; i < buf.length; i += 1) {
    const t = i / RATE;
    const decay = Math.exp(-t * 5);
    buf[i] = Math.sin(TAU * freq * t + index * decay * Math.sin(TAU * freq * ratio * t)) * decay;
  }
  return buf;
}

function brassNote(note, length, vibrato = 0.004) {
  const buf = saw(hz(note), length, vibrato);
  lowpass(buf, 1800);
  return envelope(buf, 0.03, 0.08);
}

/** The core of the fart family: a pulse train whose pitch falls, with breathy noise. */
function fart({ from, to, length, noiseAmount = 0.25, jitter = 0.04, wobble = 0, seed, wet = 0 }) {
  const rand = seeded(seed);
  const buf = seconds(length);
  let phase = 0;
  let drift = 0;
  for (let i = 0; i < buf.length; i += 1) {
    const t = i / buf.length;
    drift = drift * 0.995 + (rand() - 0.5) * jitter;
    const base = from + (to - from) * Math.pow(t, 0.7);
    const f = base * (1 + drift + wobble * Math.sin(TAU * 11 * t * length));
    phase = (phase + f / RATE) % 1;
    const pulse = Math.pow(1 - phase, 3) * (phase < 0.5 ? 1 : 0.35) * 2 - 0.6;
    buf[i] = pulse + noiseAmount * (rand() * 2 - 1) * (0.4 + wet);
  }
  lowpass(buf, 900 + wet * 1800);
  const level = 0.6 + 0.4 * Math.sin(Math.PI * 0.5);
  return envelope(buf, 0.015, Math.min(0.25, length * 0.35), level);
}

function repeatBursts(parts, gap) {
  const total = parts.reduce((sum, p) => sum + p.length / RATE + gap, 0);
  const out = seconds(total);
  let at = 0;
  for (const p of parts) {
    mix(out, p, at);
    at += p.length / RATE + gap;
  }
  return out;
}

function echo(buf, delay, feedback, taps, extra = 0) {
  const out = new Float32Array(buf.length + Math.round(delay * RATE * taps) + Math.round(extra * RATE));
  for (let t = 0; t <= taps; t += 1) mix(out, buf, delay * t, Math.pow(feedback, t));
  return out;
}

/** Normalize to a peak ceiling, trim, and fade the edges. */
function finish(buf, peakDb = -3) {
  let peak = 0;
  for (const v of buf) peak = Math.max(peak, Math.abs(v));
  const gain = peak > 0 ? Math.pow(10, peakDb / 20) / peak : 1;
  const fadeIn = Math.round(0.001 * RATE);
  const fadeOut = Math.round(0.012 * RATE);
  const out = new Float32Array(buf.length);
  for (let i = 0; i < buf.length; i += 1) {
    const edge = Math.min(1, i / fadeIn, (buf.length - 1 - i) / fadeOut);
    out[i] = buf[i] * gain * Math.max(0, edge);
  }
  return out;
}

function wav(samples) {
  const data = Buffer.alloc(samples.length * 2);
  for (let i = 0; i < samples.length; i += 1) data.writeInt16LE(Math.round(Math.max(-1, Math.min(1, samples[i])) * 32767), i * 2);
  const head = Buffer.alloc(44);
  head.write("RIFF", 0);
  head.writeUInt32LE(36 + data.length, 4);
  head.write("WAVEfmt ", 8);
  head.writeUInt32LE(16, 16);
  head.writeUInt16LE(1, 20);
  head.writeUInt16LE(1, 22);
  head.writeUInt32LE(RATE, 24);
  head.writeUInt32LE(RATE * 2, 28);
  head.writeUInt16LE(2, 32);
  head.writeUInt16LE(16, 34);
  head.write("data", 36);
  head.writeUInt32LE(data.length, 40);
  return Buffer.concat([head, data]);
}

const sounds = {};
const add = (id, title, category, tags, make) => {
  sounds[id] = { title, category, tags, make };
};

// --- Bodily humor ---------------------------------------------------------------
add("fart-tiny-squeak", "Fart - tiny squeak", "bodily-humor", ["fart", "squeak", "small"], () =>
  fart({ from: 520, to: 260, length: 0.45, seed: 11, noiseAmount: 0.1 }));
add("fart-polite", "Fart - polite", "bodily-humor", ["fart", "short", "quiet"], () =>
  fart({ from: 210, to: 120, length: 0.28, seed: 12, noiseAmount: 0.2 }));
add("fart-classic", "Fart - classic", "bodily-humor", ["fart", "classic"], () =>
  fart({ from: 190, to: 80, length: 0.9, seed: 13, wobble: 0.03 }));
add("fart-long-rumble", "Fart - long rumble", "bodily-humor", ["fart", "long", "rumble"], () =>
  fart({ from: 95, to: 38, length: 2.4, seed: 14, noiseAmount: 0.35, jitter: 0.07 }));
add("fart-wet", "Fart - wet", "bodily-humor", ["fart", "wet"], () =>
  fart({ from: 160, to: 70, length: 1.1, seed: 15, noiseAmount: 0.6, jitter: 0.1, wet: 0.7 }));
add("fart-rapid-fire", "Fart - rapid fire", "bodily-humor", ["fart", "rapid", "burst"], () =>
  repeatBursts([0, 1, 2, 3, 4].map((n) => fart({ from: 240 - n * 15, to: 120, length: 0.16, seed: 20 + n, noiseAmount: 0.3 })), 0.05));
add("fart-huge", "Fart - huge", "bodily-humor", ["fart", "big", "bass"], () =>
  fart({ from: 70, to: 28, length: 1.8, seed: 16, noiseAmount: 0.3, jitter: 0.06 }));
add("fart-whoopee-cushion", "Fart - whoopee cushion", "bodily-humor", ["fart", "cushion", "prank"], () => {
  const up = sweep(140, 320, 0.45, "square", 1.2);
  const down = sweep(320, 110, 0.9, "square", 0.8);
  const buf = seconds(1.35);
  mix(buf, up);
  mix(buf, down, 0.45);
  lowpass(buf, 1500);
  return envelope(buf, 0.01, 0.2);
});
add("fart-echoing", "Fart - echoing", "bodily-humor", ["fart", "echo", "room"], () =>
  echo(fart({ from: 170, to: 75, length: 0.7, seed: 17 }), 0.22, 0.45, 3));

// --- Fanfares and stingers ------------------------------------------------------
add("ta-da-trumpet", "Ta-da - trumpet", "fanfares-stingers", ["tada", "fanfare", "reveal"], () => {
  const buf = seconds(2);
  mix(buf, brassNote(67, 0.18), 0);
  mix(buf, brassNote(72, 1.4), 0.2);
  mix(buf, brassNote(76, 1.4), 0.2, 0.7);
  mix(buf, brassNote(79, 1.4), 0.2, 0.6);
  return buf;
});
add("ta-da-quick", "Ta-da - quick", "fanfares-stingers", ["tada", "short", "reveal"], () => {
  const buf = seconds(1.1);
  mix(buf, brassNote(72, 0.12), 0);
  mix(buf, brassNote(79, 0.8), 0.14);
  mix(buf, brassNote(84, 0.8), 0.14, 0.5);
  return buf;
});
add("ta-da-magic", "Ta-da - magic sparkle", "fanfares-stingers", ["tada", "magic", "sparkle"], () => {
  const rand = seeded(31);
  const buf = seconds(2.2);
  [84, 88, 91, 96, 100, 103].forEach((note, i) => mix(buf, envelope(bell(hz(note), 1.2, 2, 1.2), 0.002, 0.3), i * 0.07, 0.6 + rand() * 0.2));
  return buf;
});
add("dramatic-sting", "Dramatic sting", "fanfares-stingers", ["drama", "sting", "suspense"], () => {
  const buf = seconds(2.2);
  [36, 43, 48].forEach((note) => mix(buf, envelope(lowpass(saw(hz(note), 2), 700), 0.01, 1.4), 0, 0.6));
  mix(buf, envelope(noise(0.5, seeded(32), 2500), 0.002, 0.45), 0, 0.5);
  return buf;
});

// --- Reactions and comedy -------------------------------------------------------
add("rimshot-badum-tss", "Rimshot - ba-dum-tss", "reactions-comedy", ["rimshot", "joke", "drums"], () => {
  const rand = seeded(41);
  const out = seconds(1.8);
  const kick = (at) => mix(out, envelope(sweep(150, 45, 0.18), 0.001, 0.15), at, 1);
  const snare = (at) => {
    mix(out, envelope(noise(0.16, rand, 6000), 0.001, 0.14), at, 0.7);
    mix(out, envelope(sweep(220, 160, 0.12), 0.001, 0.1), at, 0.5);
  };
  kick(0);
  snare(0.18);
  snare(0.36);
  mix(out, envelope(noise(1.2, rand, 9000), 0.002, 1.1), 0.5, 0.45);
  return out;
});
add("sad-horn-fail", "Sad horn - fail", "reactions-comedy", ["fail", "sad", "horn"], () => {
  const buf = seconds(3.3);
  [[67, 0, 0.6], [66, 0.65, 0.6], [65, 1.3, 0.6]].forEach(([note, at, len]) => mix(buf, brassNote(note, len, 0.01), at));
  const last = saw(hz(64), 1.5, 0.03, 4);
  for (let i = 0; i < last.length; i += 1) last[i] *= 1;
  mix(buf, envelope(lowpass(last, 1500), 0.04, 0.4), 1.95);
  return buf;
});
add("sad-deflate", "Sad deflate", "reactions-comedy", ["fail", "deflate", "sad"], () => {
  const buf = sweep(520, 70, 1.6, "square", 0.6);
  lowpass(buf, 1400);
  return envelope(buf, 0.02, 0.3);
});
add("boing", "Boing", "reactions-comedy", ["boing", "spring", "cartoon"], () => {
  const buf = seconds(0.9);
  let phase = 0;
  for (let i = 0; i < buf.length; i += 1) {
    const t = i / RATE;
    const f = 240 + 160 * Math.sin(TAU * 9 * t) * Math.exp(-t * 4) - 80 * t;
    phase = (phase + f / RATE) % 1;
    buf[i] = Math.sin(TAU * phase) * Math.exp(-t * 3.4);
  }
  return buf;
});
add("slide-whistle-up", "Slide whistle - up", "reactions-comedy", ["whistle", "slide", "up"], () =>
  envelope(sweep(300, 1800, 1, "sine", 1.5), 0.02, 0.1));
add("slide-whistle-down", "Slide whistle - down", "reactions-comedy", ["whistle", "slide", "down"], () =>
  envelope(sweep(1800, 300, 1, "sine", 0.7), 0.02, 0.1));
add("record-scratch", "Record scratch", "reactions-comedy", ["scratch", "record", "stop"], () => {
  const rand = seeded(42);
  const buf = seconds(0.7);
  let phase = 0;
  for (let i = 0; i < buf.length; i += 1) {
    const t = i / buf.length;
    const f = 900 * Math.sin(TAU * 3.3 * t * (1 + t)) + 1200 * (1 - t);
    phase = (phase + Math.abs(f) / RATE) % 1;
    buf[i] = (phase < 0.5 ? 0.6 : -0.6) + 0.5 * (rand() * 2 - 1);
  }
  lowpass(buf, 5000);
  return envelope(buf, 0.005, 0.15);
});
add("crickets", "Crickets", "reactions-comedy", ["crickets", "awkward", "silence"], () => {
  const buf = seconds(3.2);
  for (let burst = 0; burst < 6; burst += 1) {
    for (let chirp = 0; chirp < 3; chirp += 1) {
      const tone = envelope(sweep(4300, 4700, 0.035), 0.004, 0.02);
      mix(buf, tone, burst * 0.52 + chirp * 0.06, 0.6);
    }
  }
  return buf;
});

// --- Horns and alerts -----------------------------------------------------------
const hornBlast = (length) => {
  const buf = seconds(length);
  mix(buf, saw(415, length, 0.002), 0, 0.5);
  mix(buf, saw(622, length, 0.002), 0, 0.5);
  mix(buf, square(831, length, 0.4), 0, 0.25);
  lowpass(buf, 4200);
  return envelope(buf, 0.012, 0.07);
};
add("air-horn-single", "Air horn - single", "horns-alerts", ["airhorn", "horn", "blast"], () => hornBlast(1.2));
add("air-horn-triple", "Air horn - triple", "horns-alerts", ["airhorn", "horn", "triple"], () =>
  repeatBursts([hornBlast(0.45), hornBlast(0.45), hornBlast(0.9)], 0.08));
add("air-horn-long", "Air horn - long", "horns-alerts", ["airhorn", "horn", "long"], () => hornBlast(3));
add("buzzer-wrong", "Buzzer - wrong answer", "horns-alerts", ["buzzer", "wrong", "nope"], () => {
  const buf = seconds(0.8);
  mix(buf, square(120, 0.8, 0.5), 0, 0.7);
  mix(buf, square(126, 0.8, 0.5), 0, 0.7);
  lowpass(buf, 2500);
  return envelope(buf, 0.005, 0.05);
});
add("doorbell-ding-dong", "Doorbell - ding dong", "horns-alerts", ["doorbell", "bell", "ding"], () => {
  const buf = seconds(2.4);
  mix(buf, bell(hz(79), 1.2, 2, 2.2), 0);
  mix(buf, bell(hz(72), 1.9, 2, 2.2), 0.6);
  return buf;
});
add("notification-chime", "Notification chime", "horns-alerts", ["chime", "notification", "ding"], () => {
  const buf = seconds(1.3);
  mix(buf, bell(hz(88), 0.9, 2, 1.4), 0);
  mix(buf, bell(hz(95), 1.0, 2, 1.4), 0.14);
  return buf;
});
add("siren-short", "Siren - short", "horns-alerts", ["siren", "alarm"], () => {
  const buf = seconds(2.4);
  let phase = 0;
  for (let i = 0; i < buf.length; i += 1) {
    const t = i / RATE;
    const f = 800 + 350 * Math.sin(TAU * 1.1 * t);
    phase = (phase + f / RATE) % 1;
    buf[i] = phase < 0.5 ? 0.5 : -0.5;
  }
  lowpass(buf, 3000);
  return envelope(buf, 0.02, 0.15);
});

// --- Game and arcade ------------------------------------------------------------
add("coin", "Coin", "game-arcade", ["coin", "pickup", "arcade"], () => {
  const buf = seconds(0.45);
  mix(buf, envelope(square(hz(83), 0.09, 0.5), 0.002, 0.03));
  mix(buf, envelope(square(hz(88), 0.35, 0.5), 0.002, 0.3), 0.09);
  return lowpass(buf, 7000);
});
add("power-up", "Power-up", "game-arcade", ["powerup", "level", "arcade"], () => {
  const buf = seconds(0.8);
  [60, 64, 67, 72, 76, 79].forEach((note, i) => mix(buf, envelope(square(hz(note), 0.12, 0.5), 0.002, 0.04), i * 0.09, 0.8));
  return lowpass(buf, 7000);
});
add("game-over", "Game over", "game-arcade", ["gameover", "lose", "arcade"], () => {
  const buf = seconds(1.8);
  [67, 64, 60, 55].forEach((note, i) => mix(buf, envelope(square(hz(note), i === 3 ? 0.8 : 0.28, 0.5), 0.005, i === 3 ? 0.4 : 0.05), i * 0.3, 0.8));
  return lowpass(buf, 6000);
});
add("laser", "Laser", "sci-fi-magic", ["laser", "zap", "scifi"], () =>
  envelope(sweep(2400, 200, 0.35, "square", 0.6), 0.002, 0.1));

// --- Whooshes -------------------------------------------------------------------
add("whoosh", "Whoosh", "whooshes", ["whoosh", "swoosh", "transition"], () => {
  const rand = seeded(51);
  const buf = noise(1.1, rand);
  let low = 0;
  let band = 0;
  for (let i = 0; i < buf.length; i += 1) {
    const t = i / buf.length;
    const center = 300 + 3500 * Math.sin(Math.PI * t);
    const f = 2 * Math.sin((Math.PI * center) / RATE);
    low += f * band;
    const high = buf[i] - low - 0.25 * band;
    band += f * high;
    buf[i] = band;
  }
  return envelope(buf, 0.25, 0.4);
});

const out = process.argv[2];
if (!out) {
  console.error("usage: node tools/soundboard/generate-original.mjs <output-folder>");
  process.exit(2);
}
await mkdir(out, { recursive: true });
const manifest = [];
for (const [id, spec] of Object.entries(sounds)) {
  const samples = finish(spec.make());
  const durationSeconds = samples.length / RATE;
  if (durationSeconds > 5) throw new Error(`${id} is ${durationSeconds.toFixed(2)} s; built-in sounds must be 5 s or less`);
  await writeFile(join(out, `${id}.wav`), wav(samples));
  manifest.push({ id, title: spec.title, category: spec.category, tags: spec.tags, durationMs: Math.round(durationSeconds * 1000), license: "original", source: "tools/soundboard/generate-original.mjs" });
}
await writeFile(join(out, "originals.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log(`wrote ${manifest.length} sounds to ${out}`);
