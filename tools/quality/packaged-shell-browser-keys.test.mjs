import test from "node:test";
import assert from "node:assert/strict";
import { requireDisabledBrowserKeys } from "./packaged-shell-browser-keys.mjs";

const record = (
  processId,
  state = "disabled",
  outcome = "ok",
  timeUnixMs = 200,
) =>
  JSON.stringify({
    method: "shell.browserAccelerators",
    processId,
    outcome,
    detail: { state },
    timeUnixMs,
  });

test("native shortcut gate requires read-back from the current shell process", () => {
  assert.deepEqual(requireDisabledBrowserKeys(record(42), 42, 150), {
    processId: 42,
    state: "disabled",
  });
  for (const log of [
    "",
    record(41),
    record(42, "enabled"),
    record(42, "disabled", "error"),
    record(42, "disabled", "ok", 100),
    record(42) + "\n" + record(42, "enabled"),
  ])
    assert.throws(
      () => requireDisabledBrowserKeys(log, 42, 150),
      /not verified disabled/,
    );
});
