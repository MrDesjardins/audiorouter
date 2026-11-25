import { expect, test, type Page } from "./browser-test";
import { demoSession } from "../src/fixtures";
import { fitCanvas } from "./workbench";

// A 1280x720 window at 200 % browser/display zoom leaves a 640x360 CSS
// viewport and a 2x pixel ratio: the physical screen is narrow, the layout
// sees half the logical space. This exercises the geometry that zoom can
// squeeze (top bar, tabs, canvas), not the rendered pixel density.
async function atTwoHundredPercent(page: Page) {
  const session = await page.context().newCDPSession(page);
  await session.send("Emulation.setDeviceMetricsOverride", {
    width: 640,
    height: 360,
    deviceScaleFactor: 2,
    mobile: false,
  });
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: a narrow window at 200% keeps the top bar, tabs and canvas usable`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ theme, session }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session });
      },
      { theme, session: demoSession },
    );
    await atTwoHundredPercent(page);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");

    // Confirm the emulation, so a broken harness cannot pass as an untested state.
    expect(await page.evaluate(() => ({ width: window.innerWidth, ratio: window.devicePixelRatio }))).toEqual({
      width: 640,
      ratio: 2,
    });

    const dismiss = page.getByRole("button", { name: "Dismiss route warning", exact: true });
    if (await dismiss.isVisible()) await dismiss.click();

    // The narrow layout stacks and scrolls the page (the workbench sits below
    // the canvas), so a control may begin below the fold. Nothing may leave the
    // window sideways, though: a wrapped tab row is allowed, a clipped one is not.
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    const outside = await page.evaluate(() => {
      const boxes = [
        ...document.querySelectorAll<HTMLElement>(".topbar button"),
        ...document.querySelectorAll<HTMLElement>('[aria-label="Right sidebar"] [role="tab"]'),
      ];
      return boxes.flatMap((element) => {
        const box = element.getBoundingClientRect();
        if (box.width === 0 || box.height === 0) return [];
        const name = element.textContent?.trim() || element.getAttribute("aria-label") || element.tagName;
        return box.left < -1 || box.right > window.innerWidth + 1 ? [name] : [];
      });
    });
    expect(outside).toEqual([]);
    await page.locator(".workbench-tabs").scrollIntoViewIfNeeded();
    await expect(page.getByRole("tab", { name: "Recording", exact: true })).toBeInViewport({ ratio: 1 });

    // Text at the app's 11 px floor is not clipped by its own control.
    const clipped = await page.evaluate(() => {
      const labels = [
        ...document.querySelectorAll<HTMLElement>(".topbar button, .workbench-tabs [role=tab], .flow-node-title"),
      ];
      return labels
        .filter((element) => {
          const style = getComputedStyle(element);
          return style.visibility !== "hidden" && element.getBoundingClientRect().width > 0;
        })
        .filter((element) => element.scrollWidth > element.clientWidth + 1)
        .map((element) => element.textContent?.trim() || element.tagName);
    });
    expect(clipped).toEqual([]);

    // The canvas renderer fills the narrow canvas: a 0 px renderer hides every
    // tool while the cards still have dimensions (iteration 13).
    const canvas = page.locator(".session-flow-canvas");
    await canvas.scrollIntoViewIfNeeded();
    const renderer = await canvas.locator(".react-flow").evaluate((element) => element.getBoundingClientRect().height);
    expect(renderer).toBeGreaterThan(0);
    await fitCanvas(page);
    await expect(page.getByTestId("rf__node-voice")).toBeVisible();

    // Properties remains reachable from the canvas in this layout.
    await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
    await page.getByRole("tab", { name: "Properties", exact: true }).click();
    await expect(page.locator(".main-content > .inspector")).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath(`${theme}-narrow-200-percent.png`), fullPage: false });
  });
}
