import { expect, test } from "./browser-test";
import { openSettings } from "./workbench";

// Owner, 2026-11-07: looking for "Keyboard shortcut" among fifteen pages was slow.
for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: Settings search narrows the pages and Enter opens the match`, async ({ page }, testInfo) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await openSettings(page, "Plug-ins");
    const pages = page.getByRole("tablist", { name: "Settings pages" });
    await expect(pages.getByRole("tab")).toHaveCount(15);
    await page.screenshot({ path: testInfo.outputPath(`settings-nav-${theme}.png`) });

    const search = page.getByRole("searchbox", { name: "Find a setting" });
    await search.fill("shortcut");
    await expect(pages.getByRole("tab")).toHaveText(["Keyboard shortcuts"]);
    const hint = page.locator("#settings-search-hint");
    const hintBox = await hint.boundingBox();
    await search.press("Enter");
    await expect(pages.getByRole("tab", { name: "Keyboard shortcuts" })).toHaveAttribute("aria-selected", "true");
    await page.screenshot({ path: testInfo.outputPath(`settings-search-${theme}.png`) });

    await search.fill("zzz");
    await expect(pages.getByRole("tab")).toHaveCount(0);
    await expect(hint).toContainText("No settings match");
    // UI-17: the hint changes text without moving.
    expect(await hint.boundingBox()).toEqual(hintBox);
    await search.press("Escape");
    await expect(pages.getByRole("tab")).toHaveCount(15);
  });
}
