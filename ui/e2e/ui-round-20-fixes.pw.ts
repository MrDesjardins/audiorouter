import { openPage } from "./workbench";
import { expect, test } from "./browser-test";
import { demoSession } from "../src/fixtures";

const open = async (page: import("@playwright/test").Page) => {
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
};
const tab = (page: import("@playwright/test").Page, name: string) => page.getByRole("tab", { name, exact: true });

test("each sidebar tab comes back where it was left", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 600 });
  await open(page);
  const panel = page.locator(".right-workbench");
  await tab(page, "Tools").click();
  await panel.evaluate((element) => (element.scrollTop = 240));
  await page.waitForTimeout(50);
  await tab(page, "Session").click();
  await expect(tab(page, "Session")).toHaveAttribute("aria-selected", "true");
  await expect.poll(() => panel.evaluate((element) => element.scrollTop)).toBeLessThan(240);
  await tab(page, "Tools").click();
  await expect.poll(() => panel.evaluate((element) => element.scrollTop)).toBe(240);
});

test("a section opened in a sidebar tab stays open after visiting another tab", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await open(page);
  await openPage(page, "Status");
  const section = page.locator("details.troubleshoot-group");
  await section.locator("> summary").click();
  await expect(section).toHaveAttribute("open");
  await tab(page, "Session").click();
  await expect(section).toHaveCount(0);
  await openPage(page, "Status");
  await expect(section).toHaveAttribute("open");
  // Closing it is remembered as well.
  await section.locator("> summary").click();
  await expect(section).not.toHaveAttribute("open");
  await tab(page, "Session").click();
  await openPage(page, "Status");
  await expect(section).toBeVisible();
  await expect(section).not.toHaveAttribute("open");
});

test("a section row can be clicked anywhere, not only on its words", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await open(page);
  await openPage(page, "Status");
  const section = page.locator("details.troubleshoot-group");
  const summary = section.locator("> summary");
  await summary.scrollIntoViewIfNeeded();
  const box = (await summary.boundingBox())!;
  const panel = (await section.boundingBox())!;
  // The row reaches both edges of its section, so the far right end works too.
  expect(box.x).toBeLessThanOrEqual(panel.x + 1);
  expect(box.x + box.width).toBeGreaterThanOrEqual(panel.x + panel.width - 1);
  const before = await section.evaluate((element: HTMLDetailsElement) => element.open);
  await page.mouse.click(box.x + box.width - 3, box.y + box.height / 2);
  await expect.poll(() => section.evaluate((element: HTMLDetailsElement) => element.open)).toBe(!before);
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`the top bar shows muted audio and a distinct Stop in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __routeFixtureRunning: true, __routeFixturePrivacyMuted: true });
    }, theme);
    await page.setViewportSize({ width: 1280, height: 720 });
    await open(page);
    const status = page.locator(".audio-run-state");
    await expect(status).toHaveText("Audio running · Mic muted");
    await expect(status).toHaveClass(/is-mic-muted/);
    const width = (await status.boundingBox())!.width;
    // The longest wording fits in the reserved slot without wrapping (UI-17).
    expect(await status.evaluate((element) => element.scrollWidth <= element.clientWidth + 1)).toBe(true);
    const stop = page.locator(".topbar").getByRole("button", { name: "Stop", exact: true });
    await expect(stop).toHaveClass(/stop-audio/);
    const stopBox = (await stop.boundingBox())!;
    await page.screenshot({ path: info.outputPath(`${theme}-muted-running.png`) });
    await stop.click();
    await expect(status).toHaveText("Audio stopped");
    expect((await status.boundingBox())!.width).toBe(width);
    const play = page.locator(".topbar").getByRole("button", { name: "Play", exact: true });
    // Play and Stop swap in place at the same width.
    expect((await play.boundingBox())!.width).toBe(stopBox.width);
  });

  test(`cards name what is wrong instead of saying ready in ${theme}`, async ({ page }, info) => {
    const session = structuredClone(demoSession);
    await page.addInitScript(
      ({ theme, session }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session, __routeFixturePrivacyMuted: false });
      },
      { theme, session },
    );
    await page.setViewportSize({ width: 1280, height: 720 });
    await open(page);
    await expect(page.getByTestId("rf__node-headphones").locator(".node-state")).toHaveText(/no device/i);
    await expect(page.getByTestId("rf__node-voice").locator(".node-state")).toHaveText(/no input/i);
    await page.screenshot({ path: info.outputPath(`${theme}-chips.png`) });
  });
}

test("Play with no device says where to fix it and goes there", async ({ page }) => {
  // The demo route has no devices chosen.
  await page.setViewportSize({ width: 1280, height: 720 });
  await open(page);
  await page.locator(".topbar").getByRole("button", { name: "Play", exact: true }).click();
  const message = page.locator(".global-action-message");
  await expect(message).toContainText("No audio started");
  const go = message.getByRole("button", { name: /^Go to / });
  await expect(go).toBeVisible();
  await go.click();
  await expect(tab(page, "Properties")).toHaveAttribute("aria-selected", "true");
  // It must land on the tool it named, not whichever one was selected before.
  await expect(page.locator(".main-content > .inspector").getByRole("heading", { name: "Headphones" })).toBeVisible();
  await expect(message).not.toContainText("Physical Output");
});

test("the action bar stays off two close cards until the line is pointed at", async ({ page }) => {
  const session = structuredClone(demoSession);
  session.nodes = session.nodes.slice(0, 2);
  const [first, second] = session.nodes;
  session.edges = [
    {
      id: "close",
      sourceNode: first.id,
      sourcePort: first.ports.find((port) => port.direction === "output")?.name ?? "out",
      destinationNode: second.id,
      destinationPort: second.ports.find((port) => port.direction === "input")?.name ?? "in",
      matrix: [1, 1],
      enabled: true,
    },
  ];
  await page.addInitScript((session) => Object.assign(window, { __routeFixtureSession: session }), session);
  await page.setViewportSize({ width: 1280, height: 720 });
  await open(page);
  const bar = page.locator(".edge-action-bar").first();
  await expect(bar).toHaveCount(1);
  await expect(bar).toHaveClass(/is-tight/);
  await expect(bar).toHaveCSS("opacity", "0");
  await expect(bar).toHaveCSS("pointer-events", "none");
  // Still reachable from the keyboard.
  await bar.getByRole("button").first().focus();
  await expect(bar).toHaveCSS("opacity", "1");
  await expect(bar).toHaveCSS("pointer-events", "all");
});

test("a mouse can point at a short line and reach its action bar", async ({ page }) => {
  const session = structuredClone(demoSession);
  session.edges = [
    {
      id: "voice-output",
      sourceNode: "voice",
      sourcePort: "out",
      destinationNode: "headphones",
      destinationPort: "in",
      matrix: [1, 1],
      enabled: true,
    },
  ];
  await page.addInitScript((session) => {
    Object.assign(window, { __routeFixtureSession: session });
    // Two cards close together: the line between them is shorter than the action bar.
    localStorage.setItem(
      "audiorouter.ui.layout.demo-session",
      JSON.stringify({ voice: { x: 336, y: 0 }, headphones: { x: 620, y: 0 } }),
    );
  }, session);
  await page.setViewportSize({ width: 1280, height: 720 });
  await open(page);
  const bar = page.locator(".edge-action-bar").first();
  await expect(bar).toHaveClass(/is-tight/);
  const line = (await page.getByTestId("rf__edge-voice-output").locator("path").first().boundingBox())!;
  // The real path: onto the line, then up to the bar, a few pixels at a time.
  await page.mouse.move(line.x + line.width / 2, line.y + line.height / 2);
  await expect(bar).toHaveCSS("opacity", "1");
  const remove = bar.getByRole("button", { name: "Remove connection voice-output" });
  const target = (await remove.boundingBox())!;
  await page.mouse.move(target.x + target.width / 2, target.y + target.height / 2, { steps: 12 });
  await expect(bar).toHaveCSS("opacity", "1");
  await page.mouse.down();
  await page.mouse.up();
  await expect(page.getByTestId("rf__edge-voice-output")).toHaveCount(0);
});
