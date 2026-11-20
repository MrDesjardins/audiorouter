import { useState } from "react";
import type { Session, SessionFileImportResult } from "@audiorouter/contracts";
import { formatUiError, type UiBackend } from "./backend";
import { useMessages } from "./i18n/useMessages";
import { tx } from "./i18n";

type Invoke = (command: string, args?: Record<string, unknown>) => Promise<unknown>;

function shellInvoke(): Invoke | undefined {
  return typeof window === "undefined" ? undefined : window.__TAURI_INTERNALS__?.invoke;
}

export function importSummary(result: SessionFileImportResult): string {
  const parts = [tx("workflowUi.fileOpened", { name: result.session.name })];
  if (result.renamed) parts.push(tx("workflowUi.fileRenamed"));
  const restored = result.mediaRestored + result.pluginStatesRestored;
  if (restored > 0)
    parts.push(
      tx("workflowUi.fileAssets", {
        media: tx("workflowUi.fileMedia", { count: result.mediaRestored }),
        plugins: tx("workflowUi.filePlugins", { count: result.pluginStatesRestored }),
      }),
    );
  if (result.missingAssets > 0) parts.push(tx("workflowUi.fileMissing", { count: result.missingAssets }));
  parts.push(tx("workflowUi.fileCheck"));
  return parts.join(" ");
}

/**
 * Save the current session to one `.audiorouter` file, or open such a file
 * as a new session. The file carries every node and its settings, imported
 * audio and saved plugin settings, so a setup can be backed up or moved to
 * another PC. The desktop app uses the Windows Save/Open dialogs; the browser
 * preview asks for a full path instead.
 */
export function SessionFilePanel({
  backend,
  session,
  unsaved,
  onImported,
}: {
  backend: UiBackend;
  session: Session;
  unsaved: boolean;
  onImported: (session: Session) => void;
}) {
  const tr = useMessages();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [typedPath, setTypedPath] = useState("");
  const invoke = shellInvoke();
  const choosePath = async (mode: "save" | "open"): Promise<string | null> => {
    if (!invoke) {
      const path = typedPath.trim();
      if (!path) {
        setMessage(tr("workflowUi.typePath"));
        return null;
      }
      return path;
    }
    const chosen = await invoke("choose_session_file", { mode, suggestedName: session.name });
    return typeof chosen === "string" && chosen ? chosen : null;
  };
  const run = async (action: () => Promise<void>) => {
    if (busy) return;
    setBusy(true);
    setMessage(null);
    try {
      await action();
    } finally {
      setBusy(false);
    }
  };
  const saveFile = () =>
    run(async () => {
      try {
        const path = await choosePath("save");
        if (!path) return;
        setMessage(tr("workflowUi.exporting"));
        // The Windows Save dialog already asked before replacing a file.
        const result = await backend.exportSessionFile(session.id, path, Boolean(invoke));
        setMessage(tr("workflowUi.fileExported", { name: session.name, path: result.path }));
      } catch (error) {
        setMessage(formatUiError(error, tr("workflowUi.exportError")));
      }
    });
  const openFile = () =>
    run(async () => {
      try {
        const path = await choosePath("open");
        if (!path) return;
        setMessage(tr("workflowUi.opening"));
        const result = await backend.importSessionFile(path);
        onImported(result.session);
        setMessage(importSummary(result));
      } catch (error) {
        setMessage(formatUiError(error, tr("workflowUi.openError")));
      }
    });
  return (
    <section className="session-file" aria-labelledby="session-file-heading">
      <h3 id="session-file-heading">{tr("workflowUi.sessionFile")}</h3>
      <p className="muted">{tr("workflowUi.fileIntro")}</p>
      {unsaved && (
        <p className="muted session-file-note" role="note">
          {tr("workflowUi.fileUnsaved")}
        </p>
      )}
      {!invoke && (
        <label>
          {tr("workflowUi.filePath")}
          <input
            aria-label={tr("workflowUi.sessionPath")}
            placeholder="C:\Users\you\Documents\My setup.audiorouter"
            value={typedPath}
            onChange={(event) => setTypedPath(event.target.value)}
            disabled={!backend.connected || busy}
          />
        </label>
      )}
      <div className="session-file-actions">
        <button
          type="button"
          className="secondary"
          onClick={() => void saveFile()}
          disabled={!backend.connected || busy}
        >
          {tr("workflowUi.exportFile")}
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => void openFile()}
          disabled={!backend.connected || busy}
        >
          {tr("workflowUi.openFile")}
        </button>
      </div>
      {message && (
        <p className="muted session-file-message" role="status">
          {message}
        </p>
      )}
    </section>
  );
}
