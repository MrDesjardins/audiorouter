import { describe, expect, it } from "vitest";
import { DEFAULT_CUSTOM_THEME } from "./customTheme";
import {
  contrastForRequirement,
  contrastRequirementsFor,
  meetsRequirement,
  themeContrastWarnings,
  contrastRatio,
} from "./themeContrast";

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

  it("lists every rule that involves the edited colour", () => {
    const muted = contrastRequirementsFor(DEFAULT_CUSTOM_THEME, "muted");
    expect(muted.map((requirement) => `${requirement.candidate}:${requirement.back}:${requirement.minimum}`)).toEqual([
      "fore:background:4.5",
      "fore:surface:4.5",
      "fore:raised:4.5",
    ]);
    const surface = contrastRequirementsFor(DEFAULT_CUSTOM_THEME, "surface");
    expect(surface).toHaveLength(6);
    expect(surface.every((requirement) => requirement.candidate === "back")).toBe(true);
    expect(surface.map((requirement) => requirement.fore)).toEqual([
      "text",
      "muted",
      "danger",
      "success",
      "accent",
      "border",
    ]);
    expect(contrastRequirementsFor(DEFAULT_CUSTOM_THEME, "highlight")).toEqual([]);
  });

  it("checks a candidate against both sides of a rule", () => {
    const [requirement] = contrastRequirementsFor(DEFAULT_CUSTOM_THEME, "muted");
    expect(requirement).toBeDefined();
    expect(contrastForRequirement("#aeb9c8", requirement!)).toBeGreaterThanOrEqual(4.5);
    expect(meetsRequirement("#aeb9c8", requirement!)).toBe(true);
    expect(meetsRequirement("#3a414b", requirement!)).toBe(false);
  });
});
