export type ShortcutAction = "sessionToggle" | "privacyMute";

export type ShortcutBinding = Record<ShortcutAction, string>;

export const defaultShortcutBinding: ShortcutBinding = {
  sessionToggle: "Control+Alt+S",
  privacyMute: "Control+Alt+M",
};

export function shortcutFromKeyboardEvent(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey">): string | null {
  if (!(event.ctrlKey || event.metaKey || event.altKey || event.shiftKey)) return null;
  const key = event.key.length === 1 ? event.key.toUpperCase() : event.key;
  if (["Control", "Meta", "Alt", "Shift"].includes(key)) return null;
  return `${event.ctrlKey || event.metaKey ? "Control+" : ""}${event.altKey ? "Alt+" : ""}${event.shiftKey ? "Shift+" : ""}${key}`;
}

export function shortcutConflicts(binding: ShortcutBinding): ShortcutAction[] {
  const actions = Object.keys(binding) as ShortcutAction[];
  return actions.filter((action, index) => actions.some((other, otherIndex) => otherIndex < index && binding[other] === binding[action]));
}

export function isEditableShortcutTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  return element?.isContentEditable === true || ["INPUT", "TEXTAREA", "SELECT"].includes(element?.tagName ?? "");
}
