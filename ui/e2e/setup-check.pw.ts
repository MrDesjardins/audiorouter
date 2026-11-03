import { test, expect } from "./browser-test";
import type { SetupCheckResult } from "@audiorouter/contracts";

// UI-27: the Status tab's Setup check in every theme. The result slot keeps
// its size between a long result and an empty one (UI-17).
const findings: SetupCheckResult = {
  findings: [
    {
      code: "outputFormat",
      severity: "problem",
      sessionId: "test",
      sessionName: "Continuity test",
      nodeId: "out",
      nodeName: "Result",
      deviceId: "cable-b-in",
      deviceName: "CABLE-B Input",
      message:
        '"Result" plays to CABLE-B Input, which is set to 8 channel(s) at 48 kHz. AudioRouter plays only to stereo devices, so Play is refused.',
      fix: "In Windows Sound settings, set CABLE-B Input to Stereo (Configure) at 48 kHz, or choose another output. If a game sends surround to this device, keep it and pick a different device for this output.",
    },
    {
      code: "crossSessionLoop",
      severity: "warning",
      sessionId: "test",
      sessionName: "Continuity test",
      otherSessionId: "main",
      message:
        'Playing "Continuity test" while "Patrick Main Session" plays makes a feedback loop: each sends audio into a device the other records.',
      fix: 'Stop "Patrick Main Session" first, or send "Continuity test" to a different cable.',
    },
    {
      code: "cableRate",
      severity: "tip",
      sessionId: "main",
      sessionName: "Patrick Main Session",
      nodeId: "game-out",
      nodeName: "Game Sound Out",
      deviceId: "cable-in",
      deviceName: "CABLE Input",
      message: "CABLE Input runs at 44.1 kHz. AudioRouter works at 48 kHz, so Windows converts this cable's audio.",
      fix: "In Windows Sound settings, set CABLE Input to 48000 Hz (Properties → Advanced).",
    },
  ],
  truncated: 0,
  sessionsChecked: 2,
  devicesChecked: 8,
  notChecked: [
    "Settings inside other apps, such as a game's speaker setup or the devices Discord uses (see the Apps tab).",
  ],
};

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`setup check findings in ${theme}`, async ({ page }, info) => {
    await page.addInitScript(
      ([theme, result]) => {
        localStorage.setItem("audiorouter.ui.theme", theme as string);
        Object.assign(window, { __routeFixtureSetupCheck: result });
      },
      [theme, findings] as const,
    );
    await page.goto("/route-harness.html");
    await page.getByRole("tab", { name: "Status", exact: true }).click();
    const panel = page.locator("section.setup-check");
    await expect(panel.getByText("1 problem, 1 warning, 1 tip in 2 sessions.")).toBeVisible();
    await expect(panel.getByText(/Problem: "Result" plays to CABLE-B Input/)).toBeVisible();
    await expect(panel.getByText("Session Patrick Main Session · Game Sound Out")).toBeVisible();
    const devicesHeading = page.getByRole("heading", { name: "Audio devices on this PC" });
    const withFindings = await devicesHeading.boundingBox();
    await page.screenshot({ path: info.outputPath(`${theme}-setup-check.png`) });

    await page.evaluate(() =>
      Object.assign(window, {
        __routeFixtureSetupCheck: { findings: [], truncated: 0, sessionsChecked: 2, devicesChecked: 8, notChecked: [] },
      }),
    );
    await panel.getByRole("button", { name: "Check again" }).click();
    await expect(panel.getByText("No problems found in 2 sessions.")).toBeVisible();
    await expect(panel.getByText("Every device your sessions use fits how they use it.")).toBeVisible();
    const withoutFindings = await devicesHeading.boundingBox();
    expect(withoutFindings?.y).toBe(withFindings?.y);
    await page.screenshot({ path: info.outputPath(`${theme}-setup-check-clean.png`) });
  });
}
