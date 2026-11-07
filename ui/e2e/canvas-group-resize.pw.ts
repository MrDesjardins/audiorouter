import { test, expect, type Page } from "./browser-test";
import { fitCanvas } from "./workbench";
import { demoSession } from "../src/fixtures";

type Box = { x: number; y: number; width: number; height: number };

async function openGroup(page: Page) {
  await page.addInitScript((session) => {
    Object.assign(window, { __routeFixtureSession: session });
  }, demoSession);
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/route-harness.html");
  await fitCanvas(page);
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.getByRole("button", { name: "Group", exact: true }).click();
  const group = page.locator(".visual-group-node");
  await expect(group.locator(".react-flow__resize-control").first()).toBeVisible();
  return group;
}

/** Zooms with the canvas button, then waits for the group to stop moving. */
async function zoomOut(page: Page, presses: number) {
  const button = page.getByRole("button", { name: "Zoom Out" });
  for (let press = 0; press < presses; press += 1) await button.click();
  const group = page.locator(".visual-group-node");
  let last = "";
  await expect(async () => {
    const now = JSON.stringify(await group.boundingBox());
    const settled = now === last;
    last = now;
    expect(settled).toBe(true);
  }).toPass({ intervals: [150] });
}

const sides = {
  left: { axis: "x", sign: -1 },
  right: { axis: "x", sign: 1 },
  top: { axis: "y", sign: -1 },
  bottom: { axis: "y", sign: 1 },
} as const;

/** A point on the middle of one side, `offset` screen pixels outward (negative means inside the group). */
function grabPoint(box: Box, side: keyof typeof sides, offset: number) {
  const { axis, sign } = sides[side];
  if (axis === "x") {
    return { x: (side === "left" ? box.x : box.x + box.width) + sign * offset, y: box.y + box.height / 2 };
  }
  return { x: box.x + box.width / 2, y: (side === "top" ? box.y : box.y + box.height) + sign * offset };
}

/** Like grabPoint, but on a stretch of the side no tool card covers: cards deliberately stay on top of groups. */
async function freeGrabPoint(page: Page, box: Box, side: keyof typeof sides, offset: number) {
  const middle = grabPoint(box, side, offset);
  const horizontalSide = sides[side].axis === "y";
  const length = horizontalSide ? box.width : box.height;
  for (const fraction of [0, 0.15, -0.15, 0.3, -0.3, 0.4, -0.4]) {
    const point = {
      x: middle.x + (horizontalSide ? fraction * length : 0),
      y: middle.y + (horizontalSide ? 0 : fraction * length),
    };
    const free = await page.evaluate(
      ([x, y, name]) => document.elementFromPoint(x, y)?.closest(`.react-flow__resize-control.line.${name}`) != null,
      [point.x, point.y, side] as const,
    );
    if (free) return point;
  }
  throw new Error(`No part of the ${side} side can be grabbed ${offset} px from the frame`);
}

for (const presses of [0, 3]) {
  for (const side of Object.keys(sides) as (keyof typeof sides)[]) {
    // Both ends of the grab zone: just outside the frame, and just inside it, which is what remains
    // when a neighbouring group covers the outer part.
    for (const offset of [6, -4]) {
      test(`dragging the ${side} side ${offset > 0 ? "from outside" : "from inside"} after ${presses} zoom-outs resizes only that side`, async ({
        page,
      }) => {
        const group = await openGroup(page);
        if (presses) await zoomOut(page, presses);
        const before = (await group.boundingBox())!;
        const nodesBefore = await page.getByTestId("rf__node-voice").boundingBox();
        const start = await freeGrabPoint(page, before, side, offset);
        const { axis, sign } = sides[side];
        await page.mouse.move(start.x, start.y);
        await page.mouse.down();
        await page.mouse.move(start.x + (axis === "x" ? sign * 40 : 0), start.y + (axis === "y" ? sign * 40 : 0), {
          steps: 8,
        });
        await page.mouse.up();
        const after = (await group.boundingBox())!;
        const grew = axis === "x" ? after.width - before.width : after.height - before.height;
        // The side lands on the 24 px canvas grid, so the result is within one step of the drag.
        expect(grew).toBeGreaterThan(10);
        expect(grew).toBeLessThan(70);
        // The other axis does not change, and the opposite side stays where it was.
        expect(axis === "x" ? after.height : after.width).toBeCloseTo(axis === "x" ? before.height : before.width, 0);
        if (side === "left") expect(after.x + after.width).toBeCloseTo(before.x + before.width, 0);
        if (side === "right") expect(after.x).toBeCloseTo(before.x, 0);
        if (side === "top") expect(after.y + after.height).toBeCloseTo(before.y + before.height, 0);
        if (side === "bottom") expect(after.y).toBeCloseTo(before.y, 0);
        // Resizing a frame leaves the tools alone.
        expect(await page.getByTestId("rf__node-voice").boundingBox()).toEqual(nodesBefore);
      });
    }
  }
}

test("a side's grab zone has the same screen size at any zoom", async ({ page }) => {
  const group = await openGroup(page);
  const thickness = () =>
    group.locator(".react-flow__resize-control.line.left").evaluate((element) => element.getBoundingClientRect().width);
  const atStart = await thickness();
  expect(atStart).toBeGreaterThanOrEqual(12);
  await zoomOut(page, 3);
  expect(Math.abs((await thickness()) - atStart)).toBeLessThanOrEqual(0.5);
});

test("the caption, background and corners keep their own drags next to the side zones", async ({ page }) => {
  const group = await openGroup(page);
  await zoomOut(page, 3);
  // Caption: moves the group without resizing it, even zoomed out where the caption is close to the frame.
  const caption = (await group.locator(".canvas-group-caption").boundingBox())!;
  const before = (await group.boundingBox())!;
  await page.mouse.move(caption.x + caption.width / 2, caption.y + 2);
  await page.mouse.down();
  await page.mouse.move(caption.x + caption.width / 2 + 30, caption.y + 22, { steps: 6 });
  await page.mouse.up();
  const moved = (await group.boundingBox())!;
  expect(moved.width).toBeCloseTo(before.width, 0);
  expect(moved.height).toBeCloseTo(before.height, 0);
  expect(moved.x - before.x).toBeGreaterThan(20);
  // Corner: still changes both directions.
  const corner = (await group.locator(".react-flow__resize-control.handle.bottom.right").boundingBox())!;
  await page.mouse.move(corner.x + corner.width / 2, corner.y + corner.height / 2);
  await page.mouse.down();
  await page.mouse.move(corner.x + 40, corner.y + 30, { steps: 6 });
  await page.mouse.up();
  const resized = (await group.boundingBox())!;
  expect(resized.width).toBeGreaterThan(moved.width + 25);
  expect(resized.height).toBeGreaterThan(moved.height + 20);
});

test("a locked group offers no side zones", async ({ page }) => {
  const group = await openGroup(page);
  await page.getByLabel("Lock group position and size").check();
  await expect(group.locator(".react-flow__resize-control")).toHaveCount(0);
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`a hovered side is highlighted in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    const group = await openGroup(page);
    const before = (await group.boundingBox())!;
    const point = await freeGrabPoint(page, before, "right", 2);
    await page.mouse.move(point.x, point.y);
    const line = group.locator(".react-flow__resize-control.line.right");
    await expect(line).toHaveCSS("cursor", "ew-resize");
    const width = await line.evaluate((element) => getComputedStyle(element, "::after").width);
    expect(Number.parseFloat(width)).toBeGreaterThanOrEqual(2.5);
    await page.screenshot({ path: testInfo.outputPath(`group-side-hover-${theme}.png`) });
  });
}
