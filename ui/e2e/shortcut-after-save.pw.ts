import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";
import { appendDraftConnection } from "../src/draft";

// The session shortcut must play the route just saved, like the Play button.
// Its keydown listener used to keep the pre-save session, so the saved route
// looked unsaved and started as a temporary preview (found by ESLint
// react-hooks/exhaustive-deps, code review P2-2).
test("the start/stop shortcut plays the saved route after Save", async ({ page }) => {
  const routed = appendDraftConnection(
    appendDraftConnection(demoSession, "mic", "out", "voice", "in"),
    "voice",
    "out",
    "headphones",
    "in",
  );
  await page.addInitScript(
    (session) => Object.assign(window, { __routeFixtureSession: session, __routeFixtureRunning: true }),
    routed,
  );
  await page.goto("/route-harness.html");
  const message = page.locator(".global-action-message");
  await page.locator(".topbar").getByRole("button", { name: "Stop", exact: true }).click();
  await expect(page.locator(".topbar").getByRole("button", { name: "Play", exact: true })).toBeVisible();
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await page.getByRole("tab", { name: "Properties" }).click();
  const name = page.locator(".inspector-grid").getByLabel("Node name");
  await name.fill("Voice gain renamed");
  await name.press("Tab");
  await page.locator(".topbar").getByRole("button", { name: "Save", exact: true }).click();
  await expect(message).toContainText("Route saved");
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  await page.keyboard.press("Control+Alt+S");
  await expect(page.locator(".audio-run-state")).toContainText("Audio running");
  await expect(message).toContainText("Audio session is running");
  await expect(message).not.toContainText("Temporary preview");
});
