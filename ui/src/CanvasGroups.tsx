import { useEffect, useState } from "react";
import { NodeResizer, type NodeProps } from "@xyflow/react";
import { NumberField } from "./NumberField";
import { TextField } from "./TextField";

export type CanvasGroup = { id: string; name: string; color: string; opacity: number; x: number; y: number; width: number; height: number };
export function validGroup(value: unknown): value is CanvasGroup {
  if (!value || typeof value !== "object") return false;
  const g = value as CanvasGroup;
  return typeof g.id === "string" && /^group-[a-z0-9-]{1,80}$/i.test(g.id)
    && typeof g.name === "string" && g.name.length <= 120 && /^#[0-9a-f]{6}$/i.test(g.color)
    && [g.opacity, g.x, g.y, g.width, g.height].every(Number.isFinite)
    && g.opacity >= 0 && g.opacity <= 100 && Math.abs(g.x) <= 100_000 && Math.abs(g.y) <= 100_000
    && g.width >= 180 && g.width <= 10_000 && g.height >= 100 && g.height <= 10_000;
}
export function readGroups(key: string): CanvasGroup[] {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(key) ?? "[]");
    if (!Array.isArray(value)) return [];
    const seen = new Set<string>();
    return value.filter(validGroup).filter((g) => { if (seen.has(g.id)) return false; seen.add(g.id); return true; }).slice(0, 64);
  } catch { return []; }
}
export function useCanvasGroups(sessionId: string) {
  const key = `audiorouter.ui.groups.${sessionId}`;
  const [state, setState] = useState(() => ({ key, groups: readGroups(key) }));
  const groups = state.key === key ? state.groups : [];
  const [selectedGroupId, selectGroup] = useState("");
  useEffect(() => { setState({ key, groups: readGroups(key) }); selectGroup(""); }, [key]);
  useEffect(() => {
    if (state.key !== key) return;
    try { localStorage.setItem(key, JSON.stringify(state.groups)); } catch { /* Presentation storage is optional. */ }
  }, [key, state]);
  // Persist committed React state, never speculative/replayed state updaters.
  const update = (transform: (current: CanvasGroup[]) => CanvasGroup[]) => setState((current) => ({ key, groups: transform(current.key === key ? current.groups : readGroups(key)).filter(validGroup).slice(0, 64) }));
  return { groups, selectedGroupId, selectGroup,
    addGroup: () => { const id = `group-${crypto.randomUUID()}`; update((current) => [...current, { id, name: "Group", color: "#5599dd", opacity: 25, x: -40 + current.length * 24, y: -80 + current.length * 24, width: 540, height: 340 }]); selectGroup(id); },
    changeGroup: (id: string, patch: Partial<CanvasGroup>) => update((current) => current.map((g) => { const next = g.id === id ? { ...g, ...patch, id } : g; return validGroup(next) ? next : g; })),
    removeGroup: (id: string) => { update((current) => current.filter((g) => g.id !== id)); selectGroup(""); },
  };
}
export function CanvasGroupRenderer({ data, selected }: NodeProps) {
  const group = data.group as CanvasGroup;
  return <div className="canvas-group" aria-label={`Group ${group.name}`} style={{ backgroundColor: `${group.color}${Math.round(group.opacity * 2.55).toString(16).padStart(2, "0")}` }}>
    <NodeResizer isVisible={selected} minWidth={180} minHeight={100} maxWidth={10_000} maxHeight={10_000} />
    <div className="canvas-group-caption"><strong>{group.name || "Group"}</strong></div>
  </div>;
}
export function CanvasGroupInspector({ group, onChange, onRemove }: { group: CanvasGroup; onChange: (patch: Partial<CanvasGroup>) => void; onRemove: () => void }) {
  return <section className="panel inspector" aria-labelledby="inspector-heading"><p className="eyebrow">Visual group</p><h2 id="inspector-heading">{group.name || "Group"}</h2>
    <p className="muted">Move the caption and resize the border. This group only organizes the canvas; audio connections stay unchanged.</p>
    <div className="group-properties"><label>Name<TextField aria-label="Group name" value={group.name} maxLength={120} onValue={(name) => onChange({ name })} /></label>
    <label>Background color<input type="color" aria-label="Group background color" value={group.color} onChange={(event) => onChange({ color: event.target.value })} /></label>
    <label>Opacity (%)<NumberField aria-label="Group opacity" value={group.opacity} min={0} max={100} onValue={(opacity) => onChange({ opacity })} /></label></div>
    <button type="button" className="danger" onClick={onRemove}>Delete group</button>
  </section>;
}
