import type { ParameterSpec } from "./store.js";

/** Round to the step's precision so repeated presses never drift. */
export function stepValue(current: number, step: number, mode: "add" | "set", spec: ParameterSpec | undefined): number {
  const decimals = Math.min(3, (String(Math.abs(step)).split(".")[1] ?? "").length);
  const raw = mode === "set" ? step : current + step;
  const bounded = Math.min(spec?.maximum ?? Infinity, Math.max(spec?.minimum ?? -Infinity, raw));
  return Number(bounded.toFixed(decimals));
}

export function formatValue(value: number, unit: string | undefined): string {
  const text = Number.isInteger(value) ? String(value) : value.toFixed(1);
  return unit === "%" ? `${text}%` : unit ? `${text} ${unit}` : text;
}
