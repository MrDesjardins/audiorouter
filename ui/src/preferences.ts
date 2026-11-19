import { normalizeLanguageTag } from "./i18n/resolve";

export type ThemeMode = "dark" | "light" | "high-contrast" | "custom";

const THEME_KEY = "audiorouter.ui.theme";
const LAST_SESSION_KEY = "audiorouter.ui.last-session";
/** BCP 47 UI language; absent means System default. */
const LANGUAGE_KEY = "audiorouter.ui.language";

export function readLanguagePreference(storage: Pick<Storage, "getItem"> | null): string | null {
  try {
    const value = storage?.getItem(LANGUAGE_KEY);
    if (!value || value.length > 64) return null;
    const normalized = normalizeLanguageTag(value);
    // A denied or corrupt preference falls back to System default.
    return normalized || null;
  } catch {
    return null;
  }
}

/** Save an explicit language, or remove the preference for System default. */
export function writeLanguagePreference(
  storage: Pick<Storage, "setItem" | "removeItem"> | null,
  tag: string | null,
): void {
  try {
    if (!storage) return;
    if (!tag) {
      storage.removeItem(LANGUAGE_KEY);
      return;
    }
    const normalized = normalizeLanguageTag(tag);
    if (normalized) storage.setItem(LANGUAGE_KEY, normalized);
  } catch {
    /* Optional presentation preference. */
  }
}

export function readLastSession(storage: Pick<Storage, "getItem"> | null): string | null {
  try {
    const value = storage?.getItem(LAST_SESSION_KEY);
    return value && value.length <= 256 && value.trim() === value ? value : null;
  } catch {
    return null;
  }
}

export function writeLastSession(storage: Pick<Storage, "setItem"> | null, sessionId: string): void {
  if (!sessionId || sessionId.length > 256 || sessionId.trim() !== sessionId) return;
  try {
    storage?.setItem(LAST_SESSION_KEY, sessionId);
  } catch {
    /* Optional workspace preference. */
  }
}

/** Moving lights on live connections: always, only while AudioRouter is the
 * active window, or never. Moving lights keep the window drawing at 60 fps,
 * so the default pauses them while another program (a game) is in front:
 * 46% → 17% of a core of GPU time with 8 live connections (2026-10-14). */
export type FlowAnimationMode = "on" | "focus" | "off";
const FLOW_ANIMATION_KEY = "audiorouter.ui.flow-animation";

export function readFlowAnimation(storage: Pick<Storage, "getItem"> | null): FlowAnimationMode {
  try {
    const value = storage?.getItem(FLOW_ANIMATION_KEY);
    return value === "on" || value === "off" ? value : "focus";
  } catch {
    return "focus";
  }
}

export function writeFlowAnimation(storage: Pick<Storage, "setItem"> | null, mode: FlowAnimationMode): void {
  try {
    storage?.setItem(FLOW_ANIMATION_KEY, mode);
  } catch {
    /* Optional presentation preference. */
  }
}

/** Whether parameter edits are saved automatically while audio plays. A
 * topology edit still needs Save, and enabled/bypass changes remain live. */
const LIVE_AUTOSAVE_KEY = "audiorouter.ui.live-autosave";

export function readLiveAutosave(storage: Pick<Storage, "getItem"> | null): boolean {
  try {
    return storage?.getItem(LIVE_AUTOSAVE_KEY) !== "off";
  } catch {
    return true;
  }
}

export function writeLiveAutosave(storage: Pick<Storage, "setItem"> | null, enabled: boolean): void {
  try {
    storage?.setItem(LIVE_AUTOSAVE_KEY, enabled ? "on" : "off");
  } catch {
    /* Optional workflow preference. */
  }
}

export function readTheme(storage: Pick<Storage, "getItem"> | null): ThemeMode {
  const value = storage?.getItem(THEME_KEY);
  return value === "light" || value === "high-contrast" || value === "custom" ? value : "dark";
}

export function writeTheme(storage: Pick<Storage, "setItem"> | null, theme: ThemeMode): void {
  try {
    storage?.setItem(THEME_KEY, theme);
  } catch {
    // Presentation preferences are optional and must never block the editor.
  }
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
  } catch {
    return DEFAULT_SIDEBAR_WIDTH;
  }
}

export function writeSidebarWidth(storage: Pick<Storage, "setItem"> | null, width: number): void {
  try {
    storage?.setItem(SIDEBAR_WIDTH_KEY, String(clampSidebarWidth(width)));
  } catch {
    /* Optional layout preference. */
  }
}
