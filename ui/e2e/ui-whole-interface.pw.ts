import { openPage } from "./workbench";
import { test, expect } from "./browser-test";
import { demoSession } from "../src/fixtures";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`notifications never cover the Properties panel in ${theme}`, async ({ page }) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    for (const width of [1280, 1440, 1000]) {
      await page.setViewportSize({ width, height: 720 });
      await page.goto("/route-harness.html");
      await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
      // The demo route has an unfed node, so its warning is showing; removing a node adds a message.
      await page
        .getByTestId("rf__node-voice")
        .getByRole("button", { name: /^Delete / })
        // Dispatched: at 1000 px this card runs past the canvas edge until panned, and placement is the point here.
        .dispatchEvent("click");
      const message = page.locator(".global-action-message");
      await expect(message).toContainText("Choose Undo at the top");
      const panel = await page.locator(".right-workbench").boundingBox();
      const notifications = await page.locator(".workspace-notifications").boundingBox();
      expect(notifications!.x + notifications!.width).toBeLessThanOrEqual(panel!.x);
      expect(notifications!.x).toBeGreaterThanOrEqual(0);
    }
  });

  test(`secondary clicks do not change canvas slider values in ${theme}`, async ({ page }) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const fader = page.getByTestId("rf__node-voice").getByRole("slider");
    await expect(fader).toHaveAttribute("aria-valuenow", "0");
    await fader.click({ button: "right", position: { x: 2, y: 3 } });
    await expect(fader).toHaveAttribute("aria-valuenow", "0");
    await page.keyboard.press("Escape");
    await fader.click({ button: "middle", position: { x: 2, y: 3 } });
    await expect(fader).toHaveAttribute("aria-valuenow", "0");
    await fader.focus();
    await page.keyboard.press("ArrowRight");
    await expect(fader).not.toHaveAttribute("aria-valuenow", "0");
  });

  test(`connection menus use the theme and restore keyboard focus in ${theme}`, async ({ page }, info) => {
    const session = structuredClone(demoSession);
    session.edges = [
      {
        id: "voice-output",
        sourceNode: "voice",
        sourcePort: "out",
        destinationNode: "headphones",
        destinationPort: "in",
        matrix: [1, 1],
        enabled: true,
      },
    ];
    await page.addInitScript(
      ({ theme, session }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session });
      },
      { theme, session },
    );
    await page.goto("/route-harness.html");
    const trigger = page.getByRole("button", { name: "Add processor to connection voice-output", exact: true });
    await trigger.focus();
    await page.keyboard.press("Enter");
    const menu = page.getByRole("menu", { name: "Configure connection voice-output", exact: true });
    await expect(menu.getByRole("menuitem").first()).toBeFocused();
    await page.keyboard.press("End");
    await expect(menu.getByRole("menuitem").last()).toBeFocused();
    await page.keyboard.press("Home");
    await expect(menu.getByRole("menuitem").first()).toBeFocused();
    expect(
      await menu
        .getByRole("menuitem")
        .first()
        .evaluate((element) => {
          const bounds = element.getBoundingClientRect();
          const hit = document.elementFromPoint(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
          return hit === element || element.contains(hit);
        }),
    ).toBe(true);
    await page.screenshot({ path: info.outputPath(`${theme}-connection-menu.png`) });
    const background = await menu.evaluate((element) => ({
      actual: getComputedStyle(element).backgroundColor,
      expected: getComputedStyle(element).getPropertyValue("--ambient-surface").trim(),
    }));
    // Resolve the theme token through the browser, rather than comparing hex
    // strings with computed rgb() serialization.
    const expected = await menu.evaluate((element, color) => {
      const sample = document.createElement("span");
      sample.style.color = color;
      element.append(sample);
      const result = getComputedStyle(sample).color;
      sample.remove();
      return result;
    }, background.expected);
    expect(background.actual).toBe(expected);
    await page.keyboard.press("Escape");
    await expect(menu).toBeHidden();
    await expect(trigger).toBeFocused();
  });

  test(`motion follows signal and reduced-motion preferences in ${theme}`, async ({ page }, info) => {
    const session = structuredClone(demoSession);
    session.edges = [
      {
        id: "voice-output",
        sourceNode: "voice",
        sourcePort: "out",
        destinationNode: "headphones",
        destinationPort: "in",
        matrix: [1, 1],
        enabled: true,
      },
    ];
    await page.addInitScript(
      ({ theme, session }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, {
          __routeFixtureSession: session,
          __routeFixtureRunning: true,
          __routeFixturePrivacyMuted: false,
        });
      },
      { theme, session },
    );
    await page.goto("/route-harness.html");
    await expect(page.locator(".session-flow-canvas .flow-edge-active").first()).toBeVisible();
    await expect(page.locator(".session-flow-canvas .flow-comet").first()).toBeAttached();
    await page.screenshot({ path: info.outputPath(`${theme}-active-flow.png`) });
    await page.emulateMedia({ reducedMotion: "reduce" });
    await expect(page.locator(".session-flow-canvas .flow-comet")).toHaveCount(0);
    await expect(page.locator(".session-flow-canvas .flow-edge-active").first()).toBeVisible();
    await page.screenshot({ path: info.outputPath(`${theme}-reduced-motion-flow.png`) });
    await page.locator(".topbar").getByRole("button", { name: "Stop", exact: true }).click();
    await expect(page.locator(".session-flow-canvas .flow-edge-active")).toHaveCount(0);
    await expect(page.locator(".session-flow-canvas .flow-comet")).toHaveCount(0);
    await page.screenshot({ path: info.outputPath(`${theme}-stopped-flow.png`) });
  });

  test(`cancelling a tool drag preserves the graph in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const source = page.locator(".tool-card").filter({ has: page.getByText("Gain", { exact: true }) });
    await source.scrollIntoViewIfNeeded();
    const sourceBox = (await source.boundingBox())!;
    const canvasBox = (await page.locator(".session-flow-canvas").boundingBox())!;
    const count = await page.locator(".react-flow__node").count();
    await page.mouse.move(sourceBox.x + sourceBox.width / 2, sourceBox.y + sourceBox.height / 2);
    await page.mouse.down();
    await page.mouse.move(sourceBox.x - 40, sourceBox.y + sourceBox.height / 2, { steps: 4 });
    await page.mouse.move(canvasBox.x + canvasBox.width / 2, canvasBox.y + canvasBox.height / 2, { steps: 12 });
    await page.mouse.move(canvasBox.x + canvasBox.width / 2 + 1, canvasBox.y + canvasBox.height / 2);
    await expect(page.locator(".flow-drop-preview")).toBeVisible();
    await page.keyboard.press("Escape");
    await page.mouse.up();
    await expect(page.locator(".flow-drop-preview")).toHaveCount(0);
    await expect(page.locator(".library-drag-chip")).toHaveCount(0);
    await expect(page.locator(".react-flow__node")).toHaveCount(count);
    await page.screenshot({ path: info.outputPath(`${theme}-cancelled-drag.png`) });
  });

  test(`failed reconnect retains the draft and offers safe recovery in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await page
      .locator(".tool-card")
      .filter({ has: page.getByText("Gain", { exact: true }) })
      .click();
    await expect(page.getByTestId("rf__node-gain-1")).toBeVisible();
    const save = page.locator(".topbar").getByRole("button", { name: "Save", exact: true });
    await expect(save).toBeEnabled();
    // A healthy connection has nothing to reconnect, so the button is not offered.
    await expect(page.getByRole("button", { name: "Reconnect", exact: true })).toHaveCount(0);
    const statusBox = await page.locator(".audio-run-state").boundingBox();
    await page.evaluate(() =>
      Object.assign(window, {
        __routeFixtureSnapshotError: "Fixture connection interrupted",
        __routeFixtureForceResync: true,
      }),
    );
    await page.getByRole("button", { name: "Reconnect", exact: true }).click();
    await expect(page.locator(".global-action-message")).toContainText("Reconnect failed");
    // Its arrival must not move the status beside it (UI-17).
    expect(await page.locator(".audio-run-state").boundingBox()).toEqual(statusBox);
    await expect(page.getByTestId("rf__node-gain-1")).toBeVisible();
    await expect(page.locator(".audio-run-state")).toHaveText("Status unavailable");
    await expect(save).toBeDisabled();
    await page.screenshot({ path: info.outputPath(`${theme}-retained-draft-offline.png`) });
    await page.evaluate(() =>
      Object.assign(window, { __routeFixtureSnapshotError: undefined, __routeFixtureDelayMs: 600 }),
    );
    const reconnect = page.getByRole("button", { name: "Reconnect", exact: true });
    const canvas = page.locator(".session-flow-canvas");
    const bounds = await canvas.boundingBox();
    await reconnect.click();
    await expect(reconnect).toHaveAttribute("aria-busy", "true");
    await expect(reconnect).toBeDisabled();
    expect(await canvas.boundingBox()).toEqual(bounds);
    await expect(page.locator(".global-action-message")).toContainText("Reconnected.");
    await expect(page.getByTestId("rf__node-gain-1")).toBeVisible();
    await expect(save).toBeEnabled();
    await expect(reconnect).toHaveCount(0);
  });

  test(`canvas metadata and delete targets are readable in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const audit = await page.locator(".flow-node-content").evaluateAll((cards) =>
      cards.flatMap((card) =>
        [...card.querySelectorAll(".node-kind, .node-kind-family, .node-state")].map((element) => ({
          text: element.textContent,
          size: parseFloat(getComputedStyle(element).fontSize),
        })),
      ),
    );
    await info.attach("metadata-sizes", { body: JSON.stringify(audit, null, 2), contentType: "application/json" });
    expect(audit.filter((entry) => entry.size < 11)).toEqual([]);
    const smallTargets = await page.locator(".flow-node-delete").evaluateAll((buttons) =>
      buttons.flatMap((button) => {
        const style = getComputedStyle(button);
        return parseFloat(style.minWidth) < 28 || parseFloat(style.minHeight) < 28
          ? [button.getAttribute("aria-label")]
          : [];
      }),
    );
    expect(smallTargets).toEqual([]);
    await page.screenshot({ path: info.outputPath(`${theme}-readable-canvas.png`) });
  });

  for (const viewport of [
    { width: 1280, height: 720 },
    { width: 640, height: 360 },
  ]) {
    test(`whole workbench ${theme} ${viewport.width}`, async ({ page }, info) => {
      await page.setViewportSize(viewport);
      await page.emulateMedia({ reducedMotion: "reduce" });
      const session = structuredClone(demoSession);
      session.name = "Long saved session name — microphone, game, music and headphones";
      session.nodes.find((node) => node.id === "voice")!.name =
        "Voice gain with a long descriptive name for the streaming microphone";
      await page.addInitScript(
        ({ theme, session }) => {
          localStorage.setItem("audiorouter.ui.theme", theme);
          Object.assign(window, { __routeFixtureSession: session });
        },
        { theme, session },
      );
      await page.goto("/route-harness.html");
      await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
      await page.getByRole("button", { name: "Dismiss route warning", exact: true }).click();
      expect((await page.locator(".session-flow-canvas > .react-flow").boundingBox())!.height).toBeGreaterThan(100);
      await page.getByRole("button", { name: "Fit View", exact: true }).click();
      await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
      const findings: unknown[] = [];
      for (const tab of [
        "Tools",
        "Properties",
        "Session",
        "Status",
        "Apps",
        "Virtual Cables",
        "Appearance",
        "Recording",
        "AI assistants (MCP)",
        "API",
        "Logs and support",
        "Timing",
      ]) {
        await openPage(page, tab);
        const panel = page.getByRole("tabpanel").filter({ visible: true });
        await expect(panel).toBeVisible();
        // Stacked layouts use document scrolling. Capture the actual panel
        // content, rather than counting an off-screen panel as visual evidence.
        await panel.evaluate((element) => element.scrollIntoView({ block: "start" }));
        expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(viewport.width);
        findings.push({
          tab,
          controls: await panel.locator("input, select, textarea, button").evaluateAll((controls) =>
            controls.map((control) => {
              const style = getComputedStyle(control);
              return {
                text: control.getAttribute("aria-label") ?? control.textContent?.trim().slice(0, 80),
                tag: control.tagName,
                fontSize: style.fontSize,
                disabled: control.matches(":disabled"),
                width: control.getBoundingClientRect().width,
              };
            }),
          ),
        });
        await page.screenshot({ path: info.outputPath(`${theme}-${viewport.width}-${tab.replaceAll(" ", "-")}.png`) });
      }
      await info.attach("workbench-control-inventory", {
        body: JSON.stringify(findings, null, 2),
        contentType: "application/json",
      });
    });
  }

  test(`native dropdowns keep keyboard selection and focus in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-mic").locator(".flow-node-title").click();
    const channels = page.getByRole("combobox", { name: "Input channels", exact: true });
    await channels.focus();
    await page.keyboard.press("End");
    await expect(channels).toHaveValue("right");
    await expect(channels).toBeFocused();
    await page.keyboard.press("Home");
    await expect(channels).toHaveValue("stereo");
    await page.keyboard.press("Alt+ArrowDown");
    await page.keyboard.press("Escape");
    await expect(channels).toBeFocused();
    await expect(channels).toHaveCSS("border-radius", "9px");
    await expect(channels).toHaveCSS("color-scheme", theme === "light" ? "light" : "dark");
    await page.screenshot({ path: info.outputPath(`${theme}-dropdown-focus.png`) });
  });
}
