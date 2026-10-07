import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Handle, type HandleSpec } from "./SvgHandle";
import type { DiagnosticsSnapshot, Node } from "@audiorouter/contracts";
import {
  compressorOutputDb,
  compressorSettings,
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
  const svg = useRef<SVGSVGElement>(null);
  const { min, max, grid } = AXES[kind];
  const L = 34,
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
        label: "Threshold",
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
        label: "Ratio",
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
        <text x={x(settings.thresholdDb) + 4} y={T + 10} className="dynamics-zone-label">
          turned down
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
        label: "Threshold (opens)",
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
        label: "Hysteresis (closes this far below)",
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
          turned down
        </text>
      </>
    );
  } else {
    const settings = limiterSettings(node);
    path = curve((level) => limiterOutputDb(level, settings));
    handles = [
      {
        name: "ceilingDb",
        label: "Ceiling",
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
      viewBox="0 0 222 214"
      role="group"
      aria-label="Transfer curve: sound in (across) against sound out (up)"
    >
      <rect x={L} y={T} width={R - L} height={B - T} className="dynamics-plot" />
      {guides}
      {ticks.map((value) => (
        <g key={value}>
          <line x1={x(value)} x2={x(value)} y1={T} y2={B} className="dynamics-grid" />
          <line x1={L} x2={R} y1={y(value)} y2={y(value)} className="dynamics-grid" />
          <text x={L - 4} y={y(value) + 3} textAnchor="end" className="dynamics-axis">
            {value}
          </text>
          <text x={x(value)} y={B + 11} textAnchor="middle" className="dynamics-axis">
            {value}
          </text>
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
          <title>{`Now: in ${db(sample.inputDb)} dB, out ${db(sample.outputDb)} dB`}</title>
        </circle>
      )}
      <text x={(L + R) / 2} y={212} textAnchor="middle" className="dynamics-axis-title">
        in (dBFS)
      </text>
      <text
        x={8}
        y={(T + B) / 2}
        textAnchor="middle"
        transform={`rotate(-90 8 ${(T + B) / 2})`}
        className="dynamics-axis-title"
      >
        out
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
        aria-valuetext={shown === null ? "no reading" : `${db(reduction ? -shown : shown)} dB`}
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
          label: "Ceiling line",
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
          label: "Threshold line",
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
      aria-label="Last 8 seconds: level in, level out and gain reduction"
    >
      <rect x={L} y={T} width={R - L} height={B - T} className="dynamics-plot" />
      {ticks.map((value) => (
        <g key={value}>
          <line x1={L} x2={R} y1={y(value)} y2={y(value)} className="dynamics-grid" />
          <text x={L - 4} y={y(value) + 3} textAnchor="end" className="dynamics-axis">
            {value}
          </text>
        </g>
      ))}
      {statistics && (
        <>
          <line x1={L} x2={R} y1={y(statistics.noiseDb)} y2={y(statistics.noiseDb)} className="dynamics-stat" />
          <text x={L + 4} y={y(statistics.noiseDb) - 3} className="dynamics-stat-label">
            room noise
          </text>
          <line x1={L} x2={R} y1={y(statistics.voiceDb)} y2={y(statistics.voiceDb)} className="dynamics-stat" />
          <text x={L + 4} y={y(statistics.voiceDb) - 3} className="dynamics-stat-label">
            voice
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
        now
      </text>
      <text x={L} y={STRIP_B + 11} className="dynamics-axis">
        −8 s
      </text>
      <text x={(L + R) / 2} y={STRIP_B + 11} textAnchor="middle" className="dynamics-axis">
        {kind === "gate" ? "gate open (green) · turned down (red)" : "gain reduction (dB)"}
      </text>
      <Handle spec={thresholdSpec} svg={svg} disabled={disabled} onChange={onChange} />
    </svg>
  );
}

function ResponseSketchView({ kind, node }: { kind: DynamicsKind; node: Node }) {
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
      ? `attack ${settings.attackMs} ms`
      : `look-ahead ${(settings as { lookaheadMs: number }).lookaheadMs} ms`;
  const tail =
    "holdMs" in settings
      ? `hold ${settings.holdMs} ms · release ${settings.releaseMs} ms`
      : `release ${settings.releaseMs} ms`;
  return (
    <svg
      className="dynamics-graph dynamics-sketch"
      viewBox="0 0 380 102"
      role="img"
      aria-label={`How one spoken word is shaped: ${attack}, ${tail}`}
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
        a word ({kind === "gate" ? "voice" : "loud"})
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
}: {
  kind: DynamicsKind;
  node: Node;
  telemetry: ProcessorTelemetry | null;
  channels: number;
  running: boolean;
  disabled: boolean;
  onChange: Change;
}) {
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
  let status: string;
  let pill: { text: string; tone: "good" | "idle" | "work" } = { text: "Not playing", tone: "idle" };
  // The pill names why there is no live reading, not just "Not playing".
  if (!node.enabled) {
    status = "Off: enable this tool to process sound.";
    pill = { text: "Off", tone: "idle" };
  } else if (node.bypass) {
    status = "Bypassed: sound passes unchanged. Turn Bypass off to hear and see the effect.";
    pill = { text: "Bypassed", tone: "idle" };
  } else if (!running) status = "Press Play, then talk: the moving dot shows where your voice sits on the curve.";
  else if (!telemetry) {
    status =
      "Playing, but this tool sends no readings. Save the route if it has unsaved changes, and run only one AudioRouter window: a second window shows the first window's backend.";
    pill = { text: "No readings", tone: "idle" };
  } else if (!hasLevels) {
    status =
      "This backend reports gain reduction only. Close every AudioRouter window and start the current build to see live levels.";
    pill = { text: "Old backend", tone: "idle" };
  } else if (kind === "gate") {
    const open = latest?.open ?? false;
    pill = open ? { text: "Open", tone: "good" } : { text: `Closed −${(reduction ?? 0).toFixed(0)} dB`, tone: "work" };
    status = open
      ? "Gate open: your voice passes unchanged."
      : "Gate closed: the room is turned down between words. If words are cut, lower the threshold or raise Hold.";
  } else {
    const amount = reduction ?? 0;
    pill = amount >= 0.5 ? { text: `−${amount.toFixed(1)} dB`, tone: "work" } : { text: "Not reducing", tone: "good" };
    status =
      kind === "limiter"
        ? amount >= 0.5
          ? `Holding a peak down by ${amount.toFixed(1)} dB so it stays under the ceiling.`
          : "Peaks are under the ceiling; nothing is changed."
        : amount >= 0.5
          ? `Turning loud parts down by ${amount.toFixed(1)} dB. Around 3–6 dB on loud words sounds natural.`
          : "Below the threshold: no compression right now.";
  }
  const suggestion = (() => {
    if (!statistics || kind === "limiter") return null;
    if (kind === "gate") {
      const { hysteresisDb } = gateSettings(node);
      const value = suggestGateThreshold(statistics, hysteresisDb);
      return {
        value,
        why: `closes at ${db(value - hysteresisDb, 0)} dB, ${(value - hysteresisDb - statistics.noiseDb).toFixed(0)} dB above room noise`,
      };
    }
    const settings = compressorSettings(node);
    const { thresholdDb, reductionDb } = suggestCompressorThreshold(statistics, settings.ratio);
    const why = `about ${reductionDb.toFixed(0)} dB off your loud words at ${settings.ratio.toFixed(1)}:1${reductionDb < 5.5 ? "; raise Ratio for more" : ""}`;
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
      aria-label={`${kind === "compressor" ? "Compressor" : kind === "gate" ? "Gate" : "Limiter"} live view`}
    >
      <div className="dynamics-heading">
        <div>
          <strong>Live response</strong>
          <small>Drag the round handles or the orange line. Exact values are below.</small>
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
            label="In"
            value={input.level}
            peak={input.peak}
            min={axes.min}
            max={axes.max}
            title="Level entering the tool"
          />
          <VerticalMeter
            label={kind === "gate" ? "Down" : "GR"}
            value={reduction}
            min={0}
            max={kind === "gate" ? Math.max(gateSettings(node).rangeDb, 6) : 24}
            reduction
            title={kind === "gate" ? "How far the gate turns the sound down" : "Gain reduction"}
          />
          <VerticalMeter
            label="Out"
            value={output.level}
            peak={output.peak}
            min={axes.min}
            max={axes.max}
            title="Level leaving the tool"
          />
        </div>
      </div>
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
          In
        </span>
        <span>
          <i className="dynamics-key is-output" />
          Out
        </span>
        <span>
          <i className="dynamics-key is-threshold" />
          {kind === "limiter" ? "Ceiling" : "Threshold"}
        </span>
        {kind === "gate" && (
          <span>
            <i className="dynamics-key is-close" />
            Closes below
          </span>
        )}
      </div>
      {currentThreshold !== null && (
        <div className="dynamics-suggestion">
          {/* Always rendered at a reserved size so the panel never jumps when a measurement arrives or lapses. */}
          {suggestion && statistics ? (
            <span>
              Last 8 s: room noise ≈ {db(statistics.noiseDb, 0)} dB, voice ≈ {db(statistics.voiceDb, 0)} dB. Suggested
              threshold {db(suggestion.value, 0)} dB ({suggestion.why}).
              {"quiet" in suggestion &&
                suggestion.quiet &&
                " Your voice is quiet, so the threshold is low too: turning the microphone up (Windows input level, or a Gain before this tool) works better."}
            </span>
          ) : (
            <span className="muted">
              Suggested threshold: press Play and talk normally for a few seconds, with short pauses, to measure your
              voice and room noise.
            </span>
          )}
          <button
            type="button"
            className="secondary"
            disabled={disabled || !suggestion || Math.abs(suggestion.value - currentThreshold) < 0.5}
            onClick={() => suggestion && onChange("thresholdDb", suggestion.value)}
          >
            {suggestion ? `Use ${db(suggestion.value, 0)} dB` : "Use suggestion"}
          </button>
        </div>
      )}
      <p className="muted dynamics-status">{status}</p>
      <details className="dynamics-sketch-details" open>
        <summary>How one word is shaped by Attack{kind === "gate" ? ", Hold" : ""} and Release</summary>
        <ResponseSketchView kind={kind} node={node} />
        <small className="muted">
          The line shows how far the tool turns the sound down over time (0 = unchanged). It updates as you change the
          timing values.
        </small>
      </details>
    </section>
  );
}
