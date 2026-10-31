import { test, expect } from "./browser-test";
import type { Session } from "@audiorouter/contracts";
import { setColorTheme } from "./workbench";

const session: Session = {
  id: "soundboard-flow",
  name: "Soundboard flow regression",
  schemaVersion: 1,
  revision: 1,
  nodes: [
    {
      id: "board",
      kind: "soundboard",
      name: "Soundboard 1",
      parameters: { slot1MediaId: "lib:burp-01", slot1FileName: "Burp 01" },
    },
    { id: "mic", kind: "physicalInput", name: "Microphone", parameters: {} },
    { id: "mix", kind: "mixer", name: "Mixer 4", parameters: {} },
    { id: "cable", kind: "physicalOutput", name: "Cable-A", parameters: {} },
  ].map((node) => ({
    ...node,
    kind: node.kind as Session["nodes"][number]["kind"],
    parameters: node.parameters as Session["nodes"][number]["parameters"],
    typeVersion: 1,
    enabled: true,
    bypass: false,
    ports: [
      ...(node.id === "mix" || node.id === "cable"
        ? [{ name: "in", direction: "input" as const, channels: 2 as const }]
        : []),
      ...(node.id !== "cable" ? [{ name: "out", direction: "output" as const, channels: 2 as const }] : []),
    ],
  })),
  edges: [
    ["board", "mix"],
    ["mic", "mix"],
    ["mix", "cable"],
  ].map(([sourceNode, destinationNode]) => ({
    id: `${sourceNode}-${destinationNode}`,
    sourceNode,
    destinationNode,
    sourcePort: "out",
    destinationPort: "in",
    matrix: [1, 0, 0, 1],
    enabled: true,
  })),
};

for (const theme of ["dark", "light", "high-contrast"]) {
  test(`Soundboard flow follows its source meter without moving the canvas (${theme})`, async ({ page }, testInfo) => {
    await page.addInitScript(
      ({ session, theme }) => {
        localStorage.setItem("audiorouter.ui.theme", theme);
        Object.assign(window, {
          __routeFixtureSession: session,
          __routeFixtureRunning: true,
          __routeFixturePrivacyMuted: false,
          __routeFixtureTelemetry: session.nodes.map((node) => ({
            nodeId: node.id,
            kind: node.kind,
            meter: {
              peakDb: node.id === "board" ? -120 : -12,
              rmsDb: node.id === "board" ? -120 : -18,
              clippedSamples: 0,
              channelPeakDb: [-12, -12],
              channelRmsDb: [-18, -18],
              channelClippedSamples: [0, 0],
            },
            processor: null,
            plugin: null,
          })),
        });
      },
      { session, theme },
    );
    await page.goto("/route-harness.html");
    await setColorTheme(page, theme);
    const board = page.getByTestId("rf__node-board");
    const boardEdge = page.getByTestId("rf__edge-board-mix");
    const cableEdge = page.getByTestId("rf__edge-mix-cable");
    await expect(board).toBeVisible();
    await expect(cableEdge.locator("g.flow-line.flow-edge-active")).toHaveCount(1);
    await expect(boardEdge.locator("g.flow-line.flow-edge-active")).toHaveCount(0);
    const before = await board.boundingBox();
    await page.evaluate(() => {
      const values = (
        window as unknown as { __routeFixtureTelemetry: { nodeId: string; meter: { peakDb: number; rmsDb: number } }[] }
      ).__routeFixtureTelemetry;
      const board = values.find((value) => value.nodeId === "board")!;
      board.meter.peakDb = -18;
      board.meter.rmsDb = -24;
    });
    await expect(boardEdge.locator("g.flow-line.flow-edge-active")).toHaveCount(1);
    expect(await board.boundingBox()).toEqual(before);
    await page.screenshot({ path: testInfo.outputPath(`soundboard-flow-${theme}.png`) });
    await page.evaluate(() => {
      const values = (
        window as unknown as { __routeFixtureTelemetry: { nodeId: string; meter: { peakDb: number; rmsDb: number } }[] }
      ).__routeFixtureTelemetry;
      const board = values.find((value) => value.nodeId === "board")!;
      board.meter.peakDb = -120;
      board.meter.rmsDb = -120;
    });
    await expect(boardEdge.locator("g.flow-line.flow-edge-active")).toHaveCount(0);
    await expect(cableEdge.locator("g.flow-line.flow-edge-active")).toHaveCount(1);
    await board.click();
    await page.getByRole("tab", { name: "Properties", exact: true }).click();
    const editor = page.locator(".soundboard-editor");
    const slider = editor.getByRole("slider");
    await expect(slider).toBeVisible();
    await slider.press("Home");
    await expect(slider).toHaveValue("-24");
    await slider.press("ArrowRight");
    await expect(slider).toHaveValue("-23");
    const slot = editor.locator(".soundboard-slot").first();
    expect(
      await slot.evaluate((element) => {
        const row = element.getBoundingClientRect();
        return [...element.querySelectorAll("input, select, button")].every((control) => {
          const bounds = control.getBoundingClientRect();
          return bounds.width > 0 && bounds.left >= row.left && bounds.right <= row.right;
        });
      }),
    ).toBe(true);
    await editor.screenshot({ path: testInfo.outputPath(`soundboard-properties-${theme}.png`) });
  });
}
