import { expect, test } from "./browser-test";
import { appendDraftConnection } from "../src/draft";
import { stereoDemoSession } from "./workbench";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: a slow rejected save keeps edits, blocks duplicate saves and can be retried`, async ({ page }) => {
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
          __routeFixtureCommitDelayMs: 1200,
          __routeFixtureCommitError: "Fixture save refused; retry is available",
        });
      },
      { session, theme },
    );
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await page.getByRole("tab", { name: "Session", exact: true }).click();
    await page.getByRole("button", { name: "Rename", exact: true }).click();
    const name = page.getByRole("textbox", { name: "Session name", exact: true });
    await name.fill("Retained slow save");
    const canvas = page.locator(".session-flow-canvas");
    const before = await canvas.boundingBox();
    await page.locator(".topbar").getByRole("button", { name: "Save", exact: true }).click();
    const saving = page.locator(".topbar").getByRole("button", { name: "Saving…", exact: true });
    await expect(saving).toBeDisabled();
    await expect(saving).toHaveAttribute("aria-busy", "true");
    await page.getByRole("tab", { name: "Session", exact: true }).focus();
    await page.keyboard.press("Control+s");
    expect(await canvas.boundingBox()).toEqual(before);
    await expect(page.locator(".global-action-message")).toContainText("Fixture save refused");
    await expect(name).toHaveValue("Retained slow save");
    const save = page.locator(".topbar").getByRole("button", { name: "Save", exact: true });
    await expect(save).toBeEnabled();
    expect(await canvas.boundingBox()).toEqual(before);
    expect(
      await page.evaluate(
        () =>
          (window as unknown as { __routeFixtureCalls: () => string[] })
            .__routeFixtureCalls()
            .filter((value) => value === "commit").length,
      ),
    ).toBe(1);
    await page.evaluate(() => Object.assign(window, { __routeFixtureCommitError: undefined }));
    await save.click();
    await expect(page.locator(".global-action-message")).toContainText("Route saved");
    await expect(save).toBeDisabled();
    await expect(page.locator(".header-session-name")).toHaveText("Retained slow save");
  });
}
