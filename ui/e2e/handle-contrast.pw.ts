import { expect, test } from "./browser-test";
import { fitCanvas } from "./workbench";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { mkdir, writeFile } from "node:fs/promises";

// Use the PNG decoder already bundled with our pinned Playwright test runtime.
const require = createRequire(import.meta.url);
const { PNG } = require(join(dirname(require.resolve("playwright-core/package.json")), "lib/utilsBundle.js")) as {
  PNG: { sync: { read: (buffer: Buffer) => { width: number; height: number; data: Buffer } } };
};
const luminance = (rgb: number[]) =>
  rgb.reduce((sum, value, index) => {
    const c = value / 255;
    return sum + (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4) * [0.2126, 0.7152, 0.0722][index];
  }, 0);
const contrast = (a: number[], b: number[]) => {
  const values = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (values[0] + 0.05) / (values[1] + 0.05);
};

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: visible handles contrast with the actual scene`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await fitCanvas(page);
    const results: Array<{
      state: string;
      direction: string;
      fill: number[];
      border: number[];
      ratio: number;
      sides: number[];
      changed: number;
    }> = [];
    for (const state of ["idle", "from-output"]) {
      const direction = state === "idle" ? "source" : "target";
      if (state === "from-output") {
        const output = page.getByTestId("rf__node-voice").locator('.source[data-debug-side="right"]');
        const box = (await output.boundingBox())!;
        await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
        await page.mouse.down();
        await page.mouse.move(box.x + box.width + 20, box.y + box.height / 2, { steps: 3 });
      }
      await expect(page.locator(".session-flow-canvas")).toHaveAttribute("data-connection-mode", state);
      const inactive = state === "idle" ? "target" : "source";
      await expect(page.locator(`.react-flow__handle[data-debug-direction="${inactive}"]`).first()).toHaveCSS(
        "opacity",
        "0",
      );
      const handles = page.locator(`.react-flow__handle[data-debug-direction="${direction}"]`);
      const boxes = await handles.evaluateAll((elements) =>
        elements.map((element) => {
          const rect = element.getBoundingClientRect();
          return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
        }),
      );
      const shot = PNG.sync.read(
        await page.screenshot({ path: `../target/ui-quality-review/round-25/handles-${state}-${theme}.png` }),
      );
      const hidden = await page.addStyleTag({ content: ".react-flow__handle { visibility: hidden !important; }" });
      const scene = PNG.sync.read(await page.screenshot());
      await hidden.evaluate((element) => element.parentNode?.removeChild(element));
      expect(
        await handles.evaluateAll((elements) =>
          elements.map((element) => {
            const rect = element.getBoundingClientRect();
            return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
          }),
        ),
      ).toEqual(boxes);
      const pixel = (image: typeof shot, x: number, y: number) => [
        ...image.data.subarray(
          (Math.round(y) * image.width + Math.round(x)) * 4,
          (Math.round(y) * image.width + Math.round(x)) * 4 + 3,
        ),
      ];
      for (const box of boxes) {
        const x = box.x + box.width / 2,
          y = box.y + box.height / 2;
        const fill = pixel(shot, x, y),
          border = pixel(shot, box.x + box.width - 1.5, y);
        // A handle can remain distinct through its border even when its fill
        // matches a card outline underneath. Audit every cardinal boundary,
        // including the rasterized border and immediately adjacent scene.
        const sides = [
          [1, 0],
          [-1, 0],
          [0, 1],
          [0, -1],
        ].map(([dx, dy]) => {
          const radius = (dx ? box.width : box.height) / 2;
          const candidates = [];
          for (let offset = -2; offset <= 2; offset += 0.5) {
            const sx = x + dx * (radius + offset),
              sy = y + dy * (radius + offset);
            candidates.push(contrast(fill, pixel(shot, sx, sy)));
            candidates.push(contrast(fill, pixel(scene, sx, sy)));
          }
          return Math.max(...candidates);
        });
        const ratio = Math.min(...sides);
        let changed = 0;
        for (let dx = -2; dx <= 2; dx++)
          for (let dy = -2; dy <= 2; dy++) {
            const foreground = pixel(shot, x + dx, y + dy),
              background = pixel(scene, x + dx, y + dy);
            changed = Math.max(changed, ...foreground.map((value, index) => Math.abs(value - background[index])));
          }
        results.push({ state, direction, fill, border, ratio, sides, changed });
      }
      if (state === "from-output") await page.mouse.up();
    }
    await mkdir("../target/ui-quality-review/round-25", { recursive: true });
    await writeFile(
      `../target/ui-quality-review/round-25/handle-contrast-${theme}.json`,
      JSON.stringify(results, null, 2),
    );
    expect(results.length).toBeGreaterThan(8);
    expect(results.filter((result) => result.ratio < 3)).toEqual([]);
    expect(results.filter((result) => result.changed < 10)).toEqual([]);
  });
}
