import { expect, test, type Page } from "./browser-test";
import { openPage } from "./workbench";
import type { DeviceListItem, ObsStatus, Session } from "@audiorouter/contracts";

// The Apps tab's OBS card against a simulated live OBS (D1/O1/O2). The route
// has one microphone feeding CABLE-A; OBS records the raw microphone, which is
// the fixable problem the card exists to explain.
const session: Session = {
  id: "obs-card-route",
  name: "Streaming",
  schemaVersion: 1,
  revision: 1,
  nodes: [
    {
      id: "mic",
      kind: "physicalInput",
      typeVersion: 1,
      name: "Microphone",
      enabled: true,
      bypass: false,
      parameters: { endpointId: "mic-endpoint" },
      ports: [{ name: "out", direction: "output", channels: 1 }],
    },
    {
      id: "gain",
      kind: "gain",
      typeVersion: 1,
      name: "Voice gain",
      enabled: true,
      bypass: false,
      parameters: { gainDb: 0 },
      ports: [
        { name: "in", direction: "input", channels: 1 },
        { name: "out", direction: "output", channels: 1 },
      ],
    },
    {
      id: "voice-out",
      kind: "physicalOutput",
      typeVersion: 1,
      name: "Voice to Discord",
      enabled: true,
      bypass: false,
      parameters: { endpointId: "cable-a-in" },
      ports: [{ name: "in", direction: "input", channels: 1 }],
    },
  ],
  edges: [
    {
      id: "mic-gain",
      sourceNode: "mic",
      sourcePort: "out",
      destinationNode: "gain",
      destinationPort: "in",
      matrix: [1],
      enabled: true,
    },
    {
      id: "gain-out",
      sourceNode: "gain",
      sourcePort: "out",
      destinationNode: "voice-out",
      destinationPort: "in",
      matrix: [1],
      enabled: true,
    },
  ],
};

const format = { sampleRateHz: 48_000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 };
const devices: DeviceListItem[] = [
  {
    id: "mic-endpoint",
    name: "Microphone (PD200X Podcast Microphone)",
    direction: "capture",
    state: "active",
    defaultRoles: [],
    format,
    periods: { default100ns: 100_000, minimum100ns: 30_000 },
  },
  {
    id: "cable-a-in",
    name: "CABLE-A Input (VB-Audio Virtual Cable A)",
    direction: "render",
    state: "active",
    defaultRoles: [],
    format,
    periods: { default100ns: 100_000, minimum100ns: 30_000 },
  },
  {
    id: "cable-a-out",
    name: "CABLE-A Output (VB-Audio Virtual Cable A)",
    direction: "capture",
    state: "active",
    defaultRoles: [],
    format,
    periods: { default100ns: 100_000, minimum100ns: 30_000 },
  },
  {
    id: "speakers",
    name: "Speakers (Focusrite USB Audio)",
    direction: "render",
    state: "active",
    defaultRoles: ["console"],
    format,
    periods: { default100ns: 100_000, minimum100ns: 30_000 },
  },
];

const status: ObsStatus = {
  installed: true,
  connection: "connected",
  source: "live",
  sceneCollection: "Streaming",
  passwordSource: "obsSettings",
  lastChange: { state: "none", changed: 0, restorable: false, error: null },
  inputs: [
    {
      name: "Mic/Aux",
      uuid: "uuid-mic",
      kind: "inputCapture",
      special: "mic1",
      deviceId: "mic-endpoint",
      application: null,
      muted: false,
      active: true,
      audioTracks: [1],
      monitorOnly: false,
    },
    {
      name: "Desktop Audio",
      uuid: "uuid-desktop",
      kind: "outputCapture",
      special: "desktop1",
      deviceId: "default",
      application: null,
      muted: false,
      active: true,
      audioTracks: [1],
      monitorOnly: false,
    },
  ],
};

async function openApps(page: Page, theme: string) {
  await page.addInitScript(
    ({ theme, session, devices, status }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, {
        __routeFixtureSession: session,
        __routeFixtureDevices: devices,
        __routeFixtureObsStatus: status,
      });
    },
    { theme, session, devices, status },
  );
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await openPage(page, "Apps");
  return page.locator('[data-app="obs"]');
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: the OBS card shows what OBS hears and keeps its offsets while polling`, async ({
    page,
  }, testInfo) => {
    await page.setViewportSize({ width: 1440, height: 1000 });
    const card = await openApps(page, theme);

    await expect(card.getByRole("heading", { name: "OBS Studio" })).toBeVisible();
    await expect(card.getByRole("table", { name: "What OBS hears" })).toBeVisible();
    await expect(card).toContainText("Mic/Aux");
    await expect(card).toContainText("Microphone (PD200X Podcast Microphone)");
    await expect(card).toContainText("Records your microphone without AudioRouter's effects.");
    await expect(card.getByRole("button", { name: "Fix in OBS" })).toBeEnabled();
    await expect(card.locator(".app-state-badge")).toHaveText("Check");

    // A repeated status poll must not move the card or its table (UI-17).
    const offsets = () =>
      page.evaluate(() => {
        const box = (selector: string) => {
          const element = document.querySelector(selector);
          if (!element) return null;
          // Coordinates inside the scroll container: a real layout shift, not
          // the scroll a click or the dialog may cause. The narrow layout
          // scrolls an inner panel, so window.scrollY alone is not enough.
          let top = 0;
          let left = 0;
          for (let node: Element | null = element; node; node = node.parentElement) {
            top += node.scrollTop;
            left += node.scrollLeft;
          }
          const rect = element.getBoundingClientRect();
          return { top: Math.round(rect.top + top), left: Math.round(rect.left + left) };
        };
        return {
          card: box('[data-app="obs"]'),
          table: box('[data-app="obs"] .obs-sources'),
          summary: box('[data-app="obs"] .app-card-summary'),
        };
      });
    const before = await offsets();
    await page.waitForTimeout(2_500);
    expect(await offsets()).toEqual(before);

    // Opening and cancelling the fix dialog leaves the card where it was.
    await card.getByRole("button", { name: "Fix in OBS" }).click();
    const dialog = page.getByRole("dialog", { name: "Fix in OBS" });
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText("Mic/Aux");
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    expect(await offsets()).toEqual(before);

    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await card.screenshot({ path: testInfo.outputPath(`${theme}-obs-card.png`) });
  });

  test(`${theme}: the OBS card fits a narrow window without sideways scrolling`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: 640, height: 690 });
    const card = await openApps(page, theme);
    await expect(card.getByRole("table", { name: "What OBS hears" })).toBeVisible();
    const bounds = (await card.boundingBox())!;
    expect(bounds.x).toBeGreaterThanOrEqual(0);
    expect(bounds.x + bounds.width).toBeLessThanOrEqual(640);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await card.screenshot({ path: testInfo.outputPath(`${theme}-obs-card-narrow.png`) });
  });
}
