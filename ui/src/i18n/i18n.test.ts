/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { MESSAGE_KEYS } from "./generated/messageKeys";
import {
  applyDocumentLanguage,
  applyLanguage,
  i18n,
  localeDirection,
  resolvedLanguage,
  supportedLanguageTags,
  supportedLocales,
} from "./index";
import { discoveredLocales } from "./discovery";

describe("bundled language catalogs", () => {
  it("discovers English with metadata and every generated key", () => {
    const english = supportedLocales().find((locale) => locale.tag === "en");
    expect(english).toEqual({ tag: "en", nativeName: "English", direction: "ltr" });
    expect(supportedLanguageTags()).toContain("en");
    expect(Object.keys(discoveredLocales.resources.en).length).toBeGreaterThan(0);
    for (const key of MESSAGE_KEYS) {
      const resource = i18n.getResource("en", "translation", key);
      // Plural base keys select their suffixed form with `count`; the base
      // itself has no resource string.
      if (resource !== undefined) expect(resource, key).toBeTypeOf("string");
      expect(i18n.t(key, { count: 1 }), key).not.toBe(key);
    }
  });

  it("exposes the writing direction for document setup", () => {
    expect(localeDirection("en")).toBe("ltr");
    expect(localeDirection("xx")).toBe("ltr");
  });

  it("keeps document lang and dir with the active language", () => {
    applyDocumentLanguage();
    expect(document.documentElement.lang).toBe(i18n.language);
    expect(document.documentElement.dir).toBe("ltr");
  });
});

describe("language selection at runtime", () => {
  it("falls back to English text for an unknown key without exposing the key", () => {
    expect(resolvedLanguage(null)).toBe("en");
    const missing = i18n.t("does.not.exist") as string;
    expect(missing).not.toBe("does.not.exist");
  });

  it("applies a saved preference and returns to the resolved default", async () => {
    await applyLanguage("en");
    expect(i18n.language).toBe("en");
    expect(document.documentElement.lang).toBe("en");
    await applyLanguage(null);
    expect(supportedLanguageTags()).toContain(i18n.language);
  });
});
