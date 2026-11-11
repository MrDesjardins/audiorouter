import type { Locator, Page } from "@playwright/test";
import { expect } from "@playwright/test";
import type { Session } from "@audiorouter/contracts";
import { demoSession } from "../src/fixtures";

/**
 * The demo route with every port stereo. The single-endpoint worker plays a
 * chain of one channel width only; the mono demo microphone into stereo
 * headphones plays on the multi-path worker, which needs each device chosen
 * on its node. Tests of the single-endpoint flow use this route.
 */
export const stereoDemoSession: Session = {
  ...demoSession,
  nodes: demoSession.nodes.map((node) => ({
    ...node,
    ports: node.ports.map((port) => ({ ...port, channels: 2 })),
  })),
};

/** The gear opens Settings; the pages are tabs in its side list. */
export async function openSettings(page: Page, pageName: string) {
  const pages = page.getByRole("tablist", { name: "Settings pages" });
  if (!(await pages.isVisible())) await page.getByRole("button", { name: "Settings", exact: true }).click();
  const tab = pages.getByRole("tab", { name: pageName, exact: true });
  await tab.click();
  // Lazy panels can suspend while the preceding page remains committed.
  await expect(tab).toHaveAttribute("aria-selected", "true");
}

/** Settings opens in a transition; enumeration itself does not auto-wait. */
export async function settingsPageNames(page: Page) {
  const pages = page.getByRole("tablist", { name: "Settings pages" });
  await expect(pages).toBeVisible();
  const names = (await pages.getByRole("tab").allInnerTexts()).map((name) => name.trim());
  expect(names.length).toBeGreaterThan(5);
  return names;
}

/** Leave Settings for the tab that was open before it. */
export async function backToRoute(page: Page) {
  await page.getByRole("button", { name: "Back to the route" }).click();
  await page.getByRole("tablist", { name: "Right sidebar" }).waitFor();
}

const leaveSettings = async (page: Page) => {
  if (await page.getByRole("tablist", { name: "Settings pages" }).isVisible()) await backToRoute(page);
};

/** Status has Readiness, Apps and Timing views. */
export async function openStatusView(page: Page, view: "Readiness" | "Apps" | "Timing") {
  await page.getByRole("tablist", { name: "Right sidebar" }).getByRole("tab", { name: "Status" }).click();
  const tab = page.getByRole("tablist", { name: "Status views" }).getByRole("tab", { name: view, exact: true });
  await tab.click();
  await expect(tab).toHaveAttribute("aria-selected", "true");
}

/** Canvas toolbar → Connect (the keyboard connection form). Leaves it open if it already is. */
export async function openConnectionForm(page: Page) {
  await leaveSettings(page);
  const button = page
    .getByRole("group", { name: "Canvas layout actions" })
    .getByRole("button", { name: "Connect", exact: true });
  if ((await button.getAttribute("aria-pressed")) !== "true") await button.click();
}

/** Status → Troubleshoot and recover → Manual device binding. */
export async function openDeviceTroubleshooting(page: Page) {
  await openStatusView(page, "Readiness");
  for (const selector of ["details.troubleshoot-group", "details.device-troubleshooting"]) {
    const group = page.locator(selector);
    if (!(await group.evaluate((element) => (element as HTMLDetailsElement).open)))
      await group.locator("> summary").click();
  }
}

/** Change appearance in Settings, then return to the current tab. */
export async function setColorTheme(page: Page, theme: string) {
  const pages = page.getByRole("tablist", { name: "Settings pages" });
  const current = (await pages.isVisible()) ? await pages.getByRole("tab", { selected: true }).innerText() : null;
  await openSettings(page, "Appearance");
  await page.getByRole("combobox", { name: "Color theme" }).selectOption(theme);
  if (current) await pages.getByRole("tab", { name: current, exact: true }).click();
  else await backToRoute(page);
}

export const openDeviceAccessSettings = (page: Page) => openSettings(page, "Device access");
export const openVersionSettings = (page: Page) => openSettings(page, "New versions");

const STATUS_VIEWS = new Set(["Readiness", "Apps", "Timing"]);
const DAILY_TABS = new Set(["Tools", "Properties", "Status", "Session", "Recording"]);

/** Go to any page by its visible name, wherever it lives. */
export async function openPage(page: Page, name: string) {
  if (STATUS_VIEWS.has(name)) {
    await leaveSettings(page);
    return openStatusView(page, name as "Readiness" | "Apps" | "Timing");
  }
  if (!DAILY_TABS.has(name)) return openSettings(page, name);
  await leaveSettings(page);
  const tab = page.getByRole("tablist", { name: "Right sidebar" }).getByRole("tab", { name, exact: true });
  await tab.click();
  await expect(tab).toHaveAttribute("aria-selected", "true");
}

/** Open one section of Properties if it is closed. Each tool type remembers its own. */
export async function openSection(scope: Page | Locator, title: string) {
  const section = scope.locator(`.property-section:has(> summary:has-text("${title}"))`);
  if ((await section.getAttribute("open")) === null) await section.locator("> summary").click();
  await section.locator(".property-section-body").waitFor();
}

/** Open every closed section of Properties. */
export async function openAllSections(scope: Locator) {
  const closed = scope.locator(".property-section:not([open]) > summary");
  while (await closed.count()) await closed.first().click();
}

/** Explicitly fit after edits or zooming before interacting with offscreen cards. */
export async function fitCanvas(page: Page) {
  await page.locator(".react-flow__controls-fitview").click();
  // The view eases into place.
  await page.waitForTimeout(300);
}
