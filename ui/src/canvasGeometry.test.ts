import { describe, expect, it } from "vitest";
import {
  canvasOwners,
  carriedItems,
  layoutCanvasItems,
  moveCanvasItems,
  movementLocked,
  type CanvasRect,
  type LayoutAction,
} from "./canvasGeometry";

const rect = (id: string, x = 0, y = 0, width = 200, height = 150): CanvasRect => ({
  id,
  x,
  y,
  width,
  height,
  minWidth: 100,
  minHeight: 100,
});
const group = (id: string, x: number, y: number, width: number, height: number, carry = true): CanvasRect => ({
  ...rect(id, x, y, width, height),
  group: true,
  carry,
  minWidth: 180,
  minHeight: 100,
});

describe("canvas ownership and group movement", () => {
  it("uses full containment, smallest owner, and a deterministic overlap tie", () => {
    const items = [
      group("outer", 0, 0, 1000, 800),
      group("b", 100, 100, 400, 400),
      group("a", 100, 100, 400, 400),
      rect("tool", 120, 120),
      rect("partial", 450, 450),
    ];
    const owners = canvasOwners(items);
    expect(owners.get("tool")).toBe("a");
    expect(owners.get("partial")).toBe("outer");
    expect(owners.get("a")).toBe("outer");
    expect(owners.get("b")).toBe("outer");
    expect(canvasOwners([...items].reverse())).toEqual(owners);
  });
  it("moves a nested tree once even when children are explicitly selected", () => {
    const items = [
      group("outer", 0, 0, 1000, 800),
      group("inner", 100, 100, 400, 400, false),
      rect("tool", 120, 120),
      rect("outside", 1200, 0),
    ];
    const moved = moveCanvasItems(
      items,
      new Map([
        ["outer", { x: 75, y: -30 }],
        ["inner", { x: 175, y: 70 }],
        ["tool", { x: 195, y: 90 }],
      ]),
    );
    expect(moved.map(({ id, x, y }) => ({ id, x, y }))).toEqual([
      { id: "outer", x: 75, y: -30 },
      { id: "inner", x: 175, y: 70 },
      { id: "tool", x: 195, y: 90 },
      { id: "outside", x: 1200, y: 0 },
    ]);
    expect(items[0].x).toBe(0);
    expect(carriedItems("inner", items).map((item) => item.id)).toEqual(["inner"]);
  });
  it("does not carry overlapping/partial nodes or a node owned by a smaller overlapping group", () => {
    const items = [
      group("left", 0, 0, 700, 500),
      group("right", 400, 0, 250, 400),
      rect("shared", 420, 80),
      rect("partial", -20, 10),
    ];
    // The right group is itself inside left, so both it and its owned child follow left.
    expect(carriedItems("left", items).map((item) => item.id)).toEqual(["left", "right", "shared"]);
    const overlap = [
      group("left", 0, 0, 600, 400),
      group("right", 400, -20, 300, 400),
      rect("shared", 410, 10, 150, 100),
    ];
    expect(carriedItems("left", overlap).map((item) => item.id)).toEqual(["left"]);
  });
  it("blocks carrying a locked descendant, but allows frame-only movement and editing tools independently", () => {
    const inner = { ...group("inner", 100, 100, 400, 400), locked: true };
    const items = [group("outer", 0, 0, 1000, 800), inner, rect("tool", 120, 120)];
    expect(movementLocked("outer", items)).toBe(true);
    expect(moveCanvasItems(items, new Map([["outer", { x: 100, y: 100 }]]))).toEqual(items);
    expect(
      moveCanvasItems(
        items.map((item) => (item.id === "outer" ? { ...item, carry: false } : item)),
        new Map([["outer", { x: 100, y: 100 }]]),
      )[0].x,
    ).toBe(100);
    expect(moveCanvasItems(items, new Map([["tool", { x: 140, y: 120 }]]))[2].x).toBe(140);
  });
  it("clamps a whole movement at coordinate limits without changing internal offsets", () => {
    const items = [group("outer", 99_000, 0, 1000, 500), rect("tool", 99_800, 10, 100, 100)];
    const moved = moveCanvasItems(items, new Map([["outer", { x: 100_000, y: 0 }]]));
    expect(moved[1].x).toBe(100_000);
    expect(moved[1].x - moved[0].x).toBe(800);
  });
});

describe("selection layout", () => {
  it.each<[LayoutAction, number, number]>([
    ["align-left", 0, 0],
    ["align-center", 100, 0],
    ["align-right", 200, 0],
    ["align-top", 0, 0],
    ["align-middle", 0, 100],
    ["align-bottom", 0, 200],
  ])("%s uses the outer selection bounds", (action, x, y) => {
    const items = [rect("a", 0, 0), rect("b", 200, 200)];
    const result = layoutCanvasItems(items, ["a", "b"], action).items;
    if (action === "align-left" || action === "align-center" || action === "align-right")
      expect(result.map((item) => item.x)).toEqual([x, x]);
    else expect(result.map((item) => item.y)).toEqual([y, y]);
  });
  it("aligns to a fixed locked frame and carries an unlocked group together", () => {
    const items = [
      { ...group("anchor", 800, 0, 600, 500), locked: true },
      group("move", 0, 0, 400, 400),
      rect("child", 30, 50),
    ];
    const result = layoutCanvasItems(items, ["anchor", "move", "child"], "align-left").items;
    expect(result[0]).toEqual(items[0]);
    expect(result[1].x).toBe(800);
    expect(result[2].x).toBe(830);
  });
  it("equalizes actual gaps, keeps endpoints, and is independent of selection order", () => {
    const items = [0, 110, 230, 340, 460].map((x, index) => rect(`${index}`, x, index * 10, 100, 100));
    const result = layoutCanvasItems(items, ["4", "1", "0", "3", "2"], "distribute-horizontal").items;
    expect(result.map((item) => item.x)).toEqual([0, 115, 230, 345, 460]);
    expect(result.map((item) => item.y)).toEqual(items.map((item) => item.y));
  });
  it("distributes between locked anchors and rejects insufficient room atomically", () => {
    const items = [
      rect("a", 0),
      rect("b", 150),
      { ...group("fixed", 400, 0, 200, 150), locked: true },
      rect("c", 620),
      rect("d", 1000),
    ];
    const result = layoutCanvasItems(
      items,
      items.map((item) => item.id),
      "distribute-horizontal",
    ).items;
    expect(result.map((item) => item.x)).toEqual([0, 200, 400, 700, 1000]);
    const overlapping = [rect("a"), rect("b", 50), rect("c", 100)];
    expect(layoutCanvasItems(overlapping, ["a", "b", "c"], "distribute-horizontal").items).toBe(overlapping);
  });
  it("matches sizes using a shared safe minimum and leaves locked frames and contents alone", () => {
    const items = [
      group("g", 0, 0, 600, 400),
      { ...rect("a", 20, 20), minWidth: 220 },
      rect("b", 800, 0, 400, 300),
      { ...group("fixed", 1500, 0, 180, 100), locked: true },
    ];
    const result = layoutCanvasItems(items, ["g", "a", "b", "fixed"], "size-min-width").items;
    expect(result.map((item) => item.width)).toEqual([220, 220, 220, 180]);
    expect(result.map((item) => item.x)).toEqual(items.map((item) => item.x));
    expect(layoutCanvasItems(items, ["g", "b"], "size-max-height").items.map((item) => item.height)).toEqual([
      400, 150, 400, 100,
    ]);
  });
});

it("distributes unequal heights vertically using edge gaps", () => {
  const items = [rect("a", 0, 0, 100, 100), rect("b", 30, 200, 100, 200), rect("c", 60, 700, 100, 300)];
  const result = layoutCanvasItems(items, ["c", "a", "b"], "distribute-vertical").items;
  expect(result.map((item) => item.y)).toEqual([0, 300, 700]);
  expect(result.map((item) => item.x)).toEqual([0, 30, 60]);
});
