import { expect, test } from "./browser-test";
import { fitCanvas } from "./workbench";

test("help shows custom bindings, fits a short window and leaves another dialog alone", async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem(
      "audiorouter.ui.shortcuts",
      JSON.stringify({
        sessionToggle: "Control+Alt+P",
        privacyMute: "Control+Alt+U",
      }),
    ),
  );
  await page.setViewportSize({ width: 640, height: 360 });
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.getByRole("tab", { name: "Tools", exact: true }).focus();
  await page.keyboard.press("?");
  const help = page.getByRole("dialog", { name: "Keyboard shortcuts", exact: true });
  await expect(help).toContainText("Ctrl+Alt+P");
  await expect(help).toContainText("Ctrl+Alt+U");
  await expect(help.getByRole("button", { name: "Close", exact: true })).toBeInViewport();
  const box = (await help.boundingBox())!;
  expect(box.y).toBeGreaterThanOrEqual(0);
  expect(box.y + box.height).toBeLessThanOrEqual(360);
  await page.keyboard.press("Escape");
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  await page.getByRole("button", { name: "New", exact: true }).click();
  const question = page.getByRole("dialog", { name: "New session", exact: true });
  await question.getByRole("textbox").press("F1");
  await expect(help).toHaveCount(0);
  await question.getByRole("button", { name: "Cancel", exact: true }).focus();
  await page.keyboard.press("F1");
  await expect(help).toHaveCount(0);
  await expect(question).toBeVisible();
});

test("help marks conflicting custom audio shortcuts as disabled", async ({ page }) => {
  await page.addInitScript(() =>
    localStorage.setItem(
      "audiorouter.ui.shortcuts",
      JSON.stringify({
        sessionToggle: "Control+Alt+P",
        privacyMute: "Control+Alt+P",
      }),
    ),
  );
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.getByRole("tab", { name: "Tools", exact: true }).focus();
  await page.keyboard.press("F1");
  const help = page.getByRole("dialog", { name: "Keyboard shortcuts", exact: true });
  await expect(
    help.locator("dd").filter({ hasText: "disabled: resolve the shortcut conflict in Settings" }),
  ).toHaveCount(2);
});

test("F2 renames the selected tool and cancellation preserves it", async ({ page }) => {
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await fitCanvas(page);
  const card = page.getByTestId("rf__node-voice");
  await card.locator(".flow-node-title").click();
  await card.focus();
  await page.keyboard.press("F2");
  const dialog = page.getByRole("dialog", { name: "Rename tool" });
  const name = dialog.getByRole("textbox", { name: "Tool name", exact: true });
  await expect(name).toBeFocused();
  await expect(name).toHaveValue("Voice gain");
  await name.fill("");
  await page.keyboard.press("Enter");
  await expect(dialog).toContainText("Enter a name first");
  await name.fill("New voice name");
  await page.keyboard.press("Escape");
  await expect(card).toBeFocused();
  await expect(card.locator(".flow-node-title")).toContainText("Voice gain");
  await page.keyboard.press("F2");
  await name.fill("New voice name");
  await page.keyboard.press("Enter");
  await expect(card.locator(".flow-node-title")).toContainText("New voice name");
  await expect(card).toBeFocused();
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: one-key help traps focus and returns to its origin`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.setViewportSize({ width: 640, height: 690 });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const origin = page.getByRole("tab", { name: "Tools", exact: true });
    await origin.focus();
    await page.keyboard.press("F1");
    const dialog = page.getByRole("dialog", { name: "Keyboard shortcuts", exact: true });
    const close = dialog.getByRole("button", { name: "Close", exact: true });
    await expect(close).toBeFocused();
    await expect(dialog).toContainText("Ctrl+Shift+M");
    await expect(dialog).toContainText("F2");
    await page.keyboard.press("Tab");
    const reference = dialog.locator("dl");
    await expect(reference).toBeFocused();
    await page.keyboard.press("End");
    await expect.poll(() => reference.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
    await page.keyboard.press("Home");
    await expect.poll(() => reference.evaluate((element) => element.scrollTop)).toBe(0);
    await page.keyboard.press("Shift+Tab");
    await expect(close).toBeFocused();
    const box = (await dialog.boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(640);
    expect(box.y + box.height).toBeLessThanOrEqual(690);
    await dialog.screenshot({ path: `../target/ui-quality-review/round-25/help-${theme}.png` });
    await page.keyboard.press("Escape");
    await expect(origin).toBeFocused();
    await page.keyboard.press("?");
    await expect(dialog).toBeVisible();
    await close.click();
    await expect(origin).toBeFocused();
  });
}
