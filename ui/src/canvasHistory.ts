import type { CanvasRect } from "./canvasGeometry";
import type { EdgeLayoutSides, LayoutPositions, NodeLayoutSizes } from "./layout";

/** Presentation only: graph revisions, flags and routing never enter this history. */
export type CanvasSnapshot = {
  sessionId: string;
  items: CanvasRect[];
  sizes: NodeLayoutSizes;
  sides: EdgeLayoutSides;
  edgeIds: string[];
};
export type CanvasHistory = { sessionId: string; past: CanvasSnapshot[]; future: CanvasSnapshot[] };
const LIMIT = 20;
const DEFAULT_SIDES = { source: "right", target: "left" } as const;
const own = <T>(record: Record<string, T>, id: string): T | undefined =>
  Object.hasOwn(record, id) ? record[id] : undefined;

export const emptyCanvasHistory = (sessionId: string): CanvasHistory => ({ sessionId, past: [], future: [] });
export function copyCanvasSnapshot(snapshot: CanvasSnapshot): CanvasSnapshot {
  return {
    ...snapshot,
    items: snapshot.items.map((item) => ({ ...item })),
    sizes: Object.fromEntries(Object.entries(snapshot.sizes).map(([id, size]) => [id, { ...size }])),
    sides: Object.fromEntries(Object.entries(snapshot.sides).map(([id, sides]) => [id, { ...sides }])),
    edgeIds: [...snapshot.edgeIds],
  };
}
export function geometryChanged(before: CanvasRect[], after: CanvasRect[]): boolean {
  const byId = new Map(after.map((item) => [item.id, item]));
  return before.some((item) => {
    const next = byId.get(item.id);
    return next && (item.x !== next.x || item.y !== next.y || item.width !== next.width || item.height !== next.height);
  });
}
/** Membership changes belong to graph history; only common objects are comparable. */
export function canvasLayoutChanged(before: CanvasSnapshot, after: CanvasSnapshot): boolean {
  if (before.sessionId !== after.sessionId) return false;
  if (geometryChanged(before.items, after.items)) return true;
  const current = new Set(after.edgeIds);
  return before.edgeIds.some((id) => {
    if (!current.has(id)) return false;
    const a = own(before.sides, id) ?? DEFAULT_SIDES;
    const b = own(after.sides, id) ?? DEFAULT_SIDES;
    return a.source !== b.source || a.target !== b.target;
  });
}
export function recordCanvasHistory(
  history: CanvasHistory,
  before: CanvasSnapshot,
  after: CanvasSnapshot,
): CanvasHistory {
  if (history.sessionId !== before.sessionId || !canvasLayoutChanged(before, after)) return history;
  return { ...history, past: [...history.past.slice(-(LIMIT - 1)), copyCanvasSnapshot(before)], future: [] };
}
export function arrangedCanvasSnapshot(
  snapshot: CanvasSnapshot,
  positions: LayoutPositions,
  sides: EdgeLayoutSides,
): CanvasSnapshot {
  return {
    ...snapshot,
    items: snapshot.items.map((item) => {
      const position = own(positions, item.id);
      return !item.group && position ? { ...item, ...position } : item;
    }),
    sides,
  };
}
export function restoreCanvasHistory(
  history: CanvasHistory,
  current: CanvasSnapshot,
  direction: "undo" | "redo",
): {
  history: CanvasHistory;
  snapshot: CanvasSnapshot;
  status: "empty" | "stale" | "locked" | "restored";
} {
  if (history.sessionId !== current.sessionId) return { history, snapshot: current, status: "stale" };
  const stack = direction === "undo" ? history.past : history.future;
  const previous = stack.at(-1);
  if (!previous) return { history, snapshot: current, status: "empty" };
  if (previous.sessionId !== current.sessionId) return { history, snapshot: current, status: "stale" };
  const byId = new Map(previous.items.map((item) => [item.id, item]));
  const items = current.items.map((item) => {
    const old = byId.get(item.id);
    return old ? { ...item, x: old.x, y: old.y, width: old.width, height: old.height } : item;
  });
  if (current.items.some((item, index) => item.locked && geometryChanged([item], [items[index]])))
    return { history, snapshot: current, status: "locked" };
  const sizes: NodeLayoutSizes = Object.assign(Object.create(null), current.sizes);
  for (const item of current.items)
    if (!item.group && byId.has(item.id)) {
      const size = own(previous.sizes, item.id);
      if (size) sizes[item.id] = { ...size };
      else delete sizes[item.id];
    }
  const sides: EdgeLayoutSides = Object.assign(Object.create(null), current.sides);
  const previousEdges = new Set(previous.edgeIds);
  for (const id of current.edgeIds)
    if (previousEdges.has(id)) {
      const saved = own(previous.sides, id);
      if (saved) sides[id] = { ...saved };
      else delete sides[id];
    }
  const savedCurrent = copyCanvasSnapshot(current);
  return {
    snapshot: { ...current, items, sizes, sides },
    status: "restored",
    history:
      direction === "undo"
        ? { ...history, past: history.past.slice(0, -1), future: [...history.future.slice(-(LIMIT - 1)), savedCurrent] }
        : {
            ...history,
            past: [...history.past.slice(-(LIMIT - 1)), savedCurrent],
            future: history.future.slice(0, -1),
          },
  };
}
