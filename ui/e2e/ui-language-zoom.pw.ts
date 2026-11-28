import { expect, test } from "./browser-test";
import { openSettings } from "./workbench";

/**
 * L10N-07: a longer catalog must still fit at 200 % zoom in every theme.
 * A 720 x 500 viewport is the 1440 x 1000 layout at 200 % (CSS pixels halve).
 * The pseudolocale is development-only, so its cases skip in the production
 * browser build; the English cases always run.
 */
const THEMES = ["dark", "light", "high-contrast"];
const LANGUAGE_TAGS = ["en", "fr", "zh-Hans", "en-XA"];

async function horizontalOverflow(page: Parameters<typeof openSettings>[0]) {
  return page.evaluate(() => {
    const root = document.scrollingElement ?? document.documentElement;
    return { scroll: root.scrollWidth, client: root.clientWidth };
  });
}

for (const tag of LANGUAGE_TAGS) {
  for (const theme of THEMES) {
    test(`${tag} fits the Appearance page at 200 % in ${theme}`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width: 720, height: 500 });
      await page.goto("/route-harness.html");
      await openSettings(page, "Appearance");
      const selects = page.locator("section.theme-options select");
      await expect(selects).toHaveCount(2);
      if (tag === "en-XA") {
        const hasPseudo = (await selects.nth(0).locator('option[value="en-XA"]').count()) > 0;
        test.skip(!hasPseudo, "the pseudolocale exists only in a development build");
      }
      await selects.nth(0).selectOption(tag);
      await selects.nth(1).selectOption(theme);
      await expect(page.locator("html")).toHaveAttribute("lang", tag);
      await expect(page.locator("#theme-options-title")).toBeVisible();
      await expect(page.locator(".settings-back")).toBeVisible();
      await expect(page.locator('[role="tablist"]').first()).toBeVisible();

      const size = await horizontalOverflow(page);
      expect(size.scroll, `${tag}/${theme}: no horizontal page overflow`).toBeLessThanOrEqual(size.client + 1);

      await page.screenshot({ path: testInfo.outputPath(`appearance-${tag}-${theme}.png`), fullPage: true });

      await page.locator(".settings-back").click();
      await expect(page.locator(".settings-back")).toBeHidden();
      const workbench = await horizontalOverflow(page);
      expect(workbench.scroll, `${tag}/${theme}: the route fits at 200 %`).toBeLessThanOrEqual(workbench.client + 1);
      await page.screenshot({ path: testInfo.outputPath(`route-${tag}-${theme}.png`), fullPage: true });
    });
  }
}
