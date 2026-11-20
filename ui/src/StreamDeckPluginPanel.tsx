import { useState } from "react";
import { useMessages } from "./i18n/useMessages";

export function StreamDeckPluginPanel() {
  const tr = useMessages();
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const install = async () => {
    if (!invoke || busy) return;
    setBusy(true);
    setMessage(null);
    try {
      await invoke("install_streamdeck_plugin");
      setMessage(tr("g3Misc.sdOpened"));
    } catch (error) {
      setMessage(typeof error === "string" && error.length < 200 ? error : tr("g3Misc.sdFailed"));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="streamdeck-panel" aria-labelledby="streamdeck-heading">
      <h3 id="streamdeck-heading">{tr("g3Misc.sdHeading")}</h3>
      <p className="muted">{tr("g3Misc.sdIntro")}</p>
      <div className="actions">
        <button type="button" className="secondary" onClick={() => void install()} disabled={!invoke || busy}>
          {busy ? tr("g3Misc.sdOpening") : tr("g3Misc.sdInstall")}
        </button>
      </div>
      <p role="status" className="muted">
        {message ?? (invoke ? " " : tr("g3Misc.sdNeedApp"))}
      </p>
    </section>
  );
}
