import { useEffect, useState } from "react";

type Backup = { name: string; bytes: number };

/** These shell commands remain reachable when no backend can open storage. */
export function StartupRecovery({ onRestored }: { onRestored: () => Promise<unknown> }) {
  const invoke = window.__TAURI_INTERNALS__?.invoke;
  const [message, setMessage] = useState("Checking the audio service…");
  const [backups, setBackups] = useState<Backup[]>([]);
  const [selected, setSelected] = useState("");
  const [armed, setArmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [recoveryMessage, setRecoveryMessage] = useState<string | null>(null);
  const [needsRecovery, setNeedsRecovery] = useState(false);
  useEffect(() => {
    if (!invoke) return;
    let active = true;
    let polling = false;
    let listed = false;
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
          if (!result.length)
            setMessage("No validated backup is available. Your damaged data has been kept for diagnosis.");
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
      setRecoveryMessage(
        "Backup restored. Routes are stopped and privacy mute is on. Reconnect clients and allow device access again when ready.",
      );
      await onRestored();
    } catch (error) {
      setRecoveryMessage(
        typeof error === "string" ? error : error instanceof Error ? error.message : "Backup restoration failed.",
      );
    } finally {
      setBusy(false);
      setArmed(false);
    }
  };
  return (
    <section aria-label="Saved data recovery" style={{ minHeight: "18rem" }}>
      <p role="status" style={{ height: "4.5rem", overflow: "auto" }}>
        {recoveryMessage ?? message}
      </p>
      <div style={{ visibility: needsRecovery ? "visible" : "hidden" }} aria-hidden={!needsRecovery}>
        <label style={{ display: "grid", gap: "0.5rem" }}>
          Recovery backup
          <select
            value={selected}
            disabled={!backups.length || busy}
            onChange={(event) => {
              setSelected(event.target.value);
              setArmed(false);
            }}
          >
            {!backups.length && <option value="">No backup selected</option>}
            {backups.map((backup) => (
              <option key={backup.name} value={backup.name}>
                {backup.name} ({Math.ceil(backup.bytes / 1048576)} MiB)
              </option>
            ))}
          </select>
        </label>
        <p className="muted" style={{ minHeight: "4.5rem" }}>
          Restoration keeps the damaged database, replaces saved configuration, turns privacy mute on, and resets client
          access and capture consent. Recordings do not resume.
        </p>
        <button type="button" disabled={!selected || busy} onClick={() => void restore()}>
          {busy ? "Restoring…" : armed ? "Confirm restore selected backup" : "Restore selected backup"}
        </button>
      </div>
    </section>
  );
}
