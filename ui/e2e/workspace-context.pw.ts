import { expect, test } from "./browser-test";
import { fitCanvas, openSettings } from "./workbench";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: workspace context policy preserves editing and canvas menus`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const prevented = (selector: string) =>
      page
        .locator(selector)
        .first()
        .evaluate(
          (element) => !element.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true })),
        );
    expect(await prevented(".topbar")).toBe(true);
    expect(await prevented('[role="tablist"] button')).toBe(true);
    await page.evaluate(() =>
      window.addEventListener(
        "contextmenu",
        (event) => {
          document.documentElement.dataset.contextPrevented = String(event.defaultPrevented);
        },
        { once: true },
      ),
    );
    await page.locator(".topbar").click({ button: "right" });
    await expect(page.locator("html")).toHaveAttribute("data-context-prevented", "true");
    await fitCanvas(page);
    const card = page.getByTestId("rf__node-voice");
    await card.click({ button: "right" });
    await expect(page.getByRole("menu")).toBeVisible();
    await page
      .getByRole("menu")
      .screenshot({ path: `../target/ui-quality-review/round-25/workspace-menu-${theme}.png` });
    await page.keyboard.press("Escape");
    await expect(page.getByRole("menu")).toHaveCount(0);
    await card.locator(".flow-node-title").click();
    expect(await prevented(".inspector a[href]")).toBe(false);
    await page
      .getByRole("tablist", { name: "Right sidebar" })
      .getByRole("tab", { name: "Session", exact: true })
      .click();
    const field = page.getByLabel("Session name", { exact: true });
    await expect(field).toBeEditable();
    expect(
      await field.evaluate((element) =>
        element.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true })),
      ),
    ).toBe(true);
    await openSettings(page, "Logs and support");
    const text = page.locator(".workbench-page p").filter({ hasText: "Recent interface error categories" });
    await expect(text).toBeVisible();
    await text.evaluate((element) => {
      const range = document.createRange();
      range.selectNodeContents(element);
      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
    });
    expect(await page.evaluate(() => window.getSelection()?.toString())).not.toBe("");
    expect(
      await text.evaluate((element) =>
        element.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true })),
      ),
    ).toBe(true);
    await page.evaluate(() => window.getSelection()?.removeAllRanges());
    expect(await prevented(".workbench-page")).toBe(true);
  });
}
