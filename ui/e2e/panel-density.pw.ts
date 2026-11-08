import { expect, test } from "./browser-test";
import { openPage } from "./workbench";
import { mkdir, writeFile } from "node:fs/promises";
import { SETTINGS_PAGES } from "../src/Workbench";
import { appendLibraryNode } from "../src/draft";
import { demoSession } from "../src/fixtures";
import { libraryEntries } from "../src/library";
import type { Session } from "@audiorouter/contracts";

const pages = [
  "Tools",
  "Properties",
  "Session",
  "Status",
  "Recording",
  "Apps",
  "Timing",
  ...SETTINGS_PAGES.map(([, label]) => label),
];

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: expanded tool panels retain usable buttons and contained fields`, async ({ page }) => {
    test.setTimeout(90_000);
    const session = libraryEntries.reduce<Session>(
      (current, entry) => (entry.kind ? appendLibraryNode(current, entry.kind) : current),
      { ...demoSession, id: "density-tools", nodes: [], edges: [] },
    );
    await page.addInitScript(
      ({ theme, session }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session });
      },
      { theme, session },
    );
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await page.getByRole("button", { name: "Dismiss route warning", exact: true }).click();
    await page.getByRole("button", { name: "Route list", exact: true }).click();
    const list = page.locator(".canvas-keyboard-panel .graph-list");
    const inventory = [];
    for (const width of [1280, 640]) {
      await page.setViewportSize({ width, height: width === 640 ? 360 : 720 });
      for (const node of session.nodes) {
        const escaped = node.name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
        await list.getByRole("button", { name: new RegExp(`^${escaped} `) }).click();
        const panel = page.locator(".main-content > .inspector");
        await expect(panel.locator("#inspector-heading")).toHaveAttribute("aria-label", node.name);
        await panel.locator("details.property-section").evaluateAll((elements) => {
          for (const element of elements) (element as HTMLDetailsElement).open = true;
        });
        const issues = await panel.evaluate((panel) => {
          const outer = panel.getBoundingClientRect();
          return [...panel.querySelectorAll<HTMLElement>("button, input, select, textarea")].flatMap((element) => {
            const box = element.getBoundingClientRect();
            if (!box.width || !box.height || getComputedStyle(element).visibility !== "visible") return [];
            const overflow = box.left < outer.left - 1 || box.right > outer.right + 1;
            const small = element.tagName === "BUTTON" && (box.width < 24 || box.height < 24);
            const clipped =
              element.tagName === "BUTTON" &&
              (element.scrollWidth > element.clientWidth + 1 || element.scrollHeight > element.clientHeight + 1);
            return overflow || small || clipped
              ? [
                  {
                    label: element.getAttribute("aria-label") ?? element.textContent?.trim(),
                    overflow,
                    small,
                    clipped,
                    width: box.width,
                    height: box.height,
                  },
                ]
              : [];
          });
        });
        inventory.push({ kind: node.kind, width, issues });
        if (issues.length)
          await panel.screenshot({
            path: `../target/ui-quality-review/round-27/density-${node.kind}-${width}-${theme}.png`,
          });
      }
    }
    await mkdir("../target/ui-quality-review/round-27", { recursive: true });
    await writeFile(
      `../target/ui-quality-review/round-27/tool-density-${theme}.json`,
      JSON.stringify(inventory, null, 2),
    );
    expect(inventory.filter((entry) => entry.issues.length)).toEqual([]);
  });

  test(`${theme}: workbench pages contain fields and actions at ordinary and narrow widths`, async ({ page }) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const inventory = [];
    for (const width of [1280, 640]) {
      await page.setViewportSize({ width, height: width === 640 ? 360 : 720 });
      for (const name of pages) {
        await openPage(page, name);
        const panel =
          name === "Properties"
            ? page.locator(".main-content > .inspector")
            : page.locator(".right-workbench").getByRole("tabpanel").filter({ visible: true });
        await expect(panel).toBeVisible();
        const audit = await panel.evaluate((panel) => {
          const outer = panel.getBoundingClientRect();
          const controls = [...panel.querySelectorAll<HTMLElement>("button, input, select, textarea")].filter(
            (element) => element.getClientRects().length && getComputedStyle(element).visibility === "visible",
          );
          const overflow = controls.flatMap((element) => {
            const rect = element.getBoundingClientRect();
            return rect.left < outer.left - 1 || rect.right > outer.right + 1
              ? [
                  {
                    label: element.getAttribute("aria-label") ?? element.textContent?.trim(),
                    left: rect.left,
                    right: rect.right,
                    outerLeft: outer.left,
                    outerRight: outer.right,
                  },
                ]
              : [];
          });
          const buttons = controls.filter((element) => element.tagName === "BUTTON");
          const smallTargets = buttons.flatMap((element) => {
            const rect = element.getBoundingClientRect();
            return rect.width < 24 || rect.height < 24
              ? [
                  {
                    label: element.getAttribute("aria-label") ?? element.textContent?.trim(),
                    width: rect.width,
                    height: rect.height,
                  },
                ]
              : [];
          });
          const clippedCaptions = buttons
            .filter(
              (element) =>
                element.scrollWidth > element.clientWidth + 1 || element.scrollHeight > element.clientHeight + 1,
            )
            .map((element) => element.getAttribute("aria-label") ?? element.textContent?.trim());
          return {
            panelWidth: outer.width,
            contentHeight: panel.scrollHeight,
            overflow,
            smallTargets,
            clippedCaptions,
            buttons: buttons.map((element) => ({
              label: element.textContent?.trim(),
              height: element.getBoundingClientRect().height,
            })),
          };
        });
        inventory.push({ theme, width, page: name, ...audit });
        expect.soft(audit.overflow, `${name} at ${width}`).toEqual([]);
        expect.soft(audit.smallTargets, `${name} targets at ${width}`).toEqual([]);
        expect.soft(audit.clippedCaptions, `${name} button captions at ${width}`).toEqual([]);
      }
    }
    const evidence = "../target/ui-quality-review/round-27";
    await mkdir(evidence, { recursive: true });
    await writeFile(`${evidence}/density-${theme}.json`, JSON.stringify(inventory, null, 2));
  });
}
