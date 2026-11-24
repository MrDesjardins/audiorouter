import type { Page } from "@playwright/test";
import type { PluginScanEntry } from "@audiorouter/contracts";
import { appendDraftConnection, appendLibraryNode, appendPluginPlaceholderNode } from "../src/draft";
import { demoSession } from "../src/fixtures";
import { AUDIT_THEMES } from "./audit-pages";
import { expect, test } from "./browser-test";
import { clippedText, smallTargets } from "./ui-audits";
import { boundaryContrast, composedTextContrast, focusRingContrast } from "./ui-audits-color";
import { openConnectionForm, stereoDemoSession } from "./workbench";

// Spacing and Buttons criteria for the dialogs the Round 28 table listed as not yet audited:
// connection, plugin and confirmation dialogs. Every dialog gets the size, clipping, contrast,
// field-edge and focus audits of dialog-audits.pw.ts, plus two rules about its action row:
//  - order: a cancel / keep choice comes before the confirming button (primary or danger);
//  - spacing: buttons in a row share one height and are separated by equal gaps.

const entry: PluginScanEntry = {
  path: "C:\\Program Files\\Common Files\\VST3\\audit-fixture.vst3",
  identity: {
    path: "C:\\Program Files\\Common Files\\VST3\\audit-fixture.vst3",
    binaryPath: "C:\\Program Files\\Common Files\\VST3\\audit-fixture.vst3\\Contents\\x86_64-win\\audit-fixture.vst3",
    format: "vst3",
    architecture: "x64",
    fileBytes: 1024,
    sha256: "b".repeat(64),
    vendor: "AudioRouter test fixture",
    version: "1.0.0",
    classIds: [],
    compatibility: "supportedVst3X64",
    instrument: false,
  },
  error: null,
  errorCode: null,
};
const withPlugin = appendPluginPlaceholderNode(demoSession, entry);
const sidechainSession = {
  ...withPlugin,
  nodes: withPlugin.nodes.map((node) =>
    node.id === "plugin-1"
      ? { ...node, ports: [...node.ports, { name: "sidechain", direction: "input" as const, channels: 1 as const }] }
      : node,
  ),
};
let replacementSession = appendLibraryNode({ ...sidechainSession, edges: [] }, "testSignal");
replacementSession = appendDraftConnection(replacementSession, "mic", "out", "plugin-1", "sidechain");
const replacementSource = replacementSession.nodes.find((node) => node.kind === "testSignal")!;

interface AuditedDialog {
  name: string;
  /** The fixture session and flags the page needs before it loads. */
  fixture?: Record<string, unknown>;
  open: (page: Page) => Promise<void>;
  /** How the dialog is found after `open`. */
  dialog: (page: Page) => ReturnType<Page["getByRole"]>;
}

/** The audits look inside the open modal only, not at the page behind it. */
const MODAL = '[role="dialog"][aria-modal="true"]';

const sessionTab = (page: Page) => page.getByRole("tab", { name: "Session", exact: true }).click();

const DIALOGS: AuditedDialog[] = [
  {
    name: "Delete session confirmation",
    open: async (page) => {
      await sessionTab(page);
      await page.getByRole("button", { name: "Delete session", exact: true }).click();
    },
    dialog: (page) => page.getByRole("dialog", { name: /Delete/ }),
  },
  {
    name: "New session prompt",
    open: async (page) => {
      await sessionTab(page);
      await page.getByRole("button", { name: "New", exact: true }).click();
    },
    dialog: (page) => page.getByRole("dialog", { name: "New session" }),
  },
  {
    name: "Rename tool prompt",
    open: async (page) => {
      // F2 on a selected canvas card asks for its name in the app's own prompt.
      await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
      await page.keyboard.press("F2");
    },
    dialog: (page) => page.getByRole("dialog").filter({ has: page.getByRole("textbox") }),
  },
  {
    name: "Plugin picker",
    open: async (page) => {
      await page.getByRole("tab", { name: "Tools", exact: true }).click();
      await page.getByRole("button", { name: "Scan another folder…" }).click();
    },
    dialog: (page) => page.getByRole("dialog", { name: "Add a VST2/VST3 plugin" }),
  },
  {
    name: "Application picker",
    open: async (page) => {
      await page.getByRole("tab", { name: "Tools", exact: true }).click();
      await page.locator(".tool-card").filter({ hasText: "Application source" }).click();
    },
    dialog: (page) => page.getByRole("dialog").filter({ has: page.getByRole("button", { name: "App group" }) }),
  },
  {
    // The opener sits in a panel that is display:none in the real window (styles.css,
    // `#signal-flow-panel > .connection-editor`), so only tests reach it. Its CSS is shared with
    // the plugin and application pickers.
    name: "Keyboard connection dialog",
    open: async (page) => {
      await page.locator("button", { hasText: "Keyboard connection dialog" }).dispatchEvent("click");
    },
    dialog: (page) => page.getByRole("dialog", { name: "Keyboard connection" }),
  },
  {
    name: "Replace sidechain confirmation",
    fixture: { __routeFixtureSession: replacementSession },
    open: async (page) => {
      await openConnectionForm(page);
      const form = page.locator(".workbench-connection-editor");
      const source = form.getByRole("combobox", { name: "Source output port" });
      const destination = form.getByRole("combobox", { name: "Destination input port" });
      const sourceValue = await source
        .locator("option")
        .filter({ hasText: replacementSource.name })
        .last()
        .getAttribute("value");
      const inputValue = await destination
        .locator("option")
        .filter({ hasText: "sidechain" })
        .last()
        .getAttribute("value");
      await source.selectOption(sourceValue!);
      await destination.selectOption(inputValue!);
      await form.getByRole("button", { name: "Add connection" }).click();
    },
    dialog: (page) => page.getByRole("dialog", { name: "Replace this sidechain source?" }),
  },
  {
    name: "Remove plugin confirmation",
    fixture: { __routeFixtureSession: withPlugin },
    open: async (page) => {
      await page.getByTestId("rf__node-plugin-1").locator(".flow-node-title").click();
      await page.getByRole("tab", { name: "Properties", exact: true }).click();
      await page
        .getByRole("button", { name: /Remove plugin|Unload/i })
        .first()
        .click();
    },
    dialog: (page) => page.getByRole("dialog"),
  },
  {
    name: "Device access question",
    fixture: { __routeFixtureFreshInstall: true, __routeFixtureSession: stereoDemoSession },
    open: async (page) => {
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
      await expect(page.locator(".global-action-message")).toContainText(/saved.*revision/i);
      await page.getByRole("tablist", { name: "Right sidebar" }).getByRole("tab", { name: "Status" }).click();
      await page
        .getByRole("tablist", { name: "Status views" })
        .getByRole("tab", { name: "Readiness", exact: true })
        .click();
      for (const selector of ["details.troubleshoot-group", "details.device-troubleshooting"]) {
        const group = page.locator(selector);
        if (!(await group.evaluate((element) => (element as HTMLDetailsElement).open)))
          await group.locator("> summary").click();
      }
      const sidebar = page.locator(".right-workbench");
      await sidebar.getByLabel("Native capture endpoint").selectOption("capture-preview");
      await sidebar.getByLabel("Native render endpoint").selectOption("render-preview");
      await page.locator(".topbar").getByRole("button", { name: "Play", exact: true }).click();
    },
    dialog: (page) => page.getByRole("dialog", { name: "Allow AudioRouter to use your audio devices?" }),
  },
];

/** Presses Tab and Shift+Tab repeatedly; reports each press that moved focus out of the modal. */
const focusEscapes = async (page: Page, name: string) => {
  const outside: string[] = [];
  for (const key of ["Tab", "Tab", "Tab", "Tab", "Tab", "Shift+Tab", "Shift+Tab", "Shift+Tab", "Shift+Tab"]) {
    await page.keyboard.press(key);
    const inside = await page.evaluate((modal) => !!document.activeElement?.closest(modal), MODAL);
    if (!inside) outside.push(`${name}: focus left the dialog on ${key}`);
  }
  return outside;
};

/** The action rows of a dialog: order and spacing problems, plus the gap each row uses. */
const actionRows = (dialog: ReturnType<Page["getByRole"]>, name: string) =>
  dialog.evaluate((root, label) => {
    const problems: string[] = [];
    const gaps: number[] = [];
    let rows = 0;
    for (const row of root.querySelectorAll<HTMLElement>(".actions")) {
      const buttons = [...row.querySelectorAll<HTMLElement>("button")].filter(
        (button) => button.getBoundingClientRect().width > 0,
      );
      if (buttons.length < 2) continue;
      rows += 1;
      const boxes = buttons.map((button) => button.getBoundingClientRect());
      // Only buttons on one line are compared: a wrapped row has no horizontal gap to judge.
      const sameLine = boxes.every((box) => Math.abs(box.top - boxes[0].top) < 2);
      if (sameLine) {
        const between = boxes.slice(1).map((box, index) => box.left - boxes[index].right);
        gaps.push(...between);
        if (Math.max(...between) - Math.min(...between) > 1)
          problems.push(`${label}: unequal gaps ${between.map((gap) => gap.toFixed(1)).join(", ")}`);
        if (Math.max(...boxes.map((box) => box.height)) - Math.min(...boxes.map((box) => box.height)) > 1)
          problems.push(`${label}: buttons differ in height`);
      }
      // Only a row that offers a way out is a decision row; a picker's own action rows are not.
      const affirming = buttons.filter((button) => /\b(primary|danger)\b/.test(button.className));
      const declining = buttons.filter((button) =>
        /^(cancel|not now|keep|no\b|never)/i.test(button.textContent?.trim() ?? ""),
      );
      if (affirming.length && declining.length) {
        const lastDecline = Math.max(...declining.map((button) => buttons.indexOf(button)));
        const firstAffirm = Math.min(...affirming.map((button) => buttons.indexOf(button)));
        const reading = (button: HTMLElement) => boxes[buttons.indexOf(button)].left;
        if (firstAffirm < lastDecline || Math.min(...affirming.map(reading)) < Math.max(...declining.map(reading)))
          problems.push(
            `${label}: confirming button comes before "${declining[declining.length - 1].textContent?.trim()}"`,
          );
      }
    }
    return { problems, gaps, rows };
  }, name);

for (const theme of AUDIT_THEMES) {
  for (const spec of DIALOGS) {
    test(`${spec.name} passes size, clipping, contrast, edge, focus and button-row audits in ${theme}`, async ({
      page,
    }, testInfo) => {
      await page.addInitScript(
        ({ theme, fixture }) => {
          localStorage.setItem("audiorouter.ui.theme", theme);
          Object.assign(window, fixture);
        },
        { theme, fixture: spec.fixture ?? {} },
      );
      await page.setViewportSize({ width: 1500, height: 950 });
      await page.goto("/route-harness.html");
      await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
      await spec.open(page);
      const dialog = spec.dialog(page);
      await expect(dialog).toBeVisible();
      await dialog.screenshot({ path: testInfo.outputPath(`${spec.name}-${theme}.png`) });
      // A modal keeps keyboard focus inside itself, in both directions, however often Tab is pressed.
      const outside = await focusEscapes(page, spec.name);
      const rows = await actionRows(dialog, spec.name);
      const text = await composedTextContrast(page, spec.name, MODAL);
      const edges = await boundaryContrast(page, spec.name, MODAL);
      const focus = await focusRingContrast(page, spec.name, 5);
      const problems = [
        ...(await smallTargets(page, spec.name, MODAL)),
        ...(await clippedText(page, spec.name, MODAL)),
        ...text.failures,
        ...edges.failures,
        ...focus.failures,
        ...rows.problems,
        ...outside,
      ];
      expect(text.judged, "text judged").toBeGreaterThan(3);
      expect(focus.judged, "focus ring judged").toBeGreaterThan(0);
      expect([...new Set(problems)]).toEqual([]);
    });
  }
}

test("the connect form (toolbar) passes the size, clipping, contrast and edge audits in every theme", async ({
  page,
}) => {
  for (const theme of AUDIT_THEMES) {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.setViewportSize({ width: 1500, height: 950 });
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await openConnectionForm(page);
    await expect(page.locator(".workbench-connection-editor")).toBeVisible();
    const label = `Connect form/${theme}`;
    const form = ".workbench-connection-editor";
    const problems = [
      ...(await smallTargets(page, label, form)),
      ...(await clippedText(page, label, form)),
      ...(await composedTextContrast(page, label, form)).failures,
      ...(await boundaryContrast(page, label, form)).failures,
      ...(await focusRingContrast(page, label, 8)).failures,
    ];
    expect([...new Set(problems)]).toEqual([]);
  }
});

test.describe("negative controls", () => {
  test("the button-row audit reports a swapped order, uneven gaps and a short button", async ({ page }) => {
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await page.evaluate(() => {
      const box = document.createElement("div");
      box.setAttribute("role", "dialog");
      box.setAttribute("aria-label", "probe");
      box.style.cssText = "position:fixed;top:0;left:0;z-index:9999;background:#222;padding:8px";
      box.innerHTML = `<div class="actions" style="gap:8px;align-items:flex-start">
          <button class="primary">Confirm</button><button class="secondary" style="margin-left:20px;min-height:60px">Cancel</button><button class="secondary">Later</button>
        </div>`;
      document.body.append(box);
    });
    const { problems, rows } = await actionRows(page.getByRole("dialog", { name: "probe" }), "probe");
    expect(rows).toBe(1);
    expect(problems.some((entry) => entry.includes("confirming button comes before"))).toBe(true);
    expect(problems.some((entry) => entry.includes("unequal gaps"))).toBe(true);
    expect(problems.some((entry) => entry.includes("differ in height"))).toBe(true);
  });

  test("the focus audit reports a modal that lets Tab leave it", async ({ page }) => {
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await page.evaluate(() => {
      const box = document.createElement("section");
      box.setAttribute("role", "dialog");
      box.setAttribute("aria-modal", "true");
      box.setAttribute("aria-label", "leaky");
      box.style.cssText = "position:fixed;top:0;left:0;z-index:9999;background:#222;padding:8px";
      box.innerHTML = "<button id='only'>Only</button>";
      document.body.prepend(box);
      document.getElementById("only")!.focus();
    });
    expect((await focusEscapes(page, "probe")).length).toBeGreaterThan(0);
  });

  test("the dialog audits report a button that is too small", async ({ page }) => {
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await page.evaluate(() => {
      const tiny = document.createElement("button");
      tiny.textContent = "x";
      tiny.setAttribute("aria-label", "tiny");
      tiny.style.cssText = "position:fixed;top:0;left:0;z-index:9999;width:16px;height:16px;padding:0;min-height:0";
      document.body.append(tiny);
    });
    expect((await smallTargets(page, "probe")).some((entry) => entry.includes("tiny"))).toBe(true);
  });
});
