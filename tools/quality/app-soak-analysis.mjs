#!/usr/bin/env node
// Analyze an app-soak samples.csv with the same limits the live
// tools/quality/run-app-soak.ps1 soak uses. Q5.4's synthetic soak reuses this
// module so its verdicts are testable without launching a shell.
//
//   node tools/quality/app-soak-analysis.mjs --samples <csv> [--summary <md>]
//        [--warm-up <seconds>] [--mode short|endurance] [--activity <json>]
//        [--isolation <text>] [--shell <exe>]
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export const THRESHOLDS = {
  shellPrivateMiBPerHour: { pass: 5, review: 20 },
  webviewPrivateMiBPerHour: { pass: 20, review: 60 },
  handlesPerHour: { pass: 50, review: 200 },
  threadSpreadAfterWarmUp: 8,
  logHistoryHours: 24,
  logFileBytes: 5 * 1024 * 1024,
};

const number = value => {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
};

export function parseSamplesCsv(text) {
  const [headerLine, ...lines] = text.trim().split(/\r?\n/);
  const header = headerLine.split(",");
  return lines.filter(Boolean).map(line => {
    const cells = line.split(",");
    if (cells.length !== header.length) throw new Error(`Invalid samples row: ${line}`);
    const raw = Object.fromEntries(header.map((column, index) => [column, cells[index]]));
    const bool = value => value === "True" || value === "true" || value === "1";
    return {
      elapsed: number(raw.elapsed_s),
      alive: bool(raw.shell_alive),
      shellPrivate: number(raw.shell_private_mib),
      shellWs: number(raw.shell_ws_mib),
      handles: number(raw.shell_handles),
      threads: number(raw.shell_threads),
      shellCpu: number(raw.shell_cpu_s),
      webCount: number(raw.webview_count),
      webPrivate: number(raw.webview_private_mib),
      webCpu: number(raw.webview_cpu_s),
      backendLog: number(raw.backend_log_written) ?? 0,
      shellLog: number(raw.shell_log_written) ?? 0,
      backendErrors: number(raw.backend_errors) ?? 0,
      shellErrors: number(raw.shell_errors) ?? 0,
    };
  });
}

export function slopePerHour(points, field) {
  if (!points || points.length < 3) return null;
  const n = points.length;
  const meanX = points.reduce((sum, point) => sum + point.elapsed, 0) / n;
  const values = points.map(point => point[field]);
  if (values.some(value => !Number.isFinite(value))) return null;
  const meanY = values.reduce((sum, value) => sum + value, 0) / n;
  let numerator = 0;
  let denominator = 0;
  points.forEach((point, index) => {
    const dx = point.elapsed - meanX;
    numerator += dx * (values[index] - meanY);
    denominator += dx * dx;
  });
  if (denominator === 0) return 0;
  return (numerator / denominator) * 3600;
}

export function grade(value, limit) {
  if (value === null || value === undefined) return "REVIEW (too few samples)";
  if (value < limit.pass) return "PASS";
  if (value < limit.review) return "REVIEW";
  return "FAIL";
}

function logHistoryHours(writtenBytes, elapsedSeconds, thresholds) {
  if (!(elapsedSeconds > 0)) return null;
  const bytesPerHour = writtenBytes / (elapsedSeconds / 3600);
  if (!(bytesPerHour > 0)) return Infinity;
  return (2 * thresholds.logFileBytes) / bytesPerHour;
}

export function analyzeSamples(rows, { warmUpSeconds = 60, thresholds = THRESHOLDS } = {}) {
  if (!rows.length) throw new Error("No samples to analyze");
  const steady = rows.filter(row => row.elapsed >= warmUpSeconds && row.alive);
  const elapsedSeconds = rows.at(-1).elapsed;
  const shellPrivate = slopePerHour(steady, "shellPrivate");
  const webviewPrivate = slopePerHour(steady, "webPrivate");
  const handles = slopePerHour(steady, "handles");
  const threads = steady.length
    ? Math.max(...steady.map(row => row.threads)) - Math.min(...steady.map(row => row.threads))
    : null;
  // Measure the log rate after warm-up, not from launch: first-run startup
  // writes would otherwise make a short run look like a log flood.
  const logStart = steady[0] ?? null;
  const logEnd = rows.at(-1);
  const logSeconds = logStart ? logEnd.elapsed - logStart.elapsed : null;
  const backendGrowth = logStart ? logEnd.backendLog - logStart.backendLog : null;
  const shellGrowth = logStart ? logEnd.shellLog - logStart.shellLog : null;
  const backendHours = logSeconds !== null && logSeconds >= 30 && backendGrowth >= 0
    ? logHistoryHours(backendGrowth, logSeconds, thresholds)
    : null;
  const shellHours = logSeconds !== null && logSeconds >= 30 && shellGrowth >= 0
    ? logHistoryHours(shellGrowth, logSeconds, thresholds)
    : null;
  const alive = Boolean(rows.at(-1).alive);
  const checks = [
    { key: "alive", label: "Shell alive to the end", value: alive, verdict: alive ? "PASS" : "FAIL", text: alive ? "yes" : "no" },
    {
      key: "shell_private",
      label: "Shell private bytes slope after warm-up",
      value: shellPrivate,
      verdict: grade(shellPrivate, thresholds.shellPrivateMiBPerHour),
      text: `${shellPrivate === null ? "–" : shellPrivate.toFixed(2)} MiB/h`,
    },
    {
      key: "webview_private",
      label: "WebView private bytes slope (all processes)",
      value: webviewPrivate,
      verdict: grade(webviewPrivate, thresholds.webviewPrivateMiBPerHour),
      text: `${webviewPrivate === null ? "–" : webviewPrivate.toFixed(2)} MiB/h`,
    },
    {
      key: "handles",
      label: "Shell handles slope",
      value: handles,
      verdict: grade(handles, thresholds.handlesPerHour),
      text: `${handles === null ? "–" : handles.toFixed(1)}/h`,
    },
    {
      key: "threads",
      label: "Shell threads spread after warm-up",
      value: threads,
      verdict: threads === null ? "REVIEW (too few samples)" : threads <= thresholds.threadSpreadAfterWarmUp ? "PASS" : "REVIEW",
      text: threads === null ? "–" : String(threads),
    },
    {
      key: "backend_log",
      label: "Backend log history at this rate",
      value: backendHours,
      verdict: backendHours === null ? "REVIEW (too few samples)" : backendHours >= thresholds.logHistoryHours ? "PASS" : "FAIL",
      text: backendHours === null || backendHours === Infinity ? "–" : `${backendHours.toFixed(1)} h`,
    },
    {
      key: "shell_log",
      label: "Shell log history at this rate",
      value: shellHours,
      verdict: shellHours === null ? "REVIEW (too few samples)" : shellHours >= thresholds.logHistoryHours ? "PASS" : "FAIL",
      text: shellHours === null || shellHours === Infinity ? "–" : `${shellHours.toFixed(1)} h`,
    },
  ];
  const failures = checks.filter(check => check.verdict === "FAIL").length;
  const reviews = checks.filter(check => check.verdict.startsWith("REVIEW")).length;
  return {
    rows: rows.length,
    elapsedSeconds,
    steadySamples: steady.length,
    warmUpSeconds,
    checks,
    overall: failures ? "FAIL" : reviews ? "REVIEW" : "PASS",
    last: rows.at(-1),
  };
}

export function formatSummary(analysis, { mode = "short", shell = "", isolation = "", activity = null } = {}) {
  const elapsed = analysis.elapsedSeconds;
  const minutes = (elapsed / 60).toFixed(1);
  const lines = [
    `# App soak (${mode})`,
    "",
    `Shell: ${shell || "unknown"}; ${minutes} min, ${analysis.rows} samples, warm-up ${analysis.warmUpSeconds} s, ${analysis.steadySamples} steady samples.`,
  ];
  if (isolation) lines.push(`Isolation: ${isolation}.`);
  lines.push("", "| Check | Value | Verdict |", "| --- | --- | --- |");
  for (const check of analysis.checks) lines.push(`| ${check.label} | ${check.text} | ${check.verdict} |`);
  lines.push(
    "",
    `| Last shell private | ${analysis.last.shellPrivate?.toFixed(1) ?? "–"} MiB | INFO |`,
    `| Last WebView private (${analysis.last.webCount} processes) | ${analysis.last.webPrivate?.toFixed(1) ?? "–"} MiB | INFO |`,
    `| Last shell handles / threads | ${analysis.last.handles} / ${analysis.last.threads} | INFO |`,
    `| Error lines at the end (backend / shell) | ${analysis.last.backendErrors} / ${analysis.last.shellErrors} | INFO |`,
  );
  if (mode === "short") {
    const alive = analysis.checks.find(check => check.key === "alive")?.verdict ?? "REVIEW";
    const overall = alive === "FAIL" ? "FAIL (shell died)" : "INFO (short run; the per-hour verdicts are endurance limits)";
    lines.push("", `**Overall: ${overall}.**`);
    lines.push("", "This is a short synthetic run, not endurance evidence. Slopes are directional; the thresholds are the live soak's per-hour limits.");
  } else {
    lines.push("", `**Overall: ${analysis.overall}.**`);
  }
  if (activity) {
    lines.push(
      "",
      "## Synthetic activity",
      "",
      `| Graph edits | ${activity.edits ?? "–"} |`,
      `| UI reload restarts | ${activity.reloads ?? "–"} |`,
      `| Backend RPC errors | ${activity.rpcErrors ?? "–"} |`,
      `| Uncaught page errors | ${activity.pageErrors?.length ?? "–"} |`,
      `| Backend edit to 64 painted cards | ${activity.editTo64PaintedMs == null ? "–" : `${activity.editTo64PaintedMs.toFixed(0)} ms`} |`,
      `| Post-GC JS heap (start → end) | ${activity.jsHeapStartMiB?.toFixed(1) ?? "–"} → ${activity.jsHeapEndMiB?.toFixed(1) ?? "–"} MiB |`,
      `| Shell/WebView private bytes at end | ${analysis.last.shellPrivate?.toFixed(1) ?? "–"} / ${analysis.last.webPrivate?.toFixed(1) ?? "–"} MiB |`,
      "",
      "Backend edit to 64 painted cards times the `nodes.add` + `nodes.set` RPCs and the UI refresh until 64 cards exist; it is not a user input-to-paint time and is not comparable to a route-harness page-load paint measurement.",
    );
    if (activity.pageErrors?.length) for (const error of activity.pageErrors.slice(0, 5)) lines.push(`- Page error: ${error}`);
  }
  return lines.join("\n") + "\n";
}

function parseArgs(args) {
  const values = {};
  for (let index = 0; index < args.length; index += 1) {
    const key = args[index];
    const value = args[index + 1];
    if (!value || value.startsWith("--")) throw new Error(`Missing value for ${key}`);
    values[key] = value;
    index += 1;
  }
  return values;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const args = parseArgs(process.argv.slice(2));
    if (!args["--samples"]) throw new Error("Required: --samples <csv>");
    const rows = parseSamplesCsv(readFileSync(args["--samples"], "utf8"));
    const activityPath = args["--activity"];
    const activity = activityPath && existsSync(activityPath) ? JSON.parse(readFileSync(activityPath, "utf8")) : null;
    const analysis = analyzeSamples(rows, { warmUpSeconds: Number(args["--warm-up"] ?? 60) });
    const mode = args["--mode"] ?? "short";
    const summary = formatSummary(analysis, {
      mode,
      shell: args["--shell"] ?? "",
      isolation: args["--isolation"] ?? "",
      activity,
    });
    if (args["--summary"]) writeFileSync(args["--summary"], summary);
    console.log(summary);
    const aliveFailed = analysis.checks.find(check => check.key === "alive")?.verdict === "FAIL";
    if (aliveFailed || (mode === "endurance" && analysis.overall === "FAIL")) process.exitCode = 1;
  } catch (error) {
    console.error(error.message);
    process.exitCode = 2;
  }
}