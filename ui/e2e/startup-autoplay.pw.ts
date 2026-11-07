import { backToRoute, fitCanvas, openSettings } from "./workbench";
import { test, expect } from "./browser-test";

// Advanced → "When AudioRouter starts": sign-in startup and autoplay together,
// in the app field style in every theme. The shell bridge is faked so the
// autoplay checkbox shows its saved value and saves through the shell.
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`startup and autoplay settings in ${theme}`, async ({ page }, testInfo) => {
    const invalidGeometry: string[] = [];
    page.on("console", (message) => {
      if (message.type() === "error" && /NaN|Infinity/.test(message.text())) invalidGeometry.push(message.text());
    });
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      const calls: unknown[] = [];
      let autoplay = false;
      let apiAutostart = false;
      Object.assign(window, {
        __autoplayCalls: calls,
        __TAURI_INTERNALS__: {
          invoke: async (command: string, args?: { enabled?: boolean }) => {
            if (command === "autoplay_get") return autoplay;
            if (command === "autoplay_set") {
              calls.push(args);
              if ((window as unknown as { __autoplayReject?: boolean }).__autoplayReject)
                throw new Error("save refused");
              autoplay = args?.enabled === true;
              return autoplay;
            }
            if (command === "api_autostart_get") return apiAutostart;
            if (command === "api_autostart_set") {
              calls.push({ api: args?.enabled });
              apiAutostart = args?.enabled === true;
              return apiAutostart;
            }
            throw new Error(`not available in this test: ${command}`);
          },
        },
      });
    }, theme);
    await page.goto("/route-harness.html");
    await openSettings(page, "When AudioRouter starts");
    const group = page.locator(".startup-group");
    await expect(group.getByRole("heading", { name: "Start at sign-in" })).toBeVisible();
    const autoplay = group.getByLabel("Play the selected session automatically");
    await expect(autoplay).toBeEnabled();
    await expect(autoplay).not.toBeChecked();
    await expect(group.getByTestId("autoplay-state")).toHaveText("Off");
    const height = await group.evaluate((element) => element.getBoundingClientRect().height);
    await autoplay.focus();
    await autoplay.press("Space");
    await expect(group.getByTestId("autoplay-state")).toHaveText("On");
    await expect(autoplay).toHaveAccessibleDescription("On");
    await expect(group.getByText("AudioRouter will play the selected session when it starts.")).toBeVisible();
    const api = group.getByLabel("Start the local API automatically");
    await expect(api).toBeEnabled();
    await api.check();
    await expect(group.getByText("The API will start with AudioRouter.")).toBeVisible();
    expect(await group.evaluate((element) => element.getBoundingClientRect().height)).toBe(height);
    expect(await page.evaluate(() => (window as unknown as { __autoplayCalls: unknown[] }).__autoplayCalls)).toEqual([
      { enabled: true },
      { api: true },
    ]);
    await page.evaluate(() => Object.assign(window, { __autoplayReject: true }));
    await autoplay.press("Space");
    await expect(group.getByText("Autoplay could not be saved. Try again.")).toBeVisible();
    await expect(autoplay).toBeChecked();
    await expect(group.getByTestId("autoplay-state")).toHaveText("On");
    expect(await group.evaluate((element) => element.getBoundingClientRect().height)).toBe(height);
    await group.screenshot({ path: `../target/ui-quality-review/round-25/toggle-${theme}.png` });
    await page.setViewportSize({ width: 640, height: 690 });
    const narrowHeight = await group.evaluate((element) => element.getBoundingClientRect().height);
    await page.evaluate(() => Object.assign(window, { __autoplayReject: false }));
    await autoplay.press("Space");
    await expect(group.getByTestId("autoplay-state")).toHaveText("Off");
    await expect(group.getByText("AudioRouter will start stopped.")).toBeVisible();
    expect(await group.evaluate((element) => element.getBoundingClientRect().height)).toBe(narrowHeight);
    await group.screenshot({ path: `../target/ui-quality-review/round-25/toggle-narrow-${theme}.png` });
    await group.scrollIntoViewIfNeeded();
    await group.screenshot({ path: testInfo.outputPath(`startup-autoplay-${theme}.png`) });
    await backToRoute(page);
    await fitCanvas(page);
    const transform = (await page.locator(".react-flow__viewport").getAttribute("style")) ?? "";
    await testInfo.attach("canvas-transform", { body: transform, contentType: "text/plain" });
    expect(transform).not.toMatch(/NaN|Infinity/);
    expect(invalidGeometry).toEqual([]);
  });
}
