import { expect, test } from "./browser-test";
import { stereoDemoSession } from "./workbench";
import { appendDraftConnection } from "../src/draft";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: delayed Stop and rejected Play keep the route usable and prevent repeats`, async ({ page }) => {
    const session = appendDraftConnection(
      appendDraftConnection(stereoDemoSession, "mic", "out", "voice", "in"),
      "voice",
      "out",
      "headphones",
      "in",
    );
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, {
          __routeFixtureSession: session,
          __routeFixtureRunning: true,
          __routeFixtureLifecycleDelayMs: 1200,
        });
      },
      { session, theme },
    );
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const canvas = page.locator(".session-flow-canvas");
    const before = await canvas.boundingBox();
    const stop = page.locator(".topbar").getByRole("button", { name: "Stop", exact: true });
    await stop.click();
    await expect(stop).toBeDisabled();
    await expect(stop).toHaveAttribute("aria-busy", "true");
    await page.keyboard.press("Control+Shift+S");
    const play = page.locator(".topbar").getByRole("button", { name: "Play", exact: true });
    await expect(play).toBeEnabled();
    expect(await canvas.boundingBox()).toEqual(before);
    await page.evaluate(() =>
      Object.assign(window, { __routeFixtureStartError: { code: -32000, message: "Fixture output unavailable" } }),
    );
    await play.click();
    await expect(play).toBeDisabled();
    await expect(play).toHaveAttribute("aria-busy", "true");
    await page.keyboard.press("Control+Shift+S");
    await expect(page.locator(".global-action-message")).toContainText("Fixture output unavailable");
    await expect(play).toBeEnabled();
    expect(await canvas.boundingBox()).toEqual(before);
    const calls = await page.evaluate(() =>
      (window as unknown as { __routeFixtureCalls: () => string[] }).__routeFixtureCalls(),
    );
    expect(calls.filter((call) => call === "start")).toHaveLength(1);
    expect(calls.filter((call) => call === "stop")).toHaveLength(1);
    await page.evaluate(() => Object.assign(window, { __routeFixtureStartError: undefined }));
    await play.click();
    await expect(stop).toBeEnabled();
    await expect(page.locator(".audio-run-state")).toContainText("Audio running");
  });
}
