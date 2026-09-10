/** @vitest-environment jsdom */

import { describe, expect, it } from "vitest";
import { defaultShortcutBinding, isEditableShortcutTarget, shortcutConflicts, shortcutFromKeyboardEvent } from "./shortcuts";

describe("keyboard shortcuts", () => {
  it("normalizes modifier and letter combinations", () => {
    expect(shortcutFromKeyboardEvent({ key: "s", ctrlKey: true, metaKey: false, altKey: true, shiftKey: false })).toBe("Control+Alt+S");
    expect(shortcutFromKeyboardEvent({ key: "Control", ctrlKey: true, metaKey: false, altKey: false, shiftKey: false })).toBeNull();
  });

  it("reports duplicate bindings", () => {
    expect(shortcutConflicts({ ...defaultShortcutBinding, privacyMute: defaultShortcutBinding.sessionToggle })).toEqual(["privacyMute"]);
  });

  it("does not steal typing focus", () => {
    const input = document.createElement("input");
    const button = document.createElement("button");
    expect(isEditableShortcutTarget(input)).toBe(true);
    expect(isEditableShortcutTarget(button)).toBe(false);
  });
});
