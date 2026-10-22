import { test, expect, type Page } from "./browser-test";
import { demoSession } from "../src/fixtures";

// Fun Voice Changer (DSP-20): preset choice, edited marker and Reset, tuner
// strip and sections must use the app style in every theme, and editing must
// never move the panel (UI-17).
const neutral = {
  gateDb: -90,
  correctEnabled: false,
  scale: "chromatic",
  retuneMs: 50,
  ringEnabled: false,
  ringHz: 50,
  ringDepthPercent: 100,
  crushEnabled: false,
  crushBits: 16,
  mixPercent: 100,
  outputDb: 0,
};
const robot = {
  ...neutral,
  preset: "robot",
  correctEnabled: true,
  scale: "monotone",
  retuneMs: 0,
  ringEnabled: true,
  ringDepthPercent: 70,
  crushEnabled: true,
  crushBits: 10,
  outputDb: 7,
};
const presets = [
  {
    id: "robot",
    version: 1,
    name: "Robot",
    description: "Flat, buzzing machine voice.",
    basicParameters: ["ringHz", "ringDepthPercent", "crushBits"],
    parameters: robot,
  },
  {
    id: "custom",
    version: 1,
    name: "Custom",
    description: "Everything off: build your own voice.",
    basicParameters: [],
    parameters: { ...neutral, preset: "custom" },
  },
];
const parameter = (name: string, minimum: number, maximum: number, value: number, unit?: string, step?: number) => ({
  name,
  type: "number",
  minimum,
  maximum,
  default: value,
  ...(unit ? { unit } : {}),
  ...(step ? { step } : {}),
});
const processors = [
  {
    id: "funVoiceChanger",
    version: 1,
    category: "effect",
    availability: { status: "available" },
    latencySamples: 1024,
    parameters: [
      { name: "preset", type: "string", enum: ["robot", "custom"], default: "custom" },
      parameter("gateDb", -90, -20, -90, "dBFS"),
      { name: "correctEnabled", type: "boolean", default: false },
      {
        name: "scale",
        type: "string",
        enum: ["chromatic", "major", "minor", "majorPentatonic", "minorPentatonic", "blues", "monotone"],
        default: "chromatic",
      },
      parameter("retuneMs", 0, 400, 50, "ms"),
      { name: "ringEnabled", type: "boolean", default: false },
      parameter("ringHz", 20, 2000, 50, "Hz"),
      parameter("ringDepthPercent", 0, 100, 100, "%"),
      { name: "crushEnabled", type: "boolean", default: false },
      parameter("crushBits", 4, 16, 16, undefined, 1),
      parameter("mixPercent", 0, 100, 100, "%"),
      parameter("outputDb", -24, 12, 0, "dB"),
    ],
  },
];
const session = {
  ...demoSession,
  nodes: demoSession.nodes.map((node) =>
    node.id === "voice" ? { ...node, kind: "funVoiceChanger", name: "Robot voice", parameters: robot } : node,
  ),
};
const telemetry = [
  {
    nodeId: "voice",
    kind: "funVoiceChanger",
    meter: null,
    plugin: null,
    processor: null,
    voiceChanger: { voiced: true, f0Hz: 220.9, note: 57, cents: 6.8 },
  },
];

async function open(page: Page, theme: string) {
  await page.addInitScript(
    ({ session, telemetry, theme, presets, processors }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, {
        __routeFixtureSession: session,
        __routeFixtureRunning: true,
        __routeFixtureTelemetry: telemetry,
        __routeFixtureProcessors: processors,
        __routeFixtureDiscovery: { presets: { voiceChains: [], eq: [], voiceChanger: presets } },
      });
    },
    { session, telemetry, theme, presets, processors },
  );
  await page.goto("/route-harness.html");
  await page.getByTestId("rf__node-voice").click();
  const editor = page.getByLabel("Fun Voice Changer editor");
  await expect(editor.getByLabel("Voice preset")).toHaveValue("robot");
  return editor;
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`Fun Voice Changer in ${theme}`, async ({ page }, testInfo) => {
    const editor = await open(page, theme);
    await expect(editor.getByLabel("Detected note")).toHaveText("A3 +7 ¢ · 221 Hz");
    await expect(editor.locator(".voice-changer-edited")).toHaveClass(/is-hidden/);
    await expect(editor.getByRole("button", { name: "Reset to preset" })).toBeDisabled();
    // Basic view shows the preset's sliders.
    await expect(editor.locator(".voice-changer-basic").getByLabel("Ring frequency slider")).toBeVisible();
    await editor.scrollIntoViewIfNeeded();
    await editor.screenshot({ path: testInfo.outputPath(`fun-voice-changer-${theme}.png`) });
  });
}

test("editing a preset marks it edited without moving the panel, and Reset restores it", async ({ page }) => {
  const editor = await open(page, "dark");
  const sections = editor.locator(".voice-changer-sections");
  const before = await sections.boundingBox();
  await editor.locator(".voice-changer-basic").getByLabel("Ring frequency precise value").fill("80");
  await editor.locator(".voice-changer-basic").getByLabel("Ring frequency precise value").press("Enter");
  await expect(editor.locator(".voice-changer-edited")).not.toHaveClass(/is-hidden/);
  await expect(editor.getByRole("button", { name: "Reset to preset" })).toBeEnabled();
  const after = await sections.boundingBox();
  expect(after?.y).toBe(before?.y);
  await editor.getByRole("button", { name: "Reset to preset" }).click();
  await expect(editor.locator(".voice-changer-edited")).toHaveClass(/is-hidden/);
  await expect(editor.locator(".voice-changer-basic").getByLabel("Ring frequency precise value")).toHaveValue("50");
  // Custom: every section opens.
  await editor.getByLabel("Voice preset").selectOption("custom");
  await expect(editor.locator(".voice-changer-sections details[open]")).toHaveCount(9);
  await expect(editor).toContainText("Custom starts with every effect off");
});
