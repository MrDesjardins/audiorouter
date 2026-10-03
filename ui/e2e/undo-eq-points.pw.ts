import { test, expect } from "./real-backend";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`EQ point selection and toolbar history in ${theme}`, async ({ page }) => {
    await page.goto("/backend-harness.html");
    await expect(page.getByRole("heading", { name: "Offline qualification", exact: true })).toBeVisible();
    await page.getByLabel("Color theme").selectOption(theme);
    const toolbar = page.locator(".topbar");
    const undo = toolbar.getByRole("button", { name: "Undo", exact: true });
    const redo = toolbar.getByRole("button", { name: "Redo", exact: true });
    await expect(undo).toBeDisabled();
    await expect(redo).toBeDisabled();
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    await page.locator(".tool-card").filter({ has: page.getByText("Advanced EQ", { exact: true }) }).click();
    await page.getByTestId("rf__node-parametricEq-1").locator(".flow-node-title").click();
    const editor = page.getByRole("region", { name: "Advanced EQ editor" });
    await editor.getByRole("button", { name: "Add point", exact: true }).click();
    await editor.getByLabel("EQ frequency Hz").fill("250");
    await editor.getByRole("button", { name: "Add point", exact: true }).click();
    await editor.getByLabel("EQ frequency Hz").fill("4000");
    await editor.getByLabel("EQ point").selectOption("0");
    await expect(editor.getByLabel("EQ frequency Hz")).toHaveValue("250");
    await expect(editor.getByRole("option", { name: "Peaking/Band", exact: true })).toBeAttached();
    await editor.getByLabel("EQ point").selectOption("1");
    await expect(editor.getByLabel("EQ frequency Hz")).toHaveValue("4000");
    const filter = editor.getByLabel("EQ filter type");
    await filter.selectOption("notch");
    await filter.focus();
    await filter.press("Control+z");
    await expect(filter).toHaveValue("peaking");
    await page.keyboard.press("Control+y");
    await expect(filter).toHaveValue("notch");
    await page.keyboard.press("Control+z");
    await expect(filter).toHaveValue("peaking");
    // A small hand movement while clicking a point must not edit its value.
    // Each point draws several circles (hit target, dot, marker); aim at point 2 by its label.
    const dot = editor.locator('.advanced-eq-point[aria-label="Point 2"] .advanced-eq-point-hit-target');
    const box = (await dot.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width / 2 + 2, box.y + box.height / 2 + 1);
    await page.mouse.up();
    await expect(editor.getByLabel("EQ frequency Hz")).toHaveValue("4000");
    await toolbar.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
    const gain = editor.getByLabel("EQ gain dB");
    await gain.fill("-6");
    await gain.press("Control+z");
    await expect(gain).toHaveValue("0");
    await page.keyboard.press("Control+y");
    await expect(gain).toHaveValue("-6");
    await toolbar.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
    await undo.click();
    await expect(gain).toHaveValue("0");
    await redo.click();
    await expect(gain).toHaveValue("-6");
    // Restoring an older snapshot must still commit against the new revision.
    await undo.click();
    await toolbar.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
    await redo.click();
    await expect(gain).toHaveValue("-6");
    await expect(toolbar.getByRole("button", { name: "Save", exact: true })).toBeEnabled();
    await editor.getByLabel("EQ Q width").fill("2");
    await expect(redo).toBeDisabled();
    await expect(editor.locator(".advanced-eq-status")).not.toContainText("Calculating");
    await page.screenshot({ path: `${process.env.TEMP}/audiorouter-designer-review/undo-eq-${theme}.png` });
    await page.reload();
    await expect(undo).toBeDisabled();
  });
}
