import type { Page } from "@playwright/test";
import { expect, test } from "./browser-test";
import { clippedText, lowContrastText, smallTargets } from "./ui-audits";
import { openSettings, setColorTheme } from "./workbench";

// Buttons, Contrast and Spacing criteria for dialogs outside Settings.
const DIALOGS: { name: string; open: (page: Page) => Promise<void> }[] = [
  {
    name: "Keyboard shortcuts",
    open: async (page) => {
      await page.locator(".react-flow__pane").click();
      await page.keyboard.press("F1");
      await expect(page.getByRole("dialog", { name: "Keyboard shortcuts" })).toBeVisible();
    },
  },
  {
    name: "Command search",
    open: async (page) => {
      await page.locator(".react-flow__pane").click();
      await page.keyboard.press("Control+K");
      await expect(page.getByRole("dialog")).toBeVisible();
      await page.keyboard.type("gain");
    },
  },
  {
    name: "Goal picker",
    open: async (page) => {
      await page.locator(".react-flow__pane").click();
      await page.keyboard.press("Control+K");
      await page.getByRole("combobox").fill("start from");
      await page.keyboard.press("Enter");
      await expect(page.getByRole("dialog", { name: "What do you want to do?" })).toBeVisible();
    },
  },
  ...["Add tools from here", "Connect the dots", "Press Play"].map((step, index) => ({
    name: `Tour step ${index + 1}`,
    open: async (page: Page) => {
      await openSettings(page, "Help");
      await page.getByRole("button", { name: "Take the tour" }).click();
      for (let next = 0; next < index; next += 1) await page.getByRole("button", { name: "Next" }).click();
      await expect(page.getByRole("dialog", { name: step })).toBeVisible();
    },
  })),
];

for (const theme of ["dark", "light", "high-contrast"]) {
  for (const dialog of DIALOGS) {
    test(`${dialog.name} dialog passes size, contrast and clipping audits in ${theme}`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width: 1500, height: 950 });
      await page.goto("/route-harness.html");
      await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
      await setColorTheme(page, theme);
      await dialog.open(page);
      await page.screenshot({ path: testInfo.outputPath(`${dialog.name}-${theme}.png`) });
      const problems = [
        ...(await smallTargets(page, dialog.name)),
        ...(await lowContrastText(page, dialog.name)),
        ...(await clippedText(page, dialog.name)),
      ];
      expect([...new Set(problems)]).toEqual([]);
    });
  }
}
