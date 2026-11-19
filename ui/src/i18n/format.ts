import { i18n } from "./index";

/**
 * Locale-aware display formatting. These formatters never feed request values
 * or saved data: canonical serialization keeps using plain numbers and ISO
 * dates. The caches follow the active language, so a language change starts
 * new formatters without leaking old ones into later renders.
 */

const numberFormats = new Map<string, Intl.NumberFormat>();
const dateTimeFormats = new Map<string, Intl.DateTimeFormat>();
const listFormats = new Map<string, Intl.ListFormat>();

function cacheKey(options: object | undefined): string {
  return JSON.stringify(options ?? null);
}

export function formatNumber(value: number, options?: Intl.NumberFormatOptions): string {
  const locale = i18n.language;
  const key = `${locale}\u0000${cacheKey(options)}`;
  let formatter = numberFormats.get(key);
  if (!formatter) {
    formatter = new Intl.NumberFormat(locale, options);
    numberFormats.set(key, formatter);
  }
  return formatter.format(value);
}

export function formatDateTime(value: number | Date, options?: Intl.DateTimeFormatOptions): string {
  const locale = i18n.language;
  const key = `${locale}\u0000${cacheKey(options)}`;
  let formatter = dateTimeFormats.get(key);
  if (!formatter) {
    formatter = new Intl.DateTimeFormat(locale, options);
    dateTimeFormats.set(key, formatter);
  }
  return formatter.format(value);
}

export function formatList(
  values: readonly string[],
  options?: Intl.ListFormatOptions & { type?: "conjunction" | "disjunction" | "unit" },
): string {
  const locale = i18n.language;
  const key = `${locale}\u0000${cacheKey(options)}`;
  let formatter = listFormats.get(key);
  if (!formatter) {
    formatter = new Intl.ListFormat(locale, options);
    listFormats.set(key, formatter);
  }
  return formatter.format(values);
}

/** Rebuild cached formatters; call after changing language if a caller holds one. */
export function clearFormatCaches(): void {
  numberFormats.clear();
  dateTimeFormats.clear();
  listFormats.clear();
}
