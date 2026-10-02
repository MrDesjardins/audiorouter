import { useEffect, useState } from "react";

export function LogFilesPanel() {
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  const [path, setPath] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let active = true;
    if (invoke) void invoke("log_folder_path").then((value) => {
      if (active && typeof value === "string") setPath(value);
    }).catch(() => { if (active) setMessage("Could not locate the logs folder. Try opening it below."); });
    return () => { active = false; };
  }, [invoke]);
  const open = async () => {
    if (!invoke || busy) return;
    setBusy(true); setMessage(null);
    try { await invoke("open_logs_folder"); setMessage("Logs folder opened. Select the log files to attach to your message."); }
    catch { setMessage("Could not open the logs folder. Copy the folder path and paste it into File Explorer."); }
    finally { setBusy(false); }
  };
  const copy = async () => {
    if (!path) return;
    try { await navigator.clipboard.writeText(path); setMessage("Folder path copied."); }
    catch { setMessage("Could not copy the path. Select the folder path below and copy it manually."); }
  };
  return <section className="log-files-panel" aria-labelledby="log-files-heading">
    <h3 id="log-files-heading">Send logs for support</h3>
    <p className="muted">Open the folder and attach shell.jsonl, backend.jsonl and discovery.jsonl, plus their .previous.jsonl files if present. Include what happened and the approximate time.</p>
    <div className="actions">
      <button type="button" onClick={() => void open()} disabled={!invoke || busy}>{busy ? "Opening logs…" : "Open logs folder"}</button>
      <button type="button" className="secondary" onClick={() => void copy()} disabled={!path}>Copy folder path</button>
    </div>
    {path && <label>Logs folder<input aria-label="Logs folder" readOnly value={path} onFocus={(event) => event.currentTarget.select()} /></label>}
    {!invoke && <p className="muted">Open the installed AudioRouter app to access this PC’s log files.</p>}
    {message && <p role="status">{message}</p>}
  </section>;
}
