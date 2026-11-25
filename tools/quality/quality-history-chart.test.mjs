import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { HISTORY_COLUMNS, mergeHistory, parseHistory, renderHistoryChart, writeHistory } from "./quality-history-chart.mjs";

test("merges published and collected rows without duplicating a day", () => {
  const published = [
    entry("2026-11-01", "aaaa1111"),
    entry("2026-11-02", "bbbb2222"),
  ];
  const collected = [
    entry("2026-11-02", "bbbb2222"),
    entry("2026-11-03", "cccc3333"),
  ];
  assert.deepEqual(
    mergeHistory(published, collected).map((item) => `${item.date}/${item.commit}`),
    ["2026-11-01/aaaa1111", "2026-11-02/bbbb2222", "2026-11-03/cccc3333"],
  );
});

test("a re-run refreshes a day instead of repeating it", () => {
  const published = [entry("2026-11-03", "cccc3333", "150")];
  const collected = [entry("2026-11-03", "cccc3333", "170")];
  const merged = mergeHistory(published, collected);
  assert.equal(merged.length, 1);
  assert.equal(merged[0].soak_p99_us, "170");
});

test("charts the latest complete rows and skips an incomplete one", () => {
  const history = [
    entry("2026-11-01", "aaaa1111"),
    entry("2026-11-02", "bbbb2222", ""),
    entry("2026-11-03", "cccc3333"),
  ];
  const svg = renderHistoryChart(history);
  assert.match(svg, /Last 2 recorded daily runs/);
  assert.match(svg, /updated 2026-11-03/);
  assert.match(svg, /2026-11-01/);
  assert.match(svg, /2026-11-03/);
  assert.doesNotMatch(svg, /2026-11-02/);
});

test("rejects an empty history instead of drawing an empty chart", () => {
  assert.throws(() => renderHistoryChart([]), /No complete quality history rows/);
});

test("the published header matches the collected artifact", () => {
  assert.equal(HISTORY_COLUMNS.join(","), "date,commit,rust_lines,shell_lines,ui_lines,soak_p99_us,soak_p999_us,soak_heap_growth_kib,soak_private_growth_mib,backend_ready_ms,browser_total,browser_first_pass_failures,browser_retried_tests,browser_flaky,browser_failed,browser_skipped,browser_duration_s,run_url");
});

test("a published history from before the Edge metrics still parses", () => {
  const dir = mkdtempSync(join(tmpdir(), "quality-history-"));
  try {
    const path = join(dir, "old.csv");
    writeFileSync(path, "date,commit,rust_lines,shell_lines,ui_lines,soak_p99_us,soak_p999_us,soak_heap_growth_kib,soak_private_growth_mib,backend_ready_ms\n2026-11-20,a4e92bb0,80.0,59.0,73.0,150,240,0.0,-0.1,136\n");
    const entries = parseHistory(path);
    assert.equal(entries.length, 1);
    assert.equal(entries[0].browser_first_pass_failures, "");
    assert.equal(entries[0].run_url, "");
    assert.equal(entries[0].date, "2026-11-20");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("the Edge first-pass and retry metrics survive a write and read", () => {
  const dir = mkdtempSync(join(tmpdir(), "quality-history-"));
  try {
    const path = join(dir, "new.csv");
    writeHistory(path, [{ ...entry("2026-11-24", "02ca8dc9"), browser_first_pass_failures: "1", browser_retried_tests: "1", browser_failed: "0" }]);
    const [row] = parseHistory(path);
    assert.equal(row.browser_first_pass_failures, "1");
    assert.equal(row.browser_retried_tests, "1");
    assert.equal(row.browser_failed, "0");
    assert.match(readFileSync(path, "utf8").split("\n")[0], /browser_first_pass_failures/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("an unknown published column is refused", () => {
  const dir = mkdtempSync(join(tmpdir(), "quality-history-"));
  try {
    const path = join(dir, "bad.csv");
    writeFileSync(path, "date,commit,unknown_metric\n2026-11-24,02ca8dc9,1\n");
    assert.throws(() => parseHistory(path), /Unknown quality history column/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

function entry(date, commit, soak = "150", extra = {}) {
  return {
    date,
    commit,
    rust_lines: "80.0",
    shell_lines: "59.0",
    ui_lines: "73.0",
    soak_p99_us: soak,
    soak_p999_us: "240",
    soak_heap_growth_kib: "0.0",
    soak_private_growth_mib: "-0.1",
    backend_ready_ms: "136",
    browser_total: "812",
    browser_first_pass_failures: "0",
    browser_retried_tests: "0",
    browser_flaky: "0",
    browser_failed: "0",
    browser_skipped: "16",
    browser_duration_s: "701.2",
    run_url: "https://github.com/MrDesjardins/audiorouter/actions/runs/1",
    ...extra,
  };
}
