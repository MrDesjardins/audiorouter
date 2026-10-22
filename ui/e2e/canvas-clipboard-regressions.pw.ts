import { test, expect } from "./browser-test";

test("keyboard paste brings the new node into view after panning", async ({ page }) => {
  await page.goto("/route-harness.html");
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await page.keyboard.press("Control+c");
  const canvas = (await page.locator(".session-flow-canvas").boundingBox())!;
  await page.mouse.move(canvas.x + 80, canvas.y + 80);
  await page.mouse.down({ button: "middle" });
  await page.mouse.move(canvas.x + canvas.width - 60, canvas.y + 80, { steps: 10 });
  await page.mouse.up({ button: "middle" });
  await page.keyboard.press("Control+v");
  await expect
    .poll(async () => {
      const box = await page.getByTestId("rf__node-voice-copy-1").boundingBox();
      return (
        !!box &&
        box.x >= canvas.x &&
        box.x + box.width <= canvas.x + canvas.width &&
        box.y >= canvas.y &&
        box.y + box.height <= canvas.y + canvas.height
      );
    })
    .toBe(true);
});

test("selected UI text keeps native Ctrl+C", async ({ page }) => {
  await page.goto("/route-harness.html");
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await page.evaluate(() => {
    const range = document.createRange();
    range.selectNodeContents(document.querySelector(".topbar .eyebrow")!);
    window.getSelection()!.removeAllRanges();
    window.getSelection()!.addRange(range);
    window.addEventListener("keydown", (event) => {
      if (event.key.toLowerCase() === "c") Object.assign(window, { __copyPrevented: event.defaultPrevented });
    });
  });
  await page.keyboard.press("Control+c");
  expect(await page.evaluate(() => (window as unknown as { __copyPrevented: boolean }).__copyPrevented)).toBe(false);
});

test("reserved clipboard binding cannot stop playback when copying a tool", async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem(
      "audiorouter.ui.shortcuts",
      JSON.stringify({ sessionToggle: "Control+C", privacyMute: "Control+Alt+M" }),
    );
    Object.assign(window, { __routeFixtureRunning: true });
  });
  await page.goto("/route-harness.html");
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await page.keyboard.press("Control+c");
  await expect(page.locator(".global-action-message")).toContainText("copied");
  await expect(page.locator(".audio-run-state")).toContainText("Audio running");
});
