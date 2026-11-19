/**
 * Language tags and resolution for the UI catalogs.
 *
 * `supported` holds the canonical BCP 47 tags discovered from
 * `locales/*.json`. Resolution never invents a tag: an unsupported preference
 * falls through to the ordered system languages and finally to English.
 */

export type LanguageDirection = "ltr" | "rtl";

export type LocaleMetadata = {
  tag: string;
  nativeName: string;
  direction: LanguageDirection;
};

/** Canonical casing: language lowercase, script Titlecase, region uppercase. */
export function normalizeLanguageTag(tag: string): string {
  const parts = tag.trim().replace(/_/g, "-").split("-").filter(Boolean);
  if (parts.length === 0) return "";
  const [language, ...rest] = parts;
  if (!/^[a-zA-Z]{2,3}$/.test(language)) return "";
  const normalized = [language.toLowerCase()];
  for (const part of rest) {
    if (/^[a-zA-Z]{4}$/.test(part)) normalized.push(part[0].toUpperCase() + part.slice(1).toLowerCase());
    else if (/^([a-zA-Z]{2}|\d{3})$/.test(part)) normalized.push(part.toUpperCase());
    else if (/^[a-zA-Z0-9]{1,8}$/.test(part)) normalized.push(part.toLowerCase());
    else return "";
  }
  return normalized.join("-");
}

function matchSupported(tag: string, supported: readonly string[]): string | null {
  const normalized = normalizeLanguageTag(tag);
  if (!normalized) return null;
  const byLower = new Map(supported.map((entry) => [entry.toLowerCase(), entry]));
  const parts = normalized.split("-");
  // Exact tag first, then progressively broader parents. This is what makes
  // `fr-CA` use `fr` and `zh-Hant-TW` try `zh-Hant` before `zh`.
  for (let length = parts.length; length >= 1; length -= 1) {
    const hit = byLower.get(parts.slice(0, length).join("-").toLowerCase());
    if (hit) return hit;
  }
  return null;
}

/**
 * The supported tag to use for a saved preference and the system language
 * order. `preference` is `null` for System default.
 */
export function resolveLanguage(
  preference: string | null,
  systemLanguages: readonly string[],
  supported: readonly string[],
  fallback = "en",
): string {
  const explicit = preference ? matchSupported(preference, supported) : null;
  if (explicit) return explicit;
  for (const tag of systemLanguages) {
    const hit = matchSupported(tag, supported);
    if (hit) return hit;
  }
  return matchSupported(fallback, supported) ?? supported[0] ?? fallback;
}

export function browserLanguageTags(): string[] {
  if (typeof navigator === "undefined") return [];
  const languages = navigator.languages?.filter(Boolean) ?? [];
  return languages.length > 0 ? [...languages] : navigator.language ? [navigator.language] : [];
}
