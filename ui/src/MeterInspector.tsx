import { useRef, useState } from "react";
import { nextFallingLevel, nextPeakHold, type PeakHold } from "./dynamics";
import type { Node, DiagnosticsSnapshot } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";
import { formatUiError } from "./backend";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";
const percent = (db: number) => Math.max(0, Math.min(100, ((db + 60) / 66) * 100));
const dbText = (db: number) => (db <= -120 ? "−∞" : `${db.toFixed(1)} dBFS`);

/** UI-21: where spoken peaks should land, in dBFS. */
export const VOICE_ZONE = { lowDb: -18, highDb: -6 } as const;
export type LevelVerdict = "waiting" | "quiet" | "zone" | "loud" | "tooLoud";

/** Judge a voice from its displayed (held) peak; undefined means no reading. */
export function levelVerdict(peakDb: number | undefined): LevelVerdict {
  if (peakDb === undefined || Number.isNaN(peakDb) || peakDb < -50) return "waiting";
  if (peakDb >= -1) return "tooLoud";
  if (peakDb > VOICE_ZONE.highDb) return "loud";
  if (peakDb >= VOICE_ZONE.lowDb) return "zone";
  return "quiet";
}

const VERDICT_TEXT: Record<LevelVerdict, MessageKey> = {
  waiting: "g5.meterWaiting",
  quiet: "g5.meterQuiet",
  zone: "g5.meterZone",
  loud: "g5.meterLoud",
  tooLoud: "g5.meterTooLoud",
};

/**
 * Display ballistics per channel: the RMS bar falls smoothly and the peak
 * line holds before falling, so the eye can follow speech. Readouts keep
 * the exact backend values.
 */
function useMeterBallistics(rms: number[], peaks: (number | undefined)[]) {
  const bars = useRef<({ db: number; at: number } | null)[]>([]);
  const holds = useRef<(PeakHold | null)[]>([]);
  const now = performance.now();
  return rms.map((value, channel) => {
    bars.current[channel] = value <= -120 ? null : nextFallingLevel(bars.current[channel] ?? null, value, now);
    const peak = peaks[channel];
    holds.current[channel] = peak === undefined ? null : nextPeakHold(holds.current[channel] ?? null, peak, now);
    return { rms: bars.current[channel]?.db ?? -120, peak: holds.current[channel]?.db };
  });
}
export function MeterInspector({
  node,
  sessionId,
  snapshot,
  running,
  backend,
  onUpgrade,
}: {
  node: Node;
  sessionId: string;
  snapshot: DiagnosticsSnapshot | null;
  running: boolean;
  backend: UiBackend;
  onUpgrade: () => void;
}) {
  const tr = useMessages();
  const meter = snapshot?.nodeTelemetry.find((item) => item.nodeId === node.id)?.meter;
  const channels = node.ports.find((p) => p.direction === "input")?.channels ?? 2;
  const active = running && node.enabled && !node.bypass && backend.connected;
  const clipped = meter?.channelClippedSamples.slice(0, channels).some((count) => count > 0) ?? false;
  const [message, setMessage] = useState("");
  const display = useMeterBallistics(
    Array.from({ length: channels }, (_, channel) => (active ? (meter?.channelRmsDb[channel] ?? -120) : -120)),
    Array.from({ length: channels }, (_, channel) => (active ? meter?.channelCurrentPeakDb?.[channel] : undefined)),
  );
  const verdict = levelVerdict(
    active
      ? display.reduce<number | undefined>(
          (loudest, channel) =>
            channel.peak === undefined ? loudest : Math.max(loudest ?? channel.peak, channel.peak),
          undefined,
        )
      : undefined,
  );
  const [busy, setBusy] = useState(false);
  const reset = async () => {
    if (!backend.resetMeter || busy) return;
    setBusy(true);
    try {
      await backend.resetMeter(sessionId, node.id);
      setMessage(tr("g5.meterResetDone"));
    } catch (error) {
      setMessage(formatUiError(error, tr("g5.meterResetFail")));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="meter-inspector" aria-label={tr("g5.meterAria")}>
      <div className="meter-inspector-heading">
        <h3>{tr("g5.meterHeading")}</h3>
        <button
          type="button"
          className="secondary"
          aria-label={tr("g5.meterResetAria")}
          title={tr("g5.meterResetTitle")}
          onClick={() => void reset()}
          disabled={!backend.connected || !meter || !backend.resetMeter || busy}
        >
          {busy ? tr("g5.meterResetting") : tr("g5.meterReset")}
        </button>
      </div>
      <div className={`meter-clip-status${clipped ? " meter-clipped" : ""}`} role="status">
        {clipped ? tr("g5.meterClipped") : tr("g5.meterNoClip")}
      </div>
      <p className={`meter-level-verdict is-${verdict}`} data-verdict={verdict}>
        {tr(VERDICT_TEXT[verdict])}
      </p>
      <p className="muted">
        {!backend.connected
          ? tr("g5.meterDisconnected")
          : !node.enabled
            ? tr("g5.meterOff")
            : node.bypass
              ? tr("g5.meterBypass")
              : !running
                ? tr("g5.meterPressPlay")
                : !meter
                  ? tr("g5.meterWaitingReadings")
                  : tr("g5.meterExplain")}
      </p>
      {!node.ports.some((p) => p.direction === "output") && (
        <button type="button" className="secondary" disabled={!backend.connected} onClick={onUpgrade}>
          {tr("g5.meterAddOutput")}
        </button>
      )}
      <div className="meter-inspector-display">
        <div className="meter-scale" aria-hidden="true">
          {[6, 0, -3, -6, -12, -18, -24, -36, -48, -60].map((db) => (
            <span key={db} style={{ bottom: `${percent(db)}%` }}>
              {db > 0 ? "+" : ""}
              {db}
            </span>
          ))}
        </div>
        {Array.from({ length: channels }, (_, channel) => {
          const rms = active ? (meter?.channelRmsDb[channel] ?? -120) : -120;
          const current = active ? meter?.channelCurrentPeakDb?.[channel] : undefined;
          const hold = meter?.channelPeakDb[channel] ?? -120;
          const clips = meter?.channelClippedSamples[channel] ?? 0;
          const channelName = tr(channels === 1 ? "g5.meterMono" : channel === 0 ? "g5.meterLeft" : "g5.meterRight");
          return (
            <div className="meter-channel" key={channel}>
              <div
                className="meter-channel-track"
                role="meter"
                aria-label={tr("g5.meterRmsLevel", { channel: channelName })}
                aria-valuemin={-60}
                aria-valuemax={6}
                aria-valuenow={Math.max(-60, Math.min(6, rms))}
                aria-valuetext={dbText(rms)}
              >
                <span className="meter-channel-fill" style={{ height: `${percent(display[channel].rms)}%` }} />
                <span
                  className="meter-target-zone"
                  style={{
                    bottom: `${percent(VOICE_ZONE.lowDb)}%`,
                    height: `${percent(VOICE_ZONE.highDb) - percent(VOICE_ZONE.lowDb)}%`,
                  }}
                />
                {display[channel].peak !== undefined && (
                  <span className="meter-current-marker" style={{ bottom: `${percent(display[channel].peak!)}%` }} />
                )}
                <span className="meter-hold-marker" style={{ bottom: `${percent(hold)}%` }} />
                <span className="meter-zero-line" style={{ bottom: `${percent(0)}%` }} />
              </div>
              <strong>{channelName}</strong>
              <dl className="meter-channel-values">
                <dt>{tr("g5.meterRms")}</dt>
                <dd>{dbText(rms)}</dd>
                <dt>{tr("g5.meterPeak")}</dt>
                <dd>{current === undefined ? "—" : dbText(current)}</dd>
                <dt>{tr("g5.meterHold")}</dt>
                <dd>{dbText(hold)}</dd>
                <dt>{tr("g5.meterHeadroom")}</dt>
                <dd>{hold <= -120 ? "—" : `${(-hold).toFixed(1)} dB`}</dd>
                <dt>{tr("g5.meterClippedSamples")}</dt>
                <dd className={clips ? "meter-clipped" : ""}>{clips.toLocaleString()}</dd>
                <dt>{tr("g5.meterClippedTime")}</dt>
                <dd className={clips ? "meter-clipped" : ""}>
                  {meter?.sampleRateHz ? `${(clips / meter.sampleRateHz).toFixed(3)} s` : "—"}
                </dd>
                <dt>{tr("g5.meterClippedShare")}</dt>
                <dd>{meter?.observedFrames ? `${((clips / meter.observedFrames) * 100).toFixed(2)}%` : "—"}</dd>
              </dl>
            </div>
          );
        })}
      </div>
      <p className="meter-inspector-legend">{tr("g5.meterLegend")}</p>
      {message && <p role="status">{message}</p>}
      <p className="muted">{tr("g5.meterFootnote")}</p>
    </section>
  );
}
