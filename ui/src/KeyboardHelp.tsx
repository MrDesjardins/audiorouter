import { useEffect, useId, useRef } from "react";
import { useMessages } from "./i18n/useMessages";

export function KeyboardHelp({ onClose }: { onClose: () => void }) {
  const tr = useMessages();
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
  const entries = [
    ["Ctrl+S", tr("g1.kbSave")],
    ["Ctrl+K", tr("g1.kbFind")],
    ["Ctrl+Z / Ctrl+Y", tr("g1.kbUndo")],
    ["Ctrl+A", tr("g1.kbSelectAll")],
    ["Ctrl+D", tr("g1.kbDuplicate")],
    ["F2", tr("g1.kbRename")],
    ["Ctrl+C / Ctrl+V", tr("g1.kbCopyPaste")],
    ["Ctrl+plus / Ctrl+minus", tr("g1.kbZoom")],
    ["Ctrl+0", tr("g1.kbFit")],
    [tr("g1.kbArrowKey"), tr("g1.kbArrow")],
    ["Delete", tr("g1.kbDelete")],
    [tr("g1.kbEscKey"), tr("g1.kbEsc")],
    ["Shift+F10", tr("g1.kbMenu")],
    ["F1 / ?", tr("g1.kbHelp")],
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
        <h2 id={heading}>{tr("g1.kbHeading")}</h2>
        <p>{tr("g1.kbIntro")}</p>
        <dl ref={list} tabIndex={0} aria-label={tr("g1.kbRefLabel")}>
          {entries.map(([key, action]) => (
            <div key={action}>
              <dt>{key}</dt>
              <dd>{action}</dd>
            </div>
          ))}
        </dl>
        <button ref={close} type="button" className="secondary" onClick={onClose}>
          {tr("g1.kbClose")}
        </button>
      </section>
    </div>
  );
}
