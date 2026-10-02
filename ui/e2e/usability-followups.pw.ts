import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";
import { appendLibraryNode } from "../src/draft";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`logs folder controls in ${theme}`, async ({ page }, info) => {
    await page.addInitScript(({ theme }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __TAURI_INTERNALS__: { invoke: async (command: string) => {
        if (command === "log_folder_path") return "C:/Users/Example/AppData/Local/AudioRouter/logs";
        if (command === "open_logs_folder") { Object.assign(window, { __logsOpened: true }); return null; }
        return null;
      } } });
      Object.defineProperty(navigator, "clipboard", { value: { writeText: async () => undefined } });
    }, { theme });
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/route-harness.html");
    await page.getByRole("tab", { name: "Logs", exact: true }).click();
    const panel = page.getByRole("region", { name: "Send logs for support" });
    await expect(panel.getByLabel("Logs folder", { exact: true })).toHaveValue("C:/Users/Example/AppData/Local/AudioRouter/logs");
    await panel.getByRole("button", { name: "Open logs folder" }).click();
    await expect(panel.getByRole("status")).toContainText("Logs folder opened");
    await panel.getByRole("button", { name: "Copy folder path" }).click();
    await expect(panel.getByRole("status")).toHaveText("Folder path copied.");
    const field = panel.getByLabel("Logs folder", { exact: true });
    await expect(field).toHaveCSS("border-radius", "9px");
    const box = (await panel.boundingBox())!;
    expect(box.x + box.width).toBeLessThanOrEqual(1280);
    await page.screenshot({ path: info.outputPath(`${theme}-logs.png`) });
  });
  test(`network diagrams and canvas controls in ${theme}`, async ({ page }, info) => {
    const session = appendLibraryNode(appendLibraryNode(demoSession, "networkSend"), "networkReceive");
    await page.addInitScript(({ session, theme }) => { Object.assign(window, { __routeFixtureSession: session }); localStorage.setItem("audiorouter.ui.theme", theme); }, { session, theme });
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/route-harness.html");
    await expect(page.getByRole("button", { name: "List view", exact: true })).toHaveCount(0);
    for (const [kind, field] of [["networkSend", "Receiving computer's IP address"], ["networkReceive", "Sending computer's IP address"]]) {
      await page.getByTestId(`rf__node-${kind}-1`).locator(".flow-node-title").click();
      await page.getByLabel(field, { exact: true }).fill("192.168.1.20");
      const diagram = page.getByLabel("Network audio direction");
      await diagram.scrollIntoViewIfNeeded();
      await expect(diagram).toContainText("192.168.1.20");
      await expect(diagram).toContainText("47800");
      await expect(diagram).toContainText("Audio in");
      await expect(diagram).toContainText("Audio out");
      const box = (await diagram.boundingBox())!;
      expect(box.x).toBeGreaterThanOrEqual(0);
      expect(box.x + box.width).toBeLessThanOrEqual(1280);
      await page.screenshot({ path: info.outputPath(`${theme}-${kind}.png`) });
    }
    await page.getByRole("tab", { name: "Advanced", exact: true }).click();
    await page.getByText("Keyboard graph controls", { exact: true }).click();
    await expect(page.getByLabel("Graph nodes and connections")).toBeVisible();
    await expect(page.locator(".session-flow-canvas")).toBeVisible();
  });

}
