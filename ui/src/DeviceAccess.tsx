import { useEffect, useRef, useState } from "react";
import type { UiBackend } from "./backend";

/** Play was refused because the user has not yet allowed this app to open
 * audio devices (device administration of the desktop shell). */
export function isDeviceAccessDenied(error: unknown): boolean {
  return error instanceof Error && /permission denied:\s*DeviceAdministration/i.test(error.message);
}

const ACCESS_CHANGED_EVENT = "audiorouter:device-access";

/** One-time question shown when Play first needs to open audio devices. */
export function DeviceAccessDialog({ busy, error, onAllow, onCancel }: { busy: boolean; error: string | null; onAllow: () => void; onCancel: () => void }) {
  const allow = useRef<HTMLButtonElement>(null);
  useEffect(() => allow.current?.focus(), []);
  return <div className="dialog-backdrop" role="presentation" onKeyDown={(event) => { if (event.key === "Escape" && !busy) onCancel(); }}>
    <section className="connection-dialog device-access-dialog" role="dialog" aria-modal="true" aria-labelledby="device-access-heading" aria-describedby="device-access-description">
      <h2 id="device-access-heading">Allow AudioRouter to use your audio devices?</h2>
      <p id="device-access-description">To play this session, AudioRouter opens the microphones, speakers and virtual cables in it. It asks once on this computer. You can change this later under <strong>Setup</strong>.</p>
      {error && <p className="muted" role="alert">{error}</p>}
      <div className="actions">
        <button ref={allow} type="button" className="primary" disabled={busy} onClick={onAllow}>{busy ? "Allowing…" : "Allow and play"}</button>
        <button type="button" className="secondary" disabled={busy} onClick={onCancel}>Not now</button>
      </div>
    </section>
  </div>;
}

/** Setup: whether this app may open audio devices on Play, with a toggle. */
export function DeviceAccessSetting({ backend }: { backend: Pick<UiBackend, "connected" | "getDeviceAccess" | "setDeviceAccess"> }) {
  const [allowed, setAllowed] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const available = backend.connected && Boolean(backend.getDeviceAccess && backend.setDeviceAccess);
  useEffect(() => {
    if (!available) return;
    let cancelled = false;
    const load = () => void backend.getDeviceAccess!().then((result) => { if (!cancelled) setAllowed(result.allowed); }).catch(() => { if (!cancelled) setAllowed(null); });
    load();
    window.addEventListener(ACCESS_CHANGED_EVENT, load);
    return () => { cancelled = true; window.removeEventListener(ACCESS_CHANGED_EVENT, load); };
  }, [available, backend]);
  if (!available || allowed === null) return null;
  const change = async (next: boolean) => {
    setBusy(true);
    try {
      const result = await backend.setDeviceAccess!(next, `device-access-${Date.now()}-${Math.random().toString(36).slice(2)}`);
      setAllowed(result.allowed);
      window.dispatchEvent(new Event(ACCESS_CHANGED_EVENT));
    } finally {
      setBusy(false);
    }
  };
  return <div className="recording-folder" role="group" aria-label="Audio device access">
    <strong>Audio device access</strong>
    <p className="muted">{allowed ? "AudioRouter may open the audio devices of a session when you press Play." : "AudioRouter asks before opening audio devices the first time you press Play."}</p>
    <div className="recording-folder-actions">
      <button type="button" className="secondary" disabled={busy} onClick={() => void change(!allowed)}>{allowed ? "Withdraw access" : "Allow access"}</button>
    </div>
  </div>;
}

export function announceDeviceAccessChanged() {
  window.dispatchEvent(new Event(ACCESS_CHANGED_EVENT));
}
