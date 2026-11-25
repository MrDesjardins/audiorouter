import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { envLines, formatSummary, summarizeReports } from "./test-trend.mjs";

const fixture = name => fileURLToPath(new URL(`./testdata/playwright/${name}.json`, import.meta.url));
const read = name => JSON.parse(readFileSync(fixture(name), "utf8"));

test("a steady passing run has no first-pass failures", () => {
  const summary = summarizeReports([read("passing")]);
  assert.deepEqual(
    { total: summary.total, expected: summary.expected, firstPassFailures: summary.firstPassFailures, retried: summary.retried, failed: summary.failed },
    { total: 2, expected: 2, firstPassFailures: 0, retried: 0, failed: 0 },
  );
  assert.equal(summary.durationMs, 60000);
});

test("a green retry does not erase the first attempt's failure", () => {
  const summary = summarizeReports([read("retry-passing")]);
  assert.deepEqual(
    {
      total: summary.total,
      expected: summary.expected,
      firstPassFailures: summary.firstPassFailures,
      retried: summary.retried,
      flaky: summary.flaky,
      failed: summary.failed,
    },
    { total: 3, expected: 2, firstPassFailures: 1, retried: 1, flaky: 1, failed: 0 },
  );
  assert.deepEqual(summary.firstPassFailureTests.map(entry => entry.title), ["a flaky pass that fails its first attempt"]);
  const markdown = formatSummary(summary);
  assert.match(markdown, /\| First-pass failures \(green retry included\) \| 1 \|/);
  assert.match(markdown, /\| Flaky \(passed only after a retry\) \| 1 \|/);
  assert.match(markdown, /e2e\/retry-passing\.pw\.ts:7 › a flaky pass/);
});

test("a test that fails every attempt is both a first-pass failure and failed", () => {
  const summary = summarizeReports([read("failing")]);
  assert.equal(summary.firstPassFailures, 1);
  assert.equal(summary.retried, 1);
  assert.equal(summary.failed, 1);
  assert.equal(summary.flaky, 0);
  assert.equal(summary.expected, 1);
});

test("an expected failure and a skip are not first-pass failures", () => {
  const summary = summarizeReports([read("expected-failure")]);
  assert.equal(summary.total, 2);
  assert.equal(summary.firstPassFailures, 0);
  assert.equal(summary.retried, 0);
  assert.equal(summary.failed, 0);
  assert.equal(summary.skipped, 1);
});

test("nested suites are counted", () => {
  const summary = summarizeReports([read("nested")]);
  assert.equal(summary.total, 1);
  assert.equal(summary.expected, 1);
});

test("separate shard reports aggregate and the longest shard is the duration", () => {
  const summary = summarizeReports([read("passing"), read("failing")]);
  assert.equal(summary.reports, 2);
  assert.equal(summary.total, 4);
  assert.equal(summary.firstPassFailures, 1);
  assert.equal(summary.failed, 1);
  assert.equal(summary.durationMs, 62000);
});

test("env lines carry the first-pass failure and the run link", () => {
  const summary = summarizeReports([read("retry-passing")]);
  const lines = envLines(summary, "https://github.com/owner/repo/actions/runs/1").trim().split("\n");
  assert.ok(lines.includes("browser_first_pass_failures=1"));
  assert.ok(lines.includes("browser_failed=0"));
  assert.ok(lines.includes("browser_retried_tests=1"));
  assert.ok(lines.includes("browser_flaky=1"));
  assert.ok(lines.includes("browser_duration_s=61.0"));
  assert.ok(lines.includes("run_url=https://github.com/owner/repo/actions/runs/1"));
});

test("the command writes the env file and prints the summary", () => {
  const tool = fileURLToPath(new URL("./test-trend.mjs", import.meta.url));
  const dir = mkdtempSync(join(tmpdir(), "test-trend-"));
  try {
    const envFile = join(dir, "browser.env");
    const result = spawnSync(process.execPath, [tool, "playwright", "--env", envFile, "--run-url", "https://example.test/run/1", fixture("failing")], { encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /First-pass failures/);
    assert.match(readFileSync(envFile, "utf8"), /^browser_first_pass_failures=1$/m);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("a missing report is refused instead of reporting zero failures", () => {
  const tool = fileURLToPath(new URL("./test-trend.mjs", import.meta.url));
  const result = spawnSync(process.execPath, [tool, "playwright", join(tmpdir(), "missing-playwright-report.json")], { encoding: "utf8" });
  assert.equal(result.status, 2);
  assert.match(result.stderr, /Missing Playwright report/);
});

function shardDir(dir, names) {
  const copies = new Map([
    ["passing", fixture("passing")],
    ["retry-passing", fixture("retry-passing")],
    ["failing", fixture("failing")],
  ]);
  return names.map((name, index) => {
    const target = join(dir, `report-${index + 1}.json`);
    copyFileSync(copies.get(name), target);
    return target;
  });
}

test("three of four shard reports are not measured, and the missing shard is named", () => {
  const tool = fileURLToPath(new URL("./test-trend.mjs", import.meta.url));
  const dir = mkdtempSync(join(tmpdir(), "test-trend-shards-"));
  try {
    const reports = shardDir(dir, ["retry-passing", "failing", "passing"]);
    const envFile = join(dir, "browser.env");
    const result = spawnSync(process.execPath, [tool, "playwright", "--expect-shards", "1,2,3,4", "--env", envFile, ...reports], { encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(existsSync(envFile), false, "a partial set must not write metrics");
    assert.match(result.stdout, /Not measured/);
    assert.match(result.stdout, /Missing shard 4/);
    assert.doesNotMatch(result.stdout, /browser_first_pass_failures=/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("all four shard reports produce the complete totals", () => {
  const tool = fileURLToPath(new URL("./test-trend.mjs", import.meta.url));
  const dir = mkdtempSync(join(tmpdir(), "test-trend-shards-"));
  try {
    const reports = shardDir(dir, ["retry-passing", "failing", "passing", "passing"]);
    const envFile = join(dir, "browser.env");
    const result = spawnSync(process.execPath, [tool, "playwright", "--expect-shards", "1,2,3,4", "--env", envFile, ...reports], { encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /Report files \(shards\) \| 4 \|/);
    assert.match(readFileSync(envFile, "utf8"), /^browser_total=9$/m);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("a truncated shard report is treated as a missing shard", () => {
  const tool = fileURLToPath(new URL("./test-trend.mjs", import.meta.url));
  const dir = mkdtempSync(join(tmpdir(), "test-trend-shards-"));
  try {
    const reports = shardDir(dir, ["retry-passing", "failing", "passing"]);
    writeFileSync(join(dir, "report-2.json"), '{"stats":');
    const envFile = join(dir, "browser.env");
    const result = spawnSync(process.execPath, [tool, "playwright", "--expect-shards", "1,2,3,4", "--env", envFile, ...reports], { encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(existsSync(envFile), false);
    assert.match(result.stdout, /Missing shard 2/);
    assert.match(result.stdout, /Unreadable or truncated report: .*report-2\.json/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("a structurally empty report is treated as missing", () => {
  const tool = fileURLToPath(new URL("./test-trend.mjs", import.meta.url));
  const dir = mkdtempSync(join(tmpdir(), "test-trend-shards-"));
  try {
    const reports = shardDir(dir, ["retry-passing", "failing", "passing"]);
    writeFileSync(join(dir, "report-2.json"), "{}");
    const result = spawnSync(process.execPath, [tool, "playwright", "--expect-shards", "1,2,3,4", "--env", join(dir, "browser.env"), ...reports], { encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(existsSync(join(dir, "browser.env")), false);
    assert.match(result.stdout, /Missing shard 2/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});