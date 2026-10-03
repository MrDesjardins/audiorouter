import { test, expect } from "@playwright/test";

// The Duck tool card shows a duck glyph (not the Compressor's arrow) and the
// window Quit button only exists in the desktop shell.
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`Duck tool icon in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    const card = page.locator(".tool-card").filter({ has: page.getByText("Duck", { exact: true }) });
    await expect(card.locator(".tool-card-icon svg.tool-card-glyph")).toHaveCount(1);
    await expect(card.locator(".tool-card-icon")).not.toContainText("⤓");
    await card.scrollIntoViewIfNeeded();
    await card.screenshot({ path: testInfo.outputPath(`duck-icon-${theme}.png`) });
    await expect(page.getByRole("button", { name: "Quit AudioRouter" })).toHaveCount(0);
  });
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`window Quit button asks to confirm in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      // Simulated desktop shell: only the quit command is recorded.
      const calls: string[] = [];
      Object.assign(window, { __quitCalls: calls, __TAURI_INTERNALS__: { invoke: async (command: string) => { calls.push(command); } } });
    }, theme);
    await page.goto("/route-harness.html");
    const quit = page.getByRole("button", { name: "Quit AudioRouter" });
    await quit.click();
    const confirm = page.getByRole("button", { name: "Confirm quit AudioRouter" });
    await expect(confirm).toHaveText("Click again to quit");
    await page.locator(".topbar").screenshot({ path: testInfo.outputPath(`quit-armed-${theme}.png`) });
    expect(await page.evaluate(() => (window as any).__quitCalls)).not.toContain("quit_app");
    await confirm.click();
    await expect.poll(() => page.evaluate(() => (window as any).__quitCalls)).toContain("quit_app");
  });
}
