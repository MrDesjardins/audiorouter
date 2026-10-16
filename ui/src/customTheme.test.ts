import { describe, it, expect } from "vitest";
import {
  DEFAULT_CUSTOM_THEME,
  parseCustomTheme,
  readCustomTheme,
  customThemeStyle,
  writeCustomTheme,
} from "./customTheme";
describe("custom themes", () => {
  it("round trips the bounded appearance format", () => {
    const theme = structuredClone(DEFAULT_CUSTOM_THEME);
    theme.name = "  Studio  ";
    theme.colors.accent = "#AABBCC";
    const parsed = parseCustomTheme(JSON.stringify(theme));
    expect(parsed.name).toBe("Studio");
    expect(parsed.colors.accent).toBe("#aabbcc");
    expect(customThemeStyle(parsed)["--cyan"]).toBe("#aabbcc");
  });
  it("rejects CSS, missing colors, unsupported versions and oversized files", () => {
    for (const theme of [
      { ...DEFAULT_CUSTOM_THEME, version: 2 },
      { ...DEFAULT_CUSTOM_THEME, colors: { ...DEFAULT_CUSTOM_THEME.colors, text: "url(https://example.com)" } },
      { ...DEFAULT_CUSTOM_THEME, colors: { ...DEFAULT_CUSTOM_THEME.colors, extra: "#ffffff" } },
      { ...DEFAULT_CUSTOM_THEME, name: "" },
    ])
      expect(() => parseCustomTheme(JSON.stringify(theme))).toThrow();
    expect(() => parseCustomTheme("x".repeat(32769))).toThrow();
    expect(() => parseCustomTheme("{}")).toThrow();
  });
  it("recovers from invalid or unavailable preference storage", () => {
    expect(readCustomTheme({ getItem: () => "{invalid" })).toEqual(DEFAULT_CUSTOM_THEME);
    expect(() =>
      writeCustomTheme(
        {
          setItem: () => {
            throw Error("unavailable");
          },
        },
        DEFAULT_CUSTOM_THEME,
      ),
    ).not.toThrow();
  });
});
