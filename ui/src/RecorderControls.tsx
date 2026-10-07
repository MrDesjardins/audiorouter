import { useEffect, useState } from "react";
import type { Node } from "@audiorouter/contracts";
import type { RecorderStatus, UiBackend } from "./backend";

const FORMAT_LABELS: Record<string, string> = {
  wavPcm24: "WAV 24-bit",
  wavPcm16: "WAV 16-bit",
  wavFloat32: "WAV 32-bit float",
  flac24: "FLAC 24-bit",
  flac16: "FLAC 16-bit",
  mp3: "MP3",
};

/** True while the backend reports this Recorder node as recording or paused. */
export function isNodeRecording(status: RecorderStatus | null | undefined): boolean {
  return status?.state === "recording" || status?.state === "paused";
}

/** Elapsed time since a recording was first seen running, ticking each second. */
export function useRecordingClock(recording: boolean): number {
  const [startedAt, setStartedAt] = useState<number | null>(null);
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!recording) {
      setStartedAt(null);
      return;
    }
    setStartedAt((current) => current ?? Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [recording]);
  return startedAt === null ? 0 : Math.max(0, Math.floor((now - startedAt) / 1000));
}

export const formatElapsed = (seconds: number) => {
  const hours = Math.floor(seconds / 3600),
    minutes = Math.floor((seconds % 3600) / 60),
    rest = seconds % 60;
  return `${hours > 0 ? `${hours}:` : ""}${String(minutes).padStart(hours > 0 ? 2 : 1, "0")}:${String(rest).padStart(2, "0")}`;
};

/** The one Record / Stop control, shared by the canvas node and Properties. */
export function RecordButton({
  node,
  status,
  running,
  connected,
  busy,
  onToggle,
  compact = false,
}: {
  node: Node;
  status: RecorderStatus | null | undefined;
  running: boolean;
  connected: boolean;
  busy: boolean;
  onToggle: (nodeId: string, record: boolean) => void;
  compact?: boolean;
}) {
  const recording = isNodeRecording(status);
  const elapsed = useRecordingClock(recording);
  const disabled = !connected || busy || !node.enabled || (!recording && !running);
  const title = recording
    ? "Stop and save the file"
    : !running
      ? "Press Play first: the Recorder saves what plays through it"
      : "Start recording what reaches this node";
  return (
    <button
      type="button"
      className={`record-button${recording ? " is-recording" : ""}${compact ? " is-compact" : ""} nodrag nopan`}
      aria-pressed={recording}
      aria-label={recording ? `Stop recording ${node.name}` : `Record ${node.name}`}
      title={title}
      disabled={disabled}
      onClick={(event) => {
        event.stopPropagation();
        onToggle(node.id, !recording);
      }}
    >
      <span className="record-button-icon" aria-hidden="true" />
      <span>{busy ? "…" : recording ? `Stop · ${formatElapsed(elapsed)}` : "Record"}</span>
    </button>
  );
}

const ROOT_CHANGED_EVENT = "audiorouter:recording-root";

/**
 * Where recordings are saved. Until the user approves a folder, Record is
 * refused, so this offers a suggested folder that one click creates and
 * approves. Every instance (Properties, Recording tab) stays in step.
 */
export function RecordingFolderField({
  backend,
  connected,
}: {
  backend: Pick<UiBackend, "getRecordingRoot" | "setRecordingRoot">;
  connected: boolean;
}) {
  const [root, setRoot] = useState<string | null | undefined>(undefined);
  const [value, setValue] = useState("");
  const [editing, setEditing] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const available = connected && Boolean(backend.getRecordingRoot && backend.setRecordingRoot);
  useEffect(() => {
    if (!available) return;
    let cancelled = false;
    const load = () =>
      void backend.getRecordingRoot!()
        .then((result) => {
          if (cancelled) return;
          setRoot(result.root);
          setValue((current) => current || result.root || result.suggestedRoot || "");
        })
        .catch(() => {
          if (!cancelled) setRoot(undefined);
        });
    load();
    window.addEventListener(ROOT_CHANGED_EVENT, load);
    return () => {
      cancelled = true;
      window.removeEventListener(ROOT_CHANGED_EVENT, load);
    };
  }, [available, backend]);
  if (!available || root === undefined) return null;
  const save = async () => {
    setBusy(true);
    setMessage(null);
    try {
      const result = await backend.setRecordingRoot!(
        value.trim(),
        true,
        `recording-root-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      );
      setRoot(result.root);
      setValue(result.root);
      setEditing(false);
      setMessage(result.created ? "Folder created. Recordings will be saved here." : "Recordings will be saved here.");
      window.dispatchEvent(new Event(ROOT_CHANGED_EVENT));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };
  const showField = root === null || editing;
  return (
    <div className={`recording-folder${root === null ? " is-missing" : ""}`} role="group" aria-label="Recording folder">
      <strong>Recording folder</strong>
      {root === null && (
        <p className="muted">
          Choose where recordings are saved before pressing Record. The suggested folder is created for you.
        </p>
      )}
      {showField ? (
        <>
          <label className="recording-folder-field">
            Folder path
            <input
              type="text"
              value={value}
              spellCheck={false}
              disabled={busy}
              onChange={(event) => setValue(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && value.trim()) void save();
              }}
            />
          </label>
          <div className="recording-folder-actions">
            <button type="button" className="primary" disabled={busy || !value.trim()} onClick={() => void save()}>
              {busy ? "Saving…" : "Use this folder"}
            </button>
            {root !== null && (
              <button
                type="button"
                className="secondary"
                disabled={busy}
                onClick={() => {
                  setEditing(false);
                  setValue(root);
                  setMessage(null);
                }}
              >
                Cancel
              </button>
            )}
          </div>
        </>
      ) : (
        <>
          <code className="recording-folder-path" title={root}>
            {root}
          </code>
          <div className="recording-folder-actions">
            <button
              type="button"
              className="secondary"
              onClick={() => {
                setEditing(true);
                setMessage(null);
              }}
            >
              Change folder
            </button>
          </div>
        </>
      )}
      {message && (
        <p className="muted" role="status">
          {message}
        </p>
      )}
    </div>
  );
}

export function RecorderControls({
  node,
  status,
  running,
  connected,
  busy,
  lastPath,
  message,
  onToggle,
}: {
  node: Node;
  status: RecorderStatus | null | undefined;
  running: boolean;
  connected: boolean;
  busy: boolean;
  lastPath: string | null;
  message: string | null;
  onToggle: (nodeId: string, record: boolean) => void;
}) {
  const recording = isNodeRecording(status);
  const format = FORMAT_LABELS[String(node.parameters.format ?? "wavPcm24")] ?? "WAV 24-bit";
  const split = Number(node.parameters.splitMinutes ?? 0);
  const auto = node.parameters.autoRecord === true;
  return (
    <section className="tool-visual recorder-controls" aria-label="Recorder controls">
      <div className="dynamics-heading">
        <div>
          <strong>Recording</strong>
          <small>
            {format} · {split > 0 ? `new file every ${split} min` : "one file per take"}
            {auto ? " · starts with Play" : ""}
          </small>
        </div>
        <span className={`dynamics-pill is-${recording ? "work" : "idle"}`} role="status">
          {recording ? "Recording" : running ? "Ready" : "Not playing"}
        </span>
      </div>
      <RecordButton
        node={node}
        status={status}
        running={running}
        connected={connected}
        busy={busy}
        onToggle={onToggle}
      />
      <p className="muted dynamics-status">
        {message ??
          (recording
            ? "Everything reaching this node is being saved. Stop saves the file; stopping playback also saves it."
            : running
              ? "Press Record to save what plays through this node. Turn on automatic recording below to start with every Play."
              : "Press Play, then Record. Files are saved in the recording folder below.")}
      </p>
      {lastPath && (
        <p className="recorder-last-file" title={lastPath}>
          Last file: <code>{lastPath.split(/[\\/]/).pop()}</code>
        </p>
      )}
    </section>
  );
}
