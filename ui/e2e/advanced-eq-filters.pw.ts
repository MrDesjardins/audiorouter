import { test, expect } from "./real-backend";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`Advanced EQ Band pass and All pass in ${theme}`, async ({ page }) => {
    await page.goto("/backend-harness.html");
    await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
    await page.getByLabel("Color theme").selectOption(theme);
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    await page
      .locator(".tool-card")
      .filter({ has: page.getByText("Advanced EQ", { exact: true }) })
      .click();
    await page.getByTestId("rf__node-parametricEq-1").locator(".flow-node-title").click();
    const editor = page.getByRole("region", { name: "Advanced EQ editor" });
    const graph = editor.locator(".advanced-eq-graph");
    await expect(graph.getByText("+12")).toBeVisible();
    await expect(graph.getByText("-12")).toBeVisible();
    await editor.getByRole("button", { name: "Add point", exact: true }).click();
    await expect(graph.locator(".advanced-eq-leader")).toHaveCount(1);
    await expect(graph.locator(".advanced-eq-anchor")).toHaveCount(1);
    for (const type of ["bandPass", "allPass"]) {
      await editor.getByLabel("EQ filter type").selectOption(type);
      await expect(editor.getByLabel("EQ gain dB")).toBeDisabled();
      await expect(editor.getByLabel("EQ Q width")).toBeEnabled();
      await editor.getByLabel("EQ frequency Hz").fill("1000");
      await expect(graph.locator(".advanced-eq-curve")).toBeAttached();
      await page.locator(".topbar").getByRole("button", { name: "Save", exact: true }).click();
      await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
      await page.reload();
      await page.getByTestId("rf__node-parametricEq-1").locator(".flow-node-title").click();
      await expect(editor.getByLabel("EQ filter type")).toHaveValue(type);
      await expect(editor.locator(".advanced-eq-status")).not.toContainText("Calculating");
      await expect(editor.locator(".advanced-eq-status")).not.toContainText("unavailable");
      // A flat All pass polyline has a zero-height SVG bounding box.
      await expect(graph.locator(".advanced-eq-curve")).toBeAttached();
      await editor.screenshot({ path: `${process.env.TEMP}/audiorouter-designer-review/eq-${theme}-${type}.png` });
    }
  });
}
