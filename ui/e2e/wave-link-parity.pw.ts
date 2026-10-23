import { test, expect } from "./browser-test";
import { demoSession } from "../src/fixtures";

// Wave Link parity, first round (docs/plans/active/wave-link-parity.md):
// the Meter's voice target zone (UI-21) and devices hidden from pickers
// (UI-22), each combined with the existing Meter, Status and Properties.

const meterSession = {
  ...demoSession,
  nodes: demoSession.nodes.map((n) =>
    n.id === "voice"
      ? {
          ...n,
          kind: "meter",
          name: "Voice Meter",
          parameters: {},
          ports: [
            { name: "in", direction: "input", channels: 2 },
            { name: "out", direction: "output", channels: 2 },
          ],
        }
      : n,
  ),
  edges: [
    {
      id: "mic-meter",
      sourceNode: "mic",
      sourcePort: "out",
      destinationNode: "voice",
      destinationPort: "in",
      matrix: [1, 1],
      enabled: true,
    },
    {
      id: "meter-out",
      sourceNode: "voice",
      sourcePort: "out",
      destinationNode: "headphones",
      destinationPort: "in",
      matrix: [1, 0, 0, 1],
      enabled: true,
    },
  ],
};

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`voice target zone follows talk without moving the Meter in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        // Talk with pauses: silence, quiet, in the zone, loud, too loud.
        const pattern = [-120, -30, -12, -4, -0.5, -12, -120];
        let tick = 0;
        const frame = () => {
          const peak = pattern[Math.floor(tick++ / 6) % pattern.length];
          return [
            {
              nodeId: "voice",
              kind: "meter",
              processor: null,
              plugin: null,
              meter: {
                peakDb: peak,
                rmsDb: peak - 10,
                currentPeakDb: peak,
                channelCurrentPeakDb: [peak, peak - 3],
                channelPeakDb: [-0.5, -3.5],
                channelRmsDb: [peak - 10, peak - 13],
                channelClippedSamples: [0, 0],
                clippedSamples: 0,
                observedFrames: 48000,
                sampleRateHz: 48000,
              },
            },
          ];
        };
        Object.assign(window, {
          __routeFixtureSession: session,
          __routeFixtureRunning: true,
          __routeFixtureTelemetry: frame(),
        });
        setInterval(() => Object.assign(window, { __routeFixtureTelemetry: frame() }), 50);
      },
      { session: meterSession, theme },
    );
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").click();
    const inspector = page.getByLabel("Detailed Meter");
    const verdict = inspector.locator(".meter-level-verdict");
    await expect(inspector.getByRole("meter")).toHaveCount(2);
    await expect(inspector.locator(".meter-target-zone")).toHaveCount(2);

    // Every verdict appears, and the meter below the verdict never moves (UI-17).
    const sample = await inspector.evaluate(async (element) => {
      const verdict = element.querySelector(".meter-level-verdict") as HTMLElement;
      const display = element.querySelector(".meter-inspector-display")!;
      const offsets = new Set<number>();
      const heights = new Set<number>();
      const verdicts = new Set<string>();
      const until = performance.now() + 6000;
      while (performance.now() < until) {
        offsets.add(Math.round(display.getBoundingClientRect().top - element.getBoundingClientRect().top));
        heights.add(Math.round(verdict.getBoundingClientRect().height));
        verdicts.add(verdict.dataset.verdict ?? "");
        await new Promise((resolve) => setTimeout(resolve, 40));
      }
      return { offsets: [...offsets], heights: [...heights], verdicts: [...verdicts] };
    });
    expect(sample.offsets).toHaveLength(1);
    expect(sample.heights).toHaveLength(1);
    for (const expected of ["zone", "loud", "tooLoud"]) expect(sample.verdicts).toContain(expected);

    // The existing clipping status and readouts are unchanged.
    await expect(inspector.locator(".meter-clip-status")).toHaveText("No clipping recorded");
    await expect(verdict).toBeVisible();
    await inspector.scrollIntoViewIfNeeded();
    await inspector.screenshot({ path: testInfo.outputPath(`meter-zone-${theme}.png`) });
  });

  test(`hidden devices leave the pickers but never a node's own device in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session });
      },
      { session: demoSession, theme },
    );
    await page.goto("/route-harness.html");

    // Status lists every device with a "Show in pickers" switch, on by default.
    await page.getByRole("tab", { name: "Status", exact: true }).click();
    const list = page.locator(".this-pc-devices");
    const show = list.getByLabel("Show Preview Output in device pickers");
    await expect(show).toBeChecked();
    const listTop = await list.evaluate((element) => Math.round(element.getBoundingClientRect().height));
    await pressSpace(show, false);
    await expect(show).not.toBeChecked();
    // Switching changes no layout in the list.
    expect(await list.evaluate((element) => Math.round(element.getBoundingClientRect().height))).toBe(listTop);
    await list.screenshot({ path: testInfo.outputPath(`devices-list-${theme}.png`) });

    // The unbound output's picker no longer offers it and says how many are hidden.
    await page.getByTestId("rf__node-headphones").click();
    await page.getByRole("tab", { name: "Properties", exact: true }).click();
    const picker = page.getByLabel("Physical output endpoint");
    await expect(picker).toHaveValue("");
    await expect(picker.locator("option")).toHaveText([
      "Select render endpoint",
      /Preview Game \(7\.1\)/,
      /1 device hidden/,
    ]);
    await expect(picker.locator('option[value="hidden-devices-note"]')).toBeDisabled();
    const binding = page.getByLabel("Physical output binding");
    await binding.scrollIntoViewIfNeeded();
    await binding.screenshot({ path: testInfo.outputPath(`devices-picker-${theme}.png`) });

    // Show it again and choose it; then hiding it keeps the node's own device.
    await page.getByRole("tab", { name: "Status", exact: true }).click();
    await pressSpace(show, true);
    await page.getByRole("tab", { name: "Properties", exact: true }).click();
    await picker.selectOption("render-preview");
    await expect(picker).toHaveValue("render-preview");
    await page.getByRole("tab", { name: "Status", exact: true }).click();
    await pressSpace(show, false);
    await page.getByRole("tab", { name: "Properties", exact: true }).click();
    await expect(picker).toHaveValue("render-preview");
    await expect(picker.locator("option", { hasText: "Preview Output" })).toHaveCount(1);
    await expect(picker.locator("option", { hasText: "device hidden" })).toHaveCount(0);

    // The hidden list survives a reload.
    await page.reload();
    await page.getByRole("tab", { name: "Status", exact: true }).click();
    await expect(
      page.locator(".this-pc-devices").getByLabel("Show Preview Output in device pickers"),
    ).not.toBeChecked();
  });
}

test("a corrupt hidden-device preference hides nothing", async ({ page }) => {
  await page.addInitScript((session) => {
    localStorage.setItem("audiorouter.ui.hidden-devices", "{not json");
    Object.assign(window, { __routeFixtureSession: session });
  }, demoSession);
  await page.goto("/route-harness.html");
  await page.getByTestId("rf__node-headphones").click();
  await expect(page.getByLabel("Physical output endpoint").locator("option")).toHaveCount(3);
  await page.getByRole("tab", { name: "Status", exact: true }).click();
  await expect(page.locator(".this-pc-devices input[type=checkbox]:not(:checked)")).toHaveCount(0);
});

/** Toggle a switch from the keyboard (UI-11). Workspace notifications float
 * over the sidebar's bottom edge, where the last device can sit, so a
 * pointer click there is intercepted (existing layout, recorded in the plan). */
async function pressSpace(locator: import("@playwright/test").Locator, checked: boolean) {
  await locator.focus();
  await locator.press("Space");
  if (checked) await expect(locator).toBeChecked();
  else await expect(locator).not.toBeChecked();
}
