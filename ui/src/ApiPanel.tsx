import { useEffect, useState } from "react";
import { NumberField } from "./NumberField";

type ApiStatus = { running: boolean; port: number; url: string | null; token: string | null };
export function ApiPanel() {
  const [status, setStatus] = useState<ApiStatus>({ running: false, port: 17891, url: null, token: null });
  const [port, setPort] = useState(17891);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  useEffect(() => {
    let active = true;
    void invoke?.("http_api_control", { action: "status", port: null }).then((value) => { if (active) { const next = value as ApiStatus; setStatus(next); setPort(next.port); } }).catch(() => { if (active) setMessage("Cannot read API status. Reconnect and try again."); });
    return () => { active = false; };
  }, [invoke]);
  const control = async (action: string) => {
    if (!invoke || busy) return;
    setBusy(true); setMessage("");
    try { setStatus(await invoke("http_api_control", { action, port }) as ApiStatus); }
    catch (error) { setMessage(typeof error === "string" ? error : "API action failed. Reconnect and try again."); }
    finally { setBusy(false); }
  };
  return <section className="workbench-page" role="tabpanel" aria-label="API"><p className="eyebrow">Local HTTP control</p><h2>API</h2>
    <p className="muted">Use REST calls from this PC. Changes use the same backend as this window and appear here automatically. Named-pipe JSON-RPC and MCP remain available.</p>
    {!invoke && <p role="status">HTTP control is available in the desktop application.</p>}
    <label>Port<NumberField aria-label="API port" value={port} min={1024} max={65535} step={1} disabled={status.running || busy} onValue={(value) => { if (Number.isInteger(value)) setPort(value); }} /></label>
    <p role="status">{status.running ? "API running · localhost only" : "API stopped"}</p>
    <button type="button" disabled={!invoke || busy} onClick={() => void control(status.running ? "stop" : "start")}>{busy ? "Working…" : status.running ? "Stop API" : "Start API"}</button>
    {status.url && <><label>URL<input aria-label="API URL" readOnly value={status.url} /></label>
      <button type="button" className="secondary" disabled={busy} onClick={() => void control("openDocs")}>Open Swagger documentation</button>
      <p className="muted">Swagger runs locally. Choose Authorize and enter the bearer token to try calls. Stopping or restarting the API revokes the token.</p>
      {!status.token ? <button type="button" className="secondary" disabled={busy} onClick={() => void control("reveal")}>Reveal API token</button> : <><label>Bearer token<input aria-label="API bearer token" readOnly value={status.token} /></label><button type="button" className="secondary" onClick={() => { void navigator.clipboard?.writeText(status.token!).then(() => setMessage("Token copied.")).catch(() => setMessage("Select the token and copy it manually.")); }}>Copy API token</button><button type="button" className="secondary" onClick={() => setStatus({ ...status, token: null })}>Hide API token</button></>}
    </>}
    {message && <p role="status">{message}</p>}
  </section>;
}
