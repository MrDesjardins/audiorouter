import { describe, it, expect } from "vitest";
import { demoSession } from "./fixtures";
import { smartLayout } from "./smartLayout";
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
});
