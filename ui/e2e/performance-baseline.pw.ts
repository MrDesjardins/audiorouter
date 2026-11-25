import { spawnSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import type { Browser } from "@playwright/test";
import { test, expect, type Page } from "./browser-test";
import { demoSession } from "../src/fixtures";

// Q5.5 informational baselines: a 64-node route-harness load to 64 painted
// cards (dev server, including module load), telemetry display freshness, and
// post-GC JS heap vs the renderer's OS private bytes. The load-to-paint number
// is not a user input-to-paint time and is not comparable to the synthetic
// soak's backend-edit timing. Run with AUDIOROUTER_PERF_BASELINE=1; the values
// are recorded, never gated.
test.skip(!process.env.AUDIOROUTER_PERF_BASELINE, "Set AUDIOROUTER_PERF_BASELINE=1 to record UI performance baselines");

const NODE_COUNT = 64;
const input = { ...demoSession.nodes[0], id: "perf-input", name: "Input" };
const output = { ...demoSession.nodes[2], id: "perf-output", name: "Output" };
const gainTemplate = demoSession.nodes[1];
const gains = Array.from({ length: NODE_COUNT - 2 }, (_, index) => ({
  ...gainTemplate,
  id: `perf-gain-${index}`,
  name: `Gain ${index + 1}`,
}));
const nodes = [input, ...gains, output];
const gainInput = gainTemplate.ports.find((port) => port.direction === "input")!.name;
const gainOutput = gainTemplate.ports.find((port) => port.direction === "output")!.name;
const edges = nodes.slice(0, -1).map((source, index) => ({
  id: `perf-edge-${index}`,
  sourceNode: source.id,
  sourcePort: index === 0 ? input.ports[0].name : gainOutput,
  destinationNode: nodes[index + 1].id,
  destinationPort: index + 1 === nodes.length - 1 ? output.ports[0].name : gainInput,
  matrix: [1],
  enabled: true,
}));
const session = { ...demoSession, id: "perf-session", name: "64-node baseline", revision: 0, nodes, edges };

async function inject(page: Page) {
  await page.addInitScript(
    ({ session }) => {
      Object.assign(window, {
        __routeFixtureSession: session,
        __routeFixtureRunning: true,
        __routeFixturePrivacyMuted: false,
      });
      Object.defineProperty(window, "__routeFixtureTelemetry", {
        configurable: true,
        get: () =>
          session.nodes.map((node, index) => {
            const rmsDb = -30 + Math.sin(Date.now() / 200 + index) * 12;
            return {
              nodeId: node.id,
              kind: node.kind,
              meter: {
                peakDb: rmsDb + 3,
                rmsDb,
                clippedSamples: 0,
                channelPeakDb: [rmsDb + 3, rmsDb + 2],
                channelRmsDb: [rmsDb, rmsDb - 1],
                channelClippedSamples: [0, 0],
              },
              processor: null,
              plugin: null,
            };
          }),
      });
    },
    { session },
  );
}

type Freshness = {
  error?: string;
  changes?: number;
  minMs?: number | null;
  medianMs?: number | null;
  p95Ms?: number | null;
  maxMs?: number | null;
};

async function telemetryFreshness(page: Page): Promise<Freshness> {
  return page.evaluate(async () => {
    const meter = document.querySelector(".react-flow__node .node-meter");
    if (!meter) return { error: "no meter element" };
    const start = performance.now();
    const changes: number[] = [];
    let previous = meter.getAttribute("aria-label");
    while (performance.now() - start < 3_000) {
      await new Promise((resolve) => setTimeout(resolve, 10));
      const current = meter.getAttribute("aria-label");
      if (current !== previous) {
        changes.push(performance.now());
        previous = current;
      }
    }
    const gaps = changes.map((time, index) => (index === 0 ? time - start : time - changes[index - 1]));
    gaps.sort((left, right) => left - right);
    const percentile = (fraction: number) =>
      gaps.length ? gaps[Math.min(gaps.length - 1, Math.floor(gaps.length * fraction))] : null;
    return {
      changes: changes.length,
      minMs: percentile(0),
      medianMs: percentile(0.5),
      p95Ms: percentile(0.95),
      maxMs: gaps.at(-1) ?? null,
    };
  });
}

async function postGcHeapMiB(page: Page): Promise<number | null> {
  try {
    const cdp = await page.context().newCDPSession(page);
    try {
      await cdp.send("HeapProfiler.enable");
      await cdp.send("HeapProfiler.collectGarbage");
      await cdp.send("Performance.enable");
      const { metrics } = await cdp.send("Performance.getMetrics");
      const used = metrics.find((metric) => metric.name === "JSHeapUsedSize")?.value;
      return typeof used === "number" && Number.isFinite(used) ? used / 1048576 : null;
    } finally {
      await cdp.detach().catch(() => undefined);
    }
  } catch {
    return null;
  }
}

async function rendererPrivateBytes(browser: Browser): Promise<{
  error?: string;
  processes?: Array<{ type: string; id: number; privateMiB: number | null }>;
  rendererPrivateMiB?: number;
  rendererCount?: number;
}> {
  if (process.platform !== "win32") return { error: "not Windows" };
  try {
    const cdp = await browser.newBrowserCDPSession();
    try {
      const { processInfo } = await cdp.send("SystemInfo.getProcessInfo");
      const rows = [];
      for (const process of processInfo) {
        const result = spawnSync(
          "powershell.exe",
          [
            "-NoProfile",
            "-Command",
            `(Get-Process -Id ${process.id} -ErrorAction SilentlyContinue).PrivateMemorySize64`,
          ],
          { encoding: "utf8" },
        );
        const bytes = Number(result.stdout.trim());
        rows.push({ type: process.type, id: process.id, privateMiB: Number.isFinite(bytes) ? bytes / 1048576 : null });
      }
      const renderer = rows.filter((row) => row.type === "renderer");
      return {
        processes: rows,
        rendererPrivateMiB: renderer.reduce((sum, row) => sum + (row.privateMiB ?? 0), 0),
        rendererCount: renderer.length,
      };
    } finally {
      await cdp.detach().catch(() => undefined);
    }
  } catch (error) {
    return { error: error instanceof Error ? error.message : String(error) };
  }
}

test("records 64-node paint, telemetry freshness and post-GC memory", async ({ page, browser }, testInfo) => {
  await inject(page);
  const started = Date.now();
  await page.goto("/route-harness.html");
  await expect(page.locator(".react-flow__node")).toHaveCount(NODE_COUNT, { timeout: 30_000 });
  const painted = await page.evaluate(async () => {
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    return document.querySelectorAll(".react-flow__node").length;
  });
  // From navigation through 64 attached cards and two animation frames: dev
  // module load + fixture render, not a user input-to-paint measurement.
  const loadTo64PaintedMs = Date.now() - started;
  const telemetry = await telemetryFreshness(page);
  const jsHeapMiB = await postGcHeapMiB(page);
  const memory = await rendererPrivateBytes(browser);
  const result = { nodeCount: painted, loadTo64PaintedMs, telemetry, postGcJsHeapMiB: jsHeapMiB, memory };
  writeFileSync(testInfo.outputPath("ui-performance.json"), `${JSON.stringify(result, null, 2)}\n`);
  console.log(`UI performance baseline: ${JSON.stringify(result)}`);
  expect(painted).toBe(NODE_COUNT);
  expect(telemetry.changes ?? 0).toBeGreaterThan(0);
});
