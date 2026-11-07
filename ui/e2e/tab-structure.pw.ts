import { expect, test } from "./browser-test";
import { backToRoute, openSettings, openStatusView } from "./workbench";

const SETTINGS_PAGES: [string, string][] = [
  ["Virtual Cables", "Virtual Cables"],
  ["Plug-ins", "Plug-ins"],
  ["Device access", "Device access"],
  ["Appearance", "Appearance"],
  ["When AudioRouter starts", "When AudioRouter starts"],
  ["Keyboard shortcuts", "Keyboard shortcuts"],
  ["New versions", "New versions"],
  ["Help", "Help"],
  ["AI assistants (MCP)", "MCP connections"],
  ["API", "API"],
  ["Logs and support", "Client diagnostics"],
  ["Connected clients", "Connected clients"],
  ["Session as JSON", "Session as JSON"],
  ["Tool catalog", "Tool catalog"],
  ["App identity", "App identity"],
];

test("five daily tabs, a gear, and no Advanced tab", async ({ page }) => {
  await page.goto("/route-harness.html");
  const tabs = page.getByRole("tablist", { name: "Right sidebar" }).getByRole("tab");
  await expect(tabs).toHaveText(["Tools", "Properties", "Status", "Session", "Recording"]);
  await expect(page.getByRole("button", { name: "Settings", exact: true })).toBeVisible();
});

test("every Settings page opens, with the top bar still there", async ({ page }) => {
  await page.goto("/route-harness.html");
  for (const [name, heading] of SETTINGS_PAGES) {
    await openSettings(page, name);
    await expect(page.getByRole("tabpanel").getByRole("heading", { name: heading }).first()).toBeVisible();
    await expect(page.locator(".topbar").getByRole("button", { name: "Play", exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(1440);
  }
});

test("Back to the route restores the tab, the selection and the canvas size", async ({ page }) => {
  await page.goto("/route-harness.html");
  await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
  await page.getByTestId("rf__node-voice").locator(".flow-node-title").click();
  await expect(page.getByRole("tab", { name: "Properties", exact: true })).toHaveAttribute("aria-selected", "true");
  const canvas = page.locator("#signal-flow-panel");
  const before = await canvas.boundingBox();
  await openSettings(page, "API");
  await expect(page.getByRole("tabpanel", { name: "API" })).toBeVisible();
  await expect(canvas).toBeHidden();
  expect(await canvas.boundingBox()).toEqual(before);
  await backToRoute(page);
  await expect(page.getByRole("tab", { name: "Properties", exact: true })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator(".main-content > .inspector").getByRole("heading", { name: "Voice gain" })).toBeVisible();
  expect(await canvas.boundingBox()).toEqual(before);
  // The gear goes back to the page last used.
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect(page.getByRole("tab", { name: "API", exact: true })).toHaveAttribute("aria-selected", "true");
});

test("Settings pages can be walked with the arrow keys", async ({ page }) => {
  await page.goto("/route-harness.html");
  await openSettings(page, "Virtual Cables");
  const first = page.getByRole("tab", { name: "Virtual Cables", exact: true });
  await first.focus();
  await page.keyboard.press("ArrowDown");
  await expect(page.getByRole("tab", { name: "Plug-ins", exact: true })).toBeFocused();
  await expect(page.getByRole("tab", { name: "Plug-ins", exact: true })).toHaveAttribute("aria-selected", "true");
  await page.keyboard.press("ArrowUp");
  await expect(first).toBeFocused();
});

test("Status holds Readiness, Apps, Timing and the recovery tools", async ({ page }) => {
  await page.goto("/route-harness.html");
  await openStatusView(page, "Apps");
  await expect(page.getByRole("tab", { name: "Status", exact: true })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByRole("tabpanel", { name: "Apps" })).toBeVisible();
  await openStatusView(page, "Timing");
  await expect(page.getByRole("heading", { name: "Signal timing" })).toBeVisible();
  await openStatusView(page, "Readiness");
  await page.locator("details.troubleshoot-group > summary").click();
  await expect(page.getByRole("heading", { name: "Crash recovery" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Recovery checkpoints" })).toBeVisible();
});

test("Session holds saved versions; the canvas toolbar holds the keyboard panels", async ({ page }) => {
  await page.goto("/route-harness.html");
  await page.getByRole("tab", { name: "Session", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Change history" })).toBeVisible();
  const toolbar = page.getByRole("group", { name: "Canvas layout actions" });
  await toolbar.getByRole("button", { name: "Route list", exact: true }).click();
  await expect(page.getByRole("region", { name: "Route list" }).getByLabel("Tools and connections")).toBeVisible();
  await toolbar.getByRole("button", { name: "Connect", exact: true }).click();
  const connect = page.getByRole("region", { name: "Connect without dragging" });
  await expect(connect.getByLabel("Source output port")).toBeVisible();
  await connect.getByLabel("Source output port").focus();
  await page.keyboard.press("Escape");
  await expect(connect).toHaveCount(0);
});

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`tab structure screenshots in ${theme}`, async ({ page }, info) => {
    await page.addInitScript((value) => localStorage.setItem("audiorouter.ui.theme", value), theme);
    await page.goto("/route-harness.html");
    await expect(page.getByTestId("canvas-readiness")).toHaveAttribute("data-ready", "true");
    await page.screenshot({ path: info.outputPath(`${theme}-tools.png`) });
    await openStatusView(page, "Readiness");
    await page.screenshot({ path: info.outputPath(`${theme}-status.png`) });
    await openSettings(page, "AI assistants (MCP)");
    await expect(page.getByRole("heading", { name: "MCP connections" })).toBeVisible();
    await page.screenshot({ path: info.outputPath(`${theme}-settings.png`) });
    await backToRoute(page);
    await page.getByRole("button", { name: "Route list", exact: true }).click();
    await expect(page.getByRole("region", { name: "Route list" })).toBeVisible();
    await page.screenshot({ path: info.outputPath(`${theme}-route-list.png`) });
    await page.setViewportSize({ width: 640, height: 720 });
    await openSettings(page, "Appearance");
    await page.screenshot({ path: info.outputPath(`${theme}-narrow-settings.png`) });
  });
}
