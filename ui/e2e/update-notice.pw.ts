import { openVersionSettings } from "./workbench";
import { lowContrastText } from "./ui-audits";
import { readFileSync } from "node:fs";
import { test, expect } from "./browser-test";

// UI-18: the header shows the version and, when the (here injected) release
// list has a newer release, a link to its page. No request reaches GitHub.
const version = (JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8")) as { version: string })
  .version;

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`header shows the version and a newer release in ${theme}`, async ({ page, context }, testInfo) => {
    // Count only the app page's requests; the release page popup is answered
    // locally so the test never loads github.com (whose page calls its API).
    let githubRequests = 0;
    await page.route("https://api.github.com/**", (route) => {
      githubRequests += 1;
      return route.abort();
    });
    await context.route("https://github.com/**", (route) =>
      route.fulfill({ contentType: "text/html", body: "<title>release page stub</title>" }),
    );
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      localStorage.removeItem("audiorouter.ui.update-check");
      Object.assign(window, {
        __updateCheckReleases: [
          {
            tag_name: "v9.9.9",
            draft: false,
            prerelease: true,
            assets: [{ name: "AudioRouter_9.9.9_x64-setup.exe" }],
          },
          { tag_name: "v99.0.0", draft: true, assets: [{ name: "AudioRouter_99.0.0_x64-setup.exe" }] },
          { tag_name: "v99.1.0", draft: false, assets: [] },
        ],
      });
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

    await openVersionSettings(page);
    const panel = page.getByRole("region", { name: "New versions" });
    await expect(panel.getByText("Version 9.9.9 is available.")).toBeVisible();
    await expect(panel.getByText(new RegExp(`You have ${version}\\. Next check`))).toBeVisible();
    await expect(panel.getByText(/^Checked /)).toBeVisible();
    await expect(panel.getByRole("button", { name: "Open release page" })).toBeEnabled();
    await expect(panel.getByRole("button", { name: "Check now" })).toBeEnabled();
    expect(await lowContrastText(page, "New versions")).toEqual([]);
    await panel.scrollIntoViewIfNeeded();
    await panel.screenshot({ path: testInfo.outputPath(`updates-panel-${theme}.png`) });

    // Check now fetches again and picks up a later release.
    await page.evaluate(() => {
      Object.assign(window, {
        __updateCheckReleases: [
          {
            tag_name: "v9.9.10",
            draft: false,
            prerelease: true,
            assets: [{ name: "AudioRouter_9.9.10_x64-setup.exe" }],
          },
          {
            tag_name: "v8.0.0",
            draft: false,
            prerelease: false,
            assets: [{ name: "AudioRouter_8.0.0_x64-setup.exe" }],
          },
        ],
      });
    });
    await panel.getByRole("button", { name: "Check now" }).click();
    await expect(line.getByRole("button", { name: "9.9.10 available" })).toBeVisible();
    await expect(panel.getByText("Version 9.9.10 is available.")).toBeVisible();

    // The channel choice filters prereleases immediately.
    await panel.getByRole("combobox", { name: "Versions to check" }).selectOption("stable");
    await expect(line.getByRole("button", { name: "8.0.0 available" })).toBeVisible();
    await expect(panel.getByText("Version 8.0.0 is available.")).toBeVisible();

    // Turning the check off removes the notice and forgets the last result.
    await panel.getByRole("checkbox", { name: "Check GitHub once a day for a newer version" }).uncheck();
    await expect(line.getByRole("button", { name: /available/ })).toHaveCount(0);
    await expect(panel.getByText("Checking is off.", { exact: true })).toBeVisible();
    await expect(panel.getByRole("button", { name: "Check now" })).toBeDisabled();
    expect(githubRequests).toBe(0);

    await page.reload();
    await openVersionSettings(page);
    const reloaded = page.getByRole("region", { name: "New versions" });
    await expect(reloaded.getByText("Checking is off.", { exact: true })).toBeVisible();
    await expect(reloaded.getByRole("checkbox", { name: /Check GitHub once a day/ })).not.toBeChecked();
  });
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`a failed check explains the retry in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript((theme) => {
      localStorage.setItem("audiorouter.ui.theme", theme);
      localStorage.setItem("audiorouter.ui.update-check-enabled", "on");
      localStorage.removeItem("audiorouter.ui.update-check");
      Object.defineProperty(navigator, "onLine", { configurable: true, get: () => false });
      Object.assign(window, { __updateCheckReleases: [] });
    }, theme);
    await page.goto("/route-harness.html");
    await openVersionSettings(page);
    const panel = page.getByRole("region", { name: "New versions" });
    await expect(panel.getByText("Can't check for new versions right now.")).toBeVisible();
    await expect(panel.getByText(/No internet connection\. Trying again/)).toBeVisible();
    await expect(panel.getByRole("button", { name: "Check now" })).toBeEnabled();
    expect(await lowContrastText(page, "New versions")).toEqual([]);
    await panel.getByRole("button", { name: "Check now" }).click();
    await expect(panel.getByText("Can't check for new versions right now.")).toBeVisible();
    await panel.screenshot({ path: testInfo.outputPath(`updates-failed-${theme}.png`) });
  });
}
