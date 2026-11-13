// A panel's status or error line, toned by its text (moved from App.tsx).
import { actionMessageTone } from "./actionMessage";
import { CopyErrorButton } from "./CopyErrorButton";

/**
 * A panel's status line. Error-phrased messages (see `actionMessageTone`)
 * render as a red alert; everything else stays a quiet status line.
 */
export function PanelMessage({ message, small = false }: { message: string | null | undefined; small?: boolean }) {
  if (!message) return null;
  const tone = actionMessageTone(message);
  const error = tone === "error";
  const className = `panel-message is-${tone}`;
  const content = (
    <>
      <span className="error-message-text">{message}</span>
      {error && <CopyErrorButton message={message} />}
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
