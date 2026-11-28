import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const entry = fileURLToPath(
  new URL("./packaged-shell-smoke.mjs", import.meta.url),
);
for (const [args, message] of [
  [
    ["--manual-keyboard", "true"],
    "--manual-keyboard requires --accessibility true",
  ],
  [["--accessibility", "false"], "--accessibility accepts only true"],
  [
    ["--accessibility", "true", "--manual-keyboard", "false"],
    "--manual-keyboard requires --accessibility true",
  ],
]) {
  test(`rejects invalid opt-in arguments before launching a shell: ${args.join(" ")}`, () => {
    const result = spawnSync(
      process.execPath,
      [entry, "--shell", "not-a-real-shell.exe", ...args],
      {
        encoding: "utf8",
        windowsHide: true,
        timeout: 15000,
      },
    );
    assert.equal(result.error, undefined);
    assert.equal(result.status, 1);
    assert.ok(result.stderr.includes(message), result.stderr);
    assert.ok(
      !result.stderr.includes("ENOENT"),
      "validation must precede executable access",
    );
  });
}
