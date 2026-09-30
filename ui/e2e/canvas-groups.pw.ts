import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`visual groups remain behind editable audio in ${theme}`, async ({ page }) => {
    await page.addInitScript((session) => { Object.assign(window, { __routeFixtureSession: session }); }, { ...demoSession, edges: [
      { id: "mic-voice", sourceNode: "mic", sourcePort: "out", destinationNode: "voice", destinationPort: "in", matrix: [1], enabled: true },
      { id: "voice-output", sourceNode: "voice", sourcePort: "out", destinationNode: "headphones", destinationPort: "in", matrix: [1, 1], enabled: true },
    ] });
    await page.goto("/route-harness.html");
    await page.getByLabel("Color theme").selectOption(theme);
    const nodes = await page.locator(".react-flow__node-flowNode").count();
    const edges = await page.locator(".react-flow__edge").count();
    await page.getByRole("button", { name: "Group A named visual background; no audio routing" }).click();
    await expect(page.getByLabel("Group opacity")).toHaveValue("25");
    await page.getByLabel("Group name").fill("Game");
    await page.getByLabel("Group background color").fill("#ee8822");
    await page.getByLabel("Group opacity").fill("35");
    const group = page.locator(".visual-group-node");
    await expect(group).toHaveCount(1);
    await expect(group.locator(".canvas-group-caption")).toHaveText("Game");
    await expect(group.locator(".react-flow__handle")).toHaveCount(0);
    expect(await group.evaluate((element) => Number(getComputedStyle(element).zIndex))).toBeLessThan(0);
    const box = await group.locator(".canvas-group-caption").boundingBox();
    if (!box) throw new Error("Group caption not visible");
    await page.mouse.move(box.x + 20, box.y + 12); await page.mouse.down(); await page.mouse.move(box.x + 100, box.y + 62, { steps: 12 }); await page.mouse.up();
    const resize = group.locator(".react-flow__resize-control.bottom.right");
    const handle = await resize.boundingBox();
    if (!handle) throw new Error("Group resize control not visible");
    await page.mouse.move(handle.x + handle.width / 2, handle.y + handle.height / 2); await page.mouse.down(); await page.mouse.move(handle.x + 75, handle.y + 45, { steps: 10 }); await page.mouse.up();
    await expect(page.locator(".react-flow__node-flowNode")).toHaveCount(nodes);
    await expect(page.locator(".react-flow__edge")).toHaveCount(edges);
    await expect(page.getByRole("button", { name: "Save", exact: true })).toBeDisabled();
    await page.screenshot({ path: `../docs/plans/active/evidence/2026-09-29-groups-${theme}.png` });
    await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
    await expect(page.getByLabel("Node name", { exact: true })).toBeVisible();
    await page.getByRole("tab", { name: "API", exact: true }).click();
    await expect(page.getByLabel("API port")).toHaveValue("17891");
    await expect(page.getByRole("button", { name: "Start API" })).toBeDisabled();
    await page.screenshot({ path: `../docs/plans/active/evidence/2026-09-29-api-${theme}.png` });
    await page.reload();
    await expect(page.locator(".canvas-group-caption")).toHaveText("Game");
    await page.locator(".canvas-group-caption").click();
    await page.keyboard.press("Delete");
    await expect(page.locator(".visual-group-node")).toHaveCount(0);
    await expect(page.locator(".react-flow__node-flowNode")).toHaveCount(nodes);
  });
}
