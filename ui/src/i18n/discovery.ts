import { normalizeLanguageTag, type LanguageDirection, type LocaleMetadata } from "./resolve";
import { pseudolocalizeMessages } from "./pseudo";

export const PSEUDO_LOCALE_TAG = "en-XA";

export type CatalogResource = Record<string, unknown>;

type LocaleFile = {
  meta?: unknown;
  messages?: unknown;
};

const localeFiles = import.meta.glob("./locales/*.json", { eager: true, import: "default" }) as Record<
  string,
  LocaleFile
>;

function direction(value: unknown): LanguageDirection | null {
  return value === "ltr" || value === "rtl" ? value : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Read the bundled catalogs. A malformed file is skipped with a development
 * warning: one damaged translation must not blank the interface.
 */
export function discoverLocaleCatalogs(): {
  locales: LocaleMetadata[];
  resources: Record<string, CatalogResource>;
} {
  const locales: LocaleMetadata[] = [];
  const resources: Record<string, CatalogResource> = {};
  const entries = Object.entries(localeFiles)
    .map(([path, file]) => ({
      path,
      file,
      base:
        path
          .split(/[\\/]/)
          .pop()
          ?.replace(/\.json$/i, "") ?? "",
    }))
    .sort((left, right) => left.base.localeCompare(right.base));
  for (const { path, file, base } of entries) {
    const tag = normalizeLanguageTag(base);
    const meta = isRecord(file.meta) ? file.meta : null;
    const nativeName = typeof meta?.nativeName === "string" ? meta.nativeName.trim() : "";
    const writingDirection = direction(meta?.direction);
    if (!tag || tag !== base || !nativeName || !writingDirection || !isRecord(file.messages)) {
      if (import.meta.env.DEV) console.warn(`[i18n] Ignoring malformed language file ${path}`);
      continue;
    }
    locales.push({ tag, nativeName, direction: writingDirection });
    resources[tag] = file.messages;
  }
  const english = resources.en;
  if (import.meta.env.DEV && english) {
    // Development builds only; never shipped and never used as translation
    // evidence. Selecting it exposes hardcoded strings and cramped layout.
    resources[PSEUDO_LOCALE_TAG] = pseudolocalizeMessages(english) as CatalogResource;
    locales.push({ tag: PSEUDO_LOCALE_TAG, nativeName: "Pseudo (expanded)", direction: "ltr" });
  }
  return { locales, resources };
}

export const discoveredLocales = discoverLocaleCatalogs();
