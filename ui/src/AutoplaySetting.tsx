import { useEffect, useState } from "react";

type Invoke = (command: string, args?: Record<string, unknown>) => Promise<unknown>;
type Props = { invoke?: Invoke };

const shellInvoke = () =>
  typeof window === "undefined" ? undefined : (window.__TAURI_INTERNALS__?.invoke as Invoke | undefined);

type StartupToggle = {
  id: string;
  heading: string;
  label: string;
  command: string;
  description: string;
  on: string;
  off: string;
  unavailable: string;
  failed: string;
};

/**
 * A startup choice the desktop shell reads before any window exists, saved
 * through `<command>_get` / `<command>_set` in the shell's settings file.
 */
function StartupSetting({ toggle, invoke }: { toggle: StartupToggle; invoke?: Invoke }) {
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
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
    setMessage("Saving setting…");
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
  return (
    <section className="panel startup-autoplay" aria-labelledby={`${toggle.id}-heading`}>
      <h3 id={`${toggle.id}-heading`}>{toggle.heading}</h3>
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
          {toggle.label}
        </label>
        <span className="setting-toggle-state" id={`${toggle.id}-state`} data-testid={`${toggle.id}-state`}>
          {!invoke || loadFailed ? "Unavailable" : enabled === null ? "Loading…" : enabled ? "On" : "Off"}
        </span>
      </div>
      <p className="muted">{invoke ? toggle.description : "Available in the AudioRouter desktop app."}</p>
      <p className="muted autoplay-message" role="status">
        {message ?? " "}
      </p>
    </section>
  );
}

const AUTOPLAY: StartupToggle = {
  id: "autoplay",
  heading: "Play when AudioRouter starts",
  label: "Play the selected session automatically",
  command: "autoplay",
  description:
    "Plays the session selected here when AudioRouter starts, for example at sign-in. The tray menu's Play and Stop audio work the same way without opening this window.",
  on: "AudioRouter will play the selected session when it starts.",
  off: "AudioRouter will start stopped.",
  unavailable: "Autoplay setting unavailable.",
  failed: "Autoplay could not be saved. Try again.",
};

const API_AUTOSTART: StartupToggle = {
  id: "api-autostart",
  heading: "Start the API when AudioRouter starts",
  label: "Start the local API automatically",
  command: "api_autostart",
  description:
    "Keeps controllers such as a Stream Deck connected after a restart. The API uses the port and network it last started with (Settings, API), or this PC only on port 17891.",
  on: "The API will start with AudioRouter.",
  off: "Start the API from Settings, API, when you need it.",
  unavailable: "API auto-start setting unavailable.",
  failed: "API auto-start could not be saved. Try again.",
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
