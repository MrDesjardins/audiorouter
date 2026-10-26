import { createHash, randomUUID } from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import { closeSync, openSync, writeFileSync } from "node:fs";
import net from "node:net";
import { mkdir, open, readFile, unlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { chromium } from "../../ui/node_modules/@playwright/test/index.mjs";
import { packagedShellEnvironment } from "./packaged-shell-environment.mjs";
import { assertStandardUser } from "./standard-user.mjs";

const MAX_TIMEOUT_MS = 120_000;
const MAX_LOG_BYTES = 5 * 1024 * 1024;
const CONTROL_PIPE = String.raw`\\.\pipe\audiorouter-control`;
const QUALIFICATION_CASES = new Set([
  "baseline",
  "bridge",
  "readiness",
  "page-error",
  "shell-exit",
  "cleanup-fallback",
  "cancel-before-attach",
  "cancel-after-ready",
  "cancel-after-restart",
]);
function parseArgs(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index];
    if (!key.startsWith("--") || values.has(key))
      throw new Error(`Unexpected or duplicate argument: ${key}`);
    const value = argv[index + 1];
    if (!value || value.startsWith("--"))
      throw new Error(`Missing value for ${key}`);
    values.set(key, value);
    index += 1;
  }
  const shell = values.get("--shell");
  if (!shell)
    throw new Error("Required argument: --shell <audiorouter-shell.exe>");
  const timeout = Number(values.get("--timeout-ms") ?? 60_000);
  if (
    !Number.isSafeInteger(timeout) ||
    timeout < 5_000 ||
    timeout > MAX_TIMEOUT_MS
  )
    throw new Error(
      `--timeout-ms must be an integer from 5000 to ${MAX_TIMEOUT_MS}`,
    );
  for (const key of values.keys()) {
    if (
      ![
        "--shell",
        "--output",
        "--timeout-ms",
        "--qualification-case",
        "--qualification-control",
      ].includes(key)
    )
      throw new Error(`Unknown argument: ${key}`);
  }
  const qualificationCase = values.get("--qualification-case") ?? null;
  const qualificationControl = values.get("--qualification-control") ?? null;
  if (Boolean(qualificationCase) !== Boolean(qualificationControl))
    throw new Error(
      "Qualification case and control pipe must be supplied together",
    );
  if (
    qualificationCase &&
    (!QUALIFICATION_CASES.has(qualificationCase) ||
      !qualificationControl?.startsWith(
        String.raw`\\.\pipe\audiorouter-smoke-qualification-`,
      ))
  )
    throw new Error(
      "Qualification cases require a supported case and a private qualification control pipe",
    );
  return {
    qualificationCase,
    qualificationControl,
    shell: path.resolve(shell),
    output: path.resolve(
      values.get("--output") ??
        path.join(os.tmpdir(), `audiorouter-packaged-smoke-${randomUUID()}`),
    ),
    timeout,
  };
}

function getCurrentSha() {
  const result = spawnSync("git", ["rev-parse", "HEAD"], {
    encoding: "utf8",
    windowsHide: true,
  });
  return result.status === 0 ? result.stdout.trim() : "unavailable";
}

async function assertNoExistingInstance() {
  if (process.platform !== "win32")
    throw new Error("Packaged WebView2 smoke requires Windows");
  await new Promise((resolve, reject) => {
    const socket = net.createConnection(CONTROL_PIPE);
    const timer = setTimeout(() => {
      socket.destroy();
      reject(
        new Error("Timed out checking AudioRouter control-pipe ownership"),
      );
    }, 3_000);
    socket.once("connect", () => {
      clearTimeout(timer);
      socket.destroy();
      reject(
        new Error(
          "An AudioRouter instance owns the default control pipe; close it before the packaged smoke",
        ),
      );
    });
    socket.once("error", (error) => {
      clearTimeout(timer);
      if (["ENOENT", "ECONNREFUSED"].includes(error.code ?? "")) resolve();
      else
        reject(
          new Error(
            `Could not check AudioRouter instance ownership: ${error.message}`,
          ),
        );
    });
  });
}

async function waitForFile(file, child, timeoutMs, errors = [], signal) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    signal?.throwIfAborted();
    if (!childIsRunning(child))
      throw new Error(
        `AudioRouter exited before WebView2 became available (${child.exitCode})`,
      );
    if (child.spawnfile && child.pid === undefined)
      throw new Error(`Could not start AudioRouter from ${child.spawnfile}`);
    if (errors[0]?.startsWith("Failed to ")) throw new Error(errors[0]);
    try {
      const contents = await readFile(file, "utf8");
      const [portLine, webSocketPath] = contents.split(/\r?\n/);
      const port = Number(portLine);
      if (
        Number.isSafeInteger(port) &&
        port > 0 &&
        port <= 65_535 &&
        /^\/devtools\/browser\/[a-z\d-]+$/i.test(webSocketPath ?? "")
      )
        return { port, webSocketPath };
    } catch {
      // WebView2 creates this file once its child browser is ready.
    }
    await delay(100);
  }
  throw new Error(
    "WebView2 did not publish a loopback DevTools port before timeout",
  );
}

async function waitForAppPage(browser, child, timeoutMs, knownTargets, signal) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    signal?.throwIfAborted();
    if (!childIsRunning(child))
      throw new Error(
        `AudioRouter exited before exposing its application page (${child.exitCode})`,
      );
    const pages = browser.contexts().flatMap((context) => context.pages());
    const page =
      pages.find((candidate) => candidate.url().includes("index.html")) ??
      pages[0];
    if (page) return page;
    await delay(100);
  }
  let targetDiagnostics = [];
  let diagnostics;
  try {
    diagnostics = await browser.newBrowserCDPSession();
    const result = await diagnostics.send("Target.getTargets");
    targetDiagnostics = (result.targetInfos ?? []).map(
      ({ type, url, title }) => ({
        type,
        url,
        title,
      }),
    );
  } catch (error) {
    targetDiagnostics = [`CDP target query failed: ${error.message}`];
  } finally {
    await diagnostics?.detach().catch(() => undefined);
  }
  throw new Error(
    `WebView2 exposed no application page before timeout; targets=${JSON.stringify(targetDiagnostics)}; initialTargets=${JSON.stringify(knownTargets)}`,
  );
}

async function listDevToolsTargets(port) {
  try {
    const response = await fetch(`http://127.0.0.1:${port}/json/list`, {
      signal: AbortSignal.timeout(3_000),
    });
    if (!response.ok) return [`HTTP ${response.status}`];
    const targets = await response.json();
    return targets.map(({ type, url, title, webSocketDebuggerUrl }) => ({
      type,
      url,
      title,
      webSocketDebuggerUrl,
    }));
  } catch (error) {
    return [`DevTools target list failed: ${error.message}`];
  }
}

async function connectToWebView(debugEndpoint, timeoutMs) {
  return await chromium.connectOverCDP(
    `http://localhost:${debugEndpoint.port}`,
    {
      timeout: timeoutMs,
      headers: { Origin: "http://localhost" },
    },
  );
}

async function waitForExit(child, timeoutMs) {
  if (!childIsRunning(child)) return child.exitCode ?? -1;
  return await new Promise((resolve) => {
    const finish = (code) => {
      clearTimeout(timer);
      child.off("exit", onExit);
      resolve(code);
    };
    const onExit = (code) => finish(code ?? -1);
    const timer = setTimeout(() => finish(null), timeoutMs);
    child.once("exit", onExit);
  });
}

function childIsRunning(child) {
  return Boolean(
    child?.pid && child.exitCode === null && child.signalCode === null,
  );
}

async function stopOwnedChild(child, cleanupActions = []) {
  if (!childIsRunning(child)) return;
  const result = spawnSync(
    "taskkill.exe",
    ["/PID", String(child.pid), "/T", "/F"],
    {
      encoding: "utf8",
      windowsHide: true,
      timeout: 10_000,
    },
  );
  if (result.error || result.status !== 0) {
    if (!childIsRunning(child)) return;
    child.kill();
    throw new Error(
      `Could not terminate smoke-owned process tree ${child.pid}: ${result.error?.message ?? result.stderr}`,
    );
  }
  cleanupActions.push({ pid: child.pid, method: "taskkill-tree" });
  const exitCode = await waitForExit(child, 10_000);
  if (exitCode === null)
    throw new Error(
      `Could not terminate smoke-owned AudioRouter process ${child.pid}`,
    );
}

async function boundedCopy(source, destination) {
  try {
    const file = await open(source, "r");
    try {
      const size = (await file.stat()).size;
      const length = Math.min(size, MAX_LOG_BYTES);
      const contents = Buffer.alloc(length);
      let offset = 0;
      while (offset < length) {
        const { bytesRead } = await file.read(
          contents,
          offset,
          length - offset,
          size - length + offset,
        );
        if (bytesRead === 0) break;
        offset += bytesRead;
      }
      await writeFile(destination, contents.subarray(0, offset));
    } finally {
      await file.close();
    }
  } catch {
    // A missing log is retained as absence; the smoke result records failure.
  }
}

async function connectQualificationControl(pipe) {
  if (!pipe) return null;
  return await new Promise((resolve, reject) => {
    const socket = net.createConnection(pipe);
    let buffer = "";
    const timer = setTimeout(() => {
      socket.destroy();
      reject(new Error("Qualification control connection timed out"));
    }, 5_000);
    socket.once("connect", () => {
      clearTimeout(timer);
      resolve(socket);
    });
    socket.on("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    socket.on("data", (data) => {
      buffer += data.toString("utf8");
      if (buffer.length > 4096) {
        socket.destroy(new Error("Oversized qualification control message"));
        return;
      }
      let end;
      while ((end = buffer.indexOf("\n")) >= 0) {
        const line = buffer.slice(0, end);
        buffer = buffer.slice(end + 1);
        try {
          socket.emit("message", JSON.parse(line));
        } catch {
          socket.destroy(new Error("Invalid qualification control JSON"));
        }
      }
    });
  });
}

async function qualificationCheckpoint(
  qualification,
  stage,
  child,
  signal,
  control,
) {
  const stages = {
    baseline: ["initial-ready", "restarted-ready"],
    "shell-exit": "initial-child-spawned",
    "cancel-before-attach": "initial-child-spawned",
    readiness: "page-attached",
    "page-error": "page-attached",
    bridge: "initial-ready",
    "cleanup-fallback": "initial-ready",
    "cancel-after-ready": "initial-ready",
    "cancel-after-restart": ["initial-ready", "restarted-ready"],
  };
  if (!qualification || ![].concat(stages[qualification.case]).includes(stage))
    return;
  signal.throwIfAborted();
  qualification.checkpoint = stage;
  await new Promise((resolve, reject) => {
    const finish = (error) => {
      clearTimeout(timer);
      control.off("message", onMessage);
      signal.removeEventListener("abort", onAbort);
      if (error) reject(error);
      else resolve();
    };
    const onMessage = (message) => {
      if (message?.type === "continue" && message.stage === stage) finish();
    };
    const onAbort = () => finish(signal.reason);
    const timer = setTimeout(
      () => finish(new Error(`Qualification parent did not continue ${stage}`)),
      15_000,
    );
    control.on("message", onMessage);
    signal.addEventListener("abort", onAbort, { once: true });
    control.write(
      JSON.stringify({
        type: "checkpoint",
        stage,
        shellPid: child.pid,
        runnerPid: process.pid,
      }) + "\n",
      (error) => {
        if (error) finish(error);
      },
    );
  });
}

async function injectPageFault(page, qualification, timeoutMs) {
  if (!qualification || qualification.injected) return;
  if (["bridge", "cleanup-fallback"].includes(qualification.case)) {
    await page.evaluate((fault) => {
      const original = window.fetch;
      window.__AUDIO_ROUTER_SMOKE_FAULT_HITS__ = 0;
      const replacement = (input, init) => {
        const url = new URL(
          typeof input === "string" ? input : input.url,
          window.location.href,
        );
        if (url.hostname === "ipc.localhost") {
          const payload =
            typeof init?.body === "string" ? JSON.parse(init.body) : null;
          if (
            fault === "bridge" &&
            url.pathname === "/rpc_request" &&
            payload?.request?.method === "system.describe"
          ) {
            window.__AUDIO_ROUTER_SMOKE_FAULT_HITS__ += 1;
            return Promise.resolve(
              new Response(
                JSON.stringify({
                  jsonrpc: "2.0",
                  id: payload.request.id,
                  error: {
                    code: -32000,
                    message: "packaged-smoke-seeded-bridge-failure",
                  },
                }),
                {
                  status: 200,
                  headers: {
                    "Content-Type": "application/json",
                    "Tauri-Response": "ok",
                  },
                },
              ),
            );
          }
          if (fault === "cleanup-fallback" && url.pathname === "/quit_app") {
            window.__AUDIO_ROUTER_SMOKE_FAULT_HITS__ += 1;
            return Promise.resolve(
              new Response(
                JSON.stringify("packaged-smoke-seeded-quit-refusal"),
                {
                  status: 200,
                  headers: {
                    "Content-Type": "application/json",
                    "Tauri-Response": "error",
                  },
                },
              ),
            );
          }
        }
        return original.call(window, input, init);
      };
      window.fetch = replacement;
      if (window.fetch !== replacement)
        throw new Error(
          "Qualification could not replace the native IPC fetch boundary",
        );
      // Wry freezes window.ipc and Tauri freezes its invoke exports. Windows
      // can also use the WebView2 postMessage fallback instead of fetch.
      const host = window.chrome.webview;
      const originalPost = host.postMessage;
      const replacementPost = (raw) => {
        let message;
        try {
          message = typeof raw === "string" ? JSON.parse(raw) : raw;
        } catch {
          return originalPost.call(host, raw);
        }
        if (
          fault === "bridge" &&
          message.cmd === "rpc_request" &&
          message.payload?.request?.method === "system.describe"
        ) {
          window.__AUDIO_ROUTER_SMOKE_FAULT_HITS__ += 1;
          queueMicrotask(() =>
            window.__TAURI_INTERNALS__.runCallback(message.callback, {
              jsonrpc: "2.0",
              id: message.payload.request.id,
              error: {
                code: -32000,
                message: "packaged-smoke-seeded-bridge-failure",
              },
            }),
          );
          return;
        }
        if (fault === "cleanup-fallback" && message.cmd === "quit_app") {
          window.__AUDIO_ROUTER_SMOKE_FAULT_HITS__ += 1;
          queueMicrotask(() =>
            window.__TAURI_INTERNALS__.runCallback(
              message.error,
              "packaged-smoke-seeded-quit-refusal",
            ),
          );
          return;
        }
        return originalPost.call(host, raw);
      };
      host.postMessage = replacementPost;
      if (host.postMessage !== replacementPost)
        throw new Error(
          "Qualification could not replace the WebView2 IPC boundary",
        );
    }, qualification.case);
  } else if (qualification.case === "readiness") {
    await page.evaluate(() => {
      const probe = document.querySelector('[data-testid="canvas-readiness"]');
      if (!probe)
        throw new Error("Qualification requires an existing readiness probe");
      const block = () => {
        if (probe.getAttribute("data-ready") !== "false")
          probe.setAttribute("data-ready", "false");
      };
      block();
      new MutationObserver(block).observe(probe, {
        attributes: true,
        attributeFilter: ["data-ready"],
      });
    });
  } else if (qualification.case === "page-error") {
    await page.addInitScript(() => {
      setTimeout(() => {
        throw new Error("packaged-smoke-seeded-page-error");
      }, 0);
    });
    await Promise.all([
      page.waitForEvent("pageerror", { timeout: timeoutMs }),
      page.reload({ waitUntil: "domcontentloaded", timeout: timeoutMs }),
    ]);
  } else return;
  qualification.injected = true;
}

async function waitForCanvas(page, timeoutMs, signal, phase) {
  try {
    await page
      .getByTestId("canvas-readiness")
      .waitFor({ state: "attached", timeout: timeoutMs });
    await page.waitForFunction(
      () => {
        const probe = document.querySelector(
          '[data-testid="canvas-readiness"]',
        );
        return (
          probe?.getAttribute("data-ready") === "true" &&
          probe.getAttribute("data-session-id") === "desktop-session"
        );
      },
      undefined,
      { timeout: timeoutMs },
    );
  } catch (error) {
    signal.throwIfAborted();
    throw new Error(
      `Usable canvas did not become ready (${phase}): ${error.message}`,
    );
  }
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const started = Date.now();
  await mkdir(options.output, { recursive: true });
  const profile = path.join(options.output, "profile");
  const data = path.join(options.output, "local-data");
  const appData = path.join(options.output, "app-data");
  const webview = path.join(profile, "webview");
  const metrics = {};
  const errors = [];
  let child;
  let browser;
  let page;
  let traceContext;
  let traceStarted = false;
  let webview2Version = "unavailable";
  let stdoutFd;
  let stderrFd;
  let failure;
  let shellHash;
  let devToolsTargets = [];
  const nativeBridge = {};
  const reportPath = path.join(options.output, "result.json");
  let cleanupFailure;
  const abort = new AbortController();
  const cleanupActions = [];
  const launchedChildren = [];
  const qualification = options.qualificationCase
    ? { case: options.qualificationCase, injected: false, checkpoint: null }
    : null;
  let cancelled = null;
  let cancellationWork = Promise.resolve();
  let qualificationControl;
  const onSignal = (name) => {
    if (cancelled) return;
    cancelled = name;
    if (qualification?.case.startsWith("cancel-"))
      qualification.injected = true;
    abort.abort(new Error(`Packaged smoke cancelled by ${name}`));
    cancellationWork = (async () => {
      try {
        await stopOwnedChild(child, cleanupActions);
      } catch (error) {
        cleanupFailure = error.message;
      }
      await browser?.close().catch(() => undefined);
    })();
  };
  const signalHandlers = new Map(
    [
      "SIGINT",
      "SIGTERM",
      ...(process.platform === "win32" ? ["SIGBREAK"] : []),
    ].map((name) => [name, () => onSignal(name)]),
  );
  const onDisconnect = () => onSignal("parent-disconnected");

  try {
    await readFile(reportPath);
    throw new Error(
      `Refusing to overwrite existing smoke evidence: ${reportPath}`,
    );
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
  for (const [name, handler] of signalHandlers) process.on(name, handler);

  try {
    qualificationControl = await connectQualificationControl(
      options.qualificationControl,
    );
    qualificationControl?.on("close", onDisconnect);
    const shell = await readFile(options.shell);
    shellHash = createHash("sha256").update(shell).digest("hex");
    metrics.processIdentity = assertStandardUser();
    await assertNoExistingInstance();
    await mkdir(data, { recursive: true });
    await mkdir(appData, { recursive: true });
    await mkdir(webview, { recursive: true });
    const devToolsActivePort = path.join(
      webview,
      "EBWebView",
      "DevToolsActivePort",
    );
    await unlink(devToolsActivePort).catch(() => undefined);

    stdoutFd = openSync(path.join(profile, "shell.stdout.raw"), "w");
    stderrFd = openSync(path.join(profile, "shell.stderr.raw"), "w");
    const env = packagedShellEnvironment(process.env, {
      database: path.join(data, "state.sqlite"),
      localAppData: data,
      appData,
      webview,
    });
    abort.signal.throwIfAborted();
    const launchedAt = Date.now();
    child = spawn(options.shell, [], {
      env,
      stdio: ["ignore", stdoutFd, stderrFd],
      windowsHide: false,
      detached: process.platform === "win32",
    });
    child.on("error", (error) =>
      errors.push(`Failed to start shell: ${error.message}`),
    );
    launchedChildren.push({ phase: "initial", child });
    await qualificationCheckpoint(
      qualification,
      "initial-child-spawned",
      child,
      abort.signal,
      qualificationControl,
    );
    if (qualification?.case === "shell-exit") {
      await stopOwnedChild(child, cleanupActions);
      qualification.injected = true;
    }
    let debugEndpoint;
    try {
      debugEndpoint = await waitForFile(
        devToolsActivePort,
        child,
        options.timeout,
        errors,
        abort.signal,
      );
    } catch (error) {
      if (qualification?.case === "baseline") {
        try {
          const ownedProcesses = windows("Snapshot", [
            "-RootPid",
            String(child.pid),
          ]);
          let portFileBytes = 0;
          try {
            portFileBytes = (await stat(devToolsActivePort)).size;
          } catch {
            // Absence is useful evidence: WebView2 had not published its port.
          }
          await writeFile(
            path.join(options.output, "startup-failure.json"),
            JSON.stringify(
              {
                phase: "initial-devtools-endpoint",
                elapsedMs: Date.now() - launchedAt,
                portFileBytes,
                ownedProcesses,
              },
              null,
              2,
            ),
          );
        } catch (diagnosticError) {
          errors.push(
            `Could not capture startup process evidence: ${diagnosticError.message}`,
          );
        }
      }
      throw error;
    }
    metrics.webviewDebugReadyMs = Date.now() - launchedAt;
    devToolsTargets = await listDevToolsTargets(debugEndpoint.port);
    browser = await connectToWebView(debugEndpoint, options.timeout);
    webview2Version = browser.version();
    const contexts = browser.contexts();
    traceContext = contexts[0];
    if (!traceContext)
      throw new Error("WebView2 connected without a browser context");
    await traceContext.tracing.start({
      screenshots: true,
      snapshots: true,
      sources: true,
    });
    traceStarted = true;
    page = await waitForAppPage(
      browser,
      child,
      options.timeout,
      devToolsTargets,
      abort.signal,
    );
    page.on("pageerror", (error) => errors.push(error.message));
    await page.waitForLoadState("domcontentloaded", {
      timeout: options.timeout,
    });
    await page
      .getByTestId("canvas-readiness")
      .waitFor({ state: "attached", timeout: options.timeout });
    await qualificationCheckpoint(
      qualification,
      "page-attached",
      child,
      abort.signal,
      qualificationControl,
    );
    if (["readiness", "page-error"].includes(qualification?.case))
      await injectPageFault(page, qualification, options.timeout);
    await waitForCanvas(page, options.timeout, abort.signal, "initial");
    metrics.coldUiReadyMs = Date.now() - launchedAt;
    await qualificationCheckpoint(
      qualification,
      "initial-ready",
      child,
      abort.signal,
      qualificationControl,
    );
    if (["bridge", "cleanup-fallback"].includes(qualification?.case))
      await injectPageFault(page, qualification, options.timeout);

    const rpc = (method, params = {}) =>
      page.evaluate(
        async ({ method: rpcMethod, params: rpcParams }) => {
          const core = window.__TAURI__?.core;
          if (typeof core?.invoke !== "function")
            throw new Error("Tauri core invoke is unavailable");
          return await core.invoke("rpc_request", {
            request: {
              jsonrpc: "2.0",
              id: `packaged-smoke-${rpcMethod}`,
              method: rpcMethod,
              params: rpcParams,
            },
          });
        },
        { method, params },
      );
    const describe = await rpc("system.describe");
    if (describe?.result?.protocolVersion?.major !== 1)
      throw new Error("Native bridge returned an invalid describe response");
    nativeBridge.initial = {
      protocolVersion: describe.result.protocolVersion,
      build: describe.result.build,
    };
    const access = await rpc("devices.getAccess");
    if (access?.result?.allowed !== false)
      throw new Error(
        `Fresh install unexpectedly grants device access: ${JSON.stringify(access)}`,
      );
    const status = await rpc("status.get");
    if (status?.result?.activeSessionCount !== 0)
      throw new Error(
        `Fresh install is not stopped: ${JSON.stringify(status)}`,
      );
    Object.assign(nativeBridge.initial, {
      activeSessionCount: status.result.activeSessionCount,
      deviceAccessAllowed: access.result.allowed,
    });

    const sessionName = `Packaged smoke ${new Date().toISOString().replace(/[:.]/g, "-")}`;
    await page.getByRole("tab", { name: "Session", exact: true }).click();
    await page.getByRole("button", { name: "Rename", exact: true }).click();
    await page
      .getByRole("textbox", { name: "Session name", exact: true })
      .fill(sessionName);
    const saveStartedAt = Date.now();
    await page
      .locator(".topbar")
      .getByRole("button", { name: "Save", exact: true })
      .click();
    await page
      .locator(".global-action-message")
      .filter({ hasText: /saved.*revision/i })
      .waitFor({ timeout: options.timeout });
    const saved = await rpc("sessions.get", { sessionId: "desktop-session" });
    if (saved?.result?.name !== sessionName)
      throw new Error(
        `Saved session did not persist: ${JSON.stringify(saved)}`,
      );
    metrics.sessionSaveMs = Date.now() - saveStartedAt;
    if (errors.length > 0)
      throw new Error(`Uncaught UI errors: ${errors.join(" | ")}`);

    // The owned WebView dies with quit_app. Finalize while CDP is still alive;
    // stopping after process exit races the transport's disconnect event.
    await traceContext.tracing.stop({
      path: path.join(options.output, "initial-trace.zip"),
    });
    traceStarted = false;
    await page.evaluate(() => {
      const core = window.__TAURI__?.core;
      void core?.invoke?.("quit_app")?.catch(() => undefined);
    });
    const quitCode = await waitForExit(child, 15_000);
    if (quitCode === null)
      throw new Error(
        "The smoke-owned shell did not exit after a graceful quit",
      );
    if (quitCode !== 0)
      throw new Error(`Shell quit exited unsuccessfully (${quitCode})`);
    cleanupActions.push({ pid: child.pid, method: "graceful-quit" });
    await browser.close().catch(() => undefined);
    browser = undefined;
    page = undefined;

    await unlink(devToolsActivePort).catch(() => undefined);
    abort.signal.throwIfAborted();
    const relaunchedAt = Date.now();
    child = spawn(options.shell, [], {
      env,
      stdio: ["ignore", stdoutFd, stderrFd],
      windowsHide: false,
      detached: process.platform === "win32",
    });
    child.on("error", (error) =>
      errors.push(`Failed to restart shell: ${error.message}`),
    );
    launchedChildren.push({ phase: "restarted", child });
    const relaunchEndpoint = await waitForFile(
      devToolsActivePort,
      child,
      options.timeout,
      errors,
      abort.signal,
    );
    const relaunchTargets = await listDevToolsTargets(relaunchEndpoint.port);
    devToolsTargets = relaunchTargets;
    browser = await connectToWebView(relaunchEndpoint, options.timeout);
    webview2Version = browser.version();
    traceContext = browser.contexts()[0];
    if (!traceContext)
      throw new Error(
        "Relaunched WebView2 connected without a browser context",
      );
    await traceContext.tracing.start({
      screenshots: true,
      snapshots: true,
      sources: true,
    });
    traceStarted = true;
    page = await waitForAppPage(
      browser,
      child,
      options.timeout,
      relaunchTargets,
      abort.signal,
    );
    page.on("pageerror", (error) => errors.push(error.message));
    await page.waitForLoadState("domcontentloaded", {
      timeout: options.timeout,
    });
    await waitForCanvas(page, options.timeout, abort.signal, "restarted");
    await qualificationCheckpoint(
      qualification,
      "restarted-ready",
      child,
      abort.signal,
      qualificationControl,
    );
    await page.getByRole("tab", { name: "Session", exact: true }).click();
    const restoredName = await page
      .getByLabel("Choose session", { exact: true })
      .locator("option:checked")
      .textContent();
    const restored = await rpc("sessions.get", {
      sessionId: "desktop-session",
    });
    if (restoredName !== sessionName || restored?.result?.name !== sessionName)
      throw new Error(
        "Saved session name did not survive restart in both the UI and backend",
      );
    const restartedDescribe = await rpc("system.describe");
    if (restartedDescribe?.result?.protocolVersion?.major !== 1)
      throw new Error(
        "Native bridge returned an invalid describe response after restart",
      );
    nativeBridge.restarted = {
      protocolVersion: restartedDescribe.result.protocolVersion,
      build: restartedDescribe.result.build,
    };
    const relaunchedStatus = await rpc("status.get");
    const relaunchedAccess = await rpc("devices.getAccess");
    if (relaunchedStatus?.result?.activeSessionCount !== 0)
      throw new Error(
        `Audio is not stopped after restart: ${JSON.stringify(relaunchedStatus)}`,
      );
    if (relaunchedAccess?.result?.allowed !== false)
      throw new Error(
        `Device access changed across restart: ${JSON.stringify(relaunchedAccess)}`,
      );
    Object.assign(nativeBridge.restarted, {
      activeSessionCount: relaunchedStatus.result.activeSessionCount,
      deviceAccessAllowed: relaunchedAccess.result.allowed,
    });
    if (errors.length > 0)
      throw new Error(
        `Uncaught UI errors after restart: ${errors.join(" | ")}`,
      );
    metrics.warmUiReadyMs = Date.now() - relaunchedAt;
    failure = undefined;
  } catch (error) {
    failure =
      error instanceof Error ? (error.stack ?? error.message) : String(error);
    if (
      page &&
      qualification &&
      ["bridge", "cleanup-fallback"].includes(qualification.case)
    ) {
      qualification.hits = await page
        .evaluate(() => window.__AUDIO_ROUTER_SMOKE_FAULT_HITS__ ?? 0)
        .catch(() => 0);
    }
    if (page && !cancelled) {
      try {
        await page.screenshot({
          path: path.join(options.output, "failure.png"),
          fullPage: true,
          timeout: 10_000,
        });
        await writeFile(
          path.join(options.output, "failure.html"),
          await page.content(),
        );
      } catch {
        // The page may be gone; preserve the primary failure.
      }
    }
  } finally {
    await cancellationWork;
    if (!failure && !cancelled && child && !childIsRunning(child))
      cleanupFailure = "Shell exited before final graceful cleanup";
    if (traceStarted && traceContext) {
      try {
        await traceContext.tracing.stop(
          failure
            ? { path: path.join(options.output, "trace.zip") }
            : undefined,
        );
      } catch {
        // Preserve the primary failure; the screenshot and DOM snapshot remain available.
      }
    }
    if (child) {
      if (childIsRunning(child)) {
        try {
          await page?.evaluate(() => {
            const core = window.__TAURI__?.core;
            void core?.invoke?.("quit_app")?.catch(() => undefined);
          });
          const code = await waitForExit(child, 5_000);
          if (code === null) await stopOwnedChild(child, cleanupActions);
          else if (code !== 0)
            cleanupFailure = `Smoke-owned shell quit unsuccessfully (${code})`;
          else cleanupActions.push({ pid: child.pid, method: "graceful-quit" });
        } catch {
          try {
            await stopOwnedChild(child, cleanupActions);
          } catch (error) {
            cleanupFailure =
              error instanceof Error ? error.message : String(error);
          }
        }
      }
    }
    if (browser) await browser.close().catch(() => undefined);
    for (const name of ["shell.jsonl", "shell.panic", "backend.jsonl"]) {
      await boundedCopy(
        path.join(data, "AudioRouter", "logs", name),
        path.join(options.output, name),
      );
    }
    if (stdoutFd !== undefined) closeSync(stdoutFd);
    if (stderrFd !== undefined) closeSync(stderrFd);
    await boundedCopy(
      path.join(profile, "shell.stdout.raw"),
      path.join(options.output, "shell.stdout.log"),
    );
    await boundedCopy(
      path.join(profile, "shell.stderr.raw"),
      path.join(options.output, "shell.stderr.log"),
    );
    if (cleanupFailure)
      failure = `${failure ? `${failure}\n` : ""}Cleanup failure: ${cleanupFailure}`;
    if (cancelled)
      failure = `Packaged smoke cancelled by ${cancelled}${cleanupFailure ? `; cleanup failed: ${cleanupFailure}` : ""}`;
    const report = {
      status: cancelled ? "cancelled" : failure ? "failed" : "passed",
      failure: failure ?? null,
      cancellation: cancelled,
      qualification,
      cleanupFailure: cleanupFailure ?? null,
      cleanupActions,
      launchedChildren: launchedChildren.map(({ phase, child: launched }) => ({
        phase,
        pid: launched.pid ?? null,
        exitCode: launched.exitCode,
        signalCode: launched.signalCode,
        running: childIsRunning(launched),
      })),
      sha: getCurrentSha(),
      workingTreeDirty:
        spawnSync("git", ["status", "--porcelain"], {
          encoding: "utf8",
        }).stdout.trim().length > 0,
      shell: options.shell,
      shellSha256: shellHash ?? null,
      windows: process.platform,
      node: process.version,
      windowsVersion: os.version(),
      windowsBuild: os.release(),
      webview2Version,
      durationMs: Date.now() - started,
      metrics,
      devToolsTargets,
      nativeBridge,
      uiReadyTargetsMs: { cold: 8_000, warm: 3_000 },
      uiReadyTargetsGated: false,
      timingGateNote:
        "Fresh-profile and restart timings are baseline samples, not OS-cold measurements; Q5 requires runner-variance calibration before enforcing these targets.",
      uncaughtUiErrors: errors,
      output: options.output,
    };
    writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
    for (const [name, handler] of signalHandlers) process.off(name, handler);
    qualificationControl?.off("close", onDisconnect);
    qualificationControl?.end();
  }
  if (failure) {
    process.stderr.write(
      `Packaged shell smoke failed. Evidence: ${options.output}\n${failure}\n`,
    );
    process.exitCode = cancelled === "SIGTERM" ? 143 : cancelled ? 130 : 1;
  } else {
    process.stdout.write(
      `Packaged shell smoke passed. Evidence: ${reportPath}\n`,
    );
  }
}

main().catch((error) => {
  process.stderr.write(
    `${error instanceof Error ? error.stack : String(error)}\n`,
  );
  process.exitCode = 1;
});
