import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";

const launcher = fileURLToPath(
  new URL("./standard-user-process.ps1", import.meta.url),
);
const packagedWindows = fileURLToPath(
  new URL("./packaged-shell-windows.ps1", import.meta.url),
);
const root = path.join(
  process.env.RUNNER_TEMP ?? os.tmpdir(),
  "audiorouter-standard-user-qualification",
);
const windows = { skip: process.platform !== "win32", timeout: 60_000 };
test(
  "packaged startup process snapshot reports owned process health without command lines",
  windows,
  () => {
    const snapshot = spawnSync(
      "powershell.exe",
      [
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        packagedWindows,
        "-Operation",
        "Snapshot",
        "-RootPid",
        String(process.pid),
      ],
      { encoding: "utf8", windowsHide: true, timeout: 15_000 },
    );
    assert.equal(snapshot.status, 0, snapshot.stderr);
    const processes = JSON.parse(snapshot.stdout);
    const own = processes.find((item) => item.pid === process.pid);
    assert.ok(own, "Snapshot must include its exact owned root process");
    assert.equal(typeof own.name, "string");
    assert.equal(typeof own.threadCount, "number");
    assert.ok(/^\d+$/.test(own.userCpu100ns));
    assert.ok(/^\d+$/.test(own.kernelCpu100ns));
    assert.ok(/^\d+$/.test(own.workingSetBytes));
    assert.equal("commandLine" in own, false);
  },
);
function probe(name, mode, exitCode = 0) {
  mkdirSync(root, { recursive: true });
  const output = mkdtempSync(path.join(root, `${name}-`));
  const fixture = path.join(output, "probe.mjs");
  const observed = path.join(output, "observed.json");
  const report = path.join(output, "token.json");
  writeFileSync(
    fixture,
    `import {spawn,spawnSync} from 'node:child_process';
import {writeFileSync} from 'node:fs';
if (process.argv[2] === 'leaf') { setTimeout(() => {}, 10000); }
else {
  const child = process.argv[2] === 'timeout' ? spawn(process.execPath, [import.meta.filename, 'leaf'], {stdio: 'ignore', windowsHide: true}) : null;
  const nested = child ? null : spawnSync('powershell.exe', ['-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',process.argv[5],'-InspectOnly'], {encoding:'utf8', timeout:10000,windowsHide:true});
  writeFileSync(process.argv[3], JSON.stringify({nested: nested && {status:nested.status,signal:nested.signal,error:String(nested.error??''),stdout:nested.stdout,stderr:nested.stderr},pid: process.pid, childPid: child?.pid, cwd: process.cwd(), sentinel: process.env.AUDIOROUTER_STANDARD_USER_PROBE}));
  console.log('standard-user-stdout'); console.error('standard-user-stderr');
  if (child) setTimeout(() => {}, 10000); else process.exit(Number(process.argv[4]));
}
`,
  );
  const argumentsFile = path.join(output, "arguments.json");
  writeFileSync(
    argumentsFile,
    JSON.stringify([fixture, mode, observed, String(exitCode), launcher]),
  );
  const result = spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      launcher,
      "-Executable",
      process.execPath,
      "-ArgumentsFile",
      argumentsFile,
      "-Report",
      report,
      "-TimeoutSeconds",
      mode === "timeout" ? "3" : "20",
    ],
    {
      encoding: "utf8",
      timeout: 45_000,
      windowsHide: true,
      env: { ...process.env, AUDIOROUTER_STANDARD_USER_PROBE: "token-probe" },
    },
  );
  writeFileSync(path.join(output, "stdout.log"), result.stdout ?? "");
  writeFileSync(path.join(output, "stderr.log"), result.stderr ?? "");
  assert.equal(result.error, undefined, result.stderr);
  const json = (file) =>
    JSON.parse(readFileSync(file, "utf8").replace(/^\uFEFF/, ""));
  return { result, identity: json(report), observed: json(observed), output };
}

for (const code of [0, 7, 130]) {
  test(
    `standard-user launcher preserves identity, environment, streams and exit ${code}`,
    windows,
    () => {
      const { result, identity, observed } = probe(
        `exit-${code}`,
        "exit",
        code,
      );
      assert.equal(result.status, code, result.stderr);
      assert.equal(identity.childIntegrityRid, 8192);
      assert.equal(identity.childAdmin, false);
      assert.equal(identity.exitCode, code);
      assert.equal(identity.pid, observed.pid);
      assert.equal(
        path.resolve(observed.cwd).toLowerCase(),
        process.cwd().toLowerCase(),
      );
      assert.equal(observed.sentinel, "token-probe");
      assert.equal(observed.nested.status, 0, JSON.stringify(observed.nested));
      const nestedIdentity = JSON.parse(observed.nested.stdout);
      assert.equal(nestedIdentity.parentIntegrityRid, 8192);
      assert.equal(nestedIdentity.parentAdmin, false);
      assert.match(result.stdout, /standard-user-stdout/);
      assert.match(result.stderr, /standard-user-stderr/);
    },
  );
}
test(
  "standard-user launcher fails on timeout and removes its owned descendant tree",
  windows,
  async () => {
    const { result, identity, observed } = probe("timeout", "timeout");
    assert.notEqual(result.status, 0);
    assert.match(identity.error, /timed out/);
    assert.ok(observed.childPid > 0);
    const alive = (pid) => {
      try {
        process.kill(pid, 0);
        return true;
      } catch (error) {
        if (error.code === "ESRCH") return false;
        throw error;
      }
    };
    for (
      let attempt = 0;
      attempt < 10 && [observed.pid, observed.childPid].some(alive);
      attempt += 1
    )
      await delay(100);
    assert.equal(alive(observed.pid), false, "Owned parent remained alive");
    assert.equal(
      alive(observed.childPid),
      false,
      "Owned grandchild remained alive",
    );
  },
);
