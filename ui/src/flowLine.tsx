/**
 * The drawn audio connection: a cable track, a level-coloured core with a
 * source→target gradient, a glow that follows intensity, light "comets"
 * travelling with the audio, and a fixed-size arrowhead. Colours come from
 * theme variables (--flow-*), so the same geometry works in every theme.
 */
import {
  createContext,
  useContext,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import type { FlowAnimationMode } from "./preferences";

/** Whether connection comets may move now (Setup → Animated connections). */
const FlowMotionContext = createContext(true);

/** Whether AudioRouter is the active, visible window. */
export function useWindowFocused() {
  const [focused, setFocused] = useState(
    () => typeof document === "undefined" || (document.hasFocus() && !document.hidden),
  );
  useEffect(() => {
    const onFocus = () => setFocused(!document.hidden);
    const onBlur = () => setFocused(false);
    const onVisibility = () => setFocused(!document.hidden && document.hasFocus());
    window.addEventListener("focus", onFocus);
    window.addEventListener("blur", onBlur);
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("blur", onBlur);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, []);
  return focused;
}

/**
 * Applies the Animated connections setting below it. Focus changes reach only
 * the connection layers (context consumers), not the canvas.
 */
export function FlowMotionProvider({ mode, children }: { mode: FlowAnimationMode; children: ReactNode }) {
  const focused = useWindowFocused();
  const moving = mode === "on" || (mode === "focus" && focused);
  return <FlowMotionContext.Provider value={moving}>{children}</FlowMotionContext.Provider>;
}

export type FlowSide = "left" | "right" | "top" | "bottom";
type Point = { x: number; y: number };

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
export const MAX_COMETS = 6;
/**
 * Connections show the level in steps of this size. Every connection shares
 * one canvas-sized SVG, so any change repaints all of it; a level that
 * changed the colour and width on every 20 Hz tick (with 160 ms transitions)
 * kept that layer repainting at 60 fps (2026-10-14 soak: ~50% of a core of
 * GPU time with 8 live connections).
 */
export const LEVEL_STEP_DB = 3;

/** The level a connection draws: `levelDb` rounded to `LEVEL_STEP_DB`. */
export function shownLevel(levelDb: number | null): number | null {
  return levelDb === null || !Number.isFinite(levelDb) ? levelDb : Math.round(levelDb / LEVEL_STEP_DB) * LEVEL_STEP_DB;
}
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
export function FlowArrow({
  at,
  side,
  className,
  style,
}: {
  at: Point;
  side: FlowSide;
  className: string;
  style?: CSSProperties;
}) {
  return (
    <path
      className={className}
      style={style}
      d="M-11,-6 L0,0 L-11,6 L-8,0 Z"
      transform={`translate(${at.x.toFixed(1)} ${at.y.toFixed(1)}) rotate(${ARROW_ROTATION[side]})`}
    />
  );
}

function usePrefersReducedMotion() {
  const query = "(prefers-reduced-motion: reduce)";
  const [reduced, setReduced] = useState(
    () => typeof window !== "undefined" && typeof window.matchMedia === "function" && window.matchMedia(query).matches,
  );
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
  if (numbers.length < 8)
    return Math.max(1, Math.hypot((numbers[2] ?? 0) - (numbers[0] ?? 0), (numbers[3] ?? 0) - (numbers[1] ?? 0)));
  const [x0, y0, x1, y1, x2, y2, x3, y3] = numbers;
  let length = 0,
    px = x0,
    py = y0;
  for (let step = 1; step <= 24; step += 1) {
    const t = step / 24,
      u = 1 - t;
    const x = u * u * u * x0 + 3 * u * u * t * x1 + 3 * u * t * t * x2 + t * t * t * x3;
    const y = u * u * u * y0 + 3 * u * u * t * y1 + 3 * u * t * t * y2 + t * t * t * y3;
    length += Math.hypot(x - px, y - py);
    px = x;
    py = y;
  }
  return length;
}

/** Points of an "M x,y C ..." path (or a straight "M x,y L x,y"), dense in t. */
function pathPoints(path: string, count: number): Point[] {
  const n = path.match(/-?\d+(?:\.\d+)?(?:e-?\d+)?/gi)?.map(Number) ?? [];
  const [x0 = 0, y0 = 0] = n;
  if (n.length < 8) {
    const [x1 = x0, y1 = y0] = n.slice(2);
    return Array.from({ length: count + 1 }, (_, i) => ({
      x: x0 + ((x1 - x0) * i) / count,
      y: y0 + ((y1 - y0) * i) / count,
    }));
  }
  const [, , x1, y1, x2, y2, x3, y3] = n;
  return Array.from({ length: count + 1 }, (_, i) => {
    const t = i / count,
      u = 1 - t;
    return {
      x: u * u * u * x0 + 3 * u * u * t * x1 + 3 * u * t * t * x2 + t * t * t * x3,
      y: u * u * u * y0 + 3 * u * u * t * y1 + 3 * u * t * t * y2 + t * t * t * y3,
    };
  });
}

/**
 * Transform keyframes that carry a comet along `path` at constant speed,
 * turned along the curve. Angles are unwrapped so a curve through ±180°
 * never spins the comet.
 */
export function cometKeyframes(path: string, samples = 32): Keyframe[] {
  const dense = pathPoints(path, 96);
  const lengths = [0];
  for (let i = 1; i < dense.length; i += 1)
    lengths.push(lengths[i - 1] + Math.hypot(dense[i].x - dense[i - 1].x, dense[i].y - dense[i - 1].y));
  const total = Math.max(lengths.at(-1)!, 1e-6);
  const at = (distance: number): Point => {
    let i = 1;
    while (i < lengths.length - 1 && lengths[i] < distance) i += 1;
    const span = lengths[i] - lengths[i - 1] || 1;
    const f = Math.min(1, Math.max(0, (distance - lengths[i - 1]) / span));
    return {
      x: dense[i - 1].x + (dense[i].x - dense[i - 1].x) * f,
      y: dense[i - 1].y + (dense[i].y - dense[i - 1].y) * f,
    };
  };
  const frames: Keyframe[] = [];
  let previousAngle: number | null = null;
  for (let k = 0; k <= samples; k += 1) {
    const s = (k / samples) * total;
    const point = at(s);
    const ahead = at(Math.min(total, s + total / samples / 2));
    const behind = at(Math.max(0, s - total / samples / 2));
    let angle = (Math.atan2(ahead.y - behind.y, ahead.x - behind.x) * 180) / Math.PI;
    if (previousAngle !== null) angle = previousAngle + ((((angle - previousAngle) % 360) + 540) % 360) - 180;
    previousAngle = angle;
    frames.push({
      offset: k / samples,
      transform: `translate(${point.x.toFixed(1)}px, ${point.y.toFixed(1)}px) rotate(${angle.toFixed(1)}deg)`,
    });
  }
  return frames;
}

/**
 * The travelling lights of an active connection, as small HTML elements in
 * React Flow's label layer animated by transform keyframes. The compositor
 * moves them, so the shared edge SVG is not repainted 60 times a second (as
 * SMIL animateMotion inside it did: 2026-10-14 soak, ~40% of a core of GPU
 * time with 8 live connections). Reduced motion or the Animated connections
 * setting renders none.
 */
export function FlowComets({ path, levelDb }: { path: string; levelDb: number | null }) {
  const prefersReduced = usePrefersReducedMotion();
  const motionAllowed = useContext(FlowMotionContext);
  const refs = useRef<(HTMLSpanElement | null)[]>([]);
  const animations = useRef<(Animation | null)[]>([]);
  const visual = flowVisual(shownLevel(levelDb));
  // Round the length so dragging a node does not change the speed per pixel.
  const length = Math.max(40, Math.round(bezierLength(path) / 40) * 40);
  const duration = (length / COMET_SPEED) * 1000;
  const moving = !prefersReduced && motionAllowed;
  useLayoutEffect(() => {
    if (!moving) return;
    const frames = cometKeyframes(path);
    refs.current.forEach((element, index) => {
      if (!element || typeof element.animate !== "function") return;
      const existing = animations.current[index];
      if (existing) {
        // A moved node updates the route in place: no comet restarts.
        (existing.effect as KeyframeEffect | null)?.setKeyframes(frames);
        existing.effect?.updateTiming({ duration });
        return;
      }
      animations.current[index] = element.animate(frames, {
        duration,
        delay: -(index / MAX_COMETS) * duration,
        iterations: Infinity,
        easing: "linear",
      });
    });
  }, [moving, path, duration]);
  useEffect(
    () => () => {
      animations.current.forEach((animation) => animation?.cancel());
      animations.current = [];
    },
    [moving],
  );
  if (!moving) return null;
  return (
    <div className="flow-comets" aria-hidden="true" style={{ "--comet-scale": visual.cometScale } as CSSProperties}>
      {Array.from({ length: MAX_COMETS }, (_, index) => (
        <span
          key={index}
          ref={(element) => {
            refs.current[index] = element;
          }}
          className="flow-comet"
          style={{ opacity: index < visual.comets ? 1 : 0 }}
        >
          <span className="flow-comet-body" />
        </span>
      ))}
    </div>
  );
}

/**
 * Decorative layers for an active connection. The caller draws the core
 * path (the interactive React Flow edge path) with `stroke: url(#gradientId)`;
 * the travelling lights are `FlowComets`, in the HTML label layer.
 */
export function FlowActiveLayers({
  id,
  path,
  source,
  target,
  targetSide,
  levelDb,
  core,
}: {
  id: string;
  path: string;
  source: Point;
  target: Point;
  targetSide: FlowSide;
  levelDb: number | null;
  core: (style: CSSProperties) => ReactNode;
}) {
  const level = shownLevel(levelDb);
  const visual = flowVisual(level);
  const color = flowColor(level);
  const safe = id.replace(/[^a-zA-Z0-9_-]/g, "_");
  return (
    <>
      <defs>
        <linearGradient
          id={`flow-gradient-${safe}`}
          gradientUnits="userSpaceOnUse"
          x1={source.x}
          y1={source.y}
          x2={target.x}
          y2={target.y}
        >
          <stop offset="0%" className="flow-stop" style={{ stopColor: color, stopOpacity: 0.45 }} />
          <stop offset="70%" className="flow-stop" style={{ stopColor: color, stopOpacity: 0.95 }} />
          <stop offset="100%" className="flow-stop" style={{ stopColor: color, stopOpacity: 1 }} />
        </linearGradient>
      </defs>
      <path
        d={path}
        className="flow-glow"
        style={{ stroke: color, strokeWidth: visual.glowWidth, opacity: visual.glowOpacity }}
      />
      {core({ stroke: `url(#flow-gradient-${safe})`, strokeWidth: visual.coreWidth })}
      <FlowArrow at={target} side={targetSide} className="flow-arrow is-active" style={{ fill: color }} />
    </>
  );
}
