import { expect, test } from "./browser-test";
import { openSettings } from "./workbench";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`${theme}: update toggle and asynchronous release keep their space`, async ({ page }) => {
    await page.addInitScript((value) => {
      localStorage.setItem("audiorouter.ui.theme", value);
      localStorage.setItem("audiorouter.ui.update-check-enabled", "off");
      Object.assign(window, {
        __updateCheckReleases: [
          {
            tag_name: "v99.0.0",
            draft: false,
            html_url: "https://github.com/MrDesjardins/audiorouter/releases/tag/v99.0.0",
          },
        ],
      });
    }, theme);
    await page.goto("/route-harness.html");
    await openSettings(page, "New versions");
    const panel = page.locator(".updates-panel");
    const toggle = panel.getByRole("checkbox", { name: "Check GitHub once a day for a newer version", exact: true });
    await expect(toggle).not.toBeChecked();
    await expect(panel.locator(".setting-toggle-state")).toHaveText("Off");
    const height = await panel.evaluate((element) => element.getBoundingClientRect().height);
    await toggle.focus();
    await toggle.press("Space");
    await expect(panel.locator(".setting-toggle-state")).toHaveText("On");
    await expect(toggle).toHaveAccessibleDescription("On");
    await expect(panel.getByRole("button", { name: "Open the 99.0.0 release page", exact: true })).toBeEnabled();
    expect(await panel.evaluate((element) => element.getBoundingClientRect().height)).toBe(height);
    await page.setViewportSize({ width: 640, height: 690 });
    const narrowHeight = await panel.evaluate((element) => element.getBoundingClientRect().height);
    await toggle.press("Space");
    await expect(panel.locator(".setting-toggle-state")).toHaveText("Off");
    await expect(panel.getByRole("button", { name: "No release to open", exact: true })).toBeDisabled();
    expect(await panel.evaluate((element) => element.getBoundingClientRect().height)).toBe(narrowHeight);
    await panel.screenshot({ path: `../target/ui-quality-review/round-25/updates-${theme}.png` });
  });

  test(`${theme}: setup and scan actions have separate targets`, async ({ page }) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.setViewportSize({ width: 640, height: 690 });
    await page.goto("/route-harness.html");
    for (const [name, actions] of [
      ["Device access", ["Open Tools", "Set up for Discord", "Refresh devices"]],
      ["Plug-ins", ["Scan directory", "Load last scan", "Retry scan"]],
    ] as const) {
      await openSettings(page, name);
      const panel = page.locator(".right-workbench").getByRole("tabpanel");
      await expect(panel.getByRole("heading", { name, exact: true })).toBeVisible();
      const boxes = [];
      for (const action of actions)
        boxes.push((await panel.getByRole("button", { name: action, exact: true }).boundingBox())!);
      for (let index = 1; index < boxes.length; index++) {
        const previous = boxes[index - 1],
          current = boxes[index];
        const gap =
          Math.abs(current.y - previous.y) < 1
            ? current.x - previous.x - previous.width
            : current.y - previous.y - previous.height;
        expect(gap).toBeGreaterThanOrEqual(7.5);
      }
      await panel.screenshot({
        path: `../target/ui-quality-review/round-25/actions-${theme}-${name.replaceAll(" ", "-")}.png`,
      });
    }
  });
}
