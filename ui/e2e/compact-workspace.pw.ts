import { expect, test } from "./browser-test";
import { demoSession } from "../src/fixtures";
import { appendDraftConnection } from "../src/draft";

const route = appendDraftConnection(
  appendDraftConnection(demoSession, "mic", "out", "voice", "in"),
  "voice",
  "out",
  "headphones",
  "in",
);

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`popover, inline rename and manual Save stay clear in ${theme}`, async ({ page }, info) => {
    await page.addInitScript(
      ({ route, theme }) => {
        Object.assign(window, { __routeFixtureSession: route });
        localStorage.setItem("audiorouter.ui.theme", theme);
      },
      { route, theme },
    );
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
    const canvas = page.getByLabel("Signal-flow graph");
    const initial = (await canvas.boundingBox())!;
    const selection = (await page.locator(".canvas-layout-actions").boundingBox())!;
    const heading = (await page.getByRole("heading", { name: "Canvas", exact: true }).boundingBox())!;
    expect(selection.y - heading.y - heading.height).toBeLessThan(12);
    await expect(page.getByText("No unsaved changes to this node.", { exact: true })).toHaveCount(0);
    await expect(page.getByLabel("Node name", { exact: true })).toHaveCount(0);
    const rename = page.getByRole("button", { name: "Rename Voice gain", exact: true });
    await rename.hover();
    await expect(rename.locator("svg")).toHaveCSS("opacity", "1");
    await rename.click();
    const field = page.getByLabel("Node name", { exact: true });
    await expect(field).toHaveCSS("border-radius", "9px");
    await field.fill("Discard this name");
    await field.press("Escape");
    await expect(rename).toBeFocused();
    const save = page.locator(".topbar").getByRole("button", { name: "Save", exact: true });
    await expect(save).toBeDisabled();
    await rename.click();
    await field.fill("Radio voice");
    await field.press("Enter");
    await expect(page.getByRole("button", { name: "Rename Radio voice", exact: true })).toBeVisible();
    await expect(save).toBeEnabled();
    await expect(save).toHaveAttribute("data-save-required", "true");
    const node = page.getByTestId("rf__node-voice");
    await expect(node.locator(".node-save-indicator")).toHaveText("Save required");
    const notice = page.locator(".global-action-message");
    await expect(notice).toContainText("Name changed");
    const popover = page.getByRole("complementary", { name: "Notifications" });
    await expect(popover).toHaveCSS("position", "fixed");
    const box = (await popover.boundingBox())!;
    expect(box.x + box.width).toBeLessThanOrEqual(1280);
    expect(box.y + box.height).toBeLessThanOrEqual(720);
    expect((await canvas.boundingBox())!).toEqual(initial);
    await page.screenshot({ path: info.outputPath(`${theme}-save-required.png`) });
    await page.getByRole("button", { name: "Dismiss message", exact: true }).click();
    expect((await canvas.boundingBox())!).toEqual(initial);
    await save.click();
    await expect(notice).toContainText("Route saved");
    await expect(save).toBeDisabled();
    await expect(node.locator(".node-save-indicator")).toHaveCount(0);
    expect((await canvas.boundingBox())!).toEqual(initial);
    await page.screenshot({ path: info.outputPath(`${theme}-saved.png`) });
  });

  test(`debounced live autosave and explicit name save in ${theme}`, async ({ page }, info) => {
    await page.addInitScript(
      ({ route, theme }) => {
        Object.assign(window, { __routeFixtureSession: route, __routeFixtureRunning: true });
        localStorage.setItem("audiorouter.ui.theme", theme);
      },
      { route, theme },
    );
    await page.goto("/route-harness.html");
    await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
    const canvas = page.getByLabel("Signal-flow graph");
    const before = (await canvas.boundingBox())!;
    // The autosave state lasts a 400 ms debounce. A stalled hosted runner
    // missed it while polling (2026-10-23), so record every state the node
    // shows instead of polling for a passing one.
    await page.evaluate(() => {
      const seen: string[] = [];
      Object.assign(window, { __voiceSaveStates: seen });
      const record = () => {
        const content = document.querySelector('[data-testid="rf__node-voice"] .flow-node-content');
        const state = content?.getAttribute("data-save-state");
        const label = content?.querySelector(".node-save-indicator")?.textContent;
        if (state) seen.push(`state:${state}`);
        if (label) seen.push(`label:${label}`);
      };
      new MutationObserver(record).observe(document.body, {
        subtree: true,
        attributes: true,
        childList: true,
        characterData: true,
      });
    });
    const planCalls = () =>
      page.evaluate(() => (window as unknown as { __routeFixturePlanCalls(): number }).__routeFixturePlanCalls());
    await page.getByRole("group", { name: "Gain choices" }).getByRole("button", { name: "−6 dB", exact: true }).click();
    const content = page.getByTestId("rf__node-voice").locator(".flow-node-content");
    await expect.poll(planCalls).toBe(1);
    await expect(content.locator(".node-save-indicator")).toHaveCount(0);
    const seen = await page.evaluate(() => (window as unknown as { __voiceSaveStates: string[] }).__voiceSaveStates);
    expect(seen).toContain("state:autosave");
    expect(seen).toContain("label:Autosaving…");
    const save = page.locator(".topbar").getByRole("button", { name: "Save", exact: true });
    await expect(save).toBeDisabled();
    expect((await canvas.boundingBox())!).toEqual(before);
    await page.getByRole("button", { name: "Rename Voice gain", exact: true }).click();
    await page.getByLabel("Node name", { exact: true }).fill("Live voice");
    await page.getByLabel("Node name", { exact: true }).press("Enter");
    await page.waitForTimeout(700); // A rename must not take the parameter-only autosave path.
    await expect(content).toHaveAttribute("data-save-state", "needs-save");
    await expect(save).toBeEnabled();
    expect(
      await page.evaluate(() => (window as unknown as { __routeFixturePlanCalls(): number }).__routeFixturePlanCalls()),
    ).toBe(1);
    await page.screenshot({ path: info.outputPath(`${theme}-live-name-draft.png`) });
  });

  test(`every workbench tab shares question-mark help in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((theme) => localStorage.setItem("audiorouter.ui.theme", theme), theme);
    await page.goto("/route-harness.html");
    for (const tab of [
      "Tools",
      "Properties",
      "Timing",
      "Session",
      "Status",
      "Virtual Cables",
      "UI Options",
      "Recording",
      "Advanced",
      "MCP",
      "API",
      "Logs",
    ]) {
      await page.getByRole("tab", { name: tab, exact: true }).click();
      const help = page.locator(".help-disclosure:visible");
      await expect(help).toHaveCount(1);
      await expect(help.locator(".help-icon")).toHaveText("?");
      await expect(help).not.toHaveAttribute("open");
      await help.locator("summary").focus();
      await page.keyboard.press("Enter");
      await expect(help.locator(".help-content")).toBeVisible();
      await page.keyboard.press("Enter");
      await expect(help).not.toHaveAttribute("open");
    }
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    await page.getByText("How to build a route", { exact: true }).click();
    await page.screenshot({ path: info.outputPath(`${theme}-route-help.png`) });
  });
}
