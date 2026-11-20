import { tx } from "./i18n";

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
  if (busy) return tx("workspaceAudit.audioUpdating");
  if (!running) return tx("workspaceAudit.audioStopped");
  if (micMuted) return tx("workspaceAudit.audioRunningMuted");
  return tx(
    fresh && started?.preview && generation === started.generation
      ? "workspaceAudit.audioPreview"
      : "workspaceAudit.audioRunning",
  );
}
