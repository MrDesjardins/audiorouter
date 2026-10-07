import { createContext, useContext, useEffect, useMemo, useRef, useState, type PointerEvent } from "react";
import type { Node } from "@audiorouter/contracts";
import type { UiBackend, ProcessorResponse } from "./backend";
import { NumberField } from "./NumberField";

const BAND_COUNT = 16;
const WIDTH = 380;
const HEIGHT = 248;
const LEFT = 34;
const RIGHT = 366;
const TOP = 16;
const BOTTOM = 190;
const DB_MIN = -12;
const DB_MAX = 12;
const CALLOUT_LANES = [8, 27, 198, 217] as const;
const CALLOUT_RADIUS = 9;
const CALLOUT_GAP = CALLOUT_RADIUS * 2;
const COLORS = ["#52c7f7", "#a878f9", "#72a0ff", "#ff79b1", "#ff9c6b", "#ffd166", "#66dfc4", "#b3e676"];
const TICKS = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];
const RESPONSE_FREQUENCIES = Array.from({ length: 96 }, (_, index) => 20 * Math.pow(1000, index / 95));
const FILTERS = [
  ["peaking", "Peaking/Band"],
  ["lowShelf", "Low shelf"],
  ["highShelf", "High shelf"],
  ["lowPass", "Low pass"],
  ["highPass", "High pass"],
  ["notch", "Notch"],
  ["bandPass", "Band pass"],
  ["allPass", "All pass"],
] as const;
type FilterType = (typeof FILTERS)[number][0];
type Band = { index: number; enabled: boolean; type: FilterType; frequencyHz: number; gainDb: number; q: number };

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
const xForHz = (frequency: number) => LEFT + (Math.log10(clamp(frequency, 20, 20000) / 20) / 3) * (RIGHT - LEFT);
const hzForX = (x: number) => Math.round(20 * Math.pow(1000, clamp((x - LEFT) / (RIGHT - LEFT), 0, 1)));
const yForDb = (gain: number) => TOP + ((DB_MAX - clamp(gain, DB_MIN, DB_MAX)) / (DB_MAX - DB_MIN)) * (BOTTOM - TOP);
const dbForY = (y: number) =>
  Math.round((DB_MAX - clamp((y - TOP) / (BOTTOM - TOP), 0, 1) * (DB_MAX - DB_MIN)) * 10) / 10;

export type LiveSpectrum = { levelsDb: number[]; bandFrequenciesHz: number[] };
/** The live sound entering this EQ (backend spectrum telemetry); null when not playing. */
export const EqSpectrumContext = createContext<LiveSpectrum | null>(null);
const SPECTRUM_RANGE_DB = 60;

/** Next display ceiling: jumps up to a new peak, then falls 1 dB per update, never below -40 dB. */
export function nextSpectrumTop(previous: number | null, spectrum: LiveSpectrum): number {
  const loudest = Math.max(...spectrum.levelsDb) + 3;
  return Math.max(-40, loudest, previous === null ? loudest : previous - 1);
}

/** Area points for the live spectrum under a display ceiling (top of the plot). */
export function spectrumArea(spectrum: LiveSpectrum, top: number): string | null {
  const bands = spectrum.bandFrequenciesHz
    .map((hz, index) => ({ hz, db: spectrum.levelsDb[index] ?? -160 }))
    .filter(({ hz }) => hz >= 20 && hz <= 20000);
  if (bands.length < 2) return null;
  const y = (db: number) => BOTTOM - clamp((db - (top - SPECTRUM_RANGE_DB)) / SPECTRUM_RANGE_DB, 0, 1) * (BOTTOM - TOP);
  const points = bands.map(({ hz, db }) => `${xForHz(hz).toFixed(1)},${y(db).toFixed(1)}`);
  return `${xForHz(bands[0].hz).toFixed(1)},${BOTTOM} ${points.join(" ")} ${xForHz(bands[bands.length - 1].hz).toFixed(1)},${BOTTOM}`;
}

type LabelPoint = { index: number; x: number; y: number };
type PointCallout = LabelPoint & { labelX: number; labelY: number };

/** Place numbered labels in the chart gutters while keeping leaders short and labels apart. */
export function layoutPointCallouts(points: LabelPoint[]): PointCallout[] {
  const placed: PointCallout[] = [];
  const loads = CALLOUT_LANES.map(() => 0);
  const offsets = [
    0,
    ...Array.from({ length: 18 }, (_, index) => (index % 2 === 0 ? 1 : -1) * (Math.floor(index / 2) + 1) * CALLOUT_GAP),
  ];
  for (const point of [...points].sort((a, b) => a.x - b.x || a.index - b.index)) {
    const pointAboveCenter = point.y < (TOP + BOTTOM) / 2;
    let best: { labelX: number; labelY: number; score: number; lane: number } | undefined;
    CALLOUT_LANES.forEach((labelY, lane) => {
      for (const offset of offsets) {
        const labelX = clamp(point.x + offset, LEFT + CALLOUT_RADIUS, RIGHT - CALLOUT_RADIUS);
        if (placed.some((other) => Math.hypot(other.labelX - labelX, other.labelY - labelY) < CALLOUT_GAP)) continue;
        const oppositeSide = pointAboveCenter ? lane >= 2 : lane < 2;
        const score = Math.hypot(point.x - labelX, point.y - labelY) + loads[lane] * 2 + (oppositeSide ? 24 : 0);
        if (!best || score < best.score) best = { labelX, labelY, score, lane };
      }
    });
    if (best) {
      placed.push({ ...point, labelX: best.labelX, labelY: best.labelY });
      loads[best.lane] += 1;
    }
  }
  return placed;
}

function readBands(node: Node): Band[] {
  return Array.from({ length: BAND_COUNT }, (_, index) => {
    const key = `band${index}`;
    const type = node.parameters[`${key}Type`];
    return {
      index,
      enabled:
        node.parameters[`${key}Enabled`] === true ||
        (index === 0 && node.parameters[`${key}Enabled`] === undefined && node.parameters.frequencyHz !== undefined),
      type: FILTERS.some(([id]) => id === type) ? (type as FilterType) : "peaking",
      frequencyHz: Number(
        node.parameters[`${key}FrequencyHz`] ?? (index === 0 ? node.parameters.frequencyHz : undefined) ?? 1000,
      ),
      gainDb: Number(node.parameters[`${key}GainDb`] ?? (index === 0 ? node.parameters.gainDb : undefined) ?? 0),
      q: Number(node.parameters[`${key}Q`] ?? (index === 0 ? node.parameters.q : undefined) ?? 1),
    };
  });
}

export function AdvancedEqEditor({
  node,
  backend,
  connected,
  onChange,
}: {
  node: Node;
  backend: UiBackend;
  connected: boolean;
  onChange: (name: string, value: boolean | number | string) => void;
}) {
  const bands = readBands(node);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [response, setResponse] = useState<ProcessorResponse | null>(null);
  const [responseError, setResponseError] = useState<string | null>(null);
  const [drag, setDrag] = useState<{
    index: number;
    frequencyHz: number;
    gainDb: number;
    moved: boolean;
    startX: number;
    startY: number;
  } | null>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  // Live sound entering the EQ: the ceiling follows recent peaks so a shot
  // or footstep stands out against quieter moments.
  const spectrum = useContext(EqSpectrumContext);
  const spectrumTop = useRef<number | null>(null);
  const liveArea = useMemo(() => {
    if (!spectrum) {
      spectrumTop.current = null;
      return null;
    }
    spectrumTop.current = nextSpectrumTop(spectrumTop.current, spectrum);
    return spectrumArea(spectrum, spectrumTop.current);
  }, [spectrum]);
  const selected = bands[selectedIndex] ?? bands[0];
  const enabledCount = bands.filter((band) => band.enabled).length;

  useEffect(() => {
    setSelectedIndex(0);
    setDrag(null);
  }, [node.id]);
  useEffect(() => {
    if (!connected) {
      setResponse(null);
      return;
    }
    let live = true;
    const timer = window.setTimeout(() => {
      void backend
        .processorResponse({
          sampleRateHz: 48000,
          frequenciesHz: RESPONSE_FREQUENCIES,
          bands: bands.map(({ enabled, type, frequencyHz, gainDb, q }) => ({ enabled, type, frequencyHz, gainDb, q })),
        })
        .then((value) => {
          if (live) {
            setResponse(value);
            setResponseError(null);
          }
        })
        .catch((error) => {
          if (live) {
            setResponse(null);
            setResponseError(String(error));
          }
        });
    }, 90);
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [backend, connected, node.parameters]);

  const addPoint = (frequencyHz = 1000, gainDb = 0) => {
    if (!connected) return;
    const empty = bands.find((band) => !band.enabled);
    if (!empty) return;
    const key = `band${empty.index}`;
    onChange(`${key}Type`, "peaking");
    onChange(`${key}FrequencyHz`, clamp(frequencyHz, 20, 20000));
    onChange(`${key}GainDb`, clamp(gainDb, -24, 24));
    onChange(`${key}Q`, 1);
    onChange(`${key}Enabled`, true);
    setSelectedIndex(empty.index);
  };
  const pointerCoordinates = (event: { clientX: number; clientY: number }) => {
    const rect = svgRef.current!.getBoundingClientRect();
    return {
      x: ((event.clientX - rect.left) / rect.width) * WIDTH,
      y: ((event.clientY - rect.top) / rect.height) * HEIGHT,
    };
  };
  const movePoint = (event: PointerEvent<SVGSVGElement>) => {
    if (!drag) return;
    if (!drag.moved && Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY) < 5) return;
    const { x, y } = pointerCoordinates(event);
    const type = bands[drag.index].type;
    setDrag({
      ...drag,
      frequencyHz: hzForX(x),
      gainDb: type === "peaking" || type === "lowShelf" || type === "highShelf" ? dbForY(y) : 0,
      moved: true,
    });
  };
  const finishDrag = (event: PointerEvent<SVGSVGElement>) => {
    if (!drag) return;
    if (svgRef.current?.hasPointerCapture(event.pointerId)) svgRef.current.releasePointerCapture(event.pointerId);
    if (drag.moved) {
      onChange(`band${drag.index}FrequencyHz`, drag.frequencyHz);
      if (["peaking", "lowShelf", "highShelf"].includes(bands[drag.index].type))
        onChange(`band${drag.index}GainDb`, drag.gainDb);
    }
    setDrag(null);
  };
  const curve = response?.frequenciesHz
    .map((frequency, index) => `${xForHz(frequency).toFixed(1)},${yForDb(response.magnitudeDb[index] ?? 0).toFixed(1)}`)
    .join(" ");
  const visiblePoints = bands
    .filter((band) => band.enabled)
    .map((band) => {
      const moved = drag?.index === band.index ? drag : band;
      return {
        band,
        x: xForHz(moved.frequencyHz),
        y: yForDb(band.type === "peaking" || band.type === "lowShelf" || band.type === "highShelf" ? moved.gainDb : 0),
      };
    });
  const callouts = new Map(
    layoutPointCallouts(visiblePoints.map(({ band, x, y }) => ({ index: band.index, x, y }))).map((point) => [
      point.index,
      point,
    ]),
  );

  return (
    <section className="advanced-eq" aria-label="Advanced EQ editor">
      <div className="advanced-eq-heading">
        <div>
          <strong>Frequency response</strong>
          <small>
            {enabledCount} of {BAND_COUNT} points active
          </small>
        </div>
        <button
          type="button"
          className="secondary"
          onClick={() => addPoint()}
          disabled={!connected || enabledCount === BAND_COUNT}
        >
          Add point
        </button>
      </div>
      <p className="muted">
        Choose a point below to edit it without moving it. Drag to change frequency and applicable gain. The chart shows
        ±12 dB; precise gain controls retain the full ±24 dB range. Double-click the graph to add a Peaking/Band point.
        While audio plays, the shaded area shows the sound coming in: walk or shoot and watch which frequencies rise.
      </p>
      <svg
        ref={svgRef}
        className="advanced-eq-graph"
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
        role="img"
        aria-label="EQ frequency response and movable filter points"
        onPointerMove={movePoint}
        onPointerUp={finishDrag}
        onPointerCancel={finishDrag}
        onDoubleClick={(event) => {
          if (!connected || (event.target as Element).closest(".advanced-eq-point")) return;
          const { x, y } = pointerCoordinates(event);
          addPoint(hzForX(x), dbForY(y));
        }}
      >
        <rect x={LEFT} y={TOP} width={RIGHT - LEFT} height={BOTTOM - TOP} className="advanced-eq-plot" />
        {[-12, -6, 0, 6, 12].map((db) => (
          <g key={db}>
            <line
              x1={LEFT}
              x2={RIGHT}
              y1={yForDb(db)}
              y2={yForDb(db)}
              className={db === 0 ? "advanced-eq-zero" : "advanced-eq-grid"}
            />
            <text x={LEFT - 5} y={yForDb(db) + 3} textAnchor="end" className="advanced-eq-axis">
              {db > 0 ? `+${db}` : db}
            </text>
          </g>
        ))}
        {TICKS.map((frequency) => (
          <g key={frequency}>
            <line x1={xForHz(frequency)} x2={xForHz(frequency)} y1={TOP} y2={BOTTOM} className="advanced-eq-grid" />
            <text x={xForHz(frequency)} y={HEIGHT - 12} textAnchor="middle" className="advanced-eq-axis">
              {frequency >= 1000 ? `${frequency / 1000}k` : frequency}
            </text>
          </g>
        ))}
        {liveArea && <polygon points={liveArea} className="advanced-eq-spectrum" aria-hidden="true" />}
        {liveArea && (
          <text x={LEFT + 6} y={TOP + 12} className="advanced-eq-spectrum-label">
            Live sound in (before EQ)
          </text>
        )}
        {curve && <polyline points={curve} className="advanced-eq-curve" />}
        {visiblePoints.map(({ band, x, y }) => {
          const callout = callouts.get(band.index);
          if (!callout) return null;
          return (
            <g
              key={band.index}
              className="advanced-eq-point"
              aria-label={`Point ${band.index + 1}`}
              onPointerDown={(event) => {
                if (!connected) return;
                event.stopPropagation();
                setSelectedIndex(band.index);
                setDrag({
                  index: band.index,
                  frequencyHz: band.frequencyHz,
                  gainDb: band.gainDb,
                  moved: false,
                  startX: event.clientX,
                  startY: event.clientY,
                });
                svgRef.current?.setPointerCapture(event.pointerId);
              }}
            >
              <line x1={x} y1={y} x2={callout.labelX} y2={callout.labelY} className="advanced-eq-leader" />
              <circle cx={x} cy={y} r={10} className="advanced-eq-point-hit-target" />
              <circle cx={x} cy={y} r={3} fill={COLORS[band.index % COLORS.length]} className="advanced-eq-anchor" />
              <circle
                cx={callout.labelX}
                cy={callout.labelY}
                r={selectedIndex === band.index ? 9 : 8}
                fill={COLORS[band.index % COLORS.length]}
                stroke={selectedIndex === band.index ? "#fff" : "#17222e"}
                strokeWidth={selectedIndex === band.index ? 2.5 : 2}
                className="advanced-eq-label-circle"
              />
              <text x={callout.labelX} y={callout.labelY + 3} textAnchor="middle" className="advanced-eq-point-label">
                {band.index + 1}
              </text>
            </g>
          );
        })}
      </svg>
      <p role="status" className="muted advanced-eq-status">
        {responseError ? `Response unavailable: ${responseError}` : !response ? "Calculating response…" : " "}
      </p>
      {selected && (
        <div className="advanced-eq-controls" key={selected.index}>
          <label className="advanced-eq-point-picker">
            Select point
            <select
              aria-label="EQ point"
              value={selectedIndex}
              onChange={(event) => {
                setSelectedIndex(Number(event.target.value));
                setDrag(null);
              }}
              disabled={enabledCount === 0}
            >
              {!selected.enabled && <option value={selectedIndex}>Point {selectedIndex + 1} (inactive)</option>}
              {bands
                .filter((band) => band.enabled)
                .map((band) => (
                  <option key={band.index} value={band.index}>
                    Point {band.index + 1} — {band.frequencyHz} Hz — {FILTERS.find(([type]) => type === band.type)?.[1]}
                  </option>
                ))}
            </select>
          </label>
          <div className="advanced-eq-selected">
            <strong>Point {selected.index + 1}</strong>
            <button
              type="button"
              className="secondary"
              onClick={() => onChange(`band${selected.index}Enabled`, false)}
              disabled={!connected || !selected.enabled}
            >
              Remove point
            </button>
          </div>
          <label>
            Filter
            <select
              aria-label="EQ filter type"
              value={selected.type}
              disabled={!connected || !selected.enabled}
              onChange={(event) => onChange(`band${selected.index}Type`, event.target.value)}
            >
              {FILTERS.map(([id, label]) => (
                <option key={id} value={id}>
                  {label}
                </option>
              ))}
            </select>
          </label>
          <label>
            Frequency <span>Hz</span>
            <NumberField
              aria-label="EQ frequency Hz"
              min={20}
              max={20000}
              step={1}
              value={selected.frequencyHz}
              disabled={!connected || !selected.enabled}
              onValue={(value) => onChange(`band${selected.index}FrequencyHz`, value)}
            />
          </label>
          <label>
            Gain <span>dB</span>
            <NumberField
              aria-label="EQ gain dB"
              min={-24}
              max={24}
              step={0.1}
              value={selected.gainDb}
              disabled={
                !connected ||
                !selected.enabled ||
                !(selected.type === "peaking" || selected.type === "lowShelf" || selected.type === "highShelf")
              }
              onValue={(value) => onChange(`band${selected.index}GainDb`, value)}
            />
          </label>
          <label>
            Q / width
            <NumberField
              aria-label="EQ Q width"
              min={0.1}
              max={20}
              step={0.1}
              value={selected.q}
              disabled={!connected || !selected.enabled}
              onValue={(value) => onChange(`band${selected.index}Q`, value)}
            />
          </label>
          <small>
            {selected.type === "allPass"
              ? "Changes phase without changing level. The response stays at 0 dB; frequency and Q control the phase transition."
              : selected.type === "bandPass"
                ? "Keeps a frequency band with unity gain at its center. Q controls bandwidth; gain does not apply."
                : selected.type === "notch" || selected.type === "lowPass" || selected.type === "highPass"
                  ? "Gain does not apply to this filter. Q controls the shape."
                  : "Q controls how broad or narrow the change is."}
          </small>
        </div>
      )}
    </section>
  );
}
