import { expect, test } from "./browser-test";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: group switches expose fixed states and keep their actions`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await page.getByRole("button", { name: "Group", exact: true }).click();
    for (const name of ["Lock group position and size", "Move contents with group"]) {
      const input = page.getByRole("checkbox", { name, exact: true });
      await input.scrollIntoViewIfNeeded();
      const checked = await input.isChecked();
      await expect(input).toHaveAccessibleDescription(checked ? "On" : "Off");
      const field = input.locator("xpath=ancestor::label[1]");
      const before = await field.boundingBox();
      await input.focus();
      await input.press("Space");
      await expect(input).toHaveAccessibleDescription(checked ? "Off" : "On");
      expect(await field.boundingBox()).toEqual(before);
    }
    const lockBox = (await page
      .getByRole("checkbox", { name: "Lock group position and size", exact: true })
      .boundingBox())!;
    const moveBox = (await page
      .getByRole("checkbox", { name: "Move contents with group", exact: true })
      .boundingBox())!;
    expect(lockBox.y).toBe(moveBox.y);
    await page
      .locator(".group-position-options")
      .screenshot({ path: `../target/ui-quality-review/round-25/group-states-${theme}.png` });
  });
  test(`${theme}: dither shows its state and respects recording format exclusions`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await page
      .getByRole("tablist", { name: "Right sidebar" })
      .getByRole("tab", { name: "Recording", exact: true })
      .click();
    const input = page.getByRole("checkbox", { name: "TPDF dither", exact: true });
    await input.scrollIntoViewIfNeeded();
    await expect(input).toHaveAccessibleDescription("On");
    const field = input.locator("xpath=ancestor::label[1]");
    const captionBox = (await field.locator("span").first().boundingBox())!;
    const inputBox = (await input.boundingBox())!;
    expect(captionBox.y + captionBox.height).toBeLessThanOrEqual(inputBox.y);
    const before = await field.boundingBox();
    await input.focus();
    await input.press("Space");
    await expect(input).not.toBeChecked();
    await expect(input).toHaveAccessibleDescription("Off");
    expect(await field.boundingBox()).toEqual(before);
    await field.locator("span").first().click();
    await expect(input).toHaveAccessibleDescription("On");
    await page.getByLabel("Recorder format", { exact: true }).selectOption("wavFloat32");
    await expect(input).toBeDisabled();
    await expect(input).not.toBeChecked();
    await expect(input).toHaveAccessibleDescription("Off");
    await field.screenshot({ path: `../target/ui-quality-review/round-25/dither-states-${theme}.png` });
    await page.getByLabel("Recorder format", { exact: true }).selectOption("mp3");
    await expect(input).toBeDisabled();
    await expect(input).toHaveAccessibleDescription("Off");
    await page.getByLabel("Recorder format", { exact: true }).selectOption("wavPcm24");
    await expect(input).toBeEnabled();
    await expect(input).toHaveAccessibleDescription("Off");
  });
}
