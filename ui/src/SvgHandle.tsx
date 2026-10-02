import { useState, type KeyboardEvent, type PointerEvent as ReactPointerEvent, type RefObject } from "react";

type Change = (name: string, value: number) => void;
const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));
const roundTo = (value: number, step: number) => Math.round(value / step) * step;

/** Map pointer coordinates into an SVG's viewBox, or null where layout is unavailable (tests). */
export function svgPoint(svg: SVGSVGElement | null, event: { clientX: number; clientY: number }): { x: number; y: number } | null {
  const matrix = svg?.getScreenCTM?.();
  if (!svg || !matrix || typeof svg.createSVGPoint !== "function") return null;
  const point = svg.createSVGPoint();
  point.x = event.clientX;
  point.y = event.clientY;
  const local = point.matrixTransform(matrix.inverse());
  return { x: local.x, y: local.y };
}

export type HandleSpec = {
  name: string;
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  unit: string;
  cx: number;
  cy: number;
  /** Convert a viewBox point into a new parameter value. */
  fromPoint: (point: { x: number; y: number }) => number;
  className?: string;
};

/** A draggable, keyboard-operable point (role=slider) inside an SVG. */
export function Handle({ spec, svg, disabled, onChange }: { spec: HandleSpec; svg: RefObject<SVGSVGElement | null>; disabled: boolean; onChange: Change }) {
  const [dragging, setDragging] = useState(false);
  const commit = (value: number) => {
    const next = clamp(roundTo(value, spec.step), spec.min, spec.max);
    if (Math.abs(next - spec.value) > 1e-9) onChange(spec.name, Number(next.toFixed(4)));
  };
  const move = (event: ReactPointerEvent<SVGGElement>) => {
    if (!dragging) return;
    const point = svgPoint(svg.current, event);
    if (point) commit(spec.fromPoint(point));
  };
  const key = (event: KeyboardEvent<SVGGElement>) => {
    const factor = event.shiftKey ? 10 : 1;
    const delta = event.key === "ArrowUp" || event.key === "ArrowRight" ? spec.step * factor : event.key === "ArrowDown" || event.key === "ArrowLeft" ? -spec.step * factor : 0;
    if (event.key === "Home") { event.preventDefault(); commit(spec.min); return; }
    if (event.key === "End") { event.preventDefault(); commit(spec.max); return; }
    if (delta === 0) return;
    event.preventDefault();
    commit(spec.value + delta);
  };
  return <g
    className={`dynamics-handle ${spec.className ?? ""}${dragging ? " is-dragging" : ""}`}
    role="slider" tabIndex={disabled ? -1 : 0} aria-label={spec.label} aria-disabled={disabled || undefined}
    aria-valuemin={spec.min} aria-valuemax={spec.max} aria-valuenow={spec.value} aria-valuetext={`${spec.value.toFixed(spec.step < 1 ? 1 : 0)} ${spec.unit}`.trim()}
    onPointerDown={(event) => { if (disabled) return; event.currentTarget.setPointerCapture?.(event.pointerId); setDragging(true); }}
    onPointerMove={move}
    onPointerUp={(event) => { event.currentTarget.releasePointerCapture?.(event.pointerId); setDragging(false); }}
    onPointerCancel={() => setDragging(false)}
    onKeyDown={disabled ? undefined : key}
  >
    <title>{`${spec.label}: ${spec.value.toFixed(spec.step < 1 ? 1 : 0)} ${spec.unit}. Drag, or use the arrow keys (Shift for bigger steps).`}</title>
    <circle cx={spec.cx} cy={spec.cy} r={13} className="dynamics-handle-hit" />
    <circle cx={spec.cx} cy={spec.cy} r={5.5} className="dynamics-handle-dot" />
  </g>;
}

