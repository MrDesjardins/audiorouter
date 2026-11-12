import { useState } from "react";

/** Key by message so feedback from an older failure cannot label a newer one. */
export function CopyErrorButton({ message }: { message: string }) {
  return <CopyError key={message} message={message} />;
}

function CopyError({ message }: { message: string }) {
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
        {state === "copied" ? "Copied" : "Copy error"}
      </button>
      <span className="sr-only" role="status">
        {state === "copied"
          ? "Error copied."
          : state === "failed"
            ? "Could not copy. Select the error text and press Ctrl+C."
            : ""}
      </span>
      {state === "failed" && (
        <textarea
          aria-label="Error text for manual copying"
          readOnly
          value={message}
          onFocus={(event) => event.currentTarget.select()}
        />
      )}
    </span>
  );
}
