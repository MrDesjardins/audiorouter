// Recorder and recording-library controls (moved from App.tsx).
import { useEffect, useState } from "react";
import { formatUiError, type RecorderStatus, type UiBackend } from "./backend";
import { uiIdempotencyKey } from "./idempotency";
import { PanelMessage } from "./PanelMessage";

export function formatRecordingDuration(frames: number, sampleRate: number): string {
  if (!Number.isFinite(frames) || frames < 0 || !Number.isFinite(sampleRate) || sampleRate <= 0) return "unknown";
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
  const [frameText, setFrameText] = useState("0");
  const [state, setState] = useState("idle");
  const [lastFrame, setLastFrame] = useState<number | null>(null);
  const [nodeId, setNodeId] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
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
      setMessage("Provide a recorder ID.");
      return;
    }
    if (busy || !connected) return;
    setBusy(true);
    setMessage("Creating an unarmed recorder...");
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
      setMessage(`Recorder ${result.recorderId} created unarmed at ${result.path}.`);
    } catch (error) {
      setMessage(formatUiError(error, "Unable to create recorder."));
    } finally {
      setBusy(false);
    }
  };
  const run = async (action: string, operation: () => Promise<{ state: string; lastFrame?: number | null }>) => {
    if (busy || !connected) return;
    setBusy(true);
    setMessage(`${action}...`);
    try {
      const result = await operation();
      setState(result.state);
      if (result.lastFrame !== undefined) setLastFrame(result.lastFrame ?? null);
      setMessage(`Recorder ${result.state} at frame ${frame}.`);
    } catch (error) {
      setState("failed");
      setMessage(formatUiError(error, `Unable to ${action.toLowerCase()} recorder.`));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel recorder-actions" aria-labelledby="recorder-actions-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Frame boundary control</p>
          <h2 id="recorder-actions-heading">Recorder</h2>
        </div>
        <span className="badge">{state}</span>
      </div>
      <label>
        Graph recorder node
        <select
          aria-label="Graph recorder node"
          value={selectedRecorderNodeId}
          disabled={!connected || recorderNodeIds.length === 0}
          onChange={(event) => onSelectNode(event.target.value)}
        >
          <option value="">Session recorder (no graph node)</option>
          {recorderNodeIds.map((id) => (
            <option key={id} value={id}>
              {id}
              {id === selectedRecorderNodeId ? " (selected)" : ""}
            </option>
          ))}
        </select>
      </label>
      <fieldset disabled={!connected}>
        <legend>Create unarmed recorder</legend>
        <label>
          Recorder ID
          <input aria-label="Recorder ID" value={recorderId} onChange={(event) => setRecorderId(event.target.value)} />
        </label>
        <label>
          Format
          <select
            aria-label="Recorder format"
            value={format}
            onChange={(event) => {
              const next = event.target.value as typeof format;
              onFormatChange(next);
              if (next === "wavFloat32" || next === "mp3") setDither(false);
            }}
          >
            <option value="wavPcm24">WAV PCM24</option>
            <option value="wavPcm16">WAV PCM16</option>
            <option value="wavFloat32">WAV Float32</option>
            <option value="flac16">FLAC 16</option>
            <option value="flac24">FLAC 24</option>
            <option value="mp3">MP3 192 kbps</option>
          </select>
        </label>
        <label>
          Channels
          <select
            aria-label="Recorder channels"
            value={channels}
            onChange={(event) => setChannels(Number(event.target.value) as 1 | 2)}
          >
            <option value={1}>Mono</option>
            <option value={2}>Stereo</option>
          </select>
        </label>
        <label>
          Sample rate
          <select
            aria-label="Recorder sample rate"
            value={sampleRate}
            onChange={(event) => setSampleRate(Number(event.target.value) as 44100 | 48000)}
          >
            <option value={48000}>48 kHz</option>
            <option value={44100}>44.1 kHz</option>
          </select>
        </label>
        <label>
          TPDF dither
          <input
            aria-label="TPDF dither"
            type="checkbox"
            checked={dither}
            disabled={format === "wavFloat32" || format === "mp3"}
            onChange={(event) => setDither(event.target.checked)}
          />
        </label>
        {(format === "wavFloat32" || format === "mp3") && (
          <small>
            {format === "mp3"
              ? "MP3 uses the encoder's bounded psychoacoustic quantization; TPDF dither is not applied."
              : "Float32 output is not dithered."}
          </small>
        )}
        <button type="button" className="secondary" onClick={() => void create()}>
          Create recorder
        </button>
      </fieldset>
      <label>
        Engine frame
        <input
          aria-label="Recorder engine frame"
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
            void run("Arming", () =>
              backend.armRecorder(sessionId, uiIdempotencyKey("recorder-arm"), selectedRecorderNodeId || undefined),
            )
          }
          disabled={!connected}
        >
          Arm
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() =>
            void run("Starting", () =>
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
          Start
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() =>
            void run("Pausing", () =>
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
          Pause
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() =>
            void run("Resuming", () =>
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
          Resume
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() =>
            void run("Splitting", () =>
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
          Split
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() =>
            void run("Stopping", () =>
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
          Stop
        </button>
      </div>
      {message && <PanelMessage message={message} />}
      {nodeId && (
        <p className="muted" role="status">
          Attached recorder node: {nodeId}
        </p>
      )}
      {lastFrame !== null && (
        <p className="muted" role="status">
          Backend last frame: {lastFrame}
        </p>
      )}
      <p className="muted">
        Actions are sent only to a connected backend and use explicit engine frame boundaries. The preview backend never
        arms or starts recording.
      </p>
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
  const [selectedId, setSelectedId] = useState("");
  const [newPath, setNewPath] = useState("");
  const selected = recordings.find((recording) => recording.id === selectedId) ?? recordings[0];
  useEffect(() => {
    if (selected) {
      setSelectedId(selected.id);
      setNewPath(selected.path);
    } else {
      setSelectedId("");
      setNewPath("");
    }
  }, [selected?.id, selected?.path]);
  if (!selected) return null;
  return (
    <section className="panel recording-actions" aria-labelledby="recording-actions-heading">
      <div className="section-heading">
        <div>
          <p className="eyebrow">File actions</p>
          <h2 id="recording-actions-heading">Recording operations</h2>
        </div>
        <span className="badge">explicit</span>
      </div>
      <label>
        Recording
        <select
          aria-label="Recording file action target"
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
              {recording.title || recording.id} · {recording.state}
            </option>
          ))}
        </select>
      </label>
      <label>
        New path
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
          Rename in approved directory
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => void onReveal(selected.id)}
          disabled={!connected || busy}
        >
          Reveal
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => void onRecycle(selected.id, false)}
          disabled={!connected || busy}
        >
          Preview recycle
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => {
            if (window.confirm("Recycle this recording through the operating system?"))
              void onRecycle(selected.id, true);
          }}
          disabled={!connected || busy}
        >
          Recycle recording
        </button>
      </div>
      <p className="muted">
        Rename is restricted by the backend to the approved directory. Reveal and recycle never run while disconnected;
        recycle requires an explicit confirmation.
      </p>
    </section>
  );
}
