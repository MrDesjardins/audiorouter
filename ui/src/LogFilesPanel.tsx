import { useEffect, useState } from "react";
import type { VerboseDiagnosticsStatus } from "@audiorouter/contracts";

/** The backend's opt-in verbose logging switch (`diagnostics.*Verbose`). */
export type VerboseLoggingControl = {
  get(): Promise<VerboseDiagnosticsStatus>;
  set(enabled: boolean): Promise<VerboseDiagnosticsStatus>;
};

/** "59:12" for the time left; the label keeps a fixed width (UI-17). */
export function formatVerboseRemaining(seconds: number): string {
  const bounded = Math.max(0, Math.min(3_600, Math.ceil(seconds)));
  const minutes = Math.floor(bounded / 60);
  return `${String(minutes).padStart(2, "0")}:${String(bounded % 60).padStart(2, "0")}`;
}

type VerboseState =
  | { kind: "loading" }
  | { kind: "unavailable" }
  | { kind: "off" }
  | { kind: "on"; expiresAtMs: number };

function verboseStateFrom(status: VerboseDiagnosticsStatus, now: number): VerboseState {
  return status.enabled && status.remainingSeconds > 0
    ? { kind: "on", expiresAtMs: now + status.remainingSeconds * 1000 }
    : { kind: "off" };
}

export function LogFilesPanel({ verbose, clientDiagnostics = [] }: { verbose?: VerboseLoggingControl; clientDiagnostics?: readonly string[] } = {}) {
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  const [path, setPath] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [bundleBusy, setBundleBusy] = useState(false);
  const [verboseState, setVerboseState] = useState<VerboseState>(verbose ? { kind: "loading" } : { kind: "unavailable" });
  const [verboseBusy, setVerboseBusy] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    let active = true;
    if (invoke) void invoke("log_folder_path").then((value) => {
      if (active && typeof value === "string") setPath(value);
    }).catch(() => { if (active) setMessage("Could not locate the logs folder. Try opening it below."); });
    return () => { active = false; };
  }, [invoke]);
  useEffect(() => {
    let active = true;
    if (!verbose) { setVerboseState({ kind: "unavailable" }); return; }
    void verbose.get()
      .then((status) => { if (active) setVerboseState(verboseStateFrom(status, Date.now())); })
      .catch(() => { if (active) setVerboseState({ kind: "unavailable" }); });
    return () => { active = false; };
  }, [verbose]);
  // Count down once a second while verbose logging is on; it switches itself off.
  useEffect(() => {
    if (verboseState.kind !== "on") return;
    const timer = setInterval(() => {
      const current = Date.now();
      setNow(current);
      if (current >= verboseState.expiresAtMs) setVerboseState({ kind: "off" });
    }, 1000);
    return () => clearInterval(timer);
  }, [verboseState]);
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
  const bundle = async () => {
    if (!invoke || bundleBusy) return;
    setBundleBusy(true); setMessage(null);
    try {
      const name = await invoke("export_support_bundle", { clientDiagnostics: clientDiagnostics.slice(0, 100) });
      setMessage(typeof name === "string" && /^[A-Za-z0-9._-]{1,80}$/.test(name)
        ? `Saved ${name} in the logs folder and selected it in File Explorer. Attach it to your message.`
        : "Support bundle saved in the logs folder.");
    } catch (error) {
      // The shell's messages are fixed sentences without paths.
      const text = typeof error === "string" && error.length <= 160 && !/[\\/]/.test(error) ? error : "Could not create the support bundle. Use Open logs folder instead.";
      setMessage(text);
    } finally { setBundleBusy(false); }
  };
  const toggleVerbose = async (enabled: boolean) => {
    if (!verbose || verboseBusy) return;
    setVerboseBusy(true);
    try { setVerboseState(verboseStateFrom(await verbose.set(enabled), Date.now())); setNow(Date.now()); }
    catch { setMessage("Could not change verbose logging. Check that AudioRouter is connected."); }
    finally { setVerboseBusy(false); }
  };
  const verboseOn = verboseState.kind === "on";
  const verboseStatus = verboseState.kind === "loading" ? "Checking…"
    : verboseState.kind === "unavailable" ? "Unavailable"
    : verboseOn ? `On · ${formatVerboseRemaining((verboseState.expiresAtMs - now) / 1000)} left`
    : "Off";
  return <section className="log-files-panel" aria-labelledby="log-files-heading">
    <h3 id="log-files-heading">Send logs for support</h3>
    <p className="muted">Copy support bundle saves one ZIP with the app and Windows versions, the latest backend, shell and MCP log lines, and these client diagnostics. Nothing is uploaded. Or open the folder and attach shell.jsonl, backend.jsonl, discovery.jsonl and network.jsonl, plus their .previous.jsonl files if present. For a problem between two computers, send the folder from both. Include what happened and the approximate time.</p>
    <div className="actions">
      <button type="button" onClick={() => void bundle()} disabled={!invoke || bundleBusy}>{bundleBusy ? "Saving bundle…" : "Copy support bundle"}</button>
      <button type="button" className="secondary" onClick={() => void open()} disabled={!invoke || busy}>{busy ? "Opening logs…" : "Open logs folder"}</button>
      <button type="button" className="secondary" onClick={() => void copy()} disabled={!path}>Copy folder path</button>
    </div>
    <div className="log-verbose">
      <label className="log-verbose-toggle"><input type="checkbox" checked={verboseOn} disabled={!verbose || verboseBusy || verboseState.kind === "loading" || verboseState.kind === "unavailable"} onChange={(event) => void toggleVerbose(event.target.checked)} /><span>Verbose logging</span></label>
      <span className="log-verbose-status" role="status" aria-label="Verbose logging status">{verboseStatus}</span>
    </div>
    <p className="muted">Turn on while you reproduce a problem: the logs then also record routine reads and how long each request took, for one hour at most. Settings, file paths and audio are never logged.</p>
    {path && <label>Logs folder<input aria-label="Logs folder" readOnly value={path} onFocus={(event) => event.currentTarget.select()} /></label>}
    {!invoke && <p className="muted">Open the installed AudioRouter app to access this PC’s log files.</p>}
    <p className="log-files-message" role="status">{message ?? ""}</p>
  </section>;
}
