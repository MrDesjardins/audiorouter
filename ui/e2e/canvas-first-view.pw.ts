import { expect, test } from "./browser-test";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: the initial three-card route fits at 1440`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const canvas = page.locator(".session-flow-canvas");
    await expect
      .poll(async () => {
        const view = (await canvas.boundingBox())!;
        return page.locator(".react-flow__node").evaluateAll(
          (nodes, view) =>
            nodes
              .filter((node) => {
                const box = node.getBoundingClientRect();
                return (
                  box.left < view.x ||
                  box.right > view.x + view.width ||
                  box.top < view.y ||
                  box.bottom > view.y + view.height
                );
              })
              .map((node) => node.getAttribute("data-id")),
          view,
        );
      })
      .toEqual([]);
    const scale = await page
      .locator(".react-flow__viewport")
      .evaluate((element) => new DOMMatrixReadOnly(getComputedStyle(element).transform).a);
    expect(scale).toBeGreaterThan(0);
    expect(scale).toBeLessThanOrEqual(1);
    for (const id of ["mic", "headphones"]) {
      const card = page.getByTestId(`rf__node-${id}`);
      await expect(card.locator(".node-kind-family")).toHaveCount(0);
      await expect(card.locator(".node-state")).toHaveText("no device");
      await expect(card.locator(".node-used-by")).toBeEmpty();
    }
    await page.screenshot({ path: `../target/ui-quality-review/round-26/first-view-${theme}.png` });
  });
}

for (const width of [1280, 1000, 640]) {
  test(`opening at ${width}px shows the complete route without manual zoom`, async ({ page }) => {
    await page.setViewportSize({ width, height: 720 });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await expect
      .poll(async () => {
        const view = (await page.locator(".session-flow-canvas").boundingBox())!;
        return page.locator(".react-flow__node").evaluateAll(
          (nodes, view) =>
            nodes.every((node) => {
              const box = node.getBoundingClientRect();
              return (
                box.left >= view.x &&
                box.right <= view.x + view.width &&
                box.top >= view.y &&
                box.bottom <= view.y + view.height
              );
            }),
          view,
        );
      })
      .toBe(true);
    await page.screenshot({ path: `../target/ui-quality-review/round-26/first-view-${width}.png` });
    const map = (await page.locator(".canvas-minimap").boundingBox())!;
    for (const card of await page.locator(".react-flow__node").all()) {
      const box = (await card.boundingBox())!;
      expect(box.y >= map.y + map.height || box.x + box.width <= map.x || box.x >= map.x + map.width).toBe(true);
    }
  });
}
