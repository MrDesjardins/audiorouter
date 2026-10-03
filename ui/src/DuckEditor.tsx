import { useEffect, useMemo, useRef, useState } from "react";
import type { DiagnosticsSnapshot, Node, Session } from "@audiorouter/contracts";
import { Handle, type HandleSpec } from "./SvgHandle";
import { levelStatistics, nextFallingLevel, nextPeakHold, suggestGateThreshold, type LevelSample, type PeakHold } from "./dynamics";

type ProcessorTelemetry = NonNullable<DiagnosticsSnapshot["nodeTelemetry"][number]["processor"]>;
type Change = (name: string, value: number | string | boolean) => void;
type GameRound = NonNullable<DiagnosticsSnapshot["gameRound"]>;

/** Siege phases a round-following Duck can turn the game down in (action always plays at full volume). */
export const ROUND_PHASES = [
  { name: "duckMenu", label: "Menu and matchmaking" },
  { name: "duckPrep", label: "Planning, operator selection and preparation" },
  { name: "duckBetweenRounds", label: "Between rounds and results" },
] as const;
const PHASE_LABELS: Record<GameRound["phase"], string> = { unknown: "unknown", menu: "menu", prep: "preparation", betweenRounds: "between rounds", action: "action" };

/** Plain-language state of the Stats.cc feed for a round-following Duck. */
export function roundFeedStatus(round: GameRound | null | undefined, running: boolean): string {
  if (!running) return "Press Play: AudioRouter then connects to Stats.cc on this PC. Keep Stats.cc running while you play.";
  if (!round || round.state === "off" || round.state === "connecting") return "Connecting to Stats.cc…";
  if (round.state === "unavailable") return round.feedConfigured === false
    ? "Stats.cc’s game feed is off, so the game stays at full volume. Enable it once with examples\\integrations\\stats-cc-siege (npm run setup:stats), then restart Stats.cc."
    : "Stats.cc is not running or its feed is unavailable, so the game stays at full volume. AudioRouter keeps retrying.";
  if (round.state === "waitingForUpdate" || round.phase === "unknown") return "Connected to Stats.cc. Full volume until it reports the next game state.";
  return `Stats.cc: ${PHASE_LABELS[round.phase]}.`;
}

const HISTORY_MS = 8000;
const FLOOR = -120;
const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
const db = (value: number, digits = 0) => value <= -110 ? "−∞" : `${value > 0 ? "+" : value < 0 ? "−" : ""}${Math.abs(value).toFixed(digits)}`;

export type DuckSettings = { keyNodeId: string; thresholdDb: number; amountDb: number; attackMs: number; holdMs: number; releaseMs: number };
export function duckSettings(node: Node): DuckSettings {
  const number = (name: string, fallback: number) => { const value = node.parameters[name]; return typeof value === "number" && Number.isFinite(value) ? value : fallback; };
  return {
    keyNodeId: typeof node.parameters.keyNodeId === "string" ? node.parameters.keyNodeId : "",
    thresholdDb: number("thresholdDb", -35), amountDb: number("amountDb", 6), attackMs: number("attackMs", 20), holdMs: number("holdMs", 300), releaseMs: number("releaseMs", 500),
  };
}

/** Nodes that can trigger a Duck: anything producing audio, except the Duck itself and outputs. */
export function duckTriggerChoices(session: Pick<Session, "nodes">, duckId: string): { sources: Node[]; tools: Node[] } {
  const producing = session.nodes.filter((node) => node.id !== duckId && node.ports.some((port) => port.direction === "output"));
  const isSource = (node: Node) => !node.ports.some((port) => port.direction === "input");
  return { sources: producing.filter(isSource), tools: producing.filter((node) => !isSource(node)) };
}

/** Gain over time for a sentence of speech, mirroring the engine's per-block Duck. */
export function duckSketch(settings: DuckSettings, block = 128, rate = 48000) {
  const totalMs = 1600, talkStart = 150, talkEnd = 700;
  const points: Array<{ ms: number; gainDb: number }> = [];
  let gain = 0, hold = 0, ducking = false;
  for (let frame = 0; frame < totalMs * rate / 1000; frame += block) {
    const ms = frame * 1000 / rate;
    const talking = ms >= talkStart && ms < talkEnd;
    if (talking) { ducking = true; hold = settings.holdMs * 0.001 * rate; }
    else if (hold > 0) hold = Math.max(0, hold - block);
    else ducking = false;
    const target = ducking ? -settings.amountDb : 0;
    const time = target < gain ? settings.attackMs : settings.releaseMs;
    const c = Math.exp(-block / (Math.max(time, 0.1) * 0.001 * rate));
    gain = c * gain + (1 - c) * target;
    points.push({ ms, gainDb: gain });
  }
  return { points, totalMs, talkStart, talkEnd };
}

function useHistory(telemetry: ProcessorTelemetry | null, active: boolean) {
  const [history, setHistory] = useState<LevelSample[]>([]);
  useEffect(() => {
    if (!active) { setHistory([]); return; }
    if (!telemetry?.inputLevelDb?.length) return;
    const now = performance.now();
    const sample: LevelSample = { at: now, inputDb: telemetry.inputLevelDb[0], outputDb: telemetry.outputLevelDb?.[0] ?? FLOOR, reductionDb: Math.max(0, ...telemetry.gainReductionDb), open: telemetry.gateOpen.some(Boolean) };
    setHistory((current) => [...current.filter((item) => now - item.at <= HISTORY_MS), sample]);
  }, [telemetry, active]);
  return history;
}

function useHeld(value: number | null) {
  const level = useRef<{ db: number; at: number } | null>(null);
  const peak = useRef<PeakHold | null>(null);
  if (value === null) { level.current = null; peak.current = null; return { level: null, peak: null }; }
  const now = performance.now();
  level.current = nextFallingLevel(level.current, value, now);
  peak.current = nextPeakHold(peak.current, value, now);
  return { level: level.current.db, peak: peak.current.db };
}

function HBar({ label, value, peak, min, max, marker, tone, text }: { label: string; value: number | null; peak?: number | null; min: number; max: number; marker?: number; tone: "trigger" | "duck"; text: string }) {
  const percent = (v: number) => clamp((v - min) / (max - min) * 100, 0, 100);
  return <div className={`duck-bar is-${tone}`}>
    <span className="duck-bar-label">{label}</span>
    <div className="duck-bar-track" role="meter" aria-label={label} aria-valuemin={min} aria-valuemax={max} aria-valuenow={value ?? min} aria-valuetext={text}>
      {value !== null && <span className="duck-bar-fill" style={{ width: `${percent(value)}%` }} />}
      {peak !== undefined && peak !== null && <span className="duck-bar-peak" style={{ left: `${percent(peak)}%` }} />}
      {marker !== undefined && <span className="duck-bar-marker" style={{ left: `${percent(marker)}%` }} />}
    </div>
    <strong>{text}</strong>
  </div>;
}

export function DuckEditor({ node, session, telemetry, running, disabled, onChange, gameRound = null }: { node: Node; session: Pick<Session, "nodes">; telemetry: ProcessorTelemetry | null; running: boolean; disabled: boolean; onChange: Change; gameRound?: GameRound | null }) {
  const settings = duckSettings(node);
  const roundMode = node.parameters.trigger === "siegeRound";
  const choices = duckTriggerChoices(session, node.id);
  const trigger = roundMode ? null : session.nodes.find((candidate) => candidate.id === settings.keyNodeId) ?? null;
  const active = running && node.enabled && !node.bypass && (roundMode || Boolean(trigger));
  const history = useHistory(telemetry, active);
  const latest = active ? history.at(-1) ?? null : null;
  const now = latest?.at ?? performance.now();
  const statistics = useMemo(() => levelStatistics(history), [history]);
  const triggerHeld = useHeld(latest ? latest.inputDb : null);
  const ducking = latest?.open ?? false;
  const reduction = latest?.reductionDb ?? 0;
  const svg = useRef<SVGSVGElement>(null);
  const L = 34, R = 372, T = 8, B = 112, ST = 122, SB = 150, MIN = -80;
  const x = (at: number) => R - clamp((now - at) / HISTORY_MS, 0, 1) * (R - L);
  const y = (value: number) => B - (clamp(value, MIN, 0) - MIN) / -MIN * (B - T);
  const stripY = (reductionDb: number) => ST + clamp(reductionDb / Math.max(settings.amountDb, 1), 0, 1) * (SB - ST);
  const triggerLine = history.map((item) => `${x(item.at).toFixed(1)},${y(item.inputDb).toFixed(1)}`).join(" ");
  const triggerArea = history.length > 1 ? `${x(history[0].at).toFixed(1)},${B} ${triggerLine} ${x(history.at(-1)!.at).toFixed(1)},${B}` : "";
  const gainLine = history.map((item) => `${x(item.at).toFixed(1)},${stripY(item.reductionDb).toFixed(1)}`).join(" ");
  const thresholdHandle: HandleSpec = { name: "thresholdDb", label: "Trigger level line", value: settings.thresholdDb, min: -80, max: 0, step: 1, unit: "dBFS", cx: R - 8, cy: y(settings.thresholdDb), fromPoint: (point) => MIN + (B - point.y) / (B - T) * -MIN };
  const sketch = useMemo(() => duckSketch(settings), [settings.amountDb, settings.attackMs, settings.holdMs, settings.releaseMs]); // eslint-disable-line react-hooks/exhaustive-deps
  const sx = (ms: number) => L + ms / sketch.totalMs * (R - L);
  const sy = (gainDb: number) => 20 + clamp(-gainDb / Math.max(settings.amountDb, 1), 0, 1) * 52;
  const suggestion = statistics ? suggestGateThreshold(statistics) : null;
  let status: string;
  let pill: { text: string; tone: "good" | "idle" | "work" } = { text: "Not playing", tone: "idle" };
  if (!node.enabled) { status = "Off: the audio passes unchanged."; pill = { text: "Off", tone: "idle" }; }
  else if (node.bypass) { status = "Bypassed: the audio passes unchanged."; pill = { text: "Bypassed", tone: "idle" }; }
  else if (roundMode) {
    status = roundFeedStatus(gameRound, running);
    if (running && reduction > 0.5) pill = { text: `Ducking −${reduction.toFixed(1)} dB`, tone: "work" };
    else if (running) pill = gameRound?.state === "connected" ? { text: "Full volume", tone: "good" } : { text: gameRound?.state === "unavailable" ? "No Stats.cc" : "Connecting", tone: "idle" };
  }
  else if (!trigger) { status = "Choose which source or tool triggers the ducking, usually your microphone."; pill = { text: "No trigger", tone: "idle" }; }
  else if (!running) status = `Press Play and talk: this audio goes down by ${settings.amountDb} dB while ${trigger.name} is above the trigger level.`;
  else if (!telemetry?.inputLevelDb?.length) { status = "Playing, but this Duck sends no readings. Save the route if it has unsaved changes."; pill = { text: "No readings", tone: "idle" }; }
  else if (ducking) { status = `${trigger.name} is active: this audio is ${reduction.toFixed(1)} dB down.`; pill = { text: `Ducking −${reduction.toFixed(1)} dB`, tone: "work" }; }
  else { status = `Listening to ${trigger.name}. Talk above the orange line to duck this audio.`; pill = { text: "Listening", tone: "good" }; }
  return <section className="dynamics-editor duck-editor" aria-label="Duck live view">
    <div className="dynamics-heading">
      <div><strong>Ducking</strong><small>{roundMode ? "Turns this audio down by the Amount outside Siege rounds, and back up when a round starts." : "Turns this audio down while the trigger is loud. Drag the orange line to set the trigger level."}</small></div>
      <span className={`dynamics-pill is-${pill.tone}`} role="status">{pill.text}</span>
    </div>
    <label className="duck-trigger-choice"><span>Trigger</span>
      <select aria-label="Duck trigger" value={roundMode ? "siegeRound" : "level"} disabled={disabled} onChange={(event) => onChange("trigger", event.target.value)}>
        <option value="level">Audio level (another source or tool)</option>
        <option value="siegeRound">Siege round (Stats.cc)</option>
      </select>
    </label>
    {roundMode ? <fieldset className="duck-round-phases"><legend>Turn this audio down during</legend>
      {ROUND_PHASES.map((phase) => <label key={phase.name}><input type="checkbox" checked={node.parameters[phase.name] !== false} disabled={disabled} onChange={(event) => onChange(phase.name, event.target.checked)} />{phase.label}</label>)}
      <small className="muted">Rounds in action always play at full volume. If Stats.cc stops or reports nothing, the audio returns to full volume.</small>
    </fieldset> : <label className="duck-trigger-choice"><span>Triggered by</span>
      <select aria-label="Triggered by" value={trigger ? settings.keyNodeId : ""} disabled={disabled} onChange={(event) => onChange("keyNodeId", event.target.value)}>
        <option value="">Choose a source or tool…</option>
        {choices.sources.length > 0 && <optgroup label="Sources">{choices.sources.map((choice) => <option key={choice.id} value={choice.id}>{choice.name}</option>)}</optgroup>}
        {choices.tools.length > 0 && <optgroup label="Tools">{choices.tools.map((choice) => <option key={choice.id} value={choice.id}>{choice.name}</option>)}</optgroup>}
      </select>
    </label>}
    <div className="duck-bars">
      {!roundMode && <HBar label="Trigger" tone="trigger" value={triggerHeld.level} peak={triggerHeld.peak} min={-80} max={0} marker={settings.thresholdDb} text={latest ? `${db(latest.inputDb)} dB` : "—"} />}
      <HBar label="Turned down" tone="duck" value={latest ? reduction : null} min={0} max={Math.max(settings.amountDb, 1)} text={latest ? `−${reduction.toFixed(1)} dB` : "—"} />
    </div>
    {/* Round mode has no trigger level: show only the turned-down strip. */}
    <svg ref={svg} className="dynamics-graph duck-history" viewBox={roundMode ? "0 114 380 50" : "0 0 380 164"} role="group" aria-label={roundMode ? "Last 8 seconds: how far this audio is turned down" : "Last 8 seconds: trigger level and how far this audio is turned down"}>
      {!roundMode && <>
      <rect x={L} y={T} width={R - L} height={B - T} className="dynamics-plot" />
      {[-80, -60, -40, -20, 0].map((value) => <g key={value}><line x1={L} x2={R} y1={y(value)} y2={y(value)} className="dynamics-grid" /><text x={L - 4} y={y(value) + 3} textAnchor="end" className="dynamics-axis">{value}</text></g>)}
      {statistics && <>
        <line x1={L} x2={R} y1={y(statistics.noiseDb)} y2={y(statistics.noiseDb)} className="dynamics-stat" /><text x={L + 4} y={y(statistics.noiseDb) - 3} className="dynamics-stat-label">room noise</text>
        <line x1={L} x2={R} y1={y(statistics.voiceDb)} y2={y(statistics.voiceDb)} className="dynamics-stat" /><text x={L + 4} y={y(statistics.voiceDb) - 3} className="dynamics-stat-label">voice</text>
      </>}
      {triggerArea && <polygon points={triggerArea} className="duck-trigger-area" />}
      {history.length > 1 && <polyline points={triggerLine} className="duck-trigger-line" />}
      <line x1={L} x2={R} y1={y(settings.thresholdDb)} y2={y(settings.thresholdDb)} className="dynamics-threshold" />
      </>}
      <rect x={L} y={ST} width={R - L} height={SB - ST} className="dynamics-plot" />
      {history.map((item, index) => item.open ? <rect key={item.at} x={x(item.at)} y={ST} width={Math.max(1, (history[index + 1] ? x(history[index + 1].at) : x(now)) - x(item.at))} height={SB - ST} className="duck-active-band" /> : null)}
      {history.length > 1 && <polyline points={gainLine} className="duck-gain-line" />}
      <text x={L - 4} y={ST + 8} textAnchor="end" className="dynamics-axis">0</text>
      <text x={L - 4} y={SB} textAnchor="end" className="dynamics-axis">−{settings.amountDb}</text>
      <text x={L} y={SB + 11} className="dynamics-axis">−8 s</text>
      <text x={(L + R) / 2} y={SB + 11} textAnchor="middle" className="dynamics-axis">this audio (dB)</text>
      <text x={R} y={SB + 11} textAnchor="end" className="dynamics-axis">now</text>
      {!roundMode && <Handle spec={thresholdHandle} svg={svg} disabled={disabled} onChange={(name, value) => onChange(name, value)} />}
    </svg>
    <label className="param-row duck-amount"><span className="param-caption"><span>Turn down by</span><b aria-hidden="true">−{settings.amountDb} dB</b></span>
      <input type="range" aria-label="Duck amount" aria-valuetext={`−${settings.amountDb} dB`} min={0} max={40} step={1} value={settings.amountDb} disabled={disabled} onChange={(event) => onChange("amountDb", Number(event.target.value))} />
      <span className="duck-amount-scale" aria-hidden="true"><span>0 dB</span><span>−20</span><span>−40 dB</span></span>
    </label>
    {!roundMode && suggestion !== null && statistics && <div className="dynamics-suggestion">
      <span>Last 8 s of {trigger?.name ?? "the trigger"}: room noise ≈ {db(statistics.noiseDb)} dB, voice ≈ {db(statistics.voiceDb)} dB. Suggested trigger level {db(suggestion)} dB.</span>
      <button type="button" className="secondary" disabled={disabled || Math.abs(suggestion - settings.thresholdDb) < 0.5} onClick={() => onChange("thresholdDb", suggestion)}>Use {db(suggestion)} dB</button>
    </div>}
    <p className="muted dynamics-status">{status}</p>
    {!roundMode && <details className="dynamics-sketch-details" open>
      <summary>How one sentence is ducked</summary>
      <svg className="dynamics-graph dynamics-sketch" viewBox="0 0 380 100" role="img" aria-label={`Attack ${settings.attackMs} ms, hold ${settings.holdMs} ms, release ${settings.releaseMs} ms`}>
        <rect x={L} y={20} width={R - L} height={52} className="dynamics-plot" />
        <rect x={sx(sketch.talkStart)} y={20} width={sx(sketch.talkEnd) - sx(sketch.talkStart)} height={52} className="duck-sketch-talk" />
        <text x={(sx(sketch.talkStart) + sx(sketch.talkEnd)) / 2} y={14} textAnchor="middle" className="dynamics-axis">you talk</text>
        <text x={L - 4} y={23} textAnchor="end" className="dynamics-axis">0</text>
        <text x={L - 4} y={75} textAnchor="end" className="dynamics-axis">−{settings.amountDb}</text>
        <polyline points={sketch.points.map((point) => `${sx(point.ms).toFixed(1)},${sy(point.gainDb).toFixed(1)}`).join(" ")} className="duck-gain-line" />
        <text x={sx(sketch.talkStart) + 3} y={86} className="dynamics-axis">↑ attack {settings.attackMs} ms</text>
        <text x={sx(sketch.talkEnd) + 3} y={86} className="dynamics-axis">↑ hold {settings.holdMs} ms · release {settings.releaseMs} ms</text>
      </svg>
      <small className="muted">The line shows this audio's level change: 0 is unchanged, the bottom is fully ducked.</small>
    </details>}
  </section>;
}
