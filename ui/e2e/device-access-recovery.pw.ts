import { expect, test } from "./browser-test";
import { openSettings } from "./workbench";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: Device access recovers failed reads and an uncertain write without moving`, async ({ page }) => {
    await page.addInitScript((value) => {
      localStorage.setItem("audiorouter.ui.theme", value);
      Object.assign(window, { __routeFixtureDeviceAccessReadError: true });
    }, theme);
    await page.setViewportSize({ width: 640, height: 690 });
    await page.goto("/route-harness.html");
    await openSettings(page, "Device access");
    const access = page.getByRole("group", { name: "Audio device access", exact: true });
    const state = access.getByTestId("device-access-state");
    const check = access.getByRole("button", { name: "Check again", exact: true });
    await expect(access).toContainText("Could not read audio device access");
    await expect(state).toHaveText("Unavailable");
    await expect(access.getByRole("button", { name: "Allow access", exact: true })).toBeDisabled();
    const height = await access.evaluate((element) => element.getBoundingClientRect().height);
    await page.evaluate(() => Object.assign(window, { __routeFixtureDeviceAccessReadError: false }));
    await check.click();
    await expect(state).toHaveText("Allowed");
    expect(await access.evaluate((element) => element.getBoundingClientRect().height)).toBe(height);
    await page.evaluate(() => Object.assign(window, { __routeFixtureDeviceAccessWriteError: "lost" }));
    await access.getByRole("button", { name: "Withdraw access", exact: true }).click();
    await expect(access).toContainText("Could not confirm the change");
    await expect(state).toHaveText("Last known: Allowed");
    await expect(access.getByRole("button", { name: "Withdraw access", exact: true })).toBeDisabled();
    expect(await access.evaluate((element) => element.getBoundingClientRect().height)).toBe(height);
    await access.screenshot({ path: `../target/ui-quality-review/round-25/access-error-${theme}.png` });
    await page.evaluate(() => Object.assign(window, { __routeFixtureDeviceAccessWriteError: undefined }));
    await check.click();
    await expect(state).toHaveText("Ask first");
    const allow = access.getByRole("button", { name: "Allow access", exact: true });
    await expect(allow).toBeEnabled();
    await allow.focus();
    await allow.press("Enter");
    await expect(state).toHaveText("Allowed");
    await expect(access).toContainText("Audio device access allowed.");
    expect(await access.evaluate((element) => element.getBoundingClientRect().height)).toBe(height);
    await access.screenshot({ path: `../target/ui-quality-review/round-25/access-recovered-${theme}.png` });
  });
}
