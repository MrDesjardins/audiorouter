import { themeColorLabel, type CustomTheme, type ThemeColor } from "./customTheme";
import { tx } from "./i18n";

// WCAG 1.4.3 / 1.4.11: text needs 4.5:1; state colours and accents used for marks and
// large labels need 3:1. Each pair is a foreground the app draws on a backdrop.
const PAIRS: { fore: ThemeColor; back: ThemeColor; minimum: number }[] = [
  { fore: "text", back: "background", minimum: 4.5 },
  { fore: "text", back: "surface", minimum: 4.5 },
  { fore: "text", back: "raised", minimum: 4.5 },
  { fore: "muted", back: "background", minimum: 4.5 },
  { fore: "muted", back: "surface", minimum: 4.5 },
  { fore: "muted", back: "raised", minimum: 4.5 },
  { fore: "danger", back: "surface", minimum: 4.5 },
  { fore: "danger", back: "raised", minimum: 4.5 },
  { fore: "success", back: "surface", minimum: 3 },
  { fore: "accent", back: "surface", minimum: 3 },
  { fore: "border", back: "surface", minimum: 1.5 },
];

function luminance(hex: string): number | null {
  const match = /^#([0-9a-f]{6})$/i.exec(hex);
  if (!match) return null;
  const value = Number.parseInt(match[1], 16);
  return rgbLuminance((value >> 16) & 255, (value >> 8) & 255, value & 255);
}

/** WCAG relative luminance for an sRGB colour with 0..255 channels. */
export function rgbLuminance(red: number, green: number, blue: number): number {
  const channel = (value: number) => {
    const v = value / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(red) + 0.7152 * channel(green) + 0.0722 * channel(blue);
}

export function contrastFromLuminance(first: number, second: number): number {
  return (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05);
}

export function contrastRatio(foreground: string, background: string): number | null {
  const a = luminance(foreground);
  const b = luminance(background);
  if (a === null || b === null) return null;
  return contrastFromLuminance(a, b);
}

export type ContrastWarning = { fore: ThemeColor; back: ThemeColor; ratio: number; minimum: number; text: string };

/** One contrast rule that involves the colour being edited. */
export type ContrastRequirement = {
  fore: ThemeColor;
  back: ThemeColor;
  /** Which side of the rule the edited colour is. */
  candidate: "fore" | "back";
  /** The fixed colour on the other side, as a six-digit hex value. */
  other: string;
  minimum: number;
};

/**
 * Every contrast rule that a palette entry takes part in. A foreground entry
 * is checked against each backdrop it is drawn on; a backdrop entry is
 * checked against each foreground drawn on it.
 */
export function contrastRequirementsFor(theme: CustomTheme, key: ThemeColor): ContrastRequirement[] {
  const requirements: ContrastRequirement[] = [];
  for (const { fore, back, minimum } of PAIRS) {
    if (fore === key) requirements.push({ fore, back, candidate: "fore", other: theme.colors[back], minimum });
    else if (back === key) requirements.push({ fore, back, candidate: "back", other: theme.colors[fore], minimum });
  }
  return requirements;
}

/** Contrast ratio of a candidate hex colour for one rule. */
export function contrastForRequirement(candidate: string, requirement: ContrastRequirement): number | null {
  return requirement.candidate === "fore"
    ? contrastRatio(candidate, requirement.other)
    : contrastRatio(requirement.other, candidate);
}

export function meetsRequirement(candidate: string, requirement: ContrastRequirement): boolean {
  const ratio = contrastForRequirement(candidate, requirement);
  return ratio !== null && ratio >= requirement.minimum;
}

export function themeContrastWarnings(theme: CustomTheme): ContrastWarning[] {
  const warnings: ContrastWarning[] = [];
  for (const { fore, back, minimum } of PAIRS) {
    const ratio = contrastRatio(theme.colors[fore], theme.colors[back]);
    if (ratio === null || ratio >= minimum) continue;
    warnings.push({
      fore,
      back,
      ratio,
      minimum,
      text: tx("h2.contrastText", {
        fore: themeColorLabel(fore),
        back: themeColorLabel(back).toLowerCase(),
        ratio: ratio.toFixed(1),
        minimum,
      }),
    });
  }
  return warnings;
}
