import { expect, it } from "vitest";
import { qualitySeeds } from "./qualitySeeds";
import {
  arrangedCanvasSnapshot,
  canvasLayoutChanged,
  copyCanvasSnapshot,
  emptyCanvasHistory,
  recordCanvasHistory,
  restoreCanvasHistory,
  type CanvasSnapshot,
} from "./canvasHistory";

const initial = (): CanvasSnapshot => ({
  sessionId: "one",
  items: [
    { id: "group", x: 0, y: 0, width: 600, height: 400, group: true, carry: false },
    { id: "tool", x: 800, y: 30, width: 220, height: 180 },
  ],
  sizes: {},
  sides: {},
  edgeIds: ["edge"],
});
const moved = (state: CanvasSnapshot, x: number): CanvasSnapshot => ({
  ...state,
  items: state.items.map((item) => (item.id === "tool" ? { ...item, x } : item)),
});

it("records immutable geometry, sizes and sides without borrowing caller arrays", () => {
  const before = initial();
  before.sizes.tool = { width: 220, height: 180, minWidth: 218, minHeight: 150 };
  before.sides.edge = { source: "bottom", target: "top" };
  const next = moved(before, 900);
  const history = recordCanvasHistory(emptyCanvasHistory("one"), before, next);
  before.items[1].x = 123;
  before.sizes.tool.width = 999;
  before.sides.edge.source = "left";
  before.edgeIds.push("new");
  expect(history.past[0].items[1].x).toBe(800);
  expect(history.past[0].sizes.tool.width).toBe(220);
  expect(history.past[0].sides.edge.source).toBe("bottom");
  expect(history.past[0].edgeIds).toEqual(["edge"]);
});

it("ignores no-op commands and implicit default connector sides without clearing redo", () => {
  const before = initial(),
    next = moved(before, 900);
  const recorded = recordCanvasHistory(emptyCanvasHistory("one"), before, next);
  const undone = restoreCanvasHistory(recorded, next, "undo");
  const noOp = { ...before, sides: { edge: { source: "right", target: "left" } } } as CanvasSnapshot;
  expect(canvasLayoutChanged(before, noOp)).toBe(false);
  expect(recordCanvasHistory(undone.history, before, noOp)).toBe(undone.history);
  expect(recordCanvasHistory(undone.history, before, moved(before, 950)).future).toEqual([]);
});

it("does not restore old sessions or consume their history", () => {
  const before = initial(),
    next = moved(before, 900);
  const history = recordCanvasHistory(emptyCanvasHistory("one"), before, next);
  const other = { ...next, sessionId: "two" };
  expect(recordCanvasHistory(history, other, moved(other, 950))).toBe(history);
  expect(restoreCanvasHistory(history, other, "undo")).toEqual({ history, snapshot: other, status: "stale" });
  expect(restoreCanvasHistory(emptyCanvasHistory("two"), other, "undo").status).toBe("empty");
});

it("rejects a locked restore atomically and preserves current group flags", () => {
  const before = initial();
  const next = { ...before, items: before.items.map((item) => (item.group ? { ...item, x: 100 } : item)) };
  const history = recordCanvasHistory(emptyCanvasHistory("one"), before, next);
  const locked = {
    ...next,
    items: next.items.map((item) => (item.group ? { ...item, locked: true, carry: true } : item)),
  };
  const result = restoreCanvasHistory(history, locked, "undo");
  expect(result).toEqual({ history, snapshot: locked, status: "locked" });
  const unlocked = { ...locked, items: locked.items.map((item) => ({ ...item, locked: false })) };
  const restored = restoreCanvasHistory(history, unlocked, "undo");
  expect(restored.snapshot.items[0]).toMatchObject({ x: 0, locked: false, carry: true });
});

it("keeps new objects and their preferences and never resurrects deleted ones", () => {
  const before = initial(),
    next = moved(before, 900);
  before.sizes.deleted = { width: 400, height: 300, minWidth: 218, minHeight: 150 };
  const history = recordCanvasHistory(emptyCanvasHistory("one"), before, next);
  const current: CanvasSnapshot = {
    ...next,
    items: [...next.items.filter((item) => item.id !== "group"), { id: "new", x: 5, y: 6, width: 400, height: 300 }],
    sizes: { new: { width: 400, height: 300, minWidth: 218, minHeight: 150 } },
    edgeIds: ["new-edge"],
    sides: { "new-edge": { source: "bottom", target: "top" } },
  };
  const result = restoreCanvasHistory(history, current, "undo");
  expect(result.snapshot.items.map((item) => item.id)).toEqual(["tool", "new"]);
  expect(result.snapshot.items[1]).toEqual(current.items[1]);
  expect(result.snapshot.sizes).toEqual(current.sizes);
  expect(result.snapshot.sides).toEqual(current.sides);
  expect(result.snapshot.edgeIds).toEqual(["new-edge"]);
});

it("handles prototype-shaped graph identities as ordinary own keys", () => {
  const before = initial();
  before.items[1].id = "__proto__";
  before.sizes = JSON.parse('{"__proto__":{"width":220,"height":180,"minWidth":218,"minHeight":150}}');
  before.edgeIds = ["__proto__"];
  before.sides = JSON.parse('{"__proto__":{"source":"bottom","target":"top"}}');
  const next = copyCanvasSnapshot(before);
  next.items[1].x = 999;
  next.sizes = {};
  next.sides = {};
  const result = restoreCanvasHistory(recordCanvasHistory(emptyCanvasHistory("one"), before, next), next, "undo");
  expect(Object.hasOwn(result.snapshot.sizes, "__proto__")).toBe(true);
  expect(Object.hasOwn(result.snapshot.sides, "__proto__")).toBe(true);
  expect(result.snapshot.sides.__proto__.source).toBe("bottom");
  expect({}).not.toHaveProperty("source");
});

it("arrangement changes only existing node positions and connector presentation", () => {
  const before = initial();
  const next = arrangedCanvasSnapshot(
    before,
    { group: { x: 88, y: 99 }, tool: { x: 50, y: 60 }, other: { x: 0, y: 0 } },
    {},
  );
  expect(next.items[0]).toEqual(before.items[0]);
  expect(next.items[1]).toMatchObject({ x: 50, y: 60, width: 220 });
  expect(next.sessionId).toBe(before.sessionId);
  expect(next.edgeIds).toBe(before.edgeIds);
});

// A small independent reference history models two coordinates and a connector.
// Fixed seeds and the full prefix are printed by the assertion on failure.
for (const seed of qualitySeeds([1, 47, 997, 0x12345678, 0x7fffffff]))
  it(`matches bounded action sequences, seed ${seed}`, () => {
    let rng = seed;
    const random = (limit: number) => {
      rng ^= rng << 13;
      rng ^= rng >>> 17;
      rng ^= rng << 5;
      return (rng >>> 0) % limit;
    };
    type View = { group: number; tool: number; side: "right" | "bottom" };
    let model: View = { group: 0, tool: 800, side: "right" };
    let past: View[] = [],
      future: View[] = [],
      locked = false;
    let current = initial(),
      history = emptyCanvasHistory("one");
    const trace: string[] = [];
    for (let index = 0; index < 400; index++) {
      const action = random(6),
        delta = random(5) * 10,
        which = random(2) ? "tool" : "group";
      trace.push(`${action}:${which}:${delta}`);
      if (action < 3) {
        const nextModel = { ...model };
        if (action === 2) nextModel.side = model.side === "right" ? "bottom" : "right";
        else if (which !== "group" || !locked) nextModel[which] += delta;
        const next: CanvasSnapshot = {
          ...current,
          items: current.items.map((item) => ({ ...item, x: nextModel[item.id as "group" | "tool"] })),
          sides: { edge: { source: nextModel.side, target: "left" } },
        };
        if (nextModel.group !== model.group || nextModel.tool !== model.tool || nextModel.side !== model.side) {
          past = [...past.slice(-19), { ...model }];
          future = [];
        }
        history = recordCanvasHistory(history, current, next);
        current = next;
        model = nextModel;
      } else if (action === 3) {
        locked = !locked;
        current = { ...current, items: current.items.map((item) => (item.group ? { ...item, locked } : item)) };
      } else {
        const undo = action === 4,
          target = (undo ? past : future).at(-1);
        const blocked = target && locked && target.group !== model.group;
        if (target && !blocked) {
          if (undo) {
            past = past.slice(0, -1);
            future = [...future.slice(-19), { ...model }];
          } else {
            future = future.slice(0, -1);
            past = [...past.slice(-19), { ...model }];
          }
          model = target;
        }
        const result = restoreCanvasHistory(history, current, undo ? "undo" : "redo");
        history = result.history;
        current = result.snapshot;
      }
      const evidence = `seed=${seed}; actions=${trace.join(",")}`;
      expect(
        current.items.map((item) => item.x),
        evidence,
      ).toEqual([model.group, model.tool]);
      expect(current.sides.edge?.source ?? "right", evidence).toBe(model.side);
      expect([history.past.length, history.future.length], evidence).toEqual([past.length, future.length]);
      expect(current.items[0].locked === true, evidence).toBe(locked);
      expect(current.edgeIds, evidence).toEqual(["edge"]);
    }
  });
