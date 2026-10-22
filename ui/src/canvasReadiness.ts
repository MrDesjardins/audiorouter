export type ConnectorRequirement = { id: string; source: string[]; target: string[] };
type HandleBounds = { id?: string | null; x: number; y: number; width: number; height: number };
const positive = (value: unknown) => typeof value === "number" && Number.isFinite(value) && value > 0;

/** Current card measurements and every expected handle must be usable. */
export function connectorGeometryReady(
  measured: { width?: number; height?: number } | undefined,
  bounds: { source?: readonly HandleBounds[] | null; target?: readonly HandleBounds[] | null } | null | undefined,
  expected: ConnectorRequirement,
): boolean {
  if (!positive(measured?.width) || !positive(measured?.height)) return false;
  const valid = (bound: HandleBounds, id: string) =>
    bound.id === id &&
    Number.isFinite(bound.x) &&
    Number.isFinite(bound.y) &&
    positive(bound.width) &&
    positive(bound.height);
  return (
    expected.source.every((id) => bounds?.source?.some((bound) => valid(bound, id))) &&
    expected.target.every((id) => bounds?.target?.some((bound) => valid(bound, id)))
  );
}
