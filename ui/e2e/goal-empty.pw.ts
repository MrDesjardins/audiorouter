import { expect } from "@playwright/test";
import { test } from "./real-backend";

// First-time-user walk-through (2026-11-08): choosing "Start empty" crashed the window
// ("Cannot read properties of undefined (reading 'id')"), because the app assumed every
// route has a selected tool.
test("starting from the empty goal opens an empty canvas, then a tool can be added", async ({ page }, testInfo) => {
  const consoleErrors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });
  await page.goto("/backend-harness.html");
  await expect(page.getByTestId("rf__node-voice")).toBeVisible();
  await page.keyboard.press("Control+K");
  await page.getByRole("combobox").fill("start from");
  await page.keyboard.press("Enter");
  await page.getByRole("button", { name: "Use this: Start empty" }).click();

  await expect(page.locator(".react-flow__node")).toHaveCount(0);
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
  await expect(page.getByRole("heading", { name: "No tool selected" })).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath("empty-route.png") });
  await expect(page.getByText("AudioRouter's window stopped working")).toHaveCount(0);

  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page
    .locator(".tool-card")
    .filter({ has: page.getByText("Gain", { exact: true }) })
    .click();
  await expect(page.locator(".react-flow__node")).toHaveCount(1);
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
  await expect(page.getByRole("heading", { name: "No tool selected" })).toHaveCount(0);
  await expect(page.locator("#inspector-heading")).toBeVisible();
  expect(consoleErrors.filter((text) => !text.includes("excluded by the offline E2E fixture"))).toEqual([]);
});
