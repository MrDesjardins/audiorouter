import { expect, test } from "./browser-test";
import { openStatusView } from "./workbench";
import type { DeviceListItem } from "@audiorouter/contracts";

const device: DeviceListItem = {
  id: "fixture-studio-mic",
  name: "Studio Mic",
  direction: "capture",
  state: "active",
  defaultRoles: [],
  format: { sampleRateHz: 48000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 },
  periods: { default100ns: 100000, minimum100ns: 30000 },
};
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: picker visibility states preserve the row and device identity`, async ({ page }) => {
    await page.addInitScript(
      ({ theme, device }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureDevices: [device] });
      },
      { theme, device },
    );
    await page.setViewportSize({ width: 640, height: 720 });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await openStatusView(page, "Readiness");
    const input = page.getByRole("checkbox", { name: "Show Studio Mic in device pickers", exact: true });
    await input.scrollIntoViewIfNeeded();
    await expect(input).toHaveAccessibleDescription("On");
    const row = input.locator("xpath=ancestor::li[1]");
    const before = await row.boundingBox();
    await input.focus();
    await input.press("Space");
    await expect(input).not.toBeChecked();
    await expect(input).toHaveAccessibleDescription("Off");
    await expect(row).toHaveClass(/is-hidden-device/);
    await expect(row.locator("strong")).toHaveCSS("opacity", "1");
    await expect(row.locator("small")).toHaveCSS("opacity", "1");
    expect(await row.boundingBox()).toEqual(before);
    await row.screenshot({ path: `../target/ui-quality-review/round-25/device-switch-${theme}.png` });
    await input.locator("xpath=ancestor::label[1]").locator("span").first().click();
    await expect(input).toBeChecked();
    await expect(input).toHaveAccessibleDescription("On");
  });
}
