export const THEME_COLORS = {
  background: "App background",
  surface: "Panels",
  raised: "Fields and raised surfaces",
  border: "Borders",
  text: "Main text",
  muted: "Secondary text",
  accent: "Selection accent",
  signal: "Audio signal",
  highlight: "Primary actions",
  success: "Success",
  danger: "Errors",
  plugin: "Plugin accent",
  input: "Sending ports",
  output: "Receiving ports",
} as const;
export type ThemeColor = keyof typeof THEME_COLORS;
export type CustomTheme = {
  format: "audiorouter.theme";
  version: 1;
  name: string;
  base: "dark" | "light";
  colors: Record<ThemeColor, string>;
};
export const DEFAULT_CUSTOM_THEME: CustomTheme = {
  format: "audiorouter.theme",
  version: 1,
  name: "My theme",
  base: "dark",
  colors: {
    background: "#171b21",
    surface: "#252b33",
    raised: "#303844",
    border: "#46515f",
    text: "#f6f8fb",
    muted: "#aeb9c8",
    accent: "#25b8df",
    signal: "#f6a928",
    highlight: "#ffd166",
    success: "#72e3a2",
    danger: "#ff6b6b",
    plugin: "#c792ea",
    input: "#3a86ff",
    output: "#f6a928",
  },
};
const KEY = "audiorouter.ui.custom-theme";
export const MAX_THEME_BYTES = 32 * 1024;

export function parseCustomTheme(text: string): CustomTheme {
  if (text.length > MAX_THEME_BYTES) throw new Error("Theme files must be 32 KB or smaller.");
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    throw new Error("Choose a valid AudioRouter theme JSON file.");
  }
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("Invalid theme file.");
  const theme = value as Partial<CustomTheme>;
  if (
    theme.format !== "audiorouter.theme" ||
    theme.version !== 1 ||
    typeof theme.name !== "string" ||
    !theme.name.trim() ||
    theme.name.trim().length > 80 ||
    (theme.base !== "dark" && theme.base !== "light") ||
    !theme.colors ||
    typeof theme.colors !== "object" ||
    Array.isArray(theme.colors)
  )
    throw new Error("This is not a supported AudioRouter theme (version 1).");
  const colors = {} as Record<ThemeColor, string>;
  const known = Object.keys(THEME_COLORS) as ThemeColor[];
  if (Object.keys(theme.colors).some((key) => !known.includes(key as ThemeColor)))
    throw new Error("The theme contains unsupported color settings.");
  for (const key of known) {
    const color = theme.colors[key];
    if (typeof color !== "string" || !/^#[0-9a-f]{6}$/i.test(color))
      throw new Error(`Use a six-digit hex color for ${THEME_COLORS[key]}.`);
    colors[key] = color.toLowerCase();
  }
  return { format: "audiorouter.theme", version: 1, name: theme.name.trim(), base: theme.base, colors };
}
export function readCustomTheme(storage: Pick<Storage, "getItem"> | null): CustomTheme {
  try {
    const value = storage?.getItem(KEY);
    return value ? parseCustomTheme(value) : structuredClone(DEFAULT_CUSTOM_THEME);
  } catch {
    return structuredClone(DEFAULT_CUSTOM_THEME);
  }
}
export function writeCustomTheme(storage: Pick<Storage, "setItem"> | null, theme: CustomTheme): void {
  try {
    storage?.setItem(KEY, JSON.stringify(theme));
  } catch {
    /* Optional presentation preference. */
  }
}
export function customThemeStyle(theme: CustomTheme): Record<string, string> {
  const c = theme.colors;
  const highlightRgb = [1, 3, 5]
    .map((start) => parseInt(c.highlight.slice(start, start + 2), 16) / 255)
    .map((value) => (value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4));
  const luminance = highlightRgb[0] * 0.2126 + highlightRgb[1] * 0.7152 + highlightRgb[2] * 0.0722;
  return {
    "--on-highlight": luminance > 0.179 ? "#10131a" : "#ffffff",
    "--ambient-bg": c.background,
    "--ambient-vignette": c.background,
    "--ambient-surface": c.surface,
    "--ambient-surface-hi": c.raised,
    "--ambient-edge": c.border,
    "--text-strong": c.text,
    "--text-soft": c.muted,
    "--cyan": c.accent,
    "--cyan-deep": c.accent,
    "--signal": c.signal,
    "--signal-hot": c.highlight,
    "--good": c.success,
    "--danger": c.danger,
    "--vst-accent": c.plugin,
    "--handle-input": c.input,
    "--handle-output": c.output,
  };
}
