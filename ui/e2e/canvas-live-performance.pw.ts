import { test, expect, type Page } from "@playwright/test";
import { demoSession } from "../src/fixtures";

// While audio plays, meters change 20 times a second. Those ticks must update
// the meters and moving connection lights without re-rendering the canvas:
// rebuilding every node card per tick made the WebView allocate ~47 MB/s.
// Four independent paths, like a game/voice/chat session: 12 nodes, 8 links.
const [input, gain, output] = demoSession.nodes;
const session = {
  ...demoSession,
  nodes: [0, 1, 2, 3].flatMap((path) => [input, gain, output].map((node) => ({ ...node, id: `${node.id}-${path}`, name: `${node.name} ${path + 1}` }))),
  edges: [0, 1, 2, 3].flatMap((path) => [
    { id: `in-gain-${path}`, sourceNode: `${input.id}-${path}`, sourcePort: input.ports[0].name, destinationNode: `${gain.id}-${path}`, destinationPort: gain.ports.find((port) => port.direction === "input")!.name, matrix: [1], enabled: true },
    { id: `gain-out-${path}`, sourceNode: `${gain.id}-${path}`, sourcePort: gain.ports.find((port) => port.direction === "output")!.name, destinationNode: `${output.id}-${path}`, destinationPort: output.ports[0].name, matrix: [1, 1], enabled: true },
  ]),
};

async function playWithChangingMeters(page: Page) {
  await page.addInitScript(({ session }) => {
    Object.assign(window, { __routeFixtureSession: session, __routeFixtureRunning: true, __routeFixturePrivacyMuted: false, __audiorouterCardRenders: 0 });
    Object.defineProperty(window, "__routeFixtureTelemetry", {
      configurable: true,
      get: () => session.nodes.map((node, index) => {
        const rmsDb = -30 + Math.sin(Date.now() / 200 + index) * 12;
        return { nodeId: node.id, kind: node.kind, meter: { peakDb: rmsDb + 3, rmsDb, clippedSamples: 0, channelPeakDb: [rmsDb + 3, rmsDb + 2], channelRmsDb: [rmsDb, rmsDb - 1], channelClippedSamples: [0, 0] }, processor: null, plugin: null };
      }),
    });
  }, { session });
  await page.goto("/route-harness.html");
  await expect(page.locator(".react-flow__node").first()).toBeVisible();
}

async function allocatedMbIn(page: Page, ms: number) {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("HeapProfiler.enable");
  await cdp.send("HeapProfiler.startSampling", { samplingInterval: 4096, includeObjectsCollectedByMajorGC: true, includeObjectsCollectedByMinorGC: true });
  await page.waitForTimeout(ms);
  const { profile } = await cdp.send("HeapProfiler.stopSampling");
  let total = 0;
  const walk = (node: typeof profile.head) => { total += node.selfSize; node.children.forEach(walk); };
  walk(profile.head);
  return total / 1048576;
}

test("meter ticks update the canvas without rebuilding it", async ({ page }) => {
  await playWithChangingMeters(page);
  await expect.poll(() => page.locator(".react-flow__edges .flow-edge-active").count()).toBeGreaterThan(0);
  const meter = page.locator(".react-flow__node .node-meter").first();
  const cardRenders = () => page.evaluate(() => (window as unknown as { __audiorouterCardRenders: number }).__audiorouterCardRenders);
  await page.waitForTimeout(500);
  const rendersBefore = await cardRenders();
  const firstReading = await meter.getAttribute("aria-label");
  // About 40 meter ticks: the readings move, the cards are not re-rendered.
  await expect.poll(() => meter.getAttribute("aria-label")).not.toBe(firstReading);
  await page.waitForTimeout(2000);
  await expect(page.locator(".react-flow__edges .flow-edge-active").first()).toBeAttached();
  const rendersAfter = await cardRenders();
  console.log(`card renders over ~2.5 s of meter ticks: ${rendersAfter - rendersBefore}`);
  expect(rendersAfter - rendersBefore).toBe(0);
  const megabytes = await allocatedMbIn(page, 5000);
  console.log(`canvas allocation while playing: ${megabytes.toFixed(1)} MB in 5 s`);
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`playing canvas keeps its live visuals in ${theme}, and Stop clears them`, async ({ page }, testInfo) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await playWithChangingMeters(page);
    await expect.poll(() => page.locator(".react-flow__edges .flow-edge-active").count()).toBeGreaterThan(0);
    await expect(page.locator(".react-flow__node .node-meter.is-active").first()).toBeVisible();
    await page.locator(".session-flow-canvas").screenshot({ path: testInfo.outputPath(`canvas-playing-${theme}.png`) });
    // Live values never outlive a state change: Stop clears the lights at once.
    await page.locator(".topbar").getByRole("button", { name: "Stop", exact: true }).click();
    await expect(page.locator(".react-flow__edges .flow-edge-active")).toHaveCount(0);
    await expect(page.locator(".react-flow__edges .flow-edge-stopped").first()).toBeAttached();
  });
}
