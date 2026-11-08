import { describe, expect, it } from "vitest";
import { readShortcuts, readTheme, writeTheme } from "./preferences";
import { defaultShortcutBinding } from "./shortcuts";

function storage() {
  const values = new Map<string, string>();
  return {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
  };
}

describe("theme preferences", () => {
  it("round-trips supported themes", () => {
    const state = storage();
    writeTheme(state, "high-contrast");
    expect(readTheme(state)).toBe("high-contrast");
  });

  it("fails closed to dark for missing or invalid values", () => {
    const state = storage();
    expect(readTheme(state)).toBe("dark");
    state.setItem("audiorouter.ui.theme", "neon");
    expect(readTheme(state)).toBe("dark");
  });
});

describe("shortcut preferences", () => {
  it("moves an untouched old Ctrl+Alt default to the Ctrl+Shift default but keeps a chosen key", () => {
    const saved = storage();
    saved.setItem(
      "audiorouter.ui.shortcuts",
      JSON.stringify({ sessionToggle: "Control+F12", privacyMute: "Control+Alt+M" }),
    );
    expect(readShortcuts(saved, defaultShortcutBinding)).toEqual({
      sessionToggle: "Control+F12",
      privacyMute: "Control+Shift+M",
    });
    expect(defaultShortcutBinding).toEqual({ sessionToggle: "Control+Shift+S", privacyMute: "Control+Shift+M" });
  });
});
