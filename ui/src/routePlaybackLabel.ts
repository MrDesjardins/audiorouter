/** A remembered start is not proof of the current runtime after external edits. */
export function routePlaybackLabel({
  busy,
  running,
  fresh,
  generation,
  started,
}: {
  busy: boolean;
  running: boolean;
  fresh: boolean;
  generation: number | null | undefined;
  started?: { generation: number; preview?: boolean };
}): string {
  if (busy) return "Updating audio…";
  if (!running) return "Route stopped";
  return fresh && started?.preview && generation === started.generation ? "Draft preview" : "Route running";
}
