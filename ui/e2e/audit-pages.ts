import type { Page } from "@playwright/test";
import { expect } from "@playwright/test";
import { setColorTheme, settingsPageNames } from "./workbench";

export const AUDIT_THEMES = ["dark", "light", "high-contrast"] as const;

/** The route harness at the size the other page audits use, in one colour theme. */
export async function openAuditHarness(page: Page, theme: string) {
  await page.setViewportSize({ width: 1500, height: 950 });
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await setColorTheme(page, theme);
}

/**
 * Visits Properties, Tools and Devices (when present), then every Settings page, calling `visit`
 * with a label on each. A new Settings page is covered without editing the tests.
 */
export async function eachAuditedPage(page: Page, visit: (where: string) => Promise<void>) {
  for (const name of ["Properties", "Tools", "Devices"]) {
    const tab = page.getByRole("tab", { name, exact: true });
    if ((await tab.count()) === 0) continue;
    await tab.click();
    await visit(name);
  }
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const list = page.getByRole("tablist", { name: "Settings pages" });
  for (const name of await settingsPageNames(page)) {
    const tab = list.getByRole("tab", { name, exact: true });
    await tab.click();
    await expect(tab).toHaveAttribute("aria-selected", "true");
    await visit(`Settings/${name}`);
  }
}
