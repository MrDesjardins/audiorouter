import { describe, it, expect } from "vitest";
import {
  DEFAULT_CUSTOM_THEME,
  MAX_CUSTOM_THEMES,
  addCustomTheme,
  deleteCustomTheme,
  readCustomThemeLibrary,
  selectedCustomTheme,
  updateCustomTheme,
  writeCustomThemeLibrary,
  parseCustomTheme,
  readCustomTheme,
  customThemeStyle,
  writeCustomTheme,
} from "./customTheme";
describe("custom themes", () => {
  it("round trips the bounded appearance format", () => {
    const theme = structuredClone(DEFAULT_CUSTOM_THEME);
    theme.name = "  Studio  ";
    theme.colors.accent = "#AABBCC";
    const parsed = parseCustomTheme(JSON.stringify(theme));
    expect(parsed.name).toBe("Studio");
    expect(parsed.colors.accent).toBe("#aabbcc");
    expect(customThemeStyle(parsed)["--cyan"]).toBe("#aabbcc");
  });
  it("rejects CSS, missing colors, unsupported versions and oversized files", () => {
    for (const theme of [
      { ...DEFAULT_CUSTOM_THEME, version: 2 },
      { ...DEFAULT_CUSTOM_THEME, colors: { ...DEFAULT_CUSTOM_THEME.colors, text: "url(https://example.com)" } },
      { ...DEFAULT_CUSTOM_THEME, colors: { ...DEFAULT_CUSTOM_THEME.colors, extra: "#ffffff" } },
      { ...DEFAULT_CUSTOM_THEME, name: "" },
    ])
      expect(() => parseCustomTheme(JSON.stringify(theme))).toThrow();
    expect(() => parseCustomTheme("x".repeat(32769))).toThrow();
    expect(() => parseCustomTheme("{}")).toThrow();
  });
  it("recovers from invalid or unavailable preference storage", () => {
    expect(readCustomTheme({ getItem: () => "{invalid" })).toEqual(DEFAULT_CUSTOM_THEME);
    expect(() =>
      writeCustomTheme(
        {
          setItem: () => {
            throw Error("unavailable");
          },
        },
        DEFAULT_CUSTOM_THEME,
      ),
    ).not.toThrow();
  });
});
describe("custom theme library", () => {
  const memory = () => {
    const values = new Map<string, string>();
    return {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => void values.set(key, value),
      values,
    };
  };
  it("keeps several themes with unique names and deletes only the chosen one", () => {
    let library = addCustomTheme({ selectedId: "", themes: [] }, DEFAULT_CUSTOM_THEME);
    const first = library.selectedId;
    library = addCustomTheme(library, DEFAULT_CUSTOM_THEME);
    const second = library.selectedId;
    expect(library.themes.map((entry) => entry.theme.name)).toEqual(["My theme", "My theme 2"]);
    expect(selectedCustomTheme(library)?.id).toBe(second);
    library = deleteCustomTheme(library, second);
    expect(library.themes.map((entry) => entry.id)).toEqual([first]);
    expect(library.selectedId).toBe(first);
    library = deleteCustomTheme(library, first);
    expect(library).toEqual({ selectedId: "", themes: [] });
  });
  it("bounds the number of themes", () => {
    let library = { selectedId: "", themes: [] } as ReturnType<typeof readCustomThemeLibrary>;
    for (let index = 0; index < MAX_CUSTOM_THEMES; index += 1) library = addCustomTheme(library, DEFAULT_CUSTOM_THEME);
    expect(() => addCustomTheme(library, DEFAULT_CUSTOM_THEME)).toThrow(/up to/);
  });
  it("round trips through storage, migrates the single legacy theme and skips damaged entries", () => {
    const storage = memory();
    storage.setItem("audiorouter.ui.custom-theme", JSON.stringify({ ...DEFAULT_CUSTOM_THEME, name: "Old" }));
    const migrated = readCustomThemeLibrary(storage);
    expect(migrated.themes.map((entry) => entry.theme.name)).toEqual(["Old"]);
    const library = updateCustomTheme(migrated, migrated.selectedId, { ...DEFAULT_CUSTOM_THEME, name: "Renamed" });
    writeCustomThemeLibrary(storage, library);
    expect(readCustomThemeLibrary(storage)).toEqual(library);
    const stored = JSON.parse(storage.values.get("audiorouter.ui.custom-themes")!) as {
      themes: unknown[];
    };
    stored.themes.push({ id: "theme-bad", theme: { name: "x" } }, { id: "../x", theme: DEFAULT_CUSTOM_THEME });
    storage.setItem("audiorouter.ui.custom-themes", JSON.stringify(stored));
    expect(readCustomThemeLibrary(storage).themes).toHaveLength(1);
    expect(readCustomThemeLibrary({ getItem: () => "{invalid" })).toEqual({ selectedId: "", themes: [] });
  });
});
