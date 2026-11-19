import { describe, expect, it } from "vitest";
import { readLanguagePreference, readTheme, writeLanguagePreference, writeTheme } from "./preferences";

function storage() {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
    removeItem: (key: string) => values.delete(key),
  };
}

describe("language preferences", () => {
  it("round-trips an explicit language and stores it canonically", () => {
    const state = storage();
    writeLanguagePreference(state, "pt-br");
    expect(state.getItem("audiorouter.ui.language")).toBe("pt-BR");
    expect(readLanguagePreference(state)).toBe("pt-BR");
  });

  it("uses System default when cleared and ignores corrupt values", () => {
    const state = storage();
    writeLanguagePreference(state, "de");
    writeLanguagePreference(state, null);
    expect(readLanguagePreference(state)).toBeNull();
    state.setItem("audiorouter.ui.language", "not a language");
    expect(readLanguagePreference(state)).toBeNull();
    state.setItem("audiorouter.ui.language", "x".repeat(65));
    expect(readLanguagePreference(state)).toBeNull();
  });

  it("survives denied storage", () => {
    const denied = {
      getItem: () => {
        throw new Error("denied");
      },
      setItem: () => {
        throw new Error("denied");
      },
      removeItem: () => {
        throw new Error("denied");
      },
    };
    expect(readLanguagePreference(denied)).toBeNull();
    expect(() => writeLanguagePreference(denied, "en")).not.toThrow();
  });
});

describe("theme preferences", () => {
  it("round-trips supported themes", () => {
    const state = storage();
    writeTheme(state, "high-contrast");
    expect(readTheme(state)).toBe("high-contrast");
  });

  it("fails closed to dark for missing or invalid values", () => {
    const state = storage();
    expect(readTheme(state)).toBe("dark");
    state.setItem("audiorouter.ui.theme", "neon");
    expect(readTheme(state)).toBe("dark");
  });
});
