import { test, expect } from "./browser-test";
import { demoSession } from "../src/fixtures";
import { openAppSettings } from "./workbench";

// Only the in-memory route harness is used; no native transport or profile.
for (const theme of ["dark", "light", "high-contrast"]) {
  for (const viewport of [
    { width: 1280, height: 720 },
    { width: 640, height: 360 },
  ]) {
    test(`combined UX ${theme} ${viewport.width}x${viewport.height}`, async ({ page }, info) => {
      await page.setViewportSize(viewport);
      await page.addInitScript(
        ({ theme, session }) => {
          localStorage.setItem("audiorouter.ui.theme", theme);
          Object.assign(window, {
            __routeFixtureSession: session,
            __routeFixtureSessions: [
              session,
              { ...session, id: "other", name: "Other saved route" },
              { ...session, id: "background", name: "Background route used by another app" },
            ],
            __routeFixtureRunning: true,
            __routeFixtureActiveSessionIds: [session.id, "background"],
            __routeFixtureSetupCheck: {
              findings: [
                {
                  code: "deviceMissing",
                  severity: "problem",
                  sessionId: session.id,
                  sessionName: session.name,
                  nodeId: "voice",
                  nodeName: "Voice gain",
                  message: "Inspect this saved tool before playing.",
                  fix: "Review its Properties.",
                },
              ],
              sessionsChecked: 2,
              devicesChecked: 2,
              truncated: 0,
              notChecked: [],
            },
          });
        },
        { theme, session: demoSession },
      );
      await page.goto("/route-harness.html");
      const tools = page.getByRole("tab", { name: "Tools", exact: true });
      await expect(tools).toBeVisible();
      await expect
        .poll(async () => (await page.getByLabel("Signal-flow graph", { exact: true }).boundingBox())?.height ?? 0)
        .toBeGreaterThan(100);
      const heights = await page
        .getByRole("tab")
        .evaluateAll((tabs) => tabs.map((tab) => Math.round(tab.getBoundingClientRect().height)));
      expect(new Set(heights).size).toBe(1);
      await tools.focus();
      await tools.press("End");
      await expect(page.getByRole("tab", { name: "Logs", exact: true })).toBeFocused();
      await page.keyboard.press("Home");
      await expect(tools).toBeFocused();
      await page.keyboard.press("ArrowRight");
      await expect(page.getByRole("tab", { name: "Properties", exact: true })).toBeFocused();
      const selected = page.getByRole("tab", { selected: true });
      const panelId = await selected.getAttribute("aria-controls");
      await expect(page.locator(`#${panelId}`)).toHaveAttribute(
        "aria-labelledby",
        (await selected.getAttribute("id")) ?? "",
      );
      expect(await page.getByRole("tab").evaluateAll((tabs) => tabs.filter((tab) => tab.tabIndex === 0).length)).toBe(
        1,
      );

      await page.getByRole("tab", { name: "Session", exact: true }).click();
      await expect(page.getByRole("button", { name: "Export saved version…", exact: true })).toBeVisible();
      const choice = page.getByLabel("Choose session");
      await choice.selectOption("other");
      const dialog = page.getByRole("dialog", { name: "Stop audio and switch session?" });
      await expect(dialog).toContainText(demoSession.name);
      await expect(dialog).toContainText("Background route used by another app");
      await expect(dialog.getByRole("button", { name: "Cancel", exact: true })).toBeFocused();
      await page.screenshot({ path: info.outputPath("switch-confirmation.png") });
      const bounds = await dialog.boundingBox();
      expect(bounds!.x).toBeGreaterThanOrEqual(0);
      expect(bounds!.y).toBeGreaterThanOrEqual(0);
      expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(viewport.width);
      expect(bounds!.y + bounds!.height).toBeLessThanOrEqual(viewport.height);
      await page.keyboard.press("Escape");
      await expect(dialog).toBeHidden();
      await expect(choice).toHaveValue(demoSession.id);
      const calls = () =>
        page.evaluate(() => (window as unknown as { __routeFixtureCalls: () => string[] }).__routeFixtureCalls());
      expect(await calls()).toEqual([]);

      await page.getByRole("tab", { name: "Status", exact: true }).click();
      await page.getByRole("button", { name: "Open affected tool", exact: true }).click();
      await expect(page.getByRole("tab", { name: "Properties", exact: true })).toHaveAttribute("aria-selected", "true");
      await expect(page.getByRole("tab", { name: "Properties", exact: true })).toBeFocused();
      await page.keyboard.press("Tab");
      await expect(page.locator("#workbench-properties")).toBeFocused();
      await page.evaluate(() => {
        const exit = document.createElement("button");
        exit.id = "keyboard-exit-probe";
        exit.textContent = "After the workspace";
        document.body.append(exit);
      });
      for (let step = 0; step < 100; step++) {
        if (await page.locator("#keyboard-exit-probe").evaluate((element) => element === document.activeElement)) break;
        await page.keyboard.press("Tab");
      }
      await expect(page.locator("#keyboard-exit-probe")).toBeFocused();
      await page.locator("#keyboard-exit-probe").evaluate((element) => element.remove());
      await page.screenshot({ path: info.outputPath("finding-properties.png") });
      expect(await calls()).toEqual([]);
      await page.getByRole("button", { name: "Dismiss message", exact: true }).click();

      await openAppSettings(page);
      await page.getByText("Choose a setup goal", { exact: true }).click();
      await page.getByLabel("What do you want to do?").selectOption("recording");
      await expect(page.getByText("Recording verification, not yet verified:", { exact: true })).toBeVisible();
      await page.screenshot({ path: info.outputPath("goal-guide.png") });
      await page.getByRole("button", { name: "Open Tools for this goal", exact: true }).click();
      expect(await calls()).toEqual([]);

      await page.getByRole("tab", { name: "Session", exact: true }).click();
      await choice.selectOption("other");
      await dialog.getByRole("button", { name: "Stop and switch", exact: true }).click();
      await expect(choice).toHaveValue("other");
      expect(await calls()).toEqual(["stop", "stop"]);
      await expect(page.locator(".audio-run-state")).toContainText("Audio stopped");
    });
  }
}
