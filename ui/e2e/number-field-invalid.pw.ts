import { expect, test } from "./browser-test";
import { openSettings, setColorTheme } from "./workbench";

// Inputs criterion: an out-of-range number is visibly marked and says the allowed range.
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`an out-of-range number says its range in ${theme}`, async ({ page }, testInfo) => {
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await setColorTheme(page, theme);
    await openSettings(page, "API");
    const field = page.getByRole("spinbutton", { name: "API port" });
    const range = [await field.getAttribute("aria-valuemin"), await field.getAttribute("aria-valuemax")];
    await field.fill("99999");
    await expect(field).toHaveAttribute("aria-invalid", "true");
    await expect(field).toHaveAccessibleDescription(`Enter a number from ${range[0]} to ${range[1]}.`);
    const border = await field.evaluate((element) => getComputedStyle(element).borderTopColor);
    expect(await field.evaluate((element) => getComputedStyle(element).borderTopStyle)).toBe("dashed");
    // Visible in words too, not only to Narrator or on hover (owner, Round 28).
    const hint = page.locator(".number-field-hint");
    await expect(hint).toBeVisible();
    await expect(hint).toHaveText(`Enter a number from ${range[0]} to ${range[1]}.`);
    const fieldBox = (await field.boundingBox())!;
    const hintBox = (await hint.boundingBox())!;
    expect(hintBox.y, "hint sits under the field").toBeGreaterThanOrEqual(fieldBox.y + fieldBox.height);
    // Overlays rather than moving the content below (UI-17).
    const below = page.getByRole("button").filter({ visible: true }).last();
    await field.fill("2000");
    const steady = await below.boundingBox();
    await field.fill("99999");
    expect(await below.boundingBox()).toEqual(steady);
    await page.screenshot({ path: testInfo.outputPath(`invalid-${theme}.png`) });
    await field.blur();
    await expect(field).not.toHaveAttribute("aria-invalid");
    expect(await field.evaluate((element) => getComputedStyle(element).borderTopColor)).not.toBe(border);
    expect(await field.evaluate((element) => getComputedStyle(element).borderTopStyle)).toBe("solid");
  });
}
