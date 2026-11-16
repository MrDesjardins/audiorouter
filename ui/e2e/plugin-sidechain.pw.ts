import { test, expect } from "./browser-test";
import { fitCanvas, openConnectionForm } from "./workbench";
import type { PluginScanEntry } from "@audiorouter/contracts";
import { demoSession } from "../src/fixtures";
import { appendDraftConnection, appendLibraryNode, appendPluginPlaceholderNode } from "../src/draft";

const entry: PluginScanEntry = {
  path: "C:\\Program Files\\Common Files\\VST3\\sidechain-fixture.vst3",
  identity: {
    path: "C:\\Program Files\\Common Files\\VST3\\sidechain-fixture.vst3",
    binaryPath:
      "C:\\Program Files\\Common Files\\VST3\\sidechain-fixture.vst3\\Contents\\x86_64-win\\sidechain-fixture.vst3",
    format: "vst3",
    architecture: "x64",
    fileBytes: 1024,
    sha256: "a".repeat(64),
    vendor: "AudioRouter test fixture",
    version: "1.0.0",
    classIds: [],
    compatibility: "supportedVst3X64",
    instrument: false,
  },
  error: null,
  errorCode: null,
};

const pluginSession = appendPluginPlaceholderNode(demoSession, entry);
const session = {
  ...pluginSession,
  nodes: pluginSession.nodes.map((node) =>
    node.id === "plugin-1"
      ? { ...node, ports: [...node.ports, { name: "sidechain", direction: "input" as const, channels: 1 as const }] }
      : node,
  ),
};
let replacementSession = appendLibraryNode({ ...session, edges: [] }, "testSignal");
replacementSession = appendDraftConnection(replacementSession, "mic", "out", "plugin-1", "sidechain");
const replacementSource = replacementSession.nodes.find((node) => node.kind === "testSignal")!;

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`named sidechain input is visible and unambiguous in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session });
      },
      { session, theme },
    );
    await page.setViewportSize({ width: 1500, height: 950 });
    await page.goto("/route-harness.html");
    await fitCanvas(page);

    const card = page.getByTestId("rf__node-plugin-1");
    await expect(card.getByText("Sidechain input", { exact: true })).toBeVisible();
    await expect(card.locator('[aria-label$=" sidechain input"]')).toBeVisible();
    await card.screenshot({ path: testInfo.outputPath(`sidechain-card-${theme}.png`) });

    await card.locator(".flow-node-title").click();
    await page.getByRole("tab", { name: "Properties", exact: true }).click();
    const section = page.getByRole("region", { name: "Sidechain input" });
    await expect(section).toContainText("not mixed into the main signal");
    await section.scrollIntoViewIfNeeded();
    await section.screenshot({ path: testInfo.outputPath(`sidechain-properties-${theme}.png`) });
  });
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`VST3 unit hierarchy stays readable in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session });
      },
      { session, theme },
    );
    await page.setViewportSize({ width: 1500, height: 950 });
    await page.goto("/route-harness.html");
    await page.locator('[data-testid="rf__node-plugin-1"] .flow-node-title').click();
    await page.getByRole("tab", { name: "Properties", exact: true }).click();
    const programs = page.getByRole("region", { name: "VST3 programs and presets" });
    await programs.getByRole("button", { name: "Load programs" }).click();
    const units = programs.getByRole("list", { name: "VST3 unit hierarchy" });
    await expect(units).toContainText("Root unit");
    await expect(units).toContainText("Within Root unit");
    await expect(units).toContainText("Program list · 2 programs");
    await expect(units).toContainText("Programs: Attack, Smooth");
    await programs.screenshot({ path: testInfo.outputPath(`vst3-units-${theme}.png`) });
  });
}

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`replacing an occupied sidechain is an explicit choice in ${theme}`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, { __routeFixtureSession: session });
      },
      { session: replacementSession, theme },
    );
    await page.setViewportSize({ width: 1500, height: 950 });
    await page.goto("/route-harness.html");
    const originalEdgeCount = replacementSession.edges.length;
    const edges = page.locator(".session-flow-canvas .react-flow__edge");
    await expect(edges).toHaveCount(originalEdgeCount);

    const connect = async () => {
      await openConnectionForm(page);
      const form = page.locator(".workbench-connection-editor");
      const source = form.getByRole("combobox", { name: "Source output port" });
      const destination = form.getByRole("combobox", { name: "Destination input port" });
      const sourceOption = source.locator("option").filter({ hasText: replacementSource.name }).last();
      const inputOption = destination.locator("option").filter({ hasText: "sidechain" }).last();
      const sourceValue = await sourceOption.getAttribute("value");
      const inputValue = await inputOption.getAttribute("value");
      if (!sourceValue || !inputValue) throw new Error("Sidechain replacement ports are not listed");
      await source.selectOption(sourceValue);
      await destination.selectOption(inputValue);
      await form.getByRole("button", { name: "Add connection" }).click();
    };

    await connect();
    const decision = page.getByRole("dialog", { name: "Replace this sidechain source?" });
    await expect(decision).toContainText("will not add a Mixer");
    await decision.screenshot({ path: testInfo.outputPath(`sidechain-replace-${theme}.png`) });
    await decision.getByRole("button", { name: "Keep current source" }).click();
    await expect(edges).toHaveCount(originalEdgeCount);
    await expect(page.locator(".react-flow__node").filter({ hasText: "Mixer" })).toHaveCount(0);

    await connect();
    await page
      .getByRole("dialog", { name: "Replace this sidechain source?" })
      .getByRole("button", { name: "Replace sidechain source" })
      .click();
    await expect(page.getByText(/Replaced the Sidechain input source/)).toBeVisible();
    await expect(edges).toHaveCount(originalEdgeCount);
    await expect(page.locator(".react-flow__node").filter({ hasText: "Mixer" })).toHaveCount(0);
  });
}
