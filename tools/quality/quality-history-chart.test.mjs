import assert from "node:assert/strict";
import { test } from "node:test";
import { HISTORY_COLUMNS, mergeHistory, renderHistoryChart } from "./quality-history-chart.mjs";

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
  assert.equal(HISTORY_COLUMNS.join(","), "date,commit,rust_lines,shell_lines,ui_lines,soak_p99_us,soak_p999_us,soak_heap_growth_kib,soak_private_growth_mib,backend_ready_ms");
});

function entry(date, commit, soak = "150") {
  const cells = `${date},${commit},80.0,59.0,73.0,${soak},240,0.0,-0.1,136`.split(",");
  return Object.fromEntries(HISTORY_COLUMNS.map((column, index) => [column, cells[index]]));
}
