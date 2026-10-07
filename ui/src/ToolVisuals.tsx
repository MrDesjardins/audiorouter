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
import { svgPoint } from "./SvgHandle";
import { formatParameterValue, parameterText } from "./parameterText";
import { nextFallingLevel, nextPeakHold, type PeakHold } from "./dynamics";

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
          <text x={GL - 4} y={yForDb(db) + 3} textAnchor="end" className="dynamics-axis">
            {db > 0 ? `+${db}` : db}
          </text>
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
const GRAPHIC_PRESETS: Array<{ name: string; title: string; gains: number[] }> = [
  { name: "Flat", title: "Every band at 0 dB", gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
  { name: "Voice clarity", title: "Less rumble and mud, more presence", gains: [-6, -4, -2, -1, 0, 1, 3, 4, 2, 0] },
  { name: "Warm", title: "Fuller low-mids, softer top", gains: [0, 1, 3, 2, 0, -1, -1, 0, -1, -2] },
  { name: "Cut rumble", title: "Removes desk thumps and low hum", gains: [-12, -8, -3, 0, 0, 0, 0, 0, 0, 0] },
  { name: "Less harsh", title: "Tames sharp 2–8 kHz", gains: [0, 0, 0, 0, 0, 0, -2, -4, -3, -1] },
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
    <ToolPanel
      label="Graphic EQ editor"
      title="Ten-band equalizer"
      summary="Drag a fader up to boost or down to cut that band. Double-click a fader for 0 dB."
    >
      <svg
        className="dynamics-graph"
        viewBox={`0 0 ${W} 150`}
        role="img"
        aria-label="Combined frequency response of the ten bands"
      >
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
          Response unavailable: {error}
        </p>
      )}
      <div className="graphic-eq-faders">
        {GRAPHIC_EQ_FREQUENCIES.map((hz, index) => (
          <label key={hz} className={`graphic-eq-fader${index === selected ? " is-selected" : ""}`}>
            <span className="graphic-eq-value">
              {gains[index] > 0 ? "+" : ""}
              {gains[index]}
            </span>
            <input
              type="range"
              aria-label={`${bandLabel(index)} band`}
              aria-valuetext={signedDb(gains[index])}
              min={-18}
              max={18}
              step={0.5}
              value={gains[index]}
              disabled={disabled}
              onFocus={() => setSelected(index)}
              onPointerDown={() => setSelected(index)}
              onDoubleClick={() => set(index, 0)}
              onChange={(event) => set(index, Number(event.target.value))}
            />
            <span className="graphic-eq-hz">{bandLabel(index).replace(" kHz", "k").replace(" Hz", "")}</span>
          </label>
        ))}
      </div>
      <label className="tool-exact">
        <span>{bandLabel(selected)} band, exact (dB)</span>
        <NumberField
          aria-label={`${bandLabel(selected)} precise value`}
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
        label="Graphic EQ presets"
        presets={GRAPHIC_PRESETS}
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

const TONE_PRESETS: Array<{ name: string; title: string; bassDb: number; trebleDb: number }> = [
  { name: "Flat", title: "No change", bassDb: 0, trebleDb: 0 },
  { name: "Warmer", title: "+3 dB bass, −1 dB treble", bassDb: 3, trebleDb: -1 },
  { name: "Brighter", title: "+3 dB treble", bassDb: 0, trebleDb: 3 },
  { name: "Less boomy", title: "−4 dB bass", bassDb: -4, trebleDb: 0 },
  { name: "Radio voice", title: "−6 dB bass, +4 dB treble", bassDb: -6, trebleDb: 4 },
];

function ShelfHandle({
  label,
  x,
  y,
  frequencyHz,
  gainDb,
  svg,
  disabled,
  onMove,
}: {
  label: string;
  x: number;
  y: number;
  frequencyHz: number;
  gainDb: number;
  svg: RefObject<SVGSVGElement | null>;
  disabled: boolean;
  onMove: (frequencyHz: number, gainDb: number, fromPoint?: { x: number; y: number }) => void;
}) {
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
      aria-valuetext={`${signedDb(gainDb)} from ${Math.round(frequencyHz)} Hz`}
      onPointerDown={(event) => {
        if (disabled) return;
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
      <title>{`${label}: ${signedDb(gainDb)} from ${Math.round(frequencyHz)} Hz. Drag, or arrows: up/down level, left/right frequency (Shift for bigger steps).`}</title>
      <circle cx={x} cy={y} r={13} className="dynamics-handle-hit" />
      <circle cx={x} cy={y} r={6} className="dynamics-handle-dot" />
      <text x={x} y={y - 10} textAnchor="middle" className="tool-handle-label">
        {label.split(" ")[0]}
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
    <ToolPanel
      label="Bass and Treble editor"
      title="Tone curve"
      summary="Drag Bass or Treble: up and down changes the level, sideways changes how much of the voice it affects."
    >
      <svg
        ref={svg}
        className="dynamics-graph"
        viewBox={`0 0 ${W} 170`}
        role="group"
        aria-label="Bass and treble response"
      >
        {frame}
        <rect x={GL} y={12} width={xForHz(bassHz) - GL} height={138} className="tool-zone is-bass" />
        <rect x={xForHz(trebleHz)} y={12} width={GR - xForHz(trebleHz)} height={138} className="tool-zone is-treble" />
        <ResponseCurve response={response} yForDb={yForDb} />
        <ShelfHandle
          label="Bass shelf"
          x={xForHz(bassHz)}
          y={yForDb(bassDb)}
          frequencyHz={bassHz}
          gainDb={bassDb}
          svg={svg}
          disabled={disabled}
          onMove={move("bass", [80, 1000])}
        />
        <ShelfHandle
          label="Treble shelf"
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
          Response unavailable: {error}
        </p>
      )}
      <p className="tool-readout">
        <span>
          Bass <b>{signedDb(bassDb)}</b> below {formatParameterValue(bassHz, "Hz", 1)}
        </span>
        <span>
          Treble <b>{signedDb(trebleDb)}</b> above {formatParameterValue(trebleHz, "Hz", 1)}
        </span>
      </p>
      <Presets
        label="Tone presets"
        presets={TONE_PRESETS}
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
    <ToolPanel
      label="Dehum editor"
      title="Hum removal"
      summary="Narrow cuts at the mains frequency and its multiples. Your voice between them is untouched."
    >
      <div className="tool-presets" role="group" aria-label="Mains frequency">
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
        <small>50 Hz: Europe, Asia, Africa, Australia. 60 Hz: North America and parts of South America.</small>
      </div>
      <svg
        className="dynamics-graph"
        viewBox={`0 0 ${W} ${height}`}
        role="img"
        aria-label={`Cuts of up to ${(0.36 * amount).toFixed(0)} dB at ${fundamental} Hz and ${harmonics - 1} multiples`}
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
          Depth <b>{(0.36 * amount).toFixed(0)} dB</b> at {fundamental} Hz
        </span>
        <span>
          Cuts <b>{harmonics}</b>:{" "}
          {Array.from({ length: Math.min(harmonics, 4) }, (_, index) => fundamental * (index + 1)).join(", ")}
          {harmonics > 4 ? " … Hz" : " Hz"}
        </span>
      </p>
    </ToolPanel>
  );
}

/* ------------------------------------------------------------------ */
/* Pitch: a keyboard showing where your voice moves.                  */

const PITCH_CHOICES = [
  { semitones: -12, name: "−12", title: "One octave lower" },
  { semitones: -5, name: "−5", title: "A fourth lower: noticeably deeper" },
  { semitones: -3, name: "−3", title: "Slightly deeper" },
  { semitones: 0, name: "0", title: "Original pitch" },
  { semitones: 3, name: "+3", title: "Slightly higher" },
  { semitones: 5, name: "+5", title: "A fourth higher" },
  { semitones: 12, name: "+12", title: "One octave higher" },
];
const WHITE = [0, 2, 4, 5, 7, 9, 11];

export function PitchEditor({ node, disabled, onChange }: { node: Node; disabled: boolean; onChange: Change }) {
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
  const describe =
    total === 0
      ? "Original pitch"
      : `${Math.abs(total) >= 12 ? "An octave" : Math.abs(total) >= 5 ? "Clearly" : "Slightly"} ${total < 0 ? "deeper" : "higher"}`;
  return (
    <ToolPanel
      label="Pitch editor"
      title="Voice pitch"
      summary="Moves your voice up or down without changing its speed. 12 semitones is one octave."
    >
      <div className="tool-bigvalue">
        <b>
          {total > 0 ? "+" : total < 0 ? "−" : ""}
          {Math.abs(total).toFixed(cents === 0 ? 0 : 2)}
        </b>
        <span>
          semitones · {describe} · frequency ×{ratio.toFixed(3)}
        </span>
      </div>
      <svg
        className="dynamics-graph tool-keyboard"
        viewBox={`0 0 ${W} 74`}
        role="img"
        aria-label={`Keyboard: your voice moves from the marked key by ${total.toFixed(2)} semitones`}
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
          you
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
            ▲ new
          </text>
        )}
      </svg>
      <div className="tool-presets" role="group" aria-label="Pitch choices">
        {PITCH_CHOICES.map((choice) => (
          <button
            key={choice.semitones}
            type="button"
            title={choice.title}
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
    <ToolPanel
      label="Delay editor"
      title="Delay"
      summary="Holds the sound back. Use it to line your voice up with video, or to match another device."
    >
      <svg
        className="dynamics-graph"
        viewBox={`0 0 ${W} 86`}
        role="img"
        aria-label={`Original at 0 ms, delayed copy at ${delayMs} ms`}
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
        {pulse(0, "tool-pulse is-dry", 14, "sound in")}
        {pulse(
          delayMs,
          "tool-pulse is-wet",
          42,
          delayMs === 0 ? "out (no delay)" : `out, ${Math.round(delayMs)} ms later`,
        )}
      </svg>
      <p className="tool-readout">
        <span>
          <b>{Math.round(delayMs * 48)}</b> samples at 48 kHz
        </span>
        <span>
          <b>{(delayMs / (1000 / 30)).toFixed(1)}</b> video frames at 30 fps ·{" "}
          <b>{(delayMs / (1000 / 60)).toFixed(1)}</b> at 60 fps
        </span>
      </p>
      <div className="tool-presets" role="group" aria-label="Delay choices">
        {DELAY_CHOICES.map((ms) => (
          <button
            key={ms}
            type="button"
            className={Math.round(delayMs) === ms ? "primary" : "secondary"}
            aria-pressed={Math.round(delayMs) === ms}
            disabled={disabled}
            title={ms === 33 ? "One video frame at 30 fps" : ms === 67 ? "Two video frames at 30 fps" : undefined}
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
  const isVolume = node.kind === "volume";
  const percent = isVolume ? clamp(num(node, "percent", 100), 0, 200) : 100 * Math.pow(10, num(node, "gainDb", 0) / 20);
  const gainDb = isVolume ? (percent <= 0 ? -Infinity : 20 * Math.log10(percent / 100)) : num(node, "gainDb", 0);
  const reference = -18;
  const after = reference + gainDb;
  const x = (db: number) => GL + clamp((db + 60) / 66, 0, 1) * (GR - GL);
  const choices = isVolume
    ? [
        { label: "Mute", value: 0 },
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
      label={isVolume ? "Volume editor" : "Gain editor"}
      title={isVolume ? "Volume" : "Gain"}
      summary="Every 6 dB doubles or halves the level. Watch the result stay left of 0 dBFS to avoid clipping."
    >
      <div className="tool-bigvalue">
        <b>{isVolume ? `${Math.round(percent)} %` : signedDb(gainDb)}</b>
        <span>
          {percent <= 0
            ? "silent"
            : isVolume
              ? `= ${signedDb(gainDb)}`
              : `= ${Math.round(percent)} % (×${(percent / 100).toFixed(2)})`}
        </span>
      </div>
      <svg
        className="dynamics-graph"
        viewBox={`0 0 ${W} 64`}
        role="img"
        aria-label={`A typical voice at ${reference} dBFS comes out at ${Number.isFinite(after) ? after.toFixed(1) : "silence"} dBFS`}
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
          in
        </text>
        <rect x={GL} y={12} width={x(reference) - GL} height={10} rx={2} className="tool-level is-before" />
        <text x={x(reference) + 4} y={20} className="dynamics-axis">
          typical voice −18 dBFS
        </text>
        <text x={GL - 4} y={39} textAnchor="end" className="dynamics-axis">
          out
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
          {Number.isFinite(after) ? `${after.toFixed(1)} dBFS${after > 0 ? " — clips" : ""}` : "silent"}
        </text>
      </svg>
      <div className="tool-presets" role="group" aria-label={isVolume ? "Volume choices" : "Gain choices"}>
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
  const selected = node.parameters.selected === "b" ? "b" : "a";
  const slow = node.parameters.fade === "slow";
  const sourceName = (port: "a" | "b") => {
    const edge = session.edges.find(
      (item) => item.enabled && item.destinationNode === node.id && item.destinationPort === port,
    );
    return edge
      ? (session.nodes.find((item) => item.id === edge.sourceNode)?.name ?? "Unknown source")
      : "Nothing connected";
  };
  return (
    <ToolPanel
      label="Input Switch editor"
      title="Which input you hear"
      summary={`Switching crossfades over ${slow ? "2 seconds" : "half a second"}, so there is no click.`}
    >
      <div className="input-switch-choices" role="radiogroup" aria-label="Active input">
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
            <small>{selected === port ? "Playing" : "Silent"}</small>
          </button>
        ))}
      </div>
      <div className="tool-presets" role="group" aria-label="Switch speed">
        <button
          type="button"
          className={!slow ? "primary" : "secondary"}
          aria-pressed={!slow}
          disabled={disabled}
          onClick={() => {
            if (slow) onChange("fade", "normal");
          }}
        >
          Normal fade (0.5 s)
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
          Slow fade (2 s)
        </button>
      </div>
    </ToolPanel>
  );
}

/* ------------------------------------------------------------------ */
/* One-knob cleanup tools: named strengths beside the exact value.    */

const STRENGTHS: Record<
  string,
  { parameter: string; title: string; summary: string; choices: Array<{ name: string; value: number; title: string }> }
> = {
  declick: {
    parameter: "thresholdPercent",
    title: "Click removal",
    summary: "Lower thresholds catch more clicks; too low can soften consonants like “t” and “k”.",
    choices: [
      { name: "Gentle", value: 70, title: "Only obvious clicks" },
      { name: "Normal", value: 50, title: "Mouth and cable clicks (default)" },
      { name: "Strong", value: 30, title: "Busy clicks; listen for dull consonants" },
    ],
  },
  speechDenoise: {
    parameter: "strengthPercent",
    title: "Noise reduction around speech",
    summary: "Higher strength removes more background noise but can make your voice sound processed.",
    choices: [
      { name: "Light", value: 40, title: "Quiet room" },
      { name: "Medium", value: 70, title: "Fan or traffic (default)" },
      { name: "Strong", value: 90, title: "Loud background; may sound processed" },
    ],
  },
};

export function StrengthEditor({ node, disabled, onChange }: { node: Node; disabled: boolean; onChange: Change }) {
  const spec = STRENGTHS[node.kind];
  if (!spec) return null;
  const value = clamp(num(node, spec.parameter, spec.choices[1].value), 0, 100);
  return (
    <ToolPanel label={`${spec.title} strength`} title={spec.title} summary={spec.summary}>
      <div className="tool-strength" aria-hidden="true">
        <span style={{ width: `${value}%` }} />
      </div>
      <div className="tool-presets" role="group" aria-label={`${spec.title} strength choices`}>
        {spec.choices.map((choice) => (
          <button
            key={choice.name}
            type="button"
            title={choice.title}
            className={Math.round(value) === choice.value ? "primary" : "secondary"}
            aria-pressed={Math.round(value) === choice.value}
            disabled={disabled}
            onClick={() => onChange(spec.parameter, choice.value)}
          >
            {choice.name} · {choice.value} %
          </button>
        ))}
      </div>
    </ToolPanel>
  );
}

/* ------------------------------------------------------------------ */
/* Live level strip for any playing node, with the Meter's ballistics. */

export function LiveLevelBar({ peakDb, rmsDb }: { peakDb: number; rmsDb: number }) {
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
      aria-label="Live level"
      aria-valuemin={-60}
      aria-valuemax={6}
      aria-valuenow={clamp(rmsDb, -60, 6)}
      aria-valuetext={`RMS ${rmsDb <= -110 ? "silent" : `${rmsDb.toFixed(1)} dBFS`}, peak ${peakDb <= -110 ? "silent" : `${peakDb.toFixed(1)} dBFS`}`}
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
