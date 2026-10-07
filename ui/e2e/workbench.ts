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
