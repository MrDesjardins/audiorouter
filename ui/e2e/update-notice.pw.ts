import { readFileSync } from "node:fs";
import { test, expect } from "@playwright/test";

// UI-18: the header shows the version and, when the (here injected) release
// list has a newer release, a link to its page. No request reaches GitHub.
const version = (JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8")) as { version: string }).version;

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`header shows the version and a newer release in ${theme}`, async ({ page, context }, testInfo) => {
    let githubRequests = 0;
    await context.route("https://api.github.com/**", (route) => { githubRequests += 1; return route.abort(); });
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      localStorage.removeItem("audiorouter.ui.update-check");
      Object.assign(window, { __updateCheckReleases: [{ tag_name: "v9.9.9", draft: false, prerelease: true }, { tag_name: "v99.0.0", draft: true }] });
    }, theme);
    await page.goto("/route-harness.html");
    const line = page.locator(".topbar .app-version-line");
    await expect(line).toContainText(`AudioRouter ${version}`);
    const link = line.getByRole("button", { name: "9.9.9 available" });
    await expect(link).toBeVisible();
    await page.locator(".topbar").screenshot({ path: testInfo.outputPath(`update-notice-${theme}.png`) });

    const popup = context.waitForEvent("page");
    await link.click();
    expect((await popup).url()).toBe("https://github.com/MrDesjardins/audiorouter/releases/tag/v9.9.9");
    await (await popup).close();

    // Setup → turning the check off removes the notice.
    await page.getByRole("tab", { name: "Setup", exact: true }).click();
    const panel = page.getByRole("region", { name: "New versions" });
    await expect(panel).toContainText(`You have AudioRouter ${version}. Version 9.9.9 is available.`);
    await panel.scrollIntoViewIfNeeded();
    await panel.screenshot({ path: testInfo.outputPath(`updates-panel-${theme}.png`) });
    await panel.getByRole("checkbox", { name: "Check GitHub once a day for a newer version" }).uncheck();
    await expect(line.getByRole("button", { name: /available/ })).toHaveCount(0);
    await expect(panel).toContainText("Checking is off.");
    expect(githubRequests).toBe(0);
  });
}
