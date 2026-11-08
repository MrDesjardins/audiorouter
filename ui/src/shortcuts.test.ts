/** @vitest-environment jsdom */

import { describe, expect, it } from "vitest";
import {
  defaultShortcutBinding,
  isEditableShortcutTarget,
  shortcutConflicts,
  shortcutFromKeyboardEvent,
} from "./shortcuts";

describe("keyboard shortcuts", () => {
  it("normalizes modifier and letter combinations", () => {
    expect(shortcutFromKeyboardEvent({ key: "s", ctrlKey: true, metaKey: false, altKey: true, shiftKey: false })).toBe(
      "Control+Alt+S",
    );
    expect(
      shortcutFromKeyboardEvent({ key: "Control", ctrlKey: true, metaKey: false, altKey: false, shiftKey: false }),
    ).toBeNull();
  });

  it("matches a Ctrl+Alt chord when AltGr turns the letter into a character", () => {
    // Canadian Multilingual Standard: Ctrl+Alt+M types "µ" (owner, 2026-11-07).
    const altGr = { ctrlKey: true, metaKey: false, altKey: true, shiftKey: false };
    expect(shortcutFromKeyboardEvent({ ...altGr, key: "µ", code: "KeyM" })).toBe("Control+Alt+M");
    expect(shortcutFromKeyboardEvent({ ...altGr, key: "ß", code: "KeyS" })).toBe("Control+Alt+S");
    // The defaults avoid AltGr entirely.
    const shifted = { ctrlKey: true, metaKey: false, altKey: false, shiftKey: true };
    expect(shortcutFromKeyboardEvent({ ...shifted, key: "M", code: "KeyM" })).toBe(defaultShortcutBinding.privacyMute);
    // A layout whose key types its own letter keeps that letter (AZERTY A on KeyQ).
    expect(shortcutFromKeyboardEvent({ ...altGr, key: "a", code: "KeyQ" })).toBe("Control+Alt+A");
    expect(shortcutFromKeyboardEvent({ ...altGr, key: "F5", code: "F5" })).toBe("Control+Alt+F5");
  });

  it("reports duplicate bindings", () => {
    expect(shortcutConflicts({ ...defaultShortcutBinding, privacyMute: defaultShortcutBinding.sessionToggle })).toEqual(
      ["privacyMute"],
    );
  });

  it("does not steal typing focus", () => {
    const input = document.createElement("input");
    const button = document.createElement("button");
    expect(isEditableShortcutTarget(input)).toBe(true);
    expect(isEditableShortcutTarget(button)).toBe(false);
  });
  it("reserves clipboard shortcuts so customized actions cannot stop audio while copying", () => {
    expect(shortcutConflicts({ sessionToggle: "Control+C", privacyMute: "Control+V" })).toEqual([
      "sessionToggle",
      "privacyMute",
    ]);
  });
  it("reserves Save so a customized binding cannot start audio while saving", () => {
    expect(shortcutConflicts({ ...defaultShortcutBinding, sessionToggle: "Control+S" })).toEqual(["sessionToggle"]);
  });
});
