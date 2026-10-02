import { test, expect } from "@playwright/test";
import { demoSession } from "../src/fixtures";
import { appendLibraryNode } from "../src/draft";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`saved token visibility and copying in ${theme}`, async ({ page }, info) => {
    await page.addInitScript(({ theme }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      let token = "a".repeat(64);
      Object.assign(window, { __TAURI_INTERNALS__: { invoke: async (command: string, args: { action?: string }) => {
        if (command !== "http_api_control") return null;
        if (args.action === "regenerate") token = "b".repeat(64);
        return { running: false, port: 17891, url: null, token: args.action === "reveal" || args.action === "regenerate" ? token : null };
      } } });
      Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: async (value: string) => Object.assign(window, { __copiedToken: value }) } });
    }, { theme });
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/route-harness.html");
    await page.getByRole("tab", { name: "API", exact: true }).click();
    const token = page.getByLabel("API bearer token", { exact: true });
    await expect(token).toHaveValue("a".repeat(64));
    await page.getByRole("button", { name: "Copy API token", exact: true }).click();
    await expect.poll(() => page.evaluate(() => (window as unknown as { __copiedToken: string }).__copiedToken)).toBe("a".repeat(64));
    await page.getByRole("button", { name: "Generate new token", exact: true }).click();
    await page.getByRole("button", { name: "Replace API token", exact: true }).click();
    await expect(token).toHaveValue("b".repeat(64));
    await token.scrollIntoViewIfNeeded();
    await expect(token).toHaveCSS("border-radius", "9px");
    const box = (await token.boundingBox())!; expect(box.x + box.width).toBeLessThanOrEqual(1280);
    await page.screenshot({ path: info.outputPath(`${theme}-saved-token.png`) });
  });
  test(`node and session identity controls in ${theme}`, async ({ page }, info) => {
    await page.addInitScript(({ theme }) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: async (value: string) => Object.assign(window, { __copiedIdentity: value }) } });
    }, { theme });
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
    const nodeId = page.getByLabel("Node ID", { exact: true });
    await nodeId.scrollIntoViewIfNeeded(); await expect(nodeId).toHaveText("voice");
    await page.getByRole("button", { name: "Copy node ID", exact: true }).focus();
    await page.keyboard.press("Enter");
    await expect.poll(() => page.evaluate(() => (window as unknown as { __copiedIdentity: string }).__copiedIdentity)).toBe("voice");
    await expect(page.locator(".node-identity-message")).toHaveText("Copied.");
    await page.screenshot({ path: info.outputPath(`${theme}-node-id.png`) });
    await page.getByRole("tab", { name: "Session", exact: true }).click();
    const sessionId = page.getByLabel("Session ID", { exact: true });
    await sessionId.scrollIntoViewIfNeeded(); await expect(sessionId).toHaveText("demo-session");
    await page.getByRole("button", { name: "Copy session ID", exact: true }).click();
    await expect.poll(() => page.evaluate(() => (window as unknown as { __copiedIdentity: string }).__copiedIdentity)).toBe("demo-session");
    const box = (await sessionId.boundingBox())!; expect(box.x + box.width).toBeLessThanOrEqual(1280);
    await page.screenshot({ path: info.outputPath(`${theme}-session-id.png`) });
    await page.getByRole("tab", { name: "API", exact: true }).click();
    await page.screenshot({ path: info.outputPath(`${theme}-token-controls.png`) });
  });
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
