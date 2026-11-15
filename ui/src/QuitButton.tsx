import { useEffect, useState } from "react";

const CONFIRM_MS = 5000;
/** The shell's prefix for a Quit the backend never answered (RES-4). */
export const QUIT_UNRESPONSIVE_PREFIX = "backendUnresponsive: ";

/**
 * Quit AudioRouter from the window: stops audio, finalizes recordings and
 * closes the backend, like the tray's "Quit and stop audio". The first click
 * asks for confirmation so a stray click cannot stop audio mid-game. Shown only
 * in the desktop shell.
 *
 * When the backend does not answer at all, the button becomes "Quit anyway",
 * which closes the app without finalization. A backend that answers with a
 * refusal (a recording still finishing) never gets that offer.
 */
export function QuitButton({
  onMessage,
  beforeQuit,
}: {
  onMessage: (message: string) => void;
  /** Keeps unsaved route edits for the next launch; a failure must not block quitting. */
  beforeQuit?: () => Promise<void>;
}) {
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  const [armed, setArmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [unresponsive, setUnresponsive] = useState(false);
  useEffect(() => {
    if (!armed) return;
    const timer = window.setTimeout(() => setArmed(false), CONFIRM_MS);
    return () => window.clearTimeout(timer);
  }, [armed]);
  if (!invoke) return null;
  const quit = async () => {
    if (!armed && !unresponsive) {
      setArmed(true);
      return;
    }
    setBusy(true);
    try {
      await beforeQuit?.().catch(() => undefined);
      await invoke("quit_app", unresponsive ? { force: true } : {});
      // The shell exits on success; nothing else to update.
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error);
      if (reason.startsWith(QUIT_UNRESPONSIVE_PREFIX)) {
        setUnresponsive(true);
        onMessage(
          "Quit failed: AudioRouter's background service did not answer. Click Quit anyway to close it now. Audio stops, and a recording in progress is recovered the next time you open AudioRouter.",
        );
      } else {
        onMessage(`Quit refused: ${reason}. Audio keeps playing; stop any recording and try again.`);
      }
      setArmed(false);
    } finally {
      setBusy(false);
    }
  };
  const label = busy ? "Quitting…" : unresponsive ? "Quit anyway" : armed ? "Click again to quit" : "Quit";
  return (
    <button
      type="button"
      className={armed || unresponsive ? "quit-button is-armed" : "quit-button secondary"}
      onClick={() => void quit()}
      disabled={busy}
      title={
        unresponsive
          ? "Close AudioRouter now without waiting for its background service. Recordings in progress are recovered next time."
          : "Stop audio, finish recordings and close AudioRouter (same as the tray's Quit). Unsaved edits come back next time you open it."
      }
      aria-label={unresponsive ? "Quit AudioRouter anyway" : armed ? "Confirm quit AudioRouter" : "Quit AudioRouter"}
    >
      {label}
    </button>
  );
}
