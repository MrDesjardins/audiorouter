import { useEffect, useRef, useState } from "react";
import type { UiBackend } from "./backend";

/** Play was refused because the user has not yet allowed this app to open
 * audio devices (device administration of the desktop shell). */
export function isDeviceAccessDenied(error: unknown): boolean {
  return error instanceof Error && /permission denied:\s*DeviceAdministration/i.test(error.message);
}

const ACCESS_CHANGED_EVENT = "audiorouter:device-access";

/** One-time question shown when Play first needs to open audio devices. */
export function DeviceAccessDialog({
  busy,
  error,
  onAllow,
  onCancel,
}: {
  busy: boolean;
  error: string | null;
  onAllow: () => void;
  onCancel: () => void;
}) {
  const allow = useRef<HTMLButtonElement>(null);
  useEffect(() => allow.current?.focus(), []);
  return (
    <div
      className="dialog-backdrop"
      role="presentation"
      onKeyDown={(event) => {
        if (event.key === "Escape" && !busy) onCancel();
      }}
    >
      <section
        className="connection-dialog device-access-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="device-access-heading"
        aria-describedby="device-access-description"
      >
        <h2 id="device-access-heading">Allow AudioRouter to use your audio devices?</h2>
        <p id="device-access-description">
          To play this session, AudioRouter opens the microphones, speakers and virtual cables in it. It asks once on
          this computer. You can change this later under <strong>Settings → Device access</strong>.
        </p>
        {error && (
          <p className="muted" role="alert">
            {error}
          </p>
        )}
        <div className="actions">
          <button ref={allow} type="button" className="primary" disabled={busy} onClick={onAllow}>
            {busy ? "Allowing…" : "Allow and play"}
          </button>
          <button type="button" className="secondary" disabled={busy} onClick={onCancel}>
            Not now
          </button>
        </div>
      </section>
    </div>
  );
}

/** Whether this app may open audio devices on Play, with explicit change and recheck actions. */
export function DeviceAccessSetting({
  backend,
}: {
  backend: Pick<UiBackend, "connected" | "getDeviceAccess" | "setDeviceAccess">;
}) {
  const [allowed, setAllowed] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [reading, setReading] = useState(true);
  const [readFailed, setReadFailed] = useState(false);
  const [message, setMessage] = useState("");
  const [attempt, setAttempt] = useState(0);
  const generation = useRef(0);
  const writeBusy = useRef(false);
  const readBusy = useRef(false);
  const previousBackend = useRef(backend);
  const available = backend.connected && Boolean(backend.getDeviceAccess && backend.setDeviceAccess);
  useEffect(() => {
    if (previousBackend.current !== backend || !available) setAllowed(null);
    previousBackend.current = backend;
    setBusy(false);
    if (!available) {
      setReading(false);
      setMessage("Connect to the desktop backend to check audio device access.");
      return;
    }
    let cancelled = false;
    const load = () => {
      if (writeBusy.current) return;
      const current = ++generation.current;
      readBusy.current = true;
      setReading(true);
      setReadFailed(false);
      setMessage("Checking audio device access…");
      void Promise.resolve()
        .then(() => backend.getDeviceAccess!())
        .then((result) => {
          if (!cancelled && current === generation.current) {
            setAllowed(result.allowed);
            setMessage("");
          }
        })
        .catch(() => {
          if (!cancelled && current === generation.current) {
            setReadFailed(true);
            setMessage("Could not read audio device access. Use Check again.");
          }
        })
        .finally(() => {
          if (!cancelled && current === generation.current) {
            readBusy.current = false;
            setReading(false);
          }
        });
    };
    load();
    window.addEventListener(ACCESS_CHANGED_EVENT, load);
    return () => {
      cancelled = true;
      generation.current += 1;
      readBusy.current = false;
      writeBusy.current = false;
      window.removeEventListener(ACCESS_CHANGED_EVENT, load);
    };
  }, [available, backend, attempt]);
  const change = async (next: boolean) => {
    if (!available || writeBusy.current || readBusy.current || readFailed || allowed === null) return;
    const current = ++generation.current;
    writeBusy.current = true;
    setBusy(true);
    setMessage("Saving audio device access…");
    try {
      const result = await backend.setDeviceAccess!(
        next,
        `device-access-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      );
      if (current === generation.current) {
        setAllowed(result.allowed);
        setMessage(
          result.allowed ? "Audio device access allowed." : "AudioRouter will ask before opening audio devices.",
        );
        window.dispatchEvent(new Event(ACCESS_CHANGED_EVENT));
      }
    } catch {
      if (current === generation.current) {
        setReadFailed(true);
        setMessage("Could not confirm the change. Check again before retrying.");
      }
    } finally {
      if (current === generation.current) {
        writeBusy.current = false;
        setBusy(false);
      }
    }
  };
  return (
    <div className="recording-folder device-access-setting" role="group" aria-label="Audio device access">
      <div className="setting-toggle-row">
        <strong>Audio device access</strong>
        <span className="setting-toggle-state device-access-state" data-testid="device-access-state">
          {!available
            ? "Unavailable"
            : allowed === null
              ? reading
                ? "Loading…"
                : "Unavailable"
              : `${readFailed ? "Last known: " : ""}${allowed ? "Allowed" : "Ask first"}`}
        </span>
      </div>
      <p className="muted device-access-description">
        {readFailed
          ? "The current access could not be confirmed. Check again before changing it."
          : allowed === null
            ? "Choose whether AudioRouter may open your session's audio devices when you press Play."
            : allowed
              ? "AudioRouter may open the audio devices of a session when you press Play."
              : "AudioRouter asks before opening audio devices the first time you press Play."}
      </p>
      <div className="recording-folder-actions">
        <button
          type="button"
          className="secondary"
          disabled={!available || busy || reading || readFailed || allowed === null}
          aria-busy={busy}
          onClick={() => void change(!allowed)}
        >
          {allowed ? "Withdraw access" : "Allow access"}
        </button>
        <button
          type="button"
          className="secondary"
          disabled={!available || busy || reading}
          onClick={() => setAttempt((value) => value + 1)}
        >
          Check again
        </button>
      </div>
      <p className="device-access-message" role="status">
        {message || " "}
      </p>
    </div>
  );
}

export function announceDeviceAccessChanged() {
  window.dispatchEvent(new Event(ACCESS_CHANGED_EVENT));
}
