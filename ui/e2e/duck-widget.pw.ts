import { test, expect, type Page } from "@playwright/test";
import { demoSession } from "../src/fixtures";

// The Duck canvas widget shows a duck while ducking; the Properties amount is a slider.
const session = (parameters: Record<string, unknown>) => ({
  ...demoSession,
  nodes: demoSession.nodes.map((node) =>
    node.id === "voice" ? { ...node, kind: "duck", name: "Game duck", parameters } : node,
  ),
});
const telemetry = (reduction: number, ducking: boolean) => [
  {
    nodeId: "voice",
    kind: "duck",
    meter: null,
    plugin: null,
    processor: {
      gainReductionDb: [reduction, reduction],
      gateOpen: [ducking, ducking],
      inputLevelDb: [-120, -120],
      outputLevelDb: [-38, -38],
    },
  },
];

async function open(
  page: Page,
  theme: string,
  parameters: Record<string, unknown>,
  reduction: number,
  ducking: boolean,
) {
  await page.addInitScript(
    ({ session, telemetry, theme }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, {
        __routeFixtureSession: session,
        __routeFixtureRunning: true,
        __routeFixtureTelemetry: telemetry,
      });
    },
    { session: session(parameters), telemetry: telemetry(reduction, ducking), theme },
  );
  await page.goto("/route-harness.html");
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`Duck widget shows a duck while ducking and a slider amount in ${theme}`, async ({ page }, testInfo) => {
    await open(page, theme, { trigger: "siegeRound", amountDb: 18 }, 18, true);
    const widget = page.getByTestId("rf__node-voice");
    await expect(widget.locator(".node-duck-badge")).toHaveCount(1);
    await expect(widget.getByRole("status")).toHaveText("Ducking −18.0 dB");
    await widget.screenshot({ path: testInfo.outputPath(`duck-widget-${theme}.png`) });
    await widget.locator(".flow-node-title").click();
    const slider = page.getByLabel("Duck live view").getByRole("slider", { name: "Duck amount" });
    await expect(slider).toHaveValue("18");
    await expect(page.getByLabel("Duck live view")).toContainText("Turn down by−18 dB");
    await page
      .getByLabel("Duck live view")
      .locator(".duck-amount")
      .screenshot({ path: testInfo.outputPath(`duck-slider-${theme}.png`) });
  });
}

test("a Siege-round Duck at full volume says so without a duck", async ({ page }) => {
  await open(page, "dark", { trigger: "siegeRound", amountDb: 18 }, 0, false);
  const widget = page.getByTestId("rf__node-voice");
  await expect(widget.getByRole("status")).toHaveText("Siege round · full volume");
  await expect(widget.locator(".node-duck-badge")).toHaveCount(0);
});
