import { expect, test } from "./browser-test";
import { demoSession } from "../src/fixtures";
import { fitCanvas, openSettings } from "./workbench";

for (const theme of ["dark", "light", "high-contrast"]) {
  for (const kind of ["mute", "audioFile", "duck"] as const) {
    test(`${theme}: ${kind} states remain explicit without moving the field`, async ({ page }) => {
      const parameters =
        kind === "mute"
          ? { muted: false }
          : kind === "audioFile"
            ? { loop: false }
            : { trigger: "siegeRound", duckAction: false };
      const session = {
        ...demoSession,
        nodes: demoSession.nodes.map((node) => (node.id === "voice" ? { ...node, kind, parameters } : node)),
      };
      await page.addInitScript(
        ({ theme, session }) => {
          localStorage.setItem("audiorouter.ui.theme", theme);
          Object.assign(window, {
            __routeFixtureSession: session,
            __routeFixtureDiscovery: {
              nodeTypes: [{ type: "mute@1", parameters: [{ name: "muted", type: "boolean" }] }],
            },
          });
        },
        { theme, session },
      );
      await page.setViewportSize({ width: 640, height: 720 });
      await page.goto("/route-harness.html");
      await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
      await fitCanvas(page);
      await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
      const label = kind === "mute" ? "Muted" : kind === "audioFile" ? "Loop" : "In a round, playing";
      const input = page.getByRole("checkbox", { name: label, exact: true });
      await input.scrollIntoViewIfNeeded();
      await expect(input).toHaveAccessibleDescription("Off");
      const field = input.locator("xpath=ancestor::label[1]");
      const box = await field.boundingBox();
      const geometry = () =>
        page.evaluate(() => ({
          scroll: window.scrollY,
          topbar: document.querySelector(".topbar")?.getBoundingClientRect().height,
          inspector: document.querySelector(".main-content > .inspector")?.getBoundingClientRect().top,
          scrollTop: document.querySelector(".main-content > .inspector")?.scrollTop,
        }));
      const before = await geometry();
      await input.focus();
      const focused = await geometry();
      await input.press("Space");
      await expect(input).toBeChecked();
      await expect(input).toHaveAccessibleDescription("On");
      expect({ box: await field.boundingBox(), before, focused, after: await geometry() }).toEqual({
        box,
        before,
        focused: before,
        after: before,
      });
      await field.screenshot({ path: `../target/ui-quality-review/round-25/boolean-${kind}-${theme}.png` });
      await field.locator("span").first().click();
      await expect(input).not.toBeChecked();
      await expect(input).toHaveAccessibleDescription("Off");
    });
  }

  test(`${theme}: cable low latency says On or Off and sends one request`, async ({ page }) => {
    const cables = ["a", "b"].map((letter) => ({
      busId: `cable-${letter}`,
      name: `Cable ${letter.toUpperCase()}`,
      renderEndpointId: `{render-${letter}}`,
      captureEndpointId: `{capture-${letter}}`,
      present: true,
    }));
    await page.addInitScript(
      ({ theme, cables }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, {
          __routeFixtureVirtualCableStatus: {
            state: "installed",
            driverVersion: "0.1.4.0",
            protocol: "1.1",
            defaultPeriodFrames: 480,
            cableCount: cables.length,
            cables,
          },
        });
      },
      { theme, cables },
    );
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await openSettings(page, "Virtual Cables");
    const input = page.getByRole("checkbox", { name: "Low latency cable blocks", exact: true });
    await expect(input).toBeEnabled();
    await expect(input).toHaveAccessibleDescription("Off");
    const field = input.locator("xpath=ancestor::label[1]");
    const box = await field.boundingBox();
    await input.press("Space");
    await expect(input).toHaveAccessibleDescription("On");
    await expect(input).toBeChecked();
    expect(await field.boundingBox()).toEqual(box);
    expect(
      await page.evaluate(() => (window as { __routeFixtureCableCalls?: unknown[] }).__routeFixtureCableCalls),
    ).toEqual([{ operation: "setLowLatencyOn" }]);
    await field.screenshot({ path: `../target/ui-quality-review/round-25/boolean-cable-${theme}.png` });
  });
}
