import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";
import { appendLibraryNode, appendDraftConnection } from "../src/draft";
import { join } from "node:path";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`inputless filters warn and allow direct playback in ${theme}`, async ({ page }) => {
    let session = appendLibraryNode(demoSession, "spectralGate");
    session = appendLibraryNode(session, "parametricEq");
    const gate = session.nodes.find((node) => node.kind === "spectralGate")!;
    const eq = session.nodes.find((node) => node.kind === "parametricEq")!;
    session = appendDraftConnection(session, "mic", "out", "headphones", "in");
    session = appendDraftConnection(session, gate.id, "out", eq.id, "in");
    await page.addInitScript((session) => Object.assign(window, { __routeFixtureSession: session, __routeFixtureRunning: true }), session);
    await page.goto("/route-harness.html");
    await page.getByLabel("Color theme").selectOption(theme);
    await expect(page.locator(".inactive-route-warning")).toContainText("FIR Filter Hz");
    await expect(page.locator(".inactive-route-warning")).toContainText("ignored during playback");
    await page.locator(".topbar").getByRole("button", { name: "Stop", exact: true }).click();
    await page.locator(".topbar").getByRole("button", { name: "Play", exact: true }).click();
    await expect(page.locator(".audio-run-state")).toContainText("Audio running");
    await expect(page.getByTestId(`rf__node-${gate.id}`)).toBeVisible();
    await expect(page.getByTestId(`rf__node-${eq.id}`)).toBeVisible();
    await page.screenshot({ path: join(process.env.TEMP ?? ".", `audiorouter-unfed-${theme}.png`) });
  });
}
