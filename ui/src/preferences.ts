export type ThemeMode = "dark" | "light" | "high-contrast";
import type { ShortcutBinding } from "./shortcuts";

const THEME_KEY = "audiorouter.ui.theme";
const SHORTCUT_KEY = "audiorouter.ui.shortcuts";

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

export function readShortcuts(storage: Pick<Storage, "getItem"> | null, fallback: ShortcutBinding): ShortcutBinding {
  try {
    const value = storage?.getItem(SHORTCUT_KEY);
    if (!value) return fallback;
    const parsed = JSON.parse(value) as Partial<ShortcutBinding>;
    return parsed.sessionToggle && parsed.privacyMute
      ? { sessionToggle: parsed.sessionToggle, privacyMute: parsed.privacyMute }
      : fallback;
  } catch {
    return fallback;
  }
}

export function writeShortcuts(storage: Pick<Storage, "setItem"> | null, binding: ShortcutBinding): void {
  try { storage?.setItem(SHORTCUT_KEY, JSON.stringify(binding)); } catch { /* Optional presentation preference. */ }
}
