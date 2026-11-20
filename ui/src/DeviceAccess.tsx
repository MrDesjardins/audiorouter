import { useEffect, useRef, useState } from "react";
import type { UiBackend } from "./backend";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";

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
  const tr = useMessages();
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
        <h2 id="device-access-heading">{tr("settings.deviceAccess.dialogTitle")}</h2>
        <p id="device-access-description">{tr("settings.deviceAccess.dialogDescription")}</p>
        {error && (
          <p className="muted" role="alert">
            {error}
          </p>
        )}
        <div className="actions">
          <button ref={allow} type="button" className="primary" disabled={busy} onClick={onAllow}>
            {busy ? tr("settings.deviceAccess.allowing") : tr("settings.deviceAccess.allowAndPlay")}
          </button>
          <button type="button" className="secondary" disabled={busy} onClick={onCancel}>
            {tr("settings.deviceAccess.notNow")}
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
  const tr = useMessages();
  const [allowed, setAllowed] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [reading, setReading] = useState(true);
  const [readFailed, setReadFailed] = useState(false);
  const [message, setMessage] = useState<MessageKey | null>(null);
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
      setMessage("settings.deviceAccess.connectToCheck");
      return;
    }
    let cancelled = false;
    const load = () => {
      if (writeBusy.current) return;
      const current = ++generation.current;
      readBusy.current = true;
      setReading(true);
      setReadFailed(false);
      setMessage("settings.deviceAccess.checking");
      void Promise.resolve()
        .then(() => backend.getDeviceAccess!())
        .then((result) => {
          if (!cancelled && current === generation.current) {
            setAllowed(result.allowed);
            setMessage(null);
          }
        })
        .catch(() => {
          if (!cancelled && current === generation.current) {
            setReadFailed(true);
            setMessage("settings.deviceAccess.readFailed");
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
    setMessage("settings.deviceAccess.saving");
    try {
      const result = await backend.setDeviceAccess!(
        next,
        `device-access-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      );
      if (current === generation.current) {
        setAllowed(result.allowed);
        setMessage(result.allowed ? "settings.deviceAccess.allowedMessage" : "settings.deviceAccess.withdrawnMessage");
        window.dispatchEvent(new Event(ACCESS_CHANGED_EVENT));
      }
    } catch {
      if (current === generation.current) {
        setReadFailed(true);
        setMessage("settings.deviceAccess.confirmFailed");
      }
    } finally {
      if (current === generation.current) {
        writeBusy.current = false;
        setBusy(false);
      }
    }
  };
  const stateLabel = allowed ? tr("settings.deviceAccess.allowedState") : tr("settings.deviceAccess.askFirstState");
  return (
    <div
      className="recording-folder device-access-setting"
      role="group"
      aria-label={tr("settings.deviceAccess.heading")}
    >
      <div className="setting-toggle-row">
        <strong>{tr("settings.deviceAccess.heading")}</strong>
        <span className="setting-toggle-state device-access-state" data-testid="device-access-state">
          {!available
            ? tr("common.unavailable")
            : allowed === null
              ? reading
                ? tr("common.loading")
                : tr("common.unavailable")
              : `${readFailed ? `${tr("settings.deviceAccess.lastKnown", { state: stateLabel })}` : stateLabel}`}
        </span>
      </div>
      <p className="muted device-access-description">
        {readFailed
          ? tr("settings.deviceAccess.descriptionReadFailed")
          : allowed === null
            ? tr("settings.deviceAccess.descriptionUnavailable")
            : allowed
              ? tr("settings.deviceAccess.descriptionAllowed")
              : tr("settings.deviceAccess.descriptionAskFirst")}
      </p>
      <div className="recording-folder-actions">
        <button
          type="button"
          className="secondary"
          disabled={!available || busy || reading || readFailed || allowed === null}
          aria-busy={busy}
          onClick={() => void change(!allowed)}
        >
          {allowed ? tr("settings.deviceAccess.withdraw") : tr("settings.deviceAccess.allow")}
        </button>
        <button
          type="button"
          className="secondary"
          disabled={!available || busy || reading}
          onClick={() => setAttempt((value) => value + 1)}
        >
          {tr("common.checkAgain")}
        </button>
      </div>
      <p className="device-access-message" role="status">
        {message ? tr(message) : " "}
      </p>
    </div>
  );
}

export function announceDeviceAccessChanged() {
  window.dispatchEvent(new Event(ACCESS_CHANGED_EVENT));
}
