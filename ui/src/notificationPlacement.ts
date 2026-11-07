import { useEffect, useRef } from "react";

const NARROW_LAYOUT_MAX_WIDTH = 900;
/** Matches the 1rem inset of `.workspace-notifications`. */
const CORNER_INSET = 16;
const GAP_BELOW_TOP_BAR = 8;
const RECHECK_MS = 250;

/** Whether a message in the bottom corner would cover this row of tabs. */
export function cornerCoversTabs(
  tabs: { top: number; bottom: number },
  messageHeight: number,
  viewportHeight: number,
): boolean {
  const messageBottom = viewportHeight - CORNER_INSET;
  return tabs.top < messageBottom && tabs.bottom > messageBottom - messageHeight;
}

/** Where a message moved off the tabs goes: the top of the window, below the top bar when it is showing. */
export function topPlacement(topBarBottom: number): number {
  return Math.max(CORNER_INSET, topBarBottom + GAP_BELOW_TOP_BAR);
}

/**
 * In a narrow window the page scrolls and the tabs move, so a message fixed in
 * the bottom corner can sit on top of them. While they overlap, place it at the
 * top instead, clear of the top bar. Wide layouts keep it over the canvas. Styled
 * directly: this follows scrolling, and must not re-render the app on each frame.
 */
export function useNotificationsClearTabs(contentKey: string) {
  const ref = useRef<HTMLElement>(null);
  useEffect(() => {
    let frame = 0;
    const measure = () => {
      frame = 0;
      const message = ref.current;
      if (!message) return;
      // The wrapper: the tablist inside it lays out as `display: contents` and measures zero.
      const tabs = document.querySelector(".workbench-tabs");
      const topBar = document.querySelector(".topbar");
      const moveToTop =
        tabs !== null &&
        window.innerWidth <= NARROW_LAYOUT_MAX_WIDTH &&
        cornerCoversTabs(tabs.getBoundingClientRect(), message.offsetHeight, window.innerHeight);
      message.style.top = moveToTop ? `${topPlacement(topBar?.getBoundingClientRect().bottom ?? 0)}px` : "";
      message.style.bottom = moveToTop ? "auto" : "";
    };
    const schedule = () => {
      if (!frame) frame = window.requestAnimationFrame(measure);
    };
    schedule();
    window.addEventListener("resize", schedule);
    // Capture: inner panels scroll too, and scroll events do not bubble.
    window.addEventListener("scroll", schedule, { capture: true, passive: true });
    // The tabs also move, or first appear after loading, with no scroll or resize to announce it.
    // Nothing to observe exists yet at that point, so narrow windows simply look again.
    const recheck = window.setInterval(() => {
      if (window.innerWidth <= NARROW_LAYOUT_MAX_WIDTH) schedule();
    }, RECHECK_MS);
    return () => {
      window.clearInterval(recheck);
      window.cancelAnimationFrame(frame);
      window.removeEventListener("resize", schedule);
      window.removeEventListener("scroll", schedule, { capture: true });
    };
    // Measured again when the content changes: a longer message reaches higher.
  }, [contentKey]);
  return ref;
}
