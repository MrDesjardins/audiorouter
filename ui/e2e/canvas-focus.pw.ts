import { test, expect } from "./browser-test";
import { setColorTheme, fitCanvas } from "./workbench";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: keyboard focus visibly marks a canvas card without moving it`, async ({ page }) => {
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await setColorTheme(page, theme);
    await fitCanvas(page);
    const card = page.getByTestId("rf__node-voice");
    const before = await card.boundingBox();
    // Enter keyboard modality before focusing the same wrapper reached by Tab.
    await page.keyboard.press("Tab");
    await card.focus();
    await expect(card).toBeFocused();
    const ring = await card.evaluate((element) => {
      const style = getComputedStyle(element);
      return {
        visible: element.matches(":focus-visible"),
        style: style.outlineStyle,
        width: parseFloat(style.outlineWidth),
        offset: parseFloat(style.outlineOffset),
        color: style.outlineColor,
      };
    });
    expect(ring.visible).toBe(true);
    expect(ring.style).toBe("solid");
    expect(ring.width).toBeGreaterThanOrEqual(3);
    expect(ring.offset).toBeGreaterThanOrEqual(2);
    expect(ring.color).not.toBe("rgba(0, 0, 0, 0)");
    expect(await card.boundingBox()).toEqual(before);
    await page.screenshot({ path: `../target/ui-quality-review/round-25/card-focus-${theme}.png` });
    await page.keyboard.press("Tab");
    await expect(card).not.toBeFocused();
    await expect(card).toHaveCSS("outline-style", "none");
  });
}
