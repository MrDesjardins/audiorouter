import { describe, expect, it } from "vitest";
import { normalizeLanguageTag, resolveLanguage } from "./resolve";

const supported = ["de", "en", "fr", "pt-BR", "zh-Hant", "zh-Hans"];

describe("language tags", () => {
  it("canonicalizes casing without changing meaning", () => {
    expect(normalizeLanguageTag("pt-br")).toBe("pt-BR");
    expect(normalizeLanguageTag("zh_hant_tw")).toBe("zh-Hant-TW");
    expect(normalizeLanguageTag("EN")).toBe("en");
    expect(normalizeLanguageTag("not a tag")).toBe("");
  });

  it("prefers an exact saved choice before anything else", () => {
    expect(resolveLanguage("pt-BR", ["fr-FR"], supported)).toBe("pt-BR");
    expect(resolveLanguage("PT-br", ["fr-FR"], supported)).toBe("pt-BR");
  });

  it("matches a parent and then a script parent before giving up", () => {
    expect(resolveLanguage("fr-CA", [], supported)).toBe("fr");
    expect(resolveLanguage("zh-Hant-TW", [], supported)).toBe("zh-Hant");
    expect(resolveLanguage("zh-Hans-CN", [], supported)).toBe("zh-Hans");
    // No arbitrary regional substitution: pt-PT is not served by pt-BR.
    expect(resolveLanguage("pt-PT", [], supported)).toBe("en");
  });

  it("uses the ordered system languages when no saved choice applies", () => {
    expect(resolveLanguage(null, ["ja-JP", "de-AT", "fr-FR"], supported)).toBe("de");
    expect(resolveLanguage("ja", ["de-AT"], supported)).toBe("de");
  });

  it("recovers unsupported or missing choices to English", () => {
    expect(resolveLanguage("xx", [], supported)).toBe("en");
    expect(resolveLanguage("", [], supported)).toBe("en");
    expect(resolveLanguage(null, ["ja"], supported)).toBe("en");
    expect(resolveLanguage(null, [], [])).toBe("en");
  });
});
