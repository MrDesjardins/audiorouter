import { expect, test } from "./browser-test";
import { demoSession } from "../src/fixtures";
import type { Page } from "@playwright/test";

const THEMES = ["dark", "light", "high-contrast"];

const fresh = { ...demoSession, id: "desktop-session", name: "My route", revision: 0 };

/** Removes the marker the shared fixture sets, so this page is a true first run. */
async function asFirstRun(page: Page, session = fresh) {
  await page.addInitScript((value) => {
    localStorage.removeItem("audiorouter.ui.first-run-done");
    Object.assign(window, { __routeFixtureSession: value });
  }, session);
}

test("first run offers goals once, then the tour; Not now ends it for good", async ({ page }, info) => {
  await asFirstRun(page);
  await page.goto("/route-harness.html");
  const dialog = page.getByRole("dialog", { name: "What do you want to do?" });
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText("Nothing plays until you press Play");
  await page.screenshot({ path: info.outputPath("first-run.png") });
  await dialog.getByRole("button", { name: "Use this: Clean up my mic" }).click();
  await expect(dialog).toHaveCount(0);

  const step = page.getByRole("dialog", { name: "Add tools from here" });
  await expect(step).toBeVisible();
  await expect(step).toContainText("1");
  await page.getByRole("button", { name: "Next" }).click();
  await page.getByRole("button", { name: "Next" }).click();
  await page.getByRole("button", { name: "Done" }).click();
  await expect(page.locator(".tour-layer")).toHaveCount(0);

  const calls = await page.evaluate(() =>
    (window as never as { __routeFixtureCalls(): string[] }).__routeFixtureCalls(),
  );
  expect(calls.filter((call) => /start|prepare/i.test(call))).toEqual([]);
  expect(await page.evaluate(() => localStorage.getItem("audiorouter.ui.first-run-done"))).toBe("true");

  await page.reload();
  await expect(page.locator(".topbar")).toBeVisible();
  await expect(dialog).toHaveCount(0);
  await expect(page.locator(".tour-layer")).toHaveCount(0);
});

test("Not now on first run creates nothing and does not ask again", async ({ page }) => {
  await asFirstRun(page);
  await page.goto("/route-harness.html");
  const dialog = page.getByRole("dialog", { name: "What do you want to do?" });
  await dialog.getByRole("button", { name: "Not now" }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.locator(".tour-layer")).toHaveCount(0);
  expect(
    await page.evaluate(() => (window as never as { __routeFixtureGoalCalls(): string[] }).__routeFixtureGoalCalls()),
  ).toEqual([]);
  await page.reload();
  await expect(page.locator(".topbar")).toBeVisible();
  await expect(dialog).toHaveCount(0);
});

test("someone who already has a saved route is not interrupted", async ({ page }) => {
  await asFirstRun(page, { ...fresh, id: "mine", revision: 4 });
  await page.goto("/route-harness.html");
  await expect(page.locator(".topbar")).toBeVisible();
  await page.waitForTimeout(400);
  await expect(page.getByRole("dialog", { name: "What do you want to do?" })).toHaveCount(0);
});

const micNode = demoSession.nodes.find((node) => node.kind === "physicalInput")!;

async function playing(page: Page, theme: string, peak: number) {
  await page.addInitScript(
    ({ theme, id, peak, session }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, {
        __routeFixtureSession: session,
        __routeFixtureRunning: true,
        __routeFixturePrivacyMuted: false,
        __routeFixtureTelemetry: [
          {
            nodeId: id,
            kind: "physicalInput",
            state: "running",
            processor: null,
            plugin: null,
            meter: {
              peakDb: peak,
              rmsDb: peak - 3,
              clippedSamples: 0,
              channelPeakDb: [peak],
              channelRmsDb: [peak - 3],
              channelClippedSamples: [0],
              currentPeakDb: peak,
              channelCurrentPeakDb: [peak],
            },
          },
        ],
      });
    },
    { theme, id: micNode.id, peak, session: demoSession },
  );
}

for (const theme of THEMES) {
  test(`mic test keeps one card size through every state in ${theme}`, async ({ page }, info) => {
    await playing(page, theme, -12);
    await page.goto("/route-harness.html");
    await page.getByRole("tablist", { name: "Right sidebar" }).getByRole("tab", { name: "Status" }).click();
    const card = page.locator(".mic-test");
    await card.scrollIntoViewIfNeeded();
    const height = async () => (await card.boundingBox())!.height;
    const before = await height();
    await page.screenshot({ path: info.outputPath(`${theme}-mic-idle.png`) });
    await card.getByRole("button", { name: "Test my mic" }).click();
    await expect(card).toContainText("Listening for 5 seconds…");
    expect(await height()).toBe(before);
    await page.screenshot({ path: info.outputPath(`${theme}-mic-listening.png`) });
    await expect(card).toContainText("Good: we hear you, in the voice zone", { timeout: 7000 });
    await expect(card).toContainText("It cannot tell what other apps hear.");
    await expect(card).toContainText("Peak −12 dB");
    expect(await height()).toBe(before);
    await page.screenshot({ path: info.outputPath(`${theme}-mic-good.png`) });
  });
}

test("mic test says plainly when no sound arrived, and changes nothing", async ({ page }, info) => {
  await playing(page, "dark", -90);
  await page.goto("/route-harness.html");
  await page.getByRole("tablist", { name: "Right sidebar" }).getByRole("tab", { name: "Status" }).click();
  const card = page.locator(".mic-test");
  const before = (await card.boundingBox())!.height;
  await card.getByRole("button", { name: "Test my mic" }).click();
  await expect(card).toContainText("No sound from this mic", { timeout: 7000 });
  await expect(card.getByRole("button", { name: "Open Windows sound settings" })).toBeVisible();
  expect((await card.boundingBox())!.height).toBe(before);
  await page.screenshot({ path: info.outputPath("mic-silent.png") });
  const calls = await page.evaluate(() =>
    (window as never as { __routeFixtureCalls(): string[] }).__routeFixtureCalls(),
  );
  expect(calls).toEqual([]);
});

async function openProperties(page: Page, theme: string) {
  const session = {
    ...demoSession,
    nodes: demoSession.nodes.map((node) =>
      node.id === "voice"
        ? { ...node, kind: "compressor" as const, name: "Voice", parameters: { thresholdDb: -24, ratio: 3 } }
        : node,
    ),
  };
  await playing(page, theme, -20);
  await page.addInitScript((value) => Object.assign(window, { __routeFixtureSession: value }), session);
  await page.goto("/route-harness.html");
  await page.locator(".react-flow__node").filter({ hasText: "Voice" }).first().locator(".flow-node-title").click();
  await page.getByRole("tablist", { name: "Right sidebar" }).getByRole("tab", { name: "Properties" }).click();
}

const compareCalls = (page: Page) =>
  page.evaluate(() => (window as never as { __routeFixtureCompareCalls(): string[][] }).__routeFixtureCompareCalls());

for (const theme of THEMES) {
  test(`hold to compare shows its warning and lets go in ${theme}`, async ({ page }, info) => {
    await openProperties(page, theme);
    const hold = page.getByRole("button", { name: /without this tool/ });
    await expect(page.getByText(/anyone hearing this route, on Discord or a stream, hears it too/)).toBeVisible();
    await hold.scrollIntoViewIfNeeded();
    const box = (await hold.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await expect.poll(() => compareCalls(page)).toEqual([["voice"]]);
    // A held tool sends no readings; its live panel must not report that as a fault.
    await expect(page.getByText("Comparing: this tool is out of the route while you hold.")).toBeAttached();
    await expect(page.getByText(/sends no readings/)).toHaveCount(0);
    await page.screenshot({ path: info.outputPath(`${theme}-compare-held.png`) });
    await page.mouse.up();
    await expect.poll(() => compareCalls(page)).toEqual([["voice"], []]);
    await expect(page.getByText("Comparing: this tool is out of the route while you hold.")).toHaveCount(0);
  });
}

test("a hold ends when the pointer leaves the window's focus or the panel closes", async ({ page }) => {
  await openProperties(page, "dark");
  const hold = page.getByRole("button", { name: /without this tool/ });
  await hold.scrollIntoViewIfNeeded();
  const box = (await hold.boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await expect.poll(() => compareCalls(page)).toHaveLength(1);
  await page.evaluate(() => window.dispatchEvent(new Event("blur")));
  await expect.poll(() => compareCalls(page)).toEqual([["voice"], []]);
  await page.mouse.up();

  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await expect.poll(() => compareCalls(page)).toHaveLength(3);
  // Leave the panel with the keyboard-free route a stuck hold would take.
  await page.getByRole("tablist", { name: "Right sidebar" }).getByRole("tab", { name: "Tools" }).dispatchEvent("click");
  await expect.poll(() => compareCalls(page)).toHaveLength(4);
  expect((await compareCalls(page))[3]).toEqual([]);
});
