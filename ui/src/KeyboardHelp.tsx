import { useEffect, useId, useRef } from "react";
import { shortcutConflicts, type ShortcutBinding } from "./shortcuts";

export function KeyboardHelp({ bindings, onClose }: { bindings: ShortcutBinding; onClose: () => void }) {
  const heading = useId();
  const close = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLDListElement>(null);
  useEffect(() => {
    const origin = document.activeElement;
    close.current?.focus();
    return () => {
      if (origin instanceof HTMLElement && origin.isConnected) origin.focus();
    };
  }, []);
  const conflict = shortcutConflicts(bindings).length > 0;
  const unavailable = conflict ? " (disabled: resolve the shortcut conflict in Settings)" : "";
  const entries = [
    ["Ctrl+S", "Save your edits"],
    ["Ctrl+K", "Find a tool, setting, card or session"],
    ["Ctrl+Z / Ctrl+Y", "Undo / redo the latest edit"],
    [bindings.sessionToggle.replaceAll("Control", "Ctrl"), `Start or stop the route${unavailable}`],
    [bindings.privacyMute.replaceAll("Control", "Ctrl"), `Turn microphone privacy mute on or off${unavailable}`],
    ["Ctrl+A", "Select all tools and groups on the canvas"],
    ["Ctrl+D", "Duplicate one selected tool"],
    ["F2", "Rename one selected tool"],
    ["Ctrl+C / Ctrl+V", "Copy / paste a tool without connections"],
    ["Ctrl+plus / Ctrl+minus", "Zoom the canvas in / out"],
    ["Ctrl+0", "Fit the route in view"],
    ["Arrow keys", "Move a focused card one grid step"],
    ["Delete", "Remove selected tools or connections"],
    ["Esc (while dragging)", "Cancel the drag: a card goes back, a connection is not made"],
    ["Shift+F10", "Open a focused card's menu"],
    ["F1 / ?", "Show this reference"],
  ];
  return (
    <div className="dialog-backdrop app-dialog-backdrop" role="presentation">
      <section
        className="connection-dialog app-dialog keyboard-help"
        role="dialog"
        aria-modal="true"
        aria-labelledby={heading}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            onClose();
          } else if (event.key === "Tab") {
            if (
              (!event.shiftKey && document.activeElement === close.current) ||
              (event.shiftKey && document.activeElement === list.current)
            ) {
              event.preventDefault();
              (event.shiftKey ? close.current : list.current)?.focus();
            }
          }
        }}
      >
        <h2 id={heading}>Keyboard shortcuts</h2>
        <p>
          Canvas commands work while the canvas has focus. Shortcuts leave text editing alone. Tab starts with links to
          skip between regions.
        </p>
        <dl ref={list} tabIndex={0} aria-label="Shortcut reference">
          {entries.map(([key, action]) => (
            <div key={action}>
              <dt>{key}</dt>
              <dd>{action}</dd>
            </div>
          ))}
        </dl>
        <button ref={close} type="button" className="secondary" onClick={onClose}>
          Close
        </button>
      </section>
    </div>
  );
}
