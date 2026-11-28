import type { Page } from "@playwright/test";

// Shared page audits used by the target-size, text-contrast, clipped-text and dialog tests.
// Buttons criterion: WCAG 2.5.8 asks for pointer targets of at least 24 by 24 CSS px.
// Inline links inside running text are exempt under 2.5.8, as is the canvas mini map.
// `scope` limits the audit to one container (a modal dialog), so the page behind it is not judged.
export const smallTargets = (page: Page, where: string, scope = "body") =>
  page.evaluate(
    ([label, scopeSelector]) => {
      const found: string[] = [];
      const root = document.querySelector(scopeSelector) ?? document.body;
      for (const element of root.querySelectorAll<HTMLElement>("button, a[href], [role=button], [role=tab]")) {
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
    },
    [where, scope] as const,
  );

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
export const clippedText = (page: Page, where: string, scope = "body") =>
  page.evaluate(
    ([label, scopeSelector]) => {
      const found: string[] = [];
      const root = document.querySelector(scopeSelector) ?? document.body;
      for (const element of root.querySelectorAll<HTMLElement>("*")) {
        if (!element.textContent?.trim() || element.children.length > 0) continue;
        const box = element.getBoundingClientRect();
        if (box.width === 0 || box.height === 0 || box.bottom < 0 || box.top > innerHeight) continue;
        const style = getComputedStyle(element);
        if (style.visibility === "hidden" || element.closest("[aria-hidden=true], .react-flow__minimap")) continue;
        if (style.overflowX === "visible" && style.overflowY === "visible") continue;
        if (/auto|scroll/.test(style.overflowX + style.overflowY)) continue;
        const clipped =
          element.scrollWidth > element.clientWidth + 1 || element.scrollHeight > element.clientHeight + 2;
        if (!clipped) continue;
        const text = element.textContent.trim();
        const named = element.closest("[title], [aria-label]");
        const full = named?.getAttribute("title") ?? named?.getAttribute("aria-label") ?? "";
        if (full.includes(text) || (full && text.endsWith("…"))) continue;
        found.push(`${label}: "${text.slice(0, 40)}" ${element.scrollWidth}>${element.clientWidth}`);
      }
      return found;
    },
    [where, scope] as const,
  );

// Spacing criterion for a dialog or panel: it fits the window, nothing sits closer than
// `inset` px to its edge, no two visible items overlap, stacked items keep a gap, and
// buttons that share a row line up. Items are the visible text runs and controls; a
// control's own text is part of the control. Every visible match of `selector` is judged;
// `fit: false` skips the window check for panels that scroll with the page.
export const crampedLayout = (
  page: Page,
  where: string,
  selector: string,
  { inset = 12, fit = true, rows = "" }: { inset?: number; fit?: boolean; rows?: string } = {},
) =>
  page.evaluate(
    ({ where, selector, inset, fit, rows }) => {
      const found: string[] = [];
      const CONTROL = "button, a[href], input:not([type=hidden]), select, textarea, [role=button], [role=switch]";
      const visible = (element: Element) => {
        const box = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return (
          element.checkVisibility({ contentVisibilityAuto: true, opacityProperty: true, visibilityProperty: true }) &&
          box.width > 0 &&
          box.height > 0 &&
          style.visibility !== "hidden" &&
          Number(style.opacity) > 0.05
        );
      };
      const roots = [...document.querySelectorAll<HTMLElement>(selector)].filter(
        (element) => visible(element) && !element.closest("[aria-hidden=true]"),
      );
      for (const root of roots) {
        const heading = root.querySelector("h2, h3, h4, legend")?.textContent?.trim().slice(0, 30);
        const label = roots.length > 1 && heading ? `${where} › ${heading}` : where;
        const frame = root.getBoundingClientRect();
        if (
          fit &&
          (frame.left < 0 || frame.top < 0 || frame.right > innerWidth + 0.5 || frame.bottom > innerHeight + 0.5)
        )
          found.push(`${label}: does not fit the ${innerWidth}x${innerHeight} window`);
        const name = (element: Element) =>
          (element.getAttribute("aria-label") || element.textContent?.trim() || element.tagName.toLowerCase()).slice(
            0,
            28,
          );
        const items: { element: Element; box: DOMRect }[] = [];
        for (const element of root.querySelectorAll(CONTROL)) {
          if (visible(element) && !element.parentElement?.closest(CONTROL) && !element.closest("[aria-hidden=true]"))
            items.push({ element, box: element.getBoundingClientRect() });
        }
        const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
        for (let text = walker.nextNode(); text; text = walker.nextNode()) {
          const parent = text.parentElement;
          if (!parent || !text.textContent?.trim() || parent.closest(CONTROL) || parent.closest("[aria-hidden=true]"))
            continue;
          if (!visible(parent) || items.some((item) => item.element === parent)) continue;
          const range = document.createRange();
          range.selectNodeContents(text);
          const box = range.getBoundingClientRect();
          if (box.width > 0 && box.height > 0) items.push({ element: parent, box });
        }
        // Items inside a scrolled list are judged against the list, not the dialog edge.
        const scroller = (element: Element) => {
          for (let node = element.parentElement; node && node !== root; node = node.parentElement)
            if (/auto|scroll/.test(getComputedStyle(node).overflowY)) return node;
          return null;
        };
        // A root taller than the window must scroll itself; then only its sides are judged here.
        const scrolls = root.scrollHeight > root.clientHeight + 1;
        if (scrolls && !/auto|scroll/.test(getComputedStyle(root).overflowY))
          found.push(`${label}: content runs past the bottom without a scroll bar`);
        for (const { element, box } of items) {
          if (scroller(element)) continue;
          const gaps = [box.left - frame.left, frame.right - box.right];
          if (!scrolls) gaps.push(box.top - frame.top, frame.bottom - box.bottom);
          if (Math.min(...gaps) < inset - 0.5)
            found.push(`${label}: "${name(element)}" ${Math.round(Math.min(...gaps))} px from the edge`);
        }
        for (let i = 0; i < items.length; i += 1) {
          for (let j = i + 1; j < items.length; j += 1) {
            const [a, b] = [items[i], items[j]];
            if (a.element.contains(b.element) || b.element.contains(a.element)) continue;
            if (scroller(a.element) !== scroller(b.element)) continue;
            // Rows of a list (`rows`) sit flush like menu items, and a row's own small
            // buttons (pin, guide) may sit on it; overlap between two rows still counts.
            const [rowA, rowB] = [a, b].map((item) => (rows ? item.element.closest(rows) : null));
            const sameRow = rowA !== null && rowA === rowB;
            const listed = rowA !== null && rowB !== null;
            const across = Math.min(a.box.right, b.box.right) - Math.max(a.box.left, b.box.left);
            const down = Math.min(a.box.bottom, b.box.bottom) - Math.max(a.box.top, b.box.top);
            if (across > 1 && down > 1) {
              if (!sameRow) found.push(`${label}: "${name(a.element)}" overlaps "${name(b.element)}"`);
              continue;
            }
            // Stacked: one above the other with shared width; they need a visible gap.
            const controls = [a, b].filter((item) => item.element.matches(CONTROL)).length;
            if (across > 1 && down > -4 && controls > 0 && !listed)
              found.push(`${label}: "${name(a.element)}" touches "${name(b.element)}" (${Math.round(-down)} px)`);
            // Side by side: two buttons on one row share a top and a height.
            const buttons = [a, b].every((item) => item.element.matches("button, [role=button]"));
            if (buttons && down > Math.min(a.box.height, b.box.height) / 2 && across < 1) {
              if (Math.abs(a.box.top - b.box.top) > 1 || Math.abs(a.box.height - b.box.height) > 1)
                found.push(`${label}: buttons "${name(a.element)}" and "${name(b.element)}" do not line up`);
            }
          }
        }
      }
      if (roots.length === 0) found.push(`${where}: ${selector} not found`);
      return found;
    },
    { where, selector, inset, fit, rows },
  );

// Inputs criterion: every visible field has an accessible name (label, aria-label or
// aria-labelledby), so Narrator announces what it is for.
export const unnamedFields = (page: Page, where: string) =>
  page.evaluate((label) => {
    const found: string[] = [];
    for (const field of document.querySelectorAll<HTMLElement>(
      "input:not([type=hidden]), select, textarea, [role=combobox], [role=slider], [role=switch], [role=checkbox]",
    )) {
      const box = field.getBoundingClientRect();
      const input = field as HTMLInputElement;
      if (box.width === 0 || box.height === 0 || field.closest("[aria-hidden=true]")) continue;
      // A file input hidden behind a styled button is announced through that button.
      if (input.type === "file" && getComputedStyle(field).opacity === "0") continue;
      const labelledBy = (field.getAttribute("aria-labelledby") ?? "")
        .split(/\s+/)
        .map((id) => document.getElementById(id)?.textContent?.trim() ?? "")
        .join("");
      const named =
        field.getAttribute("aria-label")?.trim() ||
        labelledBy ||
        Array.from(input.labels ?? []).some((element) => element.textContent?.trim()) ||
        field.getAttribute("title")?.trim();
      if (!named) found.push(`${label}: ${field.tagName.toLowerCase()} ${field.className || field.id || ""}`.trim());
    }
    return found;
  }, where);
