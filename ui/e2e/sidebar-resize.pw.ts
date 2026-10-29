import { setColorTheme } from "./workbench";
import { test, expect } from "./real-backend";

test("the right panel can be widened by dragging or with the keyboard, and keeps its width", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 900 });
  await page.goto("/backend-harness.html");
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  const panel = page.locator(".right-workbench");
  const handle = page.getByRole("separator", { name: "Resize the right panel" });
  const before = (await panel.boundingBox())!.width;
  const panelBox = (await panel.boundingBox())!;
  const handleBox = (await handle.boundingBox())!;
  expect(Math.abs(handleBox.y - panelBox.y)).toBeLessThan(2);
  expect(Math.abs(handleBox.height - panelBox.height)).toBeLessThan(2);
  for (const fraction of [0.05, 0.5, 0.95]) {
    const box = (await handle.boundingBox())!;
    const width = (await panel.boundingBox())!.width;
    const y = box.y + box.height * fraction;
    await page.mouse.move(box.x + box.width / 2, y);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2 - 35, y, { steps: 4 });
    await page.mouse.up();
    expect((await panel.boundingBox())!.width).toBeGreaterThan(width + 30);
    await handle.dblclick();
  }
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
  await page.screenshot({ path: test.info().outputPath("sidebar-widened.png") });
  await page.reload();
  await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
  expect(Math.abs((await panel.boundingBox())!.width - widened)).toBeLessThan(2);
  await handle.focus();
  await handle.press("ArrowRight");
  expect((await panel.boundingBox())!.width).toBeLessThan(widened - 20);
  await handle.dblclick();
  expect(Math.abs((await panel.boundingBox())!.width - before)).toBeLessThan(2);
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: full separator with Properties open`, async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/backend-harness.html");
    await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
    await setColorTheme(page, theme);
    await page.getByRole("tab", { name: "Properties", exact: true }).click();
    const handle = page.getByRole("separator", { name: "Resize the right panel" });
    const box = (await handle.boundingBox())!;
    const canvas = (await page.locator("#signal-flow-panel").boundingBox())!;
    expect(Math.abs(box.y - canvas.y)).toBeLessThan(2);
    expect(Math.abs(box.height - canvas.height)).toBeLessThan(2);
    const x = box.x + box.width / 2;
    const y = box.y + 20;
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x - 40, y, { steps: 4 });
    await page.mouse.up();
    expect((await handle.boundingBox())!.x).toBeLessThan(box.x - 35);
    await page.screenshot({ path: test.info().outputPath(`sidebar-${theme}.png`) });
  });
}
