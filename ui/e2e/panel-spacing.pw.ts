import { expect, test } from "./browser-test";
import { crampedLayout } from "./ui-audits";
import { fitCanvas, openAllSections, setColorTheme } from "./workbench";

// Spacing criterion for every panel in the sidebar tabs and on every Settings page: content
// keeps 12 px from the panel's edge, nothing overlaps or touches, and buttons on one row line
// up. Sections are opened first so their contents are judged too.
// Tool shelf rows sit flush like menu items (Menus criterion), so only overlaps between rows count.
const TOOL_ROWS = ".tool-row, .tool-library-entry, .tool-setup, .tool-group-title";
const PANELS = "section.panel, .inspector, .setup-panel, .app-card";

const layouts = [
  ...["dark", "light", "high-contrast"].map((theme) => ({ theme, width: 1500, height: 950 })),
  { theme: "dark", width: 1024, height: 700 },
];
for (const { theme, width, height } of layouts) {
  test(`sidebar and Settings panels are evenly spaced in ${theme} at ${width}x${height}`, async ({ page }) => {
    test.setTimeout(90_000);
    await page.setViewportSize({ width, height });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await setColorTheme(page, theme);
    const found: string[] = [];
    for (const name of ["Tools", "Properties", "Status", "Session", "Recording"]) {
      const tab = page.getByRole("tablist", { name: "Right sidebar" }).getByRole("tab", { name, exact: true });
      if ((await tab.count()) === 0) continue;
      await tab.click();
      await openAllSections(page.locator(".right-workbench"));
      found.push(
        ...(await crampedLayout(page, name, `.right-workbench [role=tabpanel], .right-workbench :is(${PANELS})`, {
          fit: false,
          rows: TOOL_ROWS,
        })),
      );
    }
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    const list = page.getByRole("tablist", { name: "Settings pages" });
    for (const name of (await list.getByRole("tab").allInnerTexts()).map((value) => value.trim())) {
      const tab = list.getByRole("tab", { name, exact: true });
      await tab.click();
      await expect(tab).toHaveAttribute("aria-selected", "true");
      found.push(
        ...(await crampedLayout(
          page,
          `Settings/${name}`,
          `.right-workbench [role=tabpanel], .right-workbench :is(${PANELS})`,
          { fit: false },
        )),
      );
    }
    expect([...new Set(found).values()].filter((line) => !line.endsWith("not found"))).toEqual([]);
  });
}

for (const { theme, width, height } of layouts) {
  test(`every tool's Properties is evenly spaced in ${theme} at ${width}x${height}`, async ({ page }, info) => {
    // Adds one of every tool, then visits each card: about 35 Properties panels.
    test.setTimeout(120_000);
    await page.setViewportSize({ width, height });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await setColorTheme(page, theme);
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    const tools = page.locator(".tool-card");
    const toolCount = await tools.count();
    for (let index = 0; index < toolCount; index += 1) {
      await page.getByRole("tab", { name: "Tools", exact: true }).click();
      await tools.nth(index).focus();
      await page.keyboard.press("Enter");
      await page.keyboard.press("Escape");
    }
    await fitCanvas(page);
    const found: string[] = [];
    for (const card of await page.locator(".react-flow__node .flow-node-title").all()) {
      const title = (await card.innerText()).trim();
      await card.dispatchEvent("click");
      await expect(page.locator(".main-content > .inspector")).toContainText(title);
      await openAllSections(page.locator(".main-content > .inspector"));
      found.push(...(await crampedLayout(page, `Properties/${title}`, ".main-content > .inspector", { fit: false })));
      if (/compressor|gate|limiter|duck|eq|bass|dehum/i.test(title))
        await page.locator(".main-content > .inspector").screenshot({
          path: info.outputPath(`${title.replace(/[^a-z0-9-]/gi, "-")}.png`),
        });
    }
    expect([...new Set(found)]).toEqual([]);
  });
}
