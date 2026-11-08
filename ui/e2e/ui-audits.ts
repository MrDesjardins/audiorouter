import type { Page } from "@playwright/test";

// Shared page audits used by the target-size, text-contrast, clipped-text and dialog tests.
// Buttons criterion: WCAG 2.5.8 asks for pointer targets of at least 24 by 24 CSS px.
// Inline links inside running text are exempt under 2.5.8, as is the canvas mini map.
export const smallTargets = (page: Page, where: string) =>
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

// Contrast criterion: WCAG 1.4.3 asks for 4.5:1 text contrast, or 3:1 for large text.
// Solid fills and opaque gradients (worst stop) are judged; text over a translucent layer or an
// image is left out rather than passed.
export const lowContrastText = (page: Page, where: string) =>
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

// Spacing criterion: text that does not fit must stay readable. A clipped label needs a
// tooltip (title) or an accessible name that carries the full text.
export const clippedText = (page: Page, where: string) =>
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
