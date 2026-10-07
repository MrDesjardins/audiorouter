import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";

const translate = (element: Element) => {
  const parts = (element as HTMLElement).style.transform.match(/translate\(([-\d.]+)px,\s*([-\d.]+)px\)/);
  if (!parts) throw Error("missing position");
  return { x: Number(parts[1]), y: Number(parts[2]) };
};

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`dragging a tool shows a live preview and drops it exactly there in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session });
      },
      { session: demoSession, theme },
    );
    await page.setViewportSize({ width: 1500, height: 950 });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("rf__node-mic")).toBeVisible();
    // Let the opening fit animation finish so the viewport is still.
    const viewport = () =>
      page.locator(".react-flow__viewport").evaluate((element) => (element as HTMLElement).style.transform);
    let last = "";
    await expect
      .poll(
        async () => {
          const now = await viewport();
          const still = now === last;
          last = now;
          return still;
        },
        { intervals: [150] },
      )
      .toBe(true);
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    const card = page.locator(".tool-card").filter({ has: page.getByText("Compressor", { exact: true }) });
    await card.scrollIntoViewIfNeeded();
    const canvas = page.locator(".session-flow-canvas");
    const source = (await card.boundingBox())!;
    const area = (await canvas.boundingBox())!;
    const target = { x: area.x + area.width * 0.55, y: area.y + area.height * 0.7 };
    await page.mouse.move(source.x + source.width / 2, source.y + source.height / 2);
    await page.mouse.down();
    await page.mouse.move(source.x + source.width / 2 - 40, source.y + source.height / 2, { steps: 4 });
    await page.mouse.move(target.x, target.y, { steps: 16 });
    // Emulated dragover reports the previous step; browsers repeat dragover
    // while the pointer rests, so give the preview one more event.
    await page.mouse.move(target.x + 1, target.y);
    await page.mouse.move(target.x, target.y);
    // The preview is a real Compressor card under the pointer.
    const preview = page.locator(".react-flow__node.flow-drop-preview");
    await expect(preview).toBeVisible();
    await expect(preview).toContainText("Compressor");
    const ghost = (await preview.boundingBox())!;
    const ghostAt = await preview.evaluate(translate);
    expect(target.x).toBeGreaterThan(ghost.x);
    expect(target.x).toBeLessThan(ghost.x + ghost.width);
    expect(target.y).toBeGreaterThan(ghost.y);
    expect(target.y).toBeLessThan(ghost.y + ghost.height);
    await page.screenshot({ path: testInfo.outputPath(`drag-preview-${theme}.png`) });
    await page.mouse.up();
    // The real node lands where the preview was; the preview is gone.
    const placed = page.getByTestId("rf__node-compressor-1");
    await expect(placed).toBeVisible();
    await expect(preview).toHaveCount(0);
    // Compare flow coordinates: a status line after the drop may shift the page.
    const placedAt = await placed.evaluate(translate);
    expect(Math.abs(placedAt.x - ghostAt.x)).toBeLessThanOrEqual(1);
    expect(Math.abs(placedAt.y - ghostAt.y)).toBeLessThanOrEqual(1);
    // A deliberate drop keeps the view; it does not refit away from the node.
    const settled = last;
    await page.waitForTimeout(400);
    expect(await viewport()).toBe(settled);
  });
}
