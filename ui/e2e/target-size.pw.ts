import type { Page } from "@playwright/test";
import { expect, test } from "./browser-test";
import { setColorTheme, settingsPageNames } from "./workbench";
import { smallTargets } from "./ui-audits";

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
  const names = await settingsPageNames(page);
  const small: string[] = [];
  for (const name of names) {
    const tab = list.getByRole("tab", { name, exact: true });
    await tab.click();
    await expect(tab).toHaveAttribute("aria-selected", "true");
    small.push(...(await smallTargets(page, name)));
  }
  expect([...new Set(small)]).toEqual([]);
});

test("Settings audits wait for navigation before enumerating pages", async ({ page }) => {
  await page.setContent(`<button onclick="setTimeout(() => {
    const pages = document.createElement('nav');
    pages.setAttribute('role', 'tablist');
    pages.setAttribute('aria-label', 'Settings pages');
    pages.innerHTML = ['One','Two','Three','Four','Five','Six'].map(name =>
      '<button role=tab>' + name + '</button>').join('');
    document.body.append(pages);
  }, 200)">Settings</button>`);
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  expect(await settingsPageNames(page)).toEqual(["One", "Two", "Three", "Four", "Five", "Six"]);
});
