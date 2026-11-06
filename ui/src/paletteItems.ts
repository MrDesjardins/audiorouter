import type { IconName } from "./icons";

export type PaletteGroup = "Actions" | "Tools" | "Settings" | "On this route" | "Sessions";
export const PALETTE_GROUPS: readonly PaletteGroup[] = ["Actions", "Tools", "Settings", "On this route", "Sessions"];

export type PaletteItem = {
  id: string;
  group: PaletteGroup;
  icon: IconName;
  title: string;
  detail: string;
  /** Extra words to match, such as "volume" for Gain. */
  keywords?: string;
  disabled?: boolean;
  run: () => void;
};

const SHOWN_PER_GROUP = 6;

/** 0 when it does not match. A title start beats a word start, which beats any other match. */
export function paletteScore(item: Pick<PaletteItem, "title" | "detail" | "keywords">, query: string): number {
  const needle = query.trim().toLowerCase();
  if (!needle) return 1;
  const title = item.title.toLowerCase();
  if (title.startsWith(needle)) return 5;
  if (title.split(/\s+/).some((word) => word.startsWith(needle))) return 4;
  if (title.includes(needle)) return 3;
  if (item.keywords?.toLowerCase().includes(needle)) return 2;
  if (item.detail.toLowerCase().includes(needle)) return 1;
  return 0;
}

/** Matching items in group order, best first inside each group. */
export function paletteResults(items: readonly PaletteItem[], query: string): PaletteItem[] {
  return PALETTE_GROUPS.flatMap((group) =>
    items
      .filter((item) => item.group === group)
      .map((item, index) => ({ item, index, score: paletteScore(item, query) }))
      .filter((entry) => entry.score > 0)
      .sort((a, b) => b.score - a.score || a.index - b.index)
      .slice(0, SHOWN_PER_GROUP)
      .map((entry) => entry.item),
  );
}
