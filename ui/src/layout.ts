import { readPresentationJson, writePresentationJson } from "./presentationStorage";

export type EdgeSide = "left" | "right" | "top" | "bottom";
export type EdgeLayoutSides = Record<string, { source: EdgeSide; target: EdgeSide }>;
export type LayoutPosition = { x: number; y: number };
export type LayoutPositions = Record<string, LayoutPosition>;
export type NodeLayoutSize = { width: number; height: number; minWidth: number; minHeight: number };
export type NodeLayoutSizes = Record<string, NodeLayoutSize>;

const MAX_COORDINATE = 100_000;
// Matches the domain's maximum entity ID byte length; presentation grants no authority.
const MAX_ID_BYTES = 128;
const encoder = new TextEncoder();
const record = (value: unknown): value is Record<string, unknown> =>
  !!value && typeof value === "object" && !Array.isArray(value);
function decodeRecords<T>(value: unknown, decode: (value: unknown) => T | null): Record<string, T> {
  if (!record(value)) return {};
  return Object.fromEntries(
    Object.entries(value).flatMap(([id, saved]) => {
      if (!id || id.includes("\0") || id.length > MAX_ID_BYTES || encoder.encode(id).length > MAX_ID_BYTES) return [];
      const decoded = decode(saved);
      return decoded ? [[id, decoded]] : [];
    }),
  );
}
function decodePosition(value: unknown): LayoutPosition | null {
  if (!record(value)) return null;
  const { x, y } = value;
  return typeof x === "number" &&
    Number.isFinite(x) &&
    Math.abs(x) <= MAX_COORDINATE &&
    typeof y === "number" &&
    Number.isFinite(y) &&
    Math.abs(y) <= MAX_COORDINATE
    ? { x, y }
    : null;
}
function decodeSize(value: unknown): NodeLayoutSize | null {
  if (!record(value)) return null;
  const size = value as NodeLayoutSize;
  if (
    ![size.width, size.height, size.minWidth, size.minHeight].every(
      (n) => typeof n === "number" && Number.isFinite(n) && n >= 100 && n <= 10_000,
    ) ||
    size.width < size.minWidth ||
    size.height < size.minHeight
  )
    return null;
  return { width: size.width, height: size.height, minWidth: size.minWidth, minHeight: size.minHeight };
}
function decodeSides(value: unknown): EdgeLayoutSides[string] | null {
  if (!record(value)) return null;
  const sides = value as EdgeLayoutSides[string];
  return [sides.source, sides.target].every((side) => ["left", "right", "top", "bottom"].includes(side))
    ? { source: sides.source, target: sides.target }
    : null;
}
export function readLayout(storage: Pick<Storage, "getItem"> | null, key: string): LayoutPositions {
  return decodeRecords(readPresentationJson(storage, key), decodePosition);
}
export function writeLayout(storage: Pick<Storage, "setItem"> | null, key: string, positions: LayoutPositions): void {
  writePresentationJson(storage, key, decodeRecords(positions, decodePosition));
}
export function readNodeSizes(storage: Pick<Storage, "getItem"> | null, key: string): NodeLayoutSizes {
  return decodeRecords(readPresentationJson(storage, key), decodeSize);
}
export function writeNodeSizes(storage: Pick<Storage, "setItem"> | null, key: string, sizes: NodeLayoutSizes): void {
  writePresentationJson(storage, key, decodeRecords(sizes, decodeSize));
}
export function readEdgeLayout(storage: Pick<Storage, "getItem"> | null, key: string): EdgeLayoutSides {
  return decodeRecords(readPresentationJson(storage, key), decodeSides);
}
export function writeEdgeLayout(storage: Pick<Storage, "setItem"> | null, key: string, sides: EdgeLayoutSides): void {
  writePresentationJson(storage, key, decodeRecords(sides, decodeSides));
}
export function clearLayout(storage: Pick<Storage, "removeItem"> | null, key: string): void {
  try {
    storage?.removeItem(key);
  } catch {
    /* Optional presentation storage. */
  }
}
