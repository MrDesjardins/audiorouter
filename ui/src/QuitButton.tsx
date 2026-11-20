import { useEffect, useState } from "react";
import { useMessages } from "./i18n/useMessages";

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
  const tr = useMessages();
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
        onMessage(tr("quit.unresponsiveMessage"));
      } else {
        onMessage(tr("quit.refusedMessage", { reason }));
      }
      setArmed(false);
    } finally {
      setBusy(false);
    }
  };
  const label = busy
    ? tr("quit.quitting")
    : unresponsive
      ? tr("quit.anyway")
      : armed
        ? tr("quit.confirm")
        : tr("quit.quit");
  return (
    <button
      type="button"
      className={armed || unresponsive ? "quit-button is-armed" : "quit-button secondary"}
      onClick={() => void quit()}
      disabled={busy}
      title={unresponsive ? tr("quit.anywayTitle") : tr("quit.quitTitle")}
      aria-label={unresponsive ? tr("quit.anywayLabel") : armed ? tr("quit.confirmLabel") : tr("quit.quitLabel")}
    >
      {label}
    </button>
  );
}
