import { expect, test, type Page } from "@playwright/test";
import type { Session } from "../../contracts/src/index";

const port = (name: string, direction: "input" | "output") => ({ name, direction, channels: 2 });
const node = (id: string, kind: Session["nodes"][number]["kind"], name: string, sourceOnly = false): Session["nodes"][number] => ({
  id, kind, name, typeVersion: 1, enabled: true, bypass: false, parameters: {},
  ports: sourceOnly ? [port("out", "output")] : kind === "physicalOutput" ? [port("in", "input")] : [port("in", "input"), port("out", "output")],
});
const session: Session = {
  id: "banner-regression", name: "Siege route", revision: 1, schemaVersion: 1,
  nodes: [node("game", "physicalInput", "Siege game", true), node("eq", "parametricEq", "Siege Advanced EQ"), node("mixer", "mixer", "Mixer 1"), node("output", "physicalOutput", "Physical output 1")],
  edges: [{ id: "game-eq", sourceNode: "game", sourcePort: "out", destinationNode: "eq", destinationPort: "in", enabled: true, matrix: [1, 0, 0, 1] }],
};

async function connect(page: Page, sender: string, receiver: string) {
  const input = page.getByTestId(`rf__node-${receiver}`).locator('.target[data-debug-side="left"]');
  await input.hover(); // Wait for React Flow's post-Undo layout to settle.
  const start = await input.boundingBox();
  if (!start) throw new Error("Missing input handle");
  await page.mouse.move(start.x + start.width / 2, start.y + start.height / 2);
  await page.mouse.down();
  await page.mouse.move(start.x - 15, start.y + start.height / 2, { steps: 3 });
  await expect(page.locator(".session-flow-canvas")).toHaveAttribute("data-connection-mode", "from-input");
  const end = await page.getByTestId(`rf__node-${sender}`).locator('.source[data-debug-side="right"]').boundingBox();
  if (!end) throw new Error("Missing output handle");
  await page.mouse.move(end.x + end.width / 2, end.y + end.height / 2, { steps: 12 });
  await page.mouse.up();
}

test("long Timing content scrolls in the sidebar without pushing the canvas down", async ({ page }, testInfo) => {
  const outputs = Array.from({ length: 12 }, (_, index) => node(`output-${index}`, "physicalOutput", `Output ${index + 1}`));
  const fixture: Session = { ...session, nodes: [...session.nodes.slice(0, 3), ...outputs], edges: [...session.edges, ...outputs.map((output) => ({ ...session.edges[0], id: `eq-${output.id}`, sourceNode: "eq", destinationNode: output.id }))] };
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.addInitScript((value) => { Object.assign(window, { __routeFixtureSession: value }); }, fixture);
  await page.goto("/route-harness.html");
  const panel = page.locator("#signal-flow-panel");
  const before = (await panel.boundingBox())!;
  await page.getByRole("tab", { name: "Timing", exact: true }).click();
  await expect(page.getByRole("region", { name: "Signal timing", exact: true })).toBeVisible();
  const after = (await panel.boundingBox())!;
  expect(Math.abs(after.y - before.y)).toBeLessThan(1);
  expect(Math.abs(after.height - before.height)).toBeLessThan(1);
  expect(after.y).toBeLessThan(210);
  expect(await page.locator(".right-workbench").evaluate((element) => element.scrollHeight > element.clientHeight)).toBe(true);
  for (const theme of ["dark", "light", "high-contrast"]) {
    await page.getByRole("combobox", { name: "Color theme" }).selectOption(theme);
    await page.screenshot({ path: testInfo.outputPath(`long-timing-${theme}.png`) });
  }
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`occupied input notice and Timing keep canvas usable in ${theme}`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.addInitScript((fixture) => { Object.assign(window, { __routeFixtureSession: fixture }); }, session);
    await page.goto("/route-harness.html");
    await page.getByRole("combobox", { name: "Color theme" }).selectOption(theme);
    const canvas = page.locator(".session-flow-canvas");
    const bounds = await canvas.boundingBox();
    if (!bounds) throw new Error("Missing canvas");
    for (const [id, x, y] of [["game", 90, 100], ["eq", 280, 260], ["mixer", 510, 100], ["output", 650, 260]] as const) {
      const title = await page.getByTestId(`rf__node-${id}`).locator(".flow-node-title").boundingBox();
      if (!title) throw new Error("Missing node title");
      await page.mouse.move(title.x + title.width / 2, title.y + title.height / 2);
      await page.mouse.down();
      await page.mouse.move(bounds.x + x, bounds.y + y, { steps: 10 });
      await page.mouse.up();
    }
    // This is the reversed gesture in the user's screenshot: Mixer -> occupied EQ.
    await connect(page, "mixer", "eq");
    const notice = page.locator(".global-action-message");
    await expect(notice).toContainText("Siege Advanced EQ already receives Siege game");
    await expect(notice).toContainText("receiving tool's blue input");
    await expect(page.locator(".inactive-route-warning")).toBeVisible();
    const noticeBox = (await notice.boundingBox())!;
    expect((await page.locator(".workspace-grid").boundingBox())!.y).toBeGreaterThanOrEqual(noticeBox.y + noticeBox.height - 1);
    await page.screenshot({ path: testInfo.outputPath(`banners-${theme}.png`) });
    await page.getByRole("button", { name: "Dismiss message", exact: true }).click();
    await expect(notice).not.toContainText("already receives");
    await expect(page.getByRole("button", { name: "Dismiss message", exact: true })).toHaveCount(0);
    await expect(page.getByTestId("rf__edge-game-eq")).toHaveCount(1);
    await connect(page, "mixer", "eq");
    await notice.getByRole("button", { name: "Replace input connection", exact: true }).click();
    await expect(page.getByTestId("rf__edge-game-eq")).toHaveCount(0);
    await page.locator(".topbar").getByRole("button", { name: "Undo", exact: true }).click();
    await expect(page.getByTestId("rf__edge-game-eq")).toHaveCount(1);
    await connect(page, "eq", "mixer");
    await expect(page.locator(".react-flow__edges .react-flow__edge")).toHaveCount(2);
    const beforeTiming = (await canvas.boundingBox())!;
    await page.getByRole("tab", { name: "Timing", exact: true }).click();
    expect(Math.abs((await canvas.boundingBox())!.y - beforeTiming.y)).toBeLessThan(1);
    await expect(canvas).toBeInViewport();
    await page.screenshot({ path: testInfo.outputPath(`timing-${theme}.png`) });
  });
}
