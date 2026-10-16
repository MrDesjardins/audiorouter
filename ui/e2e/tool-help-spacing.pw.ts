import { expect, test } from "@playwright/test";

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`Properties clears every tab and compact help works in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
    for (const width of [1440, 1120, 980]) {
      await page.setViewportSize({ width, height: 900 });
      await expect(page.getByRole("tab", { name: "Properties", exact: true })).toHaveAttribute("aria-selected", "true");
      await expect
        .poll(async () =>
          page.evaluate(() => {
            const inspector = document.querySelector(".main-content > .inspector")!.getBoundingClientRect();
            const tabs = [...document.querySelectorAll(".workbench-tabs [role=tab]")];
            return inspector.top - Math.max(...tabs.map((tab) => tab.getBoundingClientRect().bottom));
          }),
        )
        .toBeGreaterThanOrEqual(10);
      for (const tab of await page.locator(".workbench-tabs [role=tab]").all()) {
        const box = (await tab.boundingBox())!;
        expect(
          await page.evaluate(({ x, y }) => document.elementFromPoint(x, y)?.closest("[role=tab]") !== null, {
            x: box.x + box.width / 2,
            y: box.y + box.height / 2,
          }),
        ).toBe(true);
      }
      await page.screenshot({ path: info.outputPath(`${theme}-properties-${width}.png`) });
    }
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    const card = page
      .locator(".tool-library-entry")
      .filter({ has: page.getByRole("link", { name: "Open Gain guide", exact: true }) });
    const link = card.getByRole("link");
    const button = card.getByRole("button");
    const cardBox = (await button.boundingBox())!,
      helpBox = (await link.boundingBox())!;
    expect(helpBox.x).toBeGreaterThan(cardBox.x);
    expect(helpBox.x + helpBox.width).toBeLessThan(cardBox.x + cardBox.width);
    expect(helpBox.y + helpBox.height).toBeLessThan(cardBox.y + cardBox.height);
    await expect(page.locator(".workbench-page .tool-guide-access:not(.is-compact)")).toHaveCount(0);
    await expect(page.locator("button a, a button")).toHaveCount(0);
    const count = await page.locator(".react-flow__node").count();
    await page.evaluate(() =>
      Object.assign(window, {
        __guideCalls: [],
        __TAURI_INTERNALS__: {
          invoke: async (command: string, args: unknown) => {
            (window as unknown as { __guideCalls: unknown[] }).__guideCalls.push([command, args]);
          },
        },
      }),
    );
    await link.focus();
    await page.keyboard.press("Enter");
    expect(await page.evaluate(() => (window as unknown as { __guideCalls: unknown[] }).__guideCalls)).toEqual([
      ["open_tool_guide", { kind: "gain" }],
    ]);
    await expect(page.locator(".react-flow__node")).toHaveCount(count);
    await page.evaluate(() => {
      delete window.__TAURI_INTERNALS__;
    });
    await page.screenshot({ path: info.outputPath(`${theme}-compact-tool-help.png`) });
    await button.click();
    await expect(page.locator(".react-flow__node")).toHaveCount(count + 1);
  });
}
