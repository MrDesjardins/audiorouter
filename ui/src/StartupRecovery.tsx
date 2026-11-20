import { useEffect, useEffectEvent, useState } from "react";
import { useMessages } from "./i18n/useMessages";

type Backup = { name: string; bytes: number };

/** These shell commands remain reachable when no backend can open storage. */
export function StartupRecovery({
  onRestored,
  onDamaged,
}: {
  onRestored: () => Promise<unknown>;
  /** Told once the backend has stopped because it cannot read saved data. */
  onDamaged?: () => void;
}) {
  const tr = useMessages();
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  const [message, setMessage] = useState(() => tr("g3Misc.srChecking"));
  const noBackupText = useEffectEvent(() => tr("g3Misc.srNoBackup"));
  const [backups, setBackups] = useState<Backup[]>([]);
  const [selected, setSelected] = useState("");
  const [armed, setArmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [recoveryMessage, setRecoveryMessage] = useState<string | null>(null);
  const [needsRecovery, setNeedsRecovery] = useState(false);
  const reportDamaged = useEffectEvent(() => onDamaged?.());
  useEffect(() => {
    if (!invoke) return;
    let active = true;
    let polling = false;
    let listed = false;
    let reportedDamage = false;
    const poll = async () => {
      if (polling) return;
      polling = true;
      try {
        const state = await invoke("backend_state");
        if (!active || typeof state !== "object" || !state) return;
        const status = state as { phase?: unknown; reason?: unknown; message?: unknown };
        if (!listed && typeof status.message === "string") setMessage(status.message);
        if (!listed && status.phase === "stopped" && status.reason === "corruptDatabase") {
          setNeedsRecovery(true);
          if (!reportedDamage) {
            reportedDamage = true;
            reportDamaged();
          }
          const result = await invoke("recovery_backups");
          if (
            !Array.isArray(result) ||
            !result.every(
              (item: unknown) =>
                typeof item === "object" &&
                item !== null &&
                typeof (item as Backup).name === "string" &&
                typeof (item as Backup).bytes === "number",
            )
          ) {
            throw new Error("Invalid backup inventory");
          }
          if (!active) return;
          listed = true;
          setBackups(result as Backup[]);
          setSelected((result[0] as Backup | undefined)?.name ?? "");
          if (!result.length) setMessage(noBackupText());
        }
      } catch {
        // The backend may still be releasing its storage handle. Retry without
        // replacing the last useful diagnostic or shifting this reserved slot.
      } finally {
        polling = false;
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 1000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [invoke]);
  if (!invoke) return null;
  const restore = async () => {
    if (!armed) {
      setArmed(true);
      return;
    }
    setBusy(true);
    try {
      await invoke("restore_recovery_backup", { name: selected });
      setRecoveryMessage(tr("g3Misc.srRestored"));
      await onRestored();
    } catch (error) {
      setRecoveryMessage(
        typeof error === "string" ? error : error instanceof Error ? error.message : tr("g3Misc.srFailed"),
      );
    } finally {
      setBusy(false);
      setArmed(false);
    }
  };
  return (
    <section aria-label={tr("g3Misc.srAria")} style={{ minHeight: "18rem" }}>
      <p role="status" style={{ height: "4.5rem", overflow: "auto" }}>
        {recoveryMessage ?? message}
      </p>
      <div style={{ visibility: needsRecovery ? "visible" : "hidden" }} aria-hidden={!needsRecovery}>
        <label style={{ display: "grid", gap: "0.5rem" }}>
          {tr("g3Misc.srBackup")}
          <select
            value={selected}
            disabled={!backups.length || busy}
            onChange={(event) => {
              setSelected(event.target.value);
              setArmed(false);
            }}
          >
            {!backups.length && <option value="">{tr("g3Misc.srNoneSelected")}</option>}
            {backups.map((backup) => (
              <option key={backup.name} value={backup.name}>
                {tr("g3Misc.srMiB", { name: backup.name, size: Math.ceil(backup.bytes / 1048576) })}
              </option>
            ))}
          </select>
        </label>
        <p className="muted" style={{ minHeight: "4.5rem", marginBottom: "1rem" }}>
          {tr("g3Misc.srNote")}
        </p>
        <button type="button" disabled={!selected || busy} onClick={() => void restore()}>
          {busy ? tr("g3Misc.srRestoring") : armed ? tr("g3Misc.srConfirm") : tr("g3Misc.srRestore")}
        </button>
      </div>
    </section>
  );
}
