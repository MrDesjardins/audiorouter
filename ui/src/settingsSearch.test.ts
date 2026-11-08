import { describe, expect, it } from "vitest";
import { matchingSettingsPages, SETTINGS_PAGE_ICONS, SETTINGS_PAGES } from "./Workbench";

describe("Settings search", () => {
  it("finds pages by the words people type, not only their titles (owner, 2026-11-07)", () => {
    expect([...matchingSettingsPages("shortcut")]).toEqual(["shortcuts"]);
    expect([...matchingSettingsPages("Keyboard shortcut")]).toEqual(["shortcuts"]);
    expect(matchingSettingsPages("dark").has("uiOptions")).toBe(true);
    expect(matchingSettingsPages("vst").has("plugins")).toBe(true);
    expect(matchingSettingsPages("logs").has("diagnostics")).toBe(true);
    expect(matchingSettingsPages("zzz").size).toBe(0);
    expect(matchingSettingsPages("").size).toBe(SETTINGS_PAGES.length);
  });

  it("gives every page a distinct icon", () => {
    const icons = SETTINGS_PAGES.map(([id]) => SETTINGS_PAGE_ICONS[id]);
    expect(new Set(icons).size).toBe(icons.length);
  });
});
