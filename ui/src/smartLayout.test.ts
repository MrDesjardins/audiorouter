import { describe, it, expect } from "vitest";
import type { NodeKind, Session } from "@audiorouter/contracts";
import { demoSession } from "./fixtures";
import { LAYOUT_COLUMN_GAP, smartLayout } from "./smartLayout";

const graph = (nodes: Array<[string, NodeKind]>, edges: Array<[string, string]>): Session => ({
  ...demoSession,
  nodes: nodes.map(([id, kind]) => ({ ...demoSession.nodes[0], id, kind, name: id })),
  edges: edges.map(([sourceNode, destinationNode], index) => ({ ...demoSession.edges[0] ?? { sourcePort: "out", destinationPort: "in", matrix: [1], enabled: true }, id: `e${index}`, sourceNode, destinationNode, sourcePort: "out", destinationPort: "in", matrix: [1], enabled: true })),
});

describe("smart graph layout", () => {
  it("lays out a reverse-insertion fan-in/fan-out from connections without mutating graph", () => {
    const nodes = ["out2", "out1", "mix", "b", "a"].map(id => ({ ...demoSession.nodes[0], id }));
    const edges = [["a", "mix"], ["b", "mix"], ["mix", "out1"], ["mix", "out2"]].map(([sourceNode, destinationNode], index) => ({ ...demoSession.edges[0], id: String(index), sourceNode, destinationNode, enabled: false }));
    const s = { ...demoSession, nodes, edges }; const before = JSON.stringify(s);
    const layout = smartLayout(s, { a: { width: 400, height: 300 } });
    expect(layout.a.x).toBe(layout.b.x); expect(layout.mix.x).toBeGreaterThan(layout.a.x + 400);
    expect(layout.out1.x).toBeGreaterThan(layout.mix.x); expect(layout.out1.x).toBe(layout.out2.x);
    expect(Math.abs(layout.a.y - layout.b.y)).toBeGreaterThanOrEqual(250);
    expect(layout).toEqual(smartLayout({ ...s, nodes: [...nodes].reverse(), edges: [...edges].reverse() }, { a: { width: 400, height: 300 } }));
    expect(JSON.stringify(s)).toBe(before);
  });

  it("separates disconnected components and handles cyclic drafts deterministically", () => {
    const s = { ...demoSession, edges: [] };
    const layout = smartLayout(s);
    expect(new Set(Object.values(layout).map(p => p.y)).size).toBe(s.nodes.length);
    expect(smartLayout({ ...demoSession, edges: [{ ...demoSession.edges[0], sourceNode: "mic", destinationNode: "voice" }, { ...demoSession.edges[0], sourceNode: "voice", destinationNode: "mic" }] })).toEqual(smartLayout({ ...demoSession, edges: [{ ...demoSession.edges[0], sourceNode: "mic", destinationNode: "voice" }, { ...demoSession.edges[0], sourceNode: "voice", destinationNode: "mic" }] }));
  });

  it("puts every input on the left and every output on the right, across paths", () => {
    const layout = smartLayout(graph(
      [["mic", "physicalInput"], ["gate", "gate"], ["comp", "compressor"], ["cable", "physicalOutput"], ["monitor", "physicalOutput"], ["game", "testSignal"], ["speakers", "physicalOutput"]],
      [["mic", "gate"], ["gate", "comp"], ["comp", "cable"], ["mic", "monitor"], ["game", "speakers"]],
    ));
    expect(layout.mic.x).toBe(0);
    expect(layout.game.x).toBe(0);
    expect(layout.monitor.x).toBe(layout.cable.x);
    expect(layout.speakers.x).toBe(layout.cable.x);
    expect(layout.gate.x).toBeLessThan(layout.comp.x);
    expect(layout.comp.x).toBeLessThan(layout.cable.x);
    // About one node of space between columns.
    expect(layout.gate.x - layout.mic.x).toBeGreaterThanOrEqual(218 + LAYOUT_COLUMN_GAP);
  });

  it("keeps a simple chain level and avoids crossing connections", () => {
    const chain = smartLayout(graph([["mic", "physicalInput"], ["gate", "gate"], ["comp", "compressor"], ["out", "physicalOutput"]], [["mic", "gate"], ["gate", "comp"], ["comp", "out"]]));
    expect(new Set(Object.values(chain).map((point) => point.y)).size).toBe(1);
    // Two sources, two tools: the order on the right follows the left.
    const crossed = smartLayout(graph([["a", "physicalInput"], ["b", "physicalInput"], ["x", "gain"], ["y", "gain"]], [["a", "y"], ["b", "x"]]));
    expect(Math.sign(crossed.a.y - crossed.b.y)).toBe(Math.sign(crossed.y.y - crossed.x.y));
  });

  it("reserves room where a long connection crosses a column", () => {
    const layout = smartLayout(graph(
      [["mic", "physicalInput"], ["gate", "gate"], ["comp", "compressor"], ["cable", "physicalOutput"], ["monitor", "physicalOutput"]],
      [["mic", "gate"], ["gate", "comp"], ["comp", "cable"], ["mic", "monitor"]],
    ));
    // The direct mic → monitor line passes the gate and compressor columns
    // without running through either card.
    const lineY = (layout.mic.y + layout.monitor.y) / 2 + 90;
    for (const id of ["gate", "comp"]) {
      const box = layout[id];
      expect(lineY < box.y || lineY > box.y + 180).toBe(true);
    }
  });
});
