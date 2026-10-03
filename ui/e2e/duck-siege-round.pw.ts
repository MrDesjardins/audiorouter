import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";

// A Duck following the Siege round (Stats.cc): trigger choice, phase
// checkboxes and live status must use the app style in every theme.
const session = {
  ...demoSession,
  nodes: demoSession.nodes.map((node) => node.id === "voice"
    ? { ...node, kind: "duck", name: "Game duck", parameters: { trigger: "siegeRound", amountDb: 20, attackMs: 300, releaseMs: 800, duckBetweenRounds: false } }
    : node),
};
const telemetry = [{ nodeId: "voice", kind: "duck", meter: null, plugin: null, processor: { gainReductionDb: [20, 20], gateOpen: [true, true], inputLevelDb: [-120, -120], outputLevelDb: [-38, -38] } }];

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`Siege round Duck in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(({ session, telemetry, theme }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, {
        __routeFixtureSession: session,
        __routeFixtureRunning: true,
        __routeFixtureTelemetry: telemetry,
        __routeFixtureGameRound: { source: "statsCc", state: "connected", phase: "prep", feedConfigured: true },
      });
    }, { session, telemetry, theme });
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").click();
    const editor = page.getByLabel("Duck live view");
    await expect(editor.getByLabel("Duck trigger")).toHaveValue("siegeRound");
    await expect(editor.getByRole("status")).toHaveText("Ducking −20.0 dB");
    await expect(editor).toContainText("Stats.cc: preparation.");
    await expect(editor.getByLabel("Menu and matchmaking")).toBeChecked();
    await expect(editor.getByLabel("Between rounds and results")).not.toBeChecked();
    await expect(editor.getByLabel("Triggered by")).toHaveCount(0);
    await editor.scrollIntoViewIfNeeded();
    await editor.screenshot({ path: testInfo.outputPath(`duck-siege-round-${theme}.png`) });
    expect(await page.evaluate(() => (window as any).__routeFixtureCalls().filter((call: string) => call === "commit"))).toEqual([]);
  });
}
