import { returnToPropertiesTab } from "./workbenchPanel";
import type { CanvasRect, LayoutAction } from "./canvasGeometry";
import type { CanvasGroup } from "./CanvasGroups";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";

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
  const tr = useMessages();
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
      <small className="group-option-state">
        {mixed ? tr("g4Panel.groupMixed") : all ? tr("g4Panel.groupOnAll") : tr("g4Panel.groupOffAll")}
      </small>
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
  const tr = useMessages();
  const allLocked = items.every((item) => item.locked);
  const buttons = (entries: [LayoutAction, MessageKey][], disabled = false) =>
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
        <span>{tr(label)}</span>
      </button>
    ));
  return (
    <section
      className="panel inspector canvas-selection-inspector"
      id="workbench-properties"
      onKeyDown={returnToPropertiesTab}
      role="tabpanel"
      aria-labelledby="workbench-tab-properties"
      tabIndex={0}
    >
      <p className="eyebrow">{tr("g4Panel.layoutEyebrow")}</p>
      <h2>{tr("canvasUi.selected", { count: items.length })}</h2>
      <p className="muted">{tr("g4Panel.layoutIntro")}</p>
      {groups.length > 0 && onGroupOptionsChange && (
        <fieldset>
          <legend>{tr("g4Panel.selectedGroups", { count: groups.length })}</legend>
          <div className="group-position-options">
            <GroupOption
              label={tr("g4Panel.lockPositionSize")}
              values={groups.map((group) => group.locked === true)}
              onChange={(locked) => onGroupOptionsChange({ locked })}
            />
            <GroupOption
              label={tr("g4Panel.moveContents")}
              values={groups.map((group) => group.moveContents === true)}
              onChange={(moveContents) => onGroupOptionsChange({ moveContents })}
            />
          </div>
          <small>{tr("g4Panel.groupsApplyNote")}</small>
        </fieldset>
      )}
      <fieldset>
        <legend>{tr("g4Panel.legendAlign")}</legend>
        <div className="canvas-layout-grid">
          {buttons([
            ["align-left", "g4Panel.alignLeft"],
            ["align-center", "g4Panel.alignCenter"],
            ["align-right", "g4Panel.alignRight"],
            ["align-top", "g4Panel.alignTop"],
            ["align-middle", "g4Panel.alignMiddle"],
            ["align-bottom", "g4Panel.alignBottom"],
          ])}
        </div>
      </fieldset>
      <fieldset>
        <legend>{tr("g4Panel.legendMatchSize")}</legend>
        <div className="canvas-layout-grid">
          {buttons([
            ["size-min-width", "g4Panel.sizeMinWidth"],
            ["size-max-width", "g4Panel.sizeMaxWidth"],
            ["size-min-height", "g4Panel.sizeMinHeight"],
            ["size-max-height", "g4Panel.sizeMaxHeight"],
          ])}
        </div>
        <small>{tr("g4Panel.matchSizeNote")}</small>
      </fieldset>
      <fieldset>
        <legend>{tr("g4Panel.legendSpacing")}</legend>
        <div className="canvas-layout-grid">
          {buttons(
            [
              ["distribute-horizontal", "g4Panel.spaceHorizontal"],
              ["distribute-vertical", "g4Panel.spaceVertical"],
            ],
            items.length < 3,
          )}
        </div>
        <small>{tr("g4Panel.spacingNote")}</small>
      </fieldset>
    </section>
  );
}
