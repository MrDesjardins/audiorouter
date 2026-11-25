#!/usr/bin/env node
// Synthetic whole-app soak driver (Q5.4). Connects to the packaged shell's
// WebView over loopback CDP and performs device-free graph edits while the
// PowerShell wrapper samples shell/WebView handles, threads, private bytes and
// logs. It never presses Play and needs no audio endpoint.
//
//   node tools/quality/packaged-shell-synthetic.mjs --port <n> --minutes <n> --output <activity.json>
//
// The caller owns the shell, its disposable database/profile and the debug
// port. This process only drives the real UI/backend and records activity.
import { writeFileSync } from "node:fs";
import path from "node:path";
import { chromium } from "../../ui/node_modules/@playwright/test/index.mjs";
import { dismissFirstLaunchGoal } from "./packaged-shell-onboarding.mjs";

const sessionId = "desktop-session";
const baseNodes = 3;
const targetNodes = 64;

function parseArgs(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index];
    if (!key.startsWith("--") || values.has(key)) throw new Error(`Unexpected argument: ${key}`);
    const value = argv[index + 1];
    if (!value || value.startsWith("--")) throw new Error(`Missing value for ${key}`);
    values.set(key, value);
    index += 1;
  }
  const port = Number(values.get("--port"));
  const minutes = Number(values.get("--minutes"));
  const output = values.get("--output");
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error("--port must be 1-65535");
  if (!Number.isFinite(minutes) || minutes < 0.05 || minutes > 120) throw new Error("--minutes must be 0.05-120");
  if (!output) throw new Error("--output is required");
  return { port, minutes, output: path.resolve(output) };
}

const options = parseArgs(process.argv.slice(2));
const deadline = Date.now() + options.minutes * 60_000;
const activity = {
  startedAt: new Date().toISOString(),
  edits: 0,
  reloads: 0,
  rpcErrors: 0,
  pageErrors: [],
  editTo64PaintedMs: null,
  jsHeapStartMiB: null,
  jsHeapEndMiB: null,
  lastNodeCount: 0,
  lastReloadAt: Date.now(),
};
let lastPersist = 0;
function persist() {
  writeFileSync(options.output, `${JSON.stringify(activity, null, 2)}\n`);
  lastPersist = Date.now();
}
persist();

async function connect(port) {
  const endpoint = `http://localhost:${port}`;
  const until = Date.now() + 30_000;
  let lastError;
  while (Date.now() < until) {
    try {
      return await chromium.connectOverCDP(endpoint, { timeout: 5_000, headers: { Origin: "http://localhost" } });
    } catch (error) {
      lastError = error;
      await new Promise(resolve => setTimeout(resolve, 500));
    }
  }
  throw new Error(`Could not connect to the shell DevTools endpoint: ${lastError?.message}`);
}

function pageOf(browser) {
  const pages = browser.contexts().flatMap(context => context.pages());
  const page = pages.find(item => !item.url().startsWith("devtools://")) ?? pages[0];
  if (!page) throw new Error("No WebView page was found");
  return page;
}

async function waitForCanvas(page, timeoutMs = 30_000) {
  await page.getByTestId("canvas-readiness").waitFor({ state: "attached", timeout: timeoutMs });
  await page.waitForFunction(
    () => document.querySelector('[data-testid="canvas-readiness"]')?.getAttribute("data-ready") === "true",
    undefined,
    { timeout: timeoutMs },
  );
}

async function heapMiB(page) {
  const cdp = await page.context().newCDPSession(page);
  try {
    await cdp.send("HeapProfiler.enable");
    await cdp.send("HeapProfiler.collectGarbage");
    await cdp.send("Performance.enable");
    const { metrics } = await cdp.send("Performance.getMetrics");
    const used = metrics.find(metric => metric.name === "JSHeapUsedSize")?.value;
    if (!Number.isFinite(used)) throw new Error("JSHeapUsedSize is unavailable after collection");
    return used / 1048576;
  } finally {
    await cdp.detach().catch(() => undefined);
  }
}

async function paintedNodeCount(page, expected) {
  return page.evaluate(async ({ expected }) => {
    const painted = () => document.querySelectorAll(".react-flow__node").length;
    const start = performance.now();
    while (performance.now() - start < 10_000) {
      if (painted() >= expected) {
        await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
        return painted();
      }
      await new Promise(resolve => setTimeout(resolve, 25));
    }
    return painted();
  }, { expected });
}

async function main() {
  const browser = await connect(options.port);
  const page = pageOf(browser);
  page.on("pageerror", error => activity.pageErrors.push(error.message));
  await waitForCanvas(page);
  await dismissFirstLaunchGoal(page, 20_000).catch(() => undefined);

  const rpc = (method, params = {}) =>
    page.evaluate(
      async ({ method: rpcMethod, params: rpcParams, id }) => {
        const core = globalThis.__TAURI__?.core;
        if (typeof core?.invoke !== "function") throw new Error("Tauri core invoke is unavailable");
        return await core.invoke("rpc_request", {
          request: { jsonrpc: "2.0", id, method: rpcMethod, params: rpcParams },
        });
      },
      { method, params, id: `synthetic-${method}-${Date.now()}-${Math.random().toString(16).slice(2)}` },
    );

  activity.jsHeapStartMiB = await heapMiB(page);

  let counter = 0;
  let added = [];
  const key = () => `synthetic-${++counter}`;
  const request = async (method, params = {}) => {
    const response = await rpc(method, params);
    if (response?.error) throw new Error(`${method}: ${response.error.message ?? JSON.stringify(response.error)}`);
    return response?.result;
  };
  const edit = async () => {
    if (added.length < targetNodes - baseNodes) {
      const result = await request("nodes.add", { kind: "gain", after: "desktop-input", idempotencyKey: key() });
      const nodeId = result?.nodeId;
      if (!nodeId) throw new Error("nodes.add returned no node id");
      added.push(nodeId);
      await request("nodes.set", {
        node: nodeId,
        parameters: { gainDb: Math.round(Math.sin(counter / 3) * 600) / 100 },
        idempotencyKey: key(),
      });
      return;
    }
    // The graph accepts at most 64 nodes. After the 64-node paint baseline,
    // test one removal and then use parameter/flag edits so the soak never
    // grows past the limit. A failed removal is recorded, not retried forever.
    if (!activity.removeTested) {
      activity.removeTested = true;
      const removed = added.shift();
      try {
        await request("nodes.remove", { node: removed, bridge: true, idempotencyKey: key() });
        activity.removePassed = true;
      } catch (error) {
        activity.removeError = error.message;
      }
    }
    const target = added[counter % added.length];
    await request("nodes.set", {
      node: target,
      parameters: { gainDb: Math.round(Math.cos(counter / 4) * 600) / 100 },
      enabled: counter % 7 !== 0,
      idempotencyKey: key(),
    });
  };

  while (Date.now() < deadline) {
    try {
      if (added.length === targetNodes - baseNodes - 1 && activity.editTo64PaintedMs === null) {
        // Timed from the backend nodes.add RPC through nodes.set and the UI
        // refresh until 64 cards exist. It is not a user input-to-paint time.
        const started = performance.now();
        await edit();
        const count = await paintedNodeCount(page, targetNodes);
        activity.editTo64PaintedMs = performance.now() - started;
        activity.lastNodeCount = count;
      } else {
        await edit();
      }
      activity.edits += 1;
      await request("status.get");
    } catch (error) {
      activity.rpcErrors += 1;
      if (activity.rpcErrors > 50) throw new Error(`Too many synthetic edit failures: ${error.message}`);
    }
    if (Date.now() - (activity.lastReloadAt ?? 0) > 120_000) {
      activity.lastReloadAt = Date.now();
      await page.reload({ waitUntil: "domcontentloaded" });
      await waitForCanvas(page).catch(() => undefined);
      await dismissFirstLaunchGoal(page, 5_000).catch(() => undefined);
      activity.reloads += 1;
    }
    if (Date.now() - lastPersist > 5_000) persist();
    await new Promise(resolve => setTimeout(resolve, 250));
  }

  try {
    activity.jsHeapEndMiB = await heapMiB(page);
  } catch (error) {
    // A reload can briefly detach the CDP session; retry once, then record why.
    try {
      await waitForCanvas(page, 10_000);
      activity.jsHeapEndMiB = await heapMiB(page);
    } catch (retryError) {
      activity.jsHeapEndError = retryError instanceof Error ? retryError.message : String(retryError);
    }
  }
  activity.lastNodeCount = await page.evaluate(() => document.querySelectorAll(".react-flow__node").length);
  activity.finishedAt = new Date().toISOString();
  persist();
  await browser.close().catch(() => undefined);
  if (activity.pageErrors.length) {
    console.error(`Uncaught page errors: ${activity.pageErrors.join("; ")}`);
    process.exitCode = 1;
  }
}

main().catch(error => {
  activity.error = error.message;
  activity.finishedAt = new Date().toISOString();
  try { persist(); } catch { /* report below */ }
  console.error(error.stack ?? error.message);
  process.exitCode = 1;
});