import { useLayoutEffect, type RefObject } from "react";

/** SVG font sizes use viewBox units. Keep chart labels at the app's 11 CSS-pixel
 * floor after that transform, without enlarging or reflowing the chart itself. */
export function useSvgTextFloor(root: RefObject<HTMLElement | null>, scope: string, active: boolean) {
  useLayoutEffect(() => {
    const panel = root.current;
    if (!active || !panel || typeof ResizeObserver === "undefined") return;
    const charts = new Set<SVGSVGElement>();
    const update = (svg: SVGSVGElement) => {
      if (typeof svg.getScreenCTM !== "function" || svg.clientWidth <= 0 || svg.clientHeight <= 0) return;
      const matrix = svg.getScreenCTM();
      if (!matrix) return;
      const scale = Math.hypot(matrix.c, matrix.d);
      if (!Number.isFinite(scale) || scale <= 0) return;
      const value = `${11 / Math.min(1, scale)}px`;
      if (svg.style.getPropertyValue("--svg-text-floor") !== value) svg.style.setProperty("--svg-text-floor", value);
    };
    const resize = new ResizeObserver((entries) => {
      for (const entry of entries) update(entry.target as SVGSVGElement);
    });
    const register = (svg: SVGSVGElement) => {
      if (charts.has(svg) || !svg.querySelector("text")) return;
      charts.add(svg);
      resize.observe(svg);
      update(svg);
    };
    for (const svg of panel.querySelectorAll<SVGSVGElement>("svg")) register(svg);
    const mutations = new MutationObserver((records) => {
      let removed = false;
      for (const record of records) {
        for (const node of record.addedNodes) {
          if (!(node instanceof Element)) continue;
          const owner = node.closest<SVGSVGElement>("svg");
          if (owner) register(owner);
          for (const svg of node.querySelectorAll<SVGSVGElement>("svg")) register(svg);
        }
        if ([...record.removedNodes].some((node) => node instanceof Element)) removed = true;
      }
      if (removed)
        for (const svg of charts) {
          if (panel.contains(svg)) continue;
          resize.unobserve(svg);
          svg.style.removeProperty("--svg-text-floor");
          charts.delete(svg);
        }
    });
    mutations.observe(panel, { childList: true, subtree: true });
    return () => {
      mutations.disconnect();
      resize.disconnect();
      for (const svg of charts) svg.style.removeProperty("--svg-text-floor");
    };
  }, [root, scope, active]);
}
