import { expect, test } from "./browser-test";
import { openSettings } from "./workbench";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: ordinary Settings pages use the route and tool glossary`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    const hits: string[] = [];
    for (const name of [
      "Device access",
      "Plug-ins",
      "When AudioRouter starts",
      "Keyboard shortcuts",
      "Logs and support",
    ]) {
      await openSettings(page, name);
      const panel = page.locator(".right-workbench").getByRole("tabpanel");
      await expect(panel).toBeVisible();
      await expect(
        panel.getByRole("heading", { name: name === "Logs and support" ? "Client diagnostics" : name, exact: true }),
      ).toBeVisible();
      const text = await panel.evaluate((element) => {
        const copy = element.cloneNode(true) as HTMLElement;
        // Logs and API identifiers are evidence, not prose to translate.
        copy.querySelectorAll("pre, code, .diagnostic-list").forEach((item) => item.remove());
        return copy.textContent ?? "";
      });
      for (const match of text.matchAll(/\b(?:nodes?|graph|native|endpoints?|drafts?)\b/gi))
        hits.push(`${name}: ${match[0]}`);
      await panel.screenshot({
        path: `../target/ui-quality-review/round-25/settings-${theme}-${name.replaceAll(" ", "-")}.png`,
      });
    }
    expect(hits).toEqual([]);
  });
}
