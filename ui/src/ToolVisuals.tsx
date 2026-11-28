/**
 * Visual editors for single-purpose tools. Each one shows what its settings
 * do and offers quick choices; exact values stay editable in the fields
 * below (and here, where a dedicated control replaces them).
 */
import {
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
  type RefObject,
} from "react";
import type { Node } from "@audiorouter/contracts";
import type { ProcessorResponse, UiBackend } from "./backend";
import { NumberField } from "./NumberField";
import { SliderInput } from "./Slider";
import { svgPoint } from "./SvgHandle";
import { formatParameterValue, parameterText } from "./parameterText";
import { nextFallingLevel, nextPeakHold, type PeakHold } from "./dynamics";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";

type Change = (name: string, value: number) => void;
type Band = {
  enabled: boolean;
  type: "peaking" | "lowShelf" | "highShelf";
  frequencyHz: number;
  gainDb: number;
  q: number;
};

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
const num = (node: Node, name: string, fallback: number) => {
  const value = node.parameters[name];
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
};
const signedDb = (value: number, digits = 1) =>
  `${value > 0 ? "+" : value < 0 ? "−" : ""}${Math.abs(value).toFixed(digits)} dB`;
const hzText = (hz: number) =>
  hz >= 1000 ? `${(hz / 1000).toFixed(hz >= 10000 ? 0 : 1).replace(/\.0$/, "")}k` : `${Math.round(hz)}`;

/* ------------------------------------------------------------------ */
/* Frequency graph frame shared by the EQ-type editors.               */

const W = 380,
  GL = 34,
  GR = 370,
  GT = 12;
const FREQ_TICKS = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];
const RESPONSE_FREQUENCIES = Array.from({ length: 96 }, (_, index) => 20 * Math.pow(1000, index / 95));
const xForHz = (hz: number) => GL + (Math.log10(clamp(hz, 20, 20000) / 20) / 3) * (GR - GL);
const hzForX = (x: number) => 20 * Math.pow(1000, clamp((x - GL) / (GR - GL), 0, 1));

function useFrequencyAxes(height: number, dbMin: number, dbMax: number) {
  const bottom = height - 20;
  const yForDb = (db: number) => GT + ((dbMax - clamp(db, dbMin, dbMax)) / (dbMax - dbMin)) * (bottom - GT);
  const dbForY = (y: number) => dbMax - clamp((y - GT) / (bottom - GT), 0, 1) * (dbMax - dbMin);
  const step = dbMax - dbMin > 30 ? 12 : 6;
  const dbTicks: number[] = [];
  for (let value = Math.ceil(dbMin / step) * step; value <= dbMax; value += step) dbTicks.push(value);
  const frame = (
    <>
      <rect x={GL} y={GT} width={GR - GL} height={bottom - GT} className="dynamics-plot" />
      {dbTicks.map((db) => (
        <g key={`db${db}`}>
          <line x1={GL} x2={GR} y1={yForDb(db)} y2={yForDb(db)} className={db === 0 ? "tool-zero" : "dynamics-grid"} />
          {/* The lowest level is left unlabelled: it sat on the "20" (Hz) in the corner. */}
          {db !== dbTicks[0] && (
            <text x={GL - 4} y={yForDb(db) + 3} textAnchor="end" className="dynamics-axis">
              {db > 0 ? `+${db}` : db}
            </text>
          )}
        </g>
      ))}
      {FREQ_TICKS.map((hz) => (
        <g key={`hz${hz}`}>
          <line x1={xForHz(hz)} x2={xForHz(hz)} y1={GT} y2={bottom} className="dynamics-grid" />
          <text x={xForHz(hz)} y={height - 7} textAnchor="middle" className="dynamics-axis">
            {hzText(hz)}
          </text>
        </g>
      ))}
    </>
  );
  return { bottom, yForDb, dbForY, frame };
}

/** Exact magnitude from the backend's DSP coefficient path, debounced while dragging. */
function useEqResponse(backend: UiBackend | null, connected: boolean, bands: Band[]) {
  const [response, setResponse] = useState<ProcessorResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const key = JSON.stringify(bands);
  useEffect(() => {
    if (!backend || !connected) {
      setResponse(null);
      return;
    }
    let live = true;
    const timer = window.setTimeout(() => {
      void backend
        .processorResponse({ sampleRateHz: 48000, frequenciesHz: RESPONSE_FREQUENCIES, bands })
        .then((value) => {
          if (live) {
            setResponse(value);
            setError(null);
          }
        })
        .catch((reason) => {
          if (live) {
            setResponse(null);
            setError(String(reason));
          }
        });
    }, 60);
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [backend, connected, key]);
  return { response, error };
}

function ResponseCurve({ response, yForDb }: { response: ProcessorResponse | null; yForDb: (db: number) => number }) {
  if (!response) return null;
  const points = response.frequenciesHz
    .map((hz, index) => `${xForHz(hz).toFixed(1)},${yForDb(response.magnitudeDb[index] ?? 0).toFixed(1)}`)
    .join(" ");
  return <polyline points={points} className="tool-response" />;
}

function Presets({
  label,
  presets,
  disabled,
  isCurrent,
  onApply,
}: {
  label: string;
  presets: Array<{ name: string; title: string }>;
  disabled: boolean;
  isCurrent: (index: number) => boolean;
  onApply: (index: number) => void;
}) {
  return (
    <div className="tool-presets" role="group" aria-label={label}>
      {presets.map((preset, index) => (
        <button
          key={preset.name}
          type="button"
          className={isCurrent(index) ? "primary" : "secondary"}
          aria-pressed={isCurrent(index)}
          title={preset.title}
          disabled={disabled}
          onClick={() => onApply(index)}
        >
          {preset.name}
        </button>
      ))}
    </div>
  );
}

function ToolPanel({
  label,
  title,
  summary,
  children,
}: {
  label: string;
  title: string;
  summary?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="tool-visual" aria-label={label}>
      <div className="dynamics-heading">
        <div>
          <strong>{title}</strong>
          {summary && <small>{summary}</small>}
        </div>
      </div>
      {children}
    </section>
  );
}

/* ------------------------------------------------------------------ */
/* Graphic EQ: ten faders under the exact response curve.             */

export const GRAPHIC_EQ_FREQUENCIES = [31.5, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
/** Band names shared with the Properties text: "31.5 Hz", "1 kHz". */
const bandLabel = (index: number) => parameterText("graphicEq", `band${index}Db`).label;
type PresetText = { name: MessageKey; title: MessageKey };
const GRAPHIC_PRESETS: Array<PresetText & { gains: number[] }> = [
  { name: "g5.tvPFlat", title: "g5.tvPFlatTitle", gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
  { name: "g5.tvPClarity", title: "g5.tvPClarityTitle", gains: [-6, -4, -2, -1, 0, 1, 3, 4, 2, 0] },
  { name: "g5.tvPWarm", title: "g5.tvPWarmTitle", gains: [0, 1, 3, 2, 0, -1, -1, 0, -1, -2] },
  { name: "g5.tvPCutRumble", title: "g5.tvPCutRumbleTitle", gains: [-12, -8, -3, 0, 0, 0, 0, 0, 0, 0] },
  { name: "g5.tvPLessHarsh", title: "g5.tvPLessHarshTitle", gains: [0, 0, 0, 0, 0, 0, -2, -4, -3, -1] },
];

export function GraphicEqEditor({
  node,
  backend,
  disabled,
  onChange,
}: {
  node: Node;
  backend: UiBackend | null;
  disabled: boolean;
  onChange: Change;
}) {
  const tr = useMessages();
  const gains = GRAPHIC_EQ_FREQUENCIES.map((_, index) => clamp(num(node, `band${index}Db`, 0), -18, 18));
  const [selected, setSelected] = useState(5);
  const bands: Band[] = GRAPHIC_EQ_FREQUENCIES.map((frequencyHz, index) => ({
    enabled: gains[index] !== 0,
    type: "peaking",
    frequencyHz,
    gainDb: gains[index],
    q: 1.4,
  }));
  const { response, error } = useEqResponse(backend, !disabled, bands);
  const { yForDb, frame } = useFrequencyAxes(150, -18, 18);
  const set = (index: number, value: number) => {
    const next = clamp(Math.round(value * 2) / 2, -18, 18);
    if (next !== gains[index]) onChange(`band${index}Db`, next);
  };
  return (
    <ToolPanel label={tr("g5.tvGraphicEditor")} title={tr("g5.tvGraphicTitle")} summary={tr("g5.tvGraphicHelp")}>
      <svg className="dynamics-graph" viewBox={`0 0 ${W} 150`} role="img" aria-label={tr("g5.tvGraphicResponse")}>
        {frame}
        {GRAPHIC_EQ_FREQUENCIES.map((hz, index) => (
          <circle
            key={hz}
            cx={xForHz(hz)}
            cy={yForDb(gains[index])}
            r={index === selected ? 4.5 : 3}
            className={`tool-band-dot${index === selected ? " is-selected" : ""}`}
          />
        ))}
        <ResponseCurve response={response} yForDb={yForDb} />
      </svg>
      {error && (
        <p className="muted" role="status">
          {tr("g5.tvRespUnavailable", { error })}
        </p>
      )}
      <div className="graphic-eq-faders">
        {GRAPHIC_EQ_FREQUENCIES.map((hz, index) => (
          <label key={hz} className={`graphic-eq-fader${index === selected ? " is-selected" : ""}`}>
            <span className="graphic-eq-value">
              {gains[index] > 0 ? "+" : ""}
              {gains[index]}
            </span>
            <SliderInput
              className="is-vertical"
              aria-label={tr("g5.tvBandAria", { band: bandLabel(index) })}
              aria-valuetext={signedDb(gains[index])}
              title={tr("g5.tvRestore0")}
              min={-18}
              max={18}
              step={0.5}
              value={gains[index]}
              disabled={disabled}
              onFocus={() => setSelected(index)}
              onPointerDown={() => setSelected(index)}
              onDoubleClick={() => set(index, 0)}
              onValue={(value) => set(index, value)}
            />
            <span className="graphic-eq-hz">{bandLabel(index).replace(" kHz", "k").replace(" Hz", "")}</span>
          </label>
        ))}
      </div>
      <label className="tool-exact">
        <span>{tr("g5.tvBandExact", { band: bandLabel(selected) })}</span>
        <NumberField
          aria-label={tr("g5.preciseValue", { name: bandLabel(selected) })}
          value={gains[selected]}
          min={-18}
          max={18}
          step={0.1}
          disabled={disabled}
          onValue={(value) => {
            const exact = clamp(value, -18, 18);
            if (exact !== gains[selected]) onChange(`band${selected}Db`, exact);
          }}
        />
      </label>
      <Presets
        label={tr("g5.tvGraphicPresets")}
        presets={GRAPHIC_PRESETS.map((preset) => ({ name: tr(preset.name), title: tr(preset.title) }))}
        disabled={disabled}
        isCurrent={(index) => GRAPHIC_PRESETS[index].gains.every((gain, band) => gain === gains[band])}
        onApply={(index) =>
          GRAPHIC_PRESETS[index].gains.forEach((gain, band) => {
            if (gain !== gains[band]) onChange(`band${band}Db`, gain);
          })
        }
      />
    </ToolPanel>
  );
}

/* ------------------------------------------------------------------ */
/* Bass & Treble: two shelves you can drag on the curve.              */

const TONE_PRESETS: Array<PresetText & { bassDb: number; trebleDb: number }> = [
  { name: "g5.tvPFlat", title: "g5.tvTFlatTitle", bassDb: 0, trebleDb: 0 },
  { name: "g5.tvTWarmer", title: "g5.tvTWarmerTitle", bassDb: 3, trebleDb: -1 },
  { name: "g5.tvTBrighter", title: "g5.tvTBrighterTitle", bassDb: 0, trebleDb: 3 },
  { name: "g5.tvTLessBoomy", title: "g5.tvTLessBoomyTitle", bassDb: -4, trebleDb: 0 },
  { name: "g5.tvTRadio", title: "g5.tvTRadioTitle", bassDb: -6, trebleDb: 4 },
];

function ShelfHandle({
  label,
  shortLabel,
  x,
  y,
  frequencyHz,
  gainDb,
  svg,
  disabled,
  onMove,
}: {
  label: string;
  /** The word drawn beside the handle. */
  shortLabel: string;
  x: number;
  y: number;
  frequencyHz: number;
  gainDb: number;
  svg: RefObject<SVGSVGElement | null>;
  disabled: boolean;
  onMove: (frequencyHz: number, gainDb: number, fromPoint?: { x: number; y: number }) => void;
}) {
  const tr = useMessages();
  const [dragging, setDragging] = useState(false);
  const key = (event: KeyboardEvent<SVGGElement>) => {
    const big = event.shiftKey;
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      onMove(frequencyHz, gainDb + (event.key === "ArrowUp" ? 1 : -1) * (big ? 3 : 0.5));
    }
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      onMove(frequencyHz * Math.pow(2, (event.key === "ArrowRight" ? 1 : -1) * (big ? 1 / 3 : 1 / 12)), gainDb);
    }
  };
  return (
    <g
      className={`dynamics-handle${dragging ? " is-dragging" : ""}`}
      role="slider"
      tabIndex={disabled ? -1 : 0}
      aria-label={label}
      aria-disabled={disabled || undefined}
      aria-valuemin={-12}
      aria-valuemax={12}
      aria-valuenow={gainDb}
      aria-valuetext={tr("g5.tvShelfValue", { db: signedDb(gainDb), hz: Math.round(frequencyHz) })}
      onPointerDown={(event) => {
        if (disabled || event.button !== 0) return;
        event.currentTarget.setPointerCapture?.(event.pointerId);
        setDragging(true);
      }}
      onPointerMove={(event: ReactPointerEvent<SVGGElement>) => {
        if (!dragging) return;
        const point = svgPoint(svg.current, event);
        if (point) onMove(0, 0, point);
      }}
      onPointerUp={(event) => {
        event.currentTarget.releasePointerCapture?.(event.pointerId);
        setDragging(false);
      }}
      onPointerCancel={() => setDragging(false)}
      onKeyDown={disabled ? undefined : key}
    >
      <title>{tr("g5.tvShelfTitle", { label, db: signedDb(gainDb), hz: Math.round(frequencyHz) })}</title>
      <circle cx={x} cy={y} r={13} className="dynamics-handle-hit" />
      <circle cx={x} cy={y} r={6} className="dynamics-handle-dot" />
      <text x={x} y={y - 10} textAnchor="middle" className="tool-handle-label">
        {shortLabel}
      </text>
    </g>
  );
}

export function BassTrebleEditor({
  node,
  backend,
  disabled,
  onChange,
}: {
  node: Node;
  backend: UiBackend | null;
  disabled: boolean;
  onChange: Change;
}) {
  const tr = useMessages();
  const svg = useRef<SVGSVGElement>(null);
  const bassDb = clamp(num(node, "bassDb", 0), -12, 12);
  const trebleDb = clamp(num(node, "trebleDb", 0), -12, 12);
  const bassHz = clamp(num(node, "bassFrequencyHz", 500), 80, 1000);
  const trebleHz = clamp(num(node, "trebleFrequencyHz", 1500), 800, 12000);
  const bands: Band[] = [
    { enabled: true, type: "lowShelf", frequencyHz: bassHz, gainDb: bassDb, q: 0.707 },
    { enabled: true, type: "highShelf", frequencyHz: trebleHz, gainDb: trebleDb, q: 0.707 },
  ];
  const { response, error } = useEqResponse(backend, !disabled, bands);
  const { yForDb, dbForY, frame } = useFrequencyAxes(170, -12, 12);
  const move =
    (prefix: "bass" | "treble", range: [number, number]) =>
    (frequencyHz: number, gainDb: number, point?: { x: number; y: number }) => {
      const hz = Math.round(clamp(point ? hzForX(point.x) : frequencyHz, range[0], range[1]));
      const db = clamp(Math.round((point ? dbForY(point.y) : gainDb) * 2) / 2, -12, 12);
      const currentHz = prefix === "bass" ? bassHz : trebleHz;
      const currentDb = prefix === "bass" ? bassDb : trebleDb;
      if (hz !== currentHz) onChange(`${prefix}FrequencyHz`, hz);
      if (db !== currentDb) onChange(`${prefix}Db`, db);
    };
  return (
    <ToolPanel label={tr("g5.tvBassEditor")} title={tr("g5.tvToneCurve")} summary={tr("g5.tvToneHelp")}>
      <svg
        ref={svg}
        className="dynamics-graph"
        viewBox={`0 0 ${W} 170`}
        role="group"
        aria-label={tr("g5.tvToneResponse")}
      >
        {frame}
        <rect x={GL} y={12} width={xForHz(bassHz) - GL} height={138} className="tool-zone is-bass" />
        <rect x={xForHz(trebleHz)} y={12} width={GR - xForHz(trebleHz)} height={138} className="tool-zone is-treble" />
        <ResponseCurve response={response} yForDb={yForDb} />
        <ShelfHandle
          label={tr("g5.tvBassShelf")}
          shortLabel={tr("g5.tvBassShort")}
          x={xForHz(bassHz)}
          y={yForDb(bassDb)}
          frequencyHz={bassHz}
          gainDb={bassDb}
          svg={svg}
          disabled={disabled}
          onMove={move("bass", [80, 1000])}
        />
        <ShelfHandle
          label={tr("g5.tvTrebleShelf")}
          shortLabel={tr("g5.tvTrebleShort")}
          x={xForHz(trebleHz)}
          y={yForDb(trebleDb)}
          frequencyHz={trebleHz}
          gainDb={trebleDb}
          svg={svg}
          disabled={disabled}
          onMove={move("treble", [800, 12000])}
        />
      </svg>
      {error && (
        <p className="muted" role="status">
          {tr("g5.tvRespUnavailable", { error })}
        </p>
      )}
      <p className="tool-readout">
        <span>
          {tr("g5.tvBassShort")} <b>{signedDb(bassDb)}</b>{" "}
          {tr("g5.tvBelow", { hz: formatParameterValue(bassHz, "Hz", 1) })}
        </span>
        <span>
          {tr("g5.tvTrebleShort")} <b>{signedDb(trebleDb)}</b>{" "}
          {tr("g5.tvAbove", { hz: formatParameterValue(trebleHz, "Hz", 1) })}
        </span>
      </p>
      <Presets
        label={tr("g5.tvTonePresets")}
        presets={TONE_PRESETS.map((preset) => ({ name: tr(preset.name), title: tr(preset.title) }))}
        disabled={disabled}
        isCurrent={(index) => TONE_PRESETS[index].bassDb === bassDb && TONE_PRESETS[index].trebleDb === trebleDb}
        onApply={(index) => {
          const preset = TONE_PRESETS[index];
          if (preset.bassDb !== bassDb) onChange("bassDb", preset.bassDb);
          if (preset.trebleDb !== trebleDb) onChange("trebleDb", preset.trebleDb);
        }}
      />
    </ToolPanel>
  );
}

/* ------------------------------------------------------------------ */
/* Dehum: the exact notch comb, from the DSP's harmonic coefficients.  */

/** Magnitude (dB) of the dehum filter chain at `hz`, mirroring `ParametricEq::new_dehum`. */
export function dehumMagnitudeDb(
  hz: number,
  fundamentalHz: number,
  amountPercent: number,
  harmonics: number,
  sampleRate = 48000,
): number {
  if (amountPercent <= 0) return 0;
  const centerGain = Math.pow(10, (-0.36 * amountPercent) / 20);
  const w = (2 * Math.PI * hz) / sampleRate;
  let total = 0;
  for (let harmonic = 1; harmonic <= harmonics; harmonic += 1) {
    const f = fundamentalHz * harmonic;
    if (f >= sampleRate * 0.45) break;
    const omega = (2 * Math.PI * f) / sampleRate;
    const alpha = Math.sin(omega) / (40 * harmonic);
    const a0 = 1 + alpha;
    const b = [(1 + centerGain * alpha) / a0, (-2 * Math.cos(omega)) / a0, (1 - centerGain * alpha) / a0];
    const a = [1, (-2 * Math.cos(omega)) / a0, (1 - alpha) / a0];
    const re = (c: number[]) => c[0] + c[1] * Math.cos(w) + c[2] * Math.cos(2 * w);
    const im = (c: number[]) => -(c[1] * Math.sin(w) + c[2] * Math.sin(2 * w));
    total += 10 * Math.log10((re(b) ** 2 + im(b) ** 2) / (re(a) ** 2 + im(a) ** 2));
  }
  return total;
}

export function DehumEditor({ node, disabled, onChange }: { node: Node; disabled: boolean; onChange: Change }) {
  const tr = useMessages();
  const fundamental = clamp(num(node, "frequencyHz", 60), 45, 65);
  const amount = clamp(num(node, "amountPercent", 50), 0, 100);
  const harmonics = clamp(Math.round(num(node, "harmonics", 4)), 1, 8);
  const height = 150,
    bottom = height - 20,
    top = 12,
    minHz = 30,
    maxHz = 600;
  const x = (hz: number) => GL + (Math.log10(hz / minHz) / Math.log10(maxHz / minHz)) * (GR - GL);
  const y = (db: number) => top + clamp(-db / 40, 0, 1) * (bottom - top);
  const points = Array.from({ length: 600 }, (_, index) => minHz * Math.pow(maxHz / minHz, index / 599))
    .map((hz) => `${x(hz).toFixed(1)},${y(dehumMagnitudeDb(hz, fundamental, amount, harmonics)).toFixed(1)}`)
    .join(" ");
  const ticks = [50, 100, 200, 400];
  return (
    <ToolPanel label={tr("g5.tvDehumEditor")} title={tr("g5.tvHumTitle")} summary={tr("g5.tvHumHelp")}>
      <div className="tool-presets" role="group" aria-label={tr("g5.tvMains")}>
        {[50, 60].map((hz) => (
          <button
            key={hz}
            type="button"
            className={fundamental === hz ? "primary" : "secondary"}
            aria-pressed={fundamental === hz}
            disabled={disabled}
            onClick={() => onChange("frequencyHz", hz)}
          >
            {hz} Hz
          </button>
        ))}
        <small>{tr("g5.tvMainsHelp")}</small>
      </div>
      <svg
        className="dynamics-graph"
        viewBox={`0 0 ${W} ${height}`}
        role="img"
        aria-label={tr("g5.tvHumAria", {
          db: (0.36 * amount).toFixed(0),
          hz: fundamental,
          count: harmonics - 1,
        })}
      >
        <rect x={GL} y={top} width={GR - GL} height={bottom - top} className="dynamics-plot" />
        {[0, -10, -20, -30, -40].map((db) => (
          <g key={db}>
            <line x1={GL} x2={GR} y1={y(db)} y2={y(db)} className={db === 0 ? "tool-zero" : "dynamics-grid"} />
            <text x={GL - 4} y={y(db) + 3} textAnchor="end" className="dynamics-axis">
              {db}
            </text>
          </g>
        ))}
        {ticks.map((hz) => (
          <text key={hz} x={x(hz)} y={height - 7} textAnchor="middle" className="dynamics-axis">
            {hz}
          </text>
        ))}
        {Array.from({ length: harmonics }, (_, index) => fundamental * (index + 1))
          .filter((hz) => hz <= maxHz)
          .map((hz) => (
            <line key={hz} x1={x(hz)} x2={x(hz)} y1={top} y2={bottom} className="tool-harmonic" />
          ))}
        <polyline points={points} className="tool-response" />
      </svg>
      <p className="tool-readout">
        <span>
          {tr("g5.tvDepth")} <b>{(0.36 * amount).toFixed(0)} dB</b> {tr("g5.tvDepthAt", { hz: fundamental })}
        </span>
        <span>
          {tr("g5.tvCuts")} <b>{harmonics}</b>
          {tr("g5.tvColon")}
          {Array.from({ length: Math.min(harmonics, 4) }, (_, index) => fundamental * (index + 1)).join(", ")}
          {harmonics > 4 ? " … Hz" : " Hz"}
        </span>
      </p>
    </ToolPanel>
  );
}

/* ------------------------------------------------------------------ */
/* Pitch: a keyboard showing where your voice moves.                  */

const PITCH_CHOICES: Array<{ semitones: number; name: string; title: MessageKey }> = [
  { semitones: -12, name: "−12", title: "g5.tvPc12Down" },
  { semitones: -5, name: "−5", title: "g5.tvPc5Down" },
  { semitones: -3, name: "−3", title: "g5.tvPc3Down" },
  { semitones: 0, name: "0", title: "g5.tvOriginal" },
  { semitones: 3, name: "+3", title: "g5.tvPc3Up" },
  { semitones: 5, name: "+5", title: "g5.tvPc5Up" },
  { semitones: 12, name: "+12", title: "g5.tvPc12Up" },
];
const WHITE = [0, 2, 4, 5, 7, 9, 11];

export function PitchEditor({ node, disabled, onChange }: { node: Node; disabled: boolean; onChange: Change }) {
  const tr = useMessages();
  const semitones = clamp(Math.round(num(node, "semitones", 0)), -12, 12);
  const cents = clamp(num(node, "cents", 0), -100, 100);
  const total = semitones + cents / 100;
  const ratio = Math.pow(2, total / 12);
  // Two octaves either side of a reference key, white keys only drawn as a strip.
  const keys = Array.from({ length: 25 }, (_, index) => index - 12);
  const whites = keys.filter((key) => WHITE.includes(((key % 12) + 12) % 12));
  const keyWidth = (GR - GL) / whites.length;
  const whiteX = (key: number) => GL + whites.indexOf(key) * keyWidth;
  const blackX = (key: number) => whiteX(key - 1) + keyWidth * 0.68;
  const target = Math.round(total);
  const deeper = total < 0;
  const size = Math.abs(total) >= 12 ? "octave" : Math.abs(total) >= 5 ? "clearly" : "slightly";
  const describe = tr(
    total === 0
      ? "g5.tvOriginal"
      : size === "octave"
        ? deeper
          ? "g5.tvDescOctaveDeeper"
          : "g5.tvDescOctaveHigher"
        : size === "clearly"
          ? deeper
            ? "g5.tvDescClearlyDeeper"
            : "g5.tvDescClearlyHigher"
          : deeper
            ? "g5.tvDescSlightlyDeeper"
            : "g5.tvDescSlightlyHigher",
  );
  return (
    <ToolPanel label={tr("g5.tvPitchEditor")} title={tr("g5.tvPitchTitle")} summary={tr("g5.tvPitchHelp")}>
      <div className="tool-bigvalue">
        <b>
          {total > 0 ? "+" : total < 0 ? "−" : ""}
          {Math.abs(total).toFixed(cents === 0 ? 0 : 2)}
        </b>
        <span>{tr("g5.tvSemitonesLine", { describe, ratio: ratio.toFixed(3) })}</span>
      </div>
      <svg
        className="dynamics-graph tool-keyboard"
        viewBox={`0 0 ${W} 74`}
        role="img"
        aria-label={tr("g5.tvKeyboardAria", { n: total.toFixed(2) })}
      >
        {whites.map((key) => (
          <rect
            key={key}
            x={whiteX(key) + 0.5}
            y={6}
            width={keyWidth - 1}
            height={56}
            rx={2}
            className={`tool-key-white${key === 0 ? " is-origin" : ""}${key === target && target !== 0 ? " is-target" : ""}`}
          />
        ))}
        {keys
          .filter((key) => !WHITE.includes(((key % 12) + 12) % 12))
          .map((key) => (
            <rect
              key={key}
              x={blackX(key)}
              y={6}
              width={keyWidth * 0.64}
              height={34}
              rx={2}
              className={`tool-key-black${key === target && target !== 0 ? " is-target" : ""}`}
            />
          ))}
        <text x={whiteX(0) + keyWidth / 2} y={56} textAnchor="middle" className="tool-key-label">
          {tr("g5.tvYou")}
        </text>
        {target !== 0 && (
          <text
            x={
              WHITE.includes(((target % 12) + 12) % 12)
                ? whiteX(target) + keyWidth / 2
                : blackX(target) + keyWidth * 0.32
            }
            y={72}
            textAnchor="middle"
            className="tool-key-label is-target"
          >
            {tr("g5.tvNew")}
          </text>
        )}
      </svg>
      <div className="tool-presets" role="group" aria-label={tr("g5.tvPitchChoices")}>
        {PITCH_CHOICES.map((choice) => (
          <button
            key={choice.semitones}
            type="button"
            title={tr(choice.title)}
            className={choice.semitones === semitones && cents === 0 ? "primary" : "secondary"}
            aria-pressed={choice.semitones === semitones && cents === 0}
            disabled={disabled}
            onClick={() => {
              if (choice.semitones !== semitones) onChange("semitones", choice.semitones);
              if (cents !== 0) onChange("cents", 0);
            }}
          >
            {choice.name}
          </button>
        ))}
      </div>
    </ToolPanel>
  );
}

/* ------------------------------------------------------------------ */
/* Delay: what the time means, for lip sync and echo.                 */

const DELAY_CHOICES = [0, 20, 33, 67, 100, 200, 500];
export function DelayEditor({ node, disabled, onChange }: { node: Node; disabled: boolean; onChange: Change }) {
  const tr = useMessages();
  const delayMs = clamp(num(node, "delayMs", 0), 0, 1000);
  const span = delayMs <= 100 ? 100 : delayMs <= 300 ? 300 : 1000;
  const x = (ms: number) => GL + clamp(ms / span, 0, 1) * (GR - GL);
  const pulse = (start: number, cls: string, y: number, label: string) => (
    <g>
      <rect
        x={x(start)}
        y={y}
        width={Math.max(3, x(start + span * 0.04) - x(start))}
        height={18}
        rx={3}
        className={cls}
      />
      <text
        x={x(start) + 6 > GR - 40 ? x(start) - 4 : x(start) + 8 + Math.max(3, x(start + span * 0.04) - x(start))}
        y={y + 13}
        textAnchor={x(start) + 6 > GR - 40 ? "end" : "start"}
        className="dynamics-axis"
      >
        {label}
      </text>
    </g>
  );
  return (
    <ToolPanel label={tr("g5.tvDelayEditor")} title={tr("g5.tvDelayTitle")} summary={tr("g5.tvDelayHelp")}>
      <svg
        className="dynamics-graph"
        viewBox={`0 0 ${W} 86`}
        role="img"
        aria-label={tr("g5.tvDelayAria", { ms: delayMs })}
      >
        <rect x={GL} y={8} width={GR - GL} height={58} className="dynamics-plot" />
        {[0, 0.25, 0.5, 0.75, 1].map((fraction) => (
          <g key={fraction}>
            <line x1={x(span * fraction)} x2={x(span * fraction)} y1={8} y2={66} className="dynamics-grid" />
            <text
              x={x(span * fraction)}
              y={80}
              textAnchor={fraction === 1 ? "end" : fraction === 0 ? "start" : "middle"}
              className="dynamics-axis"
            >
              {Math.round(span * fraction)} ms
            </text>
          </g>
        ))}
        {pulse(0, "tool-pulse is-dry", 14, tr("g5.tvSoundIn"))}
        {pulse(
          delayMs,
          "tool-pulse is-wet",
          42,
          delayMs === 0 ? tr("g5.tvOutNoDelay") : tr("g5.tvOutLater", { ms: Math.round(delayMs) }),
        )}
      </svg>
      <p className="tool-readout">
        <span>
          <b>{Math.round(delayMs * 48)}</b> {tr("g5.tvSamplesAt")}
        </span>
        <span>
          <b>{(delayMs / (1000 / 30)).toFixed(1)}</b> {tr("g5.tvFramesAt30")}{" "}
          <b>{(delayMs / (1000 / 60)).toFixed(1)}</b> {tr("g5.tvFramesAt60")}
        </span>
      </p>
      <div className="tool-presets" role="group" aria-label={tr("g5.tvDelayChoices")}>
        {DELAY_CHOICES.map((ms) => (
          <button
            key={ms}
            type="button"
            className={Math.round(delayMs) === ms ? "primary" : "secondary"}
            aria-pressed={Math.round(delayMs) === ms}
            disabled={disabled}
            title={ms === 33 ? tr("g5.tvOneFrame") : ms === 67 ? tr("g5.tvTwoFrames") : undefined}
            onClick={() => onChange("delayMs", ms)}
          >
            {ms} ms
          </button>
        ))}
      </div>
    </ToolPanel>
  );
}

/* ------------------------------------------------------------------ */
/* Volume and Gain: the same change in % and dB, on a level scale.    */

export function LevelEditor({ node, disabled, onChange }: { node: Node; disabled: boolean; onChange: Change }) {
  const tr = useMessages();
  const isVolume = node.kind === "volume";
  const percent = isVolume ? clamp(num(node, "percent", 100), 0, 200) : 100 * Math.pow(10, num(node, "gainDb", 0) / 20);
  const gainDb = isVolume ? (percent <= 0 ? -Infinity : 20 * Math.log10(percent / 100)) : num(node, "gainDb", 0);
  const reference = -18;
  const after = reference + gainDb;
  const x = (db: number) => GL + clamp((db + 60) / 66, 0, 1) * (GR - GL);
  const choices = isVolume
    ? [
        { label: tr("levelEditor.mute"), value: 0 },
        { label: "50 %", value: 50 },
        { label: "71 % (−3 dB)", value: 71 },
        { label: "100 %", value: 100 },
        { label: "141 % (+3 dB)", value: 141 },
        { label: "200 %", value: 200 },
      ]
    : [-12, -6, -3, 0, 3, 6, 12].map((db) => ({
        label: `${db > 0 ? "+" : db < 0 ? "−" : ""}${Math.abs(db)} dB`,
        value: db,
      }));
  const current = isVolume ? Math.round(percent) : Math.round(gainDb * 10) / 10;
  return (
    <ToolPanel
      label={tr(isVolume ? "levelEditor.volumeEditor" : "levelEditor.gainEditor")}
      title={tr(isVolume ? "parameterUi.labels.volume" : "parameterUi.labels.gain")}
      summary={tr("levelEditor.hint")}
    >
      <div className="tool-bigvalue">
        <b>{isVolume ? `${Math.round(percent)} %` : signedDb(gainDb)}</b>
        <span>
          {percent <= 0
            ? tr("levelEditor.silent")
            : isVolume
              ? `= ${signedDb(gainDb)}`
              : `= ${Math.round(percent)} % (×${(percent / 100).toFixed(2)})`}
        </span>
      </div>
      <svg
        className="dynamics-graph"
        viewBox={`0 0 ${W} 64`}
        role="img"
        aria-label={
          Number.isFinite(after)
            ? tr("levelEditor.graph", { input: reference, output: after.toFixed(1) })
            : tr("levelEditor.graphSilent", { input: reference })
        }
      >
        <rect x={GL} y={8} width={GR - GL} height={38} className="dynamics-plot" />
        <rect x={x(0)} y={8} width={GR - x(0)} height={38} className="dynamics-over" />
        {[-60, -48, -36, -24, -12, 0, 6].map((db) => (
          <g key={db}>
            <line x1={x(db)} x2={x(db)} y1={8} y2={46} className="dynamics-grid" />
            <text x={x(db)} y={58} textAnchor={db === 6 ? "end" : "middle"} className="dynamics-axis">
              {db}
            </text>
          </g>
        ))}
        <text x={GL - 4} y={20} textAnchor="end" className="dynamics-axis">
          {tr("levelEditor.in")}
        </text>
        <rect x={GL} y={12} width={x(reference) - GL} height={10} rx={2} className="tool-level is-before" />
        <text x={x(reference) + 4} y={20} className="dynamics-axis">
          {tr("levelEditor.voice")}
        </text>
        <text x={GL - 4} y={39} textAnchor="end" className="dynamics-axis">
          {tr("levelEditor.out")}
        </text>
        {Number.isFinite(after) && (
          <rect
            x={GL}
            y={31}
            width={Math.max(0, x(after) - GL)}
            height={10}
            rx={2}
            className={`tool-level${after > 0 ? " is-over" : " is-after"}`}
          />
        )}
        <text
          x={Number.isFinite(after) && x(after) > GR - 90 ? x(after) - 4 : (Number.isFinite(after) ? x(after) : GL) + 4}
          y={39}
          textAnchor={Number.isFinite(after) && x(after) > GR - 90 ? "end" : "start"}
          className={`dynamics-axis${after > 0 ? " tool-warning" : ""}`}
        >
          {Number.isFinite(after)
            ? `${after.toFixed(1)} dBFS${after > 0 ? ` — ${tr("levelEditor.clips")}` : ""}`
            : tr("levelEditor.silent")}
        </text>
      </svg>
      <div
        className="tool-presets"
        role="group"
        aria-label={tr(isVolume ? "levelEditor.volumeChoices" : "levelEditor.gainChoices")}
      >
        {choices.map((choice) => (
          <button
            key={choice.label}
            type="button"
            className={choice.value === current ? "primary" : "secondary"}
            aria-pressed={choice.value === current}
            disabled={disabled}
            onClick={() => onChange(isVolume ? "percent" : "gainDb", choice.value)}
          >
            {choice.label}
          </button>
        ))}
      </div>
    </ToolPanel>
  );
}

/* ------------------------------------------------------------------ */
/* Input Switch: two big source buttons, named after what feeds them. */

export function InputSwitchEditor({
  node,
  session,
  disabled,
  onChange,
}: {
  node: Node;
  session: {
    nodes: Node[];
    edges: Array<{ sourceNode: string; destinationNode: string; destinationPort: string; enabled: boolean }>;
  };
  disabled: boolean;
  onChange: (name: string, value: string) => void;
}) {
  const tr = useMessages();
  const selected = node.parameters.selected === "b" ? "b" : "a";
  const slow = node.parameters.fade === "slow";
  const sourceName = (port: "a" | "b") => {
    const edge = session.edges.find(
      (item) => item.enabled && item.destinationNode === node.id && item.destinationPort === port,
    );
    return edge
      ? (session.nodes.find((item) => item.id === edge.sourceNode)?.name ?? tr("g5.tvUnknownSource"))
      : tr("g5.tvNothingConnected");
  };
  return (
    <ToolPanel
      label={tr("g5.tvInputEditor")}
      title={tr("g5.tvInputTitle")}
      summary={tr("g5.tvSwitchSummary", { time: tr(slow ? "g5.tvTwoSeconds" : "g5.tvHalfSecond") })}
    >
      <div className="input-switch-choices" role="radiogroup" aria-label={tr("g5.tvActiveInput")}>
        {(["a", "b"] as const).map((port) => (
          <button
            key={port}
            type="button"
            role="radio"
            aria-checked={selected === port}
            className={`input-switch-choice${selected === port ? " is-active" : ""}`}
            disabled={disabled}
            onClick={() => {
              if (selected !== port) onChange("selected", port);
            }}
          >
            <b>{port.toUpperCase()}</b>
            <span>{sourceName(port)}</span>
            <small>{selected === port ? tr("g5.tvPlaying") : tr("g5.tvSilent")}</small>
          </button>
        ))}
      </div>
      <div className="tool-presets" role="group" aria-label={tr("g5.tvSwitchSpeed")}>
        <button
          type="button"
          className={!slow ? "primary" : "secondary"}
          aria-pressed={!slow}
          disabled={disabled}
          onClick={() => {
            if (slow) onChange("fade", "normal");
          }}
        >
          {tr("g5.tvFadeNormal")}
        </button>
        <button
          type="button"
          className={slow ? "primary" : "secondary"}
          aria-pressed={slow}
          disabled={disabled}
          onClick={() => {
            if (!slow) onChange("fade", "slow");
          }}
        >
          {tr("g5.tvFadeSlow")}
        </button>
      </div>
    </ToolPanel>
  );
}

export function OutputSwitchEditor({
  node,
  session,
  disabled,
  onChange,
}: {
  node: Node;
  session: {
    nodes: Node[];
    edges: Array<{ sourceNode: string; sourcePort: string; destinationNode: string; enabled: boolean }>;
  };
  disabled: boolean;
  onChange: (name: string, value: string) => void;
}) {
  const tr = useMessages();
  const selected = node.parameters.selected === "b" ? "b" : "a";
  const destinationName = (port: "a" | "b") => {
    const edge = session.edges.find((item) => item.enabled && item.sourceNode === node.id && item.sourcePort === port);
    return edge
      ? (session.nodes.find((item) => item.id === edge.destinationNode)?.name ?? tr("g5.tvUnknownDest"))
      : tr("g5.tvNothingConnected");
  };
  return (
    <ToolPanel label={tr("g5.tvOutputEditor")} title={tr("g5.tvOutputTitle")} summary={tr("g5.tvOutputHelp")}>
      <div className="input-switch-choices" role="radiogroup" aria-label={tr("g5.tvActiveOutput")}>
        {(["a", "b"] as const).map((port) => (
          <button
            key={port}
            type="button"
            role="radio"
            aria-checked={selected === port}
            className={`input-switch-choice${selected === port ? " is-active" : ""}`}
            disabled={disabled}
            onClick={() => {
              if (selected !== port) onChange("selected", port);
            }}
          >
            <b>{port.toUpperCase()}</b>
            <span>{destinationName(port)}</span>
            <small>{selected === port ? tr("g5.tvPlaying") : tr("g5.tvSilent")}</small>
          </button>
        ))}
      </div>
    </ToolPanel>
  );
}

/* ------------------------------------------------------------------ */
/* One-knob cleanup tools: named strengths beside the exact value.    */

const STRENGTHS: Record<
  string,
  {
    parameter: string;
    title: MessageKey;
    summary: MessageKey;
    choices: Array<{ name: MessageKey; value: number; title: MessageKey }>;
  }
> = {
  declick: {
    parameter: "thresholdPercent",
    title: "g5.tvClickTitle",
    summary: "g5.tvClickSummary",
    choices: [
      { name: "g5.tvClickGentle", value: 70, title: "g5.tvClickGentleTitle" },
      { name: "g5.tvClickNormal", value: 50, title: "g5.tvClickNormalTitle" },
      { name: "g5.tvClickStrong", value: 30, title: "g5.tvClickStrongTitle" },
    ],
  },
  speechDenoise: {
    parameter: "strengthPercent",
    title: "g5.tvNoiseTitle",
    summary: "g5.tvNoiseSummary",
    choices: [
      { name: "g5.tvNoiseLight", value: 40, title: "g5.tvNoiseLightTitle" },
      { name: "g5.tvNoiseMedium", value: 70, title: "g5.tvNoiseMediumTitle" },
      { name: "g5.tvNoiseStrong", value: 90, title: "g5.tvNoiseStrongTitle" },
    ],
  },
};

export function StrengthEditor({ node, disabled, onChange }: { node: Node; disabled: boolean; onChange: Change }) {
  const tr = useMessages();
  const spec = STRENGTHS[node.kind];
  if (!spec) return null;
  const value = clamp(num(node, spec.parameter, spec.choices[1].value), 0, 100);
  const title = tr(spec.title);
  return (
    <ToolPanel label={tr("g5.tvStrengthAria", { title })} title={title} summary={tr(spec.summary)}>
      <div className="tool-strength" aria-hidden="true">
        <span style={{ width: `${value}%` }} />
      </div>
      <div className="tool-presets" role="group" aria-label={tr("g5.tvStrengthChoices", { title })}>
        {spec.choices.map((choice) => (
          <button
            key={choice.value}
            type="button"
            title={tr(choice.title)}
            className={Math.round(value) === choice.value ? "primary" : "secondary"}
            aria-pressed={Math.round(value) === choice.value}
            disabled={disabled}
            onClick={() => onChange(spec.parameter, choice.value)}
          >
            {tr("g5.tvChoiceValue", { name: tr(choice.name), value: choice.value })}
          </button>
        ))}
      </div>
    </ToolPanel>
  );
}

/* ------------------------------------------------------------------ */
/* Live level strip for any playing node, with the Meter's ballistics. */

export function LiveLevelBar({ peakDb, rmsDb }: { peakDb: number; rmsDb: number }) {
  const tr = useMessages();
  const hold = useRef<PeakHold | null>(null);
  const bar = useRef<{ db: number; at: number } | null>(null);
  const now = performance.now();
  hold.current = nextPeakHold(hold.current, peakDb, now);
  bar.current = nextFallingLevel(bar.current, rmsDb, now);
  const percent = (db: number) => clamp(((db + 60) / 66) * 100, 0, 100);
  return (
    <div
      className="live-level"
      role="meter"
      aria-label={tr("g5.tvLiveLevel")}
      aria-valuemin={-60}
      aria-valuemax={6}
      aria-valuenow={clamp(rmsDb, -60, 6)}
      aria-valuetext={tr("g5.tvMeterText", {
        rms: rmsDb <= -110 ? tr("g5.tvSilentWord") : `${rmsDb.toFixed(1)} dBFS`,
        peak: peakDb <= -110 ? tr("g5.tvSilentWord") : `${peakDb.toFixed(1)} dBFS`,
      })}
    >
      <span
        className={`live-level-fill${hold.current.db > 0 ? " is-over" : hold.current.db > -6 ? " is-loud" : ""}`}
        style={{ width: `${percent(bar.current.db)}%` }}
      />
      <span className="live-level-peak" style={{ left: `${percent(hold.current.db)}%` }} />
      <span className="live-level-zero" style={{ left: `${percent(0)}%` }} />
    </div>
  );
}
