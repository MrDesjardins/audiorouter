import { expect, test } from "./browser-test";
import { openSettings } from "./workbench";

// CSS viewport equivalents only: actual Windows display scaling and Narrator
// remain attended gates F3/F4. Keep all shipped languages in this layout probe.
for (const language of ["en", "fr", "zh-Hans"]) {
  for (const theme of ["dark", "light", "high-contrast"]) {
    for (const [scale, width, height] of [
      ["150-percent", 960, 666],
      ["200-percent", 720, 500],
      ["narrow-200-percent", 480, 500],
    ] as const) {
      test(`${language}/${theme}/${scale}: settings and route fit without page overflow`, async ({ page }, info) => {
        await page.setViewportSize({ width, height });
        await page.goto("/route-harness.html");
        await openSettings(page, "Appearance");
        const selects = page.locator("section.theme-options select");
        await selects.nth(1).selectOption(theme);
        await selects.nth(0).selectOption(language);
        await expect(page.locator("html")).toHaveAttribute("lang", language);
        const overflow = () =>
          page.evaluate(() => {
            const root = document.scrollingElement ?? document.documentElement;
            return root.scrollWidth - root.clientWidth;
          });
        expect(await overflow()).toBeLessThanOrEqual(1);
        await expect(page.locator("#theme-options-title")).toBeVisible();
        await page.screenshot({ path: info.outputPath("appearance.png"), fullPage: true });
        await page.locator(".settings-back").click();
        await expect(page.locator(".settings-back")).toBeHidden();
        expect(await overflow()).toBeLessThanOrEqual(1);
        const tabs = page.getByRole("tab");
        await expect(tabs).toHaveCount(5);
        await tabs.first().focus();
        await page.keyboard.press("End");
        await expect(tabs.last()).toBeFocused();
        await page.keyboard.press("Home");
        await expect(tabs.first()).toBeFocused();
        await page.screenshot({ path: info.outputPath("route.png"), fullPage: true });
      });
    }
  }
}
