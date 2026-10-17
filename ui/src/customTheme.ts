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

/** Several named custom themes, one of them selected. Stored per PC. */
export type CustomThemeEntry = { id: string; theme: CustomTheme };
export type CustomThemeLibrary = { selectedId: string; themes: CustomThemeEntry[] };
const LIBRARY_KEY = "audiorouter.ui.custom-themes";
export const MAX_CUSTOM_THEMES = 24;
const THEME_ID = /^[a-z0-9-]{1,40}$/;

function newThemeId(themes: CustomThemeEntry[]): string {
  for (;;) {
    const id = "theme-" + Math.random().toString(36).slice(2, 10);
    if (!themes.some((entry) => entry.id === id)) return id;
  }
}

/** The name itself, or "Name 2", "Name 3"… when another theme already has it. */
export function uniqueThemeName(themes: CustomThemeEntry[], name: string, exceptId?: string): string {
  const taken = new Set(themes.filter((entry) => entry.id !== exceptId).map((entry) => entry.theme.name));
  const base = name.trim() || DEFAULT_CUSTOM_THEME.name;
  if (!taken.has(base)) return base;
  for (let suffix = 2; ; suffix += 1) {
    const candidate = `${base.slice(0, 80 - String(suffix).length - 1)} ${suffix}`;
    if (!taken.has(candidate)) return candidate;
  }
}

export function selectedCustomTheme(library: CustomThemeLibrary): CustomThemeEntry | null {
  return library.themes.find((entry) => entry.id === library.selectedId) ?? null;
}

/** Add a theme (renamed if its name is taken) and select it. */
export function addCustomTheme(library: CustomThemeLibrary, theme: CustomTheme): CustomThemeLibrary {
  if (library.themes.length >= MAX_CUSTOM_THEMES)
    throw new Error(`You can keep up to ${MAX_CUSTOM_THEMES} custom themes. Delete one first.`);
  const id = newThemeId(library.themes);
  const entry = { id, theme: { ...structuredClone(theme), name: uniqueThemeName(library.themes, theme.name) } };
  return { selectedId: id, themes: [...library.themes, entry] };
}

export function updateCustomTheme(library: CustomThemeLibrary, id: string, theme: CustomTheme): CustomThemeLibrary {
  return { ...library, themes: library.themes.map((entry) => (entry.id === id ? { id, theme } : entry)) };
}

/** Remove a theme; the next remaining one (if any) becomes selected. */
export function deleteCustomTheme(library: CustomThemeLibrary, id: string): CustomThemeLibrary {
  const index = library.themes.findIndex((entry) => entry.id === id);
  if (index < 0) return library;
  const themes = library.themes.filter((entry) => entry.id !== id);
  const selectedId =
    library.selectedId === id ? (themes[Math.min(index, themes.length - 1)]?.id ?? "") : library.selectedId;
  return { selectedId, themes };
}

export function readCustomThemeLibrary(storage: Pick<Storage, "getItem"> | null): CustomThemeLibrary {
  try {
    const text = storage?.getItem(LIBRARY_KEY);
    if (text) {
      if (text.length > (MAX_CUSTOM_THEMES + 1) * MAX_THEME_BYTES) throw new Error("oversized");
      const value = JSON.parse(text) as Partial<CustomThemeLibrary>;
      const themes: CustomThemeEntry[] = [];
      for (const entry of Array.isArray(value.themes) ? value.themes.slice(0, MAX_CUSTOM_THEMES) : []) {
        const candidate = entry as Partial<CustomThemeEntry> | null;
        if (!candidate || typeof candidate.id !== "string" || !THEME_ID.test(candidate.id)) continue;
        if (themes.some((existing) => existing.id === candidate.id)) continue;
        try {
          themes.push({ id: candidate.id, theme: parseCustomTheme(JSON.stringify(candidate.theme)) });
        } catch {
          /* Skip one damaged theme; keep the others. */
        }
      }
      const selected = themes.find((entry) => entry.id === value.selectedId) ?? themes[0];
      return { selectedId: selected?.id ?? "", themes };
    }
    // Earlier versions kept a single custom theme under another key.
    const legacy = storage?.getItem(KEY);
    if (legacy) {
      const theme = parseCustomTheme(legacy);
      return { selectedId: "theme-legacy", themes: [{ id: "theme-legacy", theme }] };
    }
  } catch {
    /* Fall through to an empty library. */
  }
  return { selectedId: "", themes: [] };
}

export function writeCustomThemeLibrary(storage: Pick<Storage, "setItem"> | null, library: CustomThemeLibrary): void {
  try {
    storage?.setItem(LIBRARY_KEY, JSON.stringify(library));
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
