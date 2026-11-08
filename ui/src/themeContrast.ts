import { THEME_COLORS, type CustomTheme, type ThemeColor } from "./customTheme";

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
  { fore: "success", back: "surface", minimum: 3 },
  { fore: "accent", back: "surface", minimum: 3 },
  { fore: "border", back: "surface", minimum: 1.5 },
];

function luminance(hex: string): number | null {
  const match = /^#([0-9a-f]{6})$/i.exec(hex);
  if (!match) return null;
  const value = Number.parseInt(match[1], 16);
  const channel = (shift: number) => {
    const v = ((value >> shift) & 255) / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0);
}

export function contrastRatio(foreground: string, background: string): number | null {
  const a = luminance(foreground);
  const b = luminance(background);
  if (a === null || b === null) return null;
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

export type ContrastWarning = { fore: ThemeColor; back: ThemeColor; ratio: number; minimum: number; text: string };

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
      text: `${THEME_COLORS[fore]} on ${THEME_COLORS[back].toLowerCase()}: ${ratio.toFixed(1)}:1, needs ${minimum}:1`,
    });
  }
  return warnings;
}
