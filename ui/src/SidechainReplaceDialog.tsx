import { useEffect, useRef } from "react";
import { useMessages } from "./i18n/useMessages";

export function SidechainReplaceDialog({
  pluginName,
  currentSourceName,
  nextSourceName,
  onDecision,
}: {
  pluginName: string;
  currentSourceName: string;
  nextSourceName: string;
  onDecision: (replace: boolean) => void;
}) {
  const tr = useMessages();
  const keep = useRef<HTMLButtonElement>(null);
  const replace = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const previous = document.activeElement;
    keep.current?.focus();
    return () => {
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, []);
  return (
    <div className="dialog-backdrop" role="presentation">
      <section
        className="connection-dialog sidechain-replace-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="sidechain-replace-heading"
        aria-describedby="sidechain-replace-description"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            onDecision(false);
          }
          if (event.key === "Tab") {
            event.preventDefault();
            (document.activeElement === keep.current ? replace.current : keep.current)?.focus();
          }
        }}
      >
        <h2 id="sidechain-replace-heading">{tr("g4Panel.sidechainHeading")}</h2>
        <p id="sidechain-replace-description">
          {tr("g4Panel.sidechainDescription", { plugin: pluginName, current: currentSourceName, next: nextSourceName })}
        </p>
        <div className="actions">
          <button ref={keep} type="button" className="secondary" onClick={() => onDecision(false)}>
            {tr("g4Panel.sidechainKeep")}
          </button>
          <button ref={replace} type="button" className="primary" onClick={() => onDecision(true)}>
            {tr("g4Panel.sidechainReplace")}
          </button>
        </div>
      </section>
    </div>
  );
}
