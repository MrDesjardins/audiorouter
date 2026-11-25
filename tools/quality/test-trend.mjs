#!/usr/bin/env node
// First-pass test metrics for the nightly trend table (application-quality Q5.1).
//
//   node tools/quality/test-trend.mjs playwright [--env <file>] [--run-url <url>] <report.json>...
//
// A retried green test is not a pass. Playwright reports `spec.status` as
// `flaky` when the test failed and a later attempt passed, and the retry would
// hide the first attempt from `stats.unexpected`. This tool counts the first
// failed attempt separately (`firstPassFailures`) from the final outcome
// (`failed`), and refuses to report a retry as first-pass success. Each report
// is one CI shard; the longest shard is the suite's wall-clock duration.
import { existsSync, readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { basename, dirname } from "node:path";
import { pathToFileURL } from "node:url";

const FAILED_FIRST = new Set(["failed", "timedOut", "interrupted"]);

function walkSuite(suite, visit) {
  for (const spec of suite?.specs ?? []) visit(spec);
  for (const child of suite?.suites ?? []) walkSuite(child, visit);
}

function entryOf(spec) {
  const file = spec.file ?? spec.tests?.[0]?.file ?? "";
  return { title: spec.title, file, line: spec.line, column: spec.column };
}

export function summarizeReports(reports) {
  const summary = {
    reports: reports.length,
    total: 0,
    expected: 0,
    skipped: 0,
    flaky: 0,
    failed: 0,
    firstPassFailures: 0,
    retried: 0,
    durationMs: 0,
    firstPassFailureTests: [],
    flakyTests: [],
    failedTests: [],
  };
  for (const report of reports) {
    summary.durationMs = Math.max(summary.durationMs, Number(report?.stats?.duration ?? 0));
    for (const suite of report?.suites ?? []) {
      walkSuite(suite, spec => {
        for (const test of spec?.tests ?? []) {
          summary.total += 1;
          if (test.status === "expected") summary.expected += 1;
          else if (test.status === "skipped") summary.skipped += 1;
          else if (test.status === "flaky") {
            summary.flaky += 1;
            summary.flakyTests.push(entryOf(spec));
          } else if (test.status === "unexpected") {
            summary.failed += 1;
            summary.failedTests.push(entryOf(spec));
          }
          const results = test.results ?? [];
          if (results.length > 1 || results.some(result => Number(result.retry) > 0)) summary.retried += 1;
          // `test.fail()` has expectedStatus "failed": its failed attempt is
          // the expected result, not a first-pass failure.
          if (test.expectedStatus === "passed" && FAILED_FIRST.has(results[0]?.status)) {
            summary.firstPassFailures += 1;
            summary.firstPassFailureTests.push(entryOf(spec));
          }
        }
      });
    }
  }
  return summary;
}

function label(entry) {
  return entry.file ? `${entry.file}:${entry.line} › ${entry.title}` : entry.title;
}

export function formatSummary(summary) {
  const lines = [
    "## Edge suite (Playwright)",
    "",
    "| Metric | Value |",
    "| --- | ---: |",
    `| Report files (shards) | ${summary.reports} |`,
    `| Tests | ${summary.total} |`,
    `| Passed first try | ${summary.expected} |`,
    `| First-pass failures (green retry included) | ${summary.firstPassFailures} |`,
    `| Retried tests | ${summary.retried} |`,
    `| Flaky (passed only after a retry) | ${summary.flaky} |`,
    `| Failed after every attempt | ${summary.failed} |`,
    `| Skipped | ${summary.skipped} |`,
    `| Longest shard | ${(summary.durationMs / 1000).toFixed(1)} s |`,
    "",
  ];
  if (summary.firstPassFailures > 0 && summary.flaky > 0) {
    lines.push(
      summary.failed > 0
        ? `${summary.flaky} first-pass failure(s) passed only on retry and ${summary.failed} failed every attempt; a green retry is not counted as first-pass success.`
        : `Every first-pass failure (${summary.firstPassFailures}) passed on retry; the retry did not erase the first failure.`,
      "",
    );
  } else if (summary.firstPassFailures > 0) {
    lines.push(`No retry hid a first attempt; all ${summary.firstPassFailures} first-pass failure(s) also failed by the final attempt.`, "");
  } else {
    lines.push("No test failed its first attempt.", "");
  }
  const lists = [
    ["First-pass failures", summary.firstPassFailureTests],
    ["Flaky tests", summary.flakyTests],
    ["Failed tests", summary.failedTests],
  ];
  for (const [title, entries] of lists) {
    if (entries.length === 0) continue;
    lines.push("", `### ${title}`, "");
    for (const entry of entries.slice(0, 20)) lines.push(`- ${label(entry)}`);
    if (entries.length > 20) lines.push(`- …and ${entries.length - 20} more`);
  }
  return `${lines.join("\n")}\n`;
}

export function envLines(summary, runUrl) {
  const lines = [
    `browser_total=${summary.total}`,
    `browser_first_pass_failures=${summary.firstPassFailures}`,
    `browser_retried_tests=${summary.retried}`,
    `browser_flaky=${summary.flaky}`,
    `browser_failed=${summary.failed}`,
    `browser_skipped=${summary.skipped}`,
    `browser_duration_s=${(summary.durationMs / 1000).toFixed(1)}`,
  ];
  if (runUrl) lines.push(`run_url=${runUrl}`);
  return `${lines.join("\n")}\n`;
}

export function runPlaywright(reportPaths, { envFile, runUrl, expectShards } = {}) {
  const expected = expectShards ? expectShards.map(String) : [];
  const valid = new Map();
  const unusable = [];
  for (const path of reportPaths) {
    try {
      const report = JSON.parse(readFileSync(path, "utf8"));
      if (!Array.isArray(report?.suites) || !report?.stats || typeof report.stats !== "object") {
        unusable.push(path);
        continue;
      }
      const shard = basename(path).match(/^report-(\d+)\.json$/i)?.[1];
      if (expected.length && (!shard || !expected.includes(shard))) {
        unusable.push(path);
        continue;
      }
      valid.set(shard ?? path, report);
    } catch {
      unusable.push(path);
    }
  }
  if (expected.length) {
    const missing = expected.filter((shard) => !valid.has(shard));
    if (missing.length) return { summary: null, missing, unusable };
  } else if (unusable.length) {
    throw new Error(`Unreadable Playwright report: ${unusable.join(", ")}`);
  }
  const summary = summarizeReports([...valid.values()]);
  if (envFile) {
    mkdirSync(dirname(envFile), { recursive: true });
    writeFileSync(envFile, envLines(summary, runUrl));
  }
  return { summary, missing: [], unusable: expected.length ? unusable : [] };
}

export function formatNotMeasured({ missing, unusable }) {
  const lines = [
    "## Edge suite (Playwright)",
    "",
    "Not measured: the shard reports are incomplete, so no totals or first-pass counts are reported; `browser_*` columns stay blank.",
    "",
  ];
  for (const shard of missing) lines.push(`- Missing shard ${shard} (\`report-${shard}.json\`)`);
  for (const path of unusable) lines.push(`- Unreadable or truncated report: ${path}`);
  return `${lines.join("\n")}\n`;
}

function parseArgs(args) {
  const values = { reports: [] };
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--env") values.envFile = args[++index];
    else if (arg === "--run-url") values.runUrl = args[++index];
    else if (arg === "--expect-shards") {
      const shards = (args[++index] ?? "").split(",").map((value) => value.trim()).filter(Boolean);
      if (!shards.length || shards.some((shard) => !/^\d+$/.test(shard))) throw Error("--expect-shards needs a comma-separated list of shard numbers");
      values.expectShards = shards;
    } else if (arg.startsWith("--")) throw Error(`Unknown option: ${arg}`);
    else values.reports.push(arg);
  }
  return values;
}

const [command, ...args] = process.argv.slice(2);
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    if (command !== "playwright") {
      console.error("usage: test-trend.mjs playwright [--env <file>] [--run-url <url>] [--expect-shards 1,2,3,4] <report-<shard>.json>...");
      process.exit(2);
    }
    const options = parseArgs(args);
    if (options.reports.length === 0 && !options.expectShards) {
      console.error("usage: test-trend.mjs playwright [--env <file>] [--run-url <url>] [--expect-shards 1,2,3,4] <report-<shard>.json>...");
      process.exit(2);
    }
    const missing = options.reports.filter(path => !existsSync(path));
    if (missing.length && !options.expectShards) throw Error(`Missing Playwright report: ${missing.join(", ")}`);
    const reports = options.reports.filter(path => existsSync(path));
    const result = runPlaywright(reports, options);
    console.log(result.summary ? formatSummary(result.summary) : formatNotMeasured(result));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 2;
  }
}