import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";

// Advanced EQ shows the live sound entering it behind the response curve.
const session = {
  ...demoSession,
  nodes: demoSession.nodes.map((node) => node.id === "voice"
    ? { ...node, kind: "parametricEq", name: "Game EQ", parameters: { band0Enabled: true, band0Type: "peaking", band0FrequencyHz: 3000, band0GainDb: 6, band0Q: 1 } }
    : node),
};
const bandFrequenciesHz = Array.from({ length: 64 }, (_, index) => 20 * Math.pow(1000, index / 63));
// Game-like content: low rumble plus a footstep bump around 2–4 kHz.
const levelsDb = bandFrequenciesHz.map((hz) => -78 + 14 * Math.exp(-((Math.log2(hz / 80)) ** 2)) + 22 * Math.exp(-((Math.log2(hz / 3000)) ** 2) * 2));
const telemetry = [{ nodeId: "voice", kind: "parametricEq", meter: null, plugin: null, processor: null, spectrum: { levelsDb, bandFrequenciesHz } }];

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`Advanced EQ live spectrum in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(({ session, telemetry, theme }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __routeFixtureSession: session, __routeFixtureRunning: true, __routeFixtureTelemetry: telemetry });
    }, { session, telemetry, theme });
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").click();
    const editor = page.getByLabel("Advanced EQ editor");
    await expect(editor.locator(".advanced-eq-spectrum")).toHaveCount(1);
    await expect(editor.getByText("Live sound in (before EQ)")).toBeVisible();
    await editor.locator(".advanced-eq-graph").scrollIntoViewIfNeeded();
    await editor.locator(".advanced-eq-graph").screenshot({ path: testInfo.outputPath(`eq-spectrum-${theme}.png`) });
  });
}
