#!/usr/bin/env node
// Generate and publish informational quality and performance stats in README.
//
//   node tools/quality/publish-readme-coverage.mjs summarize <workspace.json> <shell.json> <ui-summary.json> <out.json>
//   node tools/quality/publish-readme-coverage.mjs update <README.md> <summary.json> <soak.env> <startup.env> <source-sha> <run-id> <repo> <server-url> <history.csv> <chart.svg>
import { existsSync, readFileSync, writeFileSync } from "node:fs";

const PORTABLE_CRATES = new Set(["domain", "dsp", "engine", "plugin-host", "protocol", "recording", "storage"]);
const START = "<!-- quality-stats:start -->";
const END = "<!-- quality-stats:end -->";
const HISTORY_COLUMNS = [
  "date",
  "run_id",
  "commit",
  "soak_p99_us",
  "soak_p999_us",
  "soak_heap_growth_kib",
  "soak_private_growth_mib",
  "backend_ready_ms",
];

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function empty() {
  return { lines: { covered: 0, count: 0 }, functions: { covered: 0, count: 0 }, regions: { covered: 0, count: 0 } };
}

function add(target, summary) {
  for (const key of ["lines", "functions", "regions"]) {
    target[key].covered += summary[key].covered;
    target[key].count += summary[key].count;
  }
}

function rustReport(path) {
  const report = readJson(path).data?.[0];
  if (!report?.totals || !Array.isArray(report.files)) throw new Error(`Invalid cargo llvm-cov report: ${path}`);
  const portable = empty();
  for (const file of report.files) {
    const normalized = file.filename.replaceAll("\\", "/");
    const crate = /(?:^|\/)crates\/([^/]+)\//.exec(normalized)?.[1];
    if (crate && PORTABLE_CRATES.has(crate)) add(portable, file.summary);
  }
  return { total: report.totals, portable };
}

function uiReport(path) {
  const total = readJson(path).total;
  if (!total?.lines || !total?.functions || !total?.branches) throw new Error(`Invalid Vitest coverage report: ${path}`);
  return total;
}

function metrics(workspacePath, shellPath, uiPath) {
  const workspace = rustReport(workspacePath);
  const shell = rustReport(shellPath);
  const ui = uiReport(uiPath);
  return {
    rustWorkspace: workspace.total,
    rustPortable: workspace.portable,
    rustShell: shell.total,
    ui: {
      lines: { covered: ui.lines.covered, count: ui.lines.total },
      functions: { covered: ui.functions.covered, count: ui.functions.total },
      branches: { covered: ui.branches.covered, count: ui.branches.total },
    },
  };
}

function cell(metric) {
  if (!metric || metric.count === 0) return "—";
  const percent = ((100 * metric.covered) / metric.count).toFixed(1);
  return `${percent}% (${metric.covered}/${metric.count})`;
}

function readEnv(path) {
  const result = {};
  for (const line of readFileSync(path, "utf8").split(/\r?\n/)) {
    if (!line) continue;
    const separator = line.indexOf("=");
    if (separator < 1) throw new Error(`Invalid metrics row in ${path}`);
    result[line.slice(0, separator)] = line.slice(separator + 1);
  }
  return result;
}

function numeric(metrics, name) {
  const value = Number(metrics[name]);
  if (!Number.isFinite(value)) throw new Error(`Missing or invalid ${name} metric`);
  return value;
}

function csvRow(entry) {
  return HISTORY_COLUMNS.map((column) => entry[column]).join(",");
}

function readHistory(path) {
  if (!existsSync(path)) return [];
  const [header, ...rows] = readFileSync(path, "utf8").trim().split(/\r?\n/);
  if (header !== HISTORY_COLUMNS.join(",")) throw new Error(`Invalid quality history header: ${path}`);
  return rows.filter(Boolean).map((row) => {
    const cells = row.split(",");
    if (cells.length !== HISTORY_COLUMNS.length) throw new Error(`Invalid quality history row: ${path}`);
    return Object.fromEntries(HISTORY_COLUMNS.map((column, index) => [column, cells[index]]));
  });
}

function appendHistory(path, entry) {
  const history = readHistory(path);
  const existing = history.findIndex((row) => row.run_id === entry.run_id);
  if (existing >= 0) history[existing] = entry;
  else history.push(entry);
  history.sort((left, right) => left.date.localeCompare(right.date) || Number(left.run_id) - Number(right.run_id));
  writeFileSync(path, `${HISTORY_COLUMNS.join(",")}\n${history.map(csvRow).join("\n")}\n`, "utf8");
  return history;
}

function xml(value) {
  return String(value).replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;").replaceAll("'", "&apos;");
}

function renderPanel({ x, y, width, height, title, series, entries }) {
  const plot = { x: x + 50, y: y + 49, width: width - 70, height: height - 87 };
  const bottom = plot.y + plot.height;
  const values = series.flatMap((item) => entries.map((entry) => Number(entry[item.key])));
  let min = Math.min(...values);
  let max = Math.max(...values);
  const span = Math.max(max - min, Math.abs(max) * 0.15, 1);
  min -= span * 0.08;
  max += span * 0.08;
  const px = (index) => plot.x + (entries.length < 2 ? plot.width / 2 : (index / (entries.length - 1)) * plot.width);
  const py = (value) => plot.y + ((max - value) / (max - min)) * plot.height;
  const pieces = [
    `<g role="group" aria-label="${xml(title)}">`,
    `<rect x="${x}" y="${y}" width="${width}" height="${height}" rx="12" fill="#ffffff" stroke="#d7dee8"/>`,
    `<text x="${x + 16}" y="${y + 25}" class="panel-title">${xml(title)}</text>`,
  ];

  let legendX = x + width - 16;
  for (const item of [...series].reverse()) {
    const labelWidth = item.label.length * 6 + 29;
    legendX -= labelWidth;
    pieces.push(`<line x1="${legendX}" y1="${y + 21}" x2="${legendX + 15}" y2="${y + 21}" stroke="${item.color}" stroke-width="2.5"/>`);
    pieces.push(`<text x="${legendX + 20}" y="${y + 25}" class="legend">${xml(item.label)}</text>`);
    legendX -= 10;
  }

  for (let tick = 0; tick <= 3; tick += 1) {
    const value = max - ((max - min) * tick) / 3;
    const tickY = plot.y + (plot.height * tick) / 3;
    pieces.push(`<line x1="${plot.x}" y1="${tickY}" x2="${plot.x + plot.width}" y2="${tickY}" stroke="#e7ebf1"/>`);
    pieces.push(`<text x="${plot.x - 8}" y="${tickY + 3}" text-anchor="end" class="axis">${value.toFixed(1)}</text>`);
  }

  for (const index of [...new Set([0, Math.floor((entries.length - 1) / 2), entries.length - 1])]) {
    pieces.push(`<text x="${px(index)}" y="${bottom + 20}" text-anchor="${index === 0 ? "start" : index === entries.length - 1 ? "end" : "middle"}" class="axis">${xml(entries[index].date)}</text>`);
  }

  for (const [seriesIndex, item] of series.entries()) {
    const points = entries.map((entry, index) => `${px(index).toFixed(1)},${py(Number(entry[item.key])).toFixed(1)}`);
    const latest = entries.at(-1);
    const dateAndRun = (entry) => `${entry.date} · run #${entry.run_id} · ${entry.commit}`;
    if (points.length > 1) pieces.push(`<polyline points="${points.join(" ")}" fill="none" stroke="${item.color}" stroke-width="2.5" stroke-linejoin="round" stroke-linecap="round"/>`);
    entries.forEach((entry, index) => {
      pieces.push(`<circle cx="${px(index).toFixed(1)}" cy="${py(Number(entry[item.key])).toFixed(1)}" r="2.7" fill="${item.color}"><title>${xml(`${dateAndRun(entry)} · ${item.label}: ${entry[item.key]}`)}</title></circle>`);
    });
    pieces.push(`<text x="${x + width - 16}" y="${y + height - 11 - seriesIndex * 13}" text-anchor="end" class="latest" fill="${item.color}">${xml(item.label)}: ${xml(latest[item.key])}</text>`);
  }
  pieces.push("</g>");
  return pieces.join("");
}

function renderHistoryChart(history) {
  const entries = history.slice(-90);
  const latest = entries.at(-1);
  const panels = [
    { x: 24, y: 86, title: "Engine quantum time (µs)", series: [{ key: "soak_p99_us", label: "p99", color: "#087f8c" }, { key: "soak_p999_us", label: "p99.9", color: "#c46a0b" }] },
    { x: 504, y: 86, title: "Engine live heap growth (KiB)", series: [{ key: "soak_heap_growth_kib", label: "heap", color: "#6c52a3" }] },
    { x: 24, y: 352, title: "Windows process memory growth (MiB)", series: [{ key: "soak_private_growth_mib", label: "private bytes", color: "#26734d" }] },
    { x: 504, y: 352, title: "Backend ready time (ms)", series: [{ key: "backend_ready_ms", label: "first response", color: "#2468a2" }] },
  ];
  const rendered = panels.map((panel) => renderPanel({ ...panel, width: 452, height: 242, entries })).join("");
  return [
    '<svg xmlns="http://www.w3.org/2000/svg" width="980" height="620" viewBox="0 0 980 620" role="img" aria-labelledby="chart-title chart-desc">',
    `<title id="chart-title">AudioRouter nightly quality history</title>`,
    `<desc id="chart-desc">Four historical timelines from ${entries.length} successful quality runs. Points include run and commit details in their tooltips; the complete dataset is stored in the linked CSV.</desc>`,
    '<style>text{font-family:system-ui,-apple-system,"Segoe UI",sans-serif}.heading{font-size:21px;font-weight:700;fill:#172033}.subtitle{font-size:12px;fill:#5c6879}.panel-title{font-size:13px;font-weight:650;fill:#172033}.axis{font-size:9px;fill:#6c7787}.legend{font-size:10px;fill:#495466}.latest{font-size:10px;font-weight:650}</style>',
    '<rect width="980" height="620" rx="16" fill="#f4f6f9"/>',
    '<text x="26" y="38" class="heading">AudioRouter · nightly quality history</text>',
    `<text x="26" y="62" class="subtitle">Last ${entries.length} successful runs · updated ${xml(latest.date)} · run and commit IDs are in the CSV</text>`,
    rendered,
    '<text x="26" y="613" class="subtitle">Hosted Windows runner measurements; compare trends cautiously across runner changes.</text>',
    "</svg>\n",
  ].join("\n");
}

function updateReadme(readmePath, summaryPath, soakPath, startupPath, sourceSha, runId, repository, serverUrl, historyPath, chartPath) {
  if (!/^[0-9a-f]{7,40}$/i.test(sourceSha)) throw new Error("Invalid source commit SHA");
  if (!/^\d+$/.test(runId)) throw new Error("Invalid workflow run ID");
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository)) throw new Error("Invalid repository name");
  const values = readJson(summaryPath);
  const soak = readEnv(soakPath);
  const startup = readEnv(startupPath);
  const date = new Date().toISOString().slice(0, 10);
  const entry = {
    date,
    run_id: runId,
    commit: sourceSha.slice(0, 8),
    soak_p99_us: String(numeric(soak, "soak_p99_us")),
    soak_p999_us: String(numeric(soak, "soak_p999_us")),
    soak_heap_growth_kib: String(numeric(soak, "soak_heap_growth_kib")),
    soak_private_growth_mib: String(numeric(soak, "soak_private_growth_mib")),
    backend_ready_ms: String(numeric(startup, "backend_ready_ms")),
  };
  const history = appendHistory(historyPath, entry);
  writeFileSync(chartPath, renderHistoryChart(history), "utf8");
  const commitUrl = `${serverUrl}/${repository}/commit/${sourceSha}`;
  const runUrl = `${serverUrl}/${repository}/actions/runs/${runId}`;
  const block = [
    `Updated ${date} (UTC) from [\`${sourceSha.slice(0, 8)}\`](${commitUrl}) · [quality run #${runId}](${runUrl}).`,
    "",
    "| Check | Latest result |",
    "| --- | --- |",
    `| Rust workspace coverage | ${cell(values.rustWorkspace.lines)} lines · ${cell(values.rustWorkspace.functions)} functions · ${cell(values.rustWorkspace.regions)} regions |`,
    `| Portable Rust coverage | ${cell(values.rustPortable.lines)} lines · ${cell(values.rustPortable.functions)} functions · ${cell(values.rustPortable.regions)} regions |`,
    `| Desktop shell coverage | ${cell(values.rustShell.lines)} lines · ${cell(values.rustShell.functions)} functions · ${cell(values.rustShell.regions)} regions |`,
    `| UI (Vitest) coverage | ${cell(values.ui.lines)} lines · ${cell(values.ui.functions)} functions · ${cell(values.ui.branches)} branches |`,
    `| Engine soak | PASS · p99 ${numeric(soak, "soak_p99_us").toFixed(0)} µs, p99.9 ${numeric(soak, "soak_p999_us").toFixed(0)} µs per quantum · in-process heap ${numeric(soak, "soak_heap_growth_kib").toFixed(1)} KiB · process heap ${numeric(soak, "soak_private_growth_mib").toFixed(1)} MiB · ${numeric(soak, "soak_quanta").toFixed(0)} quanta |`,
    `| Fresh-install backend readiness | ${numeric(startup, "backend_ready_ms").toFixed(0)} ms to first backend response on the hosted Windows runner; UI-ready time is not measured |`,
  ].join("\n");

  const readme = readFileSync(readmePath, "utf8");
  const start = readme.indexOf(START);
  const end = readme.indexOf(END);
  if (
    start < 0 ||
    end < 0 ||
    end < start ||
    readme.indexOf(START, start + START.length) >= 0 ||
    readme.indexOf(END, end + END.length) >= 0
  ) {
    throw new Error("README must contain exactly one correctly ordered quality-stats marker pair");
  }
  const next = `${readme.slice(0, start + START.length)}\n${block}\n${readme.slice(end)}`;
  writeFileSync(readmePath, next, "utf8");
}

const [command, ...args] = process.argv.slice(2);
if (command === "summarize" && args.length === 4) {
  writeFileSync(args[3], `${JSON.stringify(metrics(args[0], args[1], args[2]), null, 2)}\n`, "utf8");
} else if (command === "update" && args.length === 10) {
  updateReadme(...args);
} else {
  console.error("usage: publish-readme-coverage.mjs summarize <workspace.json> <shell.json> <ui-summary.json> <out.json> | update <README.md> <summary.json> <soak.env> <startup.env> <source-sha> <run-id> <repo> <server-url> <history.csv> <chart.svg>");
  process.exit(2);
}
