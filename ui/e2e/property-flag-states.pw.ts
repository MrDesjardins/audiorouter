import { expect, test } from "./browser-test";
import { fitCanvas, openSettings } from "./workbench";

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
    await expect(flags.locator(".inspector-flag-state > span:not([aria-hidden])")).toHaveText(["Off", "On"]);
    expect(await flags.evaluate((element) => element.getBoundingClientRect().height)).toBe(height);
    await flags.screenshot({ path: `../target/ui-quality-review/round-25/flags-${theme}.png` });
  });

  test(`${theme}: French flag states fit the Properties panel without moving the switches`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width: 960, height: 800 });
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/route-harness.html");
    await openSettings(page, "Appearance");
    await page.getByRole("combobox", { name: "Language", exact: true }).selectOption("fr");
    await page.getByRole("button", { name: "Retour à la route", exact: true }).click();
    await fitCanvas(page);
    await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
    const flags = page.locator(".inspector-toggles");
    await flags.scrollIntoViewIfNeeded();
    const bypass = flags.getByRole("checkbox", { name: "Contourner", exact: true });
    await expect(bypass).toHaveAccessibleDescription("Désactivé");
    const geometry = () =>
      flags.evaluate((element) => {
        const row = element.getBoundingClientRect();
        const switches = [...element.querySelectorAll("input")].map((input) => {
          const { x, y, width, height } = input.getBoundingClientRect();
          return { x: x - row.x, y: y - row.y, width, height };
        });
        return { height: row.height, switches, overflow: element.scrollWidth - element.clientWidth };
      });
    await bypass.focus();
    const before = await geometry();
    expect(before.overflow).toBe(0);
    await flags.screenshot({ path: testInfo.outputPath("french-flags.png") });
    await bypass.press("Space");
    await expect(bypass).toHaveAccessibleDescription("Activé");
    expect(await geometry()).toEqual(before);
  });
}
