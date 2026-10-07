import { useState } from "react";

export function StreamDeckPluginPanel() {
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const install = async () => {
    if (!invoke || busy) return;
    setBusy(true);
    setMessage(null);
    try {
      await invoke("install_streamdeck_plugin");
      setMessage(
        "Stream Deck opened the plugin. Confirm the install there, then add an AudioRouter action and enter the URL and token shown above.",
      );
    } catch (error) {
      setMessage(
        typeof error === "string" && error.length < 200
          ? error
          : "Could not open the Stream Deck plugin. Is the Stream Deck app installed?",
      );
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="streamdeck-panel" aria-labelledby="streamdeck-heading">
      <h3 id="streamdeck-heading">Stream Deck plugin</h3>
      <p className="muted">
        AudioRouter includes its Stream Deck plugin (Stream Deck 7.1 or newer). Start the API, install the plugin, then
        enter this API URL and token in an action’s settings.
      </p>
      <div className="actions">
        <button type="button" className="secondary" onClick={() => void install()} disabled={!invoke || busy}>
          {busy ? "Opening Stream Deck…" : "Install Stream Deck plugin"}
        </button>
      </div>
      <p role="status" className="muted">
        {message ?? (invoke ? " " : "Open the installed AudioRouter app to install the Stream Deck plugin.")}
      </p>
    </section>
  );
}
