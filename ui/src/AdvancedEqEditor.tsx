import { useEffect, useRef, useState, type PointerEvent } from "react";
import type { Node } from "@audiorouter/contracts";
import type { UiBackend, ProcessorResponse } from "./backend";

const BAND_COUNT = 16;
const WIDTH = 380;
const HEIGHT = 220;
const LEFT = 34;
const RIGHT = 366;
const TOP = 16;
const BOTTOM = 190;
const COLORS = ["#52c7f7", "#a878f9", "#72a0ff", "#ff79b1", "#ff9c6b", "#ffd166", "#66dfc4", "#b3e676"];
const TICKS = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];
const RESPONSE_FREQUENCIES = Array.from({ length: 96 }, (_, index) => 20 * Math.pow(1000, index / 95));
const FILTERS = [
  ["peaking", "Peaking"], ["lowShelf", "Low shelf"], ["highShelf", "High shelf"],
  ["lowPass", "Low pass"], ["highPass", "High pass"], ["notch", "Notch"],
] as const;
type FilterType = typeof FILTERS[number][0];
type Band = { index: number; enabled: boolean; type: FilterType; frequencyHz: number; gainDb: number; q: number };

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
const xForHz = (frequency: number) => LEFT + (Math.log10(clamp(frequency, 20, 20000) / 20) / 3) * (RIGHT - LEFT);
const hzForX = (x: number) => Math.round(20 * Math.pow(1000, clamp((x - LEFT) / (RIGHT - LEFT), 0, 1)));
const yForDb = (gain: number) => TOP + ((24 - clamp(gain, -24, 24)) / 48) * (BOTTOM - TOP);
const dbForY = (y: number) => Math.round((24 - clamp((y - TOP) / (BOTTOM - TOP), 0, 1) * 48) * 10) / 10;

function readBands(node: Node): Band[] {
  return Array.from({ length: BAND_COUNT }, (_, index) => {
    const key = `band${index}`;
    const type = node.parameters[`${key}Type`];
    return {
      index,
      enabled: node.parameters[`${key}Enabled`] === true || (index === 0 && node.parameters[`${key}Enabled`] === undefined && node.parameters.frequencyHz !== undefined),
      type: FILTERS.some(([id]) => id === type) ? type as FilterType : "peaking",
      frequencyHz: Number(node.parameters[`${key}FrequencyHz`] ?? (index === 0 ? node.parameters.frequencyHz : undefined) ?? 1000),
      gainDb: Number(node.parameters[`${key}GainDb`] ?? (index === 0 ? node.parameters.gainDb : undefined) ?? 0),
      q: Number(node.parameters[`${key}Q`] ?? (index === 0 ? node.parameters.q : undefined) ?? 1),
    };
  });
}

export function AdvancedEqEditor({ node, backend, connected, onChange }: {
  node: Node;
  backend: UiBackend;
  connected: boolean;
  onChange: (name: string, value: boolean | number | string) => void;
}) {
  const bands = readBands(node);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [response, setResponse] = useState<ProcessorResponse | null>(null);
  const [responseError, setResponseError] = useState<string | null>(null);
  const [drag, setDrag] = useState<{ index: number; frequencyHz: number; gainDb: number; moved: boolean } | null>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const selected = bands[selectedIndex] ?? bands[0];
  const enabledCount = bands.filter((band) => band.enabled).length;

  useEffect(() => { setSelectedIndex(0); setDrag(null); }, [node.id]);
  useEffect(() => {
    if (!connected) { setResponse(null); return; }
    let live = true;
    const timer = window.setTimeout(() => {
      void backend.processorResponse({
        sampleRateHz: 48000,
        frequenciesHz: RESPONSE_FREQUENCIES,
        bands: bands.map(({ enabled, type, frequencyHz, gainDb, q }) => ({ enabled, type, frequencyHz, gainDb, q })),
      }).then((value) => { if (live) { setResponse(value); setResponseError(null); } })
        .catch((error) => { if (live) { setResponse(null); setResponseError(String(error)); } });
    }, 90);
    return () => { live = false; window.clearTimeout(timer); };
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
    return { x: ((event.clientX - rect.left) / rect.width) * WIDTH, y: ((event.clientY - rect.top) / rect.height) * HEIGHT };
  };
  const movePoint = (event: PointerEvent<SVGSVGElement>) => {
    if (!drag) return;
    const { x, y } = pointerCoordinates(event);
    const type = bands[drag.index].type;
    setDrag({ index: drag.index, frequencyHz: hzForX(x), gainDb: type === "peaking" || type === "lowShelf" || type === "highShelf" ? dbForY(y) : 0, moved: true });
  };
  const finishDrag = (event: PointerEvent<SVGSVGElement>) => {
    if (!drag) return;
    if (svgRef.current?.hasPointerCapture(event.pointerId)) svgRef.current.releasePointerCapture(event.pointerId);
    if (drag.moved) {
      onChange(`band${drag.index}FrequencyHz`, drag.frequencyHz);
      onChange(`band${drag.index}GainDb`, drag.gainDb);
    }
    setDrag(null);
  };
  const curve = response?.frequenciesHz.map((frequency, index) => `${xForHz(frequency).toFixed(1)},${yForDb(response.magnitudeDb[index] ?? 0).toFixed(1)}`).join(" ");

  return <section className="advanced-eq" aria-label="Advanced EQ editor">
    <div className="advanced-eq-heading"><div><strong>Frequency response</strong><small>{enabledCount} of {BAND_COUNT} points active</small></div><button type="button" className="secondary" onClick={() => addPoint()} disabled={!connected || enabledCount === BAND_COUNT}>Add point</button></div>
    <p className="muted">Drag a point to change frequency and gain. Double-click the graph to add a peaking point. Select a point for its filter and width.</p>
    <svg ref={svgRef} className="advanced-eq-graph" viewBox={`0 0 ${WIDTH} ${HEIGHT}`} role="img" aria-label="EQ frequency response and movable filter points" onPointerMove={movePoint} onPointerUp={finishDrag} onPointerCancel={finishDrag} onDoubleClick={(event) => { if (!connected || (event.target as Element).closest(".advanced-eq-point")) return; const { x, y } = pointerCoordinates(event); addPoint(hzForX(x), dbForY(y)); }}>
      <rect x={LEFT} y={TOP} width={RIGHT - LEFT} height={BOTTOM - TOP} className="advanced-eq-plot" />
      {[-24, -12, 0, 12, 24].map((db) => <g key={db}><line x1={LEFT} x2={RIGHT} y1={yForDb(db)} y2={yForDb(db)} className={db === 0 ? "advanced-eq-zero" : "advanced-eq-grid"} /><text x={LEFT - 5} y={yForDb(db) + 3} textAnchor="end" className="advanced-eq-axis">{db > 0 ? `+${db}` : db}</text></g>)}
      {TICKS.map((frequency) => <g key={frequency}><line x1={xForHz(frequency)} x2={xForHz(frequency)} y1={TOP} y2={BOTTOM} className="advanced-eq-grid" /><text x={xForHz(frequency)} y={HEIGHT - 12} textAnchor="middle" className="advanced-eq-axis">{frequency >= 1000 ? `${frequency / 1000}k` : frequency}</text></g>)}
      {curve && <polyline points={curve} className="advanced-eq-curve" />}
      {bands.filter((band) => band.enabled).map((band) => {
        const moved = drag?.index === band.index ? drag : band;
        const y = yForDb(band.type === "peaking" || band.type === "lowShelf" || band.type === "highShelf" ? moved.gainDb : 0);
        return <g key={band.index} className="advanced-eq-point" onPointerDown={(event) => { if (!connected) return; event.stopPropagation(); setSelectedIndex(band.index); setDrag({ index: band.index, frequencyHz: band.frequencyHz, gainDb: band.gainDb, moved: false }); svgRef.current?.setPointerCapture(event.pointerId); }}>
          <circle cx={xForHz(moved.frequencyHz)} cy={y} r={selectedIndex === band.index ? 10 : 8} fill={COLORS[band.index % COLORS.length]} stroke={selectedIndex === band.index ? "#fff" : "#17222e"} strokeWidth={selectedIndex === band.index ? 2.5 : 2} />
          <text x={xForHz(moved.frequencyHz)} y={y + 3} textAnchor="middle" className="advanced-eq-point-label">{band.index + 1}</text>
        </g>;
      })}
    </svg>
    {responseError && <p role="status" className="muted">Response unavailable: {responseError}</p>}
    {!response && !responseError && <p role="status" className="muted">Calculating response…</p>}
    {selected && <div className="advanced-eq-controls">
      <div className="advanced-eq-selected"><strong>Point {selected.index + 1}</strong><button type="button" className="secondary" onClick={() => onChange(`band${selected.index}Enabled`, false)} disabled={!connected || !selected.enabled}>Remove point</button></div>
      <label>Filter<select aria-label="EQ filter type" value={selected.type} disabled={!connected || !selected.enabled} onChange={(event) => onChange(`band${selected.index}Type`, event.target.value)}>{FILTERS.map(([id, label]) => <option key={id} value={id}>{label}</option>)}</select></label>
      <label>Frequency <span>Hz</span><input aria-label="EQ frequency Hz" type="number" min={20} max={20000} value={selected.frequencyHz} disabled={!connected || !selected.enabled} onChange={(event) => { const value = Number(event.target.value); if (Number.isFinite(value) && value >= 20 && value <= 20000) onChange(`band${selected.index}FrequencyHz`, value); }} /></label>
      {(selected.type === "peaking" || selected.type === "lowShelf" || selected.type === "highShelf") && <label>Gain <span>dB</span><input aria-label="EQ gain dB" type="number" min={-24} max={24} step={0.1} value={selected.gainDb} disabled={!connected || !selected.enabled} onChange={(event) => { const value = Number(event.target.value); if (Number.isFinite(value) && value >= -24 && value <= 24) onChange(`band${selected.index}GainDb`, value); }} /></label>}
      <label>Q / width<input aria-label="EQ Q width" type="number" min={0.1} max={20} step={0.1} value={selected.q} disabled={!connected || !selected.enabled} onChange={(event) => { const value = Number(event.target.value); if (Number.isFinite(value) && value >= 0.1 && value <= 20) onChange(`band${selected.index}Q`, value); }} /></label>
      <small>{selected.type === "notch" || selected.type === "lowPass" || selected.type === "highPass" ? "Gain does not apply to this filter. Q controls the shape." : "Q controls how broad or narrow the change is."}</small>
    </div>}
  </section>;
}
