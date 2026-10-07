#!/usr/bin/env node
// Markdown summaries for the nightly quality workflow (code review P2-8).
// Informational only: nothing here fails on a coverage value.
//
//   node tools/quality/summary.mjs rust <llvm-cov.json> <title>
//       Per-crate line/function/region coverage from
//       `cargo llvm-cov report --json --summary-only`.
//   node tools/quality/summary.mjs ui <coverage-summary.json>
//       Vitest (v8) totals from the json-summary reporter.
//   node tools/quality/summary.mjs history <file.csv> key=value...
//       Append one row to a CSV history and print the last 14 rows.
//
// The Markdown goes to stdout; the workflow appends it to the step summary.
import { appendFileSync, existsSync, readFileSync, writeFileSync } from "node:fs";

// Crates that build and test on any OS (no Windows API dependency).
const PORTABLE = new Set(["domain", "dsp", "engine", "plugin-host", "protocol", "recording", "storage"]);

const percent = (covered, count) => (count === 0 ? "–" : `${((100 * covered) / count).toFixed(1)} %`);

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

/** `crates/<name>/…` → name, `src-tauri/…` → shell, anything else → other. */
function unitOf(filename) {
  const path = filename.replaceAll("\\", "/");
  const crate = /\/crates\/([^/]+)\//.exec(path);
  if (crate) return crate[1];
  if (/\/src-tauri\//.test(path)) return "src-tauri";
  return "other";
}

function emptyTotals() {
  return { lines: [0, 0], functions: [0, 0], regions: [0, 0] };
}

function add(totals, summary) {
  for (const key of Object.keys(totals)) {
    totals[key][0] += summary[key].covered;
    totals[key][1] += summary[key].count;
  }
}

function row(name, totals) {
  const cells = Object.values(totals).map(([covered, count]) => percent(covered, count));
  return `| ${name} | ${totals.lines[1]} | ${cells.join(" | ")} |`;
}

function rust(path, title) {
  const report = readJson(path).data[0];
  const units = new Map();
  const portable = emptyTotals();
  for (const file of report.files) {
    const unit = unitOf(file.filename);
    if (!units.has(unit)) units.set(unit, emptyTotals());
    add(units.get(unit), file.summary);
    if (PORTABLE.has(unit)) add(portable, file.summary);
  }
  const all = emptyTotals();
  add(all, report.totals);
  const lines = [`## ${title}`, "", "| Crate | Lines | Line cov. | Function cov. | Region cov. |", "| --- | ---: | ---: | ---: | ---: |"];
  for (const [unit, totals] of [...units].sort(([a], [b]) => a.localeCompare(b))) {
    lines.push(row(PORTABLE.has(unit) ? `${unit} (portable)` : unit, totals));
  }
  if (portable.lines[1] > 0 && portable.lines[1] !== all.lines[1]) lines.push(row("**Portable crates**", portable));
  lines.push(row("**Total**", all), "");
  console.log(lines.join("\n"));
}

function ui(path) {
  const { total } = readJson(path);
  const lines = ["## UI coverage (Vitest, v8)", "", "| Metric | Covered | Total | Coverage |", "| --- | ---: | ---: | ---: |"];
  for (const key of ["lines", "statements", "functions", "branches"]) {
    lines.push(`| ${key} | ${total[key].covered} | ${total[key].total} | ${percent(total[key].covered, total[key].total)} |`);
  }
  console.log(lines.join("\n") + "\n");
}

function history(path, pairs) {
  const record = Object.fromEntries(
    pairs.map((pair) => {
      const at = pair.indexOf("=");
      if (at < 1) throw new Error(`expected key=value, got ${pair}`);
      return [pair.slice(0, at), pair.slice(at + 1).replaceAll(",", ";")];
    }),
  );
  const keys = Object.keys(record);
  if (!existsSync(path) || readFileSync(path, "utf8").trim() === "") writeFileSync(path, `${keys.join(",")}\n`);
  const [header, ...rows] = readFileSync(path, "utf8").trim().split(/\r?\n/);
  const columns = header.split(",");
  appendFileSync(path, `${columns.map((column) => record[column] ?? "").join(",")}\n`);
  rows.push(columns.map((column) => record[column] ?? "").join(","));
  const lines = ["## Trend (last 14 runs)", "", `| ${columns.join(" | ")} |`, `| ${columns.map(() => "---").join(" | ")} |`];
  for (const line of rows.slice(-14)) lines.push(`| ${line.split(",").join(" | ")} |`);
  console.log(lines.join("\n") + "\n");
}

const [command, ...args] = process.argv.slice(2);
if (command === "rust" && args.length === 2) rust(args[0], args[1]);
else if (command === "ui" && args.length === 1) ui(args[0]);
else if (command === "history" && args.length >= 2) history(args[0], args.slice(1));
else {
  console.error("usage: summary.mjs rust <json> <title> | ui <json> | history <csv> key=value...");
  process.exit(2);
}
