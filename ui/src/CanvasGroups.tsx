import { returnToPropertiesTab } from "./workbenchPanel";
import { ToolGuideLink } from "./ToolGuideLink";
import { useEffect, useState, type CSSProperties } from "react";
import { NodeResizer, useStore, type NodeProps } from "@xyflow/react";
import { NumberField } from "./NumberField";
import { SliderField } from "./Slider";
import { TextField } from "./TextField";
import { BooleanField } from "./BooleanField";
import { HelpDisclosure } from "./HelpDisclosure";
import { optionalLocalStorage, readPresentationJson, writePresentationJson } from "./presentationStorage";
import { useMessages } from "./i18n/useMessages";
import { tx } from "./i18n";

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
  moveContents?: boolean;
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
    (g.locked === undefined || typeof g.locked === "boolean") &&
    (g.moveContents === undefined || typeof g.moveContents === "boolean")
  );
}
export function readGroups(key: string): CanvasGroup[] {
  try {
    const value = readPresentationJson(optionalLocalStorage(), key);
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
      .map(({ id, name, color, opacity, fontSize, x, y, width, height, locked, moveContents }) => ({
        id,
        name,
        color,
        opacity,
        fontSize,
        x,
        y,
        width,
        height,
        ...(locked === undefined ? {} : { locked }),
        ...(moveContents === undefined ? {} : { moveContents }),
      }))
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
  const [selectedGroupIds, selectGroups] = useState<string[]>([]);
  const selectedGroupId = selectedGroupIds[0] ?? "";
  const selectGroup = (id: string) => selectGroups(id ? [id] : []);
  useEffect(() => {
    setState({ key, groups: readGroups(key) });
    selectGroup("");
  }, [key]);
  useEffect(() => {
    if (state.key !== key) return;
    writePresentationJson(optionalLocalStorage(), key, state.groups);
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
    selectedGroupIds,
    selectGroup,
    selectGroups,
    addGroup: () => {
      const id = `group-${crypto.randomUUID()}`;
      update((current) => [
        ...current,
        {
          id,
          name: tx("g4Panel.groupDefault"),
          color: "#5599dd",
          opacity: 5,
          fontSize: 18,
          x: -40 + current.length * 24,
          y: -80 + current.length * 24,
          width: 540,
          height: 340,
          moveContents: true,
        },
      ]);
      selectGroup(id);
    },
    changeGroup: (id: string, patch: Partial<CanvasGroup>) =>
      update((current) =>
        current.map((g) => {
          const safePatch = g.locked ? { ...patch, x: g.x, y: g.y, width: g.width, height: g.height } : patch;
          const next = g.id === id ? { ...g, ...safePatch, id } : g;
          return validGroup(next) ? next : g;
        }),
      ),
    changeGroupOptions: (ids: string[], patch: Partial<Pick<CanvasGroup, "locked" | "moveContents">>) =>
      update((current) =>
        current.map((group) => {
          if (!ids.includes(group.id)) return group;
          const next = {
            ...group,
            ...(patch.locked === undefined ? {} : { locked: patch.locked }),
            ...(patch.moveContents === undefined ? {} : { moveContents: patch.moveContents }),
          };
          return validGroup(next) ? next : group;
        }),
      ),
    changeGroups: (patches: Map<string, Partial<CanvasGroup>>) =>
      update((current) =>
        current.map((group) => {
          const patch = patches.get(group.id);
          if (!patch || group.locked) return group;
          const next = { ...group, ...patch, id: group.id };
          return validGroup(next) ? next : group;
        }),
      ),
    removeGroup: (id: string) => {
      update((current) => current.filter((g) => g.id !== id));
      selectGroup("");
    },
  };
}
export function CanvasGroupRenderer({ data, selected }: NodeProps) {
  const tr = useMessages();
  const group = data.group as CanvasGroup;
  // Lets the side grab zones keep one screen size at any zoom; re-renders only when the zoom changes.
  const zoom = useStore((state) => state.transform[2]);
  return (
    <div
      className={`canvas-group${group.locked ? " is-locked" : ""}`}
      aria-label={tr("g4Panel.groupAria", { name: group.name })}
      style={
        {
          "--group-zoom": zoom,
          backgroundColor: `${group.color}${Math.round(group.opacity * 2.55)
            .toString(16)
            .padStart(2, "0")}`,
        } as CSSProperties
      }
    >
      <NodeResizer
        isVisible={selected && !group.locked}
        minWidth={180}
        minHeight={100}
        maxWidth={10_000}
        maxHeight={10_000}
        onResizeStart={() => (data.onResizeStart as (() => void) | undefined)?.()}
        onResizeEnd={() => (data.onResizeEnd as (() => void) | undefined)?.()}
      />
      <div
        className="canvas-group-caption"
        style={{ fontSize: `${group.fontSize}px` }}
        title={!group.locked && data.movementBlocked ? tr("g4Panel.groupNestedLocked") : undefined}
      >
        <strong>{group.name || tr("g4Panel.groupDefault")}</strong>
        {group.locked && (
          <span
            className="canvas-group-lock"
            role="img"
            aria-label={tr("g4Panel.groupLocked")}
            title={tr("g4Panel.groupLockedTip")}
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
  const tr = useMessages();
  return (
    <section
      className="panel inspector"
      id="workbench-properties"
      onKeyDown={returnToPropertiesTab}
      role="tabpanel"
      aria-labelledby="workbench-tab-properties"
      tabIndex={0}
    >
      <p className="eyebrow">{tr("g4Panel.visualGroup")}</p>
      <h2 id="inspector-heading">{group.name || tr("g4Panel.groupDefault")}</h2>
      <ToolGuideLink kind="group" />
      <HelpDisclosure title={tr("g4Panel.arrangeGroup")}>
        <p className="muted">
          {tr(group.locked ? "g4Panel.groupHelpLocked" : "g4Panel.groupHelpFree")} {tr("g4Panel.groupHelpOnly")}
        </p>
      </HelpDisclosure>
      <div className="group-properties">
        <div className="group-position-options">
          <BooleanField
            label={tr("g4Panel.lockPositionSize")}
            name={tr("g4Panel.lockGroupName")}
            checked={group.locked === true}
            onChange={(locked) => onChange({ locked })}
            help={<small>{tr("g4Panel.lockGroupHelp")}</small>}
          />
          <BooleanField
            label={tr("g4Panel.moveContents")}
            checked={group.moveContents === true}
            onChange={(moveContents) => onChange({ moveContents })}
            help={<small>{tr("g4Panel.moveContentsHelp")}</small>}
          />
        </div>
        <label>
          {tr("g4Panel.groupNameLabel")}
          <TextField
            aria-label={tr("g4Panel.groupNameAria")}
            value={group.name}
            maxLength={120}
            onValue={(name) => onChange({ name })}
          />
        </label>
        <label>
          {tr("g4Panel.groupColorLabel")}
          <input
            type="color"
            aria-label={tr("g4Panel.groupColorAria")}
            value={group.color}
            onChange={(event) => onChange({ color: event.target.value })}
          />
        </label>
        <label>
          {tr("g4Panel.groupFontLabel")}
          <NumberField
            aria-label={tr("g4Panel.groupFontAria")}
            value={group.fontSize}
            min={12}
            max={48}
            step={1}
            onValue={(fontSize) => onChange({ fontSize })}
          />
        </label>
        <SliderField
          label={tr("g4Panel.groupOpacity")}
          name={tr("g4Panel.groupOpacityAria")}
          value={group.opacity}
          min={1}
          max={100}
          step={1}
          unit="%"
          defaultValue={5}
          onChange={(opacity) => onChange({ opacity })}
        />
      </div>
      <button type="button" className="danger" onClick={onRemove}>
        {tr("g4Panel.deleteGroup")}
      </button>
    </section>
  );
}
