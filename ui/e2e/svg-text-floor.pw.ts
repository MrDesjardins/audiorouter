import { expect, test } from "./browser-test";
import type { Session } from "@audiorouter/contracts";
import { appendLibraryNode } from "../src/draft";
import { demoSession } from "../src/fixtures";
import { libraryEntries } from "../src/library";
import { mkdir, writeFile } from "node:fs/promises";

const session = libraryEntries.reduce<Session>(
  (current, entry) => (entry.kind ? appendLibraryNode(current, entry.kind) : current),
  { ...demoSession, id: "svg-type-audit", nodes: [], edges: [] },
);

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: SVG labels keep the 11 px rendered floor across addable tools`, async ({ page }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.addInitScript((value) => Object.assign(window, { __routeFixtureSession: value }), session);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await page.getByRole("button", { name: "Dismiss route warning", exact: true }).click();
    await page.getByRole("button", { name: "Route list", exact: true }).click();
    const list = page.locator(".canvas-keyboard-panel .graph-list");
    const inventory: Array<{
      kind: string;
      width: number;
      text: string;
      className: string;
      size: number;
      scale: number;
      pixels: number;
      clipped: boolean;
    }> = [];
    for (const width of [1280, 640]) {
      await page.setViewportSize({ width, height: 720 });
      for (const node of session.nodes) {
        const escaped = node.name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
        await list.getByRole("button", { name: new RegExp(`^${escaped} `) }).click();
        const panel = page.locator(".main-content > .inspector");
        await expect(panel.locator("#inspector-heading")).toHaveAttribute("aria-label", node.name);
        await panel.locator("details.property-section").evaluateAll((elements) => {
          for (const element of elements) (element as HTMLDetailsElement).open = true;
        });
        const labels = await panel.locator("svg text").evaluateAll((elements) =>
          elements.flatMap((element) => {
            const text = element as SVGTextElement;
            const box = text.getBoundingClientRect();
            const matrix = text.getScreenCTM();
            const chart = text.ownerSVGElement?.getBoundingClientRect();
            if (!matrix || box.width === 0 || box.height === 0 || !text.textContent?.trim()) return [];
            const size = parseFloat(getComputedStyle(text).fontSize);
            const scale = Math.hypot(matrix.c, matrix.d);
            return [
              {
                text: text.textContent.trim(),
                className: text.getAttribute("class") ?? "",
                size,
                scale,
                pixels: size * scale,
                clipped:
                  !!chart &&
                  (box.left < chart.left - 1 ||
                    box.right > chart.right + 1 ||
                    box.top < chart.top - 1 ||
                    box.bottom > chart.bottom + 1),
                edges: chart
                  ? [box.left - chart.left, chart.right - box.right, box.top - chart.top, chart.bottom - box.bottom]
                  : [],
              },
            ];
          }),
        );
        inventory.push(...labels.map((label) => ({ kind: node.kind, width, ...label })));
        if (["compressor", "duck", "parametricEq"].includes(node.kind)) {
          await panel.evaluate((element) => {
            element.scrollTop = 0;
          });
          await panel.screenshot({
            path: `../target/ui-quality-review/round-25/svg-${node.kind}-${width}-${theme}.png`,
          });
          const charts = panel.locator("svg:has(text)");
          for (let index = 0; index < (await charts.count()); index++) {
            await charts.nth(index).screenshot({
              path: `../target/ui-quality-review/round-25/chart-${node.kind}-${index}-${width}-${theme}.png`,
            });
          }
        }
      }
    }
    await mkdir("../target/ui-quality-review/round-25", { recursive: true });
    await writeFile(
      `../target/ui-quality-review/round-25/svg-type-inventory-${theme}.json`,
      JSON.stringify(inventory, null, 2),
    );
    expect(inventory.length).toBeGreaterThan(30);
    expect(inventory.filter((label) => !Number.isFinite(label.pixels) || label.pixels < 10.99)).toEqual([]);
    expect(inventory.filter((label) => label.clipped)).toEqual([]);
  });
}
