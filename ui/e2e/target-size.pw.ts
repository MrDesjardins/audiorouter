import { expect, test } from "./browser-test";
import { setColorTheme } from "./workbench";

// Buttons criterion: WCAG 2.5.8 asks for pointer targets of at least 24 by 24 CSS px.
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`every visible button and link on the workbench is at least 24 px in ${theme}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: 1500, height: 950 });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await setColorTheme(page, theme);
    const tabs = ["Properties", "Tools", "Devices"];
    const small: string[] = [];
    for (const name of tabs) {
      const tab = page.getByRole("tab", { name, exact: true });
      if ((await tab.count()) === 0) continue;
      await tab.click();
      small.push(
        ...(await page.evaluate((tabName) => {
          const found: string[] = [];
          for (const element of document.querySelectorAll<HTMLElement>("button, a[href], [role=button], [role=tab]")) {
            const box = element.getBoundingClientRect();
            if (box.width === 0 || box.height === 0) continue;
            const style = getComputedStyle(element);
            if (style.visibility === "hidden" || element.closest("[aria-hidden=true], .react-flow__minimap")) continue;
            // Inline links inside running text are exempt under 2.5.8.
            if (element.tagName === "A" && element.closest("p, li, dd")) continue;
            if (box.width < 24 || box.height < 24) {
              const label =
                element.getAttribute("aria-label") || element.textContent?.trim().slice(0, 30) || element.className;
              found.push(`${tabName}: ${label} ${Math.round(box.width)}x${Math.round(box.height)}`);
            }
          }
          return found;
        }, name)),
      );
    }
    await page.getByRole("tab", { name: "Properties", exact: true }).click();
    await page
      .locator(".workbench-panel, aside")
      .last()
      .screenshot({ path: testInfo.outputPath(`properties-${theme}.png`) });
    expect([...new Set(small)]).toEqual([]);
  });
}
