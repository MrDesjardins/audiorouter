// Real custom-protocol shell qualification for the 2026-11-09 review fixes.
// Uses generated audio sent only to an owned loopback UDP socket. No mic,
// speaker, private database, external peer or plugin execution is involved.
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { createSocket } from "node:dgram";
import { mkdir, readFile, writeFile, stat } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { chromium } from "../../ui/node_modules/@playwright/test/index.mjs";
import { packagedShellEnvironment } from "./packaged-shell-environment.mjs";

const root = path.resolve(import.meta.dirname, "../..");
const output = path.join(root, "target", `review-final-${Date.now()}`);
const shell = path.join(root, "src-tauri/target/debug/audiorouter-shell.exe");
const existing = spawnSync(
  "powershell.exe",
  [
    "-NoProfile",
    "-Command",
    "if (Get-Process audiorouter-shell -ErrorAction SilentlyContinue) { exit 1 }",
  ],
  { windowsHide: true },
);
assert.equal(
  existing.status,
  0,
  "Close every existing AudioRouter shell before this isolated check",
);
await mkdir(output, { recursive: true });
await new Promise((resolve, reject) => {
  const probe = net.connect(String.raw`\\.\pipe\audiorouter-control`);
  probe.once("connect", () => {
    probe.destroy();
    reject(
      new Error("Close the existing AudioRouter before this isolated check"),
    );
  });
  probe.once("error", (error) =>
    ["ENOENT", "ECONNREFUSED"].includes(error.code) ? resolve() : reject(error),
  );
});
assert.ok(
  (await stat(shell)).mtimeMs >
    (await stat(path.join(root, "ui/dist/index.html"))).mtimeMs,
  "Rebuild the embedded UI first",
);
const udp = createSocket("udp4");
await new Promise((resolve) => udp.bind(0, "127.0.0.1", resolve));
let packets = 0;
let peak = 0;
udp.on("message", (packet) => {
  if (packet.toString("ascii", 0, 4) !== "ARNA" || packet[4] !== 1) return;
  packets++;
  peak = 0;
  for (let offset = 20; offset + 4 <= packet.length; offset += 4)
    peak = Math.max(peak, Math.abs(packet.readFloatLE(offset)));
});
const child = spawn(shell, [], {
  windowsHide: true,
  env: {
    ...packagedShellEnvironment(process.env, {
      database: path.join(output, "state.sqlite"),
      localAppData: path.join(output, "local"),
      appData: path.join(output, "roaming"),
      webview: path.join(output, "webview"),
    }),
    // A process-local test grant, with no physical capture/render nodes.
    AUDIOROUTER_ALLOW_DEVICE_ADMIN: "1",
  },
  stdio: ["ignore", "ignore", "pipe"],
});
let stderr = "";
child.stderr.on("data", (chunk) => {
  stderr = (stderr + chunk).slice(-16000);
});
let browser;
let page;
const evidence = { output, pid: child.pid, physicalAudioNodes: 0, checks: {} };
try {
  let port;
  const deadline = Date.now() + 30000;
  while (!port && Date.now() < deadline) {
    if (child.exitCode !== null)
      throw new Error(`Shell exited: ${child.exitCode}`);
    try {
      port = Number(
        (
          await readFile(
            path.join(output, "webview/EBWebView/DevToolsActivePort"),
            "utf8",
          )
        ).split(/\r?\n/)[0],
      );
    } catch {}
    if (!port) await delay(100);
  }
  assert.ok(port, "WebView2 did not expose its diagnostic port");
  browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  page = browser.contexts().flatMap((context) => context.pages())[0];
  page.setDefaultTimeout(10000);
  await page.waitForFunction(
    () => typeof window.__TAURI_INTERNALS__?.invoke === "function",
  );
  const rpc = async (method, params, allowError = false) => {
    const response = await page.evaluate(
      ({ method, params }) =>
        window.__TAURI_INTERNALS__.invoke("rpc_request", {
          request: { jsonrpc: "2.0", id: crypto.randomUUID(), method, params },
        }),
      { method, params },
    );
    if (!allowError)
      assert.equal(
        response.error,
        undefined,
        `${method}: ${JSON.stringify(response.error)}`,
      );
    return allowError ? response : response.result;
  };
  const fetched = await rpc("sessions.get", { sessionId: "desktop-session" });
  const current = fetched.session ?? fetched;
  const portSpec = (name, direction) => ({ name, direction, channels: 2 });
  const node = (id, kind, name, ports, parameters) => ({
    id,
    kind,
    name,
    typeVersion: 1,
    enabled: true,
    bypass: false,
    ports,
    parameters,
  });
  const edge = (from, to) => ({
    id: `${from}-${to}`,
    sourceNode: from,
    sourcePort: "out",
    destinationNode: to,
    destinationPort: "in",
    matrix: [1, 0, 0, 1],
    enabled: true,
  });
  const candidate = {
    ...current,
    name: "Review generated audio",
    nodes: [
      node(
        "review-tone",
        "testSignal",
        "Review tone",
        [portSpec("out", "output")],
        { frequencyHz: 997, levelDb: -12, durationMs: 600000 },
      ),
      node(
        "review-gain",
        "gain",
        "Review gain",
        [portSpec("in", "input"), portSpec("out", "output")],
        { gainDb: -12 },
      ),
      node(
        "review-send",
        "networkSend",
        "Review loopback",
        [portSpec("in", "input")],
        { host: "127.0.0.1", port: udp.address().port },
      ),
    ],
    edges: [
      edge("review-tone", "review-gain"),
      edge("review-gain", "review-send"),
    ],
  };
  const plan = await rpc("graph.plan", {
    sessionId: current.id,
    baseRevision: current.revision,
    candidate,
  });
  await rpc("graph.commit", {
    planId: plan.planId,
    baseRevision: current.revision,
    idempotencyKey: "review-route",
  });
  await page.reload();
  const notNow = page.getByRole("button", { name: "Not now", exact: true });
  if (await notNow.isVisible()) await notNow.click();
  await page.locator(".react-flow__controls-fitview").click();
  await delay(350);
  // Header Play exercises real worker selection. Source transport is also a
  // public API user path; the canvas tone control currently requires a
  // physical-output path and is disabled for this network-only fixture.
  evidence.checks.canvasToneEnabled = await page
    .getByRole("button", { name: "Play tone", exact: true })
    .isEnabled();
  await page.getByRole("button", { name: "Play", exact: true }).click();
  await page
    .getByRole("button", { name: "Stop", exact: true })
    .waitFor({ state: "visible" });
  await rpc("audioSources.transport", {
    sessionId: current.id,
    nodeId: "review-tone",
    action: "play",
  });
  const untilPeak = async (predicate) => {
    const until = Date.now() + 8000;
    while (!predicate(peak) && Date.now() < until) await delay(25);
    assert.ok(predicate(peak), `Unexpected loopback peak: ${peak}`);
    return peak;
  };
  evidence.checks.processedPeak = await untilPeak(
    (value) => value > 0.04 && value < 0.09,
  );
  await page.locator('.react-flow__node[data-id="review-gain"]').click();
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
  const hold = page.getByRole("button", {
    name: "Hold: without this tool",
    exact: true,
  });
  await hold.waitFor({ state: "visible" });
  await hold.scrollIntoViewIfNeeded();
  await hold.focus();
  const before = await page.locator(".tool-compare-buttons").boundingBox();
  await page.keyboard.down("Space");
  evidence.checks.heldPeak = await untilPeak(
    (value) => value > 0.2 && value < 0.3,
  );
  const refused = await rpc(
    "sessions.delete",
    { sessionId: current.id, idempotencyKey: "refused-live-delete" },
    true,
  );
  assert.ok(refused.error, "Deleting a playing route must fail");
  assert.equal(await hold.getAttribute("aria-pressed"), "true");
  await page.keyboard.up("Space");
  evidence.checks.releasedPeak = await untilPeak(
    (value) => value > 0.04 && value < 0.09,
  );
  evidence.checks.refusedDeleteKeptHold = true;
  const after = await page.locator(".tool-compare-buttons").boundingBox();
  assert.deepEqual(after, before, "Compare must not shift the button row");
  await hold.focus();
  await page.keyboard.down("Space");
  await untilPeak((value) => value > 0.2 && value < 0.3);
  await page.waitForFunction(
    () =>
      document
        .querySelector(".tool-compare button")
        ?.getAttribute("aria-pressed") === "false",
    null,
    { timeout: 35000 },
  );
  evidence.checks.expiredPeak = await untilPeak(
    (value) => value > 0.04 && value < 0.09,
  );
  await page.keyboard.up("Space");
  for (const theme of ["dark", "light", "high-contrast"]) {
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await page.getByRole("tab", { name: "Appearance", exact: true }).click();
    await page.getByLabel("Color theme", { exact: true }).selectOption(theme);
    await page
      .getByRole("button", { name: "Back to the route", exact: true })
      .click();
    await page.getByRole("tab", { name: "Properties", exact: true }).click();
    await page.screenshot({ path: path.join(output, `compare-${theme}.png`) });
  }
  const packetStart = packets;
  await page.evaluate(() => {
    window.__reviewTicks = 0;
    window.__reviewTickTimer = setInterval(() => window.__reviewTicks++, 16);
  });
  const scanStart = Date.now();
  await rpc("plugins.scan", {
    directory: "C:\\Program Files\\Common Files\\VST3",
  });
  evidence.checks.scan = {
    durationMs: Date.now() - scanStart,
    packets: packets - packetStart,
    uiTicks: await page.evaluate(() => {
      clearInterval(window.__reviewTickTimer);
      return window.__reviewTicks;
    }),
  };
  assert.ok(
    evidence.checks.scan.packets > 10 && evidence.checks.scan.uiTicks > 10,
    "Audio and UI must keep servicing during scan",
  );
  const api = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("http_api_control", {
      action: "start",
      port: 0,
    }),
  );
  const secret = await page.evaluate(() =>
    window.__TAURI_INTERNALS__.invoke("http_api_control", { action: "reveal" }),
  );
  const refusedNetwork = await fetch(`${api.url}/api/v1/plugins/scan`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${secret.token}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ directory: "\\\\host\\share" }),
  });
  assert.equal(refusedNetwork.status, 403);
  evidence.checks.httpNetworkPath = 403;
  await rpc("session.stop", {
    sessionId: current.id,
    idempotencyKey: "stop-review",
  });
  await page.screenshot({ path: path.join(output, "final-stopped.png") });
  console.log(JSON.stringify(evidence, null, 2));
} catch (error) {
  evidence.error = error.stack || String(error);
  await page
    ?.screenshot({ path: path.join(output, "failure.png") })
    .catch(() => {});
  await writeFile(
    path.join(output, "page.txt"),
    (await page
      ?.locator("body")
      .innerText()
      .catch(() => "unavailable")) ?? "unavailable",
  );
  throw error;
} finally {
  await writeFile(
    path.join(output, "evidence.json"),
    JSON.stringify(evidence, null, 2),
  );
  await writeFile(path.join(output, "stderr.txt"), stderr);
  await browser?.close().catch(() => {});
  if (child.exitCode === null)
    spawnSync("taskkill.exe", ["/PID", String(child.pid), "/T", "/F"], {
      windowsHide: true,
    });
  udp.close();
}
