// Properties editor of an Audio File node (moved from App.tsx).
import { useEffect, useRef, useState } from "react";
import type { Node } from "@audiorouter/contracts";
import { formatUiError, type RecorderStatus, type UiBackend } from "./backend";
import { audioUploadProblem, uploadAudioMedia } from "./audioUpload";
import { uiIdempotencyKey } from "./idempotency";
import { PanelMessage } from "./PanelMessage";
import { BooleanField } from "./BooleanField";
import { useMessages } from "./i18n/useMessages";

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
  const tr = useMessages();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState(() => tr("g2.afChoosePrompt"));
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
      if (!recording) throw new Error(tr("g2.afTakeNotFinal"));
      const media = await backend.importTemporaryRecording(recording.id);
      if (currentSessionIdRef.current === sessionId) {
        onChange("mediaId", media.mediaId);
        onChange("fileName", media.fileName);
      }
      setMessage(tr("g2.afSoundCheckReady", { sec: Math.round(media.durationMs / 1000) }));
      activeTakeId.current = null;
      setTakeState("idle");
    } catch (error) {
      setMessage(formatUiError(error, tr("g2.afImportFailedTake")));
      setTakeState("idle");
    } finally {
      setBusy(false);
    }
  };
  const startTemporaryTake = async () => {
    if (!selectedRecorder || busy || !backend.connected || !sessionRunning) return;
    const recorderId = `audio-file-take-${Date.now().toString(36)}`;
    setBusy(true);
    setMessage(tr("g2.afPreparing"));
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
      setMessage(tr("g2.afRecording"));
      stopTimer.current = setTimeout(() => {
        void stopAndImportTake();
      }, 10_000);
    } catch (error) {
      setMessage(formatUiError(error, tr("g2.afTakeStartFailed")));
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
    setMessage(tr("g2.afImporting", { name: file.name }));
    try {
      const media = await uploadAudioMedia(backend, file);
      if (currentSessionIdRef.current !== sessionId) return;
      onChange("mediaId", media.mediaId);
      onChange("fileName", media.fileName);
      setMessage(
        tr("g2.afImported", {
          name: media.fileName,
          sec: Math.round(media.durationMs / 1000),
          channels: media.channels === 1 ? tr("g2.afChannelOne") : tr("g2.afChannelMany", { n: media.channels }),
        }),
      );
    } catch (error) {
      setMessage(formatUiError(error, tr("g2.afImportFailed")));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="node-binding-editor audio-file-editor" aria-label={tr("g2.afSettingsAria")}>
      <div>
        <p className="eyebrow">{tr("g2.afSource")}</p>
        <strong>{String(node.parameters.fileName ?? tr("g2.afNoFile"))}</strong>
      </div>
      <label>
        {tr("g2.afChooseFile")}
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
      <section className="temporary-take" aria-label={tr("g2.afSoundCheck")}>
        <p className="eyebrow">{tr("g2.afSoundCheck")}</p>
        <label>
          {tr("g2.afVoiceRecorder")}
          <select
            aria-label={tr("g2.afRecorderAria")}
            value={selectedRecorder}
            disabled={!backend.connected || busy || takeState === "recording" || recorderNodes.length === 0}
            onChange={(event) => setSelectedRecorder(event.target.value)}
          >
            <option value="">{tr("g2.afChooseRecorder")}</option>
            {recorderNodes.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name || item.id}
              </option>
            ))}
          </select>
        </label>
        {takeState === "recording" ? (
          <button type="button" className="secondary" disabled={busy} onClick={() => void stopAndImportTake()}>
            {tr("g2.afStopPrepare")}
          </button>
        ) : (
          <button
            type="button"
            className="secondary"
            disabled={!backend.connected || !sessionRunning || busy || !selectedRecorder}
            onClick={() => void startTemporaryTake()}
          >
            {takeState === "importing" ? tr("g2.afPreparingBtn") : tr("g2.afRecordBtn")}
          </button>
        )}
        <small>{tr("g2.afTakeHelp")}</small>
      </section>
      <div className="audio-file-inspector-transport" aria-label={tr("g2.afTransportAria")}>
        <button
          type="button"
          className="secondary"
          disabled={transportDisabled || !node.parameters.mediaId || state === "playing"}
          onClick={() => onTransport(node.id, "play")}
        >
          {tr("g2.afPlay")}
        </button>
        <button
          type="button"
          className="secondary"
          disabled={transportDisabled || !node.parameters.mediaId || state !== "playing"}
          onClick={() => onTransport(node.id, "pause")}
        >
          {tr("g2.afPause")}
        </button>
        <button
          type="button"
          className="secondary"
          disabled={transportDisabled || !node.parameters.mediaId || state === "stopped"}
          onClick={() => onTransport(node.id, "stop")}
        >
          {tr("g2.afStop")}
        </button>
        <span className="audio-file-node-state" role="status">
          {tr(
            state === "playing" ? "g2.afStatePlaying" : state === "paused" ? "g2.afStatePaused" : "g2.afStateStopped",
          )}
        </span>
      </div>
      <BooleanField
        label={tr("g2.afLoop")}
        checked={node.parameters.loop === true}
        disabled={!backend.connected || busy}
        onChange={(checked) => onChange("loop", checked)}
      />
      <div className="audio-file-editor-status">
        <PanelMessage message={message} />
      </div>
      <small>{tr("g2.afLimits")}</small>
    </div>
  );
}
