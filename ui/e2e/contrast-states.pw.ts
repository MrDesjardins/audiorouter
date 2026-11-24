import { expect, test } from "./browser-test";
import { AUDIT_THEMES, eachAuditedPage, openAuditHarness } from "./audit-pages";
import { lowContrastText } from "./ui-audits";
import {
  boundaryContrast,
  composedTextContrast,
  focusRingContrast,
  hoverContrast,
  type AuditResult,
} from "./ui-audits-color";

// Contrast criterion, Round 28 gaps: text over translucent layers, non-text boundaries
// (WCAG 1.4.11, 3:1), keyboard focus rings and hover states, in dark, light and high contrast.
// Each audit reports how many elements it judged; the tests require a floor so an audit that
// silently looks at nothing cannot pass.

const total = (results: AuditResult[]) => ({
  failures: [...new Set(results.flatMap((result) => result.failures))],
  judged: results.reduce((sum, result) => sum + result.judged, 0),
});

for (const theme of AUDIT_THEMES) {
  test(`text over translucent layers and field text meets AA contrast in ${theme}`, async ({ page }) => {
    test.setTimeout(90_000);
    await openAuditHarness(page, theme);
    const results: AuditResult[] = [];
    await eachAuditedPage(page, async (where) => void results.push(await composedTextContrast(page, where)));
    const { failures, judged } = total(results);
    expect(judged, "elements judged").toBeGreaterThan(800);
    expect(failures).toEqual([]);
  });

  test(`field edges and icon glyphs reach 3:1 in ${theme}`, async ({ page }) => {
    test.setTimeout(90_000);
    await openAuditHarness(page, theme);
    const results: AuditResult[] = [];
    await eachAuditedPage(page, async (where) => void results.push(await boundaryContrast(page, where)));
    const { failures, judged } = total(results);
    expect(judged, "fields and icons judged").toBeGreaterThan(80);
    expect(failures).toEqual([]);
  });

  test(`keyboard focus rings are visible and reach 3:1 in ${theme}`, async ({ page }) => {
    test.setTimeout(120_000);
    await openAuditHarness(page, theme);
    const results: AuditResult[] = [];
    results.push(await focusRingContrast(page, "workbench", 60));
    await eachAuditedPage(page, async (where) => void results.push(await focusRingContrast(page, where, 10)));
    const { failures, judged } = total(results);
    expect(judged, "focused elements judged").toBeGreaterThan(120);
    expect(failures).toEqual([]);
  });

  test(`hovered buttons and tabs keep their contrast in ${theme}`, async ({ page }) => {
    test.setTimeout(150_000);
    await openAuditHarness(page, theme);
    const results: AuditResult[] = [];
    await eachAuditedPage(
      page,
      async (where) => void results.push(await hoverContrast(page, where, "button, [role=tab]", 4)),
    );
    const { failures, judged } = total(results);
    expect(judged, "hovered elements judged").toBeGreaterThan(50);
    expect(failures).toEqual([]);
  });
}

test.describe("negative controls", () => {
  test("the translucent-layer audit reports text the solid-fill audit leaves out", async ({ page }) => {
    await openAuditHarness(page, "dark");
    await page.evaluate(() => {
      const faint = document.createElement("p");
      faint.textContent = "Faint on glass";
      // White on a 60 % white layer over the dark page: about 2.6:1.
      faint.style.cssText =
        "position:fixed;top:0;left:0;z-index:9999;color:#fff;background:rgb(255 255 255 / 60%);font-size:14px;margin:0";
      const mixed = document.createElement("p");
      mixed.textContent = "Faint mixed";
      // color-mix() is computed as color(srgb ...): the audit must still read it.
      mixed.style.cssText =
        "position:fixed;top:40px;left:0;z-index:9999;color:#fff;background:color-mix(in srgb, #fff 70%, #777);font-size:14px;margin:0";
      const field = document.createElement("input");
      field.setAttribute("aria-label", "probe field");
      field.placeholder = "Faint placeholder";
      field.style.cssText = "position:fixed;top:80px;left:0;z-index:9999;background:#444;border:1px solid #888";
      document.head.append(
        Object.assign(document.createElement("style"), {
          textContent: "input.probe::placeholder{color:#555;opacity:1}",
        }),
      );
      field.className = "probe";
      document.body.append(faint, mixed, field);
    });
    const old = await lowContrastText(page, "probe");
    expect(
      old.some((entry) => entry.includes("Faint on glass")),
      "the solid-fill audit skips translucent text",
    ).toBe(false);
    const found = (await composedTextContrast(page, "probe")).failures;
    expect(found.some((entry) => entry.includes("Faint on glass"))).toBe(true);
    expect(found.some((entry) => entry.includes("Faint mixed"))).toBe(true);
    expect(found.some((entry) => entry.includes("placeholder"))).toBe(true);
  });

  test("the edge audit reports a faint field, a faint switch track and a faint icon", async ({ page }) => {
    await openAuditHarness(page, "dark");
    await page.evaluate(() => {
      const hold = document.createElement("div");
      hold.style.cssText = "position:fixed;top:0;left:0;z-index:9999;background:#252b33;padding:12px";
      hold.innerHTML = `
        <input aria-label="faint field" style="background:#252b33;border:1px solid #3a424e">
        <button aria-label="faint icon" style="background:#252b33;color:#3a424e;border:0">
          <svg width="16" height="16"><circle cx="8" cy="8" r="6" fill="currentColor"/></svg>
        </button>
        <input aria-label="good field" style="background:#252b33;border:1px solid #8292a6">`;
      document.body.append(hold);
    });
    const { failures } = await boundaryContrast(page, "probe");
    expect(failures.some((entry) => entry.includes('"faint field"'))).toBe(true);
    expect(failures.some((entry) => entry.includes('icon button "faint icon"'))).toBe(true);
    expect(failures.some((entry) => entry.includes('"good field"'))).toBe(false);
  });

  test("the focus audit reports a missing ring and a ring that blends into the page", async ({ page }) => {
    await openAuditHarness(page, "dark");
    await page.addStyleTag({ content: "button:focus-visible{outline:none !important;box-shadow:none !important}" });
    const missing = await focusRingContrast(page, "probe", 12);
    expect(missing.failures.some((entry) => entry.includes("shows no focus indicator"))).toBe(true);
    await page.addStyleTag({
      content: "button:focus-visible{outline:2px solid #2d343e !important;outline-offset:1px !important}",
    });
    const faint = await focusRingContrast(page, "probe", 12);
    expect(faint.failures.some((entry) => /focus ring \d\.\d\d:1/.test(entry))).toBe(true);
  });

  test("the hover audit reports a button that fades when hovered", async ({ page }) => {
    await openAuditHarness(page, "dark");
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    await page.addStyleTag({
      content: ".tool-card:hover{color:#5a6372 !important;background:#3a414b !important}",
    });
    const result = await hoverContrast(page, "probe", ".tool-card", 6);
    expect(result.failures.some((entry) => entry.includes("hovered"))).toBe(true);
  });
});
