import type { Page } from "@playwright/test";
import { demoSession } from "../src/fixtures";
import { expect, test } from "./browser-test";
import { clippedText, crampedLayout, smallTargets } from "./ui-audits";
import {
  fitCanvas,
  openConnectionForm,
  openDeviceTroubleshooting,
  setColorTheme,
  stereoDemoSession,
} from "./workbench";

// Spacing criterion for every dialog the app opens: it fits the window, keeps its content
// 12 px from its edges, nothing overlaps or touches, and buttons on one row line up. Checked
// in three themes at a roomy and a small window.
const MODAL = '[role=dialog][aria-modal="true"]';

const addTool = async (page: Page, name: string) => {
  await page.getByRole("tab", { name: "Tools", exact: true }).click();
  await page.locator(".tool-card", { hasText: name }).first().click();
};

const DIALOGS: {
  name: string;
  root?: string;
  init?: (page: Page) => Promise<void>;
  open: (page: Page) => Promise<void>;
}[] = [
  {
    name: "Rename tool",
    open: async (page) => {
      await fitCanvas(page);
      const card = page.locator(".react-flow__node").first();
      await card.locator(".flow-node-title").click();
      await card.focus();
      await page.keyboard.press("F2");
      await expect(page.getByRole("dialog", { name: "Rename tool" })).toBeVisible();
    },
  },
  {
    name: "New session",
    open: async (page) => {
      await page.getByRole("tab", { name: "Session", exact: true }).click();
      await page.getByRole("button", { name: "New", exact: true }).click();
      await expect(page.getByRole("dialog", { name: "New session" })).toBeVisible();
    },
  },
  {
    name: "Delete session",
    open: async (page) => {
      await page.getByRole("tab", { name: "Session", exact: true }).click();
      await page.getByRole("button", { name: "Delete session", exact: true }).click();
      await expect(page.getByRole("dialog", { name: /^Delete/ })).toBeVisible();
    },
  },
  {
    name: "Connect without dragging",
    root: ".canvas-keyboard-panel",
    open: async (page) => {
      await openConnectionForm(page);
      await expect(page.locator(".canvas-keyboard-panel")).toBeVisible();
    },
  },
  {
    name: "Route list",
    root: ".canvas-keyboard-panel",
    open: async (page) => {
      await page
        .getByRole("group", { name: "Canvas layout actions" })
        .getByRole("button", { name: "Route list", exact: true })
        .click();
      await expect(page.locator(".canvas-keyboard-panel")).toBeVisible();
    },
  },
  {
    name: "Plugin picker",
    open: async (page) => {
      await page.getByRole("tab", { name: "Tools", exact: true }).click();
      await page.getByRole("button", { name: "Scan another folder…" }).click();
      await expect(page.locator(MODAL)).toBeVisible();
    },
  },
  {
    name: "Switch session while playing",
    init: async (page) => {
      await page.addInitScript((session) => {
        Object.assign(window, {
          __routeFixtureSession: session,
          __routeFixtureSessions: [session, { ...session, id: "other", name: "Other saved route" }],
          __routeFixtureRunning: true,
          __routeFixtureActiveSessionIds: [session.id],
        });
      }, demoSession);
    },
    open: async (page) => {
      await page.getByRole("tab", { name: "Session", exact: true }).click();
      await page.getByLabel("Choose session").selectOption("other");
      await expect(page.getByRole("dialog", { name: "Stop audio and switch session?" })).toBeVisible();
    },
  },
  {
    name: "App capture picker",
    open: async (page) => {
      await addTool(page, "Application source");
      await expect(page.getByRole("dialog", { name: "Add an application capture source" })).toBeVisible();
    },
  },
  {
    name: "Sound picker",
    open: async (page) => {
      await addTool(page, "Soundboard");
      await page.getByRole("tab", { name: "Properties", exact: true }).click();
      await page.getByRole("button", { name: "Add sound", exact: true }).click();
      await expect(page.locator(MODAL)).toBeVisible();
    },
  },
  {
    name: "Device access",
    init: async (page) => {
      await page.addInitScript(
        (session) => Object.assign(window, { __routeFixtureFreshInstall: true, __routeFixtureSession: session }),
        stereoDemoSession,
      );
    },
    open: async (page) => {
      // The question comes at the first Play of a connected, device-bound route.
      await openConnectionForm(page);
      const editor = page.locator(".workbench-connection-editor");
      for (const [source, target] of [
        ["Microphone · out · 2ch", "Voice gain · in · 2ch"],
        ["Voice gain · out · 2ch", "Headphones · in · 2ch"],
      ]) {
        await editor.getByLabel("Source output port").selectOption({ label: source });
        await editor.getByLabel("Destination input port").selectOption({ label: target });
        await editor.getByRole("button", { name: "Add connection", exact: true }).click();
      }
      await page.locator(".topbar").getByRole("button", { name: "Save", exact: true }).click();
      await openDeviceTroubleshooting(page);
      const sidebar = page.locator(".right-workbench");
      await sidebar.getByLabel("Native capture endpoint").selectOption("capture-preview");
      await sidebar.getByLabel("Native render endpoint").selectOption("render-preview");
      await page.locator(".topbar").getByRole("button", { name: "Play", exact: true }).click();
      await expect(page.getByRole("dialog", { name: "Allow AudioRouter to use your audio devices?" })).toBeVisible();
    },
  },
  {
    name: "Goal app step",
    open: async (page) => {
      await page.locator(".react-flow__pane").click();
      await page.keyboard.press("Control+K");
      await page.getByRole("combobox").fill("start from");
      await page.keyboard.press("Enter");
      await page.getByRole("button", { name: "Use this: Game and voice for Discord" }).click();
      await expect(page.getByLabel("Find an app")).toBeVisible();
    },
  },
];

for (const [width, height] of [
  [1500, 950],
  [1024, 700],
]) {
  for (const theme of ["dark", "light", "high-contrast"]) {
    for (const dialog of DIALOGS) {
      test(`${dialog.name} dialog is evenly spaced at ${width}x${height} in ${theme}`, async ({ page }, testInfo) => {
        await page.setViewportSize({ width, height });
        await dialog.init?.(page);
        await page.goto("/route-harness.html");
        await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
        await setColorTheme(page, theme);
        await dialog.open(page);
        await page
          .locator(dialog.root ?? MODAL)
          .last()
          .screenshot({ path: testInfo.outputPath(`${dialog.name}-${width}-${theme}.png`) });
        const problems = [
          ...(await crampedLayout(page, dialog.name, dialog.root ?? MODAL)),
          ...(await smallTargets(page, dialog.name, dialog.root ?? MODAL)),
          ...(await clippedText(page, dialog.name, dialog.root ?? MODAL)),
        ];
        expect([...new Set(problems)]).toEqual([]);
      });
    }
  }
}

test("the spacing audit reports touching, overlapping, edge-hugging and misaligned items", async ({ page }) => {
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.evaluate(() => {
    const probe = document.createElement("section");
    probe.className = "probe-dialog";
    probe.style.cssText = "position:fixed;top:100px;left:100px;width:300px;padding:2px;background:#000";
    probe.innerHTML = `<p style="margin:0">Edge text</p><input aria-label="Field" style="display:block;margin:0">
      <button style="margin:0">Touching</button><button style="position:relative;top:8px">Low</button>
      <span style="position:absolute;top:30px;left:10px">Over the field</span>`;
    document.body.append(probe);
  });
  const found = (await crampedLayout(page, "probe", ".probe-dialog")).join("\n");
  expect(found).toContain('"Edge text" 2 px from the edge');
  expect(found).toMatch(/"Field" touches "Touching"/);
  expect(found).toMatch(/overlaps/);
  expect(found).toMatch(/do not line up/);
});
