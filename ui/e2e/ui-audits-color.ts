import type { Locator, Page } from "@playwright/test";

// Colour audits that go further than `lowContrastText` in ui-audits.ts (Round 28 gaps):
//  - text over translucent layers, composited layer by layer (WCAG 1.4.3),
//  - the text a field shows (value and placeholder), which a text-node walk never reaches,
//  - non-text boundaries: field borders and icon-only glyphs need 3:1 (WCAG 1.4.11),
//  - keyboard focus rings need 3:1 against what they sit on (WCAG 1.4.11, 2.4.7),
//  - hover and pressed states keep their contrast.
// Every audit returns how many elements it judged and how many it had to skip, so a test can also
// require that the audit looked at something: a passing audit that judged nothing proves nothing.
//
// Known limit: layers are composited in ancestor order, which is not always paint order. An
// absolutely positioned sibling behind a text, an image, or a `backdrop-filter` makes the backdrop
// unknowable, so those elements are counted as skipped rather than passed.

export interface AuditResult {
  failures: string[];
  judged: number;
  skipped: number;
}

interface Rgba {
  r: number;
  g: number;
  b: number;
  a: number;
}

interface AuditTools {
  parse: (value: string) => Rgba | null;
  over: (top: Rgba, bottom: Rgba) => Rgba;
  ratio: (a: Rgba, b: Rgba) => number;
  /** Every opaque colour the area under `start` may show (one per gradient stop), or null. */
  backdropOf: (start: Element | null) => Rgba[] | null;
  /** The lowest contrast of `foreground` (may be translucent) against any backdrop colour. */
  worst: (foreground: Rgba, backdrops: Rgba[]) => number;
  /** Rings drawn with box-shadow: colour, blur and spread. */
  shadowRings: (value: string) => { color: Rgba; blur: number; spread: number; inset: boolean }[];
}

const installTools = () => {
  const canvas = document.createElement("canvas");
  canvas.width = 1;
  canvas.height = 1;
  const context = canvas.getContext("2d", { willReadFrequently: true })!;
  const parse = (value: string): Rgba | null => {
    const text = value.trim();
    const direct = text.match(/^rgba?\(([^)]+)\)$/);
    if (direct) {
      const [r, g, b, a = 1] = direct[1]
        .split(/[ ,/]+/)
        .filter(Boolean)
        .map(Number);
      return { r, g, b, a };
    }
    // color-mix() and friends come back from getComputedStyle as color(srgb 0.1 0.2 0.3 / 0.5).
    const srgb = text.match(/^color\(srgb ([^)]+)\)$/);
    if (srgb) {
      const [r, g, b, a = 1] = srgb[1].split(/[ /]+/).filter(Boolean).map(Number);
      return { r: r * 255, g: g * 255, b: b * 255, a };
    }
    // Anything else (oklch, lab, display-p3): let the browser convert it.
    context.clearRect(0, 0, 1, 1);
    context.fillStyle = "#010203";
    context.fillStyle = text;
    context.fillRect(0, 0, 1, 1);
    const [r, g, b, a] = context.getImageData(0, 0, 1, 1).data;
    return a === 0 && !/transparent|rgba?\(.*\/ ?0\)|, 0\)/.test(text) ? null : { r, g, b, a: a / 255 };
  };
  const channel = (value: number) => {
    const v = value / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  const luminance = ({ r, g, b }: Rgba) => 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
  const ratio = (a: Rgba, b: Rgba) => {
    const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
    return (light + 0.05) / (dark + 0.05);
  };
  const over = (top: Rgba, bottom: Rgba): Rgba => ({
    r: top.r * top.a + bottom.r * (1 - top.a),
    g: top.g * top.a + bottom.g * (1 - top.a),
    b: top.b * top.a + bottom.b * (1 - top.a),
    a: 1,
  });
  const backdropOf = (start: Element | null): Rgba[] | null => {
    // Layers from the top (nearest the content) down to the first opaque one.
    const layers: { stops: Rgba[] }[] = [];
    let opaque = false;
    for (let node: Element | null = start; node && !opaque; node = node.parentElement) {
      const style = getComputedStyle(node);
      if (style.backdropFilter && style.backdropFilter !== "none") return null;
      const image = style.backgroundImage;
      if (image !== "none") {
        if (image.includes("url(")) return null;
        const stops = (image.match(/(?:rgba?|color)\([^)]*\)/g) ?? []).map(parse);
        if (stops.length === 0 || stops.some((stop) => !stop)) return null;
        layers.push({ stops: stops as Rgba[] });
        if ((stops as Rgba[]).every((stop) => stop.a >= 0.99)) opaque = true;
      }
      if (opaque) break;
      const color = parse(style.backgroundColor);
      if (color && color.a > 0.01) {
        layers.push({ stops: [color] });
        if (color.a >= 0.99) opaque = true;
      }
    }
    if (!opaque) return null;
    let backdrops: Rgba[] = [];
    for (const layer of layers.reverse()) {
      backdrops =
        backdrops.length === 0 ? layer.stops : layer.stops.flatMap((stop) => backdrops.map((back) => over(stop, back)));
      if (backdrops.length > 48)
        backdrops = backdrops.filter((_, index) => index % Math.ceil(backdrops.length / 48) === 0);
    }
    return backdrops;
  };
  const worst = (foreground: Rgba, backdrops: Rgba[]) =>
    Math.min(...backdrops.map((back) => ratio(over(foreground, back), back)));
  const shadowRings = (value: string) => {
    const rings: { color: Rgba; blur: number; spread: number; inset: boolean }[] = [];
    for (const match of value.matchAll(
      /((?:rgba?|color)\([^)]*\))\s+(-?[\d.]+)px\s+(-?[\d.]+)px(?:\s+(-?[\d.]+)px)?(?:\s+(-?[\d.]+)px)?(\s+inset)?/g,
    )) {
      const color = parse(match[1]);
      if (color) rings.push({ color, blur: Number(match[4] ?? 0), spread: Number(match[5] ?? 0), inset: !!match[6] });
    }
    return rings;
  };
  Object.assign(window, { __auditTools: { parse, over, ratio, backdropOf, worst, shadowRings } satisfies AuditTools });
};

/** The page can be audited only after the helpers exist in it. */
const withTools = async (page: Page) => page.evaluate(installTools);

/**
 * Text contrast, with translucent layers composited, plus the value and placeholder text of
 * fields. WCAG 1.4.3: 4.5:1, or 3:1 for large text. Disabled controls are exempt.
 */
export const composedTextContrast = async (page: Page, where: string, scope = "body"): Promise<AuditResult> => {
  await withTools(page);
  return page.evaluate(
    ([label, scopeSelector]) => {
      const root = document.querySelector(scopeSelector) ?? document.body;
      const tools = (window as unknown as { __auditTools: AuditTools }).__auditTools;
      const result: AuditResult = { failures: [], judged: 0, skipped: 0 };
      const opacityOf = (start: Element) => {
        let total = 1;
        for (let node: Element | null = start; node; node = node.parentElement)
          total *= Number(getComputedStyle(node).opacity);
        return total;
      };
      const judge = (element: Element, foreground: string, text: string, kind: string) => {
        const style = getComputedStyle(element);
        const color = tools.parse(foreground);
        const backs = tools.backdropOf(element);
        if (!color || !backs || opacityOf(element) < 0.95) {
          result.skipped += 1;
          return;
        }
        result.judged += 1;
        const size = parseFloat(style.fontSize);
        const large = size >= 24 || (size >= 18.66 && Number(style.fontWeight) >= 700);
        const ratio = tools.worst(color, backs);
        if (ratio < (large ? 3 : 4.5))
          result.failures.push(`${label}: ${kind} "${text.slice(0, 30)}" ${ratio.toFixed(2)}:1`);
      };
      const visible = (element: Element) => {
        const box = element.getBoundingClientRect();
        if (box.width === 0 || box.height === 0 || box.bottom < 0 || box.top > innerHeight) return false;
        if (getComputedStyle(element).visibility === "hidden") return false;
        return !element.closest("[aria-hidden=true], .react-flow__minimap, :disabled, [aria-disabled=true]");
      };
      const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
      const seen = new Set<Element>();
      for (let text = walker.nextNode(); text; text = walker.nextNode()) {
        const element = text.parentElement;
        if (!element || seen.has(element) || !text.textContent?.trim() || !visible(element)) continue;
        seen.add(element);
        judge(element, getComputedStyle(element).color, text.textContent.trim(), "text");
      }
      for (const field of root.querySelectorAll<HTMLInputElement>("input, textarea, select")) {
        if (!visible(field) || ["checkbox", "radio", "range", "file", "hidden", "color"].includes(field.type)) continue;
        if (field.value) judge(field, getComputedStyle(field).color, field.value, "field value");
        if (field.placeholder)
          judge(field, getComputedStyle(field, "::placeholder").color, field.placeholder, "placeholder");
      }
      return result;
    },
    [where, scope] as const,
  );
};

/**
 * Non-text contrast, WCAG 1.4.11: a field must be findable, so its border or its fill reaches 3:1
 * against what it sits on. An icon-only button must show its glyph at 3:1 against its own fill.
 * Text buttons are not judged here: their label identifies them (and 1.4.3 covers the label).
 */
export const boundaryContrast = async (page: Page, where: string, scope = "body"): Promise<AuditResult> => {
  await withTools(page);
  return page.evaluate(
    ([label, scopeSelector]) => {
      const root = document.querySelector(scopeSelector) ?? document.body;
      const tools = (window as unknown as { __auditTools: AuditTools }).__auditTools;
      const result: AuditResult = { failures: [], judged: 0, skipped: 0 };
      const describe = (element: Element) =>
        `${element.tagName.toLowerCase()}${element.getAttribute("aria-label") ? ` "${element.getAttribute("aria-label")}"` : ""}.${String(element.className).split(" ")[0]}`;
      const fields = root.querySelectorAll<HTMLElement>(
        "input:not([type=hidden]):not([type=file]):not([type=range]):not([type=color]), select, textarea, [role=switch], [role=checkbox]",
      );
      for (const field of fields) {
        const box = field.getBoundingClientRect();
        if (box.width === 0 || box.height === 0 || box.bottom < 0 || box.top > innerHeight) continue;
        if (field.closest("[aria-hidden=true], .react-flow__minimap") || (field as HTMLInputElement).disabled) continue;
        const style = getComputedStyle(field);
        if (style.visibility === "hidden" || Number(style.opacity) < 0.95) continue;
        const kind = (field as HTMLInputElement).type;
        // A native checkbox or radio is drawn by the browser from its own palette.
        if ((kind === "checkbox" || kind === "radio") && style.appearance !== "none") {
          result.skipped += 1;
          continue;
        }
        const outside = tools.backdropOf(field.parentElement);
        const fill = tools.parse(style.backgroundColor);
        if (!outside || !fill) {
          result.skipped += 1;
          continue;
        }
        let best = 0;
        const fillRatio = fill.a > 0.01 ? tools.worst(fill, outside) : 1;
        best = Math.max(best, fillRatio);
        for (const side of ["Top", "Right", "Bottom", "Left"] as const) {
          const styleName = style[`border${side}Style`];
          if (styleName === "none" || styleName === "hidden" || parseFloat(style[`border${side}Width`]) < 1) continue;
          const color = tools.parse(style[`border${side}Color`]);
          if (color) best = Math.max(best, tools.worst(color, outside));
        }
        result.judged += 1;
        if (best < 3)
          result.failures.push(
            `${label}: ${describe(field)} edge ${best.toFixed(2)}:1 (border ${style.borderTopColor}, fill ${style.backgroundColor}, on ${outside.map((c) => `rgb(${Math.round(c.r)} ${Math.round(c.g)} ${Math.round(c.b)})`).join("/")})`,
          );
      }
      for (const button of root.querySelectorAll<HTMLElement>("button, [role=button], [role=tab]")) {
        const box = button.getBoundingClientRect();
        if (box.width === 0 || box.height === 0 || box.bottom < 0 || box.top > innerHeight) continue;
        if (button.textContent?.trim() || !button.querySelector("svg, img")) continue;
        if (button.closest("[aria-hidden=true], .react-flow__minimap, :disabled, [aria-disabled=true]")) continue;
        const style = getComputedStyle(button);
        if (style.visibility === "hidden" || Number(style.opacity) < 0.95) continue;
        const glyph = tools.parse(style.color);
        const backs = tools.backdropOf(button);
        if (!glyph || !backs) {
          result.skipped += 1;
          continue;
        }
        result.judged += 1;
        const ratio = tools.worst(glyph, backs);
        if (ratio < 3) result.failures.push(`${label}: icon ${describe(button)} ${ratio.toFixed(2)}:1`);
      }
      return result;
    },
    [where, scope] as const,
  );
};

/**
 * Presses Tab `steps` times and judges the focus indicator of each element reached: an outline or a
 * box-shadow ring must exist and reach 3:1 against what it sits on (outside the element for a
 * positive offset, on the element for an inset one). Tab, not `focus()`, so `:focus-visible`
 * really applies.
 */
export const focusRingContrast = async (page: Page, where: string, steps: number): Promise<AuditResult> => {
  await withTools(page);
  const result: AuditResult = { failures: [], judged: 0, skipped: 0 };
  for (let step = 0; step < steps; step += 1) {
    await page.keyboard.press("Tab");
    const outcome = await page.evaluate((label) => {
      const tools = (window as unknown as { __auditTools: AuditTools }).__auditTools;
      const element = document.activeElement;
      if (!element || element === document.body) return { end: true } as const;
      const box = element.getBoundingClientRect();
      const name = `${element.tagName.toLowerCase()} "${(element.getAttribute("aria-label") || element.textContent || element.className || "").toString().trim().slice(0, 30)}"`;
      if (box.width === 0 || box.height === 0 || !element.matches(":focus-visible"))
        return { skipped: `${label}: ${name}` } as const;
      const style = getComputedStyle(element);
      const outside = tools.backdropOf(element.parentElement);
      const inside = tools.backdropOf(element);
      if (!outside || !inside) return { skipped: `${label}: ${name}` } as const;
      const ratios: number[] = [];
      if (style.outlineStyle === "auto") {
        // The browser's own focus ring (summary, scrollable regions): two-tone by design so it stays
        // visible on any backdrop, and its computed colour does not describe what is drawn. Judged
        // as present; it is not the app's --focus-ring, which is a consistency matter, not contrast.
        ratios.push(3);
      } else if (style.outlineStyle !== "none" && parseFloat(style.outlineWidth) >= 1) {
        const color = tools.parse(style.outlineColor);
        if (color) ratios.push(tools.worst(color, parseFloat(style.outlineOffset) >= 0 ? outside : inside));
      }
      for (const ring of tools.shadowRings(style.boxShadow)) {
        if (ring.blur <= 2 && (ring.spread >= 1 || ring.inset))
          ratios.push(tools.worst(ring.color, ring.inset ? inside : outside));
      }
      if (ratios.length === 0) return { problem: `${label}: ${name} shows no focus indicator` } as const;
      const best = Math.max(...ratios);
      return best < 3
        ? ({ problem: `${label}: ${name} focus ring ${best.toFixed(2)}:1` } as const)
        : ({ ok: true } as const);
    }, where);
    if ("end" in outcome) break;
    if ("skipped" in outcome) result.skipped += 1;
    else {
      result.judged += 1;
      if ("problem" in outcome && outcome.problem) result.failures.push(outcome.problem);
    }
  }
  return result;
};

/**
 * Hovers up to `limit` visible, enabled elements matching `selector` and judges their label
 * contrast (4.5:1) or icon contrast (3:1) in the hovered state, after any transition settles.
 */
export const hoverContrast = async (
  page: Page,
  where: string,
  selector: string,
  limit: number,
): Promise<AuditResult> => {
  await withTools(page);
  await page.emulateMedia({ reducedMotion: "reduce" });
  const result: AuditResult = { failures: [], judged: 0, skipped: 0 };
  const targets: Locator = page.locator(selector).filter({ visible: true });
  const total = await targets.count();
  const stride = Math.max(1, Math.floor(total / limit));
  for (let index = 0, taken = 0; index < total && taken < limit; index += stride) {
    const target = targets.nth(index);
    if (!(await target.isEnabled().catch(() => false))) continue;
    if (
      !(await target.evaluate((element) => {
        const box = element.getBoundingClientRect();
        return (
          box.width > 0 &&
          box.bottom > 0 &&
          box.top < innerHeight &&
          !element.closest("[aria-hidden=true], .react-flow__minimap")
        );
      }))
    ) {
      result.skipped += 1;
      continue;
    }
    taken += 1;
    try {
      await target.hover({ timeout: 2000 });
    } catch {
      result.skipped += 1;
      continue;
    }
    const outcome = await target.evaluate(async (element, label) => {
      const tools = (window as unknown as { __auditTools: AuditTools }).__auditTools;
      await new Promise((resolve) => setTimeout(resolve, 150));
      if (!element.matches(":hover")) return { skipped: true } as const;
      const style = getComputedStyle(element);
      const color = tools.parse(style.color);
      const backs = tools.backdropOf(element);
      const text = element.textContent?.trim() ?? "";
      if (!color || !backs) return { skipped: true } as const;
      const iconOnly = !text && element.querySelector("svg, img");
      const ratio = tools.worst(color, backs);
      const size = parseFloat(style.fontSize);
      const large = size >= 24 || (size >= 18.66 && Number(style.fontWeight) >= 700);
      const needed = iconOnly || large ? 3 : 4.5;
      const name = element.getAttribute("aria-label") || text || String(element.className);
      return ratio < needed
        ? { problem: `${label}: hovered "${name.slice(0, 30)}" ${ratio.toFixed(2)}:1 (needs ${needed})` }
        : { ok: true };
    }, where);
    if ("skipped" in outcome) result.skipped += 1;
    else {
      result.judged += 1;
      if ("problem" in outcome && outcome.problem) result.failures.push(outcome.problem);
    }
  }
  await page.mouse.move(0, 0);
  return result;
};

/**
 * Pressed states. A toggle button (`aria-pressed`) must look different when it is on, and the
 * difference must be a visible one: some colour pair of its edge, fill or text differs by at least
 * 1.5:1 between the states (or its shadow differs), and the "on" look shows an edge or fill that
 * reaches 3:1 against its surroundings or against its own "off" look (WCAG 1.4.11, states of
 * components). Each toggle is clicked and clicked back, with the pointer moved away so hover does
 * not stand in for the pressed look. Pin buttons are left out: the pin changes shape (filled
 * glyph), which is not a colour question and is checked by its own test.
 */
export const pressedStates = async (page: Page, where: string): Promise<AuditResult> => {
  await withTools(page);
  const result: AuditResult = { failures: [], judged: 0, skipped: 0 };
  const buttons = page.locator("button[aria-pressed]:not(.tool-pin)").filter({ visible: true });
  const count = await buttons.count();
  const look = async (button: Locator) => {
    await page.mouse.move(0, 0);
    await page.waitForTimeout(250);
    return button.evaluate((element) => {
      const tools = (window as unknown as { __auditTools: AuditTools }).__auditTools;
      const style = getComputedStyle(element);
      const edge = tools.parse(style.borderTopColor);
      return {
        pressed: element.getAttribute("aria-pressed"),
        fill: tools.backdropOf(element)?.[0] ?? null,
        outside: tools.backdropOf(element.parentElement)?.[0] ?? null,
        edge: edge && parseFloat(style.borderTopWidth) >= 1 ? edge : null,
        text: tools.parse(style.color),
        shadow: style.boxShadow,
        name: element.getAttribute("aria-label") || element.textContent?.trim() || String(element.className),
      };
    });
  };
  for (let index = 0; index < count; index += 1) {
    const button = buttons.nth(index);
    const inView = await button.evaluate((element) => {
      const box = element.getBoundingClientRect();
      return box.width > 0 && box.bottom > 0 && box.top < innerHeight && !element.closest("[aria-hidden=true]");
    });
    if (!inView || !(await button.isEnabled())) {
      result.skipped += 1;
      continue;
    }
    const before = await look(button);
    await button.click();
    const after = await look(button);
    await button.click();
    await page.mouse.move(0, 0);
    if (before.pressed === after.pressed) {
      result.skipped += 1;
      continue;
    }
    const on = after.pressed === "true" ? after : before;
    const off = on === after ? before : after;
    const verdict = await page.evaluate(
      ([onLook, offLook]) => {
        const tools = (window as unknown as { __auditTools: AuditTools }).__auditTools;
        const pair = (a: Rgba | null, b: Rgba | null) => (a && b ? tools.ratio(a, b) : a || b ? 21 : 1);
        const change = Math.max(
          pair(onLook.fill, offLook.fill),
          pair(onLook.edge, offLook.edge),
          pair(onLook.text, offLook.text),
        );
        const indicator = Math.max(
          onLook.edge && onLook.outside ? tools.ratio(onLook.edge, onLook.outside) : 0,
          onLook.fill && onLook.outside ? tools.ratio(onLook.fill, onLook.outside) : 0,
        );
        const shadowChanged = onLook.shadow !== offLook.shadow;
        if (change < 1.5 && !shadowChanged) return `looks the same pressed and not pressed (${change.toFixed(2)}:1)`;
        if (indicator < 3 && change < 3)
          return `pressed look is faint (edge or fill ${indicator.toFixed(2)}:1, change ${change.toFixed(2)}:1)`;
        return "";
      },
      [on, off] as const,
    );
    result.judged += 1;
    if (verdict) result.failures.push(`${where}: "${on.name.slice(0, 30)}" ${verdict}`);
  }
  return result;
};

/** The look of a toggle button, for comparing its pressed and unpressed states. */
export const toggleLook = (button: Locator) =>
  button.evaluate((element) => {
    const tools = (window as unknown as { __auditTools: AuditTools }).__auditTools;
    const style = getComputedStyle(element);
    const backs = tools.backdropOf(element) ?? [];
    const outside = tools.backdropOf(element.parentElement) ?? [];
    const fill = backs[0] ?? null;
    const edge = tools.parse(style.borderTopColor);
    return {
      fill,
      outside: outside[0] ?? null,
      edge: edge && parseFloat(style.borderTopWidth) >= 1 ? edge : null,
      text: tools.parse(style.color),
      shadow: style.boxShadow,
      pressed: element.getAttribute("aria-pressed"),
    };
  });

/** Contrast ratio between two colours, evaluated in the page's own colour code. */
export const colorRatio = (page: Page, a: Rgba, b: Rgba) =>
  page.evaluate(
    ([first, second]) => (window as unknown as { __auditTools: AuditTools }).__auditTools.ratio(first, second),
    [a, b] as const,
  );

export const prepareColorTools = withTools;
export type { Rgba };
