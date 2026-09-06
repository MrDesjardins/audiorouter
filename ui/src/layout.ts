export type LayoutPosition = { x: number; y: number };
export type LayoutPositions = Record<string, LayoutPosition>;

const MAX_COORDINATE = 100_000;

function validPosition(value: unknown): value is LayoutPosition {
  if (!value || typeof value !== "object") return false;
  const position = value as Record<string, unknown>;
  return typeof position.x === "number" && Number.isFinite(position.x) && Math.abs(position.x) <= MAX_COORDINATE
    && typeof position.y === "number" && Number.isFinite(position.y) && Math.abs(position.y) <= MAX_COORDINATE;
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
