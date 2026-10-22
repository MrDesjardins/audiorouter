import { test, expect } from "./browser-test";
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`inspector status remains clear in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").click();
    await expect(page.getByLabel("Node status: Ready")).toBeVisible();
    await expect(page.getByLabel("Node status: Ready")).toHaveClass(/is-ready/);
    const width = await page.getByLabel("Node status: Ready").evaluate((pill) => pill.getBoundingClientRect().width);
    await page.getByLabel("Bypass", { exact: true }).check();
    await expect(page.getByLabel("Node status: Bypass")).toBeVisible();
    await expect(page.getByLabel("Node status: Bypass")).toHaveClass(/is-bypassed/);
    expect(await page.getByLabel("Node status: Bypass").evaluate((pill) => pill.getBoundingClientRect().width)).toBe(
      width,
    );
    await expect(page.getByText("This node is bypassed.", { exact: false })).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath(`inspector-${theme}.png`) });
  });
}
