import { expect, it } from "vitest";
import { canvasOwners, moveCanvasItems, type CanvasRect } from "./canvasGeometry";
import { qualitySeeds } from "./qualitySeeds";

for (const seed of qualitySeeds([47, 997, 1234567]))
  it(`preserves nested geometry across bounded generated gestures, seed ${seed}`, () => {
    let rng = seed;
    const random = (limit: number) => {
      rng ^= rng << 13;
      rng ^= rng >>> 17;
      rng ^= rng << 5;
      return (rng >>> 0) % limit;
    };
    for (let sample = 0; sample < 200; sample++) {
      const x = random(198000) - 99000,
        y = random(198000) - 99000;
      const locked = random(4) === 0;
      const items: CanvasRect[] = [
        { id: "outer", x, y, width: 600, height: 500, group: true, carry: true },
        { id: "inner", x: x + 30, y: y + 30, width: 300, height: 200, group: true, carry: false, locked },
        { id: "tool", x: x + 60, y: y + 60, width: 80, height: 70 },
        { id: "partial", x: x + 580, y: y + 100, width: 100, height: 80 },
      ];
      const original = structuredClone(items);
      const owners = canvasOwners(items);
      expect([...owners.entries()].sort()).toEqual([
        ["inner", "outer"],
        ["tool", "inner"],
      ]);
      expect([...canvasOwners([...items].reverse()).entries()].sort()).toEqual([...owners.entries()].sort());
      const targets = new Map([
        ["outer", { x: random(240000) - 120000, y: random(240000) - 120000 }],
        ["inner", { x: 99999, y: 99999 }],
        ["tool", { x: -99999, y: -99999 }],
      ]);
      const next = moveCanvasItems(items, targets),
        dx = next[0].x - x,
        dy = next[0].y - y;
      const evidence = `seed=${seed}, sample=${sample}, targets=${JSON.stringify([...targets])}, locked=${locked}`;
      if (locked) expect(next, evidence).toEqual(items);
      for (let index = 0; index < 3; index++) {
        expect(next[index], evidence).toEqual({ ...items[index], x: items[index].x + dx, y: items[index].y + dy });
        expect(Math.abs(next[index].x), evidence).toBeLessThanOrEqual(100000);
        expect(Math.abs(next[index].y), evidence).toBeLessThanOrEqual(100000);
      }
      expect(next[3], evidence).toEqual(items[3]);
      expect(items, evidence).toEqual(original);
    }
  });
