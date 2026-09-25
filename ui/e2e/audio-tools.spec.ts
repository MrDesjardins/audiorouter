import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => { await page.goto("/harness.html"); await expect(page.getByLabel("Signal-flow graph")).toBeVisible(); });

test("every supported source, processor, and destination can be added to the live graph UI", async ({ page }) => {
  const labels = ["Test Signal", "Audio file", "Input device", "Output device", "Gain", "Mixer", "Recorder", "Mute", "Meter", "Advanced EQ", "Compressor", "Gate", "Limiter", "Delay", "Graphic EQ", "Pitch shift"];
  for (const label of labels) await page.locator(".canvas-library button").filter({ hasText: new RegExp(label, "i") }).first().click();
  for (const label of labels.map((item) => item === "Input device" ? "Physical input" : item === "Output device" ? "Physical output" : item)) await expect(page.locator(".react-flow__node").filter({ hasText: label }).first(), `node ${label}`).toBeVisible();
});

test("a multi-source processing route can be assembled without losing existing nodes", async ({ page }) => {
  for (const label of ["Test Signal", "Gain", "Advanced EQ", "Compressor", "Limiter", "Meter", "Output device", "Recorder"]) {
    await page.locator(".canvas-library button").filter({ hasText: new RegExp(label, "i") }).first().click();
  }
  await expect(page.locator(".react-flow__node")).toHaveCount(11);
  for (const existing of ["Microphone", "Voice gain", "Headphones"]) await expect(page.locator(".react-flow__node").filter({ hasText: existing }).first()).toBeAttached();
  await expect(page.getByText(/visual check only/)).toBeVisible();
});

test("the canvas remains usable at 1280 by 720", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  const graph = page.getByLabel("Signal-flow graph");
  await expect(graph).toBeVisible();
  const bounds = await graph.boundingBox();
  expect(bounds).not.toBeNull();
  expect(bounds!.width).toBeGreaterThan(900);
  await expect(page.locator(".canvas-library")).toBeVisible();
});
