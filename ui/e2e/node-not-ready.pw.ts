import { openDeviceTroubleshooting, setColorTheme, stereoDemoSession } from "./workbench";
import { expect, test } from "./browser-test";

// A refused Play names its node (owner report, 2026-10-24: "audio source
// media is missing" without saying which tool): the canvas marks that node
// without resizing it, and Properties opens on it.
const refusal = {
  code: -32000,
  message:
    '"Voice gain" cannot play: AudioRouter no longer has "Temporary voice take.wav". Choose the file again in its Properties.',
  data: {
    code: "nodeNotReady",
    fieldPath: "nodes/voice",
    resourceIds: ["voice"],
    retryable: false,
    remediation: "open the named node's Properties, fix it, then press Play again",
  },
};

test("a refused Play marks the node it names, until Play works", async ({ page }, testInfo) => {
  await page.addInitScript(
    ({ session, startError }) => {
      Object.assign(window, { __routeFixtureSession: session, __routeFixtureStartError: startError });
    },
    { session: stereoDemoSession, startError: refusal },
  );
  await page.goto("/route-harness.html");
  await openDeviceTroubleshooting(page);
  const sidebar = page.locator(".right-workbench");
  await sidebar.getByLabel("Native capture endpoint").selectOption("capture-preview");
  await sidebar.getByLabel("Native render endpoint").selectOption("render-preview");
  const card = page.getByTestId("rf__node-voice");
  const before = (await card.boundingBox())!;

  await page.locator(".topbar").getByRole("button", { name: "Play", exact: true }).click();
  await expect(page.locator(".global-action-message")).toContainText('"Voice gain" cannot play');
  await expect(page.locator(".global-action-message")).not.toContainText("nodeNotReady");
  const tag = card.locator(".node-issue-tag");
  await expect(tag).toHaveText("! Cannot play");
  await expect(tag).toHaveAttribute("title", refusal.message);
  await expect(card).toHaveClass(/has-node-issue/);
  expect(await card.boundingBox()).toEqual(before);
  // Properties opens on the node to fix.
  await expect(page.getByRole("tab", { name: "Properties", exact: true })).toHaveAttribute("aria-selected", "true");
  await expect(card).toHaveClass(/selected/);
  await expect(page.locator(".main-content > .inspector")).toContainText("Voice gain");
  await expect(page.getByTestId("rf__node-headphones").locator(".node-issue-tag")).toHaveCount(0);

  for (const theme of ["dark", "light", "high-contrast"]) {
    await setColorTheme(page, theme);
    // The tag and outline sit outside the card's box.
    await page.screenshot({
      path: testInfo.outputPath(`node-not-ready-${theme}.png`),
      clip: { x: before.x - 16, y: before.y - 24, width: before.width + 32, height: before.height + 40 },
    });
  }

  // Once Play works, the mark goes away.
  await page.evaluate(() => Object.assign(window, { __routeFixtureStartError: undefined }));
  await page.locator(".topbar").getByRole("button", { name: "Play", exact: true }).click();
  await expect(page.locator(".audio-run-state")).toContainText("Audio running");
  await expect(tag).toHaveCount(0);
  await expect(card).not.toHaveClass(/has-node-issue/);
});
