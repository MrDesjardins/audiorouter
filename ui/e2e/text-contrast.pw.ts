import { DEFAULT_CUSTOM_THEME } from "../src/customTheme";
import { expect, test } from "./browser-test";
import { backToRoute, openSettings, setColorTheme } from "./workbench";
import { lowContrastText } from "./ui-audits";

// "custom" is the starting custom palette, imported the way a user would load a theme file.
for (const theme of ["dark", "light", "high-contrast", "custom"]) {
  test(`workbench and Settings text meets AA contrast in ${theme}`, async ({ page }) => {
    await page.setViewportSize({ width: 1500, height: 950 });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    if (theme === "custom") {
      await openSettings(page, "Appearance");
      await page.getByLabel("Import theme file").setInputFiles({
        name: "custom.json",
        mimeType: "application/json",
        buffer: Buffer.from(JSON.stringify(DEFAULT_CUSTOM_THEME)),
      });
      await backToRoute(page);
      await expect(page.locator(".theme-custom").first()).toBeAttached();
    } else await setColorTheme(page, theme);
    const low: string[] = [];
    for (const name of ["Properties", "Tools", "Devices"]) {
      const tab = page.getByRole("tab", { name, exact: true });
      if ((await tab.count()) === 0) continue;
      await tab.click();
      low.push(...(await lowContrastText(page, name)));
    }
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    const list = page.getByRole("tablist", { name: "Settings pages" });
    for (const name of (await list.getByRole("tab").allInnerTexts()).map((value) => value.trim())) {
      const tab = list.getByRole("tab", { name, exact: true });
      await tab.click();
      await expect(tab).toHaveAttribute("aria-selected", "true");
      low.push(...(await lowContrastText(page, `Settings/${name}`)));
    }
    expect([...new Set(low)]).toEqual([]);
  });
}

test("the contrast check reports faint text on solid and gradient fills", async ({ page }) => {
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.evaluate(() => {
    const solid = document.createElement("p");
    solid.textContent = "Faint solid";
    solid.style.cssText = "position:fixed;top:0;left:0;z-index:9999;color:#777;background:#888;font-size:14px";
    const gradient = document.createElement("p");
    gradient.textContent = "Faint gradient";
    gradient.style.cssText =
      "position:fixed;top:40px;left:0;z-index:9999;color:#fff;background:linear-gradient(#000,#eee);font-size:14px";
    document.body.append(solid, gradient);
  });
  const low = await lowContrastText(page, "probe");
  expect(low.some((entry) => entry.includes("Faint solid"))).toBe(true);
  expect(low.some((entry) => entry.includes("Faint gradient"))).toBe(true);
});
