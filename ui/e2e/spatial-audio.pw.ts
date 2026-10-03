import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";

// Surround to headphones on a Physical Input: the mode select, the loopback
// device choice and the guidance must use the app field style in every theme.
const session = {
  ...demoSession,
  nodes: demoSession.nodes.map((node) => node.kind === "physicalInput"
    ? { ...node, name: "Siege game", parameters: { endpointId: "surround-preview", spatialMode: "headphones" } }
    : node),
};
const inputId = session.nodes.find((node) => node.kind === "physicalInput")!.id;

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`surround to headphones inspector in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(({ session, theme }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __routeFixtureSession: session });
    }, { session, theme });
    await page.goto("/route-harness.html");
    await page.getByTestId(`rf__node-${inputId}`).click();
    const field = page.getByLabel("Spatial audio settings");
    await expect(page.getByLabel("Spatial audio mode")).toHaveValue("headphones");
    await expect(field).toContainText("Renders this 7.1 input with a measured head");
    const endpoint = page.getByLabel("Physical input endpoint");
    await expect(endpoint).toHaveValue("surround-preview");
    await expect(endpoint.locator("option", { hasText: "Loopback · 8-channel multichannel" })).toHaveCount(1);
    await page.getByLabel("Physical input binding").scrollIntoViewIfNeeded();
    await page.screenshot({ path: testInfo.outputPath(`spatial-audio-${theme}.png`) });
    await page.getByLabel("Spatial audio mode").selectOption("off");
    await expect(endpoint.locator("option", { hasText: "Loopback ·" })).toHaveCount(0);
    expect(await page.evaluate(() => (window as any).__routeFixtureCalls().filter((call: string) => call === "commit"))).toEqual([]);
  });
}
