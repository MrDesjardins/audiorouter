import { describe, expect, it } from "vitest";
import { DEFAULT_CUSTOM_THEME } from "./customTheme";
import { contrastRatio, themeContrastWarnings } from "./themeContrast";

describe("custom theme contrast", () => {
  it("measures the WCAG ratio", () => {
    expect(contrastRatio("#000000", "#ffffff")).toBeCloseTo(21, 5);
    expect(contrastRatio("#777777", "#777777")).toBeCloseTo(1, 5);
    expect(contrastRatio("red", "#ffffff")).toBeNull();
  });

  it("finds nothing wrong with the starting theme", () => {
    expect(themeContrastWarnings(DEFAULT_CUSTOM_THEME)).toEqual([]);
  });

  it("checks error text on buttons, which sit on raised surfaces", () => {
    // The first default (#ff6b6b) passed on panels but gave 4.26:1 on the Delete theme button.
    const theme = { ...DEFAULT_CUSTOM_THEME, colors: { ...DEFAULT_CUSTOM_THEME.colors, danger: "#ff6b6b" } };
    expect(themeContrastWarnings(theme).map((warning) => `${warning.fore}/${warning.back}`)).toEqual(["danger/raised"]);
  });

  it("names a faint pair in plain words", () => {
    const theme = { ...DEFAULT_CUSTOM_THEME, colors: { ...DEFAULT_CUSTOM_THEME.colors, muted: "#3a414b" } };
    const warnings = themeContrastWarnings(theme);
    expect(warnings.map((warning) => `${warning.fore}/${warning.back}`)).toEqual([
      "muted/background",
      "muted/surface",
      "muted/raised",
    ]);
    expect(warnings[0].text).toMatch(/^Secondary text on app background: 1\.\d:1, needs 4\.5:1$/);
  });
});
