import { describe, expect, it } from "vitest";
import { readCompactStatus, readTheme, writeCompactStatus, writeTheme } from "./preferences";

function storage() {
  const values = new Map<string, string>();
  return { getItem: (key: string) => values.get(key) ?? null, setItem: (key: string, value: string) => values.set(key, value) };
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

describe("compact status preference", () => {
  it("round-trips enabled state and fails closed for missing or invalid values", () => {
    const state = storage();
    expect(readCompactStatus(state)).toBe(false);
    writeCompactStatus(state, true);
    expect(readCompactStatus(state)).toBe(true);
    state.setItem("audiorouter.ui.compact-status", "yes");
    expect(readCompactStatus(state)).toBe(false);
  });
});
