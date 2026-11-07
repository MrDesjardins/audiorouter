import { test, expect } from "./browser-test";
import { demoSession } from "../src/fixtures";

test("the Soundboard panel has one scrollbar, not a second one inside its help", async ({ page }) => {
  const session = {
    ...demoSession,
    nodes: demoSession.nodes.map((n) => (n.id === "voice" ? { ...n, kind: "soundboard", parameters: {} } : n)),
  };
  await page.addInitScript((value) => Object.assign(window, { __routeFixtureSession: value }), session);
  await page.setViewportSize({ width: 1280, height: 832 });
  await page.goto("/route-harness.html");
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
  await expect(page.locator(".soundboard-editor-status")).toBeVisible();
  const nested = await page.evaluate(() =>
    Array.from(document.querySelectorAll(".main-content > .inspector *"))
      .filter((element) => /auto|scroll/.test(getComputedStyle(element).overflowY))
      .filter((element) => element.scrollHeight > element.clientHeight + 1)
      .map((element) => element.className),
  );
  expect(nested).toEqual([]);
});

// The owner's rule: the workspace sits in the window with one gap on all five sides that frame it:
// window to Canvas (left and bottom), header to both panels (top), Canvas to the right panel (middle),
// and right panel to window.
const sizes = [
  { width: 1280, height: 832 },
  { width: 1440, height: 1000 },
  { width: 1920, height: 1080 },
];

for (const theme of ["dark", "light", "high-contrast"]) {
  for (const size of sizes) {
    test(`workspace gaps are equal at ${size.width}x${size.height} in ${theme}`, async ({ page }, testInfo) => {
      await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
      await page.setViewportSize(size);
      await page.goto("/route-harness.html");
      await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
      const measure = () =>
        page.evaluate(() => {
          const box = (selector: string) => document.querySelector(selector)!.getBoundingClientRect();
          const canvas = box(".main-content > .canvas-panel");
          const sidebar = box(".main-content > .right-workbench");
          const header = box(".topbar");
          return {
            left: canvas.left,
            right: document.documentElement.clientWidth - sidebar.right,
            top: canvas.top - header.bottom,
            topRight: sidebar.top - header.bottom,
            bottom: window.innerHeight - canvas.bottom,
            bottomRight: window.innerHeight - sidebar.bottom,
            middle: sidebar.left - canvas.right,
          };
        });
      const expectEqual = (gaps: Record<string, number>) => {
        for (const [side, gap] of Object.entries(gaps)) {
          expect(Math.abs(gap - gaps.left), `${side} gap ${gap} against left ${gaps.left}`).toBeLessThanOrEqual(0.5);
        }
      };
      expectEqual(await measure());
      // The Properties tab has its own grid rows, so it gets its own measurement.
      await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
      await page.getByRole("tab", { name: "Properties", exact: true }).click();
      await expect(page.locator(".main-content > .inspector")).toBeVisible();
      expectEqual(await measure());
      await page.screenshot({ path: testInfo.outputPath(`workspace-${theme}-${size.width}.png`) });
    });
  }
}
