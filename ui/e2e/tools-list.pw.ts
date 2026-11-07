import type { Page } from "@playwright/test";
import { expect, test } from "./browser-test";

/** A PC that has never used Tools: none of the keys the shared fixture presets. */
async function firstRun(page: Page) {
  await page.addInitScript(() => localStorage.removeItem("audiorouter.ui.tools-open-groups"));
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
}
const topRows = (page: Page) => page.locator(".tool-group-top .tool-row strong");
const toggle = (page: Page, name: RegExp) => page.locator(".tool-group-toggle").filter({ hasText: name });

test("first run shows five common tools, and every group heading fits the first screen", async ({ page }) => {
  await firstRun(page);
  await expect(page.getByRole("heading", { name: "Common tools" })).toBeVisible();
  await expect(topRows(page)).toHaveText(["Input device", "Output device", "Gain", "Compressor", "Mixer"]);
  await expect(page.locator(".tool-group-toggle")).toHaveText([
    "Inputs6",
    "Outputs2",
    "Clean up6",
    "Level7",
    "Tone and voice6",
    "Routing and time6",
    "Canvas1",
    "Plug-ins",
    "Needs setup3",
  ]);
  for (const name of [/Clean up/, /Needs setup/])
    await expect(toggle(page, name)).toHaveAttribute("aria-expanded", "false");
  await expect(toggle(page, /Inputs/)).toHaveAttribute("aria-expanded", "true");
  const panel = page.locator(".right-workbench");
  const height = await panel.evaluate((element) => element.scrollHeight);
  expect(height).toBeLessThan(1454);
});

// 1454 px in the design counted tool rows and headings only. This also holds Recent, the
// reasons in Needs setup, the plug-in scan buttons, search and the description slot.
test("fully open is far shorter than the old 4226 px list", async ({ page }) => {
  await page.goto("/route-harness.html");
  const height = await page.locator(".right-workbench").evaluate((element) => element.scrollHeight);
  expect(height).toBeLessThan(2400);
});

test("recent tools refresh only on returning to the tab, and stay after a restart", async ({ page }) => {
  await firstRun(page);
  await toggle(page, /Level/).click();
  await page
    .locator(".tool-card")
    .filter({ has: page.getByText("Limiter", { exact: true }) })
    .click();
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page.getByRole("tab", { name: "Status", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Common tools" })).toHaveCount(0);
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Recent" })).toBeVisible();
  await expect(topRows(page)).toHaveCount(5);
  await expect(topRows(page).first()).toHaveText("Limiter");
  const before = await topRows(page).allTextContents();
  // A second add leaves the rows where they are while the tab stays open.
  await page
    .locator(".tool-card")
    .filter({ has: page.getByText("Mute", { exact: true }) })
    .click();
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  expect(await topRows(page).allTextContents()).toEqual(before);
  await page.reload();
  await expect(topRows(page).first()).toHaveText("Mute");
});

test("a pin stays on top and survives a restart", async ({ page }) => {
  await firstRun(page);
  await toggle(page, /Level/).click();
  const pin = page.getByRole("button", { name: "Pin Duck", exact: true }).last();
  await pin.focus();
  await expect(pin).toBeVisible();
  await pin.click();
  await expect(page.getByRole("button", { name: "Unpin Duck", exact: true }).first()).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await page.reload();
  await expect(topRows(page).first()).toHaveText("Duck");
});

test("tools that need setup say why, and can open Virtual Cables", async ({ page }) => {
  await firstRun(page);
  const setup = toggle(page, /Needs setup/);
  await expect(setup).toContainText("3");
  await setup.click();
  const cable = page.locator(".tool-setup").filter({ has: page.getByText("Cable input", { exact: true }) });
  await expect(cable.getByRole("button", { name: "Cable input" })).toBeDisabled();
  await expect(cable.locator("small")).toContainText("Virtual Cables");
  await cable.getByRole("button", { name: "Open Virtual Cables" }).click();
  await expect(page.getByRole("tablist", { name: "Settings pages" })).toBeVisible();
  await expect(page.getByRole("tab", { name: "Virtual Cables", exact: true })).toHaveAttribute("aria-selected", "true");
});

test("search still finds a tool in a closed group", async ({ page }) => {
  await firstRun(page);
  await page.getByLabel("Find a tool").fill("limiter");
  await expect(page.locator(".tool-group-top")).toHaveCount(0);
  await expect(page.locator(".tool-card").filter({ has: page.getByText("Limiter", { exact: true }) })).toBeVisible();
  await page.getByLabel("Find a tool").fill("zzzz");
  await page.getByRole("button", { name: "Clear tool search" }).click();
  await expect(page.getByLabel("Find a tool")).toBeFocused();
  await expect(topRows(page)).toHaveCount(5);
});

test("the description slot keeps its size and fits every tool's text (UI-17)", async ({ page }) => {
  await page.goto("/route-harness.html");
  const slot = page.locator(".tool-shelf-description");
  await expect(slot).toBeVisible();
  const height = (await slot.boundingBox())!.height;
  const panel = await page.locator(".tool-shelf").boundingBox();
  const rows = page.locator(".tool-card");
  const count = await rows.count();
  expect(count).toBeGreaterThan(25);
  for (let index = 0; index < count; index += 1) {
    await rows
      .nth(index)
      .focus({ timeout: 2000 })
      .catch(() => undefined);
    const box = (await slot.boundingBox())!;
    expect(box.height).toBe(height);
    const clipped = await slot.evaluate((element) =>
      [...element.children].some((child) => child.scrollHeight > child.clientHeight + 1),
    );
    expect(clipped, `row ${index}`).toBe(false);
  }
  expect((await page.locator(".tool-shelf").boundingBox())!.height).toBe(panel!.height);
  // Point somewhere else: the slot is pinned to the bottom of the visible panel.
  const sidebar = (await page.locator(".right-workbench").boundingBox())!;
  const shown = (await slot.boundingBox())!;
  expect(shown.y + shown.height).toBeLessThanOrEqual(sidebar.y + sidebar.height + 1);
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`Tools list screenshots in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await firstRun(page);
    await page.locator(".tool-row").nth(3).hover();
    await page.screenshot({ path: info.outputPath(`${theme}-tools-list.png`) });
    await toggle(page, /Needs setup/).click();
    await toggle(page, /Needs setup/).scrollIntoViewIfNeeded();
    await page.screenshot({ path: info.outputPath(`${theme}-tools-setup.png`) });
  });
}
