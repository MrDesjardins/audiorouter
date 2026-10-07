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
  useEffect(() => {
    if (!invoke) return;
    let active = true;
    void Promise.resolve(invoke(`${toggle.command}_get`))
      .then((value) => {
        if (active) setEnabled(value === true);
      })
      .catch(() => {
        if (active) setMessage(toggle.unavailable);
      });
    return () => {
      active = false;
    };
  }, [invoke, toggle]);
  const change = async (next: boolean) => {
    if (!invoke || busy) return;
    setBusy(true);
    try {
      setEnabled((await invoke(`${toggle.command}_set`, { enabled: next })) === true);
      setMessage(next ? toggle.on : toggle.off);
    } catch {
      setMessage(toggle.failed);
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel startup-autoplay" aria-labelledby={`${toggle.id}-heading`}>
      <h3 id={`${toggle.id}-heading`}>{toggle.heading}</h3>
      <label className="autoplay-toggle">
        <input
          type="checkbox"
          checked={enabled === true}
          disabled={!invoke || enabled === null || busy}
          onChange={(event) => void change(event.target.checked)}
        />
        {toggle.label}
      </label>
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
    "Keeps controllers such as a Stream Deck connected after a restart. The API uses the port and network it last started with (API tab), or this PC only on port 17891.",
  on: "The API will start with AudioRouter.",
  off: "Start the API from the API tab when you need it.",
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
