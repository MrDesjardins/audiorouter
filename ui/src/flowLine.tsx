/**
 * The drawn audio connection: a cable track, a level-coloured core with a
 * source→target gradient, a glow that follows intensity, light "comets"
 * travelling with the audio, and a fixed-size arrowhead. Colours come from
 * theme variables (--flow-*), so the same geometry works in every theme.
 */
import { useEffect, useState, type CSSProperties, type ReactNode } from "react";

export type FlowSide = "left" | "right" | "top" | "bottom";
type Point = { x: number; y: number };

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
export const MAX_COMETS = 6;
/** Travel speed in canvas pixels per second; constant so a level change never jolts motion. */
export const COMET_SPEED = 150;

/** Visual parameters for an active connection at `levelDb` (RMS dBFS). */
export function flowVisual(levelDb: number | null) {
  const t = levelDb === null || !Number.isFinite(levelDb) ? 0 : clamp((Math.min(levelDb, 0) + 60) / 60, 0, 1);
  return {
    intensity: t,
    coreWidth: Number((2.2 + t * 3.6).toFixed(2)),
    glowWidth: Number((8 + t * 12).toFixed(2)),
    glowOpacity: Number((0.12 + t * 0.5).toFixed(3)),
    comets: levelDb === null || levelDb <= -60 ? 0 : 1 + Math.round(t * (MAX_COMETS - 1)),
    cometScale: Number((0.75 + t * 0.6).toFixed(3)),
  };
}

/** Level colour from the theme palette: cold (quiet) → warm (speech) → hot → clip. */
export function flowColor(levelDb: number | null): string {
  if (levelDb === null || !Number.isFinite(levelDb)) return "var(--flow-cold)";
  const mix = (from: string, to: string, low: number, high: number) =>
    `color-mix(in oklab, var(${to}) ${Math.round(clamp((levelDb - low) / (high - low), 0, 1) * 100)}%, var(${from}))`;
  if (levelDb <= -40) return "var(--flow-cold)";
  if (levelDb <= -20) return mix("--flow-cold", "--flow-warm", -40, -20);
  if (levelDb <= -6) return mix("--flow-warm", "--flow-hot", -20, -6);
  if (levelDb < -1) return mix("--flow-hot", "--flow-clip", -6, -1);
  return "var(--flow-clip)";
}

const ARROW_ROTATION: Record<FlowSide, number> = { left: 0, right: 180, top: 90, bottom: -90 };

/** Arrowhead with its tip on the target handle, pointing into the node. */
export function FlowArrow({ at, side, className, style }: { at: Point; side: FlowSide; className: string; style?: CSSProperties }) {
  return <path className={className} style={style} d="M-11,-6 L0,0 L-11,6 L-8,0 Z" transform={`translate(${at.x.toFixed(1)} ${at.y.toFixed(1)}) rotate(${ARROW_ROTATION[side]})`} />;
}

function usePrefersReducedMotion() {
  const query = "(prefers-reduced-motion: reduce)";
  const [reduced, setReduced] = useState(() => typeof window !== "undefined" && typeof window.matchMedia === "function" && window.matchMedia(query).matches);
  useEffect(() => {
    if (typeof window.matchMedia !== "function") return;
    const media = window.matchMedia(query);
    const update = () => setReduced(media.matches);
    media.addEventListener?.("change", update);
    return () => media.removeEventListener?.("change", update);
  }, []);
  return reduced;
}

/** Approximate a cubic bezier path length from its "M x,y C ..." string. */
export function bezierLength(path: string): number {
  const numbers = path.match(/-?\d+(?:\.\d+)?(?:e-?\d+)?/gi)?.map(Number) ?? [];
  if (numbers.length < 8) return Math.max(1, Math.hypot((numbers[2] ?? 0) - (numbers[0] ?? 0), (numbers[3] ?? 0) - (numbers[1] ?? 0)));
  const [x0, y0, x1, y1, x2, y2, x3, y3] = numbers;
  let length = 0, px = x0, py = y0;
  for (let step = 1; step <= 24; step += 1) {
    const t = step / 24, u = 1 - t;
    const x = u * u * u * x0 + 3 * u * u * t * x1 + 3 * u * t * t * x2 + t * t * t * x3;
    const y = u * u * u * y0 + 3 * u * u * t * y1 + 3 * u * t * t * y2 + t * t * t * y3;
    length += Math.hypot(x - px, y - py);
    px = x; py = y;
  }
  return length;
}

/**
 * Decorative layers for an active connection. The caller draws the core
 * path (the interactive React Flow edge path) with `stroke: url(#gradientId)`.
 */
export function FlowActiveLayers({ id, path, source, target, targetSide, levelDb, core }: { id: string; path: string; source: Point; target: Point; targetSide: FlowSide; levelDb: number | null; core: (style: CSSProperties) => ReactNode }) {
  const reduced = usePrefersReducedMotion();
  const visual = flowVisual(levelDb);
  const color = flowColor(levelDb);
  // Round the length so dragging a node does not restart every comet.
  const length = Math.max(40, Math.round(bezierLength(path) / 40) * 40);
  const duration = length / COMET_SPEED;
  const safe = id.replace(/[^a-zA-Z0-9_-]/g, "_");
  const pathId = `flow-path-${safe}`;
  return <>
    <defs>
      <linearGradient id={`flow-gradient-${safe}`} gradientUnits="userSpaceOnUse" x1={source.x} y1={source.y} x2={target.x} y2={target.y}>
        <stop offset="0%" className="flow-stop" style={{ stopColor: color, stopOpacity: 0.45 }} />
        <stop offset="70%" className="flow-stop" style={{ stopColor: color, stopOpacity: 0.95 }} />
        <stop offset="100%" className="flow-stop" style={{ stopColor: color, stopOpacity: 1 }} />
      </linearGradient>
      <linearGradient id={`flow-comet-${safe}`} x1="0" x2="1" y1="0" y2="0">
        <stop offset="0%" className="flow-comet-tail" />
        <stop offset="100%" className="flow-comet-head" />
      </linearGradient>
    </defs>
    <path id={pathId} d={path} className="flow-glow" style={{ stroke: color, strokeWidth: visual.glowWidth, opacity: visual.glowOpacity }} />
    {core({ stroke: `url(#flow-gradient-${safe})`, strokeWidth: visual.coreWidth })}
    {/* Reduced motion keeps the colour, glow and arrow; comets only travel. */}
    {!reduced && Array.from({ length: MAX_COMETS }, (_, index) => {
      const visible = index < visual.comets;
      const phase = (index / MAX_COMETS) * duration;
      return <g key={index} className="flow-comet" style={{ opacity: visible ? 1 : 0 }}>
        <ellipse rx={7 * visual.cometScale} ry={1.9 * visual.cometScale} fill={`url(#flow-comet-${safe})`} className="flow-comet-body">
          {<animateMotion dur={`${duration.toFixed(2)}s`} begin={`-${phase.toFixed(2)}s`} repeatCount="indefinite" rotate="auto"><mpath href={`#${pathId}`} /></animateMotion>}
        </ellipse>
      </g>;
    })}
    <FlowArrow at={target} side={targetSide} className="flow-arrow is-active" style={{ fill: color }} />
  </>;
}
