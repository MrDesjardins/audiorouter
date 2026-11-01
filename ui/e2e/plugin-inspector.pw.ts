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

const telemetry = (unchangedAudio: boolean, silencedUnchanged = false) => [
  {
    nodeId: "plugin-1",
    kind: "plugin",
    meter: null,
    processor: null,
    plugin: { state: "running", failureCount: 0, outputMisses: 2, inputDrops: 0, unchangedAudio, silencedUnchanged },
  },
];

async function openPlugin(page: Page, theme: string, unchangedAudio: boolean, silencedUnchanged = false) {
  await page.addInitScript(
    ({ session, telemetry, theme }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, {
        __routeFixtureSession: session,
        __routeFixtureRunning: true,
        __routeFixtureTelemetry: telemetry,
      });
    },
    { session, telemetry: telemetry(unchangedAudio, silencedUnchanged), theme },
  );
  await page.setViewportSize({ width: 1500, height: 950 });
  await page.goto("/route-harness.html");
  await page.getByTestId("rf__node-plugin-1").locator(".flow-node-title").click();
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
  await expect(page.getByLabel("Plugin worker health")).toContainText("Audio continuity");
  return page.getByLabel("Voice protection", { exact: true });
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`a plugin that passes audio unchanged suggests a licence check without moving the panel in ${theme}`, async ({
    page,
  }, testInfo) => {
    // Two full harness loads; slow hosted runners exceeded the 30 s default.
    test.setTimeout(60_000);
    const guard = await openPlugin(page, theme, true);
    await expect(guard.getByText("may not be licensed or activated")).toBeVisible();
    await expect(guard.getByRole("button", { name: "Retry" })).toBeHidden();
    // The muted steps are laid out underneath; none of their text may show through.
    await expect(guard.getByText("probably not licensed")).toBeHidden();
    const flagged = (await guard.boundingBox())!.height;
    await guard.screenshot({ path: testInfo.outputPath(`plugin-unchanged-${theme}.png`) });

    const quiet = await page.context().newPage();
    const quietGuard = await openPlugin(quiet, theme, false);
    await expect(quietGuard.getByText("may not be licensed or activated")).toBeHidden();
    await expect(quietGuard.getByText("AudioRouter mutes it")).toBeVisible();
    // The card keeps its size whichever status it shows (UI-17).
    expect((await quietGuard.boundingBox())!.height).toBe(flagged);
  });

  test(`a muted plugin guides to Retry or Allow unchanged audio without moving the panel in ${theme}`, async ({
    page,
  }, testInfo) => {
    test.setTimeout(60_000);
    const guard = await openPlugin(page, theme, true, true);
    await expect(guard.getByText("Muted: your voice came through unchanged")).toBeVisible();
    await expect(guard.getByText("may not be licensed or activated")).toBeHidden();
    // Both ways out sit in the same card: Retry, and the switch for a plugin meant to leave audio unchanged.
    await expect(guard.getByText("Expected an effect?")).toBeVisible();
    await expect(guard.getByRole("button", { name: "Retry" })).toBeVisible();
    await expect(guard.getByText("Meant to leave your voice unchanged?")).toBeVisible();
    await expect(guard.getByLabel("Allow unchanged audio")).not.toBeChecked();
    const muted = (await guard.boundingBox())!.height;
    await guard.screenshot({ path: testInfo.outputPath(`plugin-silenced-${theme}.png`) });
    await page
      .locator(".plugin-node-editor")
      .screenshot({ path: testInfo.outputPath(`plugin-silenced-inspector-${theme}.png`) });
    // The harness plugin still passes audio through, so Retry is refused.
    await guard.getByRole("button", { name: "Retry" }).click();
    await expect(guard.getByText("Still muted")).toBeVisible();
    expect((await guard.boundingBox())!.height).toBe(muted);
    // The switch goes through the same draft path as any setting; it once failed plugin range validation.
    await guard.getByLabel("Allow unchanged audio").click();
    await expect(guard.getByLabel("Allow unchanged audio")).toBeChecked();
    await expect(page.getByText(/Draft rejected/)).toHaveCount(0);

    const quiet = await page.context().newPage();
    const quietGuard = await openPlugin(quiet, theme, false);
    await expect(quietGuard.getByRole("button", { name: "Retry" })).toBeHidden();
    expect((await quietGuard.boundingBox())!.height).toBe(muted);
  });
}
