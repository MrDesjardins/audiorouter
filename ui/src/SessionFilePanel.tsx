import { useState } from "react";
import type { Session, SessionFileImportResult } from "@audiorouter/contracts";
import { formatUiError, type UiBackend } from "./backend";

type Invoke = (command: string, args?: Record<string, unknown>) => Promise<unknown>;

function shellInvoke(): Invoke | undefined {
  return typeof window === "undefined" ? undefined : window.__TAURI_INTERNALS__?.invoke;
}

export function importSummary(result: SessionFileImportResult): string {
  const parts = [`Opened "${result.session.name}" as a new session.`];
  if (result.renamed) parts.push("A session with the same ID already existed here, so the copy is marked (imported); nothing was replaced.");
  const restored = result.mediaRestored + result.pluginStatesRestored;
  if (restored > 0) parts.push(`Restored ${result.mediaRestored} audio file${result.mediaRestored === 1 ? "" : "s"} and ${result.pluginStatesRestored} plugin setting${result.pluginStatesRestored === 1 ? "" : "s"}.`);
  if (result.missingAssets > 0) parts.push(`${result.missingAssets} node${result.missingAssets === 1 ? " refers" : "s refer"} to audio or plugin settings the file did not include; choose them again in Properties.`);
  parts.push("Check each input and output node: device names can differ on this PC.");
  return parts.join(" ");
}

/**
 * Save the current session to one `.audiorouter` file, or open such a file
 * as a new session. The file carries every node and its settings, imported
 * audio and saved plugin settings, so a setup can be backed up or moved to
 * another PC. The desktop app uses the Windows Save/Open dialogs; the browser
 * preview asks for a full path instead.
 */
export function SessionFilePanel({ backend, session, unsaved, onImported }: {
  backend: UiBackend;
  session: Session;
  unsaved: boolean;
  onImported: (session: Session) => void;
}) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [typedPath, setTypedPath] = useState("");
  const invoke = shellInvoke();
  const choosePath = async (mode: "save" | "open"): Promise<string | null> => {
    if (!invoke) {
      const path = typedPath.trim();
      if (!path) { setMessage("Type the full path of a .audiorouter file first."); return null; }
      return path;
    }
    const chosen = await invoke("choose_session_file", { mode, suggestedName: session.name });
    return typeof chosen === "string" && chosen ? chosen : null;
  };
  const run = async (action: () => Promise<void>) => {
    if (busy) return;
    setBusy(true);
    try { await action(); } finally { setBusy(false); }
  };
  const saveFile = () => run(async () => {
    try {
      const path = await choosePath("save");
      if (!path) return;
      setMessage("Saving the session file…");
      // The Windows Save dialog already asked before replacing a file.
      const result = await backend.exportSessionFile(session.id, path, Boolean(invoke));
      setMessage(`Saved "${session.name}" to ${result.path}. It includes every node and its settings, imported audio and saved plugin settings. Plugins themselves must also be installed on the other PC.`);
    } catch (error) { setMessage(formatUiError(error, "Unable to save the session file.")); }
  });
  const openFile = () => run(async () => {
    try {
      const path = await choosePath("open");
      if (!path) return;
      setMessage("Opening the session file…");
      const result = await backend.importSessionFile(path);
      onImported(result.session);
      setMessage(importSummary(result));
    } catch (error) { setMessage(formatUiError(error, "Unable to open the session file.")); }
  });
  return <section className="session-file" aria-labelledby="session-file-heading">
    <h3 id="session-file-heading">Session file</h3>
    <p className="muted">Back up this session, or move it to another PC, as one <code>.audiorouter</code> file. Opening a file adds it as a new session and never replaces one.</p>
    {unsaved && <p className="muted session-file-note" role="note">You have unsaved edits. The file holds the last saved version, so Save first to include them.</p>}
    {!invoke && <label>File path<input aria-label="Session file path" placeholder="C:\Users\you\Documents\My setup.audiorouter" value={typedPath} onChange={(event) => setTypedPath(event.target.value)} disabled={!backend.connected || busy} /></label>}
    <div className="session-file-actions">
      <button type="button" className="secondary" onClick={() => void saveFile()} disabled={!backend.connected || busy}>Save to file…</button>
      <button type="button" className="secondary" onClick={() => void openFile()} disabled={!backend.connected || busy}>Open file…</button>
    </div>
    {message && <p className="muted session-file-message" role="status">{message}</p>}
  </section>;
}
