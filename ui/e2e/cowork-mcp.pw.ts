import { setColorTheme } from "./workbench";
import { expect, test } from "@playwright/test";

for (const initial of ["new", "editor", "revoked"] as const) {
  test(`local Cowork export preserves permissions for ${initial} client`, async ({ page }, testInfo) => {
    await page.addInitScript((role) => {
      const state = window as typeof window & {
        __routeFixtureMcpClients: { clientId: string; role: string; revoked: boolean }[];
        __routeFixtureMcpGrants: string[];
        __coworkFail: boolean;
      };
      state.__routeFixtureMcpClients =
        role === "new" ? [] : [{ clientId: "audiorouter-cowork-local", role: "editor", revoked: role === "revoked" }];
      state.__routeFixtureMcpGrants = [];
      state.__coworkFail = false;
      Object.defineProperty(window, "__TAURI_INTERNALS__", {
        value: {
          invoke: async (command: string) => {
            if (command === "mcp_setup_info")
              return {
                cliPath: "C:\\AudioRouter\\audiorouter-cli.exe",
                cliAvailable: true,
                coworkPluginAvailable: true,
                databasePath: "C:\\AudioRouter\\main.sqlite",
                pipeName: "\\\\.\\pipe\\audiorouter-control",
                transport: "stdio",
              };
            if (command === "mcp_export_cowork_plugin") {
              await new Promise((resolve) => setTimeout(resolve, 200));
              if (state.__coworkFail) throw new Error("The MCP backend is unavailable. Reconnect AudioRouter.");
              return "C:\\Users\\Test\\Downloads\\AudioRouter-Cowork.zip";
            }
            return null;
          },
        },
      });
    }, initial);
    await page.goto("/route-harness.html");
    await page.getByRole("tab", { name: "MCP" }).click();
    const card = page.locator(".mcp-remote-info");
    const button = card.getByRole("button", { name: "Export Cowork plugin" });
    const height = await card.evaluate((element) => element.getBoundingClientRect().height);
    await button.click();
    await expect(button).toBeDisabled();
    await expect(card.getByRole("status")).toContainText("Saved to C:");
    await expect(button).toBeEnabled();
    expect(
      await page.evaluate(
        () => (window as typeof window & { __routeFixtureMcpGrants: string[] }).__routeFixtureMcpGrants,
      ),
    ).toEqual(initial === "editor" ? [] : ["audiorouter-cowork-local:observer"]);
    expect(await card.evaluate((element) => element.getBoundingClientRect().height)).toBe(height);
    if (initial === "new") {
      for (const theme of ["dark", "light", "high-contrast"]) {
        await setColorTheme(page, theme);
        await card.screenshot({ path: testInfo.outputPath(`cowork-${theme}.png`) });
        expect(await card.evaluate((element) => element.getBoundingClientRect().height)).toBe(height);
      }
    }
    await page.evaluate(() => {
      (window as typeof window & { __coworkFail: boolean }).__coworkFail = true;
    });
    await button.click();
    await expect(card.getByRole("status")).toContainText("Reconnect AudioRouter");
    await expect(button).toBeEnabled();
    expect(await card.evaluate((element) => element.getBoundingClientRect().height)).toBe(height);
  });
}
