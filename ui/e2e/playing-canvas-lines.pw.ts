import { test, expect, type Page } from "@playwright/test";
import type { Session } from "../../contracts/src/index";

// A two-path route shaped like a real streaming setup: a mono microphone
// through a stereo FIR Filter Hz and mono plugins to two outputs, and a
// stereo game source through an EQ. Reported 2026-09-27: lines looked gone
// while playing and could not be dragged back.
const port = (name: string, direction: "input" | "output", channels: number) => ({ name, direction, channels });
const node = (
  id: string,
  kind: Session["nodes"][number]["kind"],
  name: string,
  ports: ReturnType<typeof port>[],
  parameters: Session["nodes"][number]["parameters"] = {},
): Session["nodes"][number] => ({ id, kind, typeVersion: 1, name, enabled: true, bypass: false, parameters, ports });
const plugin = (id: string, name: string) =>
  node(id, "plugin", name, [port("in", "input", 1), port("out", "output", 1)], {
    format: "vst2",
    classId: "default",
    path: `C:\\Plugins\\${id}.dll`,
    fingerprint: "0".repeat(64),
  });
const edge = (id: string, from: string, to: string, matrix: number[]) => ({
  id,
  sourceNode: from,
  sourcePort: "out",
  destinationNode: to,
  destinationPort: "in",
  matrix,
  enabled: true,
});
const session: Session = {
  id: "streaming-route",
  name: "Streaming route",
  schemaVersion: 1,
  revision: 3,
  nodes: [
    node("mic", "physicalInput", "Microphone", [port("out", "output", 1)]),
    node("gate-hz", "spectralGate", "FIR Filter Hz", [port("in", "input", 2), port("out", "output", 2)], {
      learning: false,
      reductionDb: 40,
      thresholdDb: 3,
    }),
    plugin("reaeq", "ReaEQ"),
    plugin("reacomp", "ReaComp"),
    node("cable", "physicalOutput", "Voice to CABLE-A", [port("in", "input", 2)]),
    node("monitor", "physicalOutput", "Hear my voice", [port("in", "input", 2)]),
    node("game", "physicalInput", "Game (CABLE-B Output)", [port("out", "output", 2)]),
    node("game-eq", "parametricEq", "Game EQ", [port("in", "input", 2), port("out", "output", 2)]),
    node("game-out", "physicalOutput", "Game to headphones", [port("in", "input", 2)]),
  ],
  edges: [
    edge("edge-1", "mic", "gate-hz", [1, 1]),
    edge("edge-2", "gate-hz", "reaeq", [0.5, 0.5]),
    edge("reaeq-reacomp", "reaeq", "reacomp", [1]),
    edge("reacomp-cable", "reacomp", "cable", [1, 1]),
    edge("reacomp-monitor", "reacomp", "monitor", [1, 1]),
    edge("game-in-eq", "game", "game-eq", [1, 0, 0, 1]),
    edge("game-eq-out", "game-eq", "game-out", [1, 0, 0, 1]),
  ],
};
const telemetry = [
  { nodeId: "mic", kind: "physical-input", meter: null, processor: null, plugin: null, timing: { delayMs: 15 } },
  {
    nodeId: "gate-hz",
    kind: "spectral-gate",
    meter: null,
    processor: null,
    plugin: null,
    timing: { delayMs: 21.3 },
    spectrum: {
      levelsDb: Array(64).fill(-12),
      bandFrequenciesHz: Array.from({ length: 64 }, (_, band) => 20 * 1000 ** (band / 63)),
    },
  },
  { nodeId: "reaeq", kind: "plugin", meter: null, processor: null, plugin: { state: "running", failureCount: 0 } },
  { nodeId: "game", kind: "physical-input", meter: null, processor: null, plugin: null, timing: { delayMs: 21 } },
];

async function openPlaying(page: Page) {
  await page.addInitScript(
    ([sessionJson, telemetryJson]) => {
      Object.assign(window, {
        __routeFixtureSession: JSON.parse(sessionJson),
        __routeFixtureTelemetry: JSON.parse(telemetryJson),
        __routeFixtureRunning: true,
      });
    },
    [JSON.stringify(session), JSON.stringify(telemetry)],
  );
  await page.goto("/route-harness.html");
  await expect(page.locator(".audio-run-state")).toContainText("Audio running");
}

test("every line of a two-path route is drawn while playing", async ({ page }) => {
  await openPlaying(page);
  for (const { id } of session.edges) {
    await expect(page.getByTestId(`rf__edge-${id}`).locator("path").first()).toHaveAttribute("d", /^M-?\d/);
  }
});

test("a removed line can be dragged back while playing", async ({ page }) => {
  await openPlaying(page);
  await page.getByRole("button", { name: "Remove connection game-in-eq" }).click({ force: true });
  await expect(page.getByTestId("rf__edge-game-in-eq")).toHaveCount(0);
  const from = await page.getByTestId("rf__node-game").locator(".react-flow__handle.source").nth(1).boundingBox();
  const to = await page.getByTestId("rf__node-game-eq").locator(".react-flow__handle.target").nth(3).boundingBox();
  await page.mouse.move(from!.x + from!.width / 2, from!.y + from!.height / 2);
  await page.mouse.down();
  await page.mouse.move((from!.x + to!.x) / 2, (from!.y + to!.y) / 2, { steps: 8 });
  await page.mouse.move(to!.x + to!.width / 2, to!.y + to!.height / 2, { steps: 8 });
  await page.mouse.up();
  await expect(page.locator(".global-action-message")).toContainText("Connection added to the draft");
  await expect(page.locator('[data-testid^="rf__edge-"]')).toHaveCount(7);
});
