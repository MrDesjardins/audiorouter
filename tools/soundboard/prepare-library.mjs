// Turns the picked sourcing files into library-ready sounds and a provenance manifest.
// Usage:
//   node tools/soundboard/prepare-library.mjs <sourcing-folder> <picks.txt>[,more.txt] <licenses.json>[,more.json] <output-folder>
// For each pick: decode, mix to mono, trim leading and trailing silence, match the average
// level (with a peak ceiling), fade the end, write 16-bit 44.1 kHz WAV, and record where
// the sound came from. Sounds longer than 5 seconds after trimming are rejected, not cut,
// unless the picks line ends with "| excerpt=SECONDS": then the loudest window of that
// length is kept, faded in and out (for example a few seconds of a long crowd applause).
// Requires ffmpeg and ffprobe on the PATH.
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { tmpdir } from "node:os";

const [sourcing, picksFile, licensesFile, outDir] = process.argv.slice(2);
if (!outDir) throw new Error("usage: prepare-library.mjs <sourcing-folder> <picks.txt> <freesound-licenses.json> <output-folder>");

const TARGET_MEAN_DB = -20; // average level every sound is moved toward
const PEAK_CEILING_DB = -1; // never louder than this
const MAX_BOOST_DB = 18;
const MAX_SECONDS = 5.05; // "5 seconds or less", with rounding room for files cut at exactly 5 s

const CATEGORIES = {
  "bodily-humor": "Bodily humor",
  "reactions-comedy": "Reactions and comedy",
  "fanfares-stingers": "Fanfares and stingers",
  "horns-alerts": "Horns and alerts",
  "game-arcade": "Game and arcade",
  interface: "Interface and clicks",
  impacts: "Impacts",
  whooshes: "Whooshes",
  "sci-fi-magic": "Sci-fi and magic",
  "applause-laughs": "Applause and laughs",
};

const picks = picksFile.split(",").flatMap((f) => readFileSync(f, "utf8").split(/\r?\n/)).map((s) => s.trim()).filter(Boolean)
  .map((line) => {
    const [path, option] = line.split("|").map((s) => s.trim());
    return { path, excerpt: Number.parseFloat(option?.match(/excerpt=([\d.]+)/)?.[1]) || null };
  });
const licenses = new Map(licensesFile.split(",").flatMap((f) => JSON.parse(readFileSync(f, "utf8"))).map((r) => [`${r.folder}/${r.file}`, r]));
const originals = new Map(JSON.parse(readFileSync(join(sourcing, "original", "originals.json"), "utf8")).map((o) => [`${o.id}.wav`, o]));
const kenneyPacks = {
  "kenney_digital-audio": "digital-audio",
  "kenney_impact-sounds": "impact-sounds",
  "kenney_interface-sounds": "interface-sounds",
  "kenney_sci-fi-sounds": "sci-fi-sounds",
  "kenney_ui-audio": "ui-audio",
};

const slug = (s) => s.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
const humanize = (s) => s.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/[_-]+/g, " ").replace(/\b0+(\d)/g, "$1").trim().replace(/^./, (c) => c.toUpperCase());

function ffmpeg(args) {
  const run = spawnSync("ffmpeg", ["-hide_banner", "-y", ...args], { encoding: "utf8" });
  if (run.status !== 0) throw new Error(`ffmpeg failed: ${run.stderr.slice(-400)}`);
  return run.stderr;
}
function durationSeconds(file) {
  const run = spawnSync("ffprobe", ["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", file], { encoding: "utf8" });
  return Number.parseFloat(run.stdout);
}

/** Category, title and source record for one pick, from its path. */
function describe(rel, counters) {
  const parts = rel.split("/");
  const file = parts.at(-1);
  const stem = file.replace(/\.[^.]+$/, "");
  const number = (key) => String((counters[key] = (counters[key] ?? 0) + 1)).padStart(2, "0");
  if (parts[0] === "original") {
    const o = originals.get(file);
    return { category: o.category, title: o.title, tags: o.tags, source: { name: "Original", author: "AudioRouter", url: "tools/soundboard/generate-original.mjs", license: "original" } };
  }
  if (parts[0] === "kenney") {
    const pack = parts[1];
    const kind = stem.split("_")[0].replace(/\d+$/, "");
    const category = pack === "kenney_digital-audio" ? "game-arcade"
      : pack === "kenney_impact-sounds" ? "impacts"
      : pack === "kenney_interface-sounds" ? "interface"
      : /^laser|^computer/.test(kind) ? "sci-fi-magic"
      : /^explosion|^lowFrequency/.test(kind) ? "impacts"
      : /^slime/.test(kind) ? "reactions-comedy" : "sci-fi-magic";
    return { category, title: humanize(stem), tags: [humanize(kind).toLowerCase()],
      source: { name: `Kenney ${kenneyPacks[pack]}`, author: "Kenney (Kenney Vleugels)", url: `https://kenney.nl/assets/${kenneyPacks[pack]}`, license: "CC0" } };
  }
  // Freesound: freesound/<group>/<file>, or a pack folder <group>/<file> at the top
  const group = parts.length > 2 ? parts[1] : parts[0];
  const rec = licenses.get(`${group}/${file}`);
  if (!rec || !/zero/.test(rec.license ?? "")) throw new Error(`${rel}: no CC0 license record`);
  const source = { name: "Freesound", author: rec.user, url: rec.url, license: "CC0", title: rec.title };
  if (group === "clapping") return { category: "applause-laughs", title: `Applause - crowd ${number("clap")}`, tags: ["applause", "clapping", "crowd"], source };
  if (group === "laughter") return { category: "applause-laughs", title: `Evil laugh ${number("laugh")}`, tags: ["laugh", "evil"], source };
  if (group === "fart") return { category: "bodily-humor", title: /short-fart|close/.test(stem) ? `Fart - ${/short/.test(stem) ? "short" : "close up"}` : `Fart ${number("fart")}`, tags: ["fart"], source };
  if (group === "burp") return { category: "bodily-humor", title: `Burp ${number("burp")}`, tags: ["burp"], source };
  if (group === "tada") return { category: "fanfares-stingers", title: `Ta-da ${number("tada")}`, tags: ["tada", "fanfare"], source };
  if (group === "ohyeah") return { category: "reactions-comedy", title: "Oh yeah!", tags: ["voice", "cheer"], source, ownerException: "spoken phrase approved by the owner 2026-10-29" };
  throw new Error(`${rel}: unknown group`);
}

mkdirSync(outDir, { recursive: true });
const work = mkdtempSync(join(tmpdir(), "soundboard-"));
const manifest = [];
const rejected = [];
const counters = {};
const ids = new Set();

/** Start (seconds) of the loudest window of the given length in an audio file. */
function loudestWindowStart(file, windowSeconds) {
  const run = spawnSync("ffmpeg", ["-v", "error", "-i", file, "-f", "s16le", "-ac", "1", "-ar", "44100", "-"], { maxBuffer: 1 << 28 });
  const samples = new Int16Array(run.stdout.buffer, run.stdout.byteOffset, Math.floor(run.stdout.length / 2));
  const window = Math.round(windowSeconds * 44100);
  const step = 4410;
  let best = 0;
  let bestEnergy = -1;
  for (let start = 0; start + window <= samples.length; start += step) {
    let energy = 0;
    for (let i = start; i < start + window; i += 11) energy += samples[i] * samples[i];
    if (energy > bestEnergy) { bestEnergy = energy; best = start; }
  }
  return best / 44100;
}

for (const { path: rel, excerpt } of picks.sort((a, b) => a.path.localeCompare(b.path))) {
  const input = join(sourcing, rel);
  const info = describe(rel, counters);
  let id = slug(info.title);
  for (let n = 2; ids.has(id); n += 1) id = `${slug(info.title)}-${n}`;
  const trimmed = join(work, "trimmed.wav");
  // Pass 1: mono, 44.1 kHz, trim silence at both ends.
  ffmpeg(["-i", input, "-af",
    "aformat=channel_layouts=mono,silenceremove=start_periods=1:start_threshold=-55dB:start_silence=0.004,areverse,silenceremove=start_periods=1:start_threshold=-60dB:start_silence=0.01,areverse",
    "-ar", "44100", "-sample_fmt", "s16", trimmed]);
  let seconds = durationSeconds(trimmed);
  let fadeInSeconds = 0.001;
  let fadeOutSeconds = 0.012;
  if (excerpt && seconds > excerpt) {
    const start = loudestWindowStart(trimmed, excerpt);
    const cut = join(work, "excerpt.wav");
    ffmpeg(["-ss", start.toFixed(3), "-t", String(excerpt), "-i", trimmed, "-ar", "44100", "-sample_fmt", "s16", cut]);
    renameSync(cut, trimmed);
    seconds = durationSeconds(trimmed);
    fadeInSeconds = 0.03; // a window cut from a longer recording needs gentle edges
    fadeOutSeconds = 0.4;
  }
  if (!(seconds > 0.02)) { rejected.push({ rel, reason: "silent after trimming" }); continue; }
  if (seconds > MAX_SECONDS) { rejected.push({ rel, reason: `${seconds.toFixed(1)} s is longer than ${MAX_SECONDS} s` }); continue; }
  // Measure, then choose a gain that reaches the target average level without passing the peak ceiling.
  const stats = ffmpeg(["-i", trimmed, "-af", "volumedetect", "-f", "null", "-"]);
  const mean = Number.parseFloat(stats.match(/mean_volume: (-?[\d.]+) dB/)?.[1]);
  const peak = Number.parseFloat(stats.match(/max_volume: (-?[\d.]+) dB/)?.[1]);
  const gain = Math.max(-24, Math.min(TARGET_MEAN_DB - mean, PEAK_CEILING_DB - peak, MAX_BOOST_DB));
  const fadeStart = Math.max(0, seconds - fadeOutSeconds);
  const output = join(outDir, `${id}.wav`);
  ffmpeg(["-i", trimmed, "-af", `volume=${gain.toFixed(2)}dB,afade=t=in:d=${fadeInSeconds},afade=t=out:st=${fadeStart.toFixed(4)}:d=${fadeOutSeconds}`, "-ar", "44100", "-ac", "1", "-sample_fmt", "s16", output]);
  const check = ffmpeg(["-i", output, "-af", "volumedetect", "-f", "null", "-"]);
  ids.add(id);
  manifest.push({
    id: `lib:${id}`, file: `${id}.wav`, title: info.title, category: info.category, tags: info.tags,
    durationMs: Math.round(durationSeconds(output) * 1000),
    gainAppliedDb: Number(gain.toFixed(2)),
    meanDb: Number.parseFloat(check.match(/mean_volume: (-?[\d.]+) dB/)?.[1]),
    peakDb: Number.parseFloat(check.match(/max_volume: (-?[\d.]+) dB/)?.[1]),
    sha256: createHash("sha256").update(readFileSync(output)).digest("hex"),
    source: { ...info.source, originalFile: basename(rel) }, ownerException: info.ownerException,
  });
}
rmSync(work, { recursive: true, force: true });
writeFileSync(join(outDir, "library.json"), JSON.stringify({ categories: CATEGORIES, sounds: manifest }, null, 2) + "\n");
writeFileSync(join(outDir, "rejected.json"), JSON.stringify(rejected, null, 2) + "\n");
console.log(`prepared ${manifest.length} sounds, rejected ${rejected.length}`);
for (const r of rejected) console.log(`  rejected ${r.rel}: ${r.reason}`);
