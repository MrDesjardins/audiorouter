#!/usr/bin/env node
// Daily project size history: non-blank lines of code, tests and Markdown,
// one row per UTC day since the first commit, read straight from git objects
// (no checkout, so line endings and the working tree do not matter).
//
//   node tools/metrics/code-size.mjs update <history.csv> <out-dir> [ref]
//
// `update` adds every day missing from the CSV (the first run back-fills the
// whole history) and always recomputes the newest day, then redraws
// <out-dir>/code-size-areas.svg (front end, back end, audio driver, tooling,
// Markdown) and <out-dir>/code-size-total.svg. `ref` defaults to origin/main;
// each day uses the last commit on its first-parent history at 23:59:59 UTC.
// A change to the area rules below must bump RULES_VERSION: rows recorded
// with another version are recomputed, so the curves never mix two rule sets.
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const RULES_VERSION = 2;

export const COLUMNS = [
  "date",
  "commit",
  "rules",
  "frontend_code",
  "frontend_test",
  "backend_code",
  "backend_test",
  "driver_code",
  "driver_test",
  "tooling_code",
  "tooling_test",
  "markdown_specs",
  "markdown_other",
];

const FRONTEND_EXT = /\.(?:ts|tsx|js|jsx|mjs|cjs|css|html)$/;
const DRIVER_EXT = /\.(?:c|cpp|h|hpp|inx|inf|rc|ps1)$/;
const ACCEPTANCE_EXT = /\.(?:ps1|py)$/;
const TOOLING_EXT = /\.(?:rs|ts|tsx|js|jsx|mjs|cjs|py|ps1|psm1|sh|c|cpp|h|hpp|html|css)$/;
const EXCLUDED = /(?:^|\/)(?:node_modules|third_party|target|dist[^/]*|coverage|gen|test-results|playwright-report)\//;
// Generated Markdown would make a curve jump on each regeneration day.
const GENERATED_MARKDOWN = new Set(["docs/operations/api-reference.md"]);

/**
 * The area of a tracked file, or null when it is not counted.
 * Returns { area: "frontend"|"backend"|"driver"|"tooling", test: boolean, rust: boolean }
 * or { area: "markdown", bucket: "specs"|"other" }.
 *
 * Front end: everything under ui/ (tests: *.test.*, *.spec.*, ui/e2e/, the
 * *harness*.html test pages). Back end: Rust in crates/ (except
 * driver-helper) and src-tauri/, the TypeScript API contracts, and the
 * acceptance scripts in tests/acceptance/ except the M03 driver ones.
 * Audio driver: drivers/, crates/driver-helper and the M03 acceptance
 * scripts. Rust tests are tests/ folders, *_tests.rs / tests.rs files, and
 * #[cfg(test)] items inside source files. Tooling: scripts and programs in
 * tools/, the GitHub workflows and the git hooks (tests: *.test.*, test/
 * and tests/ folders, Rust tests as above). site/, examples/ and videos/
 * are not counted; all Markdown is counted.
 */
export function classify(path) {
  if (EXCLUDED.test(path)) return null;
  if (path.endsWith(".md")) {
    if (GENERATED_MARKDOWN.has(path)) return null;
    const specs = path.startsWith("docs/spec/") || path.startsWith("docs/plans/");
    return { area: "markdown", bucket: specs ? "specs" : "other" };
  }
  const rust = path.endsWith(".rs");
  const rustTest = rust && (/(?:^|\/)tests\//.test(path) || /(?:^|\/)(?:\w+_)?tests?\.rs$/.test(path));
  if (path.startsWith("ui/")) {
    if (!FRONTEND_EXT.test(path)) return null;
    const test = path.startsWith("ui/e2e/") || /\.(?:test|spec|pw)\.[^/]+$/.test(path) || /harness[^/]*\.html$/.test(path);
    return { area: "frontend", test, rust: false };
  }
  if (path.startsWith("drivers/")) {
    if (!DRIVER_EXT.test(path)) return null;
    return { area: "driver", test: /(?:^|\/)tests\//.test(path), rust: false };
  }
  if (path.startsWith("crates/driver-helper/")) {
    return rust ? { area: "driver", test: rustTest, rust } : null;
  }
  if (path.startsWith("crates/") || path.startsWith("src-tauri/")) {
    return rust ? { area: "backend", test: rustTest, rust } : null;
  }
  if (path.startsWith("contracts/src/") && /\.ts$/.test(path)) {
    return { area: "backend", test: /\.test\.ts$/.test(path), rust: false };
  }
  if (path.startsWith("tests/acceptance/") && ACCEPTANCE_EXT.test(path)) {
    const name = path.slice("tests/acceptance/".length);
    return { area: name.startsWith("m03-") ? "driver" : "backend", test: true, rust: false };
  }
  if (path.startsWith("tools/") || path.startsWith(".githooks/")) {
    // Git hooks have no extension.
    if (!TOOLING_EXT.test(path) && !path.startsWith(".githooks/")) return null;
    const test = rustTest || /(?:^|\/)tests?\//.test(path) || /\.(?:test|spec)\.[^/]+$/.test(path);
    return { area: "tooling", test, rust };
  }
  if (path.startsWith(".github/workflows/") && /\.ya?ml$/.test(path)) {
    return { area: "tooling", test: false, rust: false };
  }
  return null;
}

/**
 * Non-blank line counts of a file. For Rust, `test` counts the non-blank
 * lines of #[cfg(test)] items (attribute through the item's closing brace or
 * semicolon), found with a small lexer that skips comments, strings, raw
 * strings and char literals so braces inside them do not count.
 */
export function countLines(text, rust) {
  const lines = text.split(/\r?\n/);
  const total = lines.filter((line) => /\S/.test(line)).length;
  if (!rust) return { total, test: 0 };
  const testLines = rustTestLines(text);
  let test = 0;
  for (const index of testLines) if (/\S/.test(lines[index] ?? "")) test += 1;
  return { total, test };
}

function rustTestLines(text) {
  const marked = new Set();
  const n = text.length;
  let line = 0;
  let depth = 0;
  let pendingLine = -1; // line of a #[cfg(test)] waiting for its item
  let testStartLine = -1;
  let testDepth = -1; // depth outside the test item's braces
  const mark = (from, to) => {
    for (let l = from; l <= to; l += 1) marked.add(l);
  };
  const ident = (c) => /[A-Za-z0-9_]/.test(c);
  let i = 0;
  while (i < n) {
    const c = text[i];
    if (c === "\n") {
      line += 1;
      i += 1;
      continue;
    }
    if (c === "/" && text[i + 1] === "/") {
      while (i < n && text[i] !== "\n") i += 1;
      continue;
    }
    if (c === "/" && text[i + 1] === "*") {
      let nest = 1;
      i += 2;
      while (i < n && nest > 0) {
        if (text[i] === "\n") line += 1;
        if (text[i] === "/" && text[i + 1] === "*") {
          nest += 1;
          i += 2;
        } else if (text[i] === "*" && text[i + 1] === "/") {
          nest -= 1;
          i += 2;
        } else i += 1;
      }
      continue;
    }
    // Raw strings: r"..", r#".."#, br#".."#.
    if ((c === "r" || (c === "b" && text[i + 1] === "r")) && !ident(text[i - 1] ?? " ")) {
      let j = c === "b" ? i + 2 : i + 1;
      let hashes = 0;
      while (text[j] === "#") {
        hashes += 1;
        j += 1;
      }
      if (text[j] === '"') {
        const close = '"' + "#".repeat(hashes);
        const end = text.indexOf(close, j + 1);
        const stop = end < 0 ? n : end + close.length;
        for (let k = j; k < stop; k += 1) if (text[k] === "\n") line += 1;
        i = stop;
        continue;
      }
    }
    if (c === '"') {
      i += 1;
      while (i < n && text[i] !== '"') {
        if (text[i] === "\\") i += 1;
        if (text[i] === "\n") line += 1;
        i += 1;
      }
      i += 1;
      continue;
    }
    if (c === "'") {
      // A char literal ('x', '\n', '\u{1F600}'); otherwise a lifetime.
      if (text[i + 1] === "\\") {
        const end = text.indexOf("'", i + 3);
        i = end < 0 ? n : end + 1;
        continue;
      }
      const cp = text.codePointAt(i + 1);
      const width = cp > 0xffff ? 2 : 1;
      if (text[i + 1 + width] === "'") {
        i += 2 + width;
        continue;
      }
      i += 1;
      continue;
    }
    if (c === "#" && testDepth < 0 && /^#\[cfg\((?:test\)|all\(test\b)/.test(text.slice(i, i + 16))) {
      if (pendingLine < 0) pendingLine = line;
      i += 1;
      continue;
    }
    if (c === "{") {
      if (pendingLine >= 0 && testDepth < 0) {
        testStartLine = pendingLine;
        testDepth = depth;
        pendingLine = -1;
      }
      depth += 1;
    } else if (c === "}") {
      depth -= 1;
      if (testDepth >= 0 && depth === testDepth) {
        mark(testStartLine, line);
        testDepth = -1;
      }
    } else if (c === ";" && pendingLine >= 0 && testDepth < 0) {
      // `#[cfg(test)] mod tests;` or `use …;`: a one-line test item.
      mark(pendingLine, line);
      pendingLine = -1;
    }
    i += 1;
  }
  if (testDepth >= 0) mark(testStartLine, line);
  return marked;
}

function git(args, options = {}) {
  const encoding = options.binary ? undefined : "utf8";
  const result = spawnSync("git", args, { encoding, maxBuffer: 1 << 30, input: options.input });
  if (result.status !== 0) throw new Error(`git ${args.join(" ")} failed: ${result.stderr}`);
  return result.stdout;
}

/** Reads blobs with one `git cat-file --batch` call; returns sha -> Buffer. */
function readBlobs(shas) {
  const out = git(["cat-file", "--batch"], { input: shas.join("\n") + "\n", binary: true });
  const blobs = new Map();
  let offset = 0;
  while (offset < out.length) {
    const headerEnd = out.indexOf(10, offset);
    const [sha, type, size] = out.subarray(offset, headerEnd).toString("utf8").split(" ");
    if (type === "missing") throw new Error(`missing blob ${sha}`);
    const start = headerEnd + 1;
    const length = Number(size);
    blobs.set(sha, out.subarray(start, start + length));
    offset = start + length + 1;
  }
  return blobs;
}

/** Sums the area counts of one commit; `cache` maps blob sha -> counts. */
export function measureCommit(commit, cache) {
  const entries = [];
  for (const record of git(["ls-tree", "-r", "-z", "--full-tree", commit]).split("\0")) {
    if (!record) continue;
    const tab = record.indexOf("\t");
    const [, type, sha] = record.slice(0, tab).split(" ");
    if (type !== "blob") continue;
    const path = record.slice(tab + 1);
    const kind = classify(path);
    if (kind) entries.push({ sha, kind });
  }
  const unknown = [...new Set(entries.filter((e) => !cache.has(e.sha + (e.kind.rust ? "r" : ""))).map((e) => e.sha))];
  if (unknown.length > 0) {
    const blobs = readBlobs(unknown);
    for (const e of entries) {
      const key = e.sha + (e.kind.rust ? "r" : "");
      if (!cache.has(key)) cache.set(key, countLines(blobs.get(e.sha).toString("utf8"), e.kind.rust));
    }
  }
  const row = Object.fromEntries(COLUMNS.slice(3).map((c) => [c, 0]));
  for (const e of entries) {
    const counts = cache.get(e.sha + (e.kind.rust ? "r" : ""));
    if (e.kind.area === "markdown") {
      row[`markdown_${e.kind.bucket}`] += counts.total;
    } else if (e.kind.test) {
      row[`${e.kind.area}_test`] += counts.total;
    } else {
      row[`${e.kind.area}_test`] += counts.test;
      row[`${e.kind.area}_code`] += counts.total - counts.test;
    }
  }
  return row;
}

const DAY = 86_400_000;
const dayString = (ms) => new Date(ms).toISOString().slice(0, 10);

/** One {date, commit} per UTC day, oldest first, through `today`. */
export function dailyCommits(ref, today = Date.now()) {
  const log = git(["log", "--first-parent", "--format=%H %ct", ref])
    .trim()
    .split("\n")
    .map((l) => {
      const [sha, ct] = l.split(" ");
      return { sha, ms: Number(ct) * 1000 };
    });
  if (log.length === 0) return [];
  const first = Date.parse(dayString(Math.min(...log.map((c) => c.ms))));
  const days = [];
  for (let day = first; day <= today; day += DAY) {
    const end = day + DAY - 1000;
    // Newest first: the first first-parent commit at or before the day's end.
    const commit = log.find((c) => c.ms <= end);
    if (commit) days.push({ date: dayString(day), commit: commit.sha });
  }
  return days;
}

export function parseCsv(text) {
  const [header, ...lines] = text.trim().split(/\r?\n/);
  if (header !== COLUMNS.join(",")) return [];
  return lines.filter(Boolean).map((line) => {
    const cells = line.split(",");
    const row = Object.fromEntries(COLUMNS.map((c, i) => [c, cells[i]]));
    for (const c of COLUMNS.slice(2)) row[c] = Number(row[c]);
    return row;
  });
}

export function formatCsv(rows) {
  return [COLUMNS.join(","), ...rows.map((r) => COLUMNS.map((c) => r[c]).join(","))].join("\n") + "\n";
}

// ---------------------------------------------------------------- charts
// Reference data-viz palette, light surface (categorical slots 1-4).
const SURFACE = "#fcfcfb";
const PAGE = "#f4f6f9";
const INK = "#0b0b0b";
const INK_2 = "#52514e";
const MUTED = "#6c7787";
const GRID = "#e7ebf1";
const BORDER = "#d7dee8";
const SERIES = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100"];
const FONT = 'system-ui,-apple-system,"Segoe UI",sans-serif';

const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
const fmt = (v) => Math.round(v).toLocaleString("en-US");

function niceStep(max, ticks) {
  const raw = max / ticks;
  const mag = 10 ** Math.floor(Math.log10(raw));
  const norm = raw / mag;
  return (norm <= 1 ? 1 : norm <= 2 ? 2 : norm <= 2.5 ? 2.5 : norm <= 5 ? 5 : 10) * mag;
}

function shortNumber(v) {
  if (v >= 1_000_000) return `${+(v / 1_000_000).toFixed(1)}M`;
  if (v >= 1000) return `${+(v / 1000).toFixed(1)}k`;
  return String(v);
}

/** One line panel: series = [{label, color, values}], all sharing `dates`. */
function panel({ x, y, width, height, title, dates, series }) {
  const left = x + 56;
  const right = x + width - 112; // room for direct end labels
  const top = y + 58;
  const bottom = y + height - 34;
  const max = Math.max(1, ...series.flatMap((s) => s.values));
  const step = niceStep(max, 4);
  const yMax = Math.ceil(max / step) * step;
  const px = (i) => left + (dates.length > 1 ? ((right - left) * i) / (dates.length - 1) : (right - left) / 2);
  const py = (v) => bottom - ((bottom - top) * v) / yMax;
  const out = [];
  out.push(`<g role="group" aria-label="${esc(title)}">`);
  out.push(`<rect x="${x}" y="${y}" width="${width}" height="${height}" rx="12" fill="${SURFACE}" stroke="${BORDER}"/>`);
  out.push(`<text x="${x + 18}" y="${y + 28}" class="panel-title">${esc(title)}</text>`);
  // Legend row under the title (identity never by color alone).
  let lx = x + 18;
  for (const s of series) {
    out.push(`<line x1="${lx}" y1="${y + 42}" x2="${lx + 16}" y2="${y + 42}" stroke="${s.color}" stroke-width="2" stroke-dasharray="${s.dash ?? ""}"/>`);
    out.push(`<text x="${lx + 21}" y="${y + 45.5}" class="legend">${esc(s.label)}</text>`);
    lx += 30 + s.label.length * 6.2;
  }
  for (let v = 0; v <= yMax + step / 2; v += step) {
    const gy = py(v).toFixed(1);
    out.push(`<line x1="${left}" y1="${gy}" x2="${right}" y2="${gy}" stroke="${GRID}"/>`);
    out.push(`<text x="${left - 8}" y="${(+gy + 3).toFixed(1)}" text-anchor="end" class="axis">${shortNumber(v)}</text>`);
  }
  // Month starts as x ticks (first day, and the first of each month).
  const ticks = dates.map((d, i) => ({ d, i })).filter(({ d, i }) => i === 0 || d.endsWith("-01"));
  let lastX = -Infinity;
  for (const { d, i } of ticks) {
    const tx = px(i);
    if (tx - lastX < 60) continue;
    lastX = tx;
    out.push(`<text x="${tx.toFixed(1)}" y="${bottom + 16}" text-anchor="${i === 0 ? "start" : "middle"}" class="axis">${d}</text>`);
  }
  // Lines, then end labels nudged apart so they never overlap.
  const ends = [];
  for (const s of series) {
    const points = s.values.map((v, i) => `${px(i).toFixed(1)},${py(v).toFixed(1)}`).join(" ");
    out.push(`<polyline points="${points}" fill="none" stroke="${s.color}" stroke-width="2" stroke-linejoin="round" stroke-linecap="round" stroke-dasharray="${s.dash ?? ""}"/>`);
    const last = s.values.length - 1;
    out.push(`<circle cx="${px(last).toFixed(1)}" cy="${py(s.values[last]).toFixed(1)}" r="4" fill="${s.color}" stroke="${SURFACE}" stroke-width="2"/>`);
    ends.push({ y: py(s.values[last]), text: `${s.label} ${fmt(s.values[last])}` });
  }
  ends.sort((a, b) => a.y - b.y);
  for (let k = 1; k < ends.length; k += 1) ends[k].y = Math.max(ends[k].y, ends[k - 1].y + 13);
  const overflow = ends.length ? ends[ends.length - 1].y - (bottom + 4) : 0;
  if (overflow > 0) for (const e of ends) e.y -= overflow;
  for (const e of ends) out.push(`<text x="${right + 10}" y="${(e.y + 3.5).toFixed(1)}" class="value">${esc(e.text)}</text>`);
  out.push("</g>");
  return out.join("");
}

function svgDocument({ width, height, title, subtitle, description, body }) {
  return [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="chart-title chart-desc">`,
    `<title id="chart-title">${esc(title)}</title>`,
    `<desc id="chart-desc">${esc(description)}</desc>`,
    `<style>text{font-family:${FONT}}.heading{font-size:21px;font-weight:700;fill:${INK}}.subtitle{font-size:12px;fill:${INK_2}}.panel-title{font-size:14px;font-weight:650;fill:${INK}}.axis{font-size:10px;fill:${MUTED}}.legend{font-size:11px;fill:${INK_2}}.value{font-size:11px;font-weight:600;fill:${INK}}</style>`,
    `<rect width="${width}" height="${height}" rx="16" fill="${PAGE}"/>`,
    `<text x="26" y="38" class="heading">${esc(title)}</text>`,
    `<text x="26" y="60" class="subtitle">${esc(subtitle)}</text>`,
    body,
    "</svg>\n",
  ].join("\n");
}

function areaSeries(rows, codeKey, testKey, labels = ["Code", "Tests"]) {
  const code = rows.map((r) => r[codeKey]);
  const test = rows.map((r) => r[testKey]);
  return [
    { label: labels[0], color: SERIES[0], values: code },
    { label: labels[1], color: SERIES[1], values: test },
    { label: "All", color: SERIES[2], values: code.map((v, i) => v + test[i]) },
  ];
}

export function renderAreas(rows) {
  const dates = rows.map((r) => r.date);
  const last = rows[rows.length - 1];
  // Three product areas on the top row, tooling and Markdown below.
  const gap = 16;
  const top = 388;
  const inner = top * 3 + gap * 2;
  const bottom = (inner - gap) / 2;
  const h = 300;
  const panels = [
    ["Front end (ui/)", areaSeries(rows, "frontend_code", "frontend_test"), 0, 0, top],
    ["Back end (Rust crates, shell, contracts)", areaSeries(rows, "backend_code", "backend_test"), top + gap, 0, top],
    ["Audio driver (drivers/, driver helper)", areaSeries(rows, "driver_code", "driver_test"), (top + gap) * 2, 0, top],
    ["Tooling (tools/, workflows, git hooks)", areaSeries(rows, "tooling_code", "tooling_test"), 0, 1, bottom],
    ["Markdown", areaSeries(rows, "markdown_specs", "markdown_other", ["Specs & plans", "Other docs"]), bottom + gap, 1, bottom],
  ];
  const body = panels
    .map(([title, series, dx, row, width]) => panel({ x: 24 + dx, y: 80 + row * (h + gap), width, height: h, title, dates, series }))
    .join("\n");
  return svgDocument({
    width: 24 * 2 + inner,
    height: 80 + h * 2 + gap + 24,
    title: "AudioRouter · lines of code and tests by area",
    subtitle: `Non-blank lines per day, ${dates[0]} to ${last.date} · commit ${last.commit.slice(0, 8)} · daily values in the CSV`,
    description: "Five panels (front end, back end, audio driver, tooling, Markdown), each with code, tests and their sum over time.",
    body,
  });
}

export function renderTotal(rows) {
  const dates = rows.map((r) => r.date);
  const last = rows[rows.length - 1];
  const code = rows.map((r) => r.frontend_code + r.backend_code + r.driver_code + r.tooling_code);
  const test = rows.map((r) => r.frontend_test + r.backend_test + r.driver_test + r.tooling_test);
  const docs = rows.map((r) => r.markdown_specs + r.markdown_other);
  const series = [
    { label: "Code", color: SERIES[0], values: code },
    { label: "Tests", color: SERIES[1], values: test },
    { label: "Markdown", color: SERIES[2], values: docs },
    { label: "All", color: SERIES[3], values: code.map((v, i) => v + test[i] + docs[i]) },
  ];
  return svgDocument({
    width: 1000,
    height: 440,
    title: "AudioRouter · total project size",
    subtitle: `Non-blank lines per day across front end, back end, audio driver, tooling and Markdown, ${dates[0]} to ${last.date}`,
    description: "Total code, tests, Markdown and their sum over time.",
    body: panel({ x: 24, y: 80, width: 952, height: 336, title: "All areas", dates, series }),
  });
}

// ---------------------------------------------------------------- CLI
export function update(csvPath, outDir, ref = "origin/main", today = Date.now()) {
  const existing = existsSync(csvPath) ? parseCsv(readFileSync(csvPath, "utf8")) : [];
  const byDate = new Map(existing.filter((r) => r.rules === RULES_VERSION).map((r) => [r.date, r]));
  const days = dailyCommits(ref, today);
  const cache = new Map();
  const rows = [];
  let computed = 0;
  for (const [index, { date, commit }] of days.entries()) {
    const known = byDate.get(date);
    const newest = index === days.length - 1;
    if (known && known.commit === commit && !newest) {
      rows.push(known);
      continue;
    }
    rows.push({ date, commit, rules: RULES_VERSION, ...measureCommit(commit, cache) });
    computed += 1;
  }
  if (rows.length === 0) throw new Error(`no commits on ${ref}`);
  mkdirSync(outDir, { recursive: true });
  writeFileSync(csvPath, formatCsv(rows));
  writeFileSync(join(outDir, "code-size-areas.svg"), renderAreas(rows));
  writeFileSync(join(outDir, "code-size-total.svg"), renderTotal(rows));
  console.log(`${rows.length} days (${computed} measured) through ${rows[rows.length - 1].date}`);
  return rows;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  const [command, csvPath, outDir, ref] = process.argv.slice(2);
  if (command !== "update" || !csvPath || !outDir) {
    console.error("usage: code-size.mjs update <history.csv> <out-dir> [ref]");
    process.exit(2);
  }
  update(csvPath, outDir, ref);
}
