import type { Page } from "@playwright/test";

async function openAdvancedGroup(page: Page, selector: string) {
  await page.getByRole("tab", { name: "Advanced", exact: true }).click();
  const group = page.locator(selector);
  if (!(await group.evaluate((element) => (element as HTMLDetailsElement).open)))
    await group.locator("> summary").click();
}

/** Advanced → Connect nodes without dragging (the keyboard connection form). */
export const openConnectionForm = (page: Page) => openAdvancedGroup(page, "details.connection-form-group");

/** Advanced → Troubleshooting: manual device binding. */
export const openDeviceTroubleshooting = (page: Page) => openAdvancedGroup(page, "details.device-troubleshooting");

/** Change appearance in UI Options, then return to the current tab. */
export async function setColorTheme(page: Page, theme: string) {
  const selected = await page.getByRole("tab", { selected: true }).innerText();
  await page.getByRole("tab", { name: "UI Options", exact: true }).click();
  await page.getByRole("combobox", { name: "Color theme" }).selectOption(theme);
  if (selected !== "UI Options") await page.getByRole("tab", { name: selected, exact: true }).click();
}
export const openAppSettings = (page: Page) =>
  openAdvancedGroup(page, "details.advanced-group:has(> summary:text-is('Device access and version checks'))");
