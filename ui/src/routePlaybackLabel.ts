/** A remembered start is not proof of the current runtime after external edits. */
export function routePlaybackLabel({
  busy,
  running,
  fresh,
  generation,
  started,
  micMuted = false,
}: {
  busy: boolean;
  running: boolean;
  fresh: boolean;
  generation: number | null | undefined;
  started?: { generation: number; preview?: boolean };
  /** Privacy mute is on and this route has a source it silences. Outranks the preview note. */
  micMuted?: boolean;
}): string {
  if (busy) return "Updating audio…";
  if (!running) return "Audio stopped";
  if (micMuted) return "Audio running · Mic muted";
  return fresh && started?.preview && generation === started.generation
    ? "Audio running · Unsaved preview"
    : "Audio running";
}
