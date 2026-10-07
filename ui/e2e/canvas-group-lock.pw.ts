import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";

test("a locked group cannot be dragged, resized or deleted by accident", async ({ page }) => {
  await page.addInitScript((session) => {
    Object.assign(window, { __routeFixtureSession: session });
  }, demoSession);
  await page.goto("/route-harness.html");
  await page.getByRole("button", { name: "Group A named visual background; no audio routing" }).click();
  await page.getByLabel("Lock group position and size").check();
  const group = page.locator(".visual-group-node");
  await expect(group).toHaveClass(/is-locked/);
  await expect(group.locator(".canvas-group-lock")).toBeVisible();
  await expect(group.locator(".react-flow__resize-control")).toHaveCount(0);
  const before = await group.boundingBox();
  const caption = (await group.locator(".canvas-group-caption").boundingBox())!;
  await page.mouse.move(caption.x + 20, caption.y + 12);
  await page.mouse.down();
  await page.mouse.move(caption.x + 140, caption.y + 90, { steps: 10 });
  await page.mouse.up();
  await page.locator(".canvas-group-caption").click();
  await page.keyboard.press("Delete");
  await expect(group).toHaveCount(1);
  const after = await group.boundingBox();
  // Dragging a locked group pans the canvas at most; its size never changes.
  expect(after!.width).toBeCloseTo(before!.width, 0);
  expect(after!.height).toBeCloseTo(before!.height, 0);
  const stored = await page.evaluate(() =>
    JSON.parse(Object.entries(localStorage).find(([key]) => key.startsWith("audiorouter.ui.groups."))![1]),
  );
  expect(stored[0]).toMatchObject({ locked: true, x: -40, y: -80, width: 540, height: 340 });
  // Unlocking restores the resize handles.
  await page.getByLabel("Lock group position and size").uncheck();
  await page.locator(".canvas-group-caption").click();
  await expect(group.locator(".react-flow__resize-control").first()).toBeVisible();
});
