import { normalizeLanguageTag, type LanguageDirection, type LocaleMetadata } from "./resolve";
import { pseudolocalizeMessages } from "./pseudo";
import { ENGLISH_MESSAGES } from "./generated/englishMessages";

export const PSEUDO_LOCALE_TAG = "en-XA";

export type CatalogResource = Record<string, unknown>;

type LocaleFile = {
  meta?: unknown;
  messages?: unknown;
};

/**
 * The Playwright test loader runs shared modules in Node with no window and
 * no Vite `import.meta.glob` transform. Node-side imports still need English
 * text, so they use the generated module; the browser discovers every file.
 */
const nodeFallback: Record<string, LocaleFile> = {
  "./locales/en.json": {
    meta: { nativeName: "English", direction: "ltr" },
    messages: ENGLISH_MESSAGES,
  },
};

function bundledLocaleFiles(): Record<string, LocaleFile> {
  if (typeof window === "undefined") return nodeFallback;
  return import.meta.glob("./locales/*.json", { eager: true, import: "default" }) as Record<string, LocaleFile>;
}

const localeFiles = bundledLocaleFiles();
const isDevelopment = Boolean(import.meta.env?.DEV);

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
      if (isDevelopment) console.warn(`[i18n] Ignoring malformed language file ${path}`);
      continue;
    }
    locales.push({ tag, nativeName, direction: writingDirection });
    resources[tag] = file.messages;
  }
  const english = resources.en;
  if (isDevelopment && english) {
    // Development builds only; never shipped and never used as translation
    // evidence. Selecting it exposes hardcoded strings and cramped layout.
    resources[PSEUDO_LOCALE_TAG] = pseudolocalizeMessages(english) as CatalogResource;
    locales.push({ tag: PSEUDO_LOCALE_TAG, nativeName: "Pseudo (expanded)", direction: "ltr" });
  }
  return { locales, resources };
}

export const discoveredLocales = discoverLocaleCatalogs();
