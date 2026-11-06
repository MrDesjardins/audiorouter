import { openPage } from "./workbench";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { test, expect } from "./browser-test";

const TABS = [
  "Tools",
  "Properties",
  "Timing",
  "Session",
  "Status",
  "Apps",
  "Virtual Cables",
  "Appearance",
  "Recording",
  "AI assistants (MCP)",
  "API",
  "Logs and support",
];

const ALLOWED_SIZES = new Set(["11px", "12px", "14px", "16px", "20px", "24px"]);

// Live numbers must not move as their digits change (UI-17): every digit has one width.
test("live numbers keep one width whichever digits they show", async ({ page }) => {
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  const widths = await page.evaluate(() => {
    const measure = (text: string, parent: Element) => {
      const probe = document.createElement("span");
      probe.textContent = text;
      probe.style.position = "absolute";
      parent.append(probe);
      const width = probe.getBoundingClientRect().width;
      probe.remove();
      return width;
    };
    const parent = document.querySelector(".app-shell")!;
    return { ones: measure("111.1", parent), eights: measure("888.8", parent) };
  });
  expect(widths.ones).toBe(widths.eights);
  const family = await page.evaluate(() => getComputedStyle(document.body).fontFamily);
  expect(family).toContain("Segoe UI Variable Text");
  expect(family).not.toContain("Inter");
});

// Measures rendered text across every tab: nothing below the review's 11 px floor, and the same
// kind of text (running paragraphs) at the same size everywhere.
test("every workbench tab keeps a readable, consistent type scale", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  const inventory: Record<string, Record<string, string[]>> = {};
  const tooSmall: string[] = [];
  const offScale: string[] = [];
  for (const tab of TABS) {
    await openPage(page, tab);
    await page.waitForTimeout(150);
    const found = await page.evaluate(() => {
      const sizes: Record<string, string[]> = {};
      for (const root of document.querySelectorAll(".right-workbench, .main-content > .inspector")) {
        const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
        for (let text = walker.nextNode(); text; text = walker.nextNode()) {
          const value = text.textContent?.trim();
          const owner = text.parentElement;
          if (!value || !owner || owner.closest("svg, option, [aria-hidden='true']")) continue;
          const bounds = owner.getBoundingClientRect();
          if (bounds.width === 0 || bounds.height === 0) continue;
          const size = getComputedStyle(owner).fontSize;
          const tag = owner.tagName.toLowerCase();
          (sizes[size] ??= []).push(`${tag}.${owner.className}: ${value.slice(0, 40)}`);
        }
      }
      return sizes;
    });
    inventory[tab] = found;
    for (const [size, samples] of Object.entries(found)) {
      if (Number.parseFloat(size) < 11) tooSmall.push(`${tab} ${size}: ${samples[0]}`);
      if (!ALLOWED_SIZES.has(size)) offScale.push(`${tab} ${size}: ${samples[0]}`);
      // Running paragraphs use the body or help size of the shared scale, whichever the panel.
      if (size !== "14px" && size !== "12px")
        for (const sample of samples) if (/^p\.(?!.*eyebrow)/.test(sample)) offScale.push(`${tab} ${size}: ${sample}`);
    }
  }
  const directory = path.resolve("../target/ui-quality-review/iteration-14-type");
  await mkdir(directory, { recursive: true });
  await writeFile(path.join(directory, "inventory.json"), JSON.stringify(inventory, null, 1));
  expect(tooSmall).toEqual([]);
  expect(offScale).toEqual([]);
});
