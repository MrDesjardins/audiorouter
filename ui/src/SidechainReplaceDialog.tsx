import { useEffect, useRef } from "react";

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
        <h2 id="sidechain-replace-heading">Replace this sidechain source?</h2>
        <p id="sidechain-replace-description">
          The Sidechain input on {pluginName} already receives {currentSourceName}. Replace it with {nextSourceName}?
          The key goes to the plugin&apos;s auxiliary input; AudioRouter will not add a Mixer or combine it with the
          main signal.
        </p>
        <div className="actions">
          <button ref={keep} type="button" className="secondary" onClick={() => onDecision(false)}>
            Keep current source
          </button>
          <button ref={replace} type="button" className="primary" onClick={() => onDecision(true)}>
            Replace sidechain source
          </button>
        </div>
      </section>
    </div>
  );
}
