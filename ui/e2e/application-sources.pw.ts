import { expect, test } from "./browser-test";
import { openConnectionForm, setColorTheme } from "./workbench";

test("adds a closed app from its executable path as a PID-free source", async ({ page }) => {
  await page.goto("/route-harness.html");
  await page.getByRole("tab", { name: "Tools" }).click();
  await page.getByRole("button", { name: "Choose an application source" }).click();

  await page.getByLabel("Application executable path").fill("C:\\Games\\closed-game.exe");
  await page.getByRole("button", { name: "Add this app source" }).click();

  const source = page.getByTestId("rf__node-application-capture-1");
  await expect(source).toBeVisible();
  await expect(source).toContainText("closed-game.exe");
  await expect(page.locator(".global-action-message")).toContainText(/saved application selector/);
});

test("creates an app group without changing existing graph connections in all themes", async ({ page }, testInfo) => {
  await page.goto("/route-harness.html");
  await page.setViewportSize({ width: 1280, height: 800 });
  await openConnectionForm(page);
  const connectionEditor = page.locator(".workbench-connection-editor");
  await connectionEditor.getByLabel("Source output port").selectOption({ label: "Microphone · out · 1ch" });
  await connectionEditor.getByLabel("Destination input port").selectOption({ label: "Voice gain · in · 1ch" });
  await connectionEditor.getByRole("button", { name: "Add connection", exact: true }).click();
  await connectionEditor.getByLabel("Source output port").selectOption({ label: "Voice gain · out · 1ch" });
  await connectionEditor.getByLabel("Destination input port").selectOption({ label: "Headphones · in · 2ch" });
  await connectionEditor.getByRole("button", { name: "Add connection", exact: true }).click();
  await page.getByRole("tab", { name: "Tools" }).click();
  await page.getByRole("button", { name: "Choose an application source" }).click();
  await page.getByRole("button", { name: "App group" }).click();
  const path = page.getByLabel("Application executable path");
  for (const executable of ["C:\\Games\\game.exe", "C:\\Chat\\voice.exe"]) {
    await path.fill(executable);
    await page.getByRole("button", { name: "Add path to group" }).click();
  }

  const before = await page.locator(".react-flow__edges .react-flow__edge").count();
  await page.getByRole("button", { name: "Create app group" }).click();
  await expect(page.getByTestId("rf__node-application-capture-1")).toBeVisible();
  await expect(page.getByTestId("rf__node-application-capture-2")).toBeVisible();
  await expect(page.getByTestId("rf__node-mixer-1")).toBeVisible();
  await expect(page.locator(".react-flow__edges .react-flow__edge")).toHaveCount(before + 2);
  await expect(page.locator(".global-action-message")).toContainText(/application sources and a Mixer/);
  for (const theme of ["dark", "light", "high-contrast"]) {
    await setColorTheme(page, theme);
    await expect(page.getByTestId("rf__node-mixer-1")).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath(`app-group-${theme}.png`), fullPage: false });
  }
  await expect(page.getByTestId("rf__node-mic")).toBeVisible();
  // Reopening the picker starts a new group instead of re-adding these apps.
  await page.getByRole("button", { name: "Choose an application source" }).click();
  await page.getByRole("button", { name: "App group", exact: true }).click();
  await expect(page.getByRole("button", { name: "Create app group" })).toBeDisabled();
  await expect(page.getByText("C:\\Games\\game.exe")).toHaveCount(0);
});

test("keeps the application picker readable and stable in every theme", async ({ page }, testInfo) => {
  await page.goto("/route-harness.html");
  for (const theme of ["dark", "light", "high-contrast"]) {
    await setColorTheme(page, theme);
    await page.getByRole("tab", { name: "Tools" }).click();
    await page.getByRole("button", { name: "Choose an application source" }).click();
    await page.getByRole("button", { name: "App group", exact: true }).click();
    const status = page.locator(".application-picker-status");
    const before = (await status.boundingBox())?.height;
    await page.getByRole("button", { name: "Browse for .exe" }).click();
    await expect(status).toContainText(/Type the full path/);
    expect((await status.boundingBox())?.height).toBe(before);
    await page.getByLabel("Application executable path").fill("C:\\Apps\\voice.exe");
    await page.getByRole("button", { name: "Add path to group" }).click();
    await page.screenshot({ path: testInfo.outputPath(`application-picker-${theme}.png`), fullPage: false });
    await page.getByRole("button", { name: "Close application picker" }).click();
  }
});
