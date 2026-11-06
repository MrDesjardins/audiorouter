import type { Page } from "@playwright/test";
import { expect, test } from "./browser-test";

const GRID = 24;
const zoom = (page: Page) =>
  page
    .locator(".react-flow__viewport")
    .evaluate((element) => new DOMMatrixReadOnly(getComputedStyle(element).transform).a);

async function open(page: Page) {
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
}

async function drag(page: Page, id: string, dx: number, dy: number, release = true) {
  const title = page.getByTestId(`rf__node-${id}`).locator(".flow-node-title");
  // The view may still be easing into place after the last change.
  let box = (await title.boundingBox())!;
  for (let settled = 0; settled < 2;) {
    await page.waitForTimeout(120);
    const next = (await title.boundingBox())!;
    settled = next.x === box.x && next.y === box.y ? settled + 1 : 0;
    box = next;
  }
  const x = box.x + 40;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  // The drag is anchored at the first move past React Flow's threshold, so make that a small one.
  await page.mouse.move(x + 2, y + 2);
  await page.mouse.move(x + 2 + dx / 2, y + 2 + dy / 2, { steps: 4 });
  await page.mouse.move(x + 2 + dx, y + 2 + dy, { steps: 4 });
  if (release) await page.mouse.up();
}

const position = (page: Page, id: string) =>
  page.getByTestId(`rf__node-${id}`).evaluate((element) => {
    const matrix = new DOMMatrixReadOnly(getComputedStyle(element).transform);
    return { x: matrix.e, y: matrix.f };
  });

/** Snap both cards, move Voice gain away, then drag it back level and keep holding it. */
async function holdVoiceLevelWithMicrophone(page: Page) {
  await drag(page, "mic", 37, 53);
  await drag(page, "voice", 37, 53);
  const scale = await zoom(page);
  await drag(page, "voice", 0, 240 * scale);
  const mic = await position(page, "mic");
  const voice = await position(page, "voice");
  await drag(page, "voice", 0, (mic.y - voice.y) * scale, false);
}

test("a dragged card lands on the 24 px grid", async ({ page }) => {
  await open(page);
  await drag(page, "voice", 37, 53);
  const { x, y } = await position(page, "voice");
  expect(x % GRID).toBe(0);
  expect(y % GRID).toBe(0);
});

test("arrow keys move a selected card one grid step", async ({ page }) => {
  await open(page);
  await drag(page, "voice", 37, 53);
  const start = await position(page, "voice");
  await page.getByTestId("rf__node-voice").focus();
  await page.keyboard.press("ArrowRight");
  await expect.poll(() => position(page, "voice")).toEqual({ x: start.x + GRID, y: start.y });
  await page.keyboard.press("ArrowDown");
  await expect.poll(() => position(page, "voice")).toEqual({ x: start.x + GRID, y: start.y + GRID });
});

test("guides name the card lined up with, and go away on release", async ({ page }) => {
  await open(page);
  await holdVoiceLevelWithMicrophone(page);
  await expect(page.locator(".canvas-guide-label")).toContainText("Lined up with Microphone");
  await expect(page.locator(".canvas-guide").first()).toBeVisible();
  await page.mouse.up();
  await expect(page.locator(".canvas-guide-label")).toHaveCount(0);
  await expect(page.locator(".canvas-guide")).toHaveCount(0);
});

test("the map opens top right, closes, stays closed, and moving the view does not move the canvas", async ({
  page,
}) => {
  await open(page);
  const map = page.locator(".canvas-minimap");
  await expect(map).toBeVisible();
  const canvas = (await page.locator(".session-flow-canvas").boundingBox())!;
  const before = await page.locator("#signal-flow-panel").boundingBox();
  const box = (await map.boundingBox())!;
  expect(box.x + box.width).toBeGreaterThan(canvas.x + canvas.width - 40);
  expect(box.y).toBeLessThan(canvas.y + 40);
  await page.getByRole("button", { name: "Close the map" }).click();
  await expect(map).toHaveCount(0);
  expect(await page.locator("#signal-flow-panel").boundingBox()).toEqual(before);
  await page.reload();
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await expect(map).toHaveCount(0);
  await page.getByRole("button", { name: "Map", exact: true }).click();
  await expect(map).toBeVisible();
});

test("clicking the map moves the view", async ({ page }) => {
  await open(page);
  const viewport = page.locator(".react-flow__viewport");
  const before = await viewport.evaluate((element) => getComputedStyle(element).transform);
  const box = (await page.locator(".canvas-minimap").boundingBox())!;
  await page.mouse.click(box.x + 8, box.y + box.height - 8);
  await expect.poll(() => viewport.evaluate((element) => getComputedStyle(element).transform)).not.toBe(before);
});

test("a large route opens with readable text, and the fit button still shows it all", async ({ page }) => {
  const session = {
    id: "wide",
    name: "Wide route",
    revision: 0,
    edges: [],
    nodes: Array.from({ length: 15 }, (_, index) => ({
      id: `gain-${index}`,
      kind: "gain",
      typeVersion: 1,
      name: `Gain ${index}`,
      enabled: true,
      bypass: false,
      parameters: {},
      ports: [
        { name: "in", direction: "input", channels: 2 },
        { name: "out", direction: "output", channels: 2 },
      ],
    })),
  };
  await page.addInitScript((value) => Object.assign(window, { __routeFixtureSession: value }), session);
  await open(page);
  await expect.poll(() => zoom(page)).toBeGreaterThanOrEqual(1);
  const smallest = await page
    .locator(".flow-node-content")
    .first()
    .evaluate((card) => {
      const sizes = [...card.querySelectorAll<HTMLElement>("*")]
        .filter((element) => element.childElementCount === 0 && element.textContent?.trim())
        .map((element) => parseFloat(getComputedStyle(element).fontSize));
      return Math.min(...sizes);
    });
  expect(smallest * (await zoom(page))).toBeGreaterThanOrEqual(11);
  await page.locator(".react-flow__controls-fitview").click();
  await expect.poll(() => zoom(page)).toBeLessThan(1);
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`canvas aids in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await open(page);
    await holdVoiceLevelWithMicrophone(page);
    await expect(page.locator(".canvas-guide-label")).toBeVisible();
    await page.screenshot({ path: info.outputPath(`${theme}-canvas-aids.png`) });
    await page.mouse.up();
  });
}
