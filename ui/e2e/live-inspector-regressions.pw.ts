import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";
const session = {
  ...demoSession,
  nodes: demoSession.nodes.map((n) =>
    n.id === "voice"
      ? { ...n, kind: "spectralGate", parameters: { learning: false, noiseProfile: "64".repeat(64) } }
      : n,
  ),
};
test("failed live autosave does not retry unchanged draft; explicit Save still works", async ({ page }) => {
  await page.addInitScript(
    (session) =>
      Object.assign(window, {
        __routeFixtureSession: session,
        __routeFixtureRunning: true,
        __routeFixturePlanConflict: true,
      }),
    session,
  );
  await page.goto("/route-harness.html");
  // Click the title: the card centre holds an inline threshold slider, and a
  // click there is a real edit that autosave saves before "Learn again".
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await page.getByRole("button", { name: "Learn again" }).click();
  await expect(page.locator(".global-action-message")).toContainText("Another save changed");
  // More than five previous 400 ms retry intervals.
  await page.waitForTimeout(2300);
  expect(await page.evaluate(() => (window as any).__routeFixturePlanCalls())).toBe(1);
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__routeFixturePlanCalls())).toBe(2);
});
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`reading digit boundary stays stable in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, {
          __routeFixtureSession: session,
          __routeFixtureRunning: true,
          __routeFixtureTelemetry: [
            {
              nodeId: "voice",
              kind: "spectral-gate",
              meter: {
                peakDb: -99.9,
                rmsDb: -99.9,
                clippedSamples: 0,
                channelPeakDb: [-99.9],
                channelRmsDb: [-99.9],
                channelClippedSamples: [0],
              },
              processor: null,
              plugin: null,
            },
          ],
        });
      },
      { session, theme },
    );
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
    const panel = page.locator(".node-telemetry");
    await expect(panel).toContainText("-99.9");
    const before = await panel.boundingBox();
    await page.evaluate(() => {
      const fixture = (window as any).__routeFixtureTelemetry[0].meter;
      fixture.peakDb = -100;
      fixture.rmsDb = -100;
    });
    await expect(panel).toContainText("-100.0");
    expect((await panel.boundingBox())?.height).toBe(before?.height);
    await page.screenshot({ path: testInfo.outputPath(`stable-readings-${theme}.png`) });
  });
}
