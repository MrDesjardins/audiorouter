import { expect, test } from "@playwright/test";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`header status, Session shortcut and collapsed legend in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __routeFixtureRunning: true });
    }, theme);
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/route-harness.html");

    const status = page.locator(".audio-run-state");
    const canvas = page.getByLabel("Signal-flow graph");
    const legend = page.locator("details.canvas-legend");
    const notice = page.locator(".global-action-message");
    const title = page.locator(".header-session-name");
    await expect(status).toHaveText("Audio running");
    await expect(page.locator(".topbar").getByRole("button", { name: /mute mic|microphone muted/i })).toHaveCount(0);
    await expect(status).toHaveCSS("border-top-width", "0px");
    await expect(status).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
    await expect(notice).toHaveAttribute("aria-hidden", "true");
    await expect(page.getByText("Audio is running through this session.", { exact: true })).toHaveCount(0);
    await expect(legend).not.toHaveAttribute("open");
    await expect(page.getByRole("list", { name: "Connection line legend" })).not.toBeVisible();
    const collapsed = (await canvas.boundingBox())!;
    await legend.locator("summary").focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("list", { name: "Connection line legend" })).toBeVisible();
    await expect(page.locator(".canvas-handle-legend")).toBeVisible();
    expect((await canvas.boundingBox())!.height).toBeLessThan(collapsed.height);
    await page.screenshot({ path: info.outputPath(`${theme}-legend-open.png`) });
    await page.keyboard.press("Enter");
    await expect(legend).not.toHaveAttribute("open");
    expect((await canvas.boundingBox())!.height).toBe(collapsed.height);

    const sessionTab = page.getByRole("tab", { name: "Session", exact: true });
    await page.getByRole("tab", { name: "Timing", exact: true }).click();
    await title.click();
    await expect(sessionTab).toHaveAttribute("aria-selected", "true");
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    await title.focus();
    await page.keyboard.press("Enter");
    await expect(sessionTab).toHaveAttribute("aria-selected", "true");
    await page.screenshot({ path: info.outputPath(`${theme}-running.png`) });

    const statusBounds = (await status.boundingBox())!;
    const canvasBounds = (await canvas.boundingBox())!;
    await page.locator(".topbar").getByRole("button", { name: "Stop", exact: true }).click();
    await expect(status).toHaveText("Audio stopped");
    await expect(notice).toContainText("Session stopped.");
    expect((await status.boundingBox())!.width).toBe(statusBounds.width);
    expect((await canvas.boundingBox())!.y).toBe(canvasBounds.y);
    expect((await canvas.boundingBox())!.height).toBe(canvasBounds.height);
    await page.getByRole("button", { name: "Dismiss message", exact: true }).click();
    await expect(notice).toHaveAttribute("aria-hidden", "true");
    expect((await canvas.boundingBox())!.y).toBe(canvasBounds.y);
    await page.screenshot({ path: info.outputPath(`${theme}-stopped-feedback.png`) });
    await page.locator(".topbar").getByRole("button", { name: "Play", exact: true }).click();
    await expect(status).toHaveText("Audio running");
    await page.getByRole("button", { name: "Dismiss message", exact: true }).click();
    await expect(notice).toHaveAttribute("aria-hidden", "true");
    expect((await canvas.boundingBox())!.y).toBe(canvasBounds.y);
    expect((await canvas.boundingBox())!.height).toBe(canvasBounds.height);
  });

  test(`Play failure remains visible with a collapsed legend in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
    }, theme);
    await page.goto("/route-harness.html");
    const canvas = page.getByLabel("Signal-flow graph");
    const before = (await canvas.boundingBox())!;
    await page.locator(".topbar").getByRole("button", { name: "Play", exact: true }).click();
    const notice = page.locator(".global-action-message");
    await expect(notice).toContainText("No audio started.");
    await expect(notice).toBeVisible();
    expect((await canvas.boundingBox())!.y).toBe(before.y);
    await page.screenshot({ path: info.outputPath(`${theme}-play-failure.png`) });
  });
}
