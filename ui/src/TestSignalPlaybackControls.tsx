import type { MouseEvent } from "react";
import { useMessages } from "./i18n/useMessages";

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
export function TestSignalPlaybackControls({
  durationMs,
  routed,
  sessionRunning,
  playing,
  actionBusy,
  playbackReady,
  endpointPrepared,
  nodeEnabled,
  onStart,
  onStop,
}: TestSignalPlaybackControlsProps) {
  const tr = useMessages();
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
  return (
    <div className="node-test-signal-controls">
      <div className="node-test-signal-actions">
        <button
          type="button"
          className="primary"
          aria-label={tr("g1.toneAriaPlay")}
          title={tr("g1.toneTitlePlay")}
          disabled={!canStart || playing || actionBusy}
          onPointerDown={(event) => event.stopPropagation()}
          onClick={startClick}
        >
          {actionBusy && !playing ? tr("g1.toneStarting") : tr("g1.tonePlay")}
        </button>
        <button
          type="button"
          className="secondary"
          aria-label={tr("g1.toneAriaStop")}
          title={tr("g1.toneTitleStop")}
          disabled={!onStop || !playing || actionBusy}
          onPointerDown={(event) => event.stopPropagation()}
          onClick={stopClick}
        >
          {actionBusy && playing ? tr("g1.toneStopping") : tr("g1.toneStop")}
        </button>
      </div>
      {sessionRunning && routed && nodeEnabled ? (
        // The route runs either way; say only what this tone is doing (owner, 2026-11-07).
        <small className="node-test-signal-help">
          {tr("g1.toneLine", {
            state: playing ? tr("g1.tonePlaying") : tr("g1.toneOff"),
            seconds: durationSeconds.toFixed(durationSeconds < 10 ? 1 : 0),
          })}
        </small>
      ) : (
        <small className="node-test-signal-help" role="status">
          {!routed
            ? tr("g1.toneNotRouted")
            : !nodeEnabled
              ? tr("g1.toneEnable")
              : !playbackReady
                ? tr("g1.tonePreview")
                : !endpointPrepared
                  ? tr("g1.tonePrepare")
                  : tr("g1.toneReady", { seconds: durationSeconds.toFixed(durationSeconds < 10 ? 1 : 0) })}
        </small>
      )}
    </div>
  );
}
