import type { MouseEvent } from "react";

type TestSignalPlaybackControlsProps = {
  durationMs: number;
  routed: boolean;
  sessionRunning: boolean;
  playing: boolean;
  actionBusy: boolean;
  playbackReady: boolean;
  endpointPrepared: boolean;
  nodeEnabled: boolean;
  onStart?: () => void;
  onStop?: () => void;
};

/** Source-local tone controls; route Play/Stop remains in the workspace header. */
export function TestSignalPlaybackControls({ durationMs, routed, sessionRunning, playing, actionBusy, playbackReady, endpointPrepared, nodeEnabled, onStart, onStop }: TestSignalPlaybackControlsProps) {
  const finiteDurationMs = Number.isFinite(durationMs) ? Math.max(1, durationMs) : 1_000;
  const durationSeconds = finiteDurationMs / 1000;
  // Play may prepare a stopped route first; Stop only silences this source.
  const canStart = Boolean(onStart && routed && nodeEnabled);
  const stopClick = (event: MouseEvent<HTMLButtonElement>) => {
    event.stopPropagation();
    onStop?.();
  };
  const startClick = (event: MouseEvent<HTMLButtonElement>) => {
    event.stopPropagation();
    onStart?.();
  };
  return <div className="node-test-signal-controls">
    <div className="node-test-signal-actions">
      <button type="button" className="primary" aria-label="Play Test Signal" title="Plays this tone through the canvas route" disabled={!canStart || playing || actionBusy} onPointerDown={(event) => event.stopPropagation()} onClick={startClick}>{actionBusy && !playing ? "Starting…" : "Play"}</button>
      <button type="button" className="secondary" aria-label="Stop Test Signal" title="Stops this tone; the canvas route keeps running" disabled={!onStop || !playing || actionBusy} onPointerDown={(event) => event.stopPropagation()} onClick={stopClick}>{actionBusy && playing ? "Stopping…" : "Stop"}</button>
    </div>
    {sessionRunning
      ? <small className="node-test-signal-help">{playing ? "Tone playing" : "Route running · tone stopped"} · {durationSeconds.toFixed(durationSeconds < 10 ? 1 : 0)} s</small>
      : <small className="node-test-signal-help" role="status">{!routed ? "Connect to an enabled physical output." : !nodeEnabled ? "Enable this source." : !playbackReady ? "Press Play to preview this unsaved route." : !endpointPrepared ? "Play checks audio setup. Prepare endpoints in Devices." : `Ready · ${durationSeconds.toFixed(durationSeconds < 10 ? 1 : 0)} s`}</small>}
  </div>;
}
