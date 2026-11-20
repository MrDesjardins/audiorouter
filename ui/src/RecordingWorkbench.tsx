// Content of the Recording side-panel tab: recorder actions and the recording library (moved from App.tsx).
import type { Dispatch, SetStateAction } from "react";
import type { Node, RecorderFileFormat, RecordingRow, Session } from "@audiorouter/contracts";
import type { RecorderStatus, UiBackend } from "./backend";
import { RecordingFolderField } from "./RecorderControls";
import { RecorderActions, RecordingActions, formatRecordingDuration } from "./RecordingPanels";
import { recordingStateLabel } from "./recordingStateLabel";
import { useMessages } from "./i18n/useMessages";

export function RecordingWorkbench({
  backend,
  session,
  recorderStatuses,
  recorderStatusAvailable,
  draft,
  selectedNode,
  setSelectedNodeId,
  setSelectedNodeIds,
  recorderFormat,
  setRecorderFormat,
  recordings,
  recordingMutationBusyState,
  renameRecording,
  revealRecording,
  recycleRecording,
  recordingsError,
  visibleRecordings,
  recordingSearch,
  setRecordingSearch,
  metadataTitles,
  setMetadataTitles,
  metadataArtists,
  setMetadataArtists,
  metadataComments,
  setMetadataComments,
  saveRecordingMetadata,
  previewRecording,
  inspectRecovery,
  removeRecordingEntry,
  previewMessage,
  recoveryMessage,
}: {
  backend: UiBackend;
  session: Session;
  recorderStatuses: RecorderStatus[];
  recorderStatusAvailable: boolean;
  draft: Session;
  selectedNode: Node | undefined;
  setSelectedNodeId: Dispatch<SetStateAction<string>>;
  setSelectedNodeIds: (ids: string[]) => void;
  recorderFormat: RecorderFileFormat;
  setRecorderFormat: Dispatch<SetStateAction<RecorderFileFormat>>;
  recordings: RecordingRow[];
  recordingMutationBusyState: boolean;
  renameRecording: (recordingId: string, newPath: string) => Promise<void>;
  revealRecording: (recordingId: string) => Promise<void>;
  recycleRecording: (recordingId: string, confirm: boolean) => Promise<void>;
  recordingsError: string | null;
  visibleRecordings: RecordingRow[];
  recordingSearch: string;
  setRecordingSearch: Dispatch<SetStateAction<string>>;
  metadataTitles: Record<string, string>;
  setMetadataTitles: Dispatch<SetStateAction<Record<string, string>>>;
  metadataArtists: Record<string, string>;
  setMetadataArtists: Dispatch<SetStateAction<Record<string, string>>>;
  metadataComments: Record<string, string>;
  setMetadataComments: Dispatch<SetStateAction<Record<string, string>>>;
  saveRecordingMetadata: (recording: RecordingRow) => Promise<void>;
  previewRecording: (recordingId: string) => Promise<void>;
  inspectRecovery: (recordingId: string) => Promise<void>;
  removeRecordingEntry: (recordingId: string) => Promise<void>;
  previewMessage: string | null;
  recoveryMessage: string | null;
}) {
  const tr = useMessages();
  return (
    <>
      <RecordingFolderField backend={backend} connected={backend.connected} />
      <RecorderActions
        backend={backend}
        sessionId={session.id}
        connected={backend.connected}
        recorderStatuses={recorderStatuses}
        recorderStatusAvailable={recorderStatusAvailable}
        recorderNodeIds={draft.nodes.filter((node) => node.kind === "recorder").map((node) => node.id)}
        selectedNodeId={selectedNode?.id ?? ""}
        onSelectNode={(nodeId) => {
          setSelectedNodeId(nodeId);
          setSelectedNodeIds([nodeId]);
        }}
        format={recorderFormat}
        onFormatChange={setRecorderFormat}
      />
      <RecordingActions
        recordings={recordings}
        connected={backend.connected}
        busy={recordingMutationBusyState}
        onRename={renameRecording}
        onReveal={revealRecording}
        onRecycle={recycleRecording}
      />
      {/* The recording library: browse, search, preview and edit takes. Until 2026-10-04 it rendered only in an always-hidden panel. */}
      <section className="recording-library" aria-label={tr("workflowUi.library")}>
        <div className="panel">
          <div className="section-heading">
            <h2>{tr("workflowUi.recordings")}</h2>
            <span className="badge">
              {recordingsError
                ? tr("common.unavailable")
                : tr(recordingSearch.trim() ? "workflowUi.filtered" : "workflowUi.files", {
                    count: visibleRecordings.length,
                    total: recordings.length,
                  })}
            </span>
          </div>
          <label className="recording-search">
            {tr("workflowUi.searchRecordings")}
            <input
              id="recording-search"
              type="search"
              value={recordingSearch}
              onChange={(event) => setRecordingSearch(event.target.value.slice(0, 160))}
              placeholder={tr("workflowUi.searchPlaceholder")}
            />
          </label>
          {recordingsError ? (
            <p className="muted">{tr("workflowUi.libraryError", { reason: recordingsError })}</p>
          ) : recordings.length === 0 ? (
            <div className="recording-empty">
              <span aria-hidden="true">◉</span>
              <h3>{tr("workflowUi.noRecordings")}</h3>
              <p>{tr("workflowUi.noRecordingsHelp")}</p>
            </div>
          ) : visibleRecordings.length === 0 ? (
            <p className="muted">{tr("workflowUi.noMatch")}</p>
          ) : (
            visibleRecordings.map((recording) => (
              <article
                className="recording-row"
                key={recording.id}
                aria-label={recording.title || recording.path.split(/[\\/]/).pop() || recording.id}
              >
                <div className="recording-row-heading">
                  <strong>{recording.title || recording.path.split(/[\\/]/).pop()}</strong>
                  <span className="badge">
                    {recording.missing ? tr("workflowUi.missing") : recordingStateLabel(recording.state)}
                  </span>
                </div>
                <small className="recording-row-path">{recording.path}</small>
                <small>
                  {tr("workflowUi.duration", {
                    duration: formatRecordingDuration(recording.frames, recording.sampleRate),
                    bytes: recording.fileBytes,
                  })}
                </small>
                <label>
                  {tr("workflowUi.title")}
                  <input
                    aria-label={tr("workflowUi.titleFor", { id: recording.id })}
                    value={metadataTitles[recording.id] ?? recording.title ?? ""}
                    onChange={(event) =>
                      setMetadataTitles((current) => ({ ...current, [recording.id]: event.target.value }))
                    }
                  />
                </label>
                <label>
                  {tr("workflowUi.artist")}
                  <input
                    aria-label={tr("workflowUi.artistFor", { id: recording.id })}
                    value={metadataArtists[recording.id] ?? recording.artist ?? ""}
                    onChange={(event) =>
                      setMetadataArtists((current) => ({ ...current, [recording.id]: event.target.value }))
                    }
                  />
                </label>
                <label>
                  {tr("workflowUi.comment")}
                  <input
                    aria-label={tr("workflowUi.commentFor", { id: recording.id })}
                    value={metadataComments[recording.id] ?? recording.comment ?? ""}
                    onChange={(event) =>
                      setMetadataComments((current) => ({ ...current, [recording.id]: event.target.value }))
                    }
                  />
                </label>
                <div className="recording-row-actions">
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void saveRecordingMetadata(recording)}
                    disabled={!backend.connected}
                  >
                    {tr("workflowUi.saveMetadata")}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void previewRecording(recording.id)}
                    disabled={!backend.connected}
                  >
                    {tr("workflowUi.preview")}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void inspectRecovery(recording.id)}
                    disabled={!backend.connected}
                  >
                    {tr("workflowUi.recovery")}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void removeRecordingEntry(recording.id)}
                    disabled={!backend.connected}
                  >
                    {tr("workflowUi.removeEntry")}
                  </button>
                </div>
              </article>
            ))
          )}
          {previewMessage && (
            <p className="muted" role="status">
              {previewMessage}
            </p>
          )}
          {recoveryMessage && (
            <p className="muted" role="status">
              {recoveryMessage}
            </p>
          )}
        </div>
      </section>
    </>
  );
}
