import type { Page } from "@playwright/test";
import { expect, test } from "./browser-test";
import { setColorTheme } from "./workbench";

// Contrast criterion: WCAG 1.4.3 asks for 4.5:1 text contrast, or 3:1 for large text.
// Solid fills and opaque gradients (worst stop) are judged; text over a translucent layer or an
// image is left out rather than passed.
const lowContrastText = (page: Page, where: string) =>
  page.evaluate((label) => {
    const parse = (value: string) => {
      const match = value.match(/rgba?\(([^)]+)\)/);
      if (!match) return null;
      const [r, g, b, a = 1] = match[1]
        .split(/[ ,/]+/)
        .filter(Boolean)
        .map(Number);
      return { r, g, b, a };
    };
    const luminance = ({ r, g, b }: { r: number; g: number; b: number }) => {
      const channel = (value: number) => {
        const v = value / 255;
        return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
      };
      return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
    };
    // Returns every colour the text may sit on: one for a solid fill, each stop for an opaque
    // gradient. Null means the backdrop cannot be judged (translucent layer, image).
    const backgrounds = (start: Element) => {
      for (let node: Element | null = start; node; node = node.parentElement) {
        const style = getComputedStyle(node);
        if (style.backgroundImage !== "none") {
          if (style.backgroundImage.includes("url(")) return null;
          const stops = (style.backgroundImage.match(/rgba?\([^)]+\)/g) ?? []).map(parse);
          if (stops.length === 0 || stops.some((stop) => !stop || stop.a < 0.95)) return null;
          return stops as { r: number; g: number; b: number; a: number }[];
        }
        const color = parse(style.backgroundColor);
        if (color && color.a >= 0.95) return [color];
        if (color && color.a > 0.05) return null;
      }
      const body = parse(getComputedStyle(document.body).backgroundColor);
      return body ? [body] : null;
    };
    const found: string[] = [];
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    const seen = new Set<Element>();
    for (let text = walker.nextNode(); text; text = walker.nextNode()) {
      const element = text.parentElement;
      if (!element || seen.has(element) || !text.textContent?.trim()) continue;
      seen.add(element);
      const box = element.getBoundingClientRect();
      if (box.width === 0 || box.height === 0 || box.bottom < 0 || box.top > innerHeight) continue;
      const style = getComputedStyle(element);
      if (style.visibility === "hidden" || Number(style.opacity) < 0.95) continue;
      if (element.closest("[aria-hidden=true], .react-flow__minimap, :disabled, [aria-disabled=true]")) continue;
      const color = parse(style.color);
      const backs = backgrounds(element);
      if (!color || !backs || color.a < 0.95) continue;
      const ratio = Math.min(
        ...backs.map((back) => {
          const [light, dark] = [luminance(color), luminance(back)].sort((a, b) => b - a);
          return (light + 0.05) / (dark + 0.05);
        }),
      );
      const size = parseFloat(style.fontSize);
      const large = size >= 24 || (size >= 18.66 && Number(style.fontWeight) >= 700);
      if (ratio < (large ? 3 : 4.5)) {
        found.push(`${label}: "${text.textContent.trim().slice(0, 30)}" ${ratio.toFixed(2)}:1`);
      }
    }
    return found;
  }, where);

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`workbench and Settings text meets AA contrast in ${theme}`, async ({ page }) => {
    await page.setViewportSize({ width: 1500, height: 950 });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await setColorTheme(page, theme);
    const low: string[] = [];
    for (const name of ["Properties", "Tools", "Devices"]) {
      const tab = page.getByRole("tab", { name, exact: true });
      if ((await tab.count()) === 0) continue;
      await tab.click();
      low.push(...(await lowContrastText(page, name)));
    }
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    const list = page.getByRole("tablist", { name: "Settings pages" });
    for (const name of (await list.getByRole("tab").allInnerTexts()).map((value) => value.trim())) {
      const tab = list.getByRole("tab", { name, exact: true });
      await tab.click();
      await expect(tab).toHaveAttribute("aria-selected", "true");
      low.push(...(await lowContrastText(page, `Settings/${name}`)));
    }
    expect([...new Set(low)]).toEqual([]);
  });
}

test("the contrast check reports faint text on solid and gradient fills", async ({ page }) => {
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.evaluate(() => {
    const solid = document.createElement("p");
    solid.textContent = "Faint solid";
    solid.style.cssText = "position:fixed;top:0;left:0;z-index:9999;color:#777;background:#888;font-size:14px";
    const gradient = document.createElement("p");
    gradient.textContent = "Faint gradient";
    gradient.style.cssText =
      "position:fixed;top:40px;left:0;z-index:9999;color:#fff;background:linear-gradient(#000,#eee);font-size:14px";
    document.body.append(solid, gradient);
  });
  const low = await lowContrastText(page, "probe");
  expect(low.some((entry) => entry.includes("Faint solid"))).toBe(true);
  expect(low.some((entry) => entry.includes("Faint gradient"))).toBe(true);
});
