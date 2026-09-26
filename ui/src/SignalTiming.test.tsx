/** @vitest-environment jsdom */
import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import type { DiagnosticsSnapshot, Session } from "@audiorouter/contracts";
import { SignalTimingPanel, signalTimingRoutes } from "./SignalTiming";
import { demoSession } from "./fixtures";

afterEach(cleanup);

const node = (id: string, kind: Session["nodes"][number]["kind"]): Session["nodes"][number] => ({
  id, kind, typeVersion: 1, name: id, enabled: true, bypass: false, parameters: {}, ports: [],
});
const edge = (id: string, sourceNode: string, destinationNode: string): Session["edges"][number] => ({
  id, sourceNode, sourcePort: "out", destinationNode, destinationPort: "in", matrix: [1], enabled: true,
});
const session: Session = {
  ...demoSession,
  nodes: [node("mic", "physicalInput"), node("ReaFIR", "plugin"), node("ReaGate", "plugin"), node("cable-a", "physicalOutput"), node("game", "physicalInput"), node("eq", "parametricEq"), node("scarlett", "physicalOutput")],
  edges: [edge("e1", "mic", "ReaFIR"), edge("e2", "ReaFIR", "ReaGate"), edge("e3", "ReaGate", "cable-a"), edge("e4", "game", "eq"), edge("e5", "eq", "scarlett")],
};
const timed = (nodeId: string, delayMs: number, processingUsAvg?: number): DiagnosticsSnapshot["nodeTelemetry"][number] => ({
  nodeId, kind: "x", meter: null, processor: null, plugin: null, timing: { delayMs, processingUsAvg },
});

describe("signal timing", () => {
  it("lists each output's steps in travel order and finds the slowest one", () => {
    const routes = signalTimingRoutes(session, [timed("mic", 8), timed("ReaFIR", 21.3, 4), timed("ReaGate", 2.7, 3), timed("cable-a", 12), timed("eq", 0, 9)]);
    expect(routes.map((route) => route.output.id)).toEqual(["cable-a", "scarlett"]);
    const voice = routes[0];
    expect(voice.steps.map((step) => step.node.id)).toEqual(["mic", "ReaFIR", "ReaGate", "cable-a"]);
    expect(voice.totalMs).toBeCloseTo(44);
    expect(voice.slowest?.node.id).toBe("ReaFIR");
    expect(routes[1].steps.map((step) => step.delayMs)).toEqual([null, 0, null]);
  });

  it("follows the slowest Mixer input", () => {
    const mixed: Session = {
      ...session,
      nodes: [...session.nodes, node("mixer", "mixer"), node("out", "physicalOutput")],
      edges: [...session.edges, edge("m1", "ReaGate", "mixer"), edge("m2", "eq", "mixer"), edge("m3", "mixer", "out")],
    };
    const route = signalTimingRoutes(mixed, [timed("mic", 5), timed("ReaFIR", 20), timed("game", 1), timed("eq", 1)]).find((item) => item.output.id === "out");
    expect(route?.steps.map((step) => step.node.id)).toEqual(["mic", "ReaFIR", "ReaGate", "mixer", "out"]);
  });

  it("shows the total and marks the slowest step", () => {
    render(<SignalTimingPanel session={session} telemetry={[timed("mic", 8), timed("ReaFIR", 21.3), timed("cable-a", 12)]} running />);
    const voice = screen.getByRole("article", { name: "Timing to cable-a" });
    expect(within(voice).getByText("41 ms")).toBeTruthy();
    expect(within(voice).getByText("slowest").closest("li")?.textContent).toContain("ReaFIR");
    expect(within(screen.getByRole("article", { name: "Timing to scarlett" })).getByText("not measured")).toBeTruthy();
  });
});
