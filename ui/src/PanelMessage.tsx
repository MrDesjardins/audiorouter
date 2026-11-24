// A panel's status or error line, toned by the message's declared severity (moved from App.tsx).
import type { UiMessage } from "./actionMessage";
import { CopyErrorButton } from "./CopyErrorButton";

/**
 * A panel's status line. Error messages (see `UiMessage.tone`) render as a red
 * alert; everything else stays a quiet status line.
 */
export function PanelMessage({ message, small = false }: { message: UiMessage | null | undefined; small?: boolean }) {
  if (!message) return null;
  const error = message.tone === "error";
  const className = `panel-message is-${message.tone}`;
  const content = (
    <>
      <span className="error-message-text">{message.text}</span>
      {error && <CopyErrorButton message={message.text} />}
    </>
  );
  return small ? (
    <small className={className} role={error ? "alert" : "status"}>
      {content}
    </small>
  ) : (
    <p className={className} role={error ? "alert" : "status"} aria-live={error ? "assertive" : "polite"}>
      {content}
    </p>
  );
}
