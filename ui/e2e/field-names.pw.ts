import { expect, test } from "./browser-test";
import { unnamedFields } from "./ui-audits";
import { fitCanvas } from "./workbench";

test("every field on the panels and Settings pages has an accessible name", async ({ page }) => {
  // Walks about 35 cards and 15 Settings pages: about 15 s alone, more beside other workers.
  test.setTimeout(90_000);
  await page.setViewportSize({ width: 1500, height: 950 });
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  const found: string[] = [];
  for (const name of ["Properties", "Tools", "Devices"]) {
    const tab = page.getByRole("tab", { name, exact: true });
    if ((await tab.count()) === 0) continue;
    await tab.click();
    found.push(...(await unnamedFields(page, name)));
  }
  // Add one of every tool, then visit each card: every kind's Properties shows different fields.
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  const tools = page.locator(".tool-card");
  const toolCount = await tools.count();
  expect(toolCount).toBeGreaterThan(20);
  for (let index = 0; index < toolCount; index += 1) {
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    await tools.nth(index).focus();
    await page.keyboard.press("Enter");
    await page.keyboard.press("Escape");
  }
  // A few cards (app and plugin pickers) open a chooser instead of adding a card directly.
  expect(await page.locator(".react-flow__node").count()).toBeGreaterThan(toolCount - 6);
  await fitCanvas(page);
  for (const card of await page.locator(".react-flow__node .flow-node-title").all()) {
    await card.dispatchEvent("click");
    await expect(page.locator(".main-content > .inspector")).toContainText((await card.innerText()).trim());
    found.push(...(await unnamedFields(page, `Properties/${(await card.innerText()).trim()}`)));
  }
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const list = page.getByRole("tablist", { name: "Settings pages" });
  for (const name of (await list.getByRole("tab").allInnerTexts()).map((value) => value.trim())) {
    const tab = list.getByRole("tab", { name, exact: true });
    await tab.click();
    await expect(tab).toHaveAttribute("aria-selected", "true");
    found.push(...(await unnamedFields(page, `Settings/${name}`)));
  }
  expect([...new Set(found)]).toEqual([]);
});

test("the field-name audit reports a field nobody can identify", async ({ page }) => {
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.evaluate(() => {
    const field = document.createElement("input");
    field.className = "probe-unnamed";
    field.style.cssText = "position:fixed;top:0;left:0;z-index:9999";
    document.body.append(field);
  });
  expect(await unnamedFields(page, "probe")).toContain("probe: input probe-unnamed");
});
