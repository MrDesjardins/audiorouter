import { test, expect, type Page } from "./browser-test";
import { fitCanvas } from "./workbench";
import type { PluginScanEntry } from "@audiorouter/contracts";
import { demoSession } from "../src/fixtures";
import { appendPluginPlaceholderNode } from "../src/draft";

const path = "C:\\Program Files\\Common Files\\VST3\\Surge XT.vst3";
const entry: PluginScanEntry = {
  path,
  identity: {
    path,
    binaryPath: `${path}\\Contents\\x86_64-win\\Surge XT.vst3`,
    format: "vst3",
    architecture: "x64",
    fileBytes: 1024,
    sha256: "b".repeat(64),
    vendor: "Surge Synth Team",
    version: "1.4",
    classIds: [],
    compatibility: "supportedVst3X64",
    instrument: true,
  },
  error: null,
  errorCode: null,
};

const session = appendPluginPlaceholderNode(demoSession, entry);

async function openInstrument(page: Page, theme: string, running: boolean, width = 1500) {
  await page.addInitScript(
    ({ session, theme, running }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __routeFixtureSession: session, __routeFixtureRunning: running });
    },
    { session, theme, running },
  );
  await page.setViewportSize({ width, height: 950 });
  await page.goto("/route-harness.html");
  await fitCanvas(page);
  await page.getByTestId("rf__node-instrument-1").locator(".flow-node-title").click();
  await page.getByRole("tab", { name: "Properties", exact: true }).click();
  return page.getByLabel("Instrument keyboard and MIDI");
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`the Instrument keyboard plays, lists devices and does not move in ${theme}`, async ({ page }) => {
    const editor = await openInstrument(page, theme, true);
    await expect(editor.getByRole("button", { name: "C4" })).toBeEnabled();
    await expect(editor.getByRole("option", { name: "Fixture Keystation" })).toBeAttached();
    const before = await editor.boundingBox();
    const key = editor.getByRole("button", { name: "A4", exact: true });
    const box = (await key.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height - 6);
    await page.mouse.down();
    await page.mouse.up();
    await expect
      .poll(() => page.evaluate(() => (window as unknown as { __routeFixtureMidi?: unknown[] }).__routeFixtureMidi))
      .toEqual([
        { type: "noteOn", note: 69, velocity: 100 },
        { type: "noteOff", note: 69 },
      ]);
    await editor.getByRole("button", { name: "Stop all notes" }).click();
    // Pressing keys and the panic button never change the editor's size (UI-17).
    expect(await editor.boundingBox()).toEqual(before);
    await editor.screenshot({ path: `../target/ui-quality-review/instrument/instrument-${theme}.png` });
  });
}

for (const theme of ["dark", "light", "high-contrast"]) {
  for (const width of [1500, 640]) {
    test(`the stopped Instrument keeps its playing size in ${theme} at ${width}px`, async ({ page }) => {
      const stopped = await openInstrument(page, theme, false, width);
      await expect(stopped.getByRole("button", { name: "C4" })).toBeDisabled();
      await expect(stopped.getByText("A MIDI keyboard needs no clicking.", { exact: false })).toBeHidden();
      const height = (await stopped.boundingBox())!.height;
      const playing = await openInstrument(await page.context().newPage(), theme, true, width);
      expect((await playing.boundingBox())!.height).toBe(height);
    });
  }
}
