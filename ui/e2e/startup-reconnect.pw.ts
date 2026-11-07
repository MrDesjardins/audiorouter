import { expect, test } from "./browser-test";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: failed startup reconnect stays busy until the saved route arrives`, async ({ page }) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __routeFixtureSnapshotError: "Fixture saved route unavailable" });
    }, theme);
    await page.goto("/route-harness.html");
    const loading = page.getByRole("main", { name: "Loading saved session" });
    await expect(loading).toContainText("Fixture saved route unavailable");
    await expect(page.locator(".react-flow__node")).toHaveCount(0);
    await page.evaluate(() =>
      Object.assign(window, { __routeFixtureSnapshotError: undefined, __routeFixtureDelayMs: 1200 }),
    );
    const reconnect = page.getByRole("button", { name: "Reconnect", exact: true });
    await reconnect.click();
    await expect(reconnect).toBeDisabled();
    await expect(reconnect).toHaveAttribute("aria-busy", "true");
    await expect(loading).toContainText("Reconnecting");
    await expect(page.getByRole("button", { name: "Play", exact: true })).toHaveCount(0);
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await expect(page.getByTestId("rf__node-mic")).toBeVisible();
    await expect(loading).toHaveCount(0);
  });
}
