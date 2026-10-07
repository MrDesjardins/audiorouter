import { useEffect, useState } from "react";

const CONFIRM_MS = 5000;

/**
 * Quit AudioRouter from the window: stops audio, finalizes recordings and
 * closes the backend, like the tray's "Quit and stop audio". The first click
 * asks for confirmation so a stray click cannot stop audio mid-game. Shown only
 * in the desktop shell.
 */
export function QuitButton({ onMessage }: { onMessage: (message: string) => void }) {
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  const [armed, setArmed] = useState(false);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (!armed) return;
    const timer = window.setTimeout(() => setArmed(false), CONFIRM_MS);
    return () => window.clearTimeout(timer);
  }, [armed]);
  if (!invoke) return null;
  const quit = async () => {
    if (!armed) {
      setArmed(true);
      return;
    }
    setBusy(true);
    try {
      await invoke("quit_app");
      // The shell exits on success; nothing else to update.
    } catch (error) {
      onMessage(
        `Quit refused: ${error instanceof Error ? error.message : String(error)}. Audio keeps playing; stop any recording and try again.`,
      );
      setArmed(false);
    } finally {
      setBusy(false);
    }
  };
  return (
    <button
      type="button"
      className={armed ? "quit-button is-armed" : "quit-button secondary"}
      onClick={() => void quit()}
      disabled={busy}
      title="Stop audio, finish recordings and close AudioRouter (same as the tray's Quit)"
      aria-label={armed ? "Confirm quit AudioRouter" : "Quit AudioRouter"}
    >
      {busy ? "Quitting…" : armed ? "Click again to quit" : "Quit"}
    </button>
  );
}
