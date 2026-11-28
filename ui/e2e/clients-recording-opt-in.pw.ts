import { expect, test } from "./browser-test";
import { AUDIT_THEMES, openAuditHarness } from "./audit-pages";
import { openSettings } from "./workbench";

// Owner decision A1 (2026-11-27): the per-client recording opt-in renders as
// an app-styled checkbox, off by default, inside the panel in every theme.
for (const theme of AUDIT_THEMES) {
  test(`Connected clients shows the recording opt-in in ${theme}`, async ({ page }, testInfo) => {
    await openAuditHarness(page, theme);
    await openSettings(page, "Connected clients");
    const panel = page.locator(".clients-panel");
    const box = panel.getByRole("checkbox", { name: "Also allow recordings" });
    await expect(box).toBeVisible();
    await expect(box).not.toBeChecked();
    await expect(panel.getByText(/cannot choose the recording folder/)).toBeVisible();
    const noteBounds = await panel.locator("#client-recording-note").boundingBox();
    const authorizeBounds = await panel.getByRole("button", { name: "Authorize client", exact: true }).boundingBox();
    expect(noteBounds && authorizeBounds).toBeTruthy();
    if (noteBounds && authorizeBounds) {
      expect(authorizeBounds.y - (noteBounds.y + noteBounds.height)).toBeGreaterThanOrEqual(8);
    }
    const panelBox = await panel.boundingBox();
    const boxBounds = await box.boundingBox();
    expect(panelBox && boxBounds).toBeTruthy();
    if (panelBox && boxBounds) {
      expect(boxBounds.x).toBeGreaterThanOrEqual(panelBox.x);
      expect(boxBounds.x + boxBounds.width).toBeLessThanOrEqual(panelBox.x + panelBox.width);
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(1500);
    await panel.screenshot({ path: testInfo.outputPath(`clients-${theme}.png`) });
  });
}
