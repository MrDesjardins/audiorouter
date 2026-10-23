// Properties editor of an Audio File node (moved from App.tsx).
import { useEffect, useRef, useState } from "react";
import type { Node } from "@audiorouter/contracts";
import { formatUiError, type RecorderStatus, type UiBackend } from "./backend";
import { audioUploadProblem, uploadAudioMedia } from "./audioUpload";
import { uiIdempotencyKey } from "./idempotency";
import { PanelMessage } from "./PanelMessage";

export function AudioFileNodeEditor({
  node,
  backend,
  disabled,
  transportDisabled,
  sessionRunning,
  state,
  sessionId,
  currentSessionIdRef,
  recorderNodes,
  recorderStatuses,
  onChange,
  onTransport,
}: {
  node: Node;
  backend: UiBackend;
  disabled: boolean;
  transportDisabled: boolean;
  sessionRunning: boolean;
  state: "playing" | "paused" | "stopped";
  sessionId: string;
  currentSessionIdRef: { current: string };
  recorderNodes: Node[];
  recorderStatuses: RecorderStatus[];
  onChange: (name: string, value: boolean | number | string) => void;
  onTransport: (nodeId: string, action: "play" | "pause" | "stop") => void;
}) {
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState(
    "Choose a WAV or MP3 file. Audio is decoded by the backend and routed through this graph.",
  );
  const [selectedRecorder, setSelectedRecorder] = useState("");
  const [takeState, setTakeState] = useState<"idle" | "recording" | "importing">("idle");
  const activeTakeId = useRef<string | null>(null);
  const stopTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const status = recorderStatuses.find((item) => item.sessionId === sessionId && item.nodeId === selectedRecorder);
  const recorderNode = recorderNodes.find((item) => item.id === selectedRecorder);
  useEffect(() => {
    if (!recorderNodes.some((item) => item.id === selectedRecorder)) setSelectedRecorder(recorderNodes[0]?.id ?? "");
  }, [recorderNodes, selectedRecorder]);
  const stopAndImportTake = async () => {
    const recorderId = activeTakeId.current;
    if (!recorderId || !selectedRecorder || busy) return;
    if (stopTimer.current) clearTimeout(stopTimer.current);
    stopTimer.current = null;
    setBusy(true);
    setTakeState("importing");
    try {
      const liveStatuses = await backend.listRecorders();
      const liveStatus = liveStatuses.find((item) => item.sessionId === sessionId && item.nodeId === selectedRecorder);
      if (liveStatus) {
        const frame = liveStatus.lastFrame ?? status?.lastFrame ?? 0;
        await backend.stopRecorder(sessionId, frame, uiIdempotencyKey("temporary-take-stop"), selectedRecorder);
      }
      const recordings = await backend.listRecordings(sessionId);
      const recording = [...recordings]
        .reverse()
        .find((item) => item.recorderId === recorderId && item.state === "completed");
      if (!recording) throw new Error("The temporary WAV take did not finalize. Check Recorder status and try again.");
      const media = await backend.importTemporaryRecording(recording.id);
      if (currentSessionIdRef.current === sessionId) {
        onChange("mediaId", media.mediaId);
        onChange("fileName", media.fileName);
      }
      setMessage(
        `Sound Check ready · ${Math.round(media.durationMs / 1000)} sec · expires in 24 hours. Plan and commit this Audio File source before replaying it through the effects.`,
      );
      activeTakeId.current = null;
      setTakeState("idle");
    } catch (error) {
      setMessage(formatUiError(error, "Temporary recording could not be imported."));
      setTakeState("idle");
    } finally {
      setBusy(false);
    }
  };
  const startTemporaryTake = async () => {
    if (!selectedRecorder || busy || !backend.connected || !sessionRunning) return;
    const recorderId = `audio-file-take-${Date.now().toString(36)}`;
    setBusy(true);
    setMessage("Preparing a 10-second Sound Check from the selected Recorder branch...");
    try {
      const channels = recorderNode?.ports.find((port) => port.direction === "input")?.channels ?? 2;
      await backend.createRecorder({
        sessionId,
        nodeId: selectedRecorder,
        recorderId,
        format: "wavPcm16",
        sequence: Date.now(),
        channels,
        sampleRate: 48000,
        dither: false,
        queueCapacity: 8,
        maximumChunksPerPass: 1,
        idempotencyKey: uiIdempotencyKey("temporary-take-create"),
      });
      await backend.armRecorder(sessionId, uiIdempotencyKey("temporary-take-arm"), selectedRecorder);
      const frame = status?.lastFrame ?? 0;
      await backend.startRecorder(sessionId, frame, uiIdempotencyKey("temporary-take-start"), selectedRecorder);
      activeTakeId.current = recorderId;
      setTakeState("recording");
      setMessage(
        "Recording Sound Check from the selected, already-routed Recorder node. It stops automatically after 10 seconds.",
      );
      stopTimer.current = setTimeout(() => {
        void stopAndImportTake();
      }, 10_000);
    } catch (error) {
      setMessage(formatUiError(error, "Temporary recording could not start."));
      setTakeState("idle");
    } finally {
      setBusy(false);
    }
  };
  const importFile = async (file?: File) => {
    if (!file) return;
    const problem = audioUploadProblem(file);
    if (problem) {
      setMessage(problem);
      return;
    }
    setBusy(true);
    try {
      const media = await uploadAudioMedia(backend, file);
      if (currentSessionIdRef.current !== sessionId) return;
      onChange("mediaId", media.mediaId);
      onChange("fileName", media.fileName);
      setMessage(
        `${media.fileName} · ${Math.round(media.durationMs / 1000)} sec · ${media.channels} channel${media.channels === 1 ? "" : "s"}. Plan and commit this Audio File source before replaying it through the effects.`,
      );
    } catch (error) {
      setMessage(formatUiError(error, "Audio import failed."));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="node-binding-editor audio-file-editor" aria-label="Audio file source settings">
      <div>
        <p className="eyebrow">Audio source</p>
        <strong>{String(node.parameters.fileName ?? "No file selected")}</strong>
      </div>
      <label>
        Choose WAV or MP3
        <input
          type="file"
          accept="audio/wav,audio/mpeg,.wav,.mp3"
          disabled={disabled || busy || takeState === "recording"}
          onChange={(event) => {
            void importFile(event.target.files?.[0]);
            event.currentTarget.value = "";
          }}
        />
      </label>
      <section className="temporary-take" aria-label="Sound Check">
        <p className="eyebrow">Sound Check</p>
        <label>
          Voice path Recorder
          <select
            aria-label="Recorder node for temporary take"
            value={selectedRecorder}
            disabled={!backend.connected || busy || takeState === "recording" || recorderNodes.length === 0}
            onChange={(event) => setSelectedRecorder(event.target.value)}
          >
            <option value="">Choose an enabled Recorder node</option>
            {recorderNodes.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name || item.id}
              </option>
            ))}
          </select>
        </label>
        {takeState === "recording" ? (
          <button type="button" className="secondary" disabled={busy} onClick={() => void stopAndImportTake()}>
            Stop and prepare replay
          </button>
        ) : (
          <button
            type="button"
            className="secondary"
            disabled={!backend.connected || !sessionRunning || busy || !selectedRecorder}
            onClick={() => void startTemporaryTake()}
          >
            {takeState === "importing" ? "Preparing Sound Check…" : "Record Sound Check"}
          </button>
        )}
        <small>
          Connect the voice source through this Recorder node and start the prepared session. Recording stops after 10
          seconds or when you stop it. The sample expires after 24 hours. Plan and commit this Audio File source, then
          use Play to hear it through the effects in the connected route. This control does not select or open a
          microphone.
        </small>
      </section>
      <div className="audio-file-inspector-transport" aria-label="Audio file playback controls">
        <button
          type="button"
          className="secondary"
          disabled={transportDisabled || !node.parameters.mediaId || state === "playing"}
          onClick={() => onTransport(node.id, "play")}
        >
          Play
        </button>
        <button
          type="button"
          className="secondary"
          disabled={transportDisabled || !node.parameters.mediaId || state !== "playing"}
          onClick={() => onTransport(node.id, "pause")}
        >
          Pause
        </button>
        <button
          type="button"
          className="secondary"
          disabled={transportDisabled || !node.parameters.mediaId || state === "stopped"}
          onClick={() => onTransport(node.id, "stop")}
        >
          Stop
        </button>
        <span className="audio-file-node-state" role="status">
          {state}
        </span>
      </div>
      <label>
        Loop
        <input
          type="checkbox"
          checked={node.parameters.loop === true}
          disabled={!backend.connected || busy}
          onChange={(event) => onChange("loop", event.target.checked)}
        />
      </label>
      <div className="audio-file-editor-status">
        <PanelMessage message={message} />
      </div>
      <small>
        Maximum imported file size is 64 MiB and duration 120 seconds. Playback uses this source in the current route
        preview and the exact prepared native output route.
      </small>
    </div>
  );
}
