import { test, expect } from "./browser-test";
import { setColorTheme, fitCanvas } from "./workbench";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: canvas delete icons are centered`, async ({ page }) => {
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await setColorTheme(page, theme);
    await fitCanvas(page);
    const buttons = page.locator(".flow-node-delete");
    await expect(buttons.first()).toBeVisible();
    for (const button of await buttons.all()) {
      const offset = await button.evaluate((element) => {
        const box = element.getBoundingClientRect();
        const icon = element.querySelector("svg")!.getBoundingClientRect();
        return {
          x: Math.abs(icon.x + icon.width / 2 - box.x - box.width / 2),
          y: Math.abs(icon.y + icon.height / 2 - box.y - box.height / 2),
        };
      });
      expect(offset.x).toBeLessThan(0.6);
      expect(offset.y).toBeLessThan(0.6);
    }
    await page.screenshot({ path: `../target/canvas-delete-${theme}.png` });
  });
}
