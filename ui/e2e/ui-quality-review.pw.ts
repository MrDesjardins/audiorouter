import { openPage } from "./workbench";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { test, expect, type Page } from "./browser-test";
import type { PluginScanEntry } from "@audiorouter/contracts";
import { demoSession } from "../src/fixtures";
import { appendPluginPlaceholderNode } from "../src/draft";

const phase = process.env.UI_REVIEW_PHASE ?? "current";
const evidence = path.resolve("../target/ui-quality-review", phase);
const pluginEntry: PluginScanEntry = {
  path: "C:\\Program Files\\Common Files\\VST3\\uaudio_ua_1176.vst3",
  identity: {
    path: "C:\\Program Files\\Common Files\\VST3\\uaudio_ua_1176.vst3",
    binaryPath: "C:\\Program Files\\Common Files\\VST3\\uaudio_ua_1176.vst3\\Contents\\x86_64-win\\uaudio_ua_1176.vst3",
    format: "vst3",
    architecture: "x64",
    fileBytes: 1024,
    sha256: "a".repeat(64),
    vendor: "Universal Audio",
    version: "1.0.0",
    classIds: [],
    compatibility: "supportedVst3X64",
  },
  error: null,
  errorCode: null,
};
const quarantinedPluginSession = appendPluginPlaceholderNode(demoSession, pluginEntry);

async function capture(page: Page, name: string) {
  await mkdir(evidence, { recursive: true });
  await page.screenshot({ path: path.join(evidence, `${name}.png`) });
}

// Candidate audit: reports real foreground colors and composited solid ancestor
// backgrounds. Gradients/images/opacity are flagged for visual confirmation;
// this is evidence collection, not a claim of a complete WCAG audit.
async function textContrast(page: Page) {
  return page.evaluate(() => {
    const canvas = document.createElement("canvas");
    const context = canvas.getContext("2d")!;
    const rgba = (color: string) => {
      context.clearRect(0, 0, 1, 1);
      context.fillStyle = color;
      context.fillRect(0, 0, 1, 1);
      return [...context.getImageData(0, 0, 1, 1).data].map((v, i) => (i === 3 ? v / 255 : v));
    };
    const over = (front: number[], back: number[]) =>
      [0, 1, 2].map((i) => front[i] * front[3] + back[i] * (1 - front[3])).concat(1);
    const luminance = (color: number[]) =>
      color.slice(0, 3).reduce((sum, value, i) => {
        const c = value / 255;
        return sum + (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4) * [0.2126, 0.7152, 0.0722][i];
      }, 0);
    return [...document.querySelectorAll<HTMLElement>(".app-shell *")].flatMap((element) => {
      if (![...element.childNodes].some((node) => node.nodeType === Node.TEXT_NODE && node.textContent?.trim()))
        return [];
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      if (!rect.width || !rect.height || rect.bottom <= 0 || rect.top >= innerHeight || style.visibility !== "visible")
        return [];
      if (element.closest(":disabled, [aria-disabled=true]")) return [];
      const layers: number[][] = [];
      let approximate = false;
      for (let parent: HTMLElement | null = element; parent; parent = parent.parentElement) {
        const s = getComputedStyle(parent);
        approximate ||= s.backgroundImage !== "none" || Number(s.opacity) < 1;
        layers.push(rgba(s.backgroundColor));
      }
      const background = layers.reverse().reduce((back, front) => over(front, back), [255, 255, 255, 1]);
      const foreground = over(rgba(style.color), background);
      const l = [luminance(background), luminance(foreground)].sort((a, b) => b - a);
      const ratio = (l[0] + 0.05) / (l[1] + 0.05);
      const large =
        parseFloat(style.fontSize) >= 24 || (parseFloat(style.fontSize) >= 18.66 && Number(style.fontWeight) >= 700);
      return ratio < (large ? 3 : 4.5)
        ? [
            {
              text: element.innerText?.slice(0, 100),
              className: element.className,
              color: style.color,
              background,
              ratio,
              fontSize: style.fontSize,
              approximate,
            },
          ]
        : [];
    });
  });
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`canvas menu stays inside the canvas with enlarged text in ${theme}`, async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    // Exercise 200% text independently of viewport scaling. This is browser
    // geometry evidence, not attended Windows display-scaling qualification.
    await page.addStyleTag({ content: ".canvas-context-menu button { font-size: 32px; }" });
    await page.getByRole("button", { name: "Dismiss route warning", exact: true }).click();
    const canvas = page.locator(".session-flow-canvas");
    await canvas.evaluate((element) => element.scrollIntoView({ block: "end" }));
    const bounds = (await canvas.boundingBox())!;
    await page.mouse.click(bounds.x + bounds.width - 10, bounds.y + bounds.height - 10, { button: "right" });
    const menu = page.getByRole("menu", { name: "Canvas actions" });
    await expect(menu).toBeVisible();
    await expect(menu.getByRole("menuitem", { name: "Paste tool" })).toBeDisabled();
    await capture(page, `${theme}-enlarged-edge-menu`);
    const menuBounds = (await menu.boundingBox())!;
    expect.soft(menuBounds.x).toBeGreaterThanOrEqual(bounds.x);
    expect.soft(menuBounds.y).toBeGreaterThanOrEqual(bounds.y);
    expect.soft(menuBounds.x + menuBounds.width).toBeLessThanOrEqual(bounds.x + bounds.width);
    expect.soft(menuBounds.y + menuBounds.height).toBeLessThanOrEqual(bounds.y + bounds.height);
    await page.keyboard.press("Escape");
    await expect(menu).toBeHidden();
    await expect(canvas).toBeFocused();
  });

  test(`open canvas menu adapts to text and canvas resizing in ${theme}`, async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await page.getByRole("button", { name: "Dismiss route warning", exact: true }).click();
    const node = page.getByTestId("rf__node-voice");
    await node.locator(".flow-node-title").click();
    await node.focus();
    await page.keyboard.press("Control+c");
    // The "copied" message floats over this corner of the canvas, and its Dismiss button keeps its own clicks.
    await page.getByRole("button", { name: "Dismiss message", exact: true }).click();
    const canvas = page.locator(".session-flow-canvas");
    const bounds = (await canvas.boundingBox())!;
    await page.mouse.click(bounds.x + bounds.width - 10, bounds.y + bounds.height - 10, { button: "right" });
    const menu = page.getByRole("menu", { name: "Canvas actions" });
    const paste = menu.getByRole("menuitem", { name: "Paste tool" });
    await expect(paste).toBeFocused();
    await page.addStyleTag({ content: ".canvas-context-menu button { font-size: 32px; }" });
    await page.setViewportSize({ width: 1024, height: 720 });
    await expect(async () => {
      const outer = (await canvas.boundingBox())!;
      const inner = (await menu.boundingBox())!;
      expect(inner.x).toBeGreaterThanOrEqual(outer.x);
      expect(inner.y).toBeGreaterThanOrEqual(outer.y);
      expect(inner.x + inner.width).toBeLessThanOrEqual(outer.x + outer.width);
      expect(inner.y + inner.height).toBeLessThanOrEqual(outer.y + outer.height);
    }).toPass();
    await expect(paste).toBeFocused();
    await capture(page, `${theme}-resized-enlarged-edge-menu`);
    await page.keyboard.press("Enter");
    await expect(menu).toBeHidden();
    await expect(page.getByTestId("rf__node-voice-copy-1")).toBeVisible();
    await expect(canvas).toBeFocused();
  });

  test(`save language is consistent across the workbench in ${theme}`, async ({ page }) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await expect(page.locator(".topbar").getByRole("button", { name: "Save", exact: true })).toBeVisible();
    const forbidden = page
      .locator(".main-content :visible")
      .getByText(/Plan changes|Commit changes|Plan and commit|Review and plan the changes/i);
    await expect(forbidden).toHaveCount(0);
    await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
    await page.getByRole("tab", { name: "Properties", exact: true }).click();
    await expect(page.locator(".main-content > .inspector")).toContainText(/Privacy mute (silences|is on)/);
    await expect(
      page.locator(".main-content > .inspector").getByText(/Plan changes|Commit changes|Plan and commit/i),
    ).toHaveCount(0);
    await capture(page, `${theme}-save-language`);
  });

  test(`quarantined plugin health remains legible in ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ theme, session }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, {
          __routeFixtureSession: session,
          __routeFixtureRunning: true,
          __routeFixtureTelemetry: [
            {
              nodeId: "plugin-1",
              kind: "plugin",
              meter: null,
              processor: null,
              plugin: { state: "quarantined", failureCount: 3, outputMisses: 0, inputDrops: 0 },
            },
          ],
        });
      },
      { theme, session: quarantinedPluginSession },
    );
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const badge = page.getByTestId("rf__node-plugin-1").getByRole("alert");
    await expect(badge).toHaveText("Quarantined (3 failures)");
    await capture(page, `${theme}-plugin-quarantined`);
    const colors = await badge.evaluate((element) => {
      const style = getComputedStyle(element);
      const parse = (value: string) => value.match(/\d+(?:\.\d+)?/g)!.map(Number);
      const luminance = (color: number[]) =>
        color.slice(0, 3).reduce((sum, value, index) => {
          const component = value / 255;
          return (
            sum +
            (component <= 0.04045 ? component / 12.92 : ((component + 0.055) / 1.055) ** 2.4) *
              [0.2126, 0.7152, 0.0722][index]
          );
        }, 0);
      const foreground = parse(style.color);
      const background = parse(style.backgroundColor);
      const ratio = [luminance(foreground), luminance(background)].sort((a, b) => b - a);
      return {
        color: style.color,
        background: style.backgroundColor,
        ratio: (ratio[0] + 0.05) / (ratio[1] + 0.05),
        fontSize: style.fontSize,
      };
    });
    expect(colors.ratio).toBeGreaterThanOrEqual(4.5);
    expect(parseFloat(colors.fontSize)).toBeGreaterThanOrEqual(11);
  });

  test(`every built-in tool card has a visible icon in ${theme}`, async ({ page }) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const cards = page.locator(".tool-library-entry > .tool-card");
    const missing = await cards.evaluateAll((elements) =>
      elements
        .map((element) => ({
          label: element.querySelector("strong")?.textContent?.trim() ?? "",
          icon: element.querySelector<HTMLElement>(".tool-card-icon")?.textContent?.trim() ?? "",
          svg: Boolean(element.querySelector(".tool-card-icon svg")),
        }))
        .filter(({ icon, svg }) => !icon && !svg),
    );
    expect(missing).toEqual([]);
  });

  test(`route warning can be dismissed without moving the workspace in ${theme}`, async ({ page }) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    const warning = page.locator(".inactive-route-warning");
    const canvas = page.getByLabel("Signal-flow graph", { exact: true });
    await expect(warning).toBeVisible();
    const bounds = await canvas.boundingBox();
    await page.getByRole("button", { name: "Dismiss route warning", exact: true }).click();
    await expect(warning).toBeHidden();
    expect(await canvas.boundingBox()).toEqual(bounds);
    await page.getByRole("tab", { name: "Session", exact: true }).click();
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    await expect(warning).toBeHidden();
    await page
      .locator(".tool-card")
      .filter({ has: page.getByText("Gain", { exact: true }) })
      .click();
    await expect(warning).toBeVisible();
    await expect(warning).toContainText("Gain 1");
  });

  test(`canvas menu follows the theme and Escape restores focus in ${theme}`, async ({ page }) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    const node = page.getByTestId("rf__node-voice");
    await node.focus();
    await node.click({ button: "right" });
    const menu = page.getByRole("menu", { name: "Tool actions" });
    await expect(menu.getByRole("menuitem", { name: "Copy tool" })).toBeFocused();
    await capture(page, `${theme}-context-menu`);
    const menuStyles = await menu.evaluate((element) => {
      const style = getComputedStyle(element);
      const item = element.querySelector<HTMLElement>("[role='menuitem']")!;
      return { background: style.backgroundColor, foreground: getComputedStyle(item).color };
    });
    await page.keyboard.press("Escape");
    await expect(menu).toBeHidden();
    expect.soft(menuStyles.background).not.toBe("rgba(0, 0, 0, 0)");
    expect.soft(menuStyles.foreground).not.toBe(menuStyles.background);
    await expect(node).toBeFocused();
  });

  test(`loading is explicit and a failed tool search offers recovery in ${theme}`, async ({ page }) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __routeFixtureDelayMs: 1500 });
    }, theme);
    await page.goto("/route-harness.html");
    const loading = page.getByRole("main", { name: "Loading saved session" });
    await expect(loading).toBeVisible();
    await capture(page, `${theme}-loading`);
    await expect(loading).toHaveAttribute("aria-busy", "true");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const search = page.getByRole("searchbox", { name: "Find a tool" });
    await search.fill("no-such-built-in-tool");
    await capture(page, `${theme}-empty-search`);
    await expect(page.getByRole("status").filter({ hasText: "No built-in tools match" })).toBeVisible();
    await page.getByRole("button", { name: "Clear tool search", exact: true }).click();
    await expect(search).toHaveValue("");
    await expect(search).toBeFocused();
    await expect(page.locator(".tool-card").filter({ has: page.getByText("Gain", { exact: true }) })).toBeVisible();
  });

  for (const viewport of [
    { width: 1440, height: 1000 },
    { width: 1280, height: 720 },
  ]) {
    test(`review captures ${theme} ${viewport.width}`, async ({ page }) => {
      await page.setViewportSize(viewport);
      await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
      await page.goto("/route-harness.html");
      await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
      const prefix = `${theme}-${viewport.width}`;
      const audit: Record<string, unknown> = {};
      await capture(page, `${prefix}-tools`);
      audit.tools = await textContrast(page);
      await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
      await capture(page, `${prefix}-properties`);
      audit.properties = await textContrast(page);
      for (const tab of [
        "Session",
        "Status",
        "Apps",
        "Virtual Cables",
        "Appearance",
        "Recording",
        "AI assistants (MCP)",
        "API",
        "Logs and support",
        "Timing",
      ]) {
        await openPage(page, tab);
        await expect(page.getByRole("tabpanel").filter({ visible: true })).toBeVisible();
        await capture(page, `${prefix}-${tab.toLowerCase().replaceAll(" ", "-")}`);
        audit[tab] = await textContrast(page);
      }
      await writeFile(path.join(evidence, `${prefix}-contrast-candidates.json`), JSON.stringify(audit, null, 2));
    });
  }
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`secondary labels and guide links have readable contrast in ${theme}`, async ({ page }) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
    const failures = (await textContrast(page)).filter(
      (entry) =>
        entry.className === "badge" || entry.className.includes("eyebrow") || entry.text?.startsWith("Visual guide"),
    );
    expect(failures).toEqual([]);
    // Check metadata against both gradient endpoints: the solid-background
    // candidate collector cannot establish contrast over a gradient by itself.
    const nodeContrast = await page.locator(".session-flow-canvas").evaluate((canvas) => {
      const style = getComputedStyle(canvas);
      const colors = style.getPropertyValue("--node-bg").match(/#[0-9a-f]{3,8}\b/gi)!;
      const foreground = style.getPropertyValue("--node-muted").trim();
      const luminance = (hex: string) => {
        const expanded = hex.length === 4 ? "#" + [...hex.slice(1)].map((c) => c + c).join("") : hex;
        return [1, 3, 5].reduce((sum, offset, i) => {
          const c = parseInt(expanded.slice(offset, offset + 2), 16) / 255;
          return sum + (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4) * [0.2126, 0.7152, 0.0722][i];
        }, 0);
      };
      return colors.map((background) => {
        const l = [luminance(foreground), luminance(background)].sort((a, b) => b - a);
        return (l[0] + 0.05) / (l[1] + 0.05);
      });
    });
    expect(Math.min(...nodeContrast)).toBeGreaterThanOrEqual(4.5);
  });

  test(`navigation captions remain whole and focus visible in ${theme}`, async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    const tabs = page.getByRole("tab");
    await expect(tabs).toHaveCount(5);
    // Every individual word must fit its tab's content width; wrapping between
    // words is allowed. This catches Properties/Recording/Advanced splitting.
    const splitWords = await tabs.evaluateAll((tabs) =>
      tabs.flatMap((tab) => {
        const style = getComputedStyle(tab);
        const context = document.createElement("canvas").getContext("2d")!;
        context.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
        const width = tab.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
        return (tab.textContent ?? "")
          .trim()
          .split(/\s+/)
          .filter((word) => context.measureText(word).width > width + 1)
          .map((word) => ({ word, width }));
      }),
    );
    expect(splitWords).toEqual([]);
    await tabs.first().focus();
    await page.keyboard.press("End");
    await expect(page.getByRole("tab", { name: "Recording", exact: true })).toBeFocused();
    await capture(page, `${theme}-1280-keyboard-focus`);
  });
}
