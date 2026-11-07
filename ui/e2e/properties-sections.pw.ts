import type { Page } from "@playwright/test";
import { writeFileSync } from "node:fs";
import { expect, test } from "./real-backend";

async function addTool(page: Page, name: string) {
  await page.goto("/backend-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page
    .locator(".tool-card")
    .filter({ has: page.getByText(name, { exact: true }) })
    .click();
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
  await expect(page.locator(".main-content > .inspector")).toBeVisible();
}

const section = (page: Page, title: string) => page.locator(".property-section").filter({ hasText: title });

test("Compressor keeps its main controls beside the curve, and its timing values readable while closed", async ({
  page,
}) => {
  await addTool(page, "Compressor");
  const inspector = page.locator(".main-content > .inspector");
  const main = inspector.locator(".dynamics-main-controls");
  await expect(main.getByRole("slider")).toHaveCount(3);
  const timing = section(page, "Timing and knee");
  await expect(timing.locator(".property-section-summary")).toHaveText("10 ms · 150 ms · 6 dB");
  await expect(timing.locator(".property-section-body")).toHaveCount(0);
  await timing.locator("summary").click();
  await expect(timing.getByRole("slider")).toHaveCount(3);
  // Actions come last, on one row.
  const buttons = inspector.locator(".inspector-tool-actions button");
  await expect(buttons).toHaveText(["Reset", "Duplicate", "Remove"]);
  const tops = await buttons.evaluateAll((all) => all.map((button) => button.getBoundingClientRect().top));
  expect(new Set(tops).size).toBe(1);
});

test("an open section is remembered per tool type, on this PC", async ({ page }) => {
  await addTool(page, "Compressor");
  await section(page, "Timing and knee").locator("summary").click();
  await expect(section(page, "Timing and knee").locator(".property-section-body")).toBeVisible();
  await addTool(page, "Compressor");
  await expect(section(page, "Timing and knee").locator(".property-section-body")).toBeVisible();
  await addTool(page, "Gate");
  await expect(section(page, "Timing and closing").locator(".property-section-body")).toHaveCount(0);
});

test("privacy mute stays in plain sight on every tool (UI-10)", async ({ page }) => {
  for (const name of ["Compressor", "Gain", "Input device"]) {
    await addTool(page, name);
    await expect(page.getByRole("button", { name: /Turn (on|off) privacy mute/ })).toBeVisible();
  }
});

test("Input device: spatial audio is closed and shows its value", async ({ page }) => {
  await addTool(page, "Input device");
  const spatial = section(page, "Spatial audio");
  await expect(spatial.locator(".property-section-summary")).toHaveText("Off");
  await expect(spatial.locator(".property-section-body")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Apps using this device" })).toBeVisible();
});

test("a closed section keeps no setting out of reach", async ({ page }) => {
  await addTool(page, "Gate");
  await section(page, "Timing and closing").locator("summary").click();
  await expect(section(page, "Timing and closing").getByRole("slider")).toHaveCount(5);
});

// Writes the panel heights for the plan. It decides nothing by itself.
test("measure every tool's Properties height", async ({ page }, info) => {
  test.setTimeout(240_000);
  await page.goto("/backend-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  const names = await page.locator(".tool-card:not(:disabled) strong").allTextContents();
  const heights: Record<string, number> = {};
  for (const name of names.filter((item) => item !== "Group" && item !== "Application source")) {
    await addTool(page, name);
    heights[name] = await page.locator(".main-content > .inspector").evaluate((panel) => panel.scrollHeight);
  }
  writeFileSync(info.outputPath("heights.json"), JSON.stringify(heights, null, 2));
  expect(Object.keys(heights).length).toBeGreaterThan(25);
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`Properties sections screenshots in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await addTool(page, "Compressor");
    await page.screenshot({ path: info.outputPath(`${theme}-compressor.png`) });
    await addTool(page, "Input device");
    await page.screenshot({ path: info.outputPath(`${theme}-input.png`) });
  });
}
