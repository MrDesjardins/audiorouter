import type { Node } from "@audiorouter/contracts";

const BANDS = 64;
const WIDTH = 380;
const HEIGHT = 200;
const LEFT = 34;
const RIGHT = 366;
const TOP = 12;
const BOTTOM = 172;
const MIN_DB = -110;
const MAX_DB = 0;
const TICKS = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];
/** Spectrum power (dB) of a full-scale sine is about +50 dB; shift to ≈ dBFS. */
const DBFS_OFFSET = 50;
const SAMPLE_RATE = 48_000;
const FRAME = 1024;

/** Centre frequency of each of the 64 log-spaced profile bands (mirrors the backend). */
export const BAND_FREQUENCIES_HZ: number[] = (() => {
  const bins = FRAME / 2 + 1;
  const edges = Array.from({ length: BANDS + 1 }, (_, band) => Math.round(Math.pow(bins - 1, band / BANDS)));
  edges[0] = 0;
  edges[BANDS] = bins;
  for (let band = 1; band <= BANDS; band += 1) edges[band] = Math.min(bins, Math.max(edges[band], edges[band - 1] + 1));
  return Array.from({ length: BANDS }, (_, band) => {
    const low = Math.max(edges[band], 0.5);
    const high = Math.max(edges[band + 1] - 1, low);
    return Math.sqrt(low * high) * SAMPLE_RATE / FRAME;
  });
})();

/** Decode a stored 64-band profile (two hex digits per band, 1 dB steps from −160 dB).
 * A profile that learned nothing (every band at the floor) counts as none. */
export function decodeProfileDb(profile: unknown): number[] | null {
  if (typeof profile !== "string" || !/^[0-9a-fA-F]{128}$/.test(profile)) return null;
  const levels = Array.from({ length: BANDS }, (_, band) => Number.parseInt(profile.slice(band * 2, band * 2 + 2), 16) - 160);
  return levels.some((db) => db > -150) ? levels : null;
}

/** Whether a node parameter holds a usable learned noise profile. */
export function hasLearnedNoise(profile: unknown): boolean {
  return decodeProfileDb(profile) !== null;
}

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
const xForHz = (hz: number) => LEFT + (Math.log10(clamp(hz, 20, 20000) / 20) / 3) * (RIGHT - LEFT);
const yForDb = (db: number) => TOP + ((MAX_DB - clamp(db, MIN_DB, MAX_DB)) / (MAX_DB - MIN_DB)) * (BOTTOM - TOP);
const line = (levels: number[], offset = 0) => levels
  .map((db, band) => ({ hz: BAND_FREQUENCIES_HZ[band], db: db - DBFS_OFFSET + offset }))
  .filter((point) => point.hz >= 20)
  .map((point) => `${xForHz(point.hz).toFixed(1)},${yForDb(point.db).toFixed(1)}`)
  .join(" ");

/**
 * Spectral Gate: shows the live spectrum of the sound entering the tool, lets
 * the user learn the noise (the loudest level each frequency band reaches
 * while learning), and draws the resulting gate threshold. Frequencies whose
 * level stays under the threshold are turned down.
 */
export function SpectralGateEditor({ node, running, levelsDb, liveProfile, disabled, onChange }: {
  node: Node;
  running: boolean;
  levelsDb: number[] | null;
  liveProfile: string | null;
  disabled: boolean;
  onChange: (changes: Array<[string, boolean | number | string]>) => void;
}) {
  const learning = node.parameters.learning === true;
  const threshold = typeof node.parameters.thresholdDb === "number" ? node.parameters.thresholdDb : 3;
  const learned = decodeProfileDb(learning ? liveProfile : node.parameters.noiseProfile);
  const active = node.enabled && !node.bypass;
  const status = !node.enabled ? "Off: enable this tool to see its live spectrum." : node.bypass ? "Bypass: turn Bypass off to process sound and see the live spectrum." : !running ? "Start the route to see the live spectrum."
    : !levelsDb ? "Waiting for live spectrum readings. Check the incoming route and signal." : learning ? (liveProfile ? "Learning: play only the noise you want removed, then stop learning." : "Waiting for the first measurement…")
      : learned ? "Frequencies under the gate threshold line are turned down." : "Learn the noise to start blocking it.";
  return <section className="spectral-gate" aria-label="Spectral Gate editor">
    <div className="advanced-eq-heading"><div><strong>Noise at every frequency</strong><small>{learning ? "Learning now" : learned ? "Learned noise is stored" : "No noise learned yet"}</small></div>
      {!learning
        ? <button type="button" className="secondary" disabled={disabled || !running || !active} onClick={() => onChange([["learning", true]])}>{learned ? "Learn again" : "Learn noise"}</button>
        : <button type="button" className="primary" disabled={disabled || !running || !active || !liveProfile} onClick={() => liveProfile && onChange([["noiseProfile", liveProfile], ["learning", false]])}>Stop and keep</button>}
    </div>
    <svg className="advanced-eq-graph spectral-gate-graph" viewBox={`0 0 ${WIDTH} ${HEIGHT}`} role="img" aria-label="Live spectrum with the learned noise and gate threshold">
      <rect x={LEFT} y={TOP} width={RIGHT - LEFT} height={BOTTOM - TOP} className="advanced-eq-plot" />
      {[-100, -80, -60, -40, -20, 0].map((db) => <g key={db}><line x1={LEFT} x2={RIGHT} y1={yForDb(db)} y2={yForDb(db)} className="advanced-eq-grid" /><text x={LEFT - 5} y={yForDb(db) + 3} textAnchor="end" className="advanced-eq-axis">{db}</text></g>)}
      {TICKS.map((hz) => <g key={hz}><line x1={xForHz(hz)} x2={xForHz(hz)} y1={TOP} y2={BOTTOM} className="advanced-eq-grid" /><text x={xForHz(hz)} y={HEIGHT - 12} textAnchor="middle" className="advanced-eq-axis">{hz >= 1000 ? `${hz / 1000}k` : hz}</text></g>)}
      {running && active && levelsDb && <polyline points={line(levelsDb)} className="spectral-gate-live" />}
      {learned && <polyline points={line(learned)} className="spectral-gate-learned" />}
      {learned && !learning && <polyline points={line(learned, threshold)} className="spectral-gate-threshold" />}
    </svg>
    <div className="spectral-gate-legend" aria-hidden="true">
      <span><i className="spectral-gate-key is-live" />Live sound</span>
      <span><i className="spectral-gate-key is-learned" />Learned noise</span>
      <span><i className="spectral-gate-key is-threshold" />Gate threshold</span>
    </div>
    <p role="status" className="muted spectral-gate-status">{status}</p>
  </section>;
}
