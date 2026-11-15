import { DEFAULT_CUSTOM_THEME } from "../src/customTheme";
import { hexToHsv } from "../src/themeColor";
import { expect, test } from "./browser-test";
import { openSettings } from "./workbench";

// Consistency criterion: a custom theme warns when its colours make text hard to read.
test("the theme editor warns about faint text colors and clears when fixed", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 1280, height: 900 });
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
  await report.getByRole("button", { name: "app background", exact: true }).click();
  await expect(page.getByRole("textbox", { name: "App background color", exact: true })).toBeFocused();
  const warning = report.locator("li").filter({ hasText: "Secondary text on app background" });
  await warning.getByRole("button", { name: "Secondary text", exact: true }).click();
  const muted = page.getByRole("textbox", { name: "Secondary text color", exact: true });
  await expect(muted).toBeFocused();
  await expect(muted).toHaveClass(/is-picker-target/);
  const picker = page.locator(".theme-color-popover");
  await expect(picker).toBeVisible();
  await muted.click();
  await expect(picker).toBeVisible();
  await expect(picker).toContainText("Contrast line: the dimmed side misses the guidance.");
  await expect(picker).toContainText(/needs 4\.5:1/);
  await expect(picker).toContainText(/Secondary text on/);
  const pickerBox = await picker.boundingBox();
  expect(pickerBox!.x).toBeGreaterThanOrEqual(0);
  expect(pickerBox!.x + pickerBox!.width).toBeLessThanOrEqual(1280 + 1);
  await picker.scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath("theme-contrast-picker.png") });
  const canvas = picker.locator("canvas");
  await expect(canvas).toHaveCount(1);
  expect(
    await canvas.evaluate((element) => {
      const rect = element.getBoundingClientRect();
      const hit = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
      return hit?.closest(".theme-color-popover") === element.closest(".theme-color-popover");
    }),
  ).toBe(true);
  await expect
    .poll(() =>
      canvas.evaluate((element) => {
        const view = element as HTMLCanvasElement;
        const pixels = view.getContext("2d")!.getImageData(0, 0, view.width, view.height).data;
        let white = 0;
        for (let y = 0; y < view.height; y += 1) {
          for (let x = 0; x < view.width; x += 1) {
            if (x < view.width * 0.05 || y > view.height * 0.95) continue;
            const index = (y * view.width + x) * 4;
            if (pixels[index]! > 250 && pixels[index + 1]! > 250 && pixels[index + 2]! > 250) white += 1;
          }
        }
        return white;
      }),
    )
    .toBeGreaterThan(100);
  const mutedHsv = hexToHsv("#3a414b")!;
  const [shadedPixel, passingPixel] = await canvas.evaluate(
    (element, point) => {
      const view = element as HTMLCanvasElement;
      const context = view.getContext("2d")!;
      const pixel = (x: number, y: number) => Array.from(context.getImageData(x, y, 1, 1).data.slice(0, 3));
      const x = Math.round(point.s * (view.width - 1));
      const y = Math.round((1 - point.v) * (view.height - 1));
      return [pixel(x, y), pixel(0, 0)];
    },
    { s: mutedHsv.s, v: mutedHsv.v },
  );
  // The edited colour is failing, so the square dims it to 55% of #3a414b.
  for (const [index, expected] of [
    [0, 32],
    [1, 36],
    [2, 41],
  ] as const)
    expect(Math.abs(shadedPixel[index]! - expected)).toBeLessThanOrEqual(2);
  // White sits well inside the passing region, so it stays unshaded.
  expect(passingPixel).toEqual([255, 255, 255]);
  const before = await canvas.evaluate((element) => (element as HTMLCanvasElement).toDataURL());
  await picker.getByLabel("Secondary text hue").focus();
  await page.keyboard.press("ArrowRight");
  await expect.poll(() => canvas.evaluate((element) => (element as HTMLCanvasElement).toDataURL())).not.toBe(before);
  await picker
    .getByRole("group", { name: /Secondary text saturation and brightness/ })
    .click({ position: { x: 180, y: 20 } });
  await expect(muted).not.toHaveValue("#3a414b");
  await muted.fill("#aeb9c8");
  await expect(report).toContainText("every checked pair meets the WCAG contrast guideline");
  await expect(report).not.toHaveClass(/has-warnings/);
  await page.keyboard.press("Escape");
  await expect(picker).toBeHidden();

  await page.setViewportSize({ width: 640, height: 900 });
  await page.getByRole("button", { name: "Open Secondary text color picker" }).click();
  const narrowPicker = page.locator(".theme-color-popover");
  await expect(narrowPicker).toBeVisible();
  const narrowBox = await narrowPicker.boundingBox();
  expect(narrowBox!.x).toBeGreaterThanOrEqual(0);
  expect(narrowBox!.x + narrowBox!.width).toBeLessThanOrEqual(640 + 1);
  await page.keyboard.press("Escape");
  await expect(narrowPicker).toBeHidden();
});
