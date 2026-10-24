import { setColorTheme } from "./workbench";
import { expect, test, type Page } from "./browser-test";
import type { ApplicationInfo, DeviceListItem, Session } from "../../contracts/src/index";

// D2: "Set up for Discord" adds one undoable route, then the Discord card
// checks Discord against it live.

const format = { sampleRateHz: 48_000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 };
const periods = { default100ns: 100_000, minimum100ns: 30_000 };
const device = (
  id: string,
  name: string,
  direction: "capture" | "render",
  defaultRoles: DeviceListItem["defaultRoles"] = [],
): DeviceListItem => ({ id, name, direction, state: "active", defaultRoles, format, periods });
const devices = [
  device("mic", "Microphone (PD200X Podcast Microphone)", "capture", ["communications", "console"]),
  device("headphones", "Speakers (Focusrite USB Audio)", "render", ["console"]),
  device("cable-a-in", "CABLE-A Input (VB-Audio Virtual Cable A)", "render"),
  device("cable-a-out", "CABLE-A Output (VB-Audio Virtual Cable A)", "capture"),
  device("cable-a-out-16", "CABLE-A Out 16ch (VB-Audio Virtual Cable A)", "capture"),
];

const port = (name: string, direction: "input" | "output") => ({ name, direction, channels: 2 as const });
const node = (
  id: string,
  kind: "physicalInput" | "physicalOutput",
  name: string,
  endpointId: string,
): Session["nodes"][number] => ({
  id,
  kind,
  name,
  typeVersion: 1,
  enabled: true,
  bypass: false,
  parameters: { endpointId },
  ports: [kind === "physicalInput" ? port("out", "output") : port("in", "input")],
});
// A new user's route: the microphone straight to the headphones, no cable.
const session: Session = {
  id: "discord-setup",
  name: "Monitor",
  revision: 1,
  schemaVersion: 1,
  nodes: [
    node("mic", "physicalInput", "Microphone", "mic"),
    node("headphones", "physicalOutput", "Headphones", "headphones"),
  ],
  edges: [
    {
      id: "mic-headphones",
      sourceNode: "mic",
      sourcePort: "out",
      destinationNode: "headphones",
      destinationPort: "in",
      enabled: true,
      matrix: [1, 0, 0, 1],
    },
  ],
};

const discord = (microphone: string): ApplicationInfo[] => [
  {
    processId: 7,
    executable: "Discord.exe",
    executablePath: null,
    creationTime100ns: null,
    audioActivity: "active",
    captureCapability: "observed",
    audioSessionCount: 2,
    activeAudioSessionCount: 2,
    captureSessionCount: 1,
    renderSessionCount: 1,
    audioDisplayNames: [],
    audioEndpoints: [
      { deviceId: microphone, direction: "capture", active: true },
      { deviceId: "headphones", direction: "render", active: true },
    ],
  },
];

/** What Windows reports changes; the window regains focus, as after a Discord change. */
async function report(page: Page, applications: ApplicationInfo[]) {
  await page.evaluate((value) => {
    Object.assign(window, { __routeFixtureApplications: value });
    window.dispatchEvent(new Event("focus"));
  }, applications);
}

test("Set up for Discord adds one undoable route that the Discord card then checks", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.addInitScript(
    ({ fixture, applications, deviceList }) => {
      Object.assign(window, {
        __routeFixtureSession: fixture,
        __routeFixtureApplications: applications,
        __routeFixtureDevices: deviceList,
      });
    },
    { fixture: session, applications: discord("mic"), deviceList: devices },
  );
  await page.goto("/route-harness.html");
  await page.getByRole("tab", { name: "Apps", exact: true }).click();
  const card = page.locator('[data-app="discord"]');
  const microphoneRow = card.locator(".app-check-row").first();
  await expect(microphoneRow).toContainText("does not send your voice to a virtual cable yet");
  await microphoneRow.getByRole("button", { name: "Set up for Discord" }).click();

  const setup = page.locator(".discord-setup");
  await expect(setup.getByLabel("Microphone")).toHaveValue("mic");
  await expect(setup.getByLabel("Voice cable")).toHaveValue("cable-a-in");
  // The 16ch twin is never offered.
  await expect(setup.getByLabel("Voice cable").locator("option")).toHaveCount(1);
  const review = setup.getByRole("status");
  await expect(review).toContainText("→ Gate → Compressor → Limiter → CABLE-A Input");
  const withCleanup = (await review.boundingBox())!;
  const addButton = setup.getByRole("button", { name: "Add route" });
  const buttonY = (await addButton.boundingBox())!.y;
  await setup.getByRole("checkbox").uncheck();
  await expect(review).not.toContainText("Gate");
  expect((await review.boundingBox())!.height).toBe(withCleanup.height);
  expect((await addButton.boundingBox())!.y).toBe(buttonY);
  await setup.getByRole("checkbox").check();
  for (const theme of ["dark", "light", "high-contrast"]) {
    await setColorTheme(page, theme);
    await setup.screenshot({ path: testInfo.outputPath(`discord-setup-step1-${theme}.png`) });
  }
  await setColorTheme(page, "dark");

  await addButton.click();
  await expect(setup).toContainText("Step 2 of 2");
  await expect(setup).toContainText("set Input Device to CABLE-A Output (VB-Audio Virtual Cable A)");
  for (const id of ["gate-1", "compressor-1", "limiter-1", "physicalOutput-1"])
    await expect(page.getByTestId(`rf__node-${id}`)).toBeVisible();
  // Discord still records the raw microphone: the card now names the cable.
  await expect(microphoneRow).toContainText("without AudioRouter's effects");
  await expect(card.locator(".app-state-badge")).toContainText("Check");
  for (const theme of ["dark", "light", "high-contrast"]) {
    await setColorTheme(page, theme);
    await setup.screenshot({ path: testInfo.outputPath(`discord-setup-step2-${theme}.png`) });
  }
  await setColorTheme(page, "dark");

  // The user follows step 2 in Discord.
  await report(page, discord("cable-a-out"));
  await expect(card.locator(".app-state-badge")).toContainText("Ready");
  await expect(microphoneRow).toContainText("Carries your processed voice.");

  // One Undo removes the whole route.
  await page.locator(".topbar").getByRole("button", { name: "Undo", exact: true }).click();
  for (const id of ["gate-1", "compressor-1", "limiter-1", "physicalOutput-1"])
    await expect(page.getByTestId(`rf__node-${id}`)).toHaveCount(0);
  await expect(page.getByTestId("rf__node-mic")).toBeVisible();
  await expect(microphoneRow.getByRole("button", { name: "Set up for Discord" })).toBeVisible();
  // The flow returns to step 1 rather than claiming a route that is gone.
  await expect(setup).toContainText("Step 1 of 2");
  await expect(setup.getByRole("button", { name: "Add route" })).toBeVisible();
});
