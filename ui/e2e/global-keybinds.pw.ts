import { expect, test } from "./browser-test";
import { openSettings } from "./workbench";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`single and bulk deletion are explicit and recoverable: ${theme}`, async ({ page }) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      const rules = ["Microphone mute", "Start my session", "Talk while held"].map((name, index) => ({
        id: `kb-${index}`,
        name,
        enabled: true,
        trigger: { keys: `F${index + 13}`, mode: "press", passthrough: true },
        action: { type: "sessionToggle" },
      }));
      const saves: unknown[] = [];
      Object.assign(window, {
        __keybindSaves: saves,
        __TAURI_INTERNALS__: {
          invoke: async (command: string, args?: { rules?: unknown[] }) => {
            if (command === "keybinds_list") return rules;
            if (command === "keybinds_save") {
              saves.push(args?.rules);
              return args?.rules;
            }
            if (command === "keybinds_set_suspended") return;
            throw new Error(`not available in this test: ${command}`);
          },
        },
      });
    }, theme);
    await page.goto("/route-harness.html");
    await openSettings(page, "Keyboard shortcuts");
    const panel = page.locator(".shortcut-panel");
    await panel.locator("summary").filter({ hasText: "Microphone mute" }).click();
    const singleDelete = panel.getByRole("button", { name: "Delete keybinding Microphone mute", exact: true });
    await expect(singleDelete).toBeVisible();
    await expect(singleDelete.locator("svg path")).toHaveAttribute("d", /M3 6h18/);
    await panel.screenshot({ path: `../target/keybinding-ux/delete-single-${theme}.png` });
    await singleDelete.click();
    const dialog = page.getByRole("dialog", { name: "Delete “Microphone mute”?" });
    await expect(dialog).toContainText("removes only “Microphone mute” (F13)");
    await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(singleDelete).toBeFocused();
    await singleDelete.click();
    await dialog.getByRole("button", { name: "Delete keybinding", exact: true }).click();
    await expect(panel.locator("summary")).toHaveCount(2);
    await expect(panel.getByRole("button", { name: "Add keybinding", exact: true })).toBeFocused();
    await panel.getByRole("button", { name: "Manage keybindings", exact: true }).click();
    const bulk = panel.getByRole("button", { name: "Delete selected (0)", exact: true });
    await expect(bulk).toBeDisabled();
    const bulkBefore = await bulk.boundingBox();
    await panel.getByRole("checkbox", { name: "Select Start my session", exact: true }).press("Space");
    await panel.getByRole("checkbox", { name: "Select Talk while held", exact: true }).press("Space");
    const selectedDelete = panel.getByRole("button", { name: "Delete selected (2)", exact: true });
    expect(await selectedDelete.boundingBox()).toEqual(bulkBefore);
    await expect(selectedDelete.locator("svg path")).toHaveAttribute("d", /M3 6h18/);
    await panel.screenshot({ path: `../target/keybinding-ux/delete-bulk-${theme}.png` });
    await selectedDelete.click();
    await page
      .getByRole("dialog", { name: "Delete 2 keybindings?" })
      .getByRole("button", { name: "Delete 2 keybindings", exact: true })
      .click();
    await expect(panel).toContainText("No global keybindings yet.");
    const saves = await page.evaluate(
      () => (window as unknown as { __keybindSaves: { id: string }[][] }).__keybindSaves,
    );
    expect(saves.map((rules) => rules.map((rule) => rule.id))).toEqual([["kb-1", "kb-2"], []]);
  });

  test(`create a global keybinding in one form: ${theme}`, async ({ page }) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      const saves: unknown[] = [];
      Object.assign(window, {
        __keybindSaves: saves,
        __TAURI_INTERNALS__: {
          invoke: async (command: string, args?: { rules?: unknown[] }) => {
            if (command === "keybinds_list") return [];
            if (command === "keybinds_save") {
              saves.push(args?.rules);
              return args?.rules;
            }
            if (command === "keybinds_set_suspended") return;
            throw new Error(`not available in this test: ${command}`);
          },
        },
      });
    }, theme);
    await page.goto("/route-harness.html");
    await openSettings(page, "Keyboard shortcuts");
    const panel = page.locator(".shortcut-panel");
    await panel.getByRole("button", { name: "Add keybinding", exact: true }).click();
    const draft = panel.locator("details[open]");
    await expect(draft.getByLabel("Name", { exact: true })).toBeFocused();
    await draft.getByLabel("Name", { exact: true }).fill("Mute my microphone");
    await draft.getByRole("combobox", { name: "Action", exact: true }).selectOption("privacyMuteToggle");
    const shortcut = draft.getByRole("textbox", { name: /Global trigger/ });
    const save = draft.getByRole("button", { name: "Save keybinding" });
    const saveBeforeCapture = await save.boundingBox();
    await shortcut.focus();
    await expect(draft.getByText(/Listening/)).toBeVisible();
    expect(await save.boundingBox()).toEqual(saveBeforeCapture);
    await page.keyboard.press("Tab");
    await expect(draft.getByLabel("Enabled", { exact: true })).toBeFocused();
    await expect(shortcut).toHaveValue("");
    await shortcut.focus();
    await shortcut.press("Control+Shift+m");
    await expect(shortcut).toHaveValue(/Control.*Shift.*M/);
    await expect(draft.getByRole("button", { name: "Save keybinding" })).toBeEnabled();
    expect(await panel.evaluate((element) => element.scrollWidth <= element.clientWidth)).toBe(true);
    await panel.screenshot({ path: `../target/keybinding-ux/draft-${theme}.png` });
    await draft.getByRole("button", { name: "Save keybinding" }).click();
    await expect(panel.getByText("New keybinding", { exact: true })).toHaveCount(0);
    await expect(panel.getByRole("button", { name: "Add keybinding", exact: true })).toBeFocused();
    await expect(panel.locator("summary")).toContainText("Mute my microphone");
    await expect(panel.locator("summary")).toContainText("Turn privacy mute on or off");
    await expect(panel.locator("summary")).toContainText(/Control.*Shift.*M/);
    await panel.screenshot({ path: `../target/keybinding-ux/saved-${theme}.png` });
    await panel.getByRole("button", { name: "Add keybinding", exact: true }).click();
    await panel.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(panel.getByRole("button", { name: "Add keybinding", exact: true })).toBeFocused();
    expect(await page.evaluate(() => (window as unknown as { __keybindSaves: unknown[] }).__keybindSaves.length)).toBe(
      1,
    );
  });
}
