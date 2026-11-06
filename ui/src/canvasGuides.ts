/** Dashed lines shown while a card is dragged, when its edges or centre line up with another card. */
export type GuideRect = { id: string; name: string; x: number; y: number; width: number; height: number };
export type GuideLine = { axis: "x" | "y"; at: number; from: number; to: number };
export type CanvasGuides = { lines: GuideLine[]; names: string[] };

/** Canvas units. A card already off the 24 px grid still lines up when it is this close. */
export const GUIDE_TOLERANCE = 1;
const NAMES_SHOWN = 2;
export const NO_GUIDES: CanvasGuides = { lines: [], names: [] };

const stops = (start: number, size: number) => [start, start + size / 2, start + size];

export function canvasGuides(moving: GuideRect, others: readonly GuideRect[]): CanvasGuides {
  const lines = new Map<string, GuideLine>();
  const names: string[] = [];
  for (const other of others) {
    if (other.id === moving.id) continue;
    let matched = false;
    for (const axis of ["x", "y"] as const) {
      const mine = axis === "x" ? stops(moving.x, moving.width) : stops(moving.y, moving.height);
      const theirs = axis === "x" ? stops(other.x, other.width) : stops(other.y, other.height);
      for (const a of mine) {
        for (const b of theirs) {
          if (Math.abs(a - b) > GUIDE_TOLERANCE) continue;
          matched = true;
          const [start, end] =
            axis === "x"
              ? [Math.min(moving.y, other.y), Math.max(moving.y + moving.height, other.y + other.height)]
              : [Math.min(moving.x, other.x), Math.max(moving.x + moving.width, other.x + other.width)];
          const key = `${axis}:${Math.round(b)}`;
          const known = lines.get(key);
          lines.set(key, {
            axis,
            at: b,
            from: known ? Math.min(known.from, start) : start,
            to: known ? Math.max(known.to, end) : end,
          });
        }
      }
    }
    if (matched) names.push(other.name);
  }
  return { lines: [...lines.values()], names };
}

/** "Lined up with Mixer and Gain"; a long list stays short enough for one line. */
export function guideLabel(names: readonly string[]): string {
  if (names.length === 0) return "";
  const shown = names.slice(0, NAMES_SHOWN);
  const rest = names.length - shown.length;
  if (rest > 0) return `Lined up with ${shown.join(", ")} and ${rest} more`;
  return `Lined up with ${shown.join(" and ")}`;
}
