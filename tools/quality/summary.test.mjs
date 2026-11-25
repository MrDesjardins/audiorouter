import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { HISTORY_COLUMNS } from "./quality-history-chart.mjs";

const tool = fileURLToPath(new URL("./summary.mjs", import.meta.url));
const run = (dir, args) => spawnSync(process.execPath, [tool, "history", join(dir, "quality-history.csv"), ...args], { encoding: "utf8" });

function withDir(body) {
  const dir = mkdtempSync(join(tmpdir(), "quality-summary-"));
  try {
    body(dir);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test("a new history uses the full header and keeps the first-pass metrics", () => {
  withDir((dir) => {
    const result = run(dir, ["date=2026-11-24", "commit=02ca8dc9", "browser_first_pass_failures=1", "browser_failed=0", "run_url=https://example.test/run/1"]);
    assert.equal(result.status, 0, result.stderr);
    const [header, row] = readFileSync(join(dir, "quality-history.csv"), "utf8").trim().split("\n");
    assert.equal(header, HISTORY_COLUMNS.join(","));
    const cells = row.split(",");
    assert.equal(cells[HISTORY_COLUMNS.indexOf("browser_first_pass_failures")], "1");
    assert.equal(cells[HISTORY_COLUMNS.indexOf("browser_failed")], "0");
    assert.match(result.stdout, /\| date \| commit \| .*browser_first_pass_failures/);
  });
});

test("a cached history from before the Edge metrics is migrated, not truncated", () => {
  withDir((dir) => {
    const oldHeader = HISTORY_COLUMNS.slice(0, 10).join(",");
    writeFileSync(join(dir, "quality-history.csv"), `${oldHeader}\n2026-11-20,a4e92bb0,80.0,59.0,73.0,150,240,0.0,-0.1,136\n`);
    const result = run(dir, ["date=2026-11-24", "commit=02ca8dc9", "browser_first_pass_failures=2", "browser_duration_s=701.2"]);
    assert.equal(result.status, 0, result.stderr);
    const [header, oldRow, newRow] = readFileSync(join(dir, "quality-history.csv"), "utf8").trim().split("\n");
    assert.equal(header, HISTORY_COLUMNS.join(","));
    assert.equal(oldRow.split(",").slice(0, 10).join(","), "2026-11-20,a4e92bb0,80.0,59.0,73.0,150,240,0.0,-0.1,136");
    assert.equal(oldRow.split(",").length, HISTORY_COLUMNS.length);
    assert.equal(oldRow.split(",")[HISTORY_COLUMNS.indexOf("browser_first_pass_failures")], "");
    assert.equal(newRow.split(",")[HISTORY_COLUMNS.indexOf("browser_first_pass_failures")], "2");
    assert.match(result.stdout, /2026-11-20/);
    assert.match(result.stdout, /2026-11-24/);
  });
});

test("an unknown history key is refused instead of silently dropped", () => {
  withDir((dir) => {
    const result = run(dir, ["date=2026-11-24", "not_a_column=1"]);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /Unknown history column/);
  });
});