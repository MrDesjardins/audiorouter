export type LayoutPosition = { x: number; y: number };
export type LayoutPositions = Record<string, LayoutPosition>;
export type NodeLayoutSize = { width: number; height: number; minWidth: number; minHeight: number };
export type NodeLayoutSizes = Record<string, NodeLayoutSize>;

export function readNodeSizes(storage: Pick<Storage, "getItem"> | null, key: string): NodeLayoutSizes {
  try {
    const parsed: unknown = JSON.parse(storage?.getItem(key) ?? "{}");
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    return Object.fromEntries(
      Object.entries(parsed).filter(([, value]) => {
        if (!value || typeof value !== "object" || Array.isArray(value)) return false;
        const size = value as NodeLayoutSize;
        return (
          [size.width, size.height, size.minWidth, size.minHeight].every(
            (n) => typeof n === "number" && Number.isFinite(n) && n >= 100 && n <= 10_000,
          ) &&
          size.width >= size.minWidth &&
          size.height >= size.minHeight
        );
      }),
    );
  } catch {
    return {};
  }
}

export function writeNodeSizes(storage: Pick<Storage, "setItem"> | null, key: string, sizes: NodeLayoutSizes): void {
  try {
    storage?.setItem(key, JSON.stringify(sizes));
  } catch {
    /* Optional presentation storage. */
  }
}

const MAX_COORDINATE = 100_000;

function validPosition(value: unknown): value is LayoutPosition {
  if (!value || typeof value !== "object") return false;
  const position = value as Record<string, unknown>;
  return (
    typeof position.x === "number" &&
    Number.isFinite(position.x) &&
    Math.abs(position.x) <= MAX_COORDINATE &&
    typeof position.y === "number" &&
    Number.isFinite(position.y) &&
    Math.abs(position.y) <= MAX_COORDINATE
  );
}

export function readLayout(storage: Pick<Storage, "getItem"> | null, key: string): LayoutPositions {
  if (!storage) return {};
  try {
    const parsed: unknown = JSON.parse(storage.getItem(key) ?? "null");
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    return Object.fromEntries(Object.entries(parsed).filter(([, value]) => validPosition(value))) as LayoutPositions;
  } catch {
    return {};
  }
}

export function writeLayout(storage: Pick<Storage, "setItem"> | null, key: string, positions: LayoutPositions): void {
  if (!storage) return;
  const bounded = Object.fromEntries(Object.entries(positions).filter(([, value]) => validPosition(value)));
  try {
    storage.setItem(key, JSON.stringify(bounded));
  } catch {
    // Layout persistence is optional presentation state and must never block editing.
  }
}

export function clearLayout(storage: Pick<Storage, "removeItem"> | null, key: string): void {
  try {
    storage?.removeItem(key);
  } catch {
    // Layout persistence is optional presentation state.
  }
}
