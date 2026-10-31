import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
const amplitude = (kind, frequency, seconds, duration = 0.1) => {
  const bytes = readFileSync(
    new URL(
      `../../target/tool-guide-tools/${kind}/output.f32le`,
      import.meta.url,
    ),
  );
  const start = Math.round(seconds * 48000);
  const count = Math.round(48000 * duration);
  let sine = 0,
    cosine = 0;
  for (let index = start; index < start + count; index++) {
    const sample = bytes.readFloatLE(index * 8);
    const phase = (index / 48000) * 2 * Math.PI * frequency;
    sine += sample * Math.sin(phase);
    cosine += sample * Math.cos(phase);
  }
  return (2 * Math.hypot(sine, cosine)) / count;
};
const input = {
  earlyA: amplitude("inputSwitch", 110, 3.22),
  earlyB: amplitude("inputSwitch", 880, 3.22),
  lateA: amplitude("inputSwitch", 110, 4),
  lateB: amplitude("inputSwitch", 880, 4),
};
assert(
  input.earlyA > 0.04 && input.earlyB > 0.005,
  "Input Switch must carry both tones during its production crossfade",
);
assert(
  input.lateA < 0.001 && input.lateB > 0.07,
  "Input Switch must finish on B",
);
// Output Switch deliberately fades A out in 10 ms, then makes B live.
const output = {
  earlyA: amplitude("outputSwitch", 110, 3.202, 0.002),
  lateA: amplitude("outputSwitch", 110, 4),
};
assert(
  output.earlyA > 0.01 && output.lateA < 0.001,
  "Output A must fade out rather than cut abruptly",
);
console.log(JSON.stringify({ passed: true, input, output }, null, 2));
