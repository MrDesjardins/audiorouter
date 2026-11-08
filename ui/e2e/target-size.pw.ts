import type { Page } from "@playwright/test";
import { expect, test } from "./browser-test";
import { setColorTheme } from "./workbench";

// Buttons criterion: WCAG 2.5.8 asks for pointer targets of at least 24 by 24 CSS px.
// Inline links inside running text are exempt under 2.5.8, as is the canvas mini map.
const smallTargets = (page: Page, where: string) =>
  page.evaluate((label) => {
    const found: string[] = [];
    for (const element of document.querySelectorAll<HTMLElement>("button, a[href], [role=button], [role=tab]")) {
      const box = element.getBoundingClientRect();
      if (box.width === 0 || box.height === 0) continue;
      if (getComputedStyle(element).visibility === "hidden") continue;
      if (element.closest("[aria-hidden=true], .react-flow__minimap")) continue;
      if (element.tagName === "A" && element.closest("p, li, dd")) continue;
      if (box.width < 24 || box.height < 24) {
        const name =
          element.getAttribute("aria-label") || element.textContent?.trim().slice(0, 30) || element.className;
        found.push(`${label}: ${name} ${Math.round(box.width)}x${Math.round(box.height)}`);
      }
    }
    return found;
  }, where);

async function openHarness(page: Page) {
  await page.setViewportSize({ width: 1500, height: 950 });
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`every visible button and link on the workbench is at least 24 px in ${theme}`, async ({ page }, testInfo) => {
    await openHarness(page);
    await setColorTheme(page, theme);
    const small: string[] = [];
    for (const name of ["Properties", "Tools", "Devices"]) {
      const tab = page.getByRole("tab", { name, exact: true });
      if ((await tab.count()) === 0) continue;
      await tab.click();
      small.push(...(await smallTargets(page, name)));
    }
    await page.getByRole("tab", { name: "Properties", exact: true }).click();
    await page
      .locator(".workbench-panel, aside")
      .last()
      .screenshot({ path: testInfo.outputPath(`properties-${theme}.png`) });
    expect([...new Set(small)]).toEqual([]);
  });
}

test("every Settings page keeps its buttons and links at least 24 px", async ({ page }) => {
  await openHarness(page);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const list = page.getByRole("tablist", { name: "Settings pages" });
  // Walk the tabs the dialog has, so a new Settings page is covered without editing this test.
  const names = (await list.getByRole("tab").allInnerTexts()).map((name) => name.trim());
  expect(names.length).toBeGreaterThan(5);
  const small: string[] = [];
  for (const name of names) {
    const tab = list.getByRole("tab", { name, exact: true });
    await tab.click();
    await expect(tab).toHaveAttribute("aria-selected", "true");
    small.push(...(await smallTargets(page, name)));
  }
  expect([...new Set(small)]).toEqual([]);
});
