import { describe, expect, it } from "vitest";
import {
  KEYBIND_MAX,
  KEYBIND_NAME_MAX,
  capKeybindName,
  keybindIdsAlongStroke,
  keybindLimitMessage,
  paintKeybindSelection,
  chordFromCaptureKeyDown,
  chordFromCaptureModifierKeyUp,
  isValidTrigger,
  keyFromKeyboardEvent,
} from "./GlobalKeybinds";

const chord = (key: string, code: string, shiftKey = false) => ({
  key,
  code,
  shiftKey,
  ctrlKey: false,
  altKey: false,
  metaKey: false,
});

describe("global shortcut capture", () => {
  it("records shifted punctuation as the physical key", () => {
    const shift = new Set(["ShiftLeft"]);
    expect(keyFromKeyboardEvent(chord("+", "Equal", true), shift)).toBe("LShift+Equal");
    expect(keyFromKeyboardEvent(chord("?", "Slash", true), shift)).toBe("LShift+Slash");
    expect(keyFromKeyboardEvent(chord("_", "Minus", true), shift)).toBe("LShift+Minus");
    expect(keyFromKeyboardEvent(chord("{", "BracketLeft", true), shift)).toBe("LShift+BracketLeft");
    expect(keyFromKeyboardEvent(chord("A", "KeyA", true), shift)).toBe("LShift+A");
    expect(keyFromKeyboardEvent(chord("!", "Digit1", true), shift)).toBe("LShift+1");
  });

  it("keeps a typed letter when the layout's character is still a letter", () => {
    const altGr = { key: "a", code: "KeyQ", ctrlKey: true, altKey: true, shiftKey: false, metaKey: false };
    expect(keyFromKeyboardEvent(altGr, new Set(["ControlLeft", "AltRight"]))).toBe("LControl+RAlt+A");
  });

  it("ignores a key that was already held when capture started", () => {
    const pressed = new Set<string>();
    expect(chordFromCaptureKeyDown({ ...chord("F13", "F13"), repeat: true }, pressed)).toBeNull();
    expect(pressed.size).toBe(0);
    expect(chordFromCaptureModifierKeyUp("ShiftLeft", pressed, false)).toBeNull();
    expect(chordFromCaptureKeyDown({ ...chord("g", "KeyG") }, pressed)).toBe("G");
  });

  it("records a modifier that is pressed after capture starts", () => {
    const pressed = new Set<string>();
    expect(chordFromCaptureKeyDown({ ...chord("Shift", "ShiftLeft"), repeat: false }, pressed)).toBeNull();
    expect(chordFromCaptureModifierKeyUp("ShiftLeft", pressed, false)).toBe("LShift");
  });

  it("allows 100 keybinds and blocks the next one", () => {
    expect(keybindLimitMessage(KEYBIND_MAX - 1)).toBeNull();
    expect(keybindLimitMessage(KEYBIND_MAX)).toBe("You can add up to 100 global keybinds.");
  });

  it("paints a dragged selection without changing rows the pointer misses", () => {
    const selected = new Set<string>();
    const checked = paintKeybindSelection(selected, ["a"], true);
    const dragged = paintKeybindSelection(checked, ["b", "c"], true);
    expect([...dragged]).toEqual(["a", "b", "c"]);
    expect(paintKeybindSelection(dragged, ["b"], true)).toBe(dragged);
    const cleared = paintKeybindSelection(dragged, ["b", "c"], false);
    expect([...cleared]).toEqual(["a"]);
  });

  it("finds every checkbox column the pointer is over", () => {
    const boxes = [
      { id: "a", left: 0, right: 20, top: 0, bottom: 20 },
      { id: "b", left: 0, right: 20, top: 24, bottom: 44 },
    ];
    expect(keybindIdsAlongStroke(boxes, 10, 10, 10, 30)).toEqual(["a", "b"]);
    expect(keybindIdsAlongStroke(boxes, 40, 10, 40, 30)).toEqual([]);
  });

  it("caps a keybind name at 32 characters", () => {
    expect(capKeybindName("x".repeat(KEYBIND_NAME_MAX + 5))).toHaveLength(KEYBIND_NAME_MAX);
    expect(capKeybindName("slot")).toBe("slot");
  });

  it("rejects chords the parser would misread", () => {
    expect(isValidTrigger("LShift++")).toBe(false);
    expect(isValidTrigger("LShift+?")).toBe(false);
    expect(isValidTrigger("LShift+Equal")).toBe(true);
    expect(isValidTrigger("LShift+Slash")).toBe(true);
    expect(keyFromKeyboardEvent(chord("?", "IntlBackslash"), new Set())).toBeNull();
  });
});
