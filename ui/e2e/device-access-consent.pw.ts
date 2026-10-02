import { test, expect } from "@playwright/test";
import { openConnectionForm, openDeviceTroubleshooting } from "./workbench";

// A fresh install (release 0.0.1 regression): the first Play must ask once
// to open audio devices, then play, instead of a "Permission denied" banner.
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`a fresh install asks once to use audio devices, then plays, in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __routeFixtureFreshInstall: true });
    }, theme);
    await page.goto("/route-harness.html");
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

    const dialog = page.getByRole("dialog", { name: "Allow AudioRouter to use your audio devices?" });
    await expect(dialog).toBeVisible();
    await expect(page.locator(".global-action-message")).not.toContainText("Permission denied");
    await dialog.screenshot({ path: testInfo.outputPath(`device-access-dialog-${theme}.png`) });
    await dialog.getByRole("button", { name: "Allow and play" }).click();
    await expect(dialog).toHaveCount(0);
    await expect(page.locator(".audio-run-state")).toContainText("Audio running");

    // Asked once: Stop and Play again plays without the question.
    await page.locator(".topbar").getByRole("button", { name: "Stop", exact: true }).click();
    await expect(page.locator(".audio-run-state")).not.toContainText("Audio running");
    await page.locator(".topbar").getByRole("button", { name: "Play", exact: true }).click();
    await expect(page.locator(".audio-run-state")).toContainText("Audio running");
    await expect(dialog).toHaveCount(0);

    // Setup shows the choice and can withdraw it.
    await page.getByRole("tab", { name: "Setup", exact: true }).click();
    const setting = page.getByRole("group", { name: "Audio device access" });
    await expect(setting).toContainText("may open the audio devices");
    await expect(setting.getByRole("button", { name: "Withdraw access" })).toBeVisible();
    const calls = await page.evaluate(() => (window as unknown as { __routeFixtureCalls(): string[] }).__routeFixtureCalls());
    expect(calls.filter((call) => call.startsWith("device-access"))).toEqual(["device-access:true"]);
  });
}
