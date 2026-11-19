import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { discoveredLocales } from "./discovery";
import { browserLanguageTags, resolveLanguage, type LocaleMetadata } from "./resolve";
import { readLanguagePreference } from "../preferences";
import { optionalLocalStorage } from "../presentationStorage";

export const FALLBACK_LANGUAGE = "en";
export const SYSTEM_LANGUAGE = "system";

const resources: Record<string, { translation: Record<string, unknown> }> = {};
for (const [tag, messages] of Object.entries(discoveredLocales.resources)) {
  resources[tag] = { translation: messages };
}

const supportedTags = discoveredLocales.locales.map((locale) => locale.tag);
if (supportedTags.length === 0) {
  // The English catalog is the reference; a build without it is broken rather
  // than silently unlocalized.
  throw new Error("The English language catalog is missing from ui/src/i18n/locales.");
}

const discoveredPreference = readLanguagePreference(optionalLocalStorage());

// Bundled resources and `initImmediate: false` mean the first frame already
// uses the selected language: there is no English flash while a file loads.
void i18n.use(initReactI18next).init({
  resources,
  lng: resolveLanguage(discoveredPreference, browserLanguageTags(), supportedTags),
  fallbackLng: FALLBACK_LANGUAGE,
  supportedLngs: supportedTags,
  initAsync: false,
  interpolation: { escapeValue: false },
  returnNull: false,
  // English is the reference; a key that exists in no catalog is a build bug.
  // Render nothing rather than leaking a raw key into the interface.
  parseMissingKeyHandler: (key) => {
    if (import.meta.env.DEV) console.warn(`[i18n] Missing message key: ${key}`);
    return "";
  },
});

export function supportedLocales(): LocaleMetadata[] {
  return discoveredLocales.locales;
}

export function supportedLanguageTags(): string[] {
  return [...supportedTags];
}

export function localeDirection(tag: string): "ltr" | "rtl" {
  return discoveredLocales.locales.find((locale) => locale.tag === tag)?.direction ?? "ltr";
}

/** Resolve a saved preference (or System default) against the live bundles. */
export function resolvedLanguage(preference: string | null): string {
  return resolveLanguage(preference, browserLanguageTags(), supportedTags);
}

/** Apply a saved preference. `null` means System default. */
export async function applyLanguage(preference: string | null): Promise<void> {
  const next = resolvedLanguage(preference);
  if (i18n.language !== next) await i18n.changeLanguage(next);
  applyDocumentLanguage();
}

/** Keep the document language and writing direction with the catalog. */
export function applyDocumentLanguage(): void {
  if (typeof document === "undefined") return;
  document.documentElement.lang = i18n.language;
  document.documentElement.dir = localeDirection(i18n.language);
}

/** Translation for pure helpers called during a render. Components use
 * `useMessages` so a language change re-renders them. */
export function tx(key: import("./generated/messageKeys").MessageKey, options?: Record<string, unknown>): string {
  return (i18n.t as unknown as (key: string, options?: Record<string, unknown>) => string)(key, options);
}

export { i18n };
