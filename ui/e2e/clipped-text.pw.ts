import type { Page } from "@playwright/test";
import { expect, test } from "./browser-test";
import { setColorTheme } from "./workbench";

// Spacing criterion: text that does not fit must stay readable. A clipped label needs a
// tooltip (title) or an accessible name that carries the full text.
const clippedText = (page: Page, where: string) =>
  page.evaluate((label) => {
    const found: string[] = [];
    for (const element of document.querySelectorAll<HTMLElement>("body *")) {
      if (!element.textContent?.trim() || element.children.length > 0) continue;
      const box = element.getBoundingClientRect();
      if (box.width === 0 || box.height === 0 || box.bottom < 0 || box.top > innerHeight) continue;
      const style = getComputedStyle(element);
      if (style.visibility === "hidden" || element.closest("[aria-hidden=true], .react-flow__minimap")) continue;
      if (style.overflowX === "visible" && style.overflowY === "visible") continue;
      if (/auto|scroll/.test(style.overflowX + style.overflowY)) continue;
      const clipped = element.scrollWidth > element.clientWidth + 1 || element.scrollHeight > element.clientHeight + 2;
      if (!clipped) continue;
      const text = element.textContent.trim();
      const named = element.closest("[title], [aria-label]");
      const full = named?.getAttribute("title") ?? named?.getAttribute("aria-label") ?? "";
      if (full.includes(text) || (full && text.endsWith("…"))) continue;
      found.push(`${label}: "${text.slice(0, 40)}" ${element.scrollWidth}>${element.clientWidth}`);
    }
    return found;
  }, where);

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
    for (const name of (await list.getByRole("tab").allInnerTexts()).map((value) => value.trim())) {
      const tab = list.getByRole("tab", { name, exact: true });
      await tab.click();
      await expect(tab).toHaveAttribute("aria-selected", "true");
      found.push(...(await clippedText(page, `Settings/${name}`)));
    }
    expect([...new Set(found)]).toEqual([]);
  });
}
