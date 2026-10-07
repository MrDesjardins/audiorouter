// A panel's status or error line, toned by its text (moved from App.tsx).
import { actionMessageTone } from "./actionMessage";

/**
 * A panel's status line. Error-phrased messages (see `actionMessageTone`)
 * render as a red alert; everything else stays a quiet status line.
 */
export function PanelMessage({ message, small = false }: { message: string | null | undefined; small?: boolean }) {
  if (!message) return null;
  const tone = actionMessageTone(message);
  const error = tone === "error";
  const className = `panel-message is-${tone}`;
  return small ? (
    <small className={className} role={error ? "alert" : "status"}>
      {message}
    </small>
  ) : (
    <p className={className} role={error ? "alert" : "status"} aria-live={error ? "assertive" : "polite"}>
      {message}
    </p>
  );
}
