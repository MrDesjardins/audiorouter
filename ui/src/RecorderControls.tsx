import { useEffect, useState } from "react";
import type { Node } from "@audiorouter/contracts";
import type { RecorderStatus, UiBackend } from "./backend";
import { formatUiError } from "./backend";
import { useMessages } from "./i18n/useMessages";

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
  const tr = useMessages();
  const recording = isNodeRecording(status);
  const elapsed = useRecordingClock(recording);
  const disabled = !connected || busy || !node.enabled || (!recording && !running);
  const title = tr(recording ? "workflowUi.stopSave" : !running ? "workflowUi.playFirst" : "workflowUi.startNode");
  return (
    <button
      type="button"
      className={`record-button${recording ? " is-recording" : ""}${compact ? " is-compact" : ""} nodrag nopan`}
      aria-pressed={recording}
      aria-label={tr(recording ? "workflowUi.stopName" : "workflowUi.recordName", { name: node.name })}
      title={title}
      disabled={disabled}
      onClick={(event) => {
        event.stopPropagation();
        onToggle(node.id, !recording);
      }}
    >
      <span className="record-button-icon" aria-hidden="true" />
      <span>
        {busy
          ? "…"
          : recording
            ? tr("workflowUi.stopElapsed", { time: formatElapsed(elapsed) })
            : tr("workflowUi.record")}
      </span>
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
  const tr = useMessages();
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
      setMessage(tr(result.created ? "recordingUi.folderCreated" : "recordingUi.folderSaved"));
      window.dispatchEvent(new Event(ROOT_CHANGED_EVENT));
    } catch (error) {
      setMessage(formatUiError(error, tr("backendErrors.emptyFailure")));
    } finally {
      setBusy(false);
    }
  };
  const showField = root === null || editing;
  return (
    <div
      className={`recording-folder${root === null ? " is-missing" : ""}`}
      role="group"
      aria-label={tr("recordingUi.folder")}
    >
      <strong>{tr("recordingUi.folder")}</strong>
      {root === null && <p className="muted">{tr("recordingUi.folderHelp")}</p>}
      {showField ? (
        <>
          <label className="recording-folder-field">
            {tr("recordingUi.folderPath")}
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
              {tr(busy ? "topbar.saving" : "recordingUi.useFolder")}
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
                {tr("common.cancel")}
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
              {tr("recordingUi.changeFolder")}
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
  const tr = useMessages();
  const recording = isNodeRecording(status);
  const format = FORMAT_LABELS[String(node.parameters.format ?? "wavPcm24")] ?? "WAV 24-bit";
  const split = Number(node.parameters.splitMinutes ?? 0);
  const auto = node.parameters.autoRecord === true;
  return (
    <section className="tool-visual recorder-controls" aria-label={tr("workflowUi.controls")}>
      <div className="dynamics-heading">
        <div>
          <strong>{tr("recordingUi.recording")}</strong>
          <small>
            {format} · {split > 0 ? tr("workflowUi.splitInterval", { minutes: split }) : tr("workflowUi.oneFile")}
            {auto ? ` · ${tr("workflowUi.autoWithPlay")}` : ""}
          </small>
        </div>
        <span className={`dynamics-pill is-${recording ? "work" : "idle"}`} role="status">
          {tr(recording ? "recordingUi.recording" : running ? "workflowUi.ready" : "workflowUi.notPlaying")}
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
        {message ?? tr(recording ? "workflowUi.working" : running ? "workflowUi.readyHelp" : "workflowUi.stoppedHelp")}
      </p>
      {lastPath && (
        <p className="recorder-last-file" title={lastPath}>
          {tr("workflowUi.lastFile")} <code>{lastPath.split(/[\\/]/).pop()}</code>
        </p>
      )}
    </section>
  );
}
