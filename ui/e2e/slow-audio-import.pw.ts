import { expect, test } from "./browser-test";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: importing audio reports progress and retains the previous file after failure`, async ({ page }) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      Object.assign(window, { __routeFixtureUploadDurationMs: 1000 });
    }, theme);
    await page.goto("/route-harness.html");
    await page.locator(".tool-card").filter({ hasText: "Audio file" }).first().click();
    await page.getByTestId("rf__node-audioFile-1").locator(".flow-node-title").click();
    const editor = page.locator(".audio-file-editor");
    const picker = editor.getByLabel("Choose WAV or MP3");
    const file = { name: "first.wav", mimeType: "audio/wav", buffer: Buffer.from([82, 73, 70, 70]) };
    await picker.setInputFiles(file);
    await expect(editor).toContainText("first.wav · 1 sec");
    await page.evaluate(() =>
      Object.assign(window, {
        __routeFixtureUploadDelayMs: 1200,
        __routeFixtureUploadError: "Fixture audio import unavailable",
      }),
    );
    await picker.setInputFiles({ ...file, name: "replacement.wav" });
    await expect(picker).toBeDisabled();
    await expect(editor.locator(".audio-file-editor-status")).toContainText("Importing");
    await editor
      .locator(".audio-file-editor-status")
      .screenshot({ path: `../target/ui-quality-review/round-26/import-${theme}.png` });
    const before = await editor.locator(".audio-file-editor-status").boundingBox();
    await expect(editor).toContainText("Fixture audio import unavailable");
    await expect(picker).toBeEnabled();
    await expect(editor.locator("strong").first()).toHaveText("first.wav");
    expect(await editor.locator(".audio-file-editor-status").boundingBox()).toEqual(before);
    await page.evaluate(() => Object.assign(window, { __routeFixtureUploadError: undefined }));
    await picker.setInputFiles({ ...file, name: "replacement.wav" });
    await expect(editor).toContainText("replacement.wav · 1 sec");
    await expect(editor.locator("strong").first()).toHaveText("replacement.wav");
  });
}
