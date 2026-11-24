// Recorder and recording-library controls (moved from App.tsx).
import { useEffect, useState } from "react";
import { askConfirm } from "./AppDialog";
import { say, type UiMessage, withTone } from "./actionMessage";
import { uiError, type RecorderStatus, type UiBackend } from "./backend";
import { uiIdempotencyKey } from "./idempotency";
import { PanelMessage } from "./PanelMessage";
import { BooleanField } from "./BooleanField";
import { useMessages } from "./i18n/useMessages";
import { tx } from "./i18n";
import { optionLabel } from "./parameterText";
import { recordingStateLabel } from "./recordingStateLabel";

export function formatRecordingDuration(frames: number, sampleRate: number): string {
  if (!Number.isFinite(frames) || frames < 0 || !Number.isFinite(sampleRate) || sampleRate <= 0)
    return tx("recordingUi.unknown");
  const totalMilliseconds = Math.round((frames / sampleRate) * 1000);
  const hours = Math.floor(totalMilliseconds / 3_600_000);
  const minutes = Math.floor((totalMilliseconds % 3_600_000) / 60_000);
  const seconds = Math.floor((totalMilliseconds % 60_000) / 1000);
  const milliseconds = totalMilliseconds % 1000;
  return `${hours.toString().padStart(2, "0")}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}.${milliseconds.toString().padStart(3, "0")}`;
}

export function recorderHasCaptureSource(
  session: import("@audiorouter/contracts").Session,
  targetNodeId: string,
): boolean {
  const visited = new Set<string>();
  const pending = [targetNodeId];
  while (pending.length > 0) {
    const destination = pending.pop()!;
    if (visited.has(destination)) continue;
    visited.add(destination);
    for (const edge of session.edges) {
      if (!edge.enabled || edge.destinationNode !== destination) continue;
      const source = session.nodes.find((item) => item.id === edge.sourceNode);
      if (!source) continue;
      if (source.kind === "physicalInput" || source.kind === "applicationCapture") return true;
      pending.push(source.id);
    }
  }
  return false;
}

export function RecorderActions({
  backend,
  sessionId,
  connected,
  recorderStatuses,
  recorderStatusAvailable,
  recorderNodeIds,
  selectedNodeId,
  onSelectNode,
  format,
  onFormatChange,
}: {
  backend: UiBackend;
  sessionId: string;
  connected: boolean;
  recorderStatuses: RecorderStatus[];
  recorderStatusAvailable: boolean;
  recorderNodeIds: string[];
  selectedNodeId: string;
  onSelectNode: (nodeId: string) => void;
  format: import("@audiorouter/contracts").RecorderFileFormat;
  onFormatChange: (value: import("@audiorouter/contracts").RecorderFileFormat) => void;
}) {
  const tr = useMessages();
  const [frameText, setFrameText] = useState("0");
  const [state, setState] = useState("idle");
  const [lastFrame, setLastFrame] = useState<number | null>(null);
  const [nodeId, setNodeId] = useState<string | null>(null);
  const [message, setMessage] = useState<UiMessage | null>(null);
  const [recorderId, setRecorderId] = useState("voice-recording");
  const [channels, setChannels] = useState<1 | 2>(2);
  const [sampleRate, setSampleRate] = useState<44100 | 48000>(48000);
  const [dither, setDither] = useState(true);
  const [busy, setBusy] = useState(false);
  const frame = Number.parseInt(frameText, 10);
  const validFrame = Number.isSafeInteger(frame) && frame >= 0;
  const selectedRecorderNodeId = recorderNodeIds.includes(selectedNodeId) ? selectedNodeId : "";
  useEffect(() => {
    if (!recorderStatusAvailable) {
      setState("unavailable");
      setLastFrame(null);
      setNodeId(null);
      return;
    }
    const status = recorderStatuses.find((item) => item.sessionId === sessionId);
    if (status) {
      setState(status.state);
      setLastFrame(status.lastFrame);
      setNodeId(status.nodeId ?? null);
    } else {
      setState("idle");
      setLastFrame(null);
      setNodeId(null);
    }
  }, [recorderStatuses, recorderStatusAvailable, sessionId]);
  const create = async () => {
    if (!recorderId.trim()) {
      setMessage(say("recordingUi.provideId"));
      return;
    }
    if (busy || !connected) return;
    setBusy(true);
    setMessage(say("recordingUi.creating"));
    try {
      const result = await backend.createRecorder({
        sessionId,
        nodeId: selectedRecorderNodeId || undefined,
        recorderId: recorderId.trim(),
        format,
        sequence: 1,
        channels,
        sampleRate,
        dither,
        queueCapacity: 8,
        maximumChunksPerPass: 1,
        idempotencyKey: uiIdempotencyKey("recorder-create"),
      });
      setState(result.state);
      setMessage(say("recordingUi.created", { id: result.recorderId, path: result.path }));
    } catch (error) {
      setMessage(uiError(error, tr("recordingUi.createError")));
    } finally {
      setBusy(false);
    }
  };
  const run = async (operation: () => Promise<{ state: string; lastFrame?: number | null }>) => {
    if (busy || !connected) return;
    setBusy(true);
    setMessage(say("recordingUi.applying"));
    try {
      const result = await operation();
      setState(result.state);
      if (result.lastFrame !== undefined) setLastFrame(result.lastFrame ?? null);
      const updated = say("recordingUi.updated", { state: recordingStateLabel(result.state), frame });
      // A recorder that reports `failed` is a failure, whatever language its label is in.
      setMessage(result.state === "failed" ? withTone(updated, "error") : updated);
    } catch (error) {
      setState("failed");
      setMessage(uiError(error, tr("recordingUi.actionError")));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel recorder-actions" aria-labelledby="recorder-actions-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("recordingUi.manual")}</p>
          <h2 id="recorder-actions-heading">{tr("recordingUi.recorder")}</h2>
        </div>
        <span className="badge recorder-state">{recordingStateLabel(state)}</span>
      </div>
      <label>
        {tr("recordingUi.canvasRecorder")}
        <select
          aria-label={tr("recordingUi.canvasRecorder")}
          value={selectedRecorderNodeId}
          disabled={!connected || recorderNodeIds.length === 0}
          onChange={(event) => onSelectNode(event.target.value)}
        >
          <option value="">{tr("recordingUi.wholeSession")}</option>
          {recorderNodeIds.map((id) => (
            <option key={id} value={id}>
              {id}
              {id === selectedRecorderNodeId ? tr("recordingUi.selected") : ""}
            </option>
          ))}
        </select>
      </label>
      <fieldset disabled={!connected}>
        <legend>{tr("recordingUi.setup")}</legend>
        <label>
          {tr("recordingUi.recorderId")}
          <input
            aria-label={tr("recordingUi.recorderId")}
            value={recorderId}
            onChange={(event) => setRecorderId(event.target.value)}
          />
        </label>
        <label>
          {tr("recordingUi.format")}
          <select
            aria-label={tr("recordingUi.recorderFormat")}
            value={format}
            onChange={(event) => {
              const next = event.target.value as typeof format;
              onFormatChange(next);
              if (next === "wavFloat32" || next === "mp3") setDither(false);
            }}
          >
            {(["wavPcm24", "wavPcm16", "wavFloat32", "flac16", "flac24"] as const).map((fileFormat) => (
              <option key={fileFormat} value={fileFormat}>
                {optionLabel(fileFormat)}
              </option>
            ))}
            <option value="mp3">MP3 192 kbps</option>
          </select>
        </label>
        <label>
          {tr("workflowUi.channels")}
          <select
            aria-label={tr("recordingUi.recorderChannels")}
            value={channels}
            onChange={(event) => setChannels(Number(event.target.value) as 1 | 2)}
          >
            <option value={1}>{tr("recordingUi.mono")}</option>
            <option value={2}>{tr("recordingUi.stereo")}</option>
          </select>
        </label>
        <label>
          {tr("recordingUi.sampleRate")}
          <select
            aria-label={tr("recordingUi.recorderSampleRate")}
            value={sampleRate}
            onChange={(event) => setSampleRate(Number(event.target.value) as 44100 | 48000)}
          >
            <option value={48000}>48 kHz</option>
            <option value={44100}>44.1 kHz</option>
          </select>
        </label>
        <BooleanField
          label={tr("recordingUi.dither")}
          checked={dither}
          disabled={format === "wavFloat32" || format === "mp3"}
          onChange={setDither}
        />
        {(format === "wavFloat32" || format === "mp3") && (
          <small>{format === "mp3" ? tr("recordingUi.mp3Help") : tr("recordingUi.floatHelp")}</small>
        )}
        <button type="button" className="secondary" onClick={() => void create()}>
          {tr("recordingUi.create")}
        </button>
      </fieldset>
      <label>
        {tr("recordingUi.engineFrame")}
        <input
          aria-label={tr("recordingUi.recorderFrame")}
          inputMode="numeric"
          value={frameText}
          onChange={(event) => setFrameText(event.target.value)}
          disabled={!connected}
        />
      </label>
      <div className="actions">
        <button
          type="button"
          className="secondary"
          onClick={() =>
            void run(() =>
              backend.armRecorder(sessionId, uiIdempotencyKey("recorder-arm"), selectedRecorderNodeId || undefined),
            )
          }
          disabled={!connected}
        >
          {tr("recordingUi.arm")}
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() =>
            void run(() =>
              backend.startRecorder(
                sessionId,
                frame,
                uiIdempotencyKey("recorder-start"),
                selectedRecorderNodeId || undefined,
              ),
            )
          }
          disabled={!connected || !validFrame}
        >
          {tr("recordingUi.start")}
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() =>
            void run(() =>
              backend.pauseRecorder(
                sessionId,
                frame,
                uiIdempotencyKey("recorder-pause"),
                selectedRecorderNodeId || undefined,
              ),
            )
          }
          disabled={!connected || !validFrame}
        >
          {tr("recordingUi.pause")}
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() =>
            void run(() =>
              backend.resumeRecorder(
                sessionId,
                frame,
                uiIdempotencyKey("recorder-resume"),
                selectedRecorderNodeId || undefined,
              ),
            )
          }
          disabled={!connected || !validFrame}
        >
          {tr("recordingUi.resume")}
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() =>
            void run(() =>
              backend.splitRecorder(
                sessionId,
                frame,
                uiIdempotencyKey("recorder-split"),
                selectedRecorderNodeId || undefined,
              ),
            )
          }
          disabled={!connected || !validFrame}
        >
          {tr("recordingUi.split")}
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() =>
            void run(() =>
              backend.stopRecorder(
                sessionId,
                frame,
                uiIdempotencyKey("recorder-stop"),
                selectedRecorderNodeId || undefined,
              ),
            )
          }
          disabled={!connected || !validFrame}
        >
          {tr("recordingUi.stop")}
        </button>
      </div>
      {message && <PanelMessage message={message} />}
      {nodeId && (
        <p className="muted" role="status">
          {tr("recordingUi.attached", { id: nodeId })}
        </p>
      )}
      {lastFrame !== null && (
        <p className="muted" role="status">
          {tr("recordingUi.lastFrame", { frame: lastFrame })}
        </p>
      )}
      <p className="muted">{tr("recordingUi.engineHelp")}</p>
    </section>
  );
}

export function RecordingActions({
  recordings,
  connected,
  busy,
  onRename,
  onReveal,
  onRecycle,
}: {
  recordings: import("@audiorouter/contracts").RecordingRow[];
  connected: boolean;
  busy: boolean;
  onRename: (recordingId: string, newPath: string) => Promise<void>;
  onReveal: (recordingId: string) => Promise<void>;
  onRecycle: (recordingId: string, confirm: boolean) => Promise<void>;
}) {
  const tr = useMessages();
  const [selectedId, setSelectedId] = useState("");
  const [newPath, setNewPath] = useState("");
  const selected = recordings.find((recording) => recording.id === selectedId) ?? recordings[0];
  const shownId = selected?.id ?? "";
  const shownPath = selected?.path ?? "";
  useEffect(() => {
    setSelectedId(shownId);
    setNewPath(shownPath);
  }, [shownId, shownPath]);
  if (!selected) return null;
  return (
    <section className="panel recording-actions" aria-labelledby="recording-actions-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{tr("recordingUi.fileActions")}</p>
          <h2 id="recording-actions-heading">{tr("recordingUi.operations")}</h2>
        </div>
        <span className="badge">{tr("recordingUi.explicit")}</span>
      </div>
      <label>
        {tr("recordingUi.recording")}
        <select
          aria-label={tr("recordingUi.actionTarget")}
          value={selected.id}
          onChange={(event) => {
            const next = recordings.find((recording) => recording.id === event.target.value);
            setSelectedId(event.target.value);
            setNewPath(next?.path ?? "");
          }}
          disabled={busy}
        >
          {recordings.map((recording) => (
            <option key={recording.id} value={recording.id}>
              {recording.title || recording.id} · {recordingStateLabel(recording.state)}
            </option>
          ))}
        </select>
      </label>
      <label>
        {tr("recordingUi.newPath")}
        <input
          type="text"
          value={newPath}
          onChange={(event) => setNewPath(event.target.value)}
          disabled={!connected || busy}
        />
      </label>
      <div className="actions">
        <button
          type="button"
          className="secondary"
          onClick={() => void onRename(selected.id, newPath)}
          disabled={!connected || busy || !newPath.trim() || newPath === selected.path}
        >
          {tr("recordingUi.rename")}
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => void onReveal(selected.id)}
          disabled={!connected || busy}
        >
          {tr("recordingUi.reveal")}
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => void onRecycle(selected.id, false)}
          disabled={!connected || busy}
        >
          {tr("recordingUi.previewRecycle")}
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => {
            void askConfirm({
              title: tr("recordingUi.recycleTitle"),
              message: tr("recordingUi.recycleHelp"),
              confirmLabel: tr("recordingUi.recycle"),
              destructive: true,
            }).then((confirmed) => {
              if (confirmed) void onRecycle(selected.id, true);
            });
          }}
          disabled={!connected || busy}
        >
          {tr("recordingUi.recycle")}
        </button>
      </div>
      <p className="muted">{tr("recordingUi.actionsHelp")}</p>
    </section>
  );
}
