import { createContext, useContext, useEffect, useMemo, useRef, useState, type PointerEvent } from "react";
import type { Node } from "@audiorouter/contracts";
import type { UiBackend, ProcessorResponse } from "./backend";
import { NumberField } from "./NumberField";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";

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
  ["peaking", "g5.eqPeaking"],
  ["lowShelf", "g5.eqLowShelf"],
  ["highShelf", "g5.eqHighShelf"],
  ["lowPass", "g5.eqLowPass"],
  ["highPass", "g5.eqHighPass"],
  ["notch", "g5.eqNotch"],
  ["bandPass", "g5.eqBandPass"],
  ["allPass", "g5.eqAllPass"],
] as const satisfies readonly (readonly [string, MessageKey])[];
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

function readBands(parameters: Node["parameters"]): Band[] {
  return Array.from({ length: BAND_COUNT }, (_, index) => {
    const key = `band${index}`;
    const type = parameters[`${key}Type`];
    return {
      index,
      enabled:
        parameters[`${key}Enabled`] === true ||
        (index === 0 && parameters[`${key}Enabled`] === undefined && parameters.frequencyHz !== undefined),
      type: FILTERS.some(([id]) => id === type) ? (type as FilterType) : "peaking",
      frequencyHz: Number(
        parameters[`${key}FrequencyHz`] ?? (index === 0 ? parameters.frequencyHz : undefined) ?? 1000,
      ),
      gainDb: Number(parameters[`${key}GainDb`] ?? (index === 0 ? parameters.gainDb : undefined) ?? 0),
      q: Number(parameters[`${key}Q`] ?? (index === 0 ? parameters.q : undefined) ?? 1),
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
  const tr = useMessages();
  const bands = useMemo(() => readBands(node.parameters), [node.parameters]);
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
  }, [backend, connected, bands]);

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
    <section className="advanced-eq" aria-label={tr("g5.eqEditor")}>
      <div className="advanced-eq-heading">
        <div>
          <strong>{tr("g5.eqResponse")}</strong>
          <small>{tr("g5.eqActive", { count: enabledCount, total: BAND_COUNT })}</small>
        </div>
        <button
          type="button"
          className="secondary"
          onClick={() => addPoint()}
          disabled={!connected || enabledCount === BAND_COUNT}
        >
          {tr("g5.eqAddPoint")}
        </button>
      </div>
      <p className="muted">{tr("g5.eqHelp")}</p>
      <svg
        ref={svgRef}
        className="advanced-eq-graph"
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
        role="img"
        aria-label={tr("g5.eqGraph")}
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
            {tr("g5.eqLive")}
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
              aria-label={tr("g5.eqPointN", { n: band.index + 1 })}
              onPointerDown={(event) => {
                if (!connected || event.button !== 0) return;
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
        {responseError ? tr("g5.eqUnavailable", { error: responseError }) : !response ? tr("g5.eqCalculating") : " "}
      </p>
      {selected && (
        <div className="advanced-eq-controls" key={selected.index}>
          <label className="advanced-eq-point-picker">
            {tr("g5.eqSelectPoint")}
            <select
              aria-label={tr("g5.eqPointAria")}
              value={selectedIndex}
              onChange={(event) => {
                setSelectedIndex(Number(event.target.value));
                setDrag(null);
              }}
              disabled={enabledCount === 0}
            >
              {!selected.enabled && (
                <option value={selectedIndex}>{tr("g5.eqPointInactive", { n: selectedIndex + 1 })}</option>
              )}
              {bands
                .filter((band) => band.enabled)
                .map((band) => {
                  const filter = FILTERS.find(([type]) => type === band.type);
                  return (
                    <option key={band.index} value={band.index}>
                      {tr("g5.eqPointOption", {
                        n: band.index + 1,
                        hz: band.frequencyHz,
                        type: filter ? tr(filter[1]) : "",
                      })}
                    </option>
                  );
                })}
            </select>
          </label>
          <div className="advanced-eq-selected">
            <strong>{tr("g5.eqPointN", { n: selected.index + 1 })}</strong>
            <button
              type="button"
              className="secondary"
              onClick={() => onChange(`band${selected.index}Enabled`, false)}
              disabled={!connected || !selected.enabled}
            >
              {tr("g5.eqRemove")}
            </button>
          </div>
          <label>
            {tr("g5.eqFilter")}
            <select
              aria-label={tr("g5.eqFilterAria")}
              value={selected.type}
              disabled={!connected || !selected.enabled}
              onChange={(event) => onChange(`band${selected.index}Type`, event.target.value)}
            >
              {FILTERS.map(([id, label]) => (
                <option key={id} value={id}>
                  {tr(label)}
                </option>
              ))}
            </select>
          </label>
          <label>
            {tr("g5.eqFrequency")} <span>Hz</span>
            <NumberField
              aria-label={tr("g5.eqFrequencyAria")}
              min={20}
              max={20000}
              step={1}
              value={selected.frequencyHz}
              disabled={!connected || !selected.enabled}
              onValue={(value) => onChange(`band${selected.index}FrequencyHz`, value)}
            />
          </label>
          <label>
            {tr("g5.eqGain")} <span>dB</span>
            <NumberField
              aria-label={tr("g5.eqGainAria")}
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
            {tr("g5.eqQ")}
            <NumberField
              aria-label={tr("g5.eqQAria")}
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
              ? tr("g5.eqAllPassHelp")
              : selected.type === "bandPass"
                ? tr("g5.eqBandPassHelp")
                : selected.type === "notch" || selected.type === "lowPass" || selected.type === "highPass"
                  ? tr("g5.eqNoGainHelp")
                  : tr("g5.eqQHelp")}
          </small>
        </div>
      )}
    </section>
  );
}
