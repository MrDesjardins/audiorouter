import { expect, test } from "./browser-test";
import { openSettings } from "./workbench";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: a delayed scan disables competing actions and recovers after rejection`, async ({ page }) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, {
        __routeFixturePluginScanDelayMs: 1200,
        __routeFixturePluginScanError: "Fixture scan unavailable",
      });
    }, theme);
    await page.goto("/route-harness.html");
    await openSettings(page, "Plug-ins");
    const panel = page.getByRole("region", { name: "Plugin scan", exact: true });
    const directory = panel.getByRole("textbox", { name: "Absolute plugin directory", exact: true });
    await directory.fill("C:\\Plugins");
    await panel.getByRole("button", { name: "Scan directory", exact: true }).click();
    await expect(panel).toHaveAttribute("aria-busy", "true");
    for (const name of ["Scan directory", "Load last scan", "Retry scan", "Inspect path"])
      await expect(panel.getByRole("button", { name, exact: true })).toBeDisabled();
    await expect(panel).toContainText("Scanning selected directory");
    const footer = panel.locator("p.muted").last();
    const before = await footer.boundingBox();
    await expect(panel).toContainText("Fixture scan unavailable");
    await expect(panel).toHaveAttribute("aria-busy", "false");
    await expect(directory).toHaveValue("C:\\Plugins");
    expect(await footer.boundingBox()).toEqual(before);
    await page.evaluate(() => Object.assign(window, { __routeFixturePluginScanError: undefined }));
    await panel.getByRole("button", { name: "Retry scan", exact: true }).click();
    await expect(panel).toContainText("Plugin scan retry completed");
    await expect(panel.getByRole("list", { name: "Plugin scan results" })).toContainText("No VST3");
    const calls = await page.evaluate(() =>
      (window as unknown as { __routeFixtureCalls: () => string[] }).__routeFixtureCalls(),
    );
    expect(calls.filter((value) => value === "plugin-scan")).toHaveLength(1);
    expect(calls.filter((value) => value === "plugin-retry")).toHaveLength(1);
    await panel.screenshot({ path: `../target/ui-quality-review/round-26/scan-${theme}.png` });
  });
}
