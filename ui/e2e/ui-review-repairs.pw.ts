import { test, expect } from "./browser-test";
import type { Node, Session } from "@audiorouter/contracts";
import { demoSession } from "../src/fixtures";
import { fitCanvas } from "./workbench";

const node = (id: string, kind: Node["kind"]): Node => ({
  id,
  name: id,
  kind,
  typeVersion: 1,
  enabled: true,
  bypass: false,
  ports: [{ name: "out", direction: "output", channels: 2 }],
  parameters: {},
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`instrument keyboard releases the original tool on selection and keyboard blur in ${theme}`, async ({
    page,
  }) => {
    const session: Session = {
      ...demoSession,
      nodes: [node("Synth A", "instrument"), node("Synth B", "instrument")],
      edges: [],
    };
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session, __routeFixtureRunning: true });
      },
      { session, theme },
    );
    await page.goto("/route-harness.html");
    await fitCanvas(page);
    await page.getByTestId("rf__node-Synth A").locator(".flow-node-title").click();
    const key = page.getByRole("button", { name: "C4", exact: true });
    await key.focus();
    await page.keyboard.down("Enter");
    // A selection event while a key is held, without an intervening focus loss.
    await page.getByTestId("rf__node-Synth B").locator(".flow-node-title").dispatchEvent("click");
    await page.keyboard.up("Enter");
    await expect
      .poll(() =>
        page.evaluate(() => (window as unknown as { __routeFixtureMidiCalls: unknown[] }).__routeFixtureMidiCalls),
      )
      .toEqual([
        { sessionId: session.id, nodeId: "Synth A", events: [{ type: "noteOn", note: 60, velocity: 100 }] },
        { sessionId: session.id, nodeId: "Synth A", events: [{ type: "allNotesOff" }] },
      ]);
    await key.focus();
    await page.keyboard.down("Space");
    await page.keyboard.press("Tab");
    await page.keyboard.up("Space");
    await expect
      .poll(() =>
        page.evaluate(() =>
          (window as unknown as { __routeFixtureMidiCalls: unknown[] }).__routeFixtureMidiCalls.slice(-2),
        ),
      )
      .toEqual([
        { sessionId: session.id, nodeId: "Synth B", events: [{ type: "noteOn", note: 60, velocity: 100 }] },
        { sessionId: session.id, nodeId: "Synth B", events: [{ type: "noteOff", note: 60 }] },
      ]);
    await page
      .locator(".right-workbench")
      .screenshot({ path: `../target/ui-code-review-2026-11-22/instrument-${theme}.png` });
  });

  test(`external Audio File playback updates the real card Stop control in ${theme}`, async ({ page }) => {
    const file = { ...node("File", "audioFile"), parameters: { mediaId: "fixture-media", fileName: "fixture.wav" } };
    const session: Session = { ...demoSession, nodes: [file], edges: [] };
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, {
          __routeFixtureSession: session,
          __routeFixtureRunning: true,
          __routeFixtureSourceStates: {},
        });
      },
      { session, theme },
    );
    await page.goto("/route-harness.html");
    const card = page.getByTestId("rf__node-File");
    await expect(card.getByRole("button", { name: "Stop File", exact: true })).toBeDisabled();
    await page.evaluate(() => {
      (window as unknown as { __routeFixtureSourceStates: Record<string, string> }).__routeFixtureSourceStates.File =
        "playing";
    });
    await expect(card.getByRole("button", { name: "Stop File", exact: true })).toBeEnabled();
    await card.getByRole("button", { name: "Stop File", exact: true }).click();
    await expect(card.getByRole("button", { name: "Stop File", exact: true })).toBeDisabled();
    await card.locator(".flow-node-title").click();
    await page
      .locator(".right-workbench")
      .screenshot({ path: `../target/ui-code-review-2026-11-22/audio-file-${theme}.png` });
  });

  test(`plugin programs reset when selecting another plugin in ${theme}`, async ({ page }) => {
    const plugin = (id: string): Node => ({
      ...node(id, "plugin"),
      parameters: { format: "vst2", path: `C:\\Plugins\\${id}.dll`, fingerprint: "a".repeat(64) },
      ports: [
        { name: "in", direction: "input", channels: 2 },
        { name: "out", direction: "output", channels: 2 },
      ],
    });
    const session: Session = { ...demoSession, nodes: [plugin("Plugin A"), plugin("Plugin B")], edges: [] };
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session });
      },
      { session, theme },
    );
    await page.goto("/route-harness.html");
    await fitCanvas(page);
    await page.getByTestId("rf__node-Plugin A").locator(".flow-node-title").click();
    await page.getByRole("button", { name: "Load programs", exact: true }).click();
    await expect(page.locator(".plugin-programs select")).toHaveCount(1);
    await page.getByTestId("rf__node-Plugin B").locator(".flow-node-title").click();
    await expect(page.locator(".plugin-programs select")).toHaveCount(0);
    await page.getByRole("button", { name: "Load programs", exact: true }).scrollIntoViewIfNeeded();
    await page
      .locator(".right-workbench")
      .screenshot({ path: `../target/ui-code-review-2026-11-22/plugin-${theme}.png` });
  });
}
