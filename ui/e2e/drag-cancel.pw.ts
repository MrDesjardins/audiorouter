import { expect, test } from "./browser-test";
import { demoSession } from "../src/fixtures";
import { fitCanvas } from "./workbench";

// Drag/drop criterion: cancelling a drag must leave the route as it was.
test.beforeEach(async ({ page }) => {
  await page.addInitScript((session) => Object.assign(window, { __routeFixtureSession: session }), demoSession);
  await page.setViewportSize({ width: 1500, height: 950 });
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await fitCanvas(page);
});

test("Escape during a connection drag adds no connection", async ({ page }) => {
  const edges = page.locator(".react-flow__edges .react-flow__edge");
  const before = await edges.count();
  const source = page.locator(".react-flow__node .react-flow__handle.source").first();
  const target = page.locator(".react-flow__node .react-flow__handle.target").last();
  const from = (await source.boundingBox())!;
  const to = (await target.boundingBox())!;
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.down();
  await page.mouse.move(from.x + 40, from.y + 10, { steps: 4 });
  await page.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 8 });
  await page.keyboard.press("Escape");
  await page.mouse.up();
  await expect(edges).toHaveCount(before);
  await expect(page.getByRole("button", { name: "Save", exact: true })).toBeDisabled();
});

test("Escape during a card drag puts the card back", async ({ page }) => {
  const node = page.getByTestId("rf__node-mic");
  const start = await node.evaluate((element) => (element as HTMLElement).style.transform);
  const box = (await node.locator(".flow-node-title").boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + 2, box.y + box.height / 2 + 2);
  await page.mouse.move(box.x + 220, box.y + 160, { steps: 10 });
  await page.keyboard.press("Escape");
  await page.mouse.up();
  await expect.poll(() => node.evaluate((element) => (element as HTMLElement).style.transform)).toBe(start);
});

test("the same connection drag without Escape does connect (control)", async ({ page }) => {
  const edges = page.locator(".react-flow__edges .react-flow__edge");
  const before = await edges.count();
  const source = page.locator(".react-flow__node .react-flow__handle.source").first();
  const target = page.locator(".react-flow__node .react-flow__handle.target").last();
  const from = (await source.boundingBox())!;
  const to = (await target.boundingBox())!;
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.down();
  await page.mouse.move(from.x + 40, from.y + 10, { steps: 4 });
  await page.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 8 });
  await page.mouse.up();
  await expect(edges).toHaveCount(before + 1);
});
