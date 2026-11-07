import { test, expect } from "./browser-test";
import { demoSession } from "../src/fixtures";

// A connected Gain, so its status can honestly read Ready rather than No input.
const connected = {
  ...demoSession,
  edges: [
    {
      id: "mic-voice",
      sourceNode: "mic",
      sourcePort: "out",
      destinationNode: "voice",
      destinationPort: "in",
      matrix: [1],
      enabled: true,
    },
  ],
};

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`inspector status remains clear in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ theme, connected }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: connected });
      },
      { theme, connected },
    );
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").click();
    await expect(page.getByLabel("Tool status: Ready")).toBeVisible();
    await expect(page.getByLabel("Tool status: Ready")).toHaveClass(/is-ready/);
    const width = await page.getByLabel("Tool status: Ready").evaluate((pill) => pill.getBoundingClientRect().width);
    await page.getByLabel("Bypass", { exact: true }).check();
    await expect(page.getByLabel("Tool status: Bypass")).toBeVisible();
    await expect(page.getByLabel("Tool status: Bypass")).toHaveClass(/is-bypassed/);
    expect(await page.getByLabel("Tool status: Bypass").evaluate((pill) => pill.getBoundingClientRect().width)).toBe(
      width,
    );
    await expect(page.getByText("This tool is bypassed.", { exact: false })).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath(`inspector-${theme}.png`) });
  });
}
