import { expect, test } from "./browser-test";
import { fitCanvas } from "./workbench";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: Properties flags show their state without renaming or moving their controls`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await fitCanvas(page);
    await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
    const flags = page.locator(".inspector-toggles");
    await flags.scrollIntoViewIfNeeded();
    const enabled = flags.getByRole("checkbox", { name: "Enabled", exact: true });
    const bypass = flags.getByRole("checkbox", { name: "Bypass", exact: true });
    await expect(enabled).toHaveAccessibleDescription("On");
    await expect(bypass).toHaveAccessibleDescription("Off");
    const height = await flags.evaluate((element) => element.getBoundingClientRect().height);
    await bypass.focus();
    await bypass.press("Space");
    await expect(bypass).toHaveAccessibleDescription("On");
    await enabled.focus();
    await enabled.press("Space");
    await expect(enabled).toHaveAccessibleDescription("Off");
    await expect(page.locator(".inspector")).toContainText("This tool is off.");
    await expect(flags.locator(".inspector-flag-state")).toHaveText(["Off", "On"]);
    expect(await flags.evaluate((element) => element.getBoundingClientRect().height)).toBe(height);
    await flags.screenshot({ path: `../target/ui-quality-review/round-25/flags-${theme}.png` });
  });
}
