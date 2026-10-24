import { test, expect } from "./real-backend";

test("foreign saved position fields cannot replace geometry identities", async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem(
      "audiorouter.ui.layout.e2e-session",
      JSON.stringify({
        voice: { x: 445, y: 67, id: "forged", group: true, width: 9999 },
      }),
    ),
  );
  await page.goto("/backend-harness.html");
  const voice = page.getByTestId("rf__node-voice");
  await expect(voice).toBeVisible();
  const position = () => voice.evaluate((element) => (element as HTMLElement).style.transform);
  const before = await position();
  await voice.click();
  await page.keyboard.press("ArrowRight");
  await expect.poll(position).not.toBe(before);
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  expect(await position()).toBe(before);
  await expect(page.locator(".topbar").getByRole("button", { name: "Save", exact: true })).toBeDisabled();
});

test("denied local storage does not prevent startup or in-memory layout undo", async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(window, "localStorage", {
      get: () => {
        throw new DOMException("Storage denied", "SecurityError");
      },
    });
  });
  await page.goto("/backend-harness.html");
  await expect(page.getByTestId("rf__node-voice")).toBeVisible();
  await page.getByRole("button", { name: "Arrange", exact: true }).click();
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  await expect(page.getByTestId("rf__node-voice")).toBeVisible();
  await expect(page.getByRole("button", { name: "Redo layout", exact: true })).toBeEnabled();
});

test("repeating Arrange without a change does not consume an undo step", async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem("audiorouter.ui.layout.e2e-session", JSON.stringify({ voice: { x: 445, y: 67 } })),
  );
  await page.goto("/backend-harness.html");
  const voice = page.getByTestId("rf__node-voice");
  await expect(voice).toBeVisible();
  const position = () => voice.evaluate((element) => (element as HTMLElement).style.transform);
  const before = await position();
  await page.getByRole("button", { name: "Arrange", exact: true }).click();
  const arranged = await position();
  expect(arranged).not.toBe(before);
  await page.getByRole("button", { name: "Arrange", exact: true }).click();
  expect(await position()).toBe(arranged);
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  expect(await position()).toBe(before);
});

test("Undo arrange uses layout history and permits redo", async ({ page }) => {
  await page.goto("/backend-harness.html");
  await page.getByRole("button", { name: "Arrange", exact: true }).click();
  await page.getByRole("button", { name: "Undo arrange", exact: true }).click();
  await expect(page.getByRole("button", { name: "Redo layout", exact: true })).toBeEnabled();
  await expect(page.getByRole("button", { name: "Undo arrange", exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Redo layout", exact: true }).click();
  await expect(page.getByRole("button", { name: "Undo layout", exact: true })).toBeEnabled();
});

test("Undo layout retires the old Undo arrange shortcut without losing redo", async ({ page }) => {
  await page.goto("/backend-harness.html");
  await page.getByRole("button", { name: "Arrange", exact: true }).click();
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  await expect(page.getByRole("button", { name: "Undo arrange", exact: true })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Redo layout", exact: true })).toBeEnabled();
});

test("connector side changes participate in layout undo and redo", async ({ page }) => {
  await page.goto("/backend-harness.html");
  const edgeId = (await page.locator(".react-flow__edges .react-flow__edge").first().getAttribute("data-id"))!;
  await page.getByRole("button", { name: "Add processor to connection " + edgeId, exact: true }).click();
  const menu = page.getByRole("menu", { name: "Configure connection " + edgeId, exact: true });
  await menu.locator(".edge-side-picker").first().getByRole("menuitem", { name: "bottom", exact: true }).click();
  await expect(page.getByRole("button", { name: "Undo layout", exact: true })).toBeEnabled();
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  await expect(
    menu.locator(".edge-side-picker").first().getByRole("menuitem", { name: "right", exact: true }),
  ).toHaveClass("is-selected");
  await page.getByRole("button", { name: "Redo layout", exact: true }).click();
  await expect(
    menu.locator(".edge-side-picker").first().getByRole("menuitem", { name: "bottom", exact: true }),
  ).toHaveClass("is-selected");
  await expect(page.locator(".topbar").getByRole("button", { name: "Save", exact: true })).toBeDisabled();
});

test("invalid saved connector preferences cannot prevent a session opening", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("audiorouter.ui.layout.e2e-session.edges", "null"));
  await page.goto("/backend-harness.html");
  await expect(page.getByTestId("rf__node-voice")).toBeVisible();
  await expect(page.locator(".react-flow__edges .react-flow__edge")).toHaveCount(2);
});
test("connector changes still work when saving local preferences fails", async ({ page }) => {
  await page.addInitScript(() => {
    const save = Storage.prototype.setItem;
    Storage.prototype.setItem = function (key, value) {
      if (key.endsWith(".edges")) throw new DOMException("Review quota probe", "QuotaExceededError");
      save.call(this, key, value);
    };
  });
  await page.goto("/backend-harness.html");
  const edgeId = (await page.locator(".react-flow__edges .react-flow__edge").first().getAttribute("data-id"))!;
  await page.getByRole("button", { name: "Add processor to connection " + edgeId, exact: true }).click();
  const menu = page.getByRole("menu", { name: "Configure connection " + edgeId, exact: true });
  await menu.locator(".edge-side-picker").first().getByRole("menuitem", { name: "bottom", exact: true }).click();
  await expect(page.getByTestId("rf__node-voice")).toBeVisible();
  await expect(
    menu.locator(".edge-side-picker").first().getByRole("menuitem", { name: "bottom", exact: true }),
  ).toHaveClass("is-selected");
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  await expect(page.getByTestId("rf__node-voice")).toBeVisible();
});

test("connector edits and geometry undo in order and a new edit clears redo", async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem("audiorouter.ui.layout.e2e-session", JSON.stringify({ voice: { x: 445, y: 67 } })),
  );
  await page.goto("/backend-harness.html");
  const voice = page.getByTestId("rf__node-voice");
  await expect(voice).toBeVisible();
  const position = () => voice.evaluate((element) => (element as HTMLElement).style.transform);
  const before = await position();
  await page.getByRole("button", { name: "Arrange", exact: true }).click();
  const arranged = await position();
  expect(arranged).not.toBe(before);
  const edgeId = (await page.locator(".react-flow__edges .react-flow__edge").first().getAttribute("data-id"))!;
  await page.getByRole("button", { name: "Add processor to connection " + edgeId, exact: true }).click();
  const menu = page.getByRole("menu", { name: "Configure connection " + edgeId, exact: true });
  const source = menu.locator(".edge-side-picker").first();
  await source.getByRole("menuitem", { name: "bottom", exact: true }).click();
  await expect(page.getByRole("button", { name: "Undo arrange", exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  expect(await position()).toBe(arranged);
  await expect(source.getByRole("menuitem", { name: "right", exact: true })).toHaveClass("is-selected");
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  expect(await position()).toBe(before);
  await page.getByRole("button", { name: "Redo layout", exact: true }).click();
  expect(await position()).toBe(arranged);
  await source.getByRole("menuitem", { name: "top", exact: true }).click();
  await expect(page.getByRole("button", { name: "Redo layout", exact: true })).toBeDisabled();
});

test("new connections keep both chosen sides when older geometry is undone", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/backend-harness.html");
  await page.getByRole("button", { name: "Arrange", exact: true }).click();
  await page.getByRole("button", { name: "Remove connection mic-voice", exact: true }).click();
  // Drag only once the removal is drawn; dragging earlier recreated the
  // connection that was still in the draft (CI flake, 2026-10-23).
  await expect(page.getByRole("button", { name: "Remove connection mic-voice", exact: true })).toHaveCount(0);
  const source = page.getByTestId("rf__node-mic").locator('.source[data-debug-side="bottom"]');
  const target = page.getByTestId("rf__node-voice").locator('.target[data-debug-side="top"]');
  const from = (await source.boundingBox())!;
  const to = (await target.boundingBox())!;
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.down();
  await page.mouse.move(to.x + to.width / 2, to.y + to.height / 2, { steps: 12 });
  await page.mouse.up();
  await expect(page.locator(".global-action-message")).toContainText("Connection added to the draft");
  const sides = () =>
    page.evaluate(() => JSON.parse(localStorage.getItem("audiorouter.ui.layout.e2e-session.edges") ?? "{}")) as Promise<
      Record<string, { source: string; target: string }>
    >;
  const before = await sides();
  const newId = Object.keys(before).find((id) => id !== "mic-voice" && id !== "voice-output")!;
  expect(before[newId]).toEqual({ source: "bottom", target: "top" });
  await page.getByRole("button", { name: "Undo layout", exact: true }).click();
  expect((await sides())[newId]).toEqual({ source: "bottom", target: "top" });
});
