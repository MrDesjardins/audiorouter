import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";

// The user's route: Microphone → Meter → { "Hear my voice", Recorder }.
const node = (
  id: string,
  kind: string,
  name: string,
  ins: boolean,
  outs: boolean,
  parameters: Record<string, unknown> = {},
) => ({
  id,
  kind,
  typeVersion: 1,
  name,
  enabled: true,
  bypass: false,
  parameters,
  ports: [
    ...(ins ? [{ name: "in", direction: "input", channels: 2 }] : []),
    ...(outs ? [{ name: "out", direction: "output", channels: 2 }] : []),
  ],
});
const edge = (id: string, source: string, target: string) => ({
  id,
  sourceNode: source,
  sourcePort: "out",
  destinationNode: target,
  destinationPort: "in",
  matrix: [1, 0, 0, 1],
  enabled: true,
});
const session = {
  ...demoSession,
  nodes: [
    node("mic", "testSignal", "Microphone", false, true),
    node("meter", "meter", "Meter 1", true, true),
    node("hear", "physicalOutput", "Hear my voice", true, false),
    node("rec", "recorder", "Podcast recorder", true, true, {
      format: "wavPcm24",
      autoRecord: false,
      splitMinutes: 10,
    }),
  ],
  edges: [edge("mic-meter", "mic", "meter"), edge("meter-hear", "meter", "hear"), edge("meter-rec", "meter", "rec")],
};

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`one-click Record and Stop on a playing Recorder node in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session, __routeFixtureRunning: true });
      },
      { session, theme },
    );
    await page.goto("/route-harness.html");
    const recorder = page.getByTestId("rf__node-rec");
    await expect(recorder).toBeVisible();
    await recorder.getByRole("button", { name: "Record Podcast recorder" }).click();
    const stop = recorder.getByRole("button", { name: "Stop recording Podcast recorder" });
    await expect(stop).toBeVisible();
    await expect(stop).toContainText(/Stop · 0:0\d/);
    await recorder.locator(".flow-node-title").click();
    const controls = page.getByLabel("Recorder controls");
    await expect(controls.getByRole("status")).toHaveText("Recording");
    await expect(controls).toContainText("WAV 24-bit · new file every 10 min");
    await expect(controls).toContainText("C:\\Recordings\\rec-take.wav".split("\\").pop()!);
    await controls.screenshot({ path: testInfo.outputPath(`recorder-controls-${theme}.png`) });
    await recorder.screenshot({ path: testInfo.outputPath(`recorder-node-${theme}.png`) });
    await controls.getByRole("button", { name: "Stop recording Podcast recorder" }).click();
    await expect(page.locator(".global-action-message")).toContainText("Recording saved.");
    await expect(recorder.getByRole("button", { name: "Record Podcast recorder" })).toBeVisible();
    const calls = await page.evaluate(() =>
      (window as unknown as { __routeFixtureCalls(): string[] }).__routeFixtureCalls(),
    );
    expect(calls.filter((call) => call.includes("record"))).toEqual(["record:rec", "stop-record:rec"]);
  });
}

test("a take that lost audio says so at Stop and keeps the file", async ({ page }) => {
  await page.addInitScript(
    ({ session }) => {
      Object.assign(window, {
        __routeFixtureSession: session,
        __routeFixtureRunning: true,
        __routeFixtureRecordingLostAudio: true,
      });
    },
    { session },
  );
  await page.goto("/route-harness.html");
  const recorder = page.getByTestId("rf__node-rec");
  await recorder.getByRole("button", { name: "Record Podcast recorder" }).click();
  await recorder.getByRole("button", { name: "Stop recording Podcast recorder" }).click();
  // Not "Recording saved.": the user learns the take ended early and why.
  await expect(page.locator(".global-action-message")).toContainText("audio was lost");
  await expect(page.locator(".global-action-message")).not.toContainText("Recording saved.");
  await recorder.locator(".flow-node-title").click();
  const controls = page.getByLabel("Recorder controls");
  await expect(controls).toContainText("keeps everything up to that point");
  await expect(controls).toContainText("rec-take.wav");
  await expect(recorder.getByRole("button", { name: "Record Podcast recorder" })).toBeVisible();
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`first Record asks for a recording folder and one click approves it in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, {
          __routeFixtureSession: session,
          __routeFixtureRunning: true,
          __routeFixtureNoRecordingRoot: true,
        });
      },
      { session, theme },
    );
    await page.goto("/route-harness.html");
    const recorder = page.getByTestId("rf__node-rec");
    await expect(recorder).toBeVisible();
    // Pressing Record first explains exactly where to choose the folder.
    await recorder.getByRole("button", { name: "Record Podcast recorder" }).click();
    await expect(page.locator(".global-action-message")).toContainText("Recording folder");
    await recorder.locator(".flow-node-title").click();
    const folder = page.locator(".inspector").getByRole("group", { name: "Recording folder" });
    await expect(folder).toBeVisible();
    const field = folder.getByLabel("Folder path");
    await expect(field).toHaveValue("C:\\Users\\you\\Music\\AudioRouter Recordings");
    await folder.screenshot({ path: testInfo.outputPath(`recording-folder-missing-${theme}.png`) });
    // A mistyped path is refused with a plain reason and nothing changes.
    await field.fill("Recordings");
    await folder.getByRole("button", { name: "Use this folder" }).click();
    await expect(folder.getByRole("status")).toContainText("full folder path");
    await field.fill("D:\\Podcast");
    await folder.getByRole("button", { name: "Use this folder" }).click();
    await expect(folder.getByRole("status")).toContainText("Recordings will be saved here");
    await expect(folder.locator(".recording-folder-path")).toHaveText("D:\\Podcast");
    await expect(folder.getByRole("button", { name: "Change folder" })).toBeVisible();
    await folder.screenshot({ path: testInfo.outputPath(`recording-folder-set-${theme}.png`) });
    // Record now works and saves into the chosen folder.
    await page.getByLabel("Recorder controls").getByRole("button", { name: "Record Podcast recorder" }).click();
    await expect(page.getByLabel("Recorder controls").getByRole("status").first()).toHaveText("Recording");
    // The Recording tab shows the same folder.
    await page.getByRole("tab", { name: "Recording", exact: true }).click();
    await expect(
      page.getByRole("tabpanel").getByRole("group", { name: "Recording folder" }).locator(".recording-folder-path"),
    ).toHaveText("D:\\Podcast");
  });
}
