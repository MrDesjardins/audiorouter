import { test, expect } from "./browser-test";
import { fitCanvas } from "./workbench";
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

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`plugin recovery controls keep their size and show a copyable error in ${theme}`, async ({ page }, testInfo) => {
    test.setTimeout(60_000);
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session, __routeFixtureRunning: false });
      },
      { session, theme },
    );
    await page.setViewportSize({ width: 1500, height: 950 });
    await page.goto("/route-harness.html");
    await fitCanvas(page);
    await page.getByTestId("rf__node-plugin-1").locator(".flow-node-title").click();
    await page.getByRole("tab", { name: "Properties", exact: true }).click();

    const recovery = page.getByLabel("Plugin recovery", { exact: true });
    await recovery.scrollIntoViewIfNeeded();
    const useScanned = recovery.getByRole("button", { name: "Use scanned version", exact: true });
    const defaults = recovery.getByRole("button", { name: "Start from defaults", exact: true });
    await expect(useScanned).toBeEnabled();
    await expect(defaults).toBeEnabled();
    const idle = await recovery.boundingBox();
    await recovery.screenshot({ path: testInfo.outputPath(`plugin-recovery-idle-${theme}.png`) });

    // The disconnected harness backend refuses; the message must appear in its reserved slot (UI-17).
    await useScanned.click();
    await expect(recovery.getByRole("alert")).toContainText("Plugin recovery");
    await expect(recovery.getByRole("button", { name: /copy/i })).toBeVisible();
    expect(await recovery.boundingBox()).toEqual(idle);
    await recovery.screenshot({ path: testInfo.outputPath(`plugin-recovery-error-${theme}.png`) });
    await page.screenshot({ path: `../target/ui-quality-review/vst-fixes/plugin-recovery-${theme}.png` });
  });
}
