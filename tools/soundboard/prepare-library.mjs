// Turns the picked sourcing files into library-ready sounds and a provenance manifest.
// Usage:
//   node tools/soundboard/prepare-library.mjs <sourcing-folder> <picks.txt> <freesound-licenses.json> <output-folder>
// For each pick: decode, mix to mono, trim leading and trailing silence, match the average
// level (with a peak ceiling), fade the end, write 16-bit 44.1 kHz WAV, and record where
// the sound came from. Sounds longer than 5 seconds after trimming are rejected, not cut.
// Requires ffmpeg and ffprobe on the PATH.
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
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
};

const picks = readFileSync(picksFile, "utf8").split(/\r?\n/).map((s) => s.trim()).filter(Boolean);
const licenses = new Map(JSON.parse(readFileSync(licensesFile, "utf8")).map((r) => [`${r.folder}/${r.file}`, r]));
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
  // Freesound
  const rec = licenses.get(`${parts[1]}/${file}`);
  if (!rec || !/zero/.test(rec.license ?? "")) throw new Error(`${rel}: no CC0 license record`);
  const source = { name: "Freesound", author: rec.user, url: rec.url, license: "CC0", title: rec.title };
  if (parts[1] === "fart") return { category: "bodily-humor", title: /short-fart|close/.test(stem) ? `Fart - ${/short/.test(stem) ? "short" : "close up"}` : `Fart ${number("fart")}`, tags: ["fart"], source };
  if (parts[1] === "burp") return { category: "bodily-humor", title: `Burp ${number("burp")}`, tags: ["burp"], source };
  if (parts[1] === "tada") return { category: "fanfares-stingers", title: `Ta-da ${number("tada")}`, tags: ["tada", "fanfare"], source };
  if (parts[1] === "ohyeah") return { category: "reactions-comedy", title: "Oh yeah!", tags: ["voice", "cheer"], source, ownerException: "spoken phrase approved by the owner 2026-10-29" };
  throw new Error(`${rel}: unknown group`);
}

mkdirSync(outDir, { recursive: true });
const work = mkdtempSync(join(tmpdir(), "soundboard-"));
const manifest = [];
const rejected = [];
const counters = {};
const ids = new Set();

for (const rel of picks.sort()) {
  const input = join(sourcing, rel);
  const info = describe(rel, counters);
  let id = slug(info.title);
  for (let n = 2; ids.has(id); n += 1) id = `${slug(info.title)}-${n}`;
  const trimmed = join(work, "trimmed.wav");
  // Pass 1: mono, 44.1 kHz, trim silence at both ends.
  ffmpeg(["-i", input, "-af",
    "aformat=channel_layouts=mono,silenceremove=start_periods=1:start_threshold=-55dB:start_silence=0.004,areverse,silenceremove=start_periods=1:start_threshold=-60dB:start_silence=0.01,areverse",
    "-ar", "44100", "-sample_fmt", "s16", trimmed]);
  const seconds = durationSeconds(trimmed);
  if (!(seconds > 0.02)) { rejected.push({ rel, reason: "silent after trimming" }); continue; }
  if (seconds > MAX_SECONDS) { rejected.push({ rel, reason: `${seconds.toFixed(1)} s is longer than ${MAX_SECONDS} s` }); continue; }
  // Measure, then choose a gain that reaches the target average level without passing the peak ceiling.
  const stats = ffmpeg(["-i", trimmed, "-af", "volumedetect", "-f", "null", "-"]);
  const mean = Number.parseFloat(stats.match(/mean_volume: (-?[\d.]+) dB/)?.[1]);
  const peak = Number.parseFloat(stats.match(/max_volume: (-?[\d.]+) dB/)?.[1]);
  const gain = Math.max(-24, Math.min(TARGET_MEAN_DB - mean, PEAK_CEILING_DB - peak, MAX_BOOST_DB));
  const fadeStart = Math.max(0, seconds - 0.012);
  const output = join(outDir, `${id}.wav`);
  ffmpeg(["-i", trimmed, "-af", `volume=${gain.toFixed(2)}dB,afade=t=in:d=0.001,afade=t=out:st=${fadeStart.toFixed(4)}:d=0.012`, "-ar", "44100", "-ac", "1", "-sample_fmt", "s16", output]);
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
