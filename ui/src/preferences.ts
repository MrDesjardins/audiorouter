export type ThemeMode = "dark" | "light" | "high-contrast";
import type { ShortcutBinding } from "./shortcuts";

const THEME_KEY = "audiorouter.ui.theme";
const SHORTCUT_KEY = "audiorouter.ui.shortcuts";
const COMPACT_STATUS_KEY = "audiorouter.ui.compact-status";
const LAST_SESSION_KEY = "audiorouter.ui.last-session";

export function readLastSession(storage: Pick<Storage, "getItem"> | null): string | null {
  try {
    const value = storage?.getItem(LAST_SESSION_KEY);
    return value && value.length <= 256 && value.trim() === value ? value : null;
  } catch { return null; }
}

export function writeLastSession(storage: Pick<Storage, "setItem"> | null, sessionId: string): void {
  if (!sessionId || sessionId.length > 256 || sessionId.trim() !== sessionId) return;
  try { storage?.setItem(LAST_SESSION_KEY, sessionId); } catch { /* Optional workspace preference. */ }
}

export function readCompactStatus(storage: Pick<Storage, "getItem"> | null): boolean {
  try { return storage?.getItem(COMPACT_STATUS_KEY) === "true"; } catch { return false; }
}

export function writeCompactStatus(storage: Pick<Storage, "setItem"> | null, enabled: boolean): void {
  try { storage?.setItem(COMPACT_STATUS_KEY, String(enabled)); } catch { /* Optional presentation preference. */ }
}

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

const SIDEBAR_WIDTH_KEY = "audiorouter.ui.sidebar-width";
export const MIN_SIDEBAR_WIDTH = 320;
export const MAX_SIDEBAR_WIDTH = 960;
export const DEFAULT_SIDEBAR_WIDTH = 400;

export function clampSidebarWidth(width: number): number {
  if (!Number.isFinite(width)) return DEFAULT_SIDEBAR_WIDTH;
  return Math.round(Math.min(MAX_SIDEBAR_WIDTH, Math.max(MIN_SIDEBAR_WIDTH, width)));
}

export function readSidebarWidth(storage: Pick<Storage, "getItem"> | null): number {
  try {
    const value = storage?.getItem(SIDEBAR_WIDTH_KEY);
    return value ? clampSidebarWidth(Number(value)) : DEFAULT_SIDEBAR_WIDTH;
  } catch { return DEFAULT_SIDEBAR_WIDTH; }
}

export function writeSidebarWidth(storage: Pick<Storage, "setItem"> | null, width: number): void {
  try { storage?.setItem(SIDEBAR_WIDTH_KEY, String(clampSidebarWidth(width))); } catch { /* Optional layout preference. */ }
}
