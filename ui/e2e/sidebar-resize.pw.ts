import { test, expect } from "./real-backend";
import path from "node:path";

test("the right panel can be widened by dragging or with the keyboard, and keeps its width", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  const panel = page.locator(".right-workbench");
  const handle = page.getByRole("separator", { name: "Resize the right panel" });
  const before = (await panel.boundingBox())!.width;
  const grip = (await handle.boundingBox())!;
  await page.mouse.move(grip.x + grip.width / 2, grip.y + 200);
  await page.mouse.down();
  await page.mouse.move(grip.x + grip.width / 2 - 200, grip.y + 200, { steps: 8 });
  await page.mouse.up();
  const widened = (await panel.boundingBox())!.width;
  expect(widened - before).toBeGreaterThan(180);
  expect(widened - before).toBeLessThan(220);
  // Advanced EQ benefits from the extra width.
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page
    .locator(".tool-card")
    .filter({ has: page.getByText("Advanced EQ", { exact: true }) })
    .click();
  await page.getByTestId("rf__node-parametricEq-1").locator(".flow-node-title").click();
  await page.screenshot({ path: path.resolve("../target/feature-confidence-visual/sidebar-widened.png") });
  await page.reload();
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  expect(Math.abs((await panel.boundingBox())!.width - widened)).toBeLessThan(2);
  await handle.focus();
  await handle.press("ArrowRight");
  expect((await panel.boundingBox())!.width).toBeLessThan(widened - 20);
  await handle.dblclick();
  expect(Math.abs((await panel.boundingBox())!.width - before)).toBeLessThan(2);
});
