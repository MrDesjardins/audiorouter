import { useState } from "react";
import type { Node, DiagnosticsSnapshot } from "@audiorouter/contracts";
import type { UiBackend } from "./backend";
import { formatUiError } from "./backend";
const percent = (db: number) => Math.max(0, Math.min(100, (db + 60) / 66 * 100));
const dbText = (db: number) => db <= -120 ? "−∞" : `${db.toFixed(1)} dBFS`;
export function MeterInspector({ node, sessionId, snapshot, running, backend, onUpgrade }: {
  node: Node; sessionId: string; snapshot: DiagnosticsSnapshot | null; running: boolean; backend: UiBackend; onUpgrade: () => void;
}) {
  const meter = snapshot?.nodeTelemetry.find(item => item.nodeId === node.id)?.meter;
  const channels = node.ports.find(p => p.direction === "input")?.channels ?? 2;
  const active = running && node.enabled && !node.bypass && backend.connected;
  const clipped = meter?.channelClippedSamples.slice(0, channels).some(count => count > 0) ?? false;
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const reset = async () => {
    if (!backend.resetMeter || busy) return;
    setBusy(true);
    try { await backend.resetMeter(sessionId, node.id); setMessage("Peak hold and clipping counters reset. Audio keeps playing."); }
    catch (error) { setMessage(formatUiError(error, "Could not reset this meter.")); }
    finally { setBusy(false); }
  };
  return <section className="meter-inspector" aria-label="Detailed Meter">
    <div className="meter-inspector-heading"><h3>Signal level</h3><button type="button" className="secondary" aria-label="Reset peak & clipping" title="Reset peak hold and clipping counters; audio keeps playing" onClick={() => void reset()} disabled={!backend.connected || !meter || !backend.resetMeter || busy}>{busy ? "Resetting…" : "Reset"}</button></div>
    <div className={`meter-clip-status${clipped ? " meter-clipped" : ""}`} role="status">{clipped ? "Clipping recorded — reset to clear" : "No clipping recorded"}</div>
    <p className="muted">{!backend.connected ? "Disconnected: readings are unavailable." : !node.enabled ? "Off: metering is inactive; sound passes through." : node.bypass ? "Bypass: metering is inactive; sound passes through." : !running ? "Press Play to measure the sound passing through this node." : !meter ? "Waiting for this meter's readings. Check its incoming connection." : "RMS shows your average level. Peak shows the current sample peak; Hold remembers the highest peak."}</p>
    {!node.ports.some(p => p.direction === "output") && <button type="button" className="secondary" disabled={!backend.connected} onClick={onUpgrade}>Add pass-through output</button>}
    <div className="meter-inspector-display">
      <div className="meter-scale" aria-hidden="true">{[6, 0, -3, -6, -12, -18, -24, -36, -48, -60].map(db => <span key={db} style={{ bottom: `${percent(db)}%` }}>{db > 0 ? "+" : ""}{db}</span>)}</div>
      {Array.from({ length: channels }, (_, channel) => {
        const rms = active ? meter?.channelRmsDb[channel] ?? -120 : -120;
        const current = active ? meter?.channelCurrentPeakDb?.[channel] : undefined;
        const hold = meter?.channelPeakDb[channel] ?? -120;
        const clips = meter?.channelClippedSamples[channel] ?? 0;
        return <div className="meter-channel" key={channel}>
          <div className="meter-channel-track" role="meter" aria-label={`${channels === 1 ? "Mono" : channel === 0 ? "Left" : "Right"} RMS level`} aria-valuemin={-60} aria-valuemax={6} aria-valuenow={Math.max(-60, Math.min(6, rms))} aria-valuetext={dbText(rms)}>
            <span className="meter-channel-fill" style={{ height: `${percent(rms)}%` }} />
            {current !== undefined && <span className="meter-current-marker" style={{ bottom: `${percent(current)}%` }} />}
            <span className="meter-hold-marker" style={{ bottom: `${percent(hold)}%` }} />
            <span className="meter-zero-line" style={{ bottom: `${percent(0)}%` }} />
          </div>
          <strong>{channels === 1 ? "Mono" : channel === 0 ? "Left" : "Right"}</strong>
          <dl className="meter-channel-values">
            <dt>RMS</dt><dd>{dbText(rms)}</dd>
            <dt>Peak</dt><dd>{current === undefined ? "—" : dbText(current)}</dd>
            <dt>Hold</dt><dd>{dbText(hold)}</dd>
            <dt>Headroom</dt><dd>{hold <= -120 ? "—" : `${(-hold).toFixed(1)} dB`}</dd>
            <dt>Clipped samples</dt><dd className={clips ? "meter-clipped" : ""}>{clips.toLocaleString()}</dd>
            <dt>Clipped time</dt><dd className={clips ? "meter-clipped" : ""}>{meter?.sampleRateHz ? `${(clips / meter.sampleRateHz).toFixed(3)} s` : "—"}</dd>
            <dt>Clipped share</dt><dd>{meter?.observedFrames ? `${(clips / meter.observedFrames * 100).toFixed(2)}%` : "—"}</dd>
          </dl>
        </div>;
      })}
    </div>
    <p className="meter-inspector-legend">Bar: RMS · thin line: peak · gold line: hold. Above 0 dBFS exceeds full scale.</p>
    {message && <p role="status">{message}</p>}
    <p className="muted">Sample peaks, not true peak or LUFS. Clipped time counts samples above full scale per channel; it is not the length of a continuous clipping event. Statistics restart when the audio graph is prepared or replaced.</p>
  </section>;
}
