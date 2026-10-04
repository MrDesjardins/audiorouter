import { useEffect, useState } from "react";

type Invoke = (command: string, args?: Record<string, unknown>) => Promise<unknown>;

/**
 * Autoplay: the desktop shell plays the selected session when AudioRouter
 * starts (for example at sign-in), with or without the window. The setting
 * lives in the shell, which reads it before any window exists.
 */
export function AutoplaySetting({ invoke = typeof window === "undefined" ? undefined : window.__TAURI_INTERNALS__?.invoke as Invoke | undefined }: { invoke?: Invoke }) {
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  useEffect(() => {
    if (!invoke) return;
    let active = true;
    void Promise.resolve(invoke("autoplay_get")).then((value) => { if (active) setEnabled(value === true); }).catch(() => { if (active) setMessage("Autoplay setting unavailable."); });
    return () => { active = false; };
  }, [invoke]);
  const change = async (next: boolean) => {
    if (!invoke || busy) return;
    setBusy(true);
    try {
      setEnabled((await invoke("autoplay_set", { enabled: next })) === true);
      setMessage(next ? "AudioRouter will play the selected session when it starts." : "AudioRouter will start stopped.");
    } catch {
      setMessage("Autoplay could not be saved. Try again.");
    } finally { setBusy(false); }
  };
  return <section className="panel startup-autoplay" aria-labelledby="autoplay-heading">
    <h3 id="autoplay-heading">Play when AudioRouter starts</h3>
    <label className="autoplay-toggle"><input type="checkbox" checked={enabled === true} disabled={!invoke || enabled === null || busy} onChange={(event) => void change(event.target.checked)} />Play the selected session automatically</label>
    <p className="muted">{invoke
      ? "Plays the session selected here when AudioRouter starts, for example at sign-in. The tray menu's Play and Stop audio work the same way without opening this window."
      : "Available in the AudioRouter desktop app."}</p>
    <p className="muted autoplay-message" role="status">{message ?? " "}</p>
  </section>;
}
