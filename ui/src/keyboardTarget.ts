/** True when the event target is a field where the user is typing, so a
 * window-level shortcut must leave the key alone. */
export function isEditableShortcutTarget(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;
  return element?.isContentEditable === true || ["INPUT", "TEXTAREA", "SELECT"].includes(element?.tagName ?? "");
}
