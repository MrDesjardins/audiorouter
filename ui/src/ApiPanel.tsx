import { useEffect, useState, type ReactNode } from "react";
import { NumberField } from "./NumberField";

type ApiStatus = { running: boolean; port: number; url: string | null; token: string | null };
/** `builder` renders the request builder for the API base URL. */
export function ApiPanel({ builder }: { builder?: (baseUrl: string) => ReactNode } = {}) {
  const [status, setStatus] = useState<ApiStatus>({ running: false, port: 17891, url: null, token: null });
  const [port, setPort] = useState(17891);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [regenerateConfirm, setRegenerateConfirm] = useState(false);
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  useEffect(() => {
    let active = true;
    void invoke?.("http_api_control", { action: "reveal", port: null }).then((value) => { if (active) { const next = value as ApiStatus; setStatus(next); setPort(next.port); } }).catch(() => { if (active) setMessage("Cannot read API token. Try Reveal API token, or generate a replacement if the saved token cannot be recovered."); });
    return () => { active = false; };
  }, [invoke]);
  const control = async (action: string) => {
    if (!invoke || busy) return;
    setBusy(true); setMessage("");
    try {
      const next = await invoke("http_api_control", { action, port }) as ApiStatus;
      setStatus((previous) => ({ ...next, token: action === "reveal" || action === "regenerate" ? next.token : previous.token }));
      if (action === "regenerate") { setRegenerateConfirm(false); setMessage("New token generated. Copy it to update your integrations."); }
    }
    catch (error) {
      setMessage(typeof error === "string" ? error : "API action failed. Reconnect and try again.");
      // Replacement can save successfully but fail to rebind the listener.
      try { setStatus(await invoke("http_api_control", { action: "status", port }) as ApiStatus); }
      catch { setStatus((previous) => ({ ...previous, token: null })); }
    }
    finally { setBusy(false); }
  };
  const copyToken = async () => {
    if (!status.token) return;
    try { await navigator.clipboard.writeText(status.token); setMessage("Token copied."); }
    catch { setMessage("Select the token and copy it manually."); }
  };
  return <section className="workbench-page" role="tabpanel" aria-label="API"><p className="eyebrow">Local HTTP control</p><h2>API</h2>
    <p className="muted">Use REST calls from this PC. Changes use the same backend as this window and appear here automatically. Named-pipe JSON-RPC and MCP remain available.</p>
    {!invoke && <p role="status">HTTP control is available in the desktop application.</p>}
    <label>Port<NumberField aria-label="API port" value={port} min={1024} max={65535} step={1} disabled={status.running || busy} onValue={(value) => { if (Number.isInteger(value)) setPort(value); }} /></label>
    <p role="status">{status.running ? "API running · localhost only" : "API stopped"}</p>
    <button type="button" disabled={!invoke || busy} onClick={() => void control(status.running ? "stop" : "start")}>{busy ? "Working…" : status.running ? "Stop API" : "Start API"}</button>
    {status.url && <><label>URL<input aria-label="API URL" readOnly value={status.url} /></label>
      <button type="button" className="secondary" disabled={busy} onClick={() => void control("openDocs")}>Open Swagger documentation</button>
      <p className="muted">Swagger runs locally. Choose Authorize and enter the bearer token to try calls. Your token is saved securely for this Windows account and stays the same when you stop or restart the API.</p>
    </>}
    <p className="muted">Your saved token works with your own apps. It stays the same until you generate a replacement, even while the API is stopped.</p>
    {!status.token ? <button type="button" className="secondary" disabled={!invoke || busy} onClick={() => void control("reveal")}>Reveal API token</button> : <><label>Bearer token<textarea aria-label="API bearer token" rows={2} readOnly value={status.token} onFocus={(event) => event.currentTarget.select()} /></label><div className="actions"><button type="button" className="secondary" disabled={busy} onClick={() => void copyToken()}>Copy API token</button><button type="button" className="secondary" disabled={busy} onClick={() => setStatus({ ...status, token: null })}>Hide API token</button></div></>}
    <p className="muted">Generate a new token only when you want to replace it. Existing integrations will need the new token.</p>
    {!regenerateConfirm ? <button type="button" className="secondary" disabled={!invoke || busy} onClick={() => setRegenerateConfirm(true)}>Generate new token</button> : <div className="api-token-confirm"><p>Replace the saved token? This disconnects integrations using the old token.</p><div className="actions"><button type="button" disabled={busy} onClick={() => void control("regenerate")}>Replace API token</button><button type="button" className="secondary" disabled={busy} onClick={() => setRegenerateConfirm(false)}>Cancel</button></div></div>}
    {message && <p role="status">{message}</p>}
    {builder?.(status.url ?? `http://127.0.0.1:${port}`)}
  </section>;
}
