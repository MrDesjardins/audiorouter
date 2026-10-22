import { expect, it } from "vitest";
import { readLayout, readNodeSizes, readEdgeLayout, writeLayout, writeNodeSizes, writeEdgeLayout } from "./layout";

it("rejects malformed fields and roundtrips canonical records through a fixed-seed corpus", () => {
  let rng = 997;
  const random = (limit: number) => {
    rng ^= rng << 13;
    rng ^= rng >>> 17;
    rng ^= rng << 5;
    return (rng >>> 0) % limit;
  };
  const values = [
    null,
    false,
    "400",
    [],
    {},
    -100001,
    -1,
    0,
    99,
    100,
    150,
    218,
    400,
    10000,
    10001,
    100000,
    Infinity,
    NaN,
  ];
  const pick = () => values[random(values.length)];
  const sides = ["left", "right", "top", "bottom", "outside", null, [], 3];
  for (let index = 0; index < 300; index++) {
    const raw = JSON.stringify({
      position: { x: pick(), y: pick(), id: "forged", group: true },
      size: { width: pick(), height: pick(), minWidth: pick(), minHeight: pick(), foreign: "drop" },
      side: { source: sides[random(sides.length)], target: sides[random(sides.length)], foreign: "drop" },
      scalar: pick(),
    });
    let saved: string | null = raw;
    const storage = {
      getItem: () => saved,
      setItem: (_key: string, value: string) => {
        saved = value;
      },
    };
    const positions = readLayout(storage, "key"),
      sizes = readNodeSizes(storage, "key"),
      edges = readEdgeLayout(storage, "key");
    const evidence = `seed=997, index=${index}, raw=${raw}`;
    for (const position of Object.values(positions)) {
      expect(Object.keys(position).sort(), evidence).toEqual(["x", "y"]);
      expect(Number.isFinite(position.x) && Number.isFinite(position.y), evidence).toBe(true);
      expect(Math.max(Math.abs(position.x), Math.abs(position.y)), evidence).toBeLessThanOrEqual(100000);
    }
    for (const size of Object.values(sizes)) {
      expect(Object.keys(size).sort(), evidence).toEqual(["height", "minHeight", "minWidth", "width"]);
      expect(size.width >= size.minWidth && size.height >= size.minHeight, evidence).toBe(true);
      expect(Math.min(...Object.values(size)), evidence).toBeGreaterThanOrEqual(100);
      expect(Math.max(...Object.values(size)), evidence).toBeLessThanOrEqual(10000);
    }
    for (const edge of Object.values(edges)) {
      expect(Object.keys(edge).sort(), evidence).toEqual(["source", "target"]);
      expect(["left", "right", "top", "bottom"], evidence).toContain(edge.source);
      expect(["left", "right", "top", "bottom"], evidence).toContain(edge.target);
    }
    writeLayout(storage, "key", positions);
    expect(readLayout(storage, "key"), evidence).toEqual(positions);
    writeNodeSizes(storage, "key", sizes);
    expect(readNodeSizes(storage, "key"), evidence).toEqual(sizes);
    writeEdgeLayout(storage, "key", edges);
    expect(readEdgeLayout(storage, "key"), evidence).toEqual(edges);
  }
});
