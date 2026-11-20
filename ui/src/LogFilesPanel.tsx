import { useEffect, useEffectEvent, useState } from "react";
import type { VerboseDiagnosticsStatus } from "@audiorouter/contracts";
import { useMessages } from "./i18n/useMessages";

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
  { kind: "loading" } | { kind: "unavailable" } | { kind: "off" } | { kind: "on"; expiresAtMs: number };

function verboseStateFrom(status: VerboseDiagnosticsStatus, now: number): VerboseState {
  return status.enabled && status.remainingSeconds > 0
    ? { kind: "on", expiresAtMs: now + status.remainingSeconds * 1000 }
    : { kind: "off" };
}

export function LogFilesPanel({
  verbose,
  clientDiagnostics = [],
}: { verbose?: VerboseLoggingControl; clientDiagnostics?: readonly string[] } = {}) {
  const tr = useMessages();
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  const locateFailed = useEffectEvent(() => setMessage(tr("g3Logs.locateFail")));
  const [path, setPath] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [bundleBusy, setBundleBusy] = useState(false);
  const [verboseState, setVerboseState] = useState<VerboseState>(
    verbose ? { kind: "loading" } : { kind: "unavailable" },
  );
  const [verboseBusy, setVerboseBusy] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    let active = true;
    if (invoke)
      void invoke("log_folder_path")
        .then((value) => {
          if (active && typeof value === "string") setPath(value);
        })
        .catch(() => {
          if (active) locateFailed();
        });
    return () => {
      active = false;
    };
  }, [invoke]);
  useEffect(() => {
    let active = true;
    if (!verbose) {
      setVerboseState({ kind: "unavailable" });
      return;
    }
    void verbose
      .get()
      .then((status) => {
        if (active) setVerboseState(verboseStateFrom(status, Date.now()));
      })
      .catch(() => {
        if (active) setVerboseState({ kind: "unavailable" });
      });
    return () => {
      active = false;
    };
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
    setBusy(true);
    setMessage(null);
    try {
      await invoke("open_logs_folder");
      setMessage(tr("g3Logs.opened"));
    } catch {
      setMessage(tr("g3Logs.openFail"));
    } finally {
      setBusy(false);
    }
  };
  const copy = async () => {
    if (!path) return;
    try {
      await navigator.clipboard.writeText(path);
      setMessage(tr("g3Logs.copied"));
    } catch {
      setMessage(tr("g3Logs.copyFail"));
    }
  };
  const bundle = async () => {
    if (!invoke || bundleBusy) return;
    setBundleBusy(true);
    setMessage(null);
    try {
      const name = await invoke("export_support_bundle", { clientDiagnostics: clientDiagnostics.slice(0, 100) });
      setMessage(
        typeof name === "string" && /^[A-Za-z0-9._-]{1,80}$/.test(name)
          ? tr("g3Logs.bundleSaved", { name })
          : tr("g3Logs.bundleSavedPlain"),
      );
    } catch (error) {
      // The shell's messages are fixed sentences without paths.
      const text =
        typeof error === "string" && error.length <= 160 && !/[\\/]/.test(error) ? error : tr("g3Logs.bundleFail");
      setMessage(text);
    } finally {
      setBundleBusy(false);
    }
  };
  const toggleVerbose = async (enabled: boolean) => {
    if (!verbose || verboseBusy) return;
    setVerboseBusy(true);
    try {
      setVerboseState(verboseStateFrom(await verbose.set(enabled), Date.now()));
      setNow(Date.now());
    } catch {
      setMessage(tr("g3Logs.verboseFail"));
    } finally {
      setVerboseBusy(false);
    }
  };
  const verboseOn = verboseState.kind === "on";
  const verboseStatus =
    verboseState.kind === "loading"
      ? tr("g3Logs.checking")
      : verboseState.kind === "unavailable"
        ? tr("g3Logs.unavailable")
        : verboseOn
          ? tr("g3Logs.onLeft", { time: formatVerboseRemaining((verboseState.expiresAtMs - now) / 1000) })
          : tr("g3Logs.off");
  return (
    <section className="log-files-panel" aria-labelledby="log-files-heading">
      <h3 id="log-files-heading">{tr("g3Logs.heading")}</h3>
      <p className="muted">{tr("g3Logs.intro")}</p>
      <div className="actions">
        <button type="button" onClick={() => void bundle()} disabled={!invoke || bundleBusy}>
          {bundleBusy ? tr("g3Logs.saving") : tr("g3Logs.copyBundle")}
        </button>
        <button type="button" className="secondary" onClick={() => void open()} disabled={!invoke || busy}>
          {busy ? tr("g3Logs.opening") : tr("g3Logs.openFolder")}
        </button>
        <button type="button" className="secondary" onClick={() => void copy()} disabled={!path}>
          {tr("g3Logs.copyPath")}
        </button>
      </div>
      <div className="log-verbose">
        <label className="log-verbose-toggle">
          <input
            type="checkbox"
            checked={verboseOn}
            disabled={!verbose || verboseBusy || verboseState.kind === "loading" || verboseState.kind === "unavailable"}
            onChange={(event) => void toggleVerbose(event.target.checked)}
          />
          <span>{tr("g3Logs.verbose")}</span>
        </label>
        <span className="log-verbose-status" role="status" aria-label={tr("g3Logs.verboseStatus")}>
          {verboseStatus}
        </span>
      </div>
      <p className="muted">{tr("g3Logs.verboseHelp")}</p>
      {path && (
        <label>
          {tr("g3Logs.folder")}
          <input
            aria-label={tr("g3Logs.folder")}
            readOnly
            value={path}
            onFocus={(event) => event.currentTarget.select()}
          />
        </label>
      )}
      {!invoke && <p className="muted">{tr("g3Logs.needApp")}</p>}
      <p className="log-files-message" role="status">
        {message ?? ""}
      </p>
    </section>
  );
}
