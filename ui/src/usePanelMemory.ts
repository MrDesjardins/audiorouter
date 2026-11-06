import { useEffect, useLayoutEffect, useRef } from "react";

type PanelState = { scrollTop: number; open: Map<string, boolean> };

const summaryText = (details: HTMLDetailsElement) =>
  details.querySelector(":scope > summary")?.textContent?.trim() ?? "";

/** Lazy panels gain their height a moment after the tab opens, so restoring keeps trying briefly. */
const RESTORE_FRAMES = 20;

const stateFor = (memory: Map<string, PanelState>, tab: string) => {
  let state = memory.get(tab);
  if (!state) {
    state = { scrollTop: 0, open: new Map() };
    memory.set(tab, state);
  }
  return state;
};

/**
 * Each tab remembers its scroll position and which sections were open, for as long as the
 * window lives, because panels unmount when another tab is chosen. Sections are matched by
 * their visible title. Nothing is saved to disk.
 */
export function usePanelMemory(tab: string) {
  const panelRef = useRef<HTMLElement | null>(null);
  const memory = useRef(new Map<string, PanelState>());
  const currentTab = useRef(tab);

  useEffect(() => {
    const panel = panelRef.current;
    if (!panel) return;
    const remembered = memory.current;
    const onScroll = () => {
      stateFor(remembered, currentTab.current).scrollTop = panel.scrollTop;
    };
    const onToggle = (event: Event) => {
      const details = event.target;
      if (!(details instanceof HTMLDetailsElement) || !details.isConnected) return;
      const title = summaryText(details);
      if (title) stateFor(remembered, currentTab.current).open.set(title, details.open);
    };
    panel.addEventListener("scroll", onScroll, { passive: true });
    panel.addEventListener("toggle", onToggle, true);
    return () => {
      panel.removeEventListener("scroll", onScroll);
      panel.removeEventListener("toggle", onToggle, true);
    };
  }, []);

  useLayoutEffect(() => {
    currentTab.current = tab;
    const panel = panelRef.current;
    if (!panel) return;
    // Properties follows the selected tool, so a remembered place would belong to another tool.
    if (tab === "properties") {
      panel.scrollTop = 0;
      return;
    }
    const saved = stateFor(memory.current, tab);
    const target = saved.scrollTop;
    const open = new Map(saved.open);
    let frames = 0;
    let handle = 0;
    const restore = () => {
      for (const details of panel.querySelectorAll<HTMLDetailsElement>("details")) {
        const wanted = open.get(summaryText(details));
        if (wanted !== undefined && details.open !== wanted) details.open = wanted;
      }
      panel.scrollTop = target;
      frames += 1;
      if (panel.scrollTop < target - 1 && frames < RESTORE_FRAMES) handle = requestAnimationFrame(restore);
    };
    restore();
    return () => cancelAnimationFrame(handle);
  }, [tab]);

  return panelRef;
}
