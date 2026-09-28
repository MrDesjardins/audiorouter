import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";
import { join } from "node:path";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`Bass and Treble frequency controls in ${theme}`, async ({ page }) => {
    const session = { ...demoSession, nodes: demoSession.nodes.map(node => node.kind === "gain" ? { ...node, kind: "bassTreble" as const, parameters: { bassDb: 0, trebleDb: 0, bassFrequencyHz: 500, trebleFrequencyHz: 1500 } } : node) };
    await page.addInitScript(session => Object.assign(window, { __routeFixtureSession: session, __routeFixtureProcessors: [{ id: "bassTreble", version: 1, category: "native", availability: { status: "available" }, latencySamples: 0, parameters: [
      { name: "bassDb", type: "number", minimum: -12, maximum: 12, default: 0, unit: "dB" },
      { name: "trebleDb", type: "number", minimum: -12, maximum: 12, default: 0, unit: "dB" },
      { name: "bassFrequencyHz", type: "number", minimum: 80, maximum: 1000, default: 500, unit: "Hz" },
      { name: "trebleFrequencyHz", type: "number", minimum: 800, maximum: 12000, default: 1500, unit: "Hz" },
    ] }] }), session);
    await page.goto("/route-harness.html");
    await page.getByLabel("Color theme").selectOption(theme);
    const id = session.nodes.find(node => node.kind === "bassTreble")!.id;
    await page.getByTestId(`rf__node-${id}`).locator(".flow-node-title").click();
    await expect(page.getByLabel("Bass frequency precise value", { exact: true })).toHaveValue("500");
    await expect(page.getByLabel("Treble frequency precise value", { exact: true })).toHaveValue("1500");
    await page.getByLabel("Treble frequency precise value", { exact: true }).scrollIntoViewIfNeeded();
    await page.screenshot({ path: join(process.env.TEMP ?? ".", `audiorouter-bass-treble-${theme}.png`) });
  });
}
