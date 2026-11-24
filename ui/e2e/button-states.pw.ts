import { expect, test } from "./browser-test";
import { AUDIT_THEMES, eachAuditedPage, openAuditHarness } from "./audit-pages";
import { pressedStates, type AuditResult } from "./ui-audits-color";

// Buttons criterion, Round 28 gap "pressed states": a toggle that is on must look on. The Route
// list, Connect and Map toolbar buttons said so only through aria-pressed until this audit found
// they looked identical pressed and not pressed in all three themes.

for (const theme of AUDIT_THEMES) {
  test(`toggle buttons look different when pressed in ${theme}`, async ({ page }, testInfo) => {
    test.setTimeout(90_000);
    await openAuditHarness(page, theme);
    const results: AuditResult[] = [];
    results.push(await pressedStates(page, "toolbar"));
    const bar = page.getByRole("group", { name: "Canvas layout actions" });
    await bar.getByRole("button", { name: "Connect", exact: true }).click();
    await page.mouse.move(0, 0);
    await page.waitForTimeout(250);
    await bar.screenshot({ path: testInfo.outputPath(`toolbar-connect-pressed-${theme}.png`) });
    await bar.getByRole("button", { name: "Connect", exact: true }).click();
    await eachAuditedPage(page, async (where) => void results.push(await pressedStates(page, where)));
    const judged = results.reduce((sum, result) => sum + result.judged, 0);
    expect(judged, "toggle buttons judged").toBeGreaterThanOrEqual(3);
    expect([...new Set(results.flatMap((result) => result.failures))]).toEqual([]);
  });
}

test("the pressed-state audit reports a toggle that looks the same on and off", async ({ page }) => {
  await openAuditHarness(page, "dark");
  await page.addStyleTag({
    content:
      '.canvas-layout-actions > button[aria-pressed="true"]{border-color:#46515f !important;box-shadow:none !important;background:#252b33 !important}',
  });
  const result = await pressedStates(page, "probe");
  expect(result.judged).toBeGreaterThanOrEqual(3);
  expect(result.failures.length).toBeGreaterThanOrEqual(3);
  expect(result.failures.every((entry) => /looks the same|faint/.test(entry))).toBe(true);
});
