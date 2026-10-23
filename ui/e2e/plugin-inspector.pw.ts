import { test, expect, type Page } from "./browser-test";
import type { PluginScanEntry } from "@audiorouter/contracts";
import { demoSession } from "../src/fixtures";
import { appendPluginPlaceholderNode } from "../src/draft";

const entry: PluginScanEntry = {
  path: "C:\\Program Files\\Common Files\\VST3\\uaudio_ua_1176.vst3",
  identity: {
    path: "C:\\Program Files\\Common Files\\VST3\\uaudio_ua_1176.vst3",
    binaryPath: "C:\\Program Files\\Common Files\\VST3\\uaudio_ua_1176.vst3\\Contents\\x86_64-win\\uaudio_ua_1176.vst3",
    format: "vst3",
    architecture: "x64",
    fileBytes: 1024,
    sha256: "a".repeat(64),
    vendor: "Universal Audio",
    version: "1.0.0",
    classIds: [],
    compatibility: "supportedVst3X64",
  },
  error: null,
  errorCode: null,
};

const session = appendPluginPlaceholderNode(demoSession, entry);

const telemetry = (unchangedAudio: boolean) => [
  {
    nodeId: "plugin-1",
    kind: "plugin",
    meter: null,
    processor: null,
    plugin: { state: "running", failureCount: 0, outputMisses: 2, inputDrops: 0, unchangedAudio },
  },
];

async function openPlugin(page: Page, theme: string, unchangedAudio: boolean) {
  await page.addInitScript(
    ({ session, telemetry, theme }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, {
        __routeFixtureSession: session,
        __routeFixtureRunning: true,
        __routeFixtureTelemetry: telemetry,
      });
    },
    { session, telemetry: telemetry(unchangedAudio), theme },
  );
  await page.setViewportSize({ width: 1500, height: 950 });
  await page.goto("/route-harness.html");
  await page.getByTestId("rf__node-plugin-1").locator(".flow-node-title").click();
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
  const health = page.getByLabel("Plugin worker health");
  await expect(health).toContainText("Audio continuity");
  return health;
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`a plugin that passes audio unchanged suggests a licence check without moving the panel in ${theme}`, async ({
    page,
  }, testInfo) => {
    const health = await openPlugin(page, theme, true);
    const note = health.locator(".plugin-unchanged-audio");
    await expect(note).toBeVisible();
    await expect(note).toContainText("may not be licensed or activated");
    const flagged = (await health.boundingBox())!.height;
    await health.screenshot({ path: testInfo.outputPath(`plugin-unchanged-${theme}.png`) });

    const quiet = await page.context().newPage();
    const quietHealth = await openPlugin(quiet, theme, false);
    const quietNote = quietHealth.locator(".plugin-unchanged-audio");
    await expect(quietNote).toBeHidden();
    // The note's room is kept either way (UI-17).
    expect((await quietHealth.boundingBox())!.height).toBe(flagged);
  });
}
