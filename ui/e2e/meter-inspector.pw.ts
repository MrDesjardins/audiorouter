import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";
const session = {
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
  test(`large pass-through Meter and reset in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, {
          __routeFixtureSession: session,
          __routeFixtureRunning: true,
          __routeFixtureTelemetry: [
            {
              nodeId: "voice",
              kind: "meter",
              processor: null,
              plugin: null,
              meter: {
                peakDb: 3.5,
                rmsDb: -18,
                currentPeakDb: -6,
                channelCurrentPeakDb: [-6, -9],
                channelPeakDb: [3.5, -1],
                channelRmsDb: [-18, -21],
                channelClippedSamples: [480, 0],
                clippedSamples: 480,
                observedFrames: 48000,
                sampleRateHz: 48000,
              },
            },
          ],
        });
      },
      { session, theme },
    );
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").click();
    const inspector = page.getByLabel("Detailed Meter");
    await expect(inspector.getByRole("meter")).toHaveCount(2);
    await expect(inspector).toContainText("0.010 s");
    const height = (await inspector.getByRole("meter").first().boundingBox())!.height;
    expect(height).toBeGreaterThanOrEqual(260);
    await expect(page.getByTestId("rf__edge-meter-out").locator("path").first()).toHaveAttribute("d", /^M-?\d/);
    await page.screenshot({ path: testInfo.outputPath(`meter-${theme}.png`) });
    await inspector.getByRole("button", { name: "Reset peak & clipping" }).click();
    await expect(inspector).toContainText("Peak hold and clipping counters reset");
    await expect(inspector).not.toContainText("0.010 s");
    expect(
      await page.evaluate(() =>
        (window as unknown as { __routeFixtureCalls: () => string[] })
          .__routeFixtureCalls()
          .filter((x: string) => x === "commit"),
      ),
    ).toEqual([]);
  });
}
