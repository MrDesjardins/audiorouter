/**
 * Scroll behaviour that honours the reduced-motion setting. A `behavior:
 * "smooth"` option passed from script overrides the global CSS rule, so every
 * scripted scroll must ask here instead (Motion criterion, 2026-11-07).
 */
export function scrollBehavior(): ScrollBehavior {
  return window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth";
}
