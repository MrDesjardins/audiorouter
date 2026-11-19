import { useEffect, useRef } from "react";
import { useMessages } from "./i18n/useMessages";

export function SessionSwitchDialog({
  names,
  onDecision,
}: {
  names: string[];
  onDecision: (confirmed: boolean) => void;
}) {
  const tr = useMessages();
  const cancel = useRef<HTMLButtonElement>(null);
  const confirm = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const previous = document.activeElement;
    cancel.current?.focus();
    return () => {
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, []);
  return (
    <div className="dialog-backdrop" role="presentation">
      <section
        className="connection-dialog"
        style={{ maxHeight: "calc(100dvh - 2rem)", overflowY: "auto", overflowWrap: "anywhere" }}
        role="dialog"
        aria-modal="true"
        aria-labelledby="session-switch-heading"
        aria-describedby="session-switch-description"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            onDecision(false);
          }
          if (event.key === "Tab") {
            event.preventDefault();
            (document.activeElement === cancel.current ? confirm.current : cancel.current)?.focus();
          }
        }}
      >
        <h2 id="session-switch-heading">{tr("sessionSwitch.heading")}</h2>
        <p id="session-switch-description">{tr("sessionSwitch.description")}</p>
        <ul>
          {names.map((name, index) => (
            <li key={index}>{name}</li>
          ))}
        </ul>
        <div className="actions">
          <button ref={cancel} type="button" className="secondary" onClick={() => onDecision(false)}>
            {tr("sessionSwitch.cancel")}
          </button>
          <button ref={confirm} type="button" className="primary" onClick={() => onDecision(true)}>
            {tr("sessionSwitch.confirm")}
          </button>
        </div>
      </section>
    </div>
  );
}
