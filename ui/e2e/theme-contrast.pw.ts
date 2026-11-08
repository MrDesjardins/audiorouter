import { DEFAULT_CUSTOM_THEME } from "../src/customTheme";
import { expect, test } from "./browser-test";
import { openSettings } from "./workbench";

// Consistency criterion: a custom theme warns when its colours make text hard to read.
test("the theme editor warns about faint text colors and clears when fixed", async ({ page }, testInfo) => {
  await page.goto("/route-harness.html");
  await openSettings(page, "Appearance");
  await page.getByLabel("Import theme file").setInputFiles({
    name: "faint.json",
    mimeType: "application/json",
    buffer: Buffer.from(
      JSON.stringify({
        ...DEFAULT_CUSTOM_THEME,
        name: "Faint",
        colors: { ...DEFAULT_CUSTOM_THEME.colors, muted: "#3a414b" },
      }),
    ),
  });
  await page.getByText("Edit palette colors").click();
  const report = page.locator(".theme-contrast");
  await expect(report).toContainText("Some text may be hard to read");
  await expect(report).toContainText("Secondary text on app background");
  await report.scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath("theme-contrast-warning.png") });
  await page.getByLabel("Secondary text color").fill("#aeb9c8");
  await expect(report).toContainText("every pair meets the WCAG contrast guideline");
  await expect(report).not.toHaveClass(/has-warnings/);
});
