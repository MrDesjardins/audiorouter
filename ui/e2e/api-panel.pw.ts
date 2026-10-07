import { test, expect } from "@playwright/test";
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`API listener controls and address in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(() => {
      let running = false;
      let network: string | null = null;
      Object.assign(window, {
        __TAURI_INTERNALS__: {
          invoke: async (command: string, args?: { action?: string; port?: number; network?: string | null }) => {
            if (command === "http_api_addresses") return [{ address: "192.168.1.20", adapter: "Wi-Fi" }];
            if (command !== "http_api_control") return [];
            if (args?.action === "start") {
              running = true;
              network = args.network ?? null;
            }
            if (args?.action === "stop") {
              running = false;
              network = null;
            }
            const port = args?.port ?? 17891;
            return {
              running,
              port,
              url: running ? `http://127.0.0.1:${port}` : null,
              network,
              networkUrl: running && network ? `http://${network}:${port}` : null,
              token: null,
            };
          },
        },
      });
    });
    await page.goto("/route-harness.html");
    await page.getByLabel("Color theme").selectOption(theme);
    await page.getByRole("tab", { name: "API", exact: true }).click();
    await page.getByLabel("API port").fill("17894");
    await page.getByRole("button", { name: "Start API" }).click();
    await expect(page.getByLabel("API URL")).toHaveValue("http://127.0.0.1:17894");
    await expect(page.getByLabel("API network URL")).toHaveCount(0);
    await expect(page.getByLabel("API port")).toBeDisabled();
    await expect(page.getByRole("button", { name: "Open Swagger documentation" })).toBeEnabled();
    await expect(page.getByRole("button", { name: "Reveal API token" })).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath(`2026-09-29-api-running-${theme}.png`) });
    await page.getByRole("button", { name: "Stop API" }).click();
    await expect(page.getByLabel("API URL")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Start API" })).toBeVisible();
    await page.getByLabel("Who can connect").selectOption("192.168.1.20");
    await expect(page.getByText(/Traffic is not encrypted/)).toBeVisible();
    await page.getByRole("button", { name: "Start API" }).click();
    await expect(page.getByLabel("API network URL")).toHaveValue("http://192.168.1.20:17894");
    await expect(page.getByText("API running · this PC and local network (192.168.1.20)")).toBeVisible();
    await expect(page.getByLabel("Who can connect")).toBeDisabled();
    await page.screenshot({ path: testInfo.outputPath(`2026-10-04-api-network-${theme}.png`) });
  });
}
