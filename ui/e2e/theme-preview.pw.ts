import { expect, test } from "./browser-test";
import { openSettings } from "./workbench";

// Consistency criterion: the theme editor preview paints the chosen palette,
// so a colour can be judged in the workspace before the theme is used.
test("the theme preview follows edited colours and shows flowing and inactive connections", async ({
  page,
}, testInfo) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto("/route-harness.html");
  await openSettings(page, "Appearance");
  await page.getByRole("button", { name: "Create a new theme" }).click();

  const preview = page.locator(".theme-preview");
  const previewHeading = page.locator(".theme-preview-block .theme-preview-heading");
  await expect(previewHeading).toHaveText("Preview");
  await expect(preview).toBeVisible();
  const baseLabel = page.locator(".theme-editor label").filter({ hasText: "Base appearance" });
  const typography = (locator: typeof previewHeading) =>
    locator.evaluate((element) => {
      const style = getComputedStyle(element);
      return {
        fontSize: style.fontSize,
        color: style.color,
        lineHeight: style.lineHeight,
        fontWeight: style.fontWeight,
        fontFamily: style.fontFamily,
      };
    });
  expect(await typography(previewHeading)).toEqual(await typography(baseLabel));
  const headingBox = await previewHeading.boundingBox();
  const previewBox = await preview.boundingBox();
  expect(headingBox!.y + headingBox!.height).toBeLessThanOrEqual(previewBox!.y + 1);
  await expect(preview).toHaveAttribute("role", "img");
  await expect(preview).toHaveClass(/theme-dark/);
  await expect(preview.locator(".theme-preview-line.is-active")).toHaveCount(2);
  await expect(preview.locator(".theme-preview-line.is-silent")).toHaveCount(1);
  await expect(preview.locator(".theme-preview-line.is-paused")).toHaveCount(1);
  await expect(preview).toContainText("Flowing");
  await expect(preview).toContainText("Silent");
  await expect(preview).toContainText("Paused");

  await page.getByText("Edit palette colors").click();
  expect(await typography(page.locator(".theme-color-name").first())).toEqual(await typography(baseLabel));
  await page.getByRole("textbox", { name: "Selection accent color", exact: true }).fill("#ff00aa");
  await page.getByRole("textbox", { name: "Success color", exact: true }).fill("#00ff00");
  await page.getByRole("textbox", { name: "Primary actions color", exact: true }).fill("#ffee00");
  await page.getByRole("textbox", { name: "Sending ports color", exact: true }).fill("#1122ff");
  await page.getByRole("textbox", { name: "Receiving ports color", exact: true }).fill("#ff2200");

  await expect(preview.locator(".theme-preview-gain .theme-preview-kind")).toHaveCSS("color", "rgb(255, 0, 170)");
  await expect(preview.locator(".theme-preview-status-dot")).toHaveCSS("background-color", "rgb(0, 255, 0)");
  await expect(preview.locator(".theme-preview-speakers")).toHaveCSS("border-top-color", "rgb(255, 238, 0)");
  await expect(preview.locator(".theme-preview-gain .theme-preview-port.is-input")).toHaveCSS(
    "background-color",
    "rgb(17, 34, 255)",
  );
  await expect(preview.locator(".theme-preview-gain .theme-preview-port.is-output")).toHaveCSS(
    "background-color",
    "rgb(255, 34, 0)",
  );
  const activeStroke = await preview.locator(".theme-preview-line.is-active").first().getAttribute("stroke");
  expect(activeStroke).toMatch(/^url\(#theme-preview-flow-/);
  expect(await preview.evaluate((element) => element.scrollWidth <= element.clientWidth + 1)).toBe(true);

  const canvas = preview.locator(".theme-preview-canvas");
  const canvasBox = await canvas.boundingBox();
  const nodeBoxes = await preview.locator(".theme-preview-node").evaluateAll((nodes) =>
    nodes.map((node) => {
      const rect = node.getBoundingClientRect();
      return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom };
    }),
  );
  expect(nodeBoxes).toHaveLength(5);
  for (const box of nodeBoxes) {
    expect(box.left).toBeGreaterThanOrEqual(canvasBox!.x - 1);
    expect(box.top).toBeGreaterThanOrEqual(canvasBox!.y - 1);
    expect(box.right).toBeLessThanOrEqual(canvasBox!.x + canvasBox!.width + 1);
    expect(box.bottom).toBeLessThanOrEqual(canvasBox!.y + canvasBox!.height + 1);
  }
  for (let first = 0; first < nodeBoxes.length; first += 1) {
    for (let second = first + 1; second < nodeBoxes.length; second += 1) {
      const a = nodeBoxes[first]!;
      const b = nodeBoxes[second]!;
      const horizontal = Math.min(a.right, b.right) - Math.max(a.left, b.left);
      const vertical = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
      expect(horizontal > 1 && vertical > 1).toBe(false);
    }
  }

  await preview.scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath("theme-preview-dark.png") });

  await page.getByLabel("Base appearance").selectOption("light");
  await expect(preview).toHaveClass(/theme-light/);
  await preview.scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath("theme-preview-light.png") });

  await page.setViewportSize({ width: 640, height: 900 });
  await expect(preview).toBeVisible();
  expect(await preview.evaluate((element) => element.scrollWidth <= element.clientWidth + 1)).toBe(true);
  await preview.scrollIntoViewIfNeeded();
  await page.screenshot({ path: testInfo.outputPath("theme-preview-narrow.png") });
});
