import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { closeSync, openSync } from "node:fs";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { assertStandardUser } from "./standard-user.mjs";

const runnerFile = fileURLToPath(
  new URL("./packaged-shell-smoke.mjs", import.meta.url),
);
const windowsHelper = fileURLToPath(
  new URL("./packaged-shell-windows.ps1", import.meta.url),
);
const controlPipe = String.raw`\\.\pipe\audiorouter-control`;
const caseNames = [
  "baseline",
  "missing-binary",
  "occupied-pipe",
  "shell-exit",
  "bridge",
  "readiness",
  "page-error",
  "cleanup-fallback",
  "cancel-before-attach",
  "cancel-after-ready",
  "cancel-after-restart",
];

function args(argv) {
  const values = new Map();
  for (let i = 0; i < argv.length; i += 2) {
    if (
      !["--shell", "--output", "--case", "--timeout-ms"].includes(argv[i]) ||
      !argv[i + 1] ||
      values.has(argv[i])
    )
      throw new Error(`Invalid qualification argument: ${argv[i]}`);
    values.set(argv[i], argv[i + 1]);
  }
  if (!values.get("--shell"))
    throw new Error("Required: --shell <release executable>");
  const selected = values.get("--case");
  if (selected && !caseNames.includes(selected))
    throw new Error(`Unknown case: ${selected}`);
  const timeout = Number(values.get("--timeout-ms") ?? 15_000);
  if (!Number.isSafeInteger(timeout) || timeout < 5_000 || timeout > 30_000)
    throw new Error("Qualification timeout must be from 5000 to 30000 ms");
  return {
    shell: path.resolve(values.get("--shell")),
    output: path.resolve(
      values.get("--output") ??
        path.join(
          os.tmpdir(),
          `audiorouter-packaged-qualification-${randomUUID()}`,
        ),
    ),
    cases: selected ? [selected] : caseNames,
    timeout,
  };
}

function windows(operation, extra) {
  const result = spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      windowsHelper,
      "-Operation",
      operation,
      ...extra,
    ],
    {
      encoding: "utf8",
      windowsHide: true,
      timeout: 15_000,
    },
  );
  if (result.error || result.status !== 0)
    throw new Error(
      `${operation} helper failed: ${result.error?.message ?? result.stderr}`,
    );
  return JSON.parse(result.stdout.trim());
}

async function assertPipeFree() {
  await new Promise((resolve, reject) => {
    const socket = net.createConnection(controlPipe);
    const timer = setTimeout(() => {
      socket.destroy();
      reject(new Error("Control pipe preflight timed out"));
    }, 3_000);
    socket.once("connect", () => {
      clearTimeout(timer);
      socket.destroy();
      reject(
        new Error(
          "Close the existing AudioRouter instance before qualification",
        ),
      );
    });
    socket.once("error", (error) => {
      clearTimeout(timer);
      if (["ENOENT", "ECONNREFUSED"].includes(error.code)) resolve();
      else reject(error);
    });
  });
}

async function exited(child, timeout) {
  if (child.exitCode !== null || child.signalCode !== null)
    return child.exitCode ?? -1;
  return await new Promise((resolve, reject) => {
    const onExit = (code) => {
      clearTimeout(timer);
      resolve(code ?? -1);
    };
    const timer = setTimeout(() => {
      child.off("exit", onExit);
      reject(
        new Error("Smoke runner did not exit within its qualification budget"),
      );
    }, timeout);
    child.once("exit", onExit);
  });
}

async function runCase(name, options) {
  const output = path.join(options.output, name);
  await mkdir(output, { recursive: true });
  const snapshotFile = path.join(output, "owned-processes.json");
  let snapshot = [];
  const observedStages = [];
  let runner;
  let server;
  let controlServer;
  let control;
  let ownedRunnerPid;
  let checkpointWork = Promise.resolve();
  let checkpointFailure;
  let exitCode;
  let report;
  let failure;
  const cancelStage = {
    "cancel-before-attach": "initial-child-spawned",
    "cancel-after-ready": "initial-ready",
    "cancel-after-restart": "restarted-ready",
  }[name];
  try {
    await assertPipeFree();
    if (name === "occupied-pipe") {
      server = net.createServer((socket) => socket.destroy());
      await new Promise((resolve, reject) => {
        server.once("error", reject);
        server.listen(controlPipe, resolve);
      });
    }
    const injectedCase = !["missing-binary", "occupied-pipe"].includes(name);
    const controlName =
      String.raw`\\.\pipe\audiorouter-smoke-qualification-` + randomUUID();
    const onCheckpoint = (message) => {
      if (message?.type !== "checkpoint") return;
      checkpointWork = checkpointWork
        .then(async () => {
          assert.ok(
            Number.isSafeInteger(message.runnerPid) && message.runnerPid > 0,
          );
          if (ownedRunnerPid) assert.equal(message.runnerPid, ownedRunnerPid);
          else {
            ownedRunnerPid = message.runnerPid;
            const runnerTree = windows("Snapshot", [
              "-RootPid",
              String(runner.pid),
            ]);
            assert.ok(
              runnerTree.some((item) => item.pid === ownedRunnerPid),
              "Console runner must descend from the owned launcher",
            );
          }
          assert.ok(
            Number.isSafeInteger(message.shellPid) && message.shellPid > 0,
          );
          observedStages.push(message.stage);
          const captured = windows("Snapshot", [
            "-RootPid",
            String(message.shellPid),
          ]);
          assert.ok(
            captured.some((item) => item.pid === message.shellPid),
            "Snapshot must contain the owned shell",
          );
          snapshot = [
            ...new Map(
              [...snapshot, ...captured].map((item) => [
                item.pid + ":" + item.created,
                item,
              ]),
            ).values(),
          ];
          await writeFile(snapshotFile, JSON.stringify(snapshot));
          if (cancelStage === message.stage)
            windows("Break", ["-RootPid", String(ownedRunnerPid)]);
          else
            await new Promise((resolve, reject) =>
              control.write(
                JSON.stringify({ type: "continue", stage: message.stage }) +
                  "\n",
                (error) => (error ? reject(error) : resolve()),
              ),
            );
        })
        .catch((error) => {
          checkpointFailure = error;
          control?.destroy();
        });
    };
    if (injectedCase) {
      controlServer = net.createServer((socket) => {
        if (control) {
          socket.destroy();
          return;
        }
        control = socket;
        let buffer = "";
        socket.on("error", (error) => {
          checkpointFailure = error;
        });
        socket.on("data", (data) => {
          buffer += data.toString("utf8");
          if (buffer.length > 4096) {
            checkpointFailure = new Error("Oversized checkpoint message");
            socket.destroy();
            return;
          }
          let end;
          while ((end = buffer.indexOf("\n")) >= 0) {
            const line = buffer.slice(0, end);
            buffer = buffer.slice(end + 1);
            try {
              onCheckpoint(JSON.parse(line));
            } catch (error) {
              checkpointFailure = error;
              socket.destroy();
            }
          }
        });
      });
      await new Promise((resolve, reject) => {
        controlServer.once("error", reject);
        controlServer.listen(controlName, resolve);
      });
    }
    const runnerArgs = [
      runnerFile,
      "--shell",
      name === "missing-binary"
        ? path.join(output, "not-a-shell.exe")
        : options.shell,
      "--output",
      output,
      "--timeout-ms",
      String(options.timeout),
      ...(injectedCase
        ? ["--qualification-case", name, "--qualification-control", controlName]
        : []),
    ];
    const stdout = openSync(path.join(output, "launcher.stdout.log"), "w");
    const stderr = openSync(path.join(output, "launcher.stderr.log"), "w");
    try {
      if (injectedCase) {
        const argumentsFile = path.join(output, "runner-arguments.json");
        await writeFile(argumentsFile, JSON.stringify(runnerArgs));
        runner = spawn(
          "powershell.exe",
          [
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            windowsHelper,
            "-Operation",
            "Launch",
            "-Executable",
            process.execPath,
            "-ArgumentsFile",
            argumentsFile,
            "-StdoutFile",
            path.join(output, "runner.stdout.log"),
            "-StderrFile",
            path.join(output, "runner.stderr.log"),
          ],
          { stdio: ["ignore", stdout, stderr], windowsHide: true },
        );
      } else
        runner = spawn(process.execPath, runnerArgs, {
          stdio: ["ignore", stdout, stderr],
          windowsHide: true,
        });
    } finally {
      closeSync(stdout);
      closeSync(stderr);
    }
    runner.on("error", (error) => {
      checkpointFailure = error;
    });
    exitCode = await exited(runner, 90_000);
    await checkpointWork;
    if (checkpointFailure) throw checkpointFailure;
    report = JSON.parse(
      await readFile(path.join(output, "result.json"), "utf8"),
    );
    assert.equal(report.cleanupFailure, null, "Cleanup must not fail");
    assert.ok(
      report.launchedChildren.every((child) => child.running === false),
      "Every owned shell must have exited",
    );
    if (snapshot.length) {
      let alive = [];
      for (let attempt = 0; attempt < 10; attempt += 1) {
        alive = windows("Alive", ["-SnapshotFile", snapshotFile]);
        if (!alive.length) break;
        await delay(300);
      }
      assert.deepEqual(
        alive,
        [],
        "Captured shell/WebView descendants must all exit",
      );
    }
    if (name === "baseline") {
      assert.equal(exitCode, 0);
      assert.equal(report.status, "passed");
      assert.ok(
        (await stat(path.join(output, "initial-trace.zip"))).size > 0,
        "Initial trace must be finalized before closing its WebView",
      );
      assert.deepEqual(observedStages, ["initial-ready", "restarted-ready"]);
      assert.equal(report.launchedChildren.length, 2);
      assert.ok(
        report.launchedChildren.every(
          (child) => child.exitCode === 0 && child.signalCode === null,
        ),
      );
      assert.deepEqual(
        report.cleanupActions.map((action) => action.method),
        ["graceful-quit", "graceful-quit"],
      );
      for (const state of Object.values(report.nativeBridge)) {
        assert.equal(state.activeSessionCount, 0);
        assert.equal(state.deviceAccessAllowed, false);
        assert.equal(state.protocolVersion.major, 1);
      }
    } else if (cancelStage) {
      assert.equal(exitCode, 130);
      assert.equal(report.status, "cancelled");
      assert.equal(report.cancellation, "SIGBREAK");
      assert.equal(report.qualification.injected, true);
      assert.ok(observedStages.includes(cancelStage));
    } else {
      assert.equal(exitCode, 1);
      assert.equal(report.status, "failed");
      const expected = {
        "missing-binary": /ENOENT/,
        "occupied-pipe": /instance owns the default control pipe/,
        "shell-exit": /exited before WebView2 became available/,
        bridge: /Native bridge returned an invalid describe response/,
        readiness: /Usable canvas did not become ready \(initial\)/,
        "page-error": /Uncaught UI errors:.*packaged-smoke-seeded-page-error/s,
        "cleanup-fallback": /did not exit after a graceful quit/,
      }[name];
      assert.match(report.failure, expected);
      if (["missing-binary", "occupied-pipe"].includes(name))
        assert.deepEqual(report.launchedChildren, []);
      else assert.equal(report.qualification.injected, true);
      if (["bridge", "cleanup-fallback"].includes(name))
        assert.ok(
          report.qualification.hits > 0,
          "Fault must have reached the native IPC boundary",
        );
      if (name === "cleanup-fallback")
        assert.ok(
          report.cleanupActions.some(
            (action) => action.method === "taskkill-tree",
          ),
        );
      if (
        ["bridge", "readiness", "page-error", "cleanup-fallback"].includes(name)
      ) {
        for (const artifact of ["failure.png", "failure.html", "trace.zip"])
          assert.ok(
            (await stat(path.join(output, artifact))).size > 0,
            `${artifact} must be retained`,
          );
      }
    }
  } catch (error) {
    failure = error.stack ?? String(error);
  } finally {
    if (server) await new Promise((resolve) => server.close(resolve));
    if (runner && runner.exitCode === null && runner.signalCode === null) {
      control?.destroy();
      try {
        await exited(runner, 15_000);
      } catch {
        spawnSync("taskkill.exe", ["/PID", String(runner.pid), "/T", "/F"], {
          windowsHide: true,
          timeout: 10_000,
        });
        failure = `${failure ?? ""}\nQualification required emergency runner cleanup`;
      }
    }
    control?.destroy();
    if (controlServer)
      await new Promise((resolve) => controlServer.close(resolve));
    if (snapshot.length) {
      const alive = windows("Alive", ["-SnapshotFile", snapshotFile]);
      if (alive.length) {
        windows("Cleanup", ["-SnapshotFile", snapshotFile]);
        failure = `${failure ?? ""}\nQualification required emergency cleanup of ${alive.length} owned processes`;
      }
    }
  }
  const result = {
    name,
    status: failure ? "failed" : "passed",
    failure: failure ?? null,
    expectedRunnerStatus:
      name === "baseline" ? "passed" : cancelStage ? "cancelled" : "failed",
    runnerExitCode: exitCode ?? null,
    observedStages,
    capturedProcesses: snapshot.length,
    report: path.join(output, "result.json"),
  };
  process.stdout.write(
    `${name}: ${result.status}${failure ? ` (${failure.split("\n")[0]})` : ""}\n`,
  );
  return result;
}

async function main() {
  if (process.platform !== "win32")
    throw new Error("Packaged smoke qualification requires Windows");
  const options = args(process.argv.slice(2));
  await stat(options.shell);
  assertStandardUser();
  await assertPipeFree();
  await mkdir(options.output, { recursive: true });
  const summaryFile = path.join(options.output, "qualification.json");
  try {
    await stat(summaryFile);
    throw new Error("Refusing to overwrite qualification evidence");
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
  }
  const cases = [];
  for (const name of options.cases) {
    cases.push(await runCase(name, options));
    await writeFile(
      summaryFile,
      `${JSON.stringify({ status: cases.some((item) => item.status === "failed") ? "failed" : "passed", shell: options.shell, cases }, null, 2)}\n`,
    );
    if (cases.at(-1).failure?.includes("emergency")) break;
  }
  process.stdout.write(`Qualification evidence: ${summaryFile}\n`);
  if (
    cases.length !== options.cases.length ||
    cases.some((item) => item.status !== "passed")
  )
    process.exitCode = 1;
}
main().catch((error) => {
  process.stderr.write(`${error.stack ?? error}\n`);
  process.exitCode = 1;
});
