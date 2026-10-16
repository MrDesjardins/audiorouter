import { test, expect } from "@playwright/test";
import { DEFAULT_CUSTOM_THEME } from "../src/customTheme";
import { openAppSettings } from "./workbench";
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`focused settings and guided assistant setup in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await expect(page.locator(".topbar").getByLabel("Color theme")).toHaveCount(0);
    await expect(page.getByRole("tab", { name: "Setup", exact: true })).toHaveCount(0);
    await page.getByRole("tab", { name: "Status", exact: true }).click();
    await expect(page.locator(".setup-status-list > li")).toHaveCount(6);
    await expect(page.getByRole("heading", { name: "Audio devices on this PC" })).toBeVisible();
    await page.screenshot({ path: info.outputPath(theme + "-status.png") });
    await page.getByRole("tab", { name: "Virtual Cables", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Use with other apps" })).toBeVisible();
    await expect(page.locator("#setup-status-heading")).toHaveCount(0);
    await page.screenshot({ path: info.outputPath(theme + "-cables.png") });
    await page.getByRole("tab", { name: "UI Options", exact: true }).click();
    await expect(page.getByLabel("Color theme")).toBeVisible();
    await expect(page.getByLabel("Import theme file")).toHaveCSS("border-radius", "9px");
    await page.screenshot({ path: info.outputPath(theme + "-options.png") });
    await openAppSettings(page);
    await expect(page.getByRole("region", { name: "New versions" })).toBeVisible();
    await expect(page.getByRole("group", { name: "Audio device access" })).toBeVisible();
    await page.getByRole("tab", { name: "Recording", exact: true }).click();
    await expect(page.locator(".recording-library .recording-empty")).toContainText("No completed recordings yet");
    await expect(page.getByText("No completed recording files yet.", { exact: true })).toHaveCount(0);
    await page.screenshot({ path: info.outputPath(theme + "-recordings.png") });
    await page.getByRole("tab", { name: "MCP", exact: true }).click();
    await page.getByText("Connect a local assistant", { exact: true }).click();
    await expect(page.getByRole("heading", { name: "Choose your assistant" })).toBeVisible();
    await page.getByLabel("Local assistant").selectOption("claude");
    await expect(page.getByRole("heading", { name: "Run this in PowerShell" })).toBeVisible();
    await page.locator(".local-assistant-setup").screenshot({ path: info.outputPath(theme + "-local-assistant.png") });
    await page.getByRole("button", { name: "Set client permissions", exact: true }).click();
    await expect(page.locator("[data-client-permissions]")).toHaveAttribute("open", "");
  });
}
test("custom theme file import, export and persistence", async ({ page }) => {
  await page.goto("/route-harness.html");
  await page.getByRole("tab", { name: "UI Options", exact: true }).click();
  const imported = {
    ...DEFAULT_CUSTOM_THEME,
    name: "Studio mint",
    colors: { ...DEFAULT_CUSTOM_THEME.colors, accent: "#23c8a0" },
  };
  await page.getByLabel("Import theme file").setInputFiles({
    name: "studio.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(imported)),
  });
  await expect(page.getByLabel("Theme name")).toHaveValue("Studio mint");
  await expect(page.locator(".app-shell")).toHaveCSS("--cyan", "#23c8a0");
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export custom theme" }).click();
  const file = await download;
  const stream = await file.createReadStream();
  const chunks = [];
  for await (const chunk of stream!) chunks.push(chunk);
  expect(JSON.parse(Buffer.concat(chunks).toString())).toEqual(imported);
  await page.getByLabel("Import theme file").setInputFiles({
    name: "bad.json",
    mimeType: "application/json",
    buffer: Buffer.from(
      JSON.stringify({ ...imported, colors: { ...imported.colors, text: "url(https://example.com)" } }),
    ),
  });
  await expect(page.getByRole("alert")).toContainText("six-digit hex");
  await expect(page.getByLabel("Theme name")).toHaveValue("Studio mint");
  await page.reload();
  await page.getByRole("tab", { name: "UI Options", exact: true }).click();
  await expect(page.getByLabel("Color theme")).toHaveValue("custom");
  await expect(page.getByLabel("Theme name")).toHaveValue("Studio mint");
  await page.getByRole("button", { name: "Create your own theme" }).click();
  await expect(page.getByLabel("Theme name")).toHaveValue("Studio mint");
});
test("a selected tool has a direct, allowlisted visual guide", async ({ page }) => {
  await page.goto("/route-harness.html");
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await expect(page.locator(".inspector").getByRole("link", { name: /Visual guide/ })).toHaveAttribute(
    "href",
    "https://audiorouter.org/tools/gain.html",
  );
});
