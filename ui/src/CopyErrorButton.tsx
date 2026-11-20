import { useState } from "react";
import { useMessages } from "./i18n/useMessages";

/** Key by message so feedback from an older failure cannot label a newer one. */
export function CopyErrorButton({ message }: { message: string }) {
  return <CopyError key={message} message={message} />;
}

function CopyError({ message }: { message: string }) {
  const tr = useMessages();
  const [state, setState] = useState<"idle" | "copying" | "copied" | "failed">("idle");
  async function copy() {
    setState("copying");
    try {
      await navigator.clipboard.writeText(message);
      setState("copied");
    } catch {
      setState("failed");
    }
  }
  return (
    <span className="copy-error">
      <button type="button" className="secondary" disabled={state === "copying"} onClick={() => void copy()}>
        {state === "copied" ? tr("g1.copyCopied") : tr("g1.copyError")}
      </button>
      <span className="sr-only" role="status">
        {state === "copied" ? tr("g1.copyErrorCopied") : state === "failed" ? tr("g1.copyFailed") : ""}
      </span>
      {state === "failed" && (
        <textarea
          aria-label={tr("g1.copyManualLabel")}
          readOnly
          value={message}
          onFocus={(event) => event.currentTarget.select()}
        />
      )}
    </span>
  );
}
