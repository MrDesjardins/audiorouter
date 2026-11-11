import { expect, test } from "./browser-test";
import { setColorTheme, settingsPageNames } from "./workbench";
import { clippedText } from "./ui-audits";

const layouts = [
  ...["dark", "light", "high-contrast"].map((theme) => ({ theme, width: 1500, height: 950 })),
  { theme: "dark", width: 1024, height: 700 },
];
for (const { theme, width, height } of layouts) {
  test(`no workbench or Settings text is cut off unreadably in ${theme} at ${width}x${height}`, async ({ page }) => {
    await page.setViewportSize({ width, height });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await setColorTheme(page, theme);
    const found: string[] = [];
    for (const name of ["Properties", "Tools", "Devices"]) {
      const tab = page.getByRole("tab", { name, exact: true });
      if ((await tab.count()) === 0) continue;
      await tab.click();
      found.push(...(await clippedText(page, name)));
    }
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    const list = page.getByRole("tablist", { name: "Settings pages" });
    for (const name of await settingsPageNames(page)) {
      const tab = list.getByRole("tab", { name, exact: true });
      await tab.click();
      await expect(tab).toHaveAttribute("aria-selected", "true");
      found.push(...(await clippedText(page, `Settings/${name}`)));
    }
    expect([...new Set(found)]).toEqual([]);
  });
}
