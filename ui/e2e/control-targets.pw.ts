import { expect, test } from "./browser-test";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: session name and map close have usable targets`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const session = page.locator(".header-session-name");
    const close = page.getByRole("button", { name: "Close the map", exact: true });
    for (const width of [1280, 640]) {
      await page.setViewportSize({ width, height: 720 });
      for (const control of [session, close]) {
        const box = (await control.boundingBox())!;
        expect(box.width).toBeGreaterThanOrEqual(24);
        expect(box.height).toBeGreaterThanOrEqual(24);
      }
      await session.focus();
      await page
        .locator(".topbar")
        .screenshot({ path: `../target/ui-quality-review/round-25/targets-header-${width}-${theme}.png` });
      await close.focus();
      const mapBox = (await close.boundingBox())!;
      await page.screenshot({
        path: `../target/ui-quality-review/round-25/targets-map-${width}-${theme}.png`,
        clip: { x: mapBox.x - 6, y: mapBox.y - 6, width: mapBox.width + 12, height: mapBox.height + 12 },
      });
    }
    await session.focus();
    await session.press("Enter");
    await expect(
      page.getByRole("tablist", { name: "Right sidebar" }).getByRole("tab", { name: "Session", exact: true }),
    ).toHaveAttribute("aria-selected", "true");
    await close.focus();
    const canvas = page.locator(".session-flow-canvas");
    const canvasBox = await canvas.boundingBox();
    const headerBox = await page.locator(".topbar").boundingBox();
    await close.press("Enter");
    await expect(close).toHaveCount(0);
    expect(await canvas.boundingBox()).toEqual(canvasBox);
    expect(await page.locator(".topbar").boundingBox()).toEqual(headerBox);
    await expect(page.getByRole("button", { name: "Map", exact: true })).toBeFocused();
    await page.getByRole("button", { name: "Map", exact: true }).click();
    const mapBox = (await close.boundingBox())!;
    await page.mouse.click(mapBox.x + mapBox.width - 2, mapBox.y + mapBox.height - 2);
    await expect(close).toHaveCount(0);
    await page.getByRole("tablist", { name: "Right sidebar" }).getByRole("tab", { name: "Tools", exact: true }).click();
    const sessionBox = (await session.boundingBox())!;
    await page.mouse.click(sessionBox.x + sessionBox.width - 2, sessionBox.y + sessionBox.height - 2);
    await expect(
      page.getByRole("tablist", { name: "Right sidebar" }).getByRole("tab", { name: "Session", exact: true }),
    ).toHaveAttribute("aria-selected", "true");
  });
}
