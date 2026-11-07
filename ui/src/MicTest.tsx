import { useEffect, useEffectEvent, useRef, useState } from "react";
import type { DiagnosticsSnapshot, Session } from "@audiorouter/contracts";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./icons";
import { currentPeak, MIC_TEST_MS, MIC_VERDICT_TEXT, micVerdict, type MicVerdict } from "./micVerdict";

type Phase = { name: "idle" } | { name: "listening" } | { name: "done"; verdict: MicVerdict; loudest: number };

const formatPeak = (peak: number | null) =>
  peak === null || !Number.isFinite(peak) || peak < -90 ? "Peak −∞" : `Peak ${peak.toFixed(0)} dB`.replace("-", "−");

/**
 * Listens to the meter of a microphone that is already playing and says what
 * it found. It reads levels only: no recording, no change to the route, and it
 * never starts audio or picks another input. All states share one card size.
 */
export function MicTest({
  session,
  diagnostics,
  running,
  privacyMuted,
}: {
  session: Session;
  diagnostics: DiagnosticsSnapshot | null;
  running: boolean;
  privacyMuted: boolean;
}) {
  const microphones = session.nodes.filter((node) => node.kind === "physicalInput");
  const [chosen, setChosen] = useState<string | null>(null);
  const microphone = microphones.find((node) => node.id === chosen) ?? microphones[0] ?? null;
  const [phase, setPhase] = useState<Phase>({ name: "idle" });
  const [settingsFailed, setSettingsFailed] = useState(false);
  const loudest = useRef(-Infinity);
  const peak = microphone
    ? currentPeak(diagnostics?.nodeTelemetry.find((item) => item.nodeId === microphone.id)?.meter)
    : null;

  useEffect(() => {
    if (phase.name === "listening" && peak !== null) loudest.current = Math.max(loudest.current, peak);
  }, [phase.name, peak]);

  const finish = useEffectEvent(() =>
    setPhase({ name: "done", verdict: micVerdict(loudest.current), loudest: loudest.current }),
  );
  useEffect(() => {
    if (phase.name !== "listening") return;
    const timer = window.setTimeout(() => finish(), MIC_TEST_MS);
    return () => window.clearTimeout(timer);
  }, [phase.name]);

  // A listen only means something while this exact input keeps playing.
  const stopListening = useEffectEvent(() => setPhase({ name: "idle" }));
  const microphoneId = microphone?.id;
  useEffect(() => {
    if (!running) stopListening();
  }, [running]);
  useEffect(() => {
    stopListening();
  }, [microphoneId]);

  const start = () => {
    loudest.current = -Infinity;
    setSettingsFailed(false);
    setPhase({ name: "listening" });
  };
  const openSettings = () => {
    void invoke("open_sound_settings").catch(() => setSettingsFailed(true));
  };

  const blocked = !microphone
    ? { title: "No mic in this route", detail: "Add an Input device to test it here." }
    : !running
      ? {
          title: "Press Play to test this mic",
          detail: "The test listens to a playing route. It starts nothing itself.",
        }
      : privacyMuted
        ? { title: "Privacy mute is on", detail: "Turn it off to test this mic. Nothing can be heard while it is on." }
        : null;
  const text =
    phase.name === "listening"
      ? { title: "Listening for 5 seconds…", detail: "Say a few words normally." }
      : phase.name === "done" && !blocked
        ? MIC_VERDICT_TEXT[phase.verdict]
        : (blocked ?? { title: "Is your mic working?", detail: "Listens for 5 seconds and says what it found." });
  const verdict = phase.name === "done" && !blocked ? phase.verdict : null;
  // After the listen, show the loudest moment the verdict is about, not the meter now.
  const shownPeak = phase.name === "listening" ? peak : verdict && phase.name === "done" ? phase.loudest : null;
  const meterPercent = shownPeak === null ? 0 : Math.max(0, Math.min(100, ((shownPeak + 60) / 60) * 100));

  return (
    <section className="panel mic-test" aria-labelledby="mic-test-heading">
      <div className="mic-test-head">
        <h3 id="mic-test-heading">Mic test</h3>
        {microphones.length > 1 ? (
          <select
            aria-label="Mic to test"
            value={microphone?.id ?? ""}
            onChange={(event) => setChosen(event.target.value)}
            disabled={phase.name === "listening"}
          >
            {microphones.map((node) => (
              <option key={node.id} value={node.id}>
                {node.name}
              </option>
            ))}
          </select>
        ) : (
          <span className="muted mic-test-name">{microphone?.name ?? ""}</span>
        )}
      </div>
      <div className={`mic-test-result${verdict ? ` is-${verdict}` : ""}`} role="status">
        <Icon
          name={
            verdict === "good"
              ? "circle-check"
              : verdict
                ? "circle-alert"
                : phase.name === "listening"
                  ? "ear"
                  : "circle-dashed"
          }
          size={18}
        />
        <div>
          <strong>{text.title}</strong>
          <small>{text.detail}</small>
        </div>
      </div>
      <div className="mic-test-meter" aria-hidden="true">
        <i style={{ transform: `scaleX(${meterPercent / 100})` }} />
        <b />
      </div>
      <div className="mic-test-scale">
        <span>{formatPeak(shownPeak)}</span>
        <span>Voice zone: −18 to −6 dB</span>
      </div>
      <div className="actions mic-test-actions">
        {phase.name === "listening" ? (
          <button type="button" className="secondary" onClick={() => setPhase({ name: "idle" })}>
            Stop test
          </button>
        ) : (
          <button type="button" className="secondary" disabled={blocked !== null} onClick={start}>
            {phase.name === "done" ? "Test again" : "Test my mic"}
          </button>
        )}
        <button
          type="button"
          className="secondary"
          hidden={verdict !== "silent"}
          onClick={openSettings}
          title={settingsFailed ? "Open Sound settings from the Windows Start menu." : undefined}
        >
          Open Windows sound settings
        </button>
      </div>
    </section>
  );
}
