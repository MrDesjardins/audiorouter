import { useRef, type KeyboardEvent, type PointerEvent } from "react";
import { clampSidebarWidth, DEFAULT_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH, MIN_SIDEBAR_WIDTH } from "./preferences";

const KEYBOARD_STEP = 24;

/**
 * Drag handle on the left edge of the right sidebar. Dragging left widens
 * the sidebar (for example for Advanced EQ bands); arrow keys resize it in
 * steps, Home/End jump to the limits and double-click restores the default.
 * The new width is reported continuously and persisted by the caller.
 */
export function SidebarResizer({ width, onWidth }: { width: number; onWidth: (width: number) => void }) {
  const drag = useRef<{ startX: number; startWidth: number } | null>(null);
  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    drag.current = { startX: event.clientX, startWidth: width };
    event.currentTarget.setPointerCapture(event.pointerId);
  };
  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    if (!drag.current) return;
    // The sidebar is on the right: moving the handle left makes it wider.
    const viewportLimit = typeof window === "undefined" ? MAX_SIDEBAR_WIDTH : window.innerWidth * 0.7;
    onWidth(Math.min(viewportLimit, clampSidebarWidth(drag.current.startWidth + (drag.current.startX - event.clientX))));
  };
  const onPointerUp = (event: PointerEvent<HTMLDivElement>) => {
    drag.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const next = event.key === "ArrowLeft" ? width + KEYBOARD_STEP
      : event.key === "ArrowRight" ? width - KEYBOARD_STEP
        : event.key === "Home" ? MIN_SIDEBAR_WIDTH
          : event.key === "End" ? MAX_SIDEBAR_WIDTH
            : null;
    if (next === null) return;
    event.preventDefault();
    onWidth(clampSidebarWidth(next));
  };
  return <div
    className="sidebar-resizer"
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize the right panel"
    aria-valuemin={MIN_SIDEBAR_WIDTH}
    aria-valuemax={MAX_SIDEBAR_WIDTH}
    aria-valuenow={width}
    title="Drag to resize the panel. Double-click to reset."
    tabIndex={0}
    onPointerDown={onPointerDown}
    onPointerMove={onPointerMove}
    onPointerUp={onPointerUp}
    onPointerCancel={onPointerUp}
    onKeyDown={onKeyDown}
    onDoubleClick={() => onWidth(DEFAULT_SIDEBAR_WIDTH)}
  />;
}
