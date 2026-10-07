// Content of the Recording side-panel tab: recorder actions and the recording library (moved from App.tsx).
import type { Dispatch, SetStateAction } from "react";
import type { Node, RecorderFileFormat, RecordingRow, Session } from "@audiorouter/contracts";
import type { RecorderStatus, UiBackend } from "./backend";
import { RecordingFolderField } from "./RecorderControls";
import { RecorderActions, RecordingActions, formatRecordingDuration } from "./RecordingPanels";

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
  selectedNode: Node;
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
        selectedNodeId={selectedNode.id}
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
      <p className="muted">
        {recordingsError ??
          (recordings.length === 0
            ? "No completed recording files yet."
            : `${recordings.length} recording files are available.`)}
      </p>
      {/* The recording library: browse, search, preview and edit takes. Until 2026-10-04 it rendered only in an always-hidden panel. */}
      <section className="recording-library" aria-label="Recording library">
        <div className="panel">
          <div className="section-heading">
            <h2>Recordings</h2>
            <span className="badge">
              {recordingsError
                ? "unavailable"
                : `${visibleRecordings.length}${recordingSearch.trim() ? ` of ${recordings.length}` : ""} file${visibleRecordings.length === 1 ? "" : "s"}`}
            </span>
          </div>
          <label className="recording-search">
            Search recordings
            <input
              id="recording-search"
              type="search"
              value={recordingSearch}
              onChange={(event) => setRecordingSearch(event.target.value.slice(0, 160))}
              placeholder="Title, path, or status"
            />
          </label>
          {recordingsError ? (
            <p className="muted">Recording library unavailable: {recordingsError}</p>
          ) : recordings.length === 0 ? (
            <p className="muted">
              No recording has been armed. Completed recordings will appear here with path and status.
            </p>
          ) : visibleRecordings.length === 0 ? (
            <p className="muted">No recording matches this search.</p>
          ) : (
            visibleRecordings.map((recording) => (
              <article
                className="recording-row"
                key={recording.id}
                aria-label={recording.title || recording.path.split(/[\\/]/).pop() || recording.id}
              >
                <div className="recording-row-heading">
                  <strong>{recording.title || recording.path.split(/[\\/]/).pop()}</strong>
                  <span className="badge">{recording.missing ? "missing" : recording.state}</span>
                </div>
                <small className="recording-row-path">{recording.path}</small>
                <small>
                  Duration {formatRecordingDuration(recording.frames, recording.sampleRate)} · {recording.fileBytes}{" "}
                  bytes
                </small>
                <label>
                  Title
                  <input
                    aria-label={`Title for ${recording.id}`}
                    value={metadataTitles[recording.id] ?? recording.title ?? ""}
                    onChange={(event) =>
                      setMetadataTitles((current) => ({ ...current, [recording.id]: event.target.value }))
                    }
                  />
                </label>
                <label>
                  Artist
                  <input
                    aria-label={`Artist for ${recording.id}`}
                    value={metadataArtists[recording.id] ?? recording.artist ?? ""}
                    onChange={(event) =>
                      setMetadataArtists((current) => ({ ...current, [recording.id]: event.target.value }))
                    }
                  />
                </label>
                <label>
                  Comment
                  <input
                    aria-label={`Comment for ${recording.id}`}
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
                    Save metadata
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void previewRecording(recording.id)}
                    disabled={!backend.connected}
                  >
                    Preview
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void inspectRecovery(recording.id)}
                    disabled={!backend.connected}
                  >
                    Recovery
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void removeRecordingEntry(recording.id)}
                    disabled={!backend.connected}
                  >
                    Remove entry
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
