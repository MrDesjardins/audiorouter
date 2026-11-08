import { expect, test } from "./browser-test";
import { demoSession } from "../src/fixtures";
import { fitCanvas } from "./workbench";
import type { DeviceListItem } from "@audiorouter/contracts";

const longName = "Studio monitoring output " + "with a deliberately long Windows device name ".repeat(4);
const devices: DeviceListItem[] = ["output-a", "output-b"].map((id) => ({
  id,
  name: longName,
  direction: "render",
  state: "active",
  defaultRoles: [],
  format: { sampleRateHz: 48_000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 },
  periods: { default100ns: 100_000, minimum100ns: 30_000 },
}));

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: native device picker supports keyboard selection and Escape retains focus`, async ({ page }) => {
    await page.addInitScript(
      ({ theme, session }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session });
      },
      { theme, session: demoSession },
    );
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await fitCanvas(page);
    await page.getByTestId("rf__node-headphones").click();
    const picker = page.getByLabel("Output device endpoint");
    await expect(picker).toHaveValue("");
    await picker.focus();
    await picker.press("ArrowDown");
    await expect(picker).toHaveValue("surround-preview");
    await expect(page.getByTestId("rf__node-headphones").locator(".node-save-indicator")).toHaveText("Unsaved");
    await picker.press("Alt+ArrowDown");
    await picker.press("Escape");
    await expect(picker).toBeFocused();
    await expect(picker).toHaveValue("surround-preview");
    await picker.press("Tab");
    await expect(picker).not.toBeFocused();
    expect(
      await page.evaluate(() =>
        (window as unknown as { __routeFixtureCalls: () => string[] })
          .__routeFixtureCalls()
          .filter((call) => call === "commit"),
      ),
    ).toEqual([]);
    await page.setViewportSize({ width: 640, height: 690 });
    await picker.scrollIntoViewIfNeeded();
    const bounds = (await picker.boundingBox())!;
    expect(bounds.x).toBeGreaterThanOrEqual(0);
    expect(bounds.x + bounds.width).toBeLessThanOrEqual(640);
    await page
      .getByLabel("Output device binding")
      .screenshot({ path: `../target/ui-quality-review/round-27/device-picker-${theme}.png` });
  });

  test(`${theme}: long duplicate device names retain distinct keyboard choices without overflow`, async ({ page }) => {
    await page.addInitScript(
      ({ theme, session, devices }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session, __routeFixtureDevices: devices });
      },
      { theme, session: demoSession, devices },
    );
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await fitCanvas(page);
    await page.getByTestId("rf__node-headphones").click();
    const picker = page.getByLabel("Output device endpoint");
    await expect(picker.locator("option")).toHaveCount(3);
    await expect(picker.locator('option[value="output-a"]')).toContainText(longName);
    await expect(picker.locator('option[value="output-b"]')).toContainText(longName);
    await picker.focus();
    await picker.press("ArrowDown");
    await expect(picker).toHaveValue("output-a");
    await picker.press("ArrowDown");
    await expect(picker).toHaveValue("output-b");
    for (const width of [1280, 640]) {
      await page.setViewportSize({ width, height: 720 });
      await picker.scrollIntoViewIfNeeded();
      const binding = page.getByLabel("Output device binding");
      const outer = (await binding.boundingBox())!;
      const inner = (await picker.boundingBox())!;
      expect(inner.x).toBeGreaterThanOrEqual(outer.x);
      expect(inner.x + inner.width).toBeLessThanOrEqual(outer.x + outer.width);
      expect(inner.x + inner.width).toBeLessThanOrEqual(width);
      expect(await binding.evaluate((element) => element.scrollWidth <= element.clientWidth + 1)).toBe(true);
      await binding.screenshot({ path: `../target/ui-quality-review/round-27/long-device-${width}-${theme}.png` });
    }
  });
}
