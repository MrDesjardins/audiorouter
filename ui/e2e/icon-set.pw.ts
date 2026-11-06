import { expect, test } from "./browser-test";

test("tools, cards and connection actions draw icons, not font characters", async ({ page }) => {
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  const toolIcons = page.locator(".tool-card-icon");
  const count = await toolIcons.count();
  expect(count).toBeGreaterThan(20);
  for (let index = 0; index < count; index += 1) {
    await expect(toolIcons.nth(index).locator("svg.icon")).toHaveCount(1);
    await expect(toolIcons.nth(index)).toHaveText("");
  }
  const kinds = page.locator(".flow-node-content .node-kind");
  await expect(kinds.first().locator("svg.icon")).toHaveCount(1);
  // The word stays beside the picture.
  expect((await kinds.first().innerText()).trim().length).toBeGreaterThan(0);
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`icon set screenshots in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await page.screenshot({ path: info.outputPath(`${theme}-icons.png`) });
    const [color, background] = await page
      .locator(".tool-card-icon svg")
      .first()
      .evaluate((svg) => {
        const icon = getComputedStyle(svg).color;
        return [icon, getComputedStyle(svg.parentElement!).backgroundColor];
      });
    expect(color).not.toBe(background);
  });
}
