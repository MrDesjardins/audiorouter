import { useEffect, useState } from "react";
import { NodeResizer, type NodeProps } from "@xyflow/react";
import { NumberField } from "./NumberField";
import { TextField } from "./TextField";

/** `locked` stops accidental moving or resizing on the canvas; Properties still edits it. */
export type CanvasGroup = {
  id: string;
  name: string;
  color: string;
  opacity: number;
  fontSize: number;
  x: number;
  y: number;
  width: number;
  height: number;
  locked?: boolean;
};
export function validGroup(value: unknown): value is CanvasGroup {
  if (!value || typeof value !== "object") return false;
  const g = value as CanvasGroup;
  return (
    typeof g.id === "string" &&
    /^group-[a-z0-9-]{1,80}$/i.test(g.id) &&
    typeof g.name === "string" &&
    g.name.length <= 120 &&
    /^#[0-9a-f]{6}$/i.test(g.color) &&
    [g.opacity, g.fontSize, g.x, g.y, g.width, g.height].every(Number.isFinite) &&
    g.opacity >= 1 &&
    g.opacity <= 100 &&
    g.fontSize >= 12 &&
    g.fontSize <= 48 &&
    Math.abs(g.x) <= 100_000 &&
    Math.abs(g.y) <= 100_000 &&
    g.width >= 180 &&
    g.width <= 10_000 &&
    g.height >= 100 &&
    g.height <= 10_000 &&
    (g.locked === undefined || typeof g.locked === "boolean")
  );
}
export function readGroups(key: string): CanvasGroup[] {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(key) ?? "[]");
    if (!Array.isArray(value)) return [];
    const seen = new Set<string>();
    return value
      .map((group) => {
        if (!group || typeof group !== "object" || Array.isArray(group)) return group;
        const saved = group as Record<string, unknown>;
        const withFontSize = "fontSize" in saved ? saved : { ...saved, fontSize: 18 };
        return typeof saved.opacity === "number" && Number.isFinite(saved.opacity)
          ? { ...withFontSize, opacity: Math.max(1, Math.min(100, Math.round(saved.opacity))) }
          : withFontSize;
      })
      .filter(validGroup)
      .filter((g) => {
        if (seen.has(g.id)) return false;
        seen.add(g.id);
        return true;
      })
      .slice(0, 64);
  } catch {
    return [];
  }
}
export function useCanvasGroups(sessionId: string) {
  const key = `audiorouter.ui.groups.${sessionId}`;
  const [state, setState] = useState(() => ({ key, groups: readGroups(key) }));
  const groups = state.key === key ? state.groups : [];
  const [selectedGroupId, selectGroup] = useState("");
  useEffect(() => {
    setState({ key, groups: readGroups(key) });
    selectGroup("");
  }, [key]);
  useEffect(() => {
    if (state.key !== key) return;
    try {
      localStorage.setItem(key, JSON.stringify(state.groups));
    } catch {
      /* Presentation storage is optional. */
    }
  }, [key, state]);
  // Persist committed React state, never speculative/replayed state updaters.
  const update = (transform: (current: CanvasGroup[]) => CanvasGroup[]) =>
    setState((current) => ({
      key,
      groups: transform(current.key === key ? current.groups : readGroups(key))
        .filter(validGroup)
        .slice(0, 64),
    }));
  return {
    groups,
    selectedGroupId,
    selectGroup,
    addGroup: () => {
      const id = `group-${crypto.randomUUID()}`;
      update((current) => [
        ...current,
        {
          id,
          name: "Group",
          color: "#5599dd",
          opacity: 5,
          fontSize: 18,
          x: -40 + current.length * 24,
          y: -80 + current.length * 24,
          width: 540,
          height: 340,
        },
      ]);
      selectGroup(id);
    },
    changeGroup: (id: string, patch: Partial<CanvasGroup>) =>
      update((current) =>
        current.map((g) => {
          const next = g.id === id ? { ...g, ...patch, id } : g;
          return validGroup(next) ? next : g;
        }),
      ),
    removeGroup: (id: string) => {
      update((current) => current.filter((g) => g.id !== id));
      selectGroup("");
    },
  };
}
export function CanvasGroupRenderer({ data, selected }: NodeProps) {
  const group = data.group as CanvasGroup;
  return (
    <div
      className={`canvas-group${group.locked ? " is-locked" : ""}`}
      aria-label={`Group ${group.name}`}
      style={{
        backgroundColor: `${group.color}${Math.round(group.opacity * 2.55)
          .toString(16)
          .padStart(2, "0")}`,
      }}
    >
      <NodeResizer
        isVisible={selected && !group.locked}
        minWidth={180}
        minHeight={100}
        maxWidth={10_000}
        maxHeight={10_000}
      />
      <div className="canvas-group-caption" style={{ fontSize: `${group.fontSize}px` }}>
        <strong>{group.name || "Group"}</strong>
        {group.locked && (
          <span
            className="canvas-group-lock"
            role="img"
            aria-label="Locked"
            title="Locked: unlock it in Properties to move or resize"
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <rect x="3" y="7" width="10" height="7" rx="1.5" />
              <path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2" />
            </svg>
          </span>
        )}
      </div>
    </div>
  );
}
export function CanvasGroupInspector({
  group,
  onChange,
  onRemove,
}: {
  group: CanvasGroup;
  onChange: (patch: Partial<CanvasGroup>) => void;
  onRemove: () => void;
}) {
  return (
    <section className="panel inspector" aria-labelledby="inspector-heading">
      <p className="eyebrow">Visual group</p>
      <h2 id="inspector-heading">{group.name || "Group"}</h2>
      <p className="muted">
        {group.locked
          ? "Locked: the group cannot be moved, resized or deleted from the canvas. Unlock it to change its place or size."
          : "Drag the caption or background to move the group; resize its border."}{" "}
        This group only organizes the canvas; audio connections stay unchanged.
      </p>
      <div className="group-properties">
        <label className="group-lock-toggle">
          Lock position and size
          <input
            type="checkbox"
            aria-label="Lock group position and size"
            checked={group.locked === true}
            onChange={(event) => onChange({ locked: event.target.checked })}
          />
          <small>Prevents dragging or resizing the rectangle by accident.</small>
        </label>
        <label>
          Name
          <TextField
            aria-label="Group name"
            value={group.name}
            maxLength={120}
            onValue={(name) => onChange({ name })}
          />
        </label>
        <label>
          Background color
          <input
            type="color"
            aria-label="Group background color"
            value={group.color}
            onChange={(event) => onChange({ color: event.target.value })}
          />
        </label>
        <label>
          Font size (px)
          <NumberField
            aria-label="Group font size"
            value={group.fontSize}
            min={12}
            max={48}
            step={1}
            onValue={(fontSize) => onChange({ fontSize })}
          />
        </label>
        <label>
          Opacity (%)
          <input
            aria-label="Group opacity"
            type="range"
            min={1}
            max={100}
            step={1}
            value={group.opacity}
            onChange={(event) => onChange({ opacity: Number(event.target.value) })}
          />
          <output>{group.opacity}%</output>
        </label>
      </div>
      <button type="button" className="danger" onClick={onRemove}>
        Delete group
      </button>
    </section>
  );
}
