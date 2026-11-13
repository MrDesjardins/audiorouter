import { expect, test } from "./browser-test";
import { openDeviceTroubleshooting, stereoDemoSession } from "./workbench";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`copies a complete plugin error without moving the route in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ theme, session }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, {
          __routeFixtureSession: session,
          __routeFixtureStartError: {
            code: -32602,
            message:
              "Plugin worker executable audiorouter-plugin-worker.exe was not found. Checked C:\\AudioRouter\\resources. Rebuild the complete package.",
          },
        });
        Object.defineProperty(navigator, "clipboard", {
          configurable: true,
          value: {
            writeText: async (text: string) => {
              Object.assign(window, { copiedError: text });
            },
          },
        });
      },
      { theme, session: stereoDemoSession },
    );
    await page.goto("/route-harness.html");
    await openDeviceTroubleshooting(page);
    await page.getByLabel("Native capture endpoint").selectOption("capture-preview");
    await page.getByLabel("Native render endpoint").selectOption("render-preview");
    await page.locator(".topbar").getByRole("button", { name: "Play", exact: true }).click();
    const bar = page.locator(".global-action-message");
    const text = await bar.locator(".global-action-message-text").innerText();
    await expect(bar.getByRole("button", { name: "Copy error" })).toBeVisible();
    const before = await bar.boundingBox();
    await bar.getByRole("button", { name: "Copy error" }).click();
    await expect(bar.getByRole("button", { name: "Copied", exact: true })).toBeVisible();
    expect(await page.evaluate(() => (window as unknown as { copiedError: string }).copiedError)).toBe(text);
    expect(await bar.boundingBox()).toEqual(before);
    await bar.screenshot({ path: testInfo.outputPath(`copy-error-${theme}.png`) });
  });
}
