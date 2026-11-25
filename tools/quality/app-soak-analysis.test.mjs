import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { analyzeSamples, formatSummary, grade, parseSamplesCsv, slopePerHour } from "./app-soak-analysis.mjs";

function samples({ count = 21, step = 30, shellPrivate = () => 100, webPrivate = () => 300, handles = () => 500, threads = () => 20, backendLog = () => 50000, shellLog = () => 50000 } = {}) {
  return Array.from({ length: count }, (_, index) => ({
    elapsed: index * step,
    alive: true,
    shellPrivate: shellPrivate(index),
    webPrivate: webPrivate(index),
    webCount: 4,
    handles: handles(index),
    threads: threads(index),
    backendLog: backendLog(index),
    shellLog: shellLog(index),
    backendErrors: 0,
    shellErrors: 0,
  }));
}

test("flat passing data passes every check", () => {
  const analysis = analyzeSamples(samples(), { warmUpSeconds: 60 });
  assert.equal(analysis.overall, "PASS");
  assert.ok(analysis.checks.every(check => check.verdict === "PASS"));
  assert.equal(analysis.steadySamples, 19);
});

test("a handle leak fails and a private-bytes drift is reviewed", () => {
  const analysis = analyzeSamples(
    samples({ shellPrivate: index => 100 + index * 0.08, handles: index => 500 + index * 2 }),
    { warmUpSeconds: 60 },
  );
  assert.equal(analysis.overall, "FAIL");
  const privateCheck = analysis.checks.find(check => check.key === "shell_private");
  assert.equal(privateCheck.verdict, "REVIEW");
  assert.equal(analysis.checks.find(check => check.key === "handles").verdict, "FAIL");
});

test("a log flood is a failure because the history falls below 24 h", () => {
  const analysis = analyzeSamples(samples({ backendLog: index => index * 200_000, shellLog: index => index * 200_000 }), { warmUpSeconds: 60 });
  assert.equal(analysis.checks.find(check => check.key === "backend_log").verdict, "FAIL");
  assert.equal(analysis.checks.find(check => check.key === "shell_log").verdict, "FAIL");
});

test("a dead shell fails", () => {
  const rows = samples();
  rows.at(-1).alive = false;
  assert.equal(analyzeSamples(rows, { warmUpSeconds: 60 }).checks.find(check => check.key === "alive").verdict, "FAIL");
});

test("too few steady samples review instead of inventing a slope", () => {
  const analysis = analyzeSamples(samples({ count: 2, step: 5, backendLog: () => 0, shellLog: () => 0 }), { warmUpSeconds: 60 });
  assert.equal(analysis.checks.find(check => check.key === "shell_private").verdict, "REVIEW (too few samples)");
  assert.equal(analysis.overall, "REVIEW");
});

test("slope and grade use the live soak's limits", () => {
  assert.equal(slopePerHour(samples({ shellPrivate: index => index * 2 }), "shellPrivate"), 240);
  assert.equal(grade(4.9, { pass: 5, review: 20 }), "PASS");
  assert.equal(grade(10, { pass: 5, review: 20 }), "REVIEW");
  assert.equal(grade(20, { pass: 5, review: 20 }), "FAIL");
  assert.match(grade(null, { pass: 5, review: 20 }), /^REVIEW/);
});

test("samples CSV from the PowerShell soak parses Windows booleans and numbers", () => {
  const csv = [
    "utc,elapsed_s,shell_alive,shell_private_mib,shell_ws_mib,shell_handles,shell_threads,shell_cpu_s,webview_count,webview_private_mib,webview_cpu_s,backend_log_written,shell_log_written,backend_errors,shell_errors",
    "2026-11-24T01:00:00Z,0,True,100.0,200.0,500,20,1.0,4,300.0,2.0,1000,2000,0,0",
    "2026-11-24T01:00:30Z,30,True,100.1,200.1,500,20,1.1,4,300.1,2.1,2000,3000,0,1",
  ].join("\n");
  const [first, second] = parseSamplesCsv(csv);
  assert.equal(first.alive, true);
  assert.equal(first.shellPrivate, 100);
  assert.equal(second.elapsed, 30);
  assert.equal(second.backendLog, 2000);
  assert.equal(second.shellErrors, 1);
});

test("the summary says a short run is not endurance", () => {
  const analysis = analyzeSamples(samples(), { warmUpSeconds: 60 });
  const markdown = formatSummary(analysis, {
    mode: "short",
    shell: "C:\\shell.exe",
    isolation: "no file changed",
    activity: { edits: 42, reloads: 2, rpcErrors: 0, pageErrors: [], editTo64PaintedMs: 812, jsHeapStartMiB: 40, jsHeapEndMiB: 42 },
  });
  assert.match(markdown, /not endurance evidence/);
  assert.match(markdown, /Overall: INFO/);
  assert.match(markdown, /Graph edits \| 42/);
  assert.match(markdown, /Backend edit to 64 painted cards \| 812 ms/);
  assert.match(markdown, /not comparable to a route-harness page-load paint measurement/);
});

test("the command can fail on a leaking sample file", () => {
  const tool = fileURLToPath(new URL("./app-soak-analysis.mjs", import.meta.url));
  const dir = mkdtempSync(join(tmpdir(), "app-soak-analysis-"));
  try {
    const rows = samples({ handles: index => 500 + index * 2 });
    const header = "utc,elapsed_s,shell_alive,shell_private_mib,shell_ws_mib,shell_handles,shell_threads,shell_cpu_s,webview_count,webview_private_mib,webview_cpu_s,backend_log_written,shell_log_written,backend_errors,shell_errors";
    const csv = [
      header,
      ...rows.map(row => `2026-11-24T01:00:00Z,${row.elapsed},True,${row.shellPrivate},200,${row.handles},${row.threads},1,4,${row.webPrivate},2,${row.backendLog},${row.shellLog},0,0`),
    ].join("\n");
    writeFileSync(join(dir, "samples.csv"), csv);
    const result = spawnSync(process.execPath, [tool, "--samples", join(dir, "samples.csv"), "--warm-up", "60", "--mode", "endurance", "--summary", join(dir, "summary.md")], { encoding: "utf8" });
    assert.equal(result.status, 1);
    assert.match(readFileSync(join(dir, "summary.md"), "utf8"), /Overall: FAIL/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("a short run reports the same leak but is not marked as endurance failure", () => {
  const tool = fileURLToPath(new URL("./app-soak-analysis.mjs", import.meta.url));
  const dir = mkdtempSync(join(tmpdir(), "app-soak-analysis-"));
  try {
    const rows = samples({ handles: index => 500 + index * 2 });
    const header = "utc,elapsed_s,shell_alive,shell_private_mib,shell_ws_mib,shell_handles,shell_threads,shell_cpu_s,webview_count,webview_private_mib,webview_cpu_s,backend_log_written,shell_log_written,backend_errors,shell_errors";
    const csv = [
      header,
      ...rows.map(row => `2026-11-24T01:00:00Z,${row.elapsed},True,${row.shellPrivate},200,${row.handles},${row.threads},1,4,${row.webPrivate},2,${row.backendLog},${row.shellLog},0,0`),
    ].join("\n");
    writeFileSync(join(dir, "samples.csv"), csv);
    const result = spawnSync(process.execPath, [tool, "--samples", join(dir, "samples.csv"), "--warm-up", "60", "--summary", join(dir, "summary.md")], { encoding: "utf8" });
    assert.equal(result.status, 0);
    const summary = readFileSync(join(dir, "summary.md"), "utf8");
    assert.match(summary, /Shell handles slope \| 240\.0\/h \| FAIL \|/);
    assert.match(summary, /Overall: INFO \(short run/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
