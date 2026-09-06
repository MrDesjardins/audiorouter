export type ThemeMode = "dark" | "light" | "high-contrast";

const THEME_KEY = "audiorouter.ui.theme";

export function readTheme(storage: Pick<Storage, "getItem"> | null): ThemeMode {
  const value = storage?.getItem(THEME_KEY);
  return value === "light" || value === "high-contrast" ? value : "dark";
}

export function writeTheme(storage: Pick<Storage, "setItem"> | null, theme: ThemeMode): void {
  try {
    storage?.setItem(THEME_KEY, theme);
  } catch {
    // Presentation preferences are optional and must never block the editor.
  }
}
