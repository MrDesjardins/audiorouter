import { test, expect } from "@playwright/test";
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`inspector status remains clear in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(theme => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").click();
    await expect(page.getByLabel("Node status: Ready")).toBeVisible();
    await page.getByLabel("Bypass", { exact: true }).check();
    await expect(page.getByLabel("Node status: Bypass")).toBeVisible();
    await expect(page.getByText("This node is bypassed.", { exact: false })).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath(`inspector-${theme}.png`) });
  });
}
