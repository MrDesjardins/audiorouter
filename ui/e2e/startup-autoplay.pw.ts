import { test, expect } from "@playwright/test";

// Advanced → "When AudioRouter starts": sign-in startup and autoplay together,
// in the app field style in every theme. The shell bridge is faked so the
// autoplay checkbox shows its saved value and saves through the shell.
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`startup and autoplay settings in ${theme}`, async ({ page }, testInfo) => {
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
            if (command === "autoplay_set") { calls.push(args); autoplay = args?.enabled === true; return autoplay; }
            if (command === "api_autostart_get") return apiAutostart;
            if (command === "api_autostart_set") { calls.push({ api: args?.enabled }); apiAutostart = args?.enabled === true; return apiAutostart; }
            throw new Error(`not available in this test: ${command}`);
          },
        },
      });
    }, theme);
    await page.goto("/route-harness.html");
    await page.getByRole("tab", { name: "Advanced" }).click();
    const group = page.locator("details.startup-group");
    await group.locator("summary").click();
    await expect(group.getByRole("heading", { name: "Start at sign-in" })).toBeVisible();
    const autoplay = group.getByLabel("Play the selected session automatically");
    await expect(autoplay).toBeEnabled();
    await expect(autoplay).not.toBeChecked();
    await autoplay.check();
    await expect(group.getByText("AudioRouter will play the selected session when it starts.")).toBeVisible();
    const api = group.getByLabel("Start the local API automatically");
    await expect(api).toBeEnabled();
    await api.check();
    await expect(group.getByText("The API will start with AudioRouter.")).toBeVisible();
    expect(await page.evaluate(() => (window as unknown as { __autoplayCalls: unknown[] }).__autoplayCalls)).toEqual([{ enabled: true }, { api: true }]);
    await group.scrollIntoViewIfNeeded();
    await group.screenshot({ path: testInfo.outputPath(`startup-autoplay-${theme}.png`) });
  });
}
