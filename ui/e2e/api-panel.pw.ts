import { test, expect } from "@playwright/test";
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`API listener controls and address in ${theme}`, async ({ page }) => {
    await page.addInitScript(() => {
      let running = false;
      Object.assign(window, { __TAURI_INTERNALS__: { invoke: async (command: string, args?: { action?: string; port?: number }) => {
        if (command !== "http_api_control") return [];
        if (args?.action === "start") running = true;
        if (args?.action === "stop") running = false;
        return { running, port: args?.port ?? 17891, url: running ? `http://127.0.0.1:${args?.port ?? 17891}` : null, token: null };
      } } });
    });
    await page.goto("/route-harness.html");
    await page.getByLabel("Color theme").selectOption(theme);
    await page.getByRole("tab", { name: "API", exact: true }).click();
    await page.getByLabel("API port").fill("17894");
    await page.getByRole("button", { name: "Start API" }).click();
    await expect(page.getByLabel("API URL")).toHaveValue("http://127.0.0.1:17894");
    await expect(page.getByLabel("API port")).toBeDisabled();
    await expect(page.getByRole("button", { name: "Open Swagger documentation" })).toBeEnabled();
    await expect(page.getByRole("button", { name: "Reveal API token" })).toBeVisible();
    await page.screenshot({ path: `../docs/plans/active/evidence/2026-09-29-api-running-${theme}.png` });
    await page.getByRole("button", { name: "Stop API" }).click();
    await expect(page.getByLabel("API URL")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Start API" })).toBeVisible();
  });
}
