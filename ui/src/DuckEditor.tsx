import { useComparing } from "./compareHold";
import { PropertySection } from "./PropertySection";
import { BooleanField } from "./BooleanField";
import { useEffect, useMemo, useRef, useState } from "react";
import type { DiagnosticsSnapshot, Node, Session } from "@audiorouter/contracts";
import { Handle, type HandleSpec } from "./SvgHandle";
import { SliderField } from "./Slider";
import { nextFallingLevel, nextPeakHold, suggestGateThreshold, type LevelSample, type PeakHold } from "./dynamics";
import { useLastStatistics } from "./DynamicsEditor";
import { useMessages } from "./i18n/useMessages";
import { tx } from "./i18n";
import type { MessageKey } from "./i18n/generated/messageKeys";

type ProcessorTelemetry = NonNullable<DiagnosticsSnapshot["nodeTelemetry"][number]["processor"]>;
type Change = (name: string, value: number | string | boolean) => void;
type GameRound = NonNullable<DiagnosticsSnapshot["gameRound"]>;

/** Siege phases a round-following Duck can turn this audio down in, in game order, with the engine's defaults. */
export const ROUND_PHASES = [
  { name: "duckMenu", label: "g5.duckPhaseMenu", defaultOn: true },
  { name: "duckPrep", label: "g5.duckPhasePrep", defaultOn: true },
  { name: "duckAction", label: "g5.duckPhaseAction", defaultOn: false },
  { name: "duckBetweenRounds", label: "g5.duckPhaseBetween", defaultOn: true },
] as const satisfies readonly { name: string; label: MessageKey; defaultOn: boolean }[];
const phaseOn = (node: Node, phase: (typeof ROUND_PHASES)[number]) =>
  typeof node.parameters[phase.name] === "boolean" ? node.parameters[phase.name] === true : phase.defaultOn;
const PHASE_LABELS: Record<GameRound["phase"], MessageKey> = {
  unknown: "g5.duckPhUnknown",
  menu: "g5.duckPhMenu",
  prep: "g5.duckPhPrep",
  betweenRounds: "g5.duckPhBetween",
  action: "g5.duckPhAction",
};

/** Plain-language state of the Stats.cc feed for a round-following Duck. */
export function roundFeedStatus(round: GameRound | null | undefined, running: boolean): string {
  if (!running) return tx("g5.duckFeedIdle");
  if (!round || round.state === "off" || round.state === "connecting") return tx("g5.duckFeedConnecting");
  if (round.state === "unavailable")
    return round.feedConfigured === false ? tx("g5.duckFeedOff") : tx("g5.duckFeedUnavailable");
  if (round.state === "waitingForUpdate" || round.phase === "unknown") return tx("g5.duckFeedWaiting");
  return tx("g5.duckFeedPhase", { phase: tx(PHASE_LABELS[round.phase]) });
}

const HISTORY_MS = 8000;
const FLOOR = -120;
const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
const db = (value: number, digits = 0) =>
  value <= -110 ? "−∞" : `${value > 0 ? "+" : value < 0 ? "−" : ""}${Math.abs(value).toFixed(digits)}`;

export type DuckSettings = {
  keyNodeId: string;
  thresholdDb: number;
  amountDb: number;
  attackMs: number;
  holdMs: number;
  releaseMs: number;
};
export function duckSettings(node: Node): DuckSettings {
  const number = (name: string, fallback: number) => {
    const value = node.parameters[name];
    return typeof value === "number" && Number.isFinite(value) ? value : fallback;
  };
  return {
    keyNodeId: typeof node.parameters.keyNodeId === "string" ? node.parameters.keyNodeId : "",
    thresholdDb: number("thresholdDb", -35),
    amountDb: number("amountDb", 6),
    attackMs: number("attackMs", 20),
    holdMs: number("holdMs", 300),
    releaseMs: number("releaseMs", 500),
  };
}

/** Nodes that can trigger a Duck: anything producing audio, except the Duck itself and outputs. */
export function duckTriggerChoices(
  session: Pick<Session, "nodes">,
  duckId: string,
): { sources: Node[]; tools: Node[] } {
  const producing = session.nodes.filter(
    (node) => node.id !== duckId && node.ports.some((port) => port.direction === "output"),
  );
  const isSource = (node: Node) => !node.ports.some((port) => port.direction === "input");
  return { sources: producing.filter(isSource), tools: producing.filter((node) => !isSource(node)) };
}

/** Gain over time for a sentence of speech, mirroring the engine's per-block Duck. */
export function duckSketch(settings: DuckSettings, block = 128, rate = 48000) {
  const totalMs = 1600,
    talkStart = 150,
    talkEnd = 700;
  const points: Array<{ ms: number; gainDb: number }> = [];
  let gain = 0,
    hold = 0,
    ducking = false;
  for (let frame = 0; frame < (totalMs * rate) / 1000; frame += block) {
    const ms = (frame * 1000) / rate;
    const talking = ms >= talkStart && ms < talkEnd;
    if (talking) {
      ducking = true;
      hold = settings.holdMs * 0.001 * rate;
    } else if (hold > 0) hold = Math.max(0, hold - block);
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
    if (!active) {
      setHistory([]);
      return;
    }
    if (!telemetry?.inputLevelDb?.length) return;
    const now = performance.now();
    const sample: LevelSample = {
      at: now,
      inputDb: telemetry.inputLevelDb[0],
      outputDb: telemetry.outputLevelDb?.[0] ?? FLOOR,
      reductionDb: Math.max(0, ...telemetry.gainReductionDb),
      open: telemetry.gateOpen.some(Boolean),
    };
    setHistory((current) => [...current.filter((item) => now - item.at <= HISTORY_MS), sample]);
  }, [telemetry, active]);
  return history;
}

function useHeld(value: number | null) {
  const level = useRef<{ db: number; at: number } | null>(null);
  const peak = useRef<PeakHold | null>(null);
  if (value === null) {
    level.current = null;
    peak.current = null;
    return { level: null, peak: null };
  }
  const now = performance.now();
  level.current = nextFallingLevel(level.current, value, now);
  peak.current = nextPeakHold(peak.current, value, now);
  return { level: level.current.db, peak: peak.current.db };
}

function HBar({
  label,
  value,
  peak,
  min,
  max,
  marker,
  tone,
  text,
}: {
  label: string;
  value: number | null;
  peak?: number | null;
  min: number;
  max: number;
  marker?: number;
  tone: "trigger" | "duck";
  text: string;
}) {
  const percent = (v: number) => clamp(((v - min) / (max - min)) * 100, 0, 100);
  return (
    <div className={`duck-bar is-${tone}`}>
      <span className="duck-bar-label">{label}</span>
      <div
        className="duck-bar-track"
        role="meter"
        aria-label={label}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-valuenow={value ?? min}
        aria-valuetext={text}
      >
        {value !== null && <span className="duck-bar-fill" style={{ transform: `scaleX(${percent(value) / 100})` }} />}
        {peak !== undefined && peak !== null && (
          <span className="duck-bar-peak" style={{ left: `${percent(peak)}%` }} />
        )}
        {marker !== undefined && <span className="duck-bar-marker" style={{ left: `${percent(marker)}%` }} />}
      </div>
      <strong>{text}</strong>
    </div>
  );
}

export function DuckEditor({
  node,
  session,
  telemetry,
  running,
  disabled,
  onChange,
  gameRound = null,
}: {
  node: Node;
  session: Pick<Session, "nodes">;
  telemetry: ProcessorTelemetry | null;
  running: boolean;
  disabled: boolean;
  onChange: Change;
  gameRound?: GameRound | null;
}) {
  const tr = useMessages();
  const settings = duckSettings(node);
  const roundMode = node.parameters.trigger === "siegeRound";
  const choices = duckTriggerChoices(session, node.id);
  const trigger = roundMode ? null : (session.nodes.find((candidate) => candidate.id === settings.keyNodeId) ?? null);
  const active = running && node.enabled && !node.bypass && (roundMode || Boolean(trigger));
  const history = useHistory(telemetry, active);
  const latest = active ? (history.at(-1) ?? null) : null;
  const now = latest?.at ?? performance.now();
  const statistics = useLastStatistics(history);
  const triggerHeld = useHeld(latest ? latest.inputDb : null);
  const ducking = latest?.open ?? false;
  const reduction = latest?.reductionDb ?? 0;
  const svg = useRef<SVGSVGElement>(null);
  const L = 34,
    R = 372,
    T = 8,
    B = 112,
    ST = 122,
    SB = 150,
    MIN = -80;
  const x = (at: number) => R - clamp((now - at) / HISTORY_MS, 0, 1) * (R - L);
  const y = (value: number) => B - ((clamp(value, MIN, 0) - MIN) / -MIN) * (B - T);
  const stripY = (reductionDb: number) => ST + clamp(reductionDb / Math.max(settings.amountDb, 1), 0, 1) * (SB - ST);
  const triggerLine = history.map((item) => `${x(item.at).toFixed(1)},${y(item.inputDb).toFixed(1)}`).join(" ");
  const triggerArea =
    history.length > 1
      ? `${x(history[0].at).toFixed(1)},${B} ${triggerLine} ${x(history.at(-1)!.at).toFixed(1)},${B}`
      : "";
  const gainLine = history.map((item) => `${x(item.at).toFixed(1)},${stripY(item.reductionDb).toFixed(1)}`).join(" ");
  const thresholdHandle: HandleSpec = {
    name: "thresholdDb",
    label: tr("g5.duckTriggerLine"),
    value: settings.thresholdDb,
    min: -80,
    max: 0,
    step: 1,
    unit: "dBFS",
    cx: R - 8,
    cy: y(settings.thresholdDb),
    fromPoint: (point) => MIN + ((B - point.y) / (B - T)) * -MIN,
  };
  const sketch = useMemo(
    () => duckSketch(settings),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [settings.amountDb, settings.attackMs, settings.holdMs, settings.releaseMs],
  );
  const sx = (ms: number) => L + (ms / sketch.totalMs) * (R - L);
  const sy = (gainDb: number) => 20 + clamp(-gainDb / Math.max(settings.amountDb, 1), 0, 1) * 52;
  const suggestion = statistics ? suggestGateThreshold(statistics) : null;
  const comparing = useComparing(node.id);
  let status: string;
  let pill: { text: string; tone: "good" | "idle" | "work" } = { text: tr("g5.duckNotPlaying"), tone: "idle" };
  if (!node.enabled) {
    status = tr("g5.duckOffStatus");
    pill = { text: tr("g5.duckOff"), tone: "idle" };
  } else if (node.bypass) {
    status = tr("g5.duckBypassStatus");
    pill = { text: tr("g5.duckBypassed"), tone: "idle" };
  } else if (comparing) {
    status = tr("g5.duckComparingStatus");
    pill = { text: tr("g5.duckComparing"), tone: "idle" };
  } else if (roundMode) {
    status = roundFeedStatus(gameRound, running);
    if (running && reduction > 0.5)
      pill = { text: tr("g5.duckDuckingPill", { db: reduction.toFixed(1) }), tone: "work" };
    else if (running)
      pill =
        gameRound?.state === "connected"
          ? { text: tr("g5.duckFullVolume"), tone: "good" }
          : {
              text: gameRound?.state === "unavailable" ? tr("g5.duckNoStatsCc") : tr("g5.duckConnecting"),
              tone: "idle",
            };
  } else if (!trigger) {
    status = tr("g5.duckChooseTriggerStatus");
    pill = { text: tr("g5.duckNoTrigger"), tone: "idle" };
  } else if (!running) status = tr("g5.duckPressPlayStatus", { amount: settings.amountDb, name: trigger.name });
  else if (!telemetry?.inputLevelDb?.length) {
    status = tr("g5.duckNoReadingsStatus");
    pill = { text: tr("g5.duckNoReadings"), tone: "idle" };
  } else if (ducking) {
    status = tr("g5.duckActiveStatus", { name: trigger.name, db: reduction.toFixed(1) });
    pill = { text: tr("g5.duckDuckingPill", { db: reduction.toFixed(1) }), tone: "work" };
  } else {
    status = tr("g5.duckListeningStatus", { name: trigger.name });
    pill = { text: tr("g5.duckListening"), tone: "good" };
  }
  return (
    <section className="dynamics-editor duck-editor" aria-label={tr("g5.duckLiveView")}>
      <div className="dynamics-heading">
        <div>
          <strong>{tr("g5.duckHeading")}</strong>
          <small>{roundMode ? tr("g5.duckRoundHelp") : tr("g5.duckLevelHelp")}</small>
        </div>
        <span className={`dynamics-pill is-${pill.tone}`} role="status">
          {pill.text}
        </span>
      </div>
      <label className="duck-trigger-choice">
        <span>{tr("g5.duckTrigger")}</span>
        <select
          aria-label={tr("g5.duckTriggerAria")}
          value={roundMode ? "siegeRound" : "level"}
          disabled={disabled}
          onChange={(event) => onChange("trigger", event.target.value)}
        >
          <option value="level">{tr("g5.duckByLevel")}</option>
          <option value="siegeRound">{tr("g5.duckByRound")}</option>
        </select>
      </label>
      {roundMode ? (
        <fieldset className="duck-round-phases">
          <legend>{tr("g5.duckTurnDownDuring")}</legend>
          {ROUND_PHASES.map((phase) => (
            <BooleanField
              key={phase.name}
              label={tr(phase.label)}
              checked={phaseOn(node, phase)}
              disabled={disabled}
              onChange={(checked) => onChange(phase.name, checked)}
            />
          ))}
          <small className="muted">{tr("g5.duckUntickedHelp")}</small>
        </fieldset>
      ) : (
        <label className="duck-trigger-choice">
          <span>{tr("g5.duckTriggeredBy")}</span>
          <select
            aria-label={tr("g5.duckTriggeredBy")}
            value={trigger ? settings.keyNodeId : ""}
            disabled={disabled}
            onChange={(event) => onChange("keyNodeId", event.target.value)}
          >
            <option value="">{tr("g5.duckChooseSource")}</option>
            {choices.sources.length > 0 && (
              <optgroup label={tr("g5.duckSources")}>
                {choices.sources.map((choice) => (
                  <option key={choice.id} value={choice.id}>
                    {choice.name}
                  </option>
                ))}
              </optgroup>
            )}
            {choices.tools.length > 0 && (
              <optgroup label={tr("g5.duckTools")}>
                {choices.tools.map((choice) => (
                  <option key={choice.id} value={choice.id}>
                    {choice.name}
                  </option>
                ))}
              </optgroup>
            )}
          </select>
        </label>
      )}
      <div className="duck-bars">
        {!roundMode && (
          <HBar
            label={tr("g5.duckTrigger")}
            tone="trigger"
            value={triggerHeld.level}
            peak={triggerHeld.peak}
            min={-80}
            max={0}
            marker={settings.thresholdDb}
            text={latest ? `${db(latest.inputDb)} dB` : "—"}
          />
        )}
        <HBar
          label={tr("g5.duckTurnedDown")}
          tone="duck"
          value={latest ? reduction : null}
          min={0}
          max={Math.max(settings.amountDb, 1)}
          text={latest ? `−${reduction.toFixed(1)} dB` : "—"}
        />
      </div>
      {/* Round mode has no trigger level: show only the turned-down strip. */}
      <svg
        ref={svg}
        className="dynamics-graph duck-history"
        viewBox={roundMode ? "0 114 380 50" : "0 0 380 164"}
        role="group"
        aria-label={roundMode ? tr("g5.duckHistoryRound") : tr("g5.duckHistoryLevel")}
      >
        {!roundMode && (
          <>
            <rect x={L} y={T} width={R - L} height={B - T} className="dynamics-plot" />
            {/* No label at -80: it sat on the strip's "0" just below (spacing audit). */}
            {[-60, -40, -20, 0].map((value) => (
              <g key={value}>
                <line x1={L} x2={R} y1={y(value)} y2={y(value)} className="dynamics-grid" />
                <text x={L - 4} y={Math.max(17, y(value) + 3)} textAnchor="end" className="dynamics-axis">
                  {value}
                </text>
              </g>
            ))}
            {statistics && (
              <>
                <line x1={L} x2={R} y1={y(statistics.noiseDb)} y2={y(statistics.noiseDb)} className="dynamics-stat" />
                <text x={L + 4} y={y(statistics.noiseDb) - 3} className="dynamics-stat-label">
                  {tr("g5.duckRoomNoise")}
                </text>
                <line x1={L} x2={R} y1={y(statistics.voiceDb)} y2={y(statistics.voiceDb)} className="dynamics-stat" />
                <text x={L + 4} y={y(statistics.voiceDb) - 3} className="dynamics-stat-label">
                  {tr("g5.duckVoice")}
                </text>
              </>
            )}
            {triggerArea && <polygon points={triggerArea} className="duck-trigger-area" />}
            {history.length > 1 && <polyline points={triggerLine} className="duck-trigger-line" />}
            <line
              x1={L}
              x2={R}
              y1={y(settings.thresholdDb)}
              y2={y(settings.thresholdDb)}
              className="dynamics-threshold"
            />
          </>
        )}
        <rect x={L} y={ST} width={R - L} height={SB - ST} className="dynamics-plot" />
        {history.map((item, index) =>
          item.open ? (
            <rect
              key={item.at}
              x={x(item.at)}
              y={ST}
              width={Math.max(1, (history[index + 1] ? x(history[index + 1].at) : x(now)) - x(item.at))}
              height={SB - ST}
              className="duck-active-band"
            />
          ) : null,
        )}
        {history.length > 1 && <polyline points={gainLine} className="duck-gain-line" />}
        <text x={L - 4} y={ST + 8} textAnchor="end" className="dynamics-axis">
          0
        </text>
        <text x={L - 4} y={SB} textAnchor="end" className="dynamics-axis">
          −{settings.amountDb}
        </text>
        <text x={L} y={SB + 11} className="dynamics-axis">
          −8 s
        </text>
        <text x={(L + R) / 2} y={SB + 11} textAnchor="middle" className="dynamics-axis">
          {tr("g5.duckThisAudio")}
        </text>
        <text x={R} y={SB + 11} textAnchor="end" className="dynamics-axis">
          {tr("g5.duckNow")}
        </text>
        {!roundMode && (
          <Handle
            spec={thresholdHandle}
            svg={svg}
            disabled={disabled}
            onChange={(name, value) => onChange(name, value)}
          />
        )}
      </svg>
      <SliderField
        className="duck-amount"
        label={tr("g5.duckTurnDownBy")}
        name={tr("g5.duckAmount")}
        valueText={`−${settings.amountDb} dB`}
        value={settings.amountDb}
        min={0}
        max={40}
        step={1}
        unit="dB"
        defaultValue={6}
        disabled={disabled}
        onChange={(value) => onChange("amountDb", value)}
      />
      {!roundMode && (
        <div className="dynamics-suggestion">
          {/* Always rendered at a reserved size so the panel never jumps when a measurement arrives or lapses. */}
          {suggestion !== null && statistics ? (
            <span>
              {tr("g5.duckSuggestMeasured", {
                name: trigger?.name ?? tr("g5.duckTheTrigger"),
                noise: db(statistics.noiseDb),
                voice: db(statistics.voiceDb),
                level: db(suggestion),
              })}
            </span>
          ) : (
            <span className="muted">
              {tr("g5.duckSuggestIdle", { name: trigger?.name ?? tr("g5.duckTheTrigger") })}
            </span>
          )}
          <button
            type="button"
            className="secondary"
            disabled={disabled || suggestion === null || Math.abs(suggestion - settings.thresholdDb) < 0.5}
            onClick={() => suggestion !== null && onChange("thresholdDb", suggestion)}
          >
            {suggestion !== null ? tr("g5.duckUseDb", { db: db(suggestion) }) : tr("g5.duckUseSuggestion")}
          </button>
        </div>
      )}
      <p className="muted dynamics-status">{status}</p>
      {!roundMode && (
        <PropertySection kind="duck" title={tr("g5.duckSentence")}>
          <svg
            className="dynamics-graph dynamics-sketch"
            viewBox="0 0 380 100"
            role="img"
            aria-label={tr("g5.duckSketchAria", {
              attack: settings.attackMs,
              hold: settings.holdMs,
              release: settings.releaseMs,
            })}
          >
            <rect x={L} y={20} width={R - L} height={52} className="dynamics-plot" />
            <rect
              x={sx(sketch.talkStart)}
              y={20}
              width={sx(sketch.talkEnd) - sx(sketch.talkStart)}
              height={52}
              className="duck-sketch-talk"
            />
            <text
              x={(sx(sketch.talkStart) + sx(sketch.talkEnd)) / 2}
              y={14}
              textAnchor="middle"
              className="dynamics-axis"
            >
              {tr("g5.duckYouTalk")}
            </text>
            <text x={L - 4} y={23} textAnchor="end" className="dynamics-axis">
              0
            </text>
            <text x={L - 4} y={75} textAnchor="end" className="dynamics-axis">
              −{settings.amountDb}
            </text>
            <polyline
              points={sketch.points
                .map((point) => `${sx(point.ms).toFixed(1)},${sy(point.gainDb).toFixed(1)}`)
                .join(" ")}
              className="duck-gain-line"
            />
            <text x={sx(sketch.talkStart) + 3} y={86} className="dynamics-axis">
              {tr("g5.duckSketchAttack", { ms: settings.attackMs })}
            </text>
            <text x={sx(sketch.talkEnd) + 3} y={86} className="dynamics-axis">
              {tr("g5.duckSketchHold", { hold: settings.holdMs, release: settings.releaseMs })}
            </text>
          </svg>
          <small className="muted">{tr("g5.duckSketchHelp")}</small>
        </PropertySection>
      )}
    </section>
  );
}
