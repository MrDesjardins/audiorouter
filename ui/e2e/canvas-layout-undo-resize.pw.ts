import { test, expect, type Page, type Locator } from "./browser-test";
import { fitCanvas } from "./workbench";
import { demoSession } from "../src/fixtures";

async function dragBy(page: Page, x: number, y: number, dx: number, dy: number) {
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.mouse.move(x + dx, y + dy, { steps: 8 });
  await page.mouse.up();
}

async function open(page: Page) {
  await page.addInitScript((session) => Object.assign(window, { __routeFixtureSession: session }), demoSession);
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await fitCanvas(page);
}

const addGroup = (page: Page) => page.getByRole("button", { name: "Group", exact: true }).click();

/** Two groups side by side, both selected. */
async function twoSelectedGroups(page: Page) {
  // Saved above the tools, so no card can cover a caption or a side.
  await page.addInitScript((key) => {
    const group = { name: "Group", color: "#5599dd", opacity: 5, fontSize: 18, y: -600, width: 400, height: 300 };
    localStorage.setItem(
      key,
      JSON.stringify([
        { ...group, id: "group-a", x: -40 },
        { ...group, id: "group-b", x: 500, width: 300, height: 200 },
      ]),
    );
  }, `audiorouter.ui.groups.${demoSession.id}`);
  await open(page);
  const first = page.locator(".visual-group-node").nth(0);
  const second = page.locator(".visual-group-node").nth(1);
  await first.locator(".canvas-group-caption").click();
  await second.locator(".canvas-group-caption").click({ modifiers: ["Control"] });
  await expect(first).toHaveClass(/selected/);
  await expect(second).toHaveClass(/selected/);
  return { first, second };
}

const box = async (locator: Locator) => (await locator.boundingBox())!;
const close = (a: number, b: number) => Math.abs(a - b) < 1;

test("dragging one selected group's side resizes every selected group the same way", async ({ page }) => {
  const { first, second } = await twoSelectedGroups(page);
  const [a, b] = [await box(first), await box(second)];
  await dragBy(page, b.x + b.width + 3, b.y + b.height - 40, 50, 0);
  const [a2, b2] = [await box(first), await box(second)];
  expect(b2.width - b.width).toBeGreaterThan(40);
  expect(close(a2.width - a.width, b2.width - b.width)).toBe(true);
  // Heights and left edges did not change.
  expect(close(a2.height, a.height) && close(b2.height, b.height)).toBe(true);
  expect(close(a2.x, a.x) && close(b2.x, b.x)).toBe(true);
  // The selection survives, so a second side can follow.
  await expect(first).toHaveClass(/selected/);
  await expect(second).toHaveClass(/selected/);
  const left = await box(first);
  await dragBy(page, left.x - 3, left.y + 30, -30, 0);
  const [a3, b3] = [await box(first), await box(second)];
  expect(a3.x).toBeLessThan(a2.x - 20);
  expect(close(a3.x - a2.x, b3.x - b2.x)).toBe(true);
  expect(close(a3.x + a3.width, a2.x + a2.width)).toBe(true);
});

test("a corner drag on one selected group moves both axes of all, as one undo step", async ({ page }) => {
  const { first, second } = await twoSelectedGroups(page);
  const [a, b] = [await box(first), await box(second)];
  const corner = await box(second.locator(".react-flow__resize-control.handle.bottom.right"));
  await dragBy(page, corner.x + corner.width / 2, corner.y + corner.height / 2, 40, 30);
  const [a2, b2] = [await box(first), await box(second)];
  expect(close(a2.width - a.width, b2.width - b.width)).toBe(true);
  expect(close(a2.height - a.height, b2.height - b.height)).toBe(true);
  expect(b2.width).toBeGreaterThan(b.width + 30);
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  const [a3, b3] = [await box(first), await box(second)];
  expect(close(a3.width, a.width) && close(b3.width, b.width)).toBe(true);
  await page.getByRole("button", { name: "Redo layout", exact: true }).click();
  expect(close((await box(first)).width, a2.width)).toBe(true);
});

test("a locked group stays put while the other selected group resizes", async ({ page }) => {
  const { first, second } = await twoSelectedGroups(page);
  await first.locator(".canvas-group-caption").click();
  await page.getByLabel("Lock group position and size").check();
  await second.locator(".canvas-group-caption").click({ modifiers: ["Control"] });
  const [a, b] = [await box(first), await box(second)];
  await dragBy(page, b.x + b.width + 3, b.y + b.height - 40, 50, 0);
  expect((await box(second)).width).toBeGreaterThan(b.width + 40);
  expect(close((await box(first)).width, a.width)).toBe(true);
});

test("Ctrl+Z, Ctrl+Y and the top-bar buttons undo a group resize", async ({ page }) => {
  await open(page);
  await addGroup(page);
  const group = page.locator(".visual-group-node").first();
  const undo = page.locator(".topbar").getByRole("button", { name: "Undo", exact: true });
  const redo = page.locator(".topbar").getByRole("button", { name: "Redo", exact: true });
  await expect(undo).toBeDisabled();
  const before = await box(group);
  await dragBy(page, before.x + before.width + 3, before.y + 10, 60, 0);
  const after = await box(group);
  expect(after.width).toBeGreaterThan(before.width + 30);
  await page.keyboard.press("Control+z");
  expect(close((await box(group)).width, before.width)).toBe(true);
  await page.keyboard.press("Control+y");
  expect(close((await box(group)).width, after.width)).toBe(true);
  await expect(undo).toBeEnabled();
  await undo.click();
  expect(close((await box(group)).width, before.width)).toBe(true);
  await expect(redo).toBeEnabled();
  await redo.click();
  expect(close((await box(group)).width, after.width)).toBe(true);
});

test("undo takes the newest change first, and a route edit is still undone after it", async ({ page }) => {
  await open(page);
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
  const inspector = page.locator(".main-content > .inspector");
  await inspector.getByRole("button", { name: "+3 dB", exact: true }).click();
  const gain = page.getByTestId("rf__node-voice").locator(".node-fader-readout strong");
  await expect(gain).toContainText("3.0");
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await addGroup(page);
  const group = page.locator(".visual-group-node").first();
  const before = await box(group);
  await dragBy(page, before.x + before.width + 3, before.y + 10, 60, 0);
  expect((await box(group)).width).toBeGreaterThan(before.width + 30);
  await page.mouse.click(5, 5);
  await page.keyboard.press("Control+z");
  expect(close((await box(group)).width, before.width)).toBe(true);
  await expect(gain).toContainText("3.0");
  await page.keyboard.press("Control+z");
  await expect(gain).not.toContainText("3.0");
  // Redo comes back in the opposite order.
  await page.keyboard.press("Control+y");
  await expect(gain).toContainText("3.0");
  expect(close((await box(group)).width, before.width)).toBe(true);
  await page.keyboard.press("Control+y");
  expect((await box(group)).width).toBeGreaterThan(before.width + 30);
});

test("the right panel's width undoes and redoes, by mouse, keyboard and double-click", async ({ page }) => {
  await open(page);
  const handle = page.getByRole("separator", { name: "Resize the right panel" });
  const width = async () => Number(await handle.getAttribute("aria-valuenow"));
  const start = await width();
  const undoLayout = page.getByRole("button", { name: "Undo layout", exact: true });
  await expect(undoLayout).toBeDisabled();
  const bounds = await box(handle);
  await dragBy(page, bounds.x + 6, bounds.y + 200, -80, 0);
  const dragged = await width();
  expect(dragged).toBeGreaterThan(start + 70);
  // One drag is one step, however many pixels it crossed.
  await expect(undoLayout).toBeEnabled();
  await undoLayout.click();
  expect(await width()).toBe(start);
  await expect(undoLayout).toBeDisabled();
  await page.keyboard.press("Control+y");
  expect(await width()).toBe(dragged);
  await handle.focus();
  await page.keyboard.press("ArrowLeft");
  expect(await width()).toBe(dragged + 24);
  await page.keyboard.press("Control+z");
  expect(await width()).toBe(dragged);
  await handle.dblclick();
  expect(await width()).toBe(400);
  await page.keyboard.press("Control+z");
  expect(await width()).toBe(dragged);
  // The restored width is what a restart would load.
  const stored = await page.evaluate(() => Object.entries(localStorage).find(([key]) => /sidebar/i.test(key))?.[1]);
  expect(stored).toContain(String(dragged));
});
