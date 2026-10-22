import { test, expect, type Page } from "./browser-test";
import { demoSession } from "../src/fixtures";
import type { CanvasGroup } from "../src/CanvasGroups";

const outer: CanvasGroup = {
  id: "group-outer",
  name: "Outer",
  x: -40,
  y: -60,
  width: 850,
  height: 500,
  color: "#5599dd",
  opacity: 5,
  fontSize: 18,
  moveContents: true,
};
const inner: CanvasGroup = {
  ...outer,
  id: "group-inner",
  name: "Inner",
  x: 260,
  y: 70,
  width: 300,
  height: 300,
  moveContents: false,
};

async function fixture(page: Page, groups: CanvasGroup[], theme = "dark", five = false) {
  await page.addInitScript(
    ({ session, groups, theme, five }) => {
      const nodes = five
        ? Array.from({ length: 5 }, (_, index) => ({ ...session.nodes[1], id: `gain-${index}`, name: `Gain ${index}` }))
        : session.nodes;
      Object.assign(window, { __routeFixtureSession: { ...session, nodes, edges: [] } });
      localStorage.setItem("audiorouter.ui.theme", theme);
      if (localStorage.getItem("canvas-layout-fixture-ready")) return;
      localStorage.setItem("canvas-layout-fixture-ready", "yes");
      localStorage.setItem(`audiorouter.ui.groups.${session.id}`, JSON.stringify(groups));
      localStorage.setItem(
        `audiorouter.ui.layout.${session.id}`,
        JSON.stringify(
          five
            ? Object.fromEntries([0, 228, 466, 694, 932].map((x, index) => [`gain-${index}`, { x, y: index * 30 }]))
            : { mic: { x: 0, y: 100 }, voice: { x: 300, y: 100 }, headphones: { x: 1000, y: 100 } },
        ),
      );
    },
    { session: demoSession, groups, theme, five },
  );
  await page.goto("/route-harness.html");
  await expect(page.getByTestId(five ? "rf__node-gain-0" : "rf__node-voice")).toBeVisible();
  let previous = "";
  await expect
    .poll(
      async () => {
        const current = await page.locator(".react-flow__viewport").getAttribute("style");
        const same = current === previous;
        previous = current ?? "";
        return same;
      },
      { intervals: [100] },
    )
    .toBe(true);
}

async function geometry(page: Page) {
  return page.locator(".react-flow__node").evaluateAll((elements) =>
    Object.fromEntries(
      elements.map((element) => {
        const html = element as HTMLElement;
        const match = html.style.transform.match(/translate\(([-\d.]+)px,\s*([-\d.]+)px\)/)!;
        return [
          html.dataset.id!,
          { x: Number(match[1]), y: Number(match[2]), width: html.offsetWidth, height: html.offsetHeight },
        ];
      }),
    ),
  );
}

async function dragGroup(page: Page, id: string, dx = 80, dy = 45) {
  const caption = page.getByTestId(`rf__node-${id}`).locator(".canvas-group-caption");
  const box = (await caption.boundingBox())!;
  await page.mouse.move(box.x + 14, box.y + 12);
  await page.mouse.down();
  await page.mouse.move(box.x + 14 + dx, box.y + 12 + dy, { steps: 12 });
  await page.mouse.up();
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`group carries its nested tools exactly once in ${theme}`, async ({ page }, info) => {
    await fixture(page, [outer, inner], theme);
    const before = await geometry(page);
    await dragGroup(page, outer.id);
    const after = await geometry(page);
    const dx = after[outer.id].x - before[outer.id].x;
    const dy = after[outer.id].y - before[outer.id].y;
    expect(dx).toBeGreaterThan(20);
    for (const id of [inner.id, "mic", "voice"]) {
      expect(after[id].x - before[id].x).toBeCloseTo(dx, 2);
      expect(after[id].y - before[id].y).toBeCloseTo(dy, 2);
    }
    expect(after.headphones).toEqual(before.headphones);
    await page.getByRole("button", { name: "Undo layout", exact: true }).click();
    await expect.poll(async () => (await geometry(page)).voice.x).toBe(before.voice.x);
    await page.getByRole("button", { name: "Redo layout", exact: true }).click();
    await expect.poll(async () => (await geometry(page)).voice.x).toBe(after.voice.x);
    await expect(page.getByLabel("Move contents with group")).toBeChecked();
    await page.screenshot({ path: info.outputPath(`group-move-${theme}.png`) });
    const saved = await page.evaluate(
      (id) => ({
        groups: JSON.parse(localStorage.getItem(`audiorouter.ui.groups.${id}`)!),
        layout: JSON.parse(localStorage.getItem(`audiorouter.ui.layout.${id}`)!),
      }),
      demoSession.id,
    );
    expect(saved.layout.voice.x).toBeCloseTo(after.voice.x, 2);
    expect(saved.groups.find((group: CanvasGroup) => group.id === inner.id).x).toBeCloseTo(after[inner.id].x, 2);
    await page.reload();
    await expect(page.getByTestId("rf__node-voice")).toBeVisible();
    expect((await geometry(page)).voice.x).toBeCloseTo(after.voice.x, 2);
    await page.getByTestId(`rf__node-${outer.id}`).locator(".canvas-group-caption").click();
    await page.getByLabel("Move contents with group").uncheck();
    const frameOnlyBefore = await geometry(page);
    await dragGroup(page, outer.id, 30, 20);
    const frameOnlyAfter = await geometry(page);
    expect(frameOnlyAfter.voice).toEqual(frameOnlyBefore.voice);
    expect(frameOnlyAfter[inner.id]).toEqual(frameOnlyBefore[inner.id]);
  });

  test(`mixed selection matches tool and group size in ${theme}`, async ({ page }, info) => {
    await fixture(page, [{ ...outer, width: 600, height: 400 }], theme);
    await page.getByTestId("rf__node-mic").locator(".flow-node-title").click();
    await page
      .getByTestId("rf__node-voice")
      .locator(".flow-node-title")
      .click({ modifiers: ["Control"] });
    await page
      .getByTestId(`rf__node-${outer.id}`)
      .locator(".canvas-group-caption")
      .click({ modifiers: ["Control"] });
    const inspector = page.getByRole("region", { name: "Selection layout" });
    await expect(inspector.getByRole("heading")).toHaveText("3 items selected");
    await inspector.getByRole("button", { name: "Largest width", exact: true }).click();
    await inspector.getByRole("button", { name: "Largest height", exact: true }).click();
    await expect
      .poll(async () => {
        const items = await geometry(page);
        return [items.mic.width, items.voice.width, items[outer.id].width];
      })
      .toEqual([600, 600, 600]);
    const sized = await geometry(page);
    expect([sized.mic.height, sized.voice.height, sized[outer.id].height]).toEqual([400, 400, 400]);
    await expect(page.locator(".topbar").getByRole("button", { name: "Save", exact: true })).toBeDisabled();
    await page.screenshot({ path: info.outputPath(`selection-layout-${theme}.png`) });
    const stored = await page.evaluate(
      (id) => JSON.parse(localStorage.getItem(`audiorouter.ui.layout.${id}.sizes`)! as string),
      demoSession.id,
    );
    expect(stored.mic).toMatchObject({ width: 600, height: 400 });
    await page.getByRole("button", { name: "Undo layout", exact: true }).click();
    await expect.poll(async () => (await geometry(page)).mic.height).not.toBe(400);
    await page.getByRole("button", { name: "Redo layout", exact: true }).click();
    await expect.poll(async () => (await geometry(page)).mic.height).toBe(400);
    await page.reload();
    await expect
      .poll(async () => {
        const items = await geometry(page);
        return [items.mic?.width, items.mic?.height];
      })
      .toEqual([600, 400]);
  });
}

test("locked nested frame blocks carrying while frame-only movement is allowed", async ({ page }) => {
  await fixture(page, [outer, { ...inner, locked: true }]);
  const before = await geometry(page);
  await dragGroup(page, outer.id);
  expect((await geometry(page))[outer.id]).toEqual(before[outer.id]);
  await page.getByTestId(`rf__node-${outer.id}`).locator(".canvas-group-caption").click();
  await page.getByLabel("Move contents with group").uncheck();
  await dragGroup(page, outer.id, 30, 20);
  const after = await geometry(page);
  expect(after[outer.id].x).not.toBe(before[outer.id].x);
  expect(after[inner.id]).toEqual(before[inner.id]);
  expect(after.voice).toEqual(before.voice);
});

test("five tools align and distribute equal edge gaps without changing audio", async ({ page }) => {
  await fixture(page, [], "dark", true);
  for (let index = 0; index < 5; index++)
    await page
      .getByTestId(`rf__node-gain-${index}`)
      .locator(".flow-node-title")
      .click(index ? { modifiers: ["Control"] } : {});
  const inspector = page.getByRole("region", { name: "Selection layout" });
  await expect(inspector.getByRole("heading")).toHaveText("5 items selected");
  await inspector.getByRole("button", { name: "Align top", exact: true }).click();
  await inspector.getByRole("button", { name: "Space horizontally", exact: true }).click();
  const items = Object.entries(await geometry(page))
    .filter(([id]) => id.startsWith("gain-"))
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([, item]) => item);
  expect(items.map((item) => item.y)).toEqual([0, 0, 0, 0, 0]);
  const gaps = items.slice(1).map((item, index) => item.x - items[index].x - items[index].width);
  expect(Math.max(...gaps) - Math.min(...gaps)).toBeLessThan(1);
  expect(items[0].x).toBe(0);
  expect(items[4].x).toBe(932);
  await expect(page.locator(".topbar").getByRole("button", { name: "Save", exact: true })).toBeDisabled();
});

test("resizing a carrying frame keeps contents still and undo respects a new lock", async ({ page }) => {
  await fixture(page, [outer]);
  await page.getByTestId(`rf__node-${outer.id}`).locator(".canvas-group-caption").click();
  const before = await geometry(page);
  const handle = page.getByTestId(`rf__node-${outer.id}`).locator(".react-flow__resize-control.handle.top.left");
  const box = (await handle.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x - 25, box.y - 25, { steps: 8 });
  await page.mouse.up();
  const after = await geometry(page);
  expect(after[outer.id].width).toBeGreaterThan(before[outer.id].width);
  expect(after.voice).toEqual(before.voice);
  expect(after.mic).toEqual(before.mic);
  await page.getByLabel("Lock group position and size").check();
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  await expect(page.locator(".global-action-message")).toContainText("Unlock the affected groups");
  expect((await geometry(page))[outer.id]).toEqual(after[outer.id]);
  await page.getByLabel("Lock group position and size").uncheck();
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  await expect.poll(async () => (await geometry(page))[outer.id].width).toBe(before[outer.id].width);
});

test("dragging a selected carrying group and child moves the child only once", async ({ page }) => {
  await fixture(page, [outer, inner]);
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await page
    .getByTestId("rf__node-" + outer.id)
    .locator(".canvas-group-caption")
    .click({ modifiers: ["Control"] });
  await expect(page.getByRole("region", { name: "Selection layout" }).getByRole("heading")).toHaveText(
    "2 items selected",
  );
  const before = await geometry(page);
  await dragGroup(page, outer.id, 50, 30);
  const after = await geometry(page);
  const dx = after[outer.id].x - before[outer.id].x;
  expect(dx).toBeGreaterThan(20);
  for (const id of [inner.id, "voice", "mic"]) expect(after[id].x - before[id].x).toBeCloseTo(dx, 2);
});

test("alignment and size matching preserve a selected locked group", async ({ page }) => {
  await fixture(page, [{ ...outer, locked: true, moveContents: false }]);
  await page.getByTestId("rf__node-headphones").locator(".flow-node-title").click();
  await page
    .getByTestId("rf__node-" + outer.id)
    .locator(".canvas-group-caption")
    .click({ modifiers: ["Control"] });
  const before = await geometry(page);
  const inspector = page.getByRole("region", { name: "Selection layout" });
  await inspector.getByRole("button", { name: "Align left", exact: true }).click();
  await inspector.getByRole("button", { name: "Largest width", exact: true }).click();
  const after = await geometry(page);
  expect(after[outer.id]).toEqual(before[outer.id]);
  expect(after.headphones.x).toBe(before[outer.id].x);
  expect(after.headphones.width).toBe(before[outer.id].width);
});

test("Shift marquee selects tools and keyboard movement can be undone", async ({ page }) => {
  await fixture(page, [], "dark", true);
  const first = (await page.getByTestId("rf__node-gain-0").boundingBox())!;
  const last = (await page.getByTestId("rf__node-gain-4").boundingBox())!;
  await page.keyboard.down("Shift");
  await page.mouse.move(first.x - 10, first.y - 10);
  await page.mouse.down();
  await page.mouse.move(last.x + last.width + 10, last.y + last.height + 10, { steps: 10 });
  await page.mouse.up();
  await page.keyboard.up("Shift");
  await expect(page.getByRole("region", { name: "Selection layout" }).getByRole("heading")).toHaveText(
    "5 items selected",
  );
  await page.locator(".react-flow__pane").click({ position: { x: 10, y: 10 } });
  await page.getByTestId("rf__node-gain-0").locator(".flow-node-title").click();
  const before = await geometry(page);
  await page.getByTestId("rf__node-gain-0").focus();
  await page.keyboard.press("ArrowRight");
  await expect.poll(async () => (await geometry(page))["gain-0"].x).toBeGreaterThan(before["gain-0"].x);
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  await expect.poll(async () => (await geometry(page))["gain-0"].x).toBe(before["gain-0"].x);
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`multi-group options apply only to selected groups in ${theme}`, async ({ page }, info) => {
    const other = { ...inner, id: "group-other", name: "Other", x: 1000, y: 400 };
    await fixture(page, [outer, { ...inner, locked: true }, other], theme);
    await page
      .getByTestId("rf__node-" + outer.id)
      .locator(".canvas-group-caption")
      .click();
    await page
      .getByTestId("rf__node-" + inner.id)
      .locator(".canvas-group-caption")
      .click({ modifiers: ["Control"] });
    const panel = page.getByRole("region", { name: "Selection layout" });
    const lock = panel.getByRole("checkbox", { name: "Lock position and size", exact: true });
    const carry = panel.getByRole("checkbox", { name: "Move contents with group", exact: true });
    await expect(lock).toBeVisible();
    expect(await lock.evaluate((element) => (element as HTMLInputElement).indeterminate)).toBe(true);
    expect(await carry.evaluate((element) => (element as HTMLInputElement).indeterminate)).toBe(true);
    await expect(panel.getByRole("button", { name: "Align left", exact: true }).locator("svg")).toHaveCount(1);
    await page.screenshot({ path: info.outputPath(`group-options-icons-${theme}.png`) });
    await panel.getByRole("button", { name: "Space horizontally", exact: true }).scrollIntoViewIfNeeded();
    await expect(panel.getByRole("button", { name: "Space horizontally", exact: true }).locator("svg")).toHaveCount(1);
    await page.screenshot({ path: info.outputPath(`group-spacing-icons-${theme}.png`) });
    const before = await geometry(page);
    await lock.click();
    await expect(lock).toBeChecked();
    await expect(panel.getByRole("button", { name: "Align left", exact: true })).toBeDisabled();
    await lock.click();
    await expect(lock).not.toBeChecked();
    await page
      .getByTestId("rf__node-voice")
      .locator(".flow-node-title")
      .click({ modifiers: ["Control"] });
    await expect(panel.getByRole("heading")).toHaveText("3 items selected");
    await carry.click();
    await expect(carry).toBeChecked();
    await carry.click();
    await expect(carry).not.toBeChecked();
    expect(await geometry(page)).toEqual(before);
    const saved = (await page.evaluate(
      (id) => JSON.parse(localStorage.getItem(`audiorouter.ui.groups.${id}`)!),
      demoSession.id,
    )) as CanvasGroup[];
    for (const id of [outer.id, inner.id])
      expect(saved.find((group) => group.id === id)).toMatchObject({ locked: false, moveContents: false });
    expect(saved.find((group) => group.id === other.id)).toEqual(other);
    await expect(page.locator(".topbar").getByRole("button", { name: "Save", exact: true })).toBeDisabled();
    await page.reload();
    await page
      .getByTestId("rf__node-" + outer.id)
      .locator(".canvas-group-caption")
      .click();
    await expect(page.getByLabel("Move contents with group")).not.toBeChecked();
  });
}
