/** Local canvas geometry only. No graph identities, routing or parameters change. */
export type CanvasRect = {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  group?: boolean;
  locked?: boolean;
  carry?: boolean;
  minWidth?: number;
  minHeight?: number;
};
export type LayoutAction =
  | "align-left"
  | "align-center"
  | "align-right"
  | "align-top"
  | "align-middle"
  | "align-bottom"
  | "size-min-width"
  | "size-max-width"
  | "size-min-height"
  | "size-max-height"
  | "distribute-horizontal"
  | "distribute-vertical";

const LIMIT = 100_000;
const contains = (outer: CanvasRect, inner: CanvasRect) =>
  outer.x <= inner.x &&
  outer.y <= inner.y &&
  outer.x + outer.width >= inner.x + inner.width &&
  outer.y + outer.height >= inner.y + inner.height;

/** Strictly increasing parent area prevents cycles, even for identical groups. */
export function canvasOwners(items: CanvasRect[]): Map<string, string> {
  const groups = items
    .filter((item) => item.group)
    .sort((a, b) => a.width * a.height - b.width * b.height || a.id.localeCompare(b.id));
  const owners = new Map<string, string>();
  for (const item of items) {
    const owner = groups.find(
      (group) =>
        group.id !== item.id &&
        contains(group, item) &&
        (!item.group || group.width * group.height > item.width * item.height),
    );
    if (owner) owners.set(item.id, owner.id);
  }
  return owners;
}

export function carriedItems(id: string, items: CanvasRect[], owners = canvasOwners(items)): CanvasRect[] {
  const root = items.find((item) => item.id === id);
  if (!root) return [];
  if (!root.group || !root.carry) return [root];
  return items.filter((item) => {
    let ancestor: string | undefined = item.id;
    while (ancestor) {
      if (ancestor === id) return true;
      ancestor = owners.get(ancestor);
    }
    return false;
  });
}

export function movementLocked(id: string, items: CanvasRect[], owners = canvasOwners(items)): boolean {
  return carriedItems(id, items, owners).some((item) => item.locked);
}

/** A selected carrying ancestor owns the movement; selected children never move twice. */
export function movementRoots(items: CanvasRect[], selectedIds: string[], owners = canvasOwners(items)): CanvasRect[] {
  const selected = new Set(selectedIds);
  const byId = new Map(items.map((item) => [item.id, item]));
  return items.filter(
    (item) =>
      selected.has(item.id) &&
      (() => {
        let ancestor = owners.get(item.id);
        while (ancestor) {
          if (selected.has(ancestor) && byId.get(ancestor)?.carry) return false;
          ancestor = owners.get(ancestor);
        }
        return true;
      })(),
  );
}

/** Targets and ownership use one frozen snapshot for the entire gesture/action. */
export function moveCanvasItems(items: CanvasRect[], targets: Map<string, { x: number; y: number }>): CanvasRect[] {
  const owners = canvasOwners(items);
  const changes = new Map<string, { x: number; y: number }>();
  for (const root of movementRoots(items, [...targets.keys()], owners)) {
    if (movementLocked(root.id, items, owners)) continue;
    const target = targets.get(root.id)!;
    if (!Number.isFinite(target.x) || !Number.isFinite(target.y)) continue;
    const members = carriedItems(root.id, items, owners);
    const dx = Math.max(
      ...members.map((item) => -LIMIT - item.x),
      Math.min(target.x - root.x, ...members.map((item) => LIMIT - item.x)),
    );
    const dy = Math.max(
      ...members.map((item) => -LIMIT - item.y),
      Math.min(target.y - root.y, ...members.map((item) => LIMIT - item.y)),
    );
    for (const item of members) changes.set(item.id, { x: item.x + dx, y: item.y + dy });
  }
  return items.map((item) => (changes.has(item.id) ? { ...item, ...changes.get(item.id)! } : item));
}

export function layoutCanvasItems(
  items: CanvasRect[],
  selectedIds: string[],
  action: LayoutAction,
): { items: CanvasRect[]; message: string } {
  const selected = items.filter((item) => selectedIds.includes(item.id));
  if (selected.length < 2) return { items, message: "Select at least two items." };
  if (action.startsWith("size-")) {
    const dimension = action.endsWith("width") ? "width" : "height";
    const minimum = dimension === "width" ? "minWidth" : "minHeight";
    const values = selected.map((item) => item[dimension]);
    const requested = action.startsWith("size-min") ? Math.min(...values) : Math.max(...values);
    const target = Math.max(
      requested,
      ...selected
        .filter((item) => !item.locked)
        .map(
          (item) =>
            item[minimum] ?? (item.group ? (dimension === "width" ? 180 : 100) : dimension === "width" ? 218 : 150),
        ),
    );
    if (target > 10_000) return { items, message: "The requested size exceeds the canvas limit." };
    return {
      items: items.map((item) =>
        selectedIds.includes(item.id) && !item.locked ? { ...item, [dimension]: target } : item,
      ),
      message: "Sizes updated. Locked groups stayed fixed; tool content minima are preserved.",
    };
  }
  const owners = canvasOwners(items);
  const roots = movementRoots(items, selectedIds, owners);
  const fixed = roots.filter((item) => movementLocked(item.id, items, owners));
  const targets = new Map<string, { x: number; y: number }>();
  if (action.startsWith("distribute-")) {
    if (roots.length < 3) return { items, message: "Select at least three independent items to distribute." };
    const horizontal = action === "distribute-horizontal";
    const axis = horizontal ? "x" : "y";
    const dimension = horizontal ? "width" : "height";
    const sorted = [...roots].sort((a, b) => a[axis] - b[axis] || a.id.localeCompare(b.id));
    const anchors = sorted
      .map((item, index) => (index === 0 || index === sorted.length - 1 || fixed.includes(item) ? index : -1))
      .filter((index) => index >= 0);
    for (let segment = 1; segment < anchors.length; segment++) {
      const start = anchors[segment - 1],
        end = anchors[segment];
      const inner = sorted.slice(start + 1, end);
      const available =
        sorted[end][axis] -
        sorted[start][axis] -
        sorted[start][dimension] -
        inner.reduce((sum, item) => sum + item[dimension], 0);
      if (available < -0.001)
        return { items, message: "Not enough room to distribute without overlap. Move the endpoints farther apart." };
      const gap = Math.max(0, available) / (end - start);
      let cursor = sorted[start][axis] + sorted[start][dimension] + gap;
      for (const item of inner) {
        targets.set(item.id, { x: item.x, y: item.y, [axis]: cursor });
        cursor += item[dimension] + gap;
      }
    }
  } else {
    const reference = fixed.length ? fixed : roots;
    const left = Math.min(...reference.map((item) => item.x));
    const right = Math.max(...reference.map((item) => item.x + item.width));
    const top = Math.min(...reference.map((item) => item.y));
    const bottom = Math.max(...reference.map((item) => item.y + item.height));
    for (const item of roots) {
      if (fixed.includes(item)) continue;
      targets.set(item.id, {
        x:
          action === "align-left"
            ? left
            : action === "align-right"
              ? right - item.width
              : action === "align-center"
                ? (left + right - item.width) / 2
                : item.x,
        y:
          action === "align-top"
            ? top
            : action === "align-bottom"
              ? bottom - item.height
              : action === "align-middle"
                ? (top + bottom - item.height) / 2
                : item.y,
      });
    }
  }
  return { items: moveCanvasItems(items, targets), message: "Layout updated. Locked groups stayed fixed." };
}
