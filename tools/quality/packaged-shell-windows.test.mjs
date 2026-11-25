import test from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { windows, windowsHelperPath } from "./packaged-shell-windows.mjs";

const windowsOnly = { skip: process.platform !== "win32" };

test("helper script resolves beside the shared module", () => {
  assert.ok(existsSync(windowsHelperPath));
});

test("snapshot includes the calling process", windowsOnly, () => {
  const rows = windows("Snapshot", ["-RootPid", String(process.pid)]);
  assert.ok(Array.isArray(rows));
  assert.ok(
    rows.some((row) => row.pid === process.pid),
    `snapshot missed pid ${process.pid}: ${JSON.stringify(rows)}`,
  );
});

test("unknown operation keeps its name in the error", windowsOnly, () => {
  assert.throws(
    () => windows("NoSuchOperation", []),
    /NoSuchOperation helper failed/,
  );
});

test("the smoke runner binds every windows() call", () => {
  const source = readFileSync(
    new URL("./packaged-shell-smoke.mjs", import.meta.url),
    "utf8",
  );
  if (!/\bwindows\(/.test(source)) return;
  const bound =
    /import\s*\{[^}]*\bwindows\b[^}]*\}\s*from\s*["']\.\/packaged-shell-windows\.mjs["']/.test(
      source,
    ) ||
    /function\s+windows\s*\(/.test(source) ||
    /\b(?:const|let|var)\s+windows\s*=/.test(source);
  assert.ok(
    bound,
    "packaged-shell-smoke.mjs calls windows() without importing or defining it",
  );
});