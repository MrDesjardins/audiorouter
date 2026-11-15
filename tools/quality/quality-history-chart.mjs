#!/usr/bin/env node
// Keep and draw the nightly quality history published on the `quality-data`
// branch. The rows come from the `quality-history-collected` artifact that the
// trend job has already accumulated in the Actions cache.
//
//   node tools/quality/quality-history-chart.mjs merge <published.csv|-> <collected.csv> <out.csv>
//   node tools/quality/quality-history-chart.mjs chart <history.csv> <chart.svg>
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export const HISTORY_COLUMNS = [
  "date",
  "commit",
  "rust_lines",
  "shell_lines",
  "ui_lines",
  "soak_p99_us",
  "soak_p999_us",
  "soak_heap_growth_kib",
  "soak_private_growth_mib",
  "backend_ready_ms",
];
const PLOT_COLUMNS = ["soak_p99_us", "soak_p999_us", "soak_heap_growth_kib", "soak_private_growth_mib", "backend_ready_ms"];

export function parseHistory(path) {
  const text = readFileSync(path, "utf8").trim();
  if (!text) throw new Error(`Empty quality history: ${path}`);
  const [header, ...rows] = text.split(/\r?\n/);
  if (header !== HISTORY_COLUMNS.join(",")) throw new Error(`Invalid quality history header: ${path}`);
  return rows
    .filter(Boolean)
    .map((row) => {
      const cells = row.split(",");
      if (cells.length !== HISTORY_COLUMNS.length) throw new Error(`Invalid quality history row in ${path}: ${row}`);
      return Object.fromEntries(HISTORY_COLUMNS.map((column, index) => [column, cells[index]]));
    });
}

function rowKey(entry) {
  return `${entry.date}|${entry.commit}`;
}

// The Actions cache can be evicted, so never let a short collected file
// truncate what was already published. Incoming rows win for the same day and
// commit, which makes a re-run refresh a day without duplicating it.
export function mergeHistory(published, collected) {
  const merged = new Map(published.map((entry) => [rowKey(entry), entry]));
  for (const entry of collected) merged.set(rowKey(entry), entry);
  return [...merged.values()].sort((left, right) => left.date.localeCompare(right.date) || left.commit.localeCompare(right.commit));
}

export function writeHistory(path, history) {
  writeFileSync(path, `${HISTORY_COLUMNS.join(",")}\n${history.map((entry) => HISTORY_COLUMNS.map((column) => entry[column]).join(",")).join("\n")}\n`, "utf8");
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
    const label = (entry) => `${entry.date}${entry.commit ? ` · ${entry.commit}` : ""}`;
    if (points.length > 1) pieces.push(`<polyline points="${points.join(" ")}" fill="none" stroke="${item.color}" stroke-width="2.5" stroke-linejoin="round" stroke-linecap="round"/>`);
    entries.forEach((entry, index) => {
      pieces.push(`<circle cx="${px(index).toFixed(1)}" cy="${py(Number(entry[item.key])).toFixed(1)}" r="2.7" fill="${item.color}"><title>${xml(`${label(entry)} · ${item.label}: ${entry[item.key]}`)}</title></circle>`);
    });
    pieces.push(`<text x="${x + width - 16}" y="${y + height - 11 - seriesIndex * 13}" text-anchor="end" class="latest" fill="${item.color}">${xml(item.label)}: ${xml(latest[item.key])}</text>`);
  }
  pieces.push("</g>");
  return pieces.join("");
}

export function renderHistoryChart(history) {
  const entries = history
    .filter((entry) => PLOT_COLUMNS.every((column) => entry[column] !== "" && Number.isFinite(Number(entry[column]))))
    .slice(-90);
  if (entries.length === 0) throw new Error("No complete quality history rows to chart");
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
    `<desc id="chart-desc">Four historical timelines from ${entries.length} recorded daily quality runs. Points include the commit in their tooltips; the complete dataset, including coverage, is stored in the linked CSV.</desc>`,
    '<style>text{font-family:system-ui,-apple-system,"Segoe UI",sans-serif}.heading{font-size:21px;font-weight:700;fill:#172033}.subtitle{font-size:12px;fill:#5c6879}.panel-title{font-size:13px;font-weight:650;fill:#172033}.axis{font-size:9px;fill:#6c7787}.legend{font-size:10px;fill:#495466}.latest{font-size:10px;font-weight:650}</style>',
    '<rect width="980" height="620" rx="16" fill="#f4f6f9"/>',
    '<text x="26" y="38" class="heading">AudioRouter · nightly quality history</text>',
    `<text x="26" y="62" class="subtitle">Last ${entries.length} recorded daily runs · updated ${xml(latest.date)} · commit IDs are in the CSV</text>`,
    rendered,
    '<text x="26" y="613" class="subtitle">Hosted Windows runner measurements; compare trends cautiously across runner changes.</text>',
    "</svg>\n",
  ].join("\n");
}

const [command, ...args] = process.argv.slice(2);
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  if (command === "merge" && args.length === 3) {
    const published = args[0] === "-" || !existsSync(args[0]) ? [] : parseHistory(args[0]);
    writeHistory(args[2], mergeHistory(published, parseHistory(args[1])));
  } else if (command === "chart" && args.length === 2) {
    writeFileSync(args[1], renderHistoryChart(parseHistory(args[0])), "utf8");
  } else {
    console.error("usage: quality-history-chart.mjs merge <published.csv|-> <collected.csv> <out.csv> | chart <history.csv> <out.svg>");
    process.exitCode = 2;
  }
}
