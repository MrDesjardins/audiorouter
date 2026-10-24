import { setColorTheme } from "./workbench";
import { expect, test, type Page } from "./browser-test";
import type { ApplicationInfo, DeviceListItem, Session } from "../../contracts/src/index";

// D1: device nodes say which apps use their device, and a change in what
// Windows reports never moves the canvas or the Properties panel (UI-17).

const format = { sampleRateHz: 48_000, channels: 2, bitsPerSample: 32, formatTag: 3, bytesPerFrame: 8 };
const periods = { default100ns: 100_000, minimum100ns: 30_000 };
const device = (id: string, name: string, direction: "capture" | "render"): DeviceListItem => ({
  id,
  name,
  direction,
  state: "active",
  defaultRoles: [],
  format,
  periods,
});
const devices = [
  device("mic", "Microphone (PD200X Podcast Microphone)", "capture"),
  device("headphones", "Speakers (Focusrite USB Audio)", "render"),
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
const session: Session = {
  id: "device-used-by",
  name: "Voice route",
  revision: 1,
  schemaVersion: 1,
  nodes: [
    node("mic", "physicalInput", "Microphone", "mic"),
    node("voice", "physicalOutput", "Voice cable", "cable-a-in"),
    node("headphones", "physicalOutput", "Headphones", "headphones"),
  ],
  edges: [
    {
      id: "mic-voice",
      sourceNode: "mic",
      sourcePort: "out",
      destinationNode: "voice",
      destinationPort: "in",
      enabled: true,
      matrix: [1, 0, 0, 1],
    },
  ],
};

const discord = (endpoints: ApplicationInfo["audioEndpoints"]): ApplicationInfo => ({
  processId: 7,
  executable: "Discord.exe",
  executablePath: null,
  creationTime100ns: null,
  audioActivity: "active",
  captureCapability: "observed",
  audioSessionCount: endpoints.length,
  activeAudioSessionCount: endpoints.length,
  captureSessionCount: endpoints.filter((endpoint) => endpoint.direction === "capture").length,
  renderSessionCount: endpoints.filter((endpoint) => endpoint.direction === "render").length,
  audioDisplayNames: [],
  audioEndpoints: endpoints,
});
const onCable = [
  discord([
    { deviceId: "cable-a-out", direction: "capture", active: true },
    { deviceId: "headphones", direction: "render", active: true },
  ]),
];
const onTwin = [
  discord([
    { deviceId: "cable-a-out-16", direction: "capture", active: true },
    { deviceId: "headphones", direction: "render", active: true },
  ]),
];

/** What Windows reports changes; the window regains focus, as after a Discord change. */
async function report(page: Page, applications: ApplicationInfo[]) {
  await page.evaluate((value) => {
    Object.assign(window, { __routeFixtureApplications: value });
    window.dispatchEvent(new Event("focus"));
  }, applications);
}

test("device nodes name the apps on their device without moving anything", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.addInitScript(
    ({ fixture, applications, deviceList }) => {
      Object.assign(window, {
        __routeFixtureSession: fixture,
        __routeFixtureApplications: applications,
        __routeFixtureDevices: deviceList,
      });
    },
    { fixture: session, applications: onCable, deviceList: devices },
  );
  await page.goto("/route-harness.html");
  const voice = page.getByTestId("rf__node-voice");
  const footer = voice.locator(".node-used-by");
  await expect(footer).toHaveText("Heard by Discord");
  await expect(footer).toHaveAttribute(
    "title",
    "Discord records CABLE-A Output (VB-Audio Virtual Cable A), so it hears what this node sends",
  );
  // Discord's speaker: it shares the device, it does not hear this node.
  await expect(page.getByTestId("rf__node-headphones").locator(".node-used-by")).toHaveText("Shared with Discord");
  await expect(page.getByTestId("rf__node-mic").locator(".node-used-by")).toHaveText("No app using it");
  // The footer never widens the card or pushes its ports out.
  const micCard = page.getByTestId("rf__node-mic");
  await micCard.screenshot({ path: testInfo.outputPath("device-used-by-mic-card.png") });
  const cardBox = (await micCard.boundingBox())!;
  const footerBox = (await micCard.locator(".node-used-by").boundingBox())!;
  expect(footerBox.x + footerBox.width).toBeLessThanOrEqual(cardBox.x + cardBox.width);
  // Its text starts where the port lines below start.
  const textLeft = (selector: string) =>
    micCard.locator(selector).evaluate((element) => {
      const range = document.createRange();
      range.selectNodeContents(element);
      return range.getBoundingClientRect().left;
    });
  expect(Math.abs((await textLeft(".node-used-by")) - (await textLeft(".node-port-count")))).toBeLessThan(0.5);
  // A long list of apps on the microphone keeps the card's width.
  const many = ["Discord.exe", "obs64.exe", "Spotify.exe", "RainbowSix.exe", "chrome.exe"].map((executable, index) => ({
    ...discord([{ deviceId: "mic", direction: "capture", active: true }]),
    processId: 20 + index,
    executable,
  }));
  await report(page, many);
  const micFooter = micCard.locator(".node-used-by");
  await expect(micFooter).toContainText("Discord and OBS skip your processing");
  expect((await micCard.boundingBox())!.width).toBe(cardBox.width);
  const longBox = (await micFooter.boundingBox())!;
  expect(longBox.x + longBox.width).toBeLessThanOrEqual(cardBox.x + cardBox.width);
  await micCard.screenshot({ path: testInfo.outputPath("device-used-by-mic-card-long.png") });
  await report(page, onCable);

  await voice.click();
  const section = page.locator(".device-apps");
  await expect(section).toContainText("Discord records CABLE-A Output");
  const below = section.locator("xpath=following-sibling::*[1]");
  const before = {
    card: (await voice.boundingBox())!,
    section: (await section.boundingBox())!,
    below: await below.boundingBox(),
  };

  // Discord moves to the cable's 16ch twin: the footer warns, nothing moves.
  await report(page, onTwin);
  await expect(footer).toHaveText("! Discord is on CABLE-A Out 16ch");
  await expect(footer).toHaveClass(/is-warning/);
  await expect(section.locator("li.is-warning")).toContainText(
    "Discord records from CABLE-A Out 16ch (VB-Audio Virtual Cable A), not CABLE-A Output",
  );
  const after = {
    card: (await voice.boundingBox())!,
    section: (await section.boundingBox())!,
    below: await below.boundingBox(),
  };
  expect(after.card.height).toBe(before.card.height);
  expect(after.section.y).toBe(before.section.y);
  expect(after.section.height).toBe(before.section.height);
  expect(after.below?.y).toBe(before.below?.y);

  for (const theme of ["dark", "light", "high-contrast"]) {
    await setColorTheme(page, theme);
    await page.screenshot({ path: testInfo.outputPath(`device-used-by-${theme}.png`) });
  }
});
