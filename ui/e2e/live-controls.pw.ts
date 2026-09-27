import { test, expect, type Page } from "@playwright/test";
import { openConnectionForm, openDeviceTroubleshooting } from "./workbench";

async function playingRoute(page: Page, mode = "") {
  await page.goto(`/route-harness.html?flags=${mode}`);
  await openConnectionForm(page);
  const editor = page.locator(".workbench-connection-editor");
  for (const [source, target] of [["Microphone · out · 1ch", "Voice gain · in · 1ch"], ["Voice gain · out · 1ch", "Headphones · in · 2ch"]]) {
    await editor.getByLabel("Source output port").selectOption({ label: source });
    await editor.getByLabel("Destination input port").selectOption({ label: target });
    await editor.getByRole("button", { name: "Add connection", exact: true }).click();
  }
  await page.locator(".topbar").getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
  await openDeviceTroubleshooting(page);
  const sidebar = page.locator(".right-workbench");
  await sidebar.getByLabel("Native capture endpoint").selectOption("capture-preview");
  await sidebar.getByLabel("Native render endpoint").selectOption("render-preview");
  await page.locator(".topbar").getByRole("button", { name: "Play", exact: true }).click();
  await expect(page.locator(".audio-run-state")).toContainText("Audio running");
}

const calls = (page: Page) => page.evaluate(() => (window as unknown as { __routeFixtureCalls(): string[] }).__routeFixtureCalls());

test("live Enabled and Bypass changes retain Play and never call Stop or Start again", async ({ page }) => {
  await playingRoute(page);
  const inspector = page.locator(".main-content > .inspector");
  for (const id of ["mic", "voice", "headphones"]) {
    await page.getByTestId(`rf__node-${id}`).locator(".flow-node-title").click();
    await expect(page.getByRole("tab", { name: "Properties", exact: true })).toHaveAttribute("aria-selected", "true");
    await inspector.getByLabel("Enabled", { exact: true }).uncheck();
    await expect(page.locator(".global-action-message")).toContainText("applied to the playing audio");
    await expect(page.locator(".audio-run-state")).toContainText("Audio running");
    await inspector.getByLabel("Enabled", { exact: true }).check();
    await expect(page.locator(".global-action-message")).toContainText("applied to the playing audio");
  }
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await inspector.getByLabel("Bypass", { exact: true }).check();
  await expect(page.locator(".global-action-message")).toContainText("applied to the playing audio");
  await inspector.getByLabel("Bypass", { exact: true }).uncheck();
  await expect(page.locator(".global-action-message")).toContainText("applied to the playing audio");
  const ledger = await calls(page);
  expect(ledger.filter(call => call === "stop")).toHaveLength(0);
  expect(ledger.filter(call => call === "start")).toHaveLength(1);
  expect(ledger.filter(call => call === "commit")).toHaveLength(9);
});

for (const mode of ["reject", "restart"]) {
  test(`live flag ${mode} reports failure without stopping or claiming applied audio`, async ({ page }) => {
    await playingRoute(page, mode);
    await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
    await page.locator(".main-content > .inspector").getByLabel("Bypass", { exact: true }).click();
    await expect(page.locator(".global-action-message")).toContainText(mode === "reject" ? "Fixture commit rejected" : "could not take this change");
    await expect(page.locator(".audio-run-state")).toContainText("Audio running");
    expect((await calls(page)).filter(call => call === "stop")).toHaveLength(0);
    if (mode === "reject") await expect(page.locator(".main-content > .inspector").getByLabel("Bypass", { exact: true })).not.toBeChecked();
  });
}

test("live flags refuse unsaved topology edits without silently saving the draft", async ({ page }) => {
  await playingRoute(page);
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page.locator(".tool-card").filter({ has: page.getByText("Gain", { exact: true }) }).click();
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await page.locator(".main-content > .inspector").getByLabel("Bypass", { exact: true }).click();
  await expect(page.locator(".global-action-message")).toContainText("Save your pending route edits");
  await expect(page.locator(".main-content > .inspector").getByLabel("Bypass", { exact: true })).not.toBeChecked();
  expect((await calls(page)).filter(call => call === "commit")).toHaveLength(1);
  await expect(page.locator(".audio-run-state")).toContainText("Audio running");
});
