import { useEffect, useRef } from "react";

export function SessionSwitchDialog({
  names,
  onDecision,
}: {
  names: string[];
  onDecision: (confirmed: boolean) => void;
}) {
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
        <h2 id="session-switch-heading">Stop audio and switch session?</h2>
        <p id="session-switch-description">
          Switching stops all sessions listed below, including audio used by other apps. Their active recordings will
          stop and finalize; a finalization failure may leave recoverable partial files. The selected session stays
          stopped until you press Play.
        </p>
        <ul>
          {names.map((name, index) => (
            <li key={index}>{name}</li>
          ))}
        </ul>
        <div className="actions">
          <button ref={cancel} type="button" className="secondary" onClick={() => onDecision(false)}>
            Cancel
          </button>
          <button ref={confirm} type="button" className="primary" onClick={() => onDecision(true)}>
            Stop and switch
          </button>
        </div>
      </section>
    </div>
  );
}
