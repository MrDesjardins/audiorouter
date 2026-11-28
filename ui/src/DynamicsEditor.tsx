import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useComparing } from "./compareHold";
import { Handle, type HandleSpec } from "./SvgHandle";
import { PropertySection } from "./PropertySection";
import { useMessages } from "./i18n/useMessages";
import type { DiagnosticsSnapshot, Node } from "@audiorouter/contracts";
import {
  compressorOutputDb,
  compressorSettings,
  dynamicsControlSplit,
  dynamicsTimingSummary,
  gateOutputDb,
  gateSettings,
  levelStatistics,
  limiterOutputDb,
  limiterSettings,
  nextFallingLevel,
  nextPeakHold,
  QUIET_VOICE_DB,
  ratioForOutputAtFullScale,
  responseSketch,
  suggestCompressorThreshold,
  suggestGateThreshold,
  type DynamicsKind,
  type LevelSample,
  type PeakHold,
} from "./dynamics";

type ProcessorTelemetry = NonNullable<DiagnosticsSnapshot["nodeTelemetry"][number]["processor"]>;
type Change = (name: string, value: number) => void;

const HISTORY_MS = 8000;
const FLOOR = -120;
const AXES: Record<DynamicsKind, { min: number; max: number; grid: number }> = {
  compressor: { min: -60, max: 6, grid: 12 },
  gate: { min: -80, max: 0, grid: 10 },
  limiter: { min: -24, max: 6, grid: 6 },
};
const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
const db = (value: number, digits = 1) =>
  value <= -110 ? "−∞" : `${value > 0 ? "+" : value < 0 ? "−" : ""}${Math.abs(value).toFixed(digits)}`;
const maxOf = (values: number[] | undefined, channels: number) =>
  values && values.length ? Math.max(...values.slice(0, channels)) : null;

function useHistory(
  kind: DynamicsKind,
  telemetry: ProcessorTelemetry | null,
  channels: number,
  active: boolean,
): LevelSample[] {
  const [history, setHistory] = useState<LevelSample[]>([]);
  useEffect(() => {
    if (!active) {
      setHistory([]);
      return;
    }
    if (!telemetry) return;
    const now = performance.now();
    const sample: LevelSample = {
      at: now,
      inputDb: maxOf(telemetry.inputLevelDb, channels) ?? FLOOR,
      outputDb: maxOf(telemetry.outputLevelDb, channels) ?? FLOOR,
      reductionDb: maxOf(telemetry.gainReductionDb, channels) ?? 0,
      // Only a Gate has an open state; other tools report false here.
      open: kind === "gate" && telemetry.gateOpen.length ? telemetry.gateOpen.slice(0, channels).some(Boolean) : null,
    };
    setHistory((current) => [...current.filter((item) => now - item.at <= HISTORY_MS), sample]);
  }, [kind, telemetry, channels, active]);
  return history;
}

/**
 * Level statistics that keep the last good measurement while readings
 * continue, so the suggestion does not appear and vanish each time a pause
 * or a burst of talk briefly makes the estimate unusable.
 */
export function useLastStatistics(history: LevelSample[]) {
  const last = useRef<ReturnType<typeof levelStatistics>>(null);
  const live = useMemo(() => levelStatistics(history), [history]);
  if (!history.length) last.current = null;
  else if (live) last.current = live;
  return last.current;
}

/** Smooth a reading for display: instant rise, steady fall, optional held peak. */
function useBallistics(value: number | null) {
  const level = useRef<{ db: number; at: number } | null>(null);
  const peak = useRef<PeakHold | null>(null);
  const now = performance.now();
  if (value === null) {
    level.current = null;
    peak.current = null;
    return { level: null, peak: null };
  }
  level.current = nextFallingLevel(level.current, value, now);
  peak.current = nextPeakHold(peak.current, value, now);
  return { level: level.current.db, peak: peak.current.db };
}

function TransferCurve({
  kind,
  node,
  sample,
  trail,
  disabled,
  onChange,
}: {
  kind: DynamicsKind;
  node: Node;
  sample: LevelSample | null;
  trail: LevelSample[];
  disabled: boolean;
  onChange: Change;
}) {
  const tr = useMessages();
  const svg = useRef<SVGSVGElement>(null);
  const { min, max, grid } = AXES[kind];
  const L = 46,
    R = 214,
    T = 10,
    B = 190;
  const x = (value: number) => L + ((clamp(value, min, max) - min) / (max - min)) * (R - L);
  const y = (value: number) => B - ((clamp(value, min, max) - min) / (max - min)) * (B - T);
  const fromX = (px: number) => min + ((px - L) / (R - L)) * (max - min);
  const fromY = (py: number) => min + ((B - py) / (B - T)) * (max - min);
  const curve = (output: (level: number) => number) =>
    Array.from({ length: 121 }, (_, index) => min + (index * (max - min)) / 120)
      .map((level) => `${x(level).toFixed(1)},${y(output(level)).toFixed(1)}`)
      .join(" ");
  const ticks: number[] = [];
  for (let value = max - (max % grid); value >= min; value -= grid) ticks.push(value);
  let path: string;
  let handles: HandleSpec[];
  let guides: ReactNode;
  if (kind === "compressor") {
    const settings = compressorSettings(node);
    path = curve((level) => compressorOutputDb(level, settings));
    const ratioOut = settings.thresholdDb + (0 - settings.thresholdDb) / settings.ratio + settings.makeupDb;
    handles = [
      {
        name: "thresholdDb",
        label: tr("dynamicsUi.threshold"),
        value: settings.thresholdDb,
        min: -60,
        max: 0,
        step: 0.5,
        unit: "dBFS",
        cx: x(settings.thresholdDb),
        cy: y(compressorOutputDb(settings.thresholdDb, settings)),
        fromPoint: (p) => fromX(p.x),
      },
      {
        name: "ratio",
        label: tr("dynamicsUi.ratio"),
        value: settings.ratio,
        min: 1,
        max: 20,
        step: 0.1,
        unit: ": 1",
        cx: x(0),
        cy: y(ratioOut),
        className: "is-secondary",
        fromPoint: (p) => ratioForOutputAtFullScale(settings.thresholdDb, fromY(p.y) - settings.makeupDb),
      },
    ];
    guides = (
      <>
        <rect
          x={x(settings.thresholdDb)}
          y={T}
          width={R - x(settings.thresholdDb)}
          height={B - T}
          className="dynamics-zone"
        />
        <line x1={x(settings.thresholdDb)} x2={x(settings.thresholdDb)} y1={T} y2={B} className="dynamics-threshold" />
        <text x={R - 4} y={T + 14} textAnchor="end" className="dynamics-zone-label">
          {tr("dynamicsUi.turnedDown")}
        </text>
      </>
    );
  } else if (kind === "gate") {
    const settings = gateSettings(node);
    path = curve((level) => gateOutputDb(level, settings));
    const close = settings.thresholdDb - settings.hysteresisDb;
    handles = [
      {
        name: "thresholdDb",
        label: tr("dynamicsUi.thresholdOpens"),
        value: settings.thresholdDb,
        min: -80,
        max: 0,
        step: 0.5,
        unit: "dBFS",
        cx: x(settings.thresholdDb),
        cy: y(settings.thresholdDb),
        fromPoint: (p) => fromX(p.x),
      },
      {
        name: "hysteresisDb",
        label: tr("dynamicsUi.hysteresis"),
        value: settings.hysteresisDb,
        min: 0,
        max: 12,
        step: 0.5,
        unit: "dB",
        cx: x(close),
        cy: B,
        className: "is-secondary",
        fromPoint: (p) => settings.thresholdDb - fromX(p.x),
      },
    ];
    guides = (
      <>
        <rect x={L} y={T} width={x(settings.thresholdDb) - L} height={B - T} className="dynamics-zone" />
        <line x1={x(settings.thresholdDb)} x2={x(settings.thresholdDb)} y1={T} y2={B} className="dynamics-threshold" />
        {settings.hysteresisDb > 0 && (
          <line x1={x(close)} x2={x(close)} y1={T} y2={B} className="dynamics-threshold is-close" />
        )}
        <text x={L + 4} y={T + 10} className="dynamics-zone-label">
          {tr("dynamicsUi.turnedDown")}
        </text>
      </>
    );
  } else {
    const settings = limiterSettings(node);
    path = curve((level) => limiterOutputDb(level, settings));
    handles = [
      {
        name: "ceilingDb",
        label: tr("dynamicsUi.ceiling"),
        value: settings.ceilingDb,
        min: -12,
        max: 0,
        step: 0.1,
        unit: "dBFS",
        cx: x(settings.ceilingDb),
        cy: y(settings.ceilingDb),
        fromPoint: (p) => fromY(p.y),
      },
    ];
    guides = (
      <>
        <rect
          x={x(settings.ceilingDb)}
          y={T}
          width={R - x(settings.ceilingDb)}
          height={B - T}
          className="dynamics-zone"
        />
        <line x1={L} x2={R} y1={y(settings.ceilingDb)} y2={y(settings.ceilingDb)} className="dynamics-threshold" />
      </>
    );
  }
  const live = sample && sample.inputDb > min - 1;
  return (
    <svg
      ref={svg}
      className="dynamics-graph dynamics-transfer"
      viewBox="0 0 222 222"
      role="group"
      aria-label={tr("dynamicsUi.transferLabel")}
    >
      <rect x={L} y={T} width={R - L} height={B - T} className="dynamics-plot" />
      {guides}
      {ticks.map((value) => (
        <g key={value}>
          <line x1={x(value)} x2={x(value)} y1={T} y2={B} className="dynamics-grid" />
          <line x1={L} x2={R} y1={y(value)} y2={y(value)} className="dynamics-grid" />
          <text x={L - 4} y={Math.max(17, y(value) + 3)} textAnchor="end" className="dynamics-axis">
            {value}
          </text>
          {/* Across, every other number, and none in the corner the up axis already labels:
              at Properties width the full set ran together (spacing audit). */}
          {value % (2 * grid) === 0 && value !== min && (
            <text x={x(value)} y={B + 12} textAnchor="middle" className="dynamics-axis">
              {value}
            </text>
          )}
        </g>
      ))}
      {max > 0 && <rect x={L} y={T} width={R - L} height={y(0) - T} className="dynamics-over" />}
      <line x1={x(min)} y1={y(min)} x2={x(max)} y2={y(max)} className="dynamics-unity" />
      <polyline points={path} className="dynamics-curve" />
      {trail
        .filter((item) => item.inputDb > min - 1)
        .map((item, index, all) => (
          <circle
            key={item.at}
            cx={x(item.inputDb)}
            cy={y(item.outputDb)}
            r={2}
            className="dynamics-trail"
            opacity={((index + 1) / all.length) * 0.6}
          />
        ))}
      {live && (
        <circle cx={x(sample.inputDb)} cy={y(sample.outputDb)} r={5} className="dynamics-live-dot">
          <title>{tr("dynamicsUi.now", { input: db(sample.inputDb), output: db(sample.outputDb) })}</title>
        </circle>
      )}
      <text x={(L + R) / 2} y={B + 29} textAnchor="middle" className="dynamics-axis-title">
        {tr("dynamicsUi.axisIn")}
      </text>
      <text
        x={14}
        y={(T + B) / 2}
        textAnchor="middle"
        transform={`rotate(-90 14 ${(T + B) / 2})`}
        className="dynamics-axis-title"
      >
        {tr("dynamicsUi.axisOut")}
      </text>
      {handles.map((spec) => (
        <Handle key={spec.name} spec={spec} svg={svg} disabled={disabled} onChange={onChange} />
      ))}
    </svg>
  );
}

function VerticalMeter({
  label,
  value,
  peak,
  min,
  max,
  reduction = false,
  title,
}: {
  label: string;
  value: number | null;
  peak?: number | null;
  min: number;
  max: number;
  reduction?: boolean;
  title: string;
}) {
  const tr = useMessages();
  const percent = (v: number) => clamp(((v - min) / (max - min)) * 100, 0, 100);
  const shown = value === null ? null : reduction ? clamp(value, 0, max) : value;
  return (
    <div className={`dynamics-meter${reduction ? " is-reduction" : ""}`} title={title}>
      <div
        className="dynamics-meter-track"
        role="meter"
        aria-label={title}
        aria-valuemin={reduction ? 0 : min}
        aria-valuemax={max}
        aria-valuenow={shown ?? (reduction ? 0 : min)}
        aria-valuetext={shown === null ? tr("dynamicsUi.noReading") : `${db(reduction ? -shown : shown)} dB`}
      >
        {shown !== null &&
          (reduction ? (
            <span
              className="dynamics-meter-fill"
              style={{ top: 0, height: `${clamp((shown / max) * 100, 0, 100)}%` }}
            />
          ) : (
            <span className="dynamics-meter-fill" style={{ bottom: 0, height: `${percent(shown)}%` }} />
          ))}
        {!reduction && peak !== undefined && peak !== null && (
          <span className="dynamics-meter-peak" style={{ bottom: `${percent(peak)}%` }} />
        )}
      </div>
      <strong>{label}</strong>
      <span>{shown === null ? "—" : reduction ? `${shown.toFixed(1)}` : db(shown)}</span>
    </div>
  );
}

function LevelHistory({
  kind,
  node,
  history,
  now,
  statistics,
  disabled,
  onChange,
}: {
  kind: DynamicsKind;
  node: Node;
  history: LevelSample[];
  now: number;
  statistics: { noiseDb: number; voiceDb: number } | null;
  disabled: boolean;
  onChange: Change;
}) {
  const tr = useMessages();
  const svg = useRef<SVGSVGElement>(null);
  const { min, max, grid } = AXES[kind];
  const L = 34,
    R = 372,
    T = 8,
    B = 112,
    STRIP_T = 120,
    STRIP_B = 146;
  const x = (at: number) => R - clamp((now - at) / HISTORY_MS, 0, 1) * (R - L);
  const y = (value: number) => B - ((clamp(value, min, max) - min) / (max - min)) * (B - T);
  const fromY = (py: number) => min + ((B - py) / (B - T)) * (max - min);
  const line = (pick: (item: LevelSample) => number) =>
    history.map((item) => `${x(item.at).toFixed(1)},${y(pick(item)).toFixed(1)}`).join(" ");
  const inputArea =
    history.length > 1
      ? `${x(history[0].at).toFixed(1)},${B} ${line((item) => item.inputDb)} ${x(history.at(-1)!.at).toFixed(1)},${B}`
      : "";
  const reductionScale = kind === "gate" ? Math.max(gateSettings(node).rangeDb, 6) : 24;
  const ticks: number[] = [];
  for (let value = max - (max % grid); value >= min; value -= grid) ticks.push(value);
  const threshold =
    kind === "compressor"
      ? compressorSettings(node).thresholdDb
      : kind === "gate"
        ? gateSettings(node).thresholdDb
        : limiterSettings(node).ceilingDb;
  const thresholdSpec: HandleSpec =
    kind === "limiter"
      ? {
          name: "ceilingDb",
          label: tr("dynamicsUi.ceilingLine"),
          value: threshold,
          min: -12,
          max: 0,
          step: 0.1,
          unit: "dBFS",
          cx: R - 8,
          cy: y(threshold),
          fromPoint: (p) => fromY(p.y),
        }
      : {
          name: "thresholdDb",
          label: tr("dynamicsUi.thresholdLine"),
          value: threshold,
          min: kind === "gate" ? -80 : -60,
          max: 0,
          step: 0.5,
          unit: "dBFS",
          cx: R - 8,
          cy: y(threshold),
          fromPoint: (p) => fromY(p.y),
        };
  return (
    <svg
      ref={svg}
      className="dynamics-graph dynamics-history"
      viewBox="0 0 380 160"
      role="group"
      aria-label={tr("dynamicsUi.historyLabel")}
    >
      <rect x={L} y={T} width={R - L} height={B - T} className="dynamics-plot" />
      {ticks.map((value) => (
        <g key={value}>
          <line x1={L} x2={R} y1={y(value)} y2={y(value)} className="dynamics-grid" />
          {/* Every other number, and none at the bottom beside the strip's own "0": a label
              per grid line overlapped its neighbours at Properties width (spacing audit). */}
          {value % (2 * grid) === 0 && value !== min && (
            <text x={L - 4} y={Math.max(17, y(value) + 3)} textAnchor="end" className="dynamics-axis">
              {value}
            </text>
          )}
        </g>
      ))}
      {statistics && (
        <>
          <line x1={L} x2={R} y1={y(statistics.noiseDb)} y2={y(statistics.noiseDb)} className="dynamics-stat" />
          <text x={L + 4} y={y(statistics.noiseDb) - 3} className="dynamics-stat-label">
            {tr("dynamicsUi.roomNoise")}
          </text>
          <line x1={L} x2={R} y1={y(statistics.voiceDb)} y2={y(statistics.voiceDb)} className="dynamics-stat" />
          <text x={L + 4} y={y(statistics.voiceDb) - 3} className="dynamics-stat-label">
            {tr("dynamicsUi.voice")}
          </text>
        </>
      )}
      {inputArea && <polygon points={inputArea} className="dynamics-input-area" />}
      {history.length > 1 && <polyline points={line((item) => item.inputDb)} className="dynamics-input-line" />}
      {history.length > 1 && <polyline points={line((item) => item.outputDb)} className="dynamics-output-line" />}
      <line x1={L} x2={R} y1={y(threshold)} y2={y(threshold)} className="dynamics-threshold" />
      {kind === "gate" && gateSettings(node).hysteresisDb > 0 && (
        <line
          x1={L}
          x2={R}
          y1={y(threshold - gateSettings(node).hysteresisDb)}
          y2={y(threshold - gateSettings(node).hysteresisDb)}
          className="dynamics-threshold is-close"
        />
      )}
      <rect x={L} y={STRIP_T} width={R - L} height={STRIP_B - STRIP_T} className="dynamics-plot" />
      {history.map((item, index) => {
        const next = history[index + 1];
        const width = Math.max(1, (next ? x(next.at) : x(now)) - x(item.at));
        const height = clamp(item.reductionDb / reductionScale, 0, 1) * (STRIP_B - STRIP_T);
        return (
          <g key={item.at}>
            {item.open !== null && (
              <rect
                x={x(item.at)}
                y={STRIP_T}
                width={width}
                height={STRIP_B - STRIP_T}
                className={item.open ? "dynamics-open" : "dynamics-closed"}
              />
            )}
            {height > 0.2 && (
              <rect x={x(item.at)} y={STRIP_T} width={width} height={height} className="dynamics-reduction" />
            )}
          </g>
        );
      })}
      <text x={L - 4} y={STRIP_T + 9} textAnchor="end" className="dynamics-axis">
        0
      </text>
      <text x={L - 4} y={STRIP_B} textAnchor="end" className="dynamics-axis">
        −{reductionScale}
      </text>
      <text x={R - 4} y={STRIP_B + 11} textAnchor="end" className="dynamics-axis">
        {tr("dynamicsUi.axisNow")}
      </text>
      <text x={L} y={STRIP_B + 11} className="dynamics-axis">
        {tr("dynamicsUi.axisEightSeconds")}
      </text>
      <text x={(L + R) / 2} y={STRIP_B + 11} textAnchor="middle" className="dynamics-axis">
        {kind === "gate" ? tr("dynamicsUi.gateStrip") : tr("dynamicsUi.reductionStrip")}
      </text>
      <Handle spec={thresholdSpec} svg={svg} disabled={disabled} onChange={onChange} />
    </svg>
  );
}

function ResponseSketchView({ kind, node }: { kind: DynamicsKind; node: Node }) {
  const tr = useMessages();
  const sketch = useMemo(() => responseSketch(kind, node), [kind, node]);
  const L = 34,
    R = 372,
    T = 22,
    B = 84;
  const lowest = Math.min(...sketch.points.map((point) => point.gainDb), -1);
  const floor = Math.max(lowest * 1.15, -90);
  const x = (ms: number) => L + (ms / sketch.totalMs) * (R - L);
  const y = (gainDb: number) => T + clamp(gainDb / floor, 0, 1) * (B - T);
  const points = sketch.points.map((point) => `${x(point.ms).toFixed(1)},${y(point.gainDb).toFixed(1)}`).join(" ");
  const settings =
    kind === "compressor" ? compressorSettings(node) : kind === "gate" ? gateSettings(node) : limiterSettings(node);
  const attack =
    "attackMs" in settings
      ? tr("dynamicsUi.attack", { ms: settings.attackMs })
      : tr("dynamicsUi.lookahead", { ms: (settings as { lookaheadMs: number }).lookaheadMs });
  const tail =
    "holdMs" in settings
      ? tr("dynamicsUi.tailHold", { hold: settings.holdMs, release: settings.releaseMs })
      : tr("dynamicsUi.tailRelease", { ms: settings.releaseMs });
  return (
    <svg
      className="dynamics-graph dynamics-sketch"
      viewBox="0 0 380 102"
      role="img"
      aria-label={tr("dynamicsUi.sketchLabel", { attack, tail })}
    >
      <rect x={L} y={T} width={R - L} height={B - T} className="dynamics-plot" />
      <rect
        x={x(sketch.wordStartMs)}
        y={T}
        width={x(sketch.wordEndMs) - x(sketch.wordStartMs)}
        height={B - T}
        className="dynamics-word"
      />
      <text
        x={(x(sketch.wordStartMs) + x(sketch.wordEndMs)) / 2}
        y={T - 8}
        textAnchor="middle"
        className="dynamics-axis"
      >
        {kind === "gate" ? tr("dynamicsUi.wordVoice") : tr("dynamicsUi.wordLoud")}
      </text>
      <line x1={L} x2={R} y1={y(0)} y2={y(0)} className="dynamics-grid" />
      <text x={L - 4} y={y(0) + 3} textAnchor="end" className="dynamics-axis">
        0
      </text>
      <line x1={L} x2={R} y1={y(lowest)} y2={y(lowest)} className="dynamics-grid" />
      <text x={L - 4} y={y(lowest) + 3} textAnchor="end" className="dynamics-axis">
        {db(lowest, 0)}
      </text>
      <polyline points={points} className="dynamics-gain-line" />
      <text x={x(sketch.wordStartMs) + 3} y={B + 11} className="dynamics-axis">
        ↑ {attack}
      </text>
      <text x={x(sketch.wordEndMs) + 3} y={B + 11} className="dynamics-axis">
        ↑ {tail}
      </text>
    </svg>
  );
}

/**
 * Live, visual editor for Compressor, Gate and Limiter. The transfer curve
 * and history lines are draggable; every value stays editable as an exact
 * number in the fields below.
 */
export function DynamicsEditor({
  kind,
  node,
  telemetry,
  channels,
  running,
  disabled,
  onChange,
  mainControls,
  timingControls,
}: {
  kind: DynamicsKind;
  node: Node;
  telemetry: ProcessorTelemetry | null;
  channels: number;
  running: boolean;
  disabled: boolean;
  onChange: Change;
  /** The settings people change most, right under the curve. */
  mainControls?: ReactNode;
  /** The rest, in a section that shows their values while closed. */
  timingControls?: ReactNode;
}) {
  const tr = useMessages();
  const active = running && node.enabled && !node.bypass;
  const hasLevels = Boolean(telemetry?.inputLevelDb?.length);
  const history = useHistory(kind, hasLevels ? telemetry : null, channels, active);
  const latest = active && hasLevels ? (history.at(-1) ?? null) : null;
  const now = latest?.at ?? performance.now();
  const statistics = useLastStatistics(history);
  const input = useBallistics(latest?.inputDb ?? null);
  const output = useBallistics(latest?.outputDb ?? null);
  const reduction = latest ? latest.reductionDb : null;
  const trail = history.slice(-20, -1);
  const axes = AXES[kind];
  const comparing = useComparing(node.id);
  let status: string;
  let pill: { text: string; tone: "good" | "idle" | "work" } = { text: tr("dynamicsUi.notPlaying"), tone: "idle" };
  // The pill names why there is no live reading, not just "Not playing".
  if (!node.enabled) {
    status = tr("dynamicsUi.offStatus");
    pill = { text: tr("dynamicsUi.off"), tone: "idle" };
  } else if (node.bypass) {
    status = tr("dynamicsUi.bypassedStatus");
    pill = { text: tr("dynamicsUi.bypassed"), tone: "idle" };
  } else if (comparing) {
    status = tr("dynamicsUi.comparingStatus");
    pill = { text: tr("dynamicsUi.comparing"), tone: "idle" };
  } else if (!running) status = tr("dynamicsUi.pressPlay");
  else if (!telemetry) {
    status = tr("dynamicsUi.noReadingsStatus");
    pill = { text: tr("dynamicsUi.noReadings"), tone: "idle" };
  } else if (!hasLevels) {
    status = tr("dynamicsUi.oldBackendStatus");
    pill = { text: tr("dynamicsUi.oldBackend"), tone: "idle" };
  } else if (kind === "gate") {
    const open = latest?.open ?? false;
    pill = open
      ? { text: tr("dynamicsUi.open"), tone: "good" }
      : { text: tr("dynamicsUi.closed", { db: (reduction ?? 0).toFixed(0) }), tone: "work" };
    status = open ? tr("dynamicsUi.gateOpenStatus") : tr("dynamicsUi.gateClosedStatus");
  } else {
    const amount = reduction ?? 0;
    const amountText = amount.toFixed(1);
    pill =
      amount >= 0.5
        ? { text: tr("dynamicsUi.reducing", { db: amountText }), tone: "work" }
        : { text: tr("dynamicsUi.notReducing"), tone: "good" };
    status =
      kind === "limiter"
        ? amount >= 0.5
          ? tr("dynamicsUi.limiterHolding", { db: amountText })
          : tr("dynamicsUi.limiterUnder")
        : amount >= 0.5
          ? tr("dynamicsUi.compressorTurning", { db: amountText })
          : tr("dynamicsUi.compressorBelow");
  }
  const suggestion = (() => {
    if (!statistics || kind === "limiter") return null;
    if (kind === "gate") {
      const { hysteresisDb } = gateSettings(node);
      const value = suggestGateThreshold(statistics, hysteresisDb);
      return {
        value,
        why: tr("dynamicsUi.gateWhy", {
          close: db(value - hysteresisDb, 0),
          above: (value - hysteresisDb - statistics.noiseDb).toFixed(0),
        }),
      };
    }
    const settings = compressorSettings(node);
    const { thresholdDb, reductionDb } = suggestCompressorThreshold(statistics, settings.ratio);
    const why = tr(reductionDb < 5.5 ? "dynamicsUi.compressorWhyMore" : "dynamicsUi.compressorWhy", {
      db: reductionDb.toFixed(0),
      ratio: settings.ratio.toFixed(1),
    });
    return { value: thresholdDb, why, quiet: statistics.voiceDb < QUIET_VOICE_DB };
  })();
  const currentThreshold =
    kind === "gate"
      ? gateSettings(node).thresholdDb
      : kind === "compressor"
        ? compressorSettings(node).thresholdDb
        : null;
  return (
    <section
      className="dynamics-editor"
      aria-label={tr("dynamicsUi.liveView", {
        tool: tr(
          kind === "compressor"
            ? "dynamicsUi.kindCompressor"
            : kind === "gate"
              ? "dynamicsUi.kindGate"
              : "dynamicsUi.kindLimiter",
        ),
      })}
    >
      <div className="dynamics-heading">
        <div>
          <strong>{tr("dynamicsUi.liveResponse")}</strong>
          <small>{tr("dynamicsUi.dragHandles")}</small>
        </div>
        <span className={`dynamics-pill is-${pill.tone}`} role="status">
          {pill.text}
        </span>
      </div>
      <div className="dynamics-top">
        <TransferCurve
          kind={kind}
          node={node}
          sample={latest}
          trail={active ? trail : []}
          disabled={disabled}
          onChange={onChange}
        />
        <div className="dynamics-meters">
          <VerticalMeter
            label={tr("dynamicsUi.in")}
            value={input.level}
            peak={input.peak}
            min={axes.min}
            max={axes.max}
            title={tr("dynamicsUi.levelIn")}
          />
          <VerticalMeter
            label={kind === "gate" ? tr("dynamicsUi.down") : tr("dynamicsUi.gr")}
            value={reduction}
            min={0}
            max={kind === "gate" ? Math.max(gateSettings(node).rangeDb, 6) : 24}
            reduction
            title={kind === "gate" ? tr("dynamicsUi.gateDown") : tr("dynamicsUi.gainReduction")}
          />
          <VerticalMeter
            label={tr("dynamicsUi.out")}
            value={output.level}
            peak={output.peak}
            min={axes.min}
            max={axes.max}
            title={tr("dynamicsUi.levelOut")}
          />
        </div>
      </div>
      {mainControls && <div className="dynamics-main-controls">{mainControls}</div>}
      {currentThreshold !== null && (
        <div className="dynamics-suggestion">
          {/* An invisible copy of the longest text shares the grid cell, so the slot is the right height at any panel
              width and the panel never jumps when a measurement arrives, lapses or turns into the quiet-voice advice. */}
          <div
            className="dynamics-suggestion-text"
            data-reserve={`${tr("dynamicsUi.suggestionText", {
              noise: "−100",
              voice: "−100",
              value: "−100",
              why: tr("dynamicsUi.compressorWhyMore", { db: 10, ratio: "10.0" }),
            })}${kind === "compressor" ? tr("dynamicsUi.quietAdvice") : ""}`}
          >
            {suggestion && statistics ? (
              <span>
                {tr("dynamicsUi.suggestionText", {
                  noise: db(statistics.noiseDb, 0),
                  voice: db(statistics.voiceDb, 0),
                  value: db(suggestion.value, 0),
                  why: suggestion.why,
                })}
                {"quiet" in suggestion && suggestion.quiet && tr("dynamicsUi.quietAdvice")}
              </span>
            ) : (
              <span className="muted">{tr("dynamicsUi.suggestionPrompt")}</span>
            )}
          </div>
          <button
            type="button"
            className="secondary"
            disabled={disabled || !suggestion || Math.abs(suggestion.value - currentThreshold) < 0.5}
            onClick={() => suggestion && onChange("thresholdDb", suggestion.value)}
          >
            {suggestion ? tr("dynamicsUi.use", { db: db(suggestion.value, 0) }) : tr("dynamicsUi.useSuggestion")}
          </button>
        </div>
      )}
      <p className="muted dynamics-status">{status}</p>
      {timingControls && (
        <PropertySection
          kind={kind}
          title={tr(
            kind === "compressor"
              ? "dynamicsUi.timingKnee"
              : kind === "gate"
                ? "dynamicsUi.timingClosing"
                : "dynamicsUi.timing",
          )}
          sectionId={dynamicsControlSplit(kind).title}
          summary={dynamicsTimingSummary(kind, node)}
        >
          {timingControls}
        </PropertySection>
      )}
      <PropertySection kind={kind} title={tr("dynamicsUi.lastEight")} sectionId="Last 8 seconds">
        <LevelHistory
          kind={kind}
          node={node}
          history={active ? history : []}
          now={now}
          statistics={statistics}
          disabled={disabled}
          onChange={onChange}
        />
        <div className="dynamics-legend" aria-hidden="true">
          <span>
            <i className="dynamics-key is-input" />
            {tr("dynamicsUi.in")}
          </span>
          <span>
            <i className="dynamics-key is-output" />
            {tr("dynamicsUi.out")}
          </span>
          <span>
            <i className="dynamics-key is-threshold" />
            {kind === "limiter" ? tr("dynamicsUi.ceiling") : tr("dynamicsUi.threshold")}
          </span>
          {kind === "gate" && (
            <span>
              <i className="dynamics-key is-close" />
              {tr("dynamicsUi.closesBelow")}
            </span>
          )}
        </div>
      </PropertySection>
      <PropertySection kind={kind} title={tr("dynamicsUi.howWord")} sectionId="How one word is shaped">
        <ResponseSketchView kind={kind} node={node} />
        <small className="muted">{tr("dynamicsUi.wordNote")}</small>
      </PropertySection>
    </section>
  );
}
