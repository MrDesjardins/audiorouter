import { expect, test } from "./browser-test";
import { fitCanvas, openSettings, stereoDemoSession } from "./workbench";
import { appendDraftConnection } from "../src/draft";

test("canvas Ctrl+A selects all, and Ctrl+D duplicates only a single selected tool", async ({ page }) => {
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  const canvas = page.locator(".session-flow-canvas");
  await canvas.focus();
  await page.keyboard.press("Control+a");
  await expect(page.locator(".react-flow__node.selected")).toHaveCount(3);
  await page.keyboard.press("Control+d");
  await expect(page.locator(".react-flow__node")).toHaveCount(3);
  await fitCanvas(page);
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await page.getByTestId("rf__node-voice").focus();
  await page.keyboard.press("Control+d");
  await expect(page.locator(".react-flow__node")).toHaveCount(4);
  await expect(page.locator(".react-flow__node.selected")).toHaveCount(1);
  await expect(page.locator(".react-flow__node.selected .flow-node-title")).toContainText("Voice gain");
  await page.getByRole("button", { name: /Rename Voice gain/ }).click();
  const name = page.getByRole("textbox", { name: "Tool name", exact: true });
  await name.press("Control+a");
  await name.press("Control+d");
  await expect(page.locator(".react-flow__node")).toHaveCount(4);
  await expect(page.locator(".react-flow__node.selected")).toHaveCount(1);
});

test("canvas select-all includes groups without duplicating a mixed selection", async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem(
      "audiorouter.ui.groups.demo-session",
      JSON.stringify([
        {
          id: "group-keyboard",
          name: "Voice tools",
          color: "#25b8df",
          opacity: 5,
          fontSize: 18,
          x: -24,
          y: -48,
          width: 1000,
          height: 260,
        },
      ]),
    ),
  );
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.locator(".session-flow-canvas").focus();
  await page.keyboard.press("Control+a");
  await expect(page.locator(".react-flow__node.selected")).toHaveCount(4);
  await page.keyboard.press("Control+d");
  await expect(page.locator(".react-flow__node")).toHaveCount(4);
});

test("canvas keyboard zoom preserves positions and reduced motion", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  const canvas = page.locator(".session-flow-canvas");
  const positions = () =>
    page
      .locator(".react-flow__node")
      .evaluateAll((nodes) => nodes.map((node) => (node as HTMLElement).style.transform));
  const before = await positions();
  const zoom = () =>
    page
      .locator(".react-flow__viewport")
      .evaluate((element) => new DOMMatrixReadOnly(getComputedStyle(element).transform).a);
  await canvas.focus();
  const start = await zoom();
  await page.keyboard.press("Control+=");
  await expect.poll(zoom).toBeGreaterThan(start);
  const larger = await zoom();
  await page.keyboard.press("Control+-");
  await expect.poll(zoom).toBeLessThan(larger);
  await page.keyboard.press("Control+0");
  await expect
    .poll(async () => {
      const view = (await canvas.boundingBox())!;
      return page.locator(".react-flow__node").evaluateAll(
        (nodes, view) =>
          nodes.every((node) => {
            const box = node.getBoundingClientRect();
            return box.left >= view.x && box.right <= view.x + view.width;
          }),
        view,
      );
    })
    .toBe(true);
  expect(await positions()).toEqual(before);
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: skip links reach the canvas and workbench without changing layout`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const canvas = page.locator(".session-flow-canvas");
    const before = await canvas.boundingBox();
    await page.keyboard.press("Tab");
    const skip = page.getByRole("link", { name: "Skip to canvas", exact: true });
    await expect(skip).toBeFocused();
    await expect(skip).toBeInViewport();
    expect(await skip.evaluate((element) => getComputedStyle(element).backgroundColor)).not.toBe("rgba(0, 0, 0, 0)");
    await page.screenshot({ path: `../target/ui-quality-review/round-25/skip-${theme}.png` });
    await page.keyboard.press("Enter");
    await expect(canvas).toBeFocused();
    expect(await canvas.boundingBox()).toEqual(before);
    await page.getByRole("link", { name: "Skip to tools and settings", exact: true }).focus();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("complementary", { name: "Tools and settings" })).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(page.getByRole("tab", { name: "Tools", exact: true })).toBeFocused();
    expect(await canvas.boundingBox()).toEqual(before);
    await openSettings(page, "Keyboard shortcuts");
    const help = page.locator(".shortcut-panel");
    await expect(help).toContainText("Ctrl+D duplicates one selected tool");
    expect(await help.evaluate((element) => element.scrollWidth <= element.clientWidth)).toBe(true);
    await help.screenshot({ path: `../target/ui-quality-review/round-25/shortcuts-${theme}.png` });
  });
}

test("Ctrl+S saves through the header action and leaves editing and dialogs alone", async ({ page }) => {
  const routed = appendDraftConnection(
    appendDraftConnection(stereoDemoSession, "mic", "out", "voice", "in"),
    "voice",
    "out",
    "headphones",
    "in",
  );
  await page.addInitScript((session) => Object.assign(window, { __routeFixtureSession: session }), routed);
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  await page.getByRole("button", { name: "Rename", exact: true }).click();
  const name = page.getByRole("textbox", { name: "Session name", exact: true });
  await name.fill("Keyboard saved route");
  const save = page.locator(".topbar").getByRole("button", { name: "Save", exact: true });
  await expect(save).toBeEnabled();
  await name.press("Control+s");
  await expect(save).toBeEnabled();
  await page.getByRole("button", { name: "New", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "New session" });
  await dialog.getByRole("button", { name: "Cancel", exact: true }).focus();
  await page.keyboard.press("Control+s");
  await expect(save).toBeEnabled();
  await page.keyboard.press("Escape");
  await page.getByRole("tab", { name: "Session", exact: true }).focus();
  await page.keyboard.press("Control+s");
  await expect(page.locator(".global-action-message")).toContainText("Route saved");
  await expect(save).toBeDisabled();
  await expect(page.locator(".header-session-name")).toHaveText("Keyboard saved route");
  expect(
    await page.evaluate(
      () =>
        (window as unknown as { __routeFixtureCalls: () => string[] })
          .__routeFixtureCalls()
          .filter((call) => call === "commit").length,
    ),
  ).toBe(1);
});
