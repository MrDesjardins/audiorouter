import { useEffect, useState } from "react";
import { useMessages } from "./i18n/useMessages";
import type { MessageKey } from "./i18n/generated/messageKeys";

type Invoke = (command: string, args?: Record<string, unknown>) => Promise<unknown>;
type Props = { invoke?: Invoke };

const shellInvoke = () =>
  typeof window === "undefined" ? undefined : (window.__TAURI_INTERNALS__?.invoke as Invoke | undefined);

type StartupToggle = {
  id: string;
  heading: MessageKey;
  label: MessageKey;
  command: string;
  description: MessageKey;
  on: MessageKey;
  off: MessageKey;
  unavailable: MessageKey;
  failed: MessageKey;
};

/**
 * A startup choice the desktop shell reads before any window exists, saved
 * through `<command>_get` / `<command>_set` in the shell's settings file.
 */
function StartupSetting({ toggle, invoke }: { toggle: StartupToggle; invoke?: Invoke }) {
  const tr = useMessages();
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<MessageKey | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);
  useEffect(() => {
    setEnabled(null);
    setLoadFailed(false);
    setMessage(null);
    if (!invoke) return;
    let active = true;
    void Promise.resolve()
      .then(() => invoke(`${toggle.command}_get`))
      .then((value) => {
        if (active) setEnabled(value === true);
      })
      .catch(() => {
        if (active) {
          setLoadFailed(true);
          setMessage(toggle.unavailable);
        }
      });
    return () => {
      active = false;
    };
  }, [invoke, toggle]);
  const change = async (next: boolean) => {
    if (!invoke || busy) return;
    setBusy(true);
    setMessage("common.saving");
    try {
      const confirmed = (await invoke(`${toggle.command}_set`, { enabled: next })) === true;
      setEnabled(confirmed);
      setMessage(confirmed ? toggle.on : toggle.off);
    } catch {
      setMessage(toggle.failed);
    } finally {
      setBusy(false);
    }
  };
  // The message state stores keys; render them in the active language.
  const messageText = message ? tr(message) : null;
  return (
    <section className="panel startup-autoplay" aria-labelledby={`${toggle.id}-heading`}>
      <h3 id={`${toggle.id}-heading`}>{tr(toggle.heading)}</h3>
      <div className="setting-toggle-row">
        <label className="autoplay-toggle">
          <input
            type="checkbox"
            checked={enabled === true}
            disabled={!invoke || enabled === null || busy}
            aria-describedby={`${toggle.id}-state`}
            aria-busy={busy}
            onChange={(event) => void change(event.target.checked)}
          />
          {tr(toggle.label)}
        </label>
        <span className="setting-toggle-state" id={`${toggle.id}-state`} data-testid={`${toggle.id}-state`}>
          {!invoke || loadFailed
            ? tr("common.unavailable")
            : enabled === null
              ? tr("common.loading")
              : enabled
                ? tr("common.on")
                : tr("common.off")}
        </span>
      </div>
      <p className="muted">{invoke ? tr(toggle.description) : tr("common.availableInDesktop")}</p>
      <p className="muted autoplay-message" role="status">
        {messageText ?? " "}
      </p>
    </section>
  );
}

const AUTOPLAY: StartupToggle = {
  id: "autoplay",
  heading: "settings.startup.autoplayHeading",
  label: "settings.startup.autoplayLabel",
  command: "autoplay",
  description: "settings.startup.autoplayDescription",
  on: "settings.startup.autoplayOn",
  off: "settings.startup.autoplayOff",
  unavailable: "settings.startup.autoplayUnavailable",
  failed: "settings.startup.autoplayFailed",
};

const API_AUTOSTART: StartupToggle = {
  id: "api-autostart",
  heading: "settings.startup.apiHeading",
  label: "settings.startup.apiLabel",
  command: "api_autostart",
  description: "settings.startup.apiDescription",
  on: "settings.startup.apiOn",
  off: "settings.startup.apiOff",
  unavailable: "settings.startup.apiUnavailable",
  failed: "settings.startup.apiFailed",
};

/**
 * Autoplay: the desktop shell plays the selected session when AudioRouter
 * starts (for example at sign-in), with or without the window.
 */
export function AutoplaySetting({ invoke = shellInvoke() }: Props) {
  return <StartupSetting toggle={AUTOPLAY} invoke={invoke} />;
}

/** API auto-start: the desktop shell starts the local HTTP API at launch. */
export function ApiAutostartSetting({ invoke = shellInvoke() }: Props) {
  return <StartupSetting toggle={API_AUTOSTART} invoke={invoke} />;
}
