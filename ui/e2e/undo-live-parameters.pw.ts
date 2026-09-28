import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";

test("parameter Undo/Redo survives live autosave and later backend refreshes", async ({ page }) => {
  const session = { ...demoSession, nodes: demoSession.nodes.map((node) => node.id === "voice" ? { ...node, kind: "parametricEq" as const, parameters: { band0Enabled: true, band0Type: "peaking", band0GainDb: 0 } } : node), edges: [
    { id: "mic-voice", sourceNode: "mic", sourcePort: "out", destinationNode: "voice", destinationPort: "in", matrix: [1], enabled: true },
    { id: "voice-out", sourceNode: "voice", sourcePort: "out", destinationNode: "headphones", destinationPort: "in", matrix: [1, 1], enabled: true },
  ] };
  await page.addInitScript((session) => Object.assign(window, { __routeFixtureSession: session, __routeFixtureRunning: true }), session);
  await page.goto("/route-harness.html");
  await expect(page.locator(".audio-run-state")).toContainText("Audio running");
  const gainNode = demoSession.nodes.find((node) => node.kind === "gain")!;
  await page.getByTestId(`rf__node-${gainNode.id}`).locator(".flow-node-title").click();
  const value = page.getByLabel("EQ gain dB");
  const before = await value.inputValue();
  await value.fill("-12");
  await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
  await page.locator(".topbar").getByRole("button", { name: "Undo", exact: true }).click();
  await expect(value).toHaveValue(before);
  await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
  await page.waitForTimeout(1200); // cross the normal snapshot refresh
  await page.keyboard.press("Control+y");
  await expect(value).toHaveValue("-12");
  await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
  await value.fill("-8");
  await expect(page.locator(".topbar").getByRole("button", { name: "Redo", exact: true })).toBeDisabled();
});
