/** Keep useful native editing/copy actions, without showing browser commands
 * when a workspace panel or navigation control is right-clicked. */
export function keepNativeContextMenu(target: EventTarget | null, selection: Selection | null): boolean {
  if (!(target instanceof Element)) return false;
  if (target.closest("a[href], pre, code, textarea")) return true;
  const input = target.closest("input");
  if (
    input instanceof HTMLInputElement &&
    ["text", "search", "email", "url", "tel", "password", "number"].includes(input.type)
  )
    return true;
  const editable = target.closest("[contenteditable]");
  if (editable && editable.getAttribute("contenteditable") !== "false") return true;
  if (!selection || selection.isCollapsed) return false;
  for (let index = 0; index < selection.rangeCount; index++) {
    if (selection.getRangeAt(index).intersectsNode(target)) return true;
  }
  return false;
}
