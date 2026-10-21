import type { CanvasRect, LayoutAction } from "./canvasGeometry";

export function CanvasLayoutInspector({
  items,
  onAction,
}: {
  items: CanvasRect[];
  onAction: (action: LayoutAction) => void;
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
        {label}
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
