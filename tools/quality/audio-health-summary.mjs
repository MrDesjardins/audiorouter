#!/usr/bin/env node
// Summarise the backend's one-minute `audioHealth` records.
//
//   node tools/quality/audio-health-summary.mjs [backend.previous.jsonl] backend.jsonl
//
// Reads only counts and times (the log holds no audio). Records written before
// the 2026-11 latency repair carry no `targetMs` and are reported separately.
// Field meanings: docs/operations/audio-health-log.md.
import fs from "node:fs";

const files = process.argv.slice(2);
if (files.length === 0) {
  console.error("usage: node tools/quality/audio-health-summary.mjs <backend.jsonl>...");
  process.exit(2);
}

const records = [];
for (const file of files) {
  for (const line of fs.readFileSync(file, "utf8").split("\n")) {
    if (!line.includes('"audioHealth"')) continue;
    try {
      records.push(JSON.parse(line));
    } catch {
      // A torn last line while the log is being written.
    }
  }
}
records.sort((a, b) => a.timeUnixMs - b.timeUnixMs);
if (records.length === 0) {
  console.log("No audioHealth records.");
  process.exit(0);
}

const STALL_INPUT_GAP_MS = 30;
const STALL_THREAD_GAP_MS = 10;
const percentile = (values, p) => {
  if (values.length === 0) return "-";
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.floor((p / 100) * sorted.length))];
};
const utc = (ms) => new Date(ms).toISOString().replace("T", " ").slice(0, 16);
const outputs = (record) => record.plane?.outputs ?? [];
const inputs = (record) => record.plane?.inputs ?? [];
const passes = (record) => record.plane?.audioThread?.passes ?? record.plane?.audioService?.passes ?? 0;
const repaired = (record) => outputs(record).some((output) => "targetMs" in output);

// Counters are totals since Play: a new process, a counter going backwards or
// a gap of more than five minutes starts a new run.
const startsRun = (previous, record) =>
  !previous ||
  previous.processId !== record.processId ||
  passes(record) < passes(previous) ||
  record.timeUnixMs - previous.timeUnixMs > 5 * 60_000;
const stalled = (record) =>
  Math.max(0, ...inputs(record).map((input) => input.maxPacketGapMs ?? 0)) > STALL_INPUT_GAP_MS ||
  (record.plane?.audioThread?.intervalMaxGapMs ?? 0) > STALL_THREAD_GAP_MS;

console.log(`${records.length} records, ${utc(records[0].timeUnixMs)} to ${utc(records.at(-1).timeUnixMs)} UTC`);
console.log(`versions: ${[...new Set(records.map((record) => record.version))].join(", ")}`);

for (const [label, subset] of [
  ["Before the latency repair (no targetMs)", records.filter((record) => !repaired(record))],
  ["After the latency repair", records.filter(repaired)],
]) {
  if (subset.length === 0) continue;
  console.log(`\n## ${label}: ${(subset.length / 60).toFixed(1)} h playing`);
  const perOutput = new Map();
  const perInput = new Map();
  const perCushion = new Map();
  const threadGaps = [];
  let startupUnderruns = 0;
  let previous = null;
  for (const record of subset) {
    const fresh = startsRun(previous, record);
    const stall = stalled(record);
    const gap = record.plane?.audioThread?.intervalMaxGapMs;
    if (gap != null) threadGaps.push(gap);
    for (const output of outputs(record)) {
      const entry = perOutput.get(output.nodeId) ?? {
        minutes: 0,
        queue: [],
        underruns: 0,
        stallUnderruns: 0,
        fullDrops: 0,
        latencyTrims: 0,
        slippedFrames: 0,
      };
      perOutput.set(output.nodeId, entry);
      entry.minutes += 1;
      if (output.queueMs != null) entry.queue.push(output.queueMs);
      const before = fresh ? null : outputs(previous).find((candidate) => candidate.nodeId === output.nodeId);
      if (!before) {
        // The first minute of a run carries the stream start.
        startupUnderruns += output.underruns ?? 0;
        continue;
      }
      const delta = (field) => Math.max(0, (output[field] ?? 0) - (before[field] ?? 0));
      const underruns = delta("underruns");
      if (stall) entry.stallUnderruns += underruns;
      else entry.underruns += underruns;
      entry.fullDrops += delta("fullDrops");
      entry.latencyTrims += delta("latencyTrims");
      entry.slippedFrames += delta("slippedFrames");
      if (before.targetMs != null) {
        const key = `${output.nodeId} at ${before.targetMs} ms`;
        const cushion = perCushion.get(key) ?? { minutes: 0, underruns: 0 };
        perCushion.set(key, cushion);
        cushion.minutes += 1;
        if (!stall) cushion.underruns += underruns;
      }
    }
    for (const input of inputs(record)) {
      const entry = perInput.get(input.nodeId) ?? { gaps: [], trimmedPackets: 0, slippedFrames: 0 };
      perInput.set(input.nodeId, entry);
      if (input.maxPacketGapMs != null) entry.gaps.push(input.maxPacketGapMs);
      const before = fresh ? null : inputs(previous).find((candidate) => candidate.nodeId === input.nodeId);
      if (before) {
        entry.trimmedPackets += Math.max(0, (input.trimmedPackets ?? 0) - (before.trimmedPackets ?? 0));
        entry.slippedFrames += Math.max(0, (input.slippedFrames ?? 0) - (before.slippedFrames ?? 0));
      }
    }
    previous = record;
  }
  console.log(`start-of-run underruns (all outputs): ${startupUnderruns}`);
  for (const [id, entry] of perOutput) {
    const hours = entry.minutes / 60;
    console.log(
      `output ${id}: ${hours.toFixed(1)} h; queueMs p50 ${percentile(entry.queue, 50)} p90 ${percentile(entry.queue, 90)} ` +
        `max ${percentile(entry.queue, 100)}; underruns ${entry.underruns} without a stall (${(entry.underruns / hours).toFixed(2)}/h) ` +
        `+ ${entry.stallUnderruns} during stalls; fullDrops ${entry.fullDrops}; latencyTrims ${entry.latencyTrims}; ` +
        `slippedFrames ${entry.slippedFrames}`,
    );
  }
  for (const [id, entry] of perInput) {
    console.log(
      `input ${id}: maxPacketGapMs p50 ${percentile(entry.gaps, 50)} p99 ${percentile(entry.gaps, 99)} ` +
        `max ${percentile(entry.gaps, 100)}; trimmedPackets ${entry.trimmedPackets}; slippedFrames ${entry.slippedFrames}`,
    );
  }
  console.log(
    `audio thread intervalMaxGapMs p50 ${percentile(threadGaps, 50)} p99 ${percentile(threadGaps, 99)} max ${percentile(threadGaps, 100)}`,
  );
  if (perCushion.size > 0) {
    console.log("underruns without a stall, by the cushion (targetMs) in force:");
    for (const [key, cushion] of [...perCushion].sort()) {
      const hours = cushion.minutes / 60;
      console.log(`  ${key}: ${hours.toFixed(1)} h, ${cushion.underruns} (${(cushion.underruns / hours).toFixed(2)}/h)`);
    }
  }
}
console.log(
  `\nA stall is a minute with an input gap above ${STALL_INPUT_GAP_MS} ms or an audio-thread gap above ${STALL_THREAD_GAP_MS} ms.`,
);
