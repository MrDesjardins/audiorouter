/**
 * Route edits and layout changes keep separate histories: each restores only its own data. This log
 * only remembers which kind came last, so Ctrl+Z and the top-bar Undo can take the newest change.
 */
export type EditKind = "route" | "layout";
export type EditOrder = { past: EditKind[]; future: EditKind[] };
export type EditDirection = "undo" | "redo";

const LIMIT = 200;

export const emptyEditOrder = (): EditOrder => ({ past: [], future: [] });

/** A new edit of one kind clears that kind's redo; the other kind's redo stays available. */
export function recordEdit(order: EditOrder, kind: EditKind): EditOrder {
  return {
    past: [...order.past, kind].slice(-LIMIT),
    future: order.future.filter((entry) => entry !== kind),
  };
}

function withoutLast(entries: EditKind[], kind: EditKind): EditKind[] {
  const index = entries.lastIndexOf(kind);
  return index < 0 ? entries : [...entries.slice(0, index), ...entries.slice(index + 1)];
}

/** One history moved a step, by whichever control asked for it. */
export function recordRestore(order: EditOrder, kind: EditKind, direction: EditDirection): EditOrder {
  return direction === "undo"
    ? { past: withoutLast(order.past, kind), future: [...order.future, kind].slice(-LIMIT) }
    : { past: [...order.past, kind].slice(-LIMIT), future: withoutLast(order.future, kind) };
}

/**
 * Which history the next undo or redo belongs to: the newest logged kind that can act, otherwise any
 * history that can. Entries can outlive their history (a session switch clears layout history), so
 * availability decides, not the log alone.
 */
export function nextEditKind(
  order: EditOrder,
  direction: EditDirection,
  available: Record<EditKind, boolean>,
): EditKind | null {
  const entries = direction === "undo" ? order.past : order.future;
  for (let index = entries.length - 1; index >= 0; index -= 1) if (available[entries[index]]) return entries[index];
  if (available.route) return "route";
  return available.layout ? "layout" : null;
}
