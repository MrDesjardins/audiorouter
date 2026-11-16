/** @vitest-environment jsdom */

import { describe, expect, it } from "vitest";
import { isEditableShortcutTarget } from "./keyboardTarget";

describe("keyboard targets", () => {
  it("does not steal typing focus", () => {
    const input = document.createElement("input");
    const textarea = document.createElement("textarea");
    const select = document.createElement("select");
    const editable = document.createElement("div");
    Object.defineProperty(editable, "isContentEditable", { value: true });
    const button = document.createElement("button");

    expect(isEditableShortcutTarget(input)).toBe(true);
    expect(isEditableShortcutTarget(textarea)).toBe(true);
    expect(isEditableShortcutTarget(select)).toBe(true);
    expect(isEditableShortcutTarget(editable)).toBe(true);
    expect(isEditableShortcutTarget(button)).toBe(false);
  });
});
