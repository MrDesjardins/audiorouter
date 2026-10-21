import type { CanvasRect, LayoutAction } from "./canvasGeometry";
import type { CanvasGroup } from "./CanvasGroups";

const iconPaths: Record<LayoutAction, string> = {
  "align-left": "M4 3v18M8 5h12v5H8zM8 14h8v5H8z",
  "align-center": "M12 2v20M3 5h18v5H3zM7 14h10v5H7z",
  "align-right": "M20 3v18M4 5h12v5H4zM8 14h8v5H8z",
  "align-top": "M3 4h18M5 8h5v12H5zM14 8h5v8h-5z",
  "align-middle": "M2 12h20M5 3h5v18H5zM14 7h5v10h-5z",
  "align-bottom": "M3 20h18M5 4h5v12H5zM14 8h5v8h-5z",
  "size-min-width": "M7 4v16M17 4v16M1 12h5m-3-3 3 3-3 3M23 12h-5m3-3-3 3 3 3",
  "size-max-width": "M3 4v16M21 4v16M11 12H4m3-3-3 3 3 3M13 12h7m-3-3 3 3-3 3",
  "size-min-height": "M4 7h16M4 17h16M12 1v5m-3-3 3 3 3-3M12 23v-5m-3 3 3-3 3 3",
  "size-max-height": "M4 3h16M4 21h16M12 11V4m-3 3 3-3 3 3M12 13v7m-3-3 3 3 3-3",
  "distribute-horizontal": "M2 4h4v11H2zM10 6h4v9h-4zM18 4h4v11h-4zM3 20h18m-3-3 3 3-3 3M6 17l-3 3 3 3",
  "distribute-vertical": "M4 2h11v4H4zM6 10h9v4H6zM4 18h11v4H4zM20 3v18m-3-3 3 3 3-3M17 6l3-3 3 3",
};

function GroupOption({
  label,
  values,
  onChange,
}: {
  label: string;
  values: boolean[];
  onChange: (value: boolean) => void;
}) {
  const all = values.every(Boolean);
  const mixed = !all && values.some(Boolean);
  return (
    <label className="group-lock-toggle">
      {label}
      <input
        type="checkbox"
        aria-label={label}
        checked={all}
        ref={(input) => {
          if (input) input.indeterminate = mixed;
        }}
        onChange={(event) => onChange(event.target.checked)}
      />
      <small className="group-option-state">{mixed ? "Mixed" : all ? "On for all" : "Off for all"}</small>
    </label>
  );
}

export function CanvasLayoutInspector({
  items,
  onAction,
  groups = [],
  onGroupOptionsChange,
}: {
  items: CanvasRect[];
  onAction: (action: LayoutAction) => void;
  groups?: CanvasGroup[];
  onGroupOptionsChange?: (patch: Partial<Pick<CanvasGroup, "locked" | "moveContents">>) => void;
}) {
  const allLocked = items.every((item) => item.locked);
  const buttons = (entries: [LayoutAction, string][], disabled = false) =>
    entries.map(([action, label]) => (
      <button
        key={action}
        type="button"
        className="secondary"
        disabled={allLocked || disabled}
        onClick={() => onAction(action)}
      >
        <svg className="canvas-layout-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
          <path d={iconPaths[action]} />
        </svg>
        <span>{label}</span>
      </button>
    ));
  return (
    <section className="panel inspector canvas-selection-inspector" aria-label="Selection layout">
      <p className="eyebrow">Canvas selection</p>
      <h2>{items.length} items selected</h2>
      <p className="muted">
        Arrange the selected tools and groups. Locked groups stay fixed. Select more items with Ctrl-click or
        Shift-drag.
      </p>
      {groups.length > 0 && onGroupOptionsChange && (
        <fieldset>
          <legend>Selected groups ({groups.length})</legend>
          <div className="group-position-options">
            <GroupOption
              label="Lock position and size"
              values={groups.map((group) => group.locked === true)}
              onChange={(locked) => onGroupOptionsChange({ locked })}
            />
            <GroupOption
              label="Move contents with group"
              values={groups.map((group) => group.moveContents === true)}
              onChange={(moveContents) => onGroupOptionsChange({ moveContents })}
            />
          </div>
          <small>Applies only to the selected groups. Tools keep their own settings.</small>
        </fieldset>
      )}
      <fieldset>
        <legend>Align</legend>
        <div className="canvas-layout-grid">
          {buttons([
            ["align-left", "Align left"],
            ["align-center", "Align center"],
            ["align-right", "Align right"],
            ["align-top", "Align top"],
            ["align-middle", "Align middle"],
            ["align-bottom", "Align bottom"],
          ])}
        </div>
      </fieldset>
      <fieldset>
        <legend>Match size</legend>
        <div className="canvas-layout-grid">
          {buttons([
            ["size-min-width", "Smallest width"],
            ["size-max-width", "Largest width"],
            ["size-min-height", "Smallest height"],
            ["size-max-height", "Largest height"],
          ])}
        </div>
        <small>Tool content keeps its minimum usable size. Group contents are not scaled.</small>
      </fieldset>
      <fieldset>
        <legend>Equal spacing</legend>
        <div className="canvas-layout-grid">
          {buttons(
            [
              ["distribute-horizontal", "Space horizontally"],
              ["distribute-vertical", "Space vertically"],
            ],
            items.length < 3,
          )}
        </div>
        <small>Keeps the endpoints and locked anchors fixed, then equalizes the gaps between edges.</small>
      </fieldset>
    </section>
  );
}
