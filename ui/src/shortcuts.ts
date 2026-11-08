export type ShortcutAction = "sessionToggle" | "privacyMute";

export type ShortcutBinding = Record<ShortcutAction, string>;

// Ctrl+Shift, not Ctrl+Alt: Windows treats Ctrl+Alt as AltGr, which types a
// character on many layouts, and Ctrl+Alt+M never reached the owner's window
// at all (2026-11-07).
export const defaultShortcutBinding: ShortcutBinding = {
  sessionToggle: "Control+Shift+S",
  privacyMute: "Control+Shift+M",
};

/** Earlier defaults; a saved value equal to one of these follows the new default. */
export const previousDefaultShortcutBinding: ShortcutBinding = {
  sessionToggle: "Control+Alt+S",
  privacyMute: "Control+Alt+M",
};

export function shortcutFromKeyboardEvent(
  event: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey"> &
    Partial<Pick<KeyboardEvent, "code">>,
): string | null {
  if (!(event.ctrlKey || event.metaKey || event.altKey || event.shiftKey)) return null;
  // On Windows Ctrl+Alt is AltGr, so many layouts (Canadian Multilingual:
  // Ctrl+Alt+M types "µ") report a character instead of the letter pressed.
  // Keep the typed letter when it is plain, otherwise use the physical key.
  const physical = /^(?:Key([A-Z])|Digit([0-9]))$/.exec(event.code ?? "");
  const plain = /^[A-Za-z0-9]$/.test(event.key);
  const typed = event.key.length === 1 ? event.key.toUpperCase() : event.key;
  const key = !plain && physical ? (physical[1] ?? physical[2]) : typed;
  if (["Control", "Meta", "Alt", "Shift"].includes(key)) return null;
  return `${event.ctrlKey || event.metaKey ? "Control+" : ""}${event.altKey ? "Alt+" : ""}${event.shiftKey ? "Shift+" : ""}${key}`;
}

export function shortcutConflicts(binding: ShortcutBinding): ShortcutAction[] {
  const actions = Object.keys(binding) as ShortcutAction[];
  return actions.filter(
    (action, index) =>
      ["Control+C", "Control+V", "Control+S"].includes(binding[action]) ||
      actions.some((other, otherIndex) => otherIndex < index && binding[other] === binding[action]),
  );
}

export function isEditableShortcutTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  return element?.isContentEditable === true || ["INPUT", "TEXTAREA", "SELECT"].includes(element?.tagName ?? "");
}
