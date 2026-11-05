import { useRef, useState, type Dispatch, type SetStateAction } from "react";
import type { RecordingRow } from "@audiorouter/contracts";
import { formatUiError, type UiBackend } from "./backend";
import { uiIdempotencyKey } from "./idempotency";

export function useRecordingActions({
  backend,
  setRecordings,
  metadataTitles,
  metadataArtists,
  metadataComments,
}: {
  backend: UiBackend;
  setRecordings: Dispatch<SetStateAction<RecordingRow[]>>;
  metadataTitles: Record<string, string>;
  metadataArtists: Record<string, string>;
  metadataComments: Record<string, string>;
}) {
  const [previewMessage, setPreviewMessage] = useState<string | null>(null);
  const [recoveryMessage, setRecoveryMessage] = useState<string | null>(null);
  const previewRequest = useRef(0);
  const recoveryRequest = useRef(0);
  const recordingMutationBusy = useRef(false);
  const [recordingMutationBusyState, setRecordingMutationBusyState] = useState(false);
  const previewRecording = async (recordingId: string) => {
    const request = ++previewRequest.current;
    setPreviewMessage("Inspecting recording...");
    try {
      const result = await backend.previewRecording(recordingId);
      if (request === previewRequest.current)
        setPreviewMessage(`${String(result.preview.status)} recording preview loaded.`);
    } catch (error) {
      if (request === previewRequest.current) setPreviewMessage(formatUiError(error, "Recording preview unavailable."));
    }
  };
  const inspectRecovery = async (recordingId: string) => {
    const request = ++recoveryRequest.current;
    setRecoveryMessage("Inspecting recorder recovery...");
    try {
      const result = await backend.getRecordingRecovery(recordingId);
      if (request === recoveryRequest.current)
        setRecoveryMessage(
          result.status === "missing"
            ? "No persisted recovery checkpoint is available."
            : `Recovery checkpoint: ${result.checkpoint.state}.`,
        );
    } catch (error) {
      if (request === recoveryRequest.current)
        setRecoveryMessage(formatUiError(error, "Recording recovery unavailable."));
    }
  };
  const saveRecordingMetadata = async (recording: RecordingRow) => {
    if (!backend.connected || recordingMutationBusy.current) return;
    recordingMutationBusy.current = true;
    setRecordingMutationBusyState(true);
    try {
      const title = metadataTitles[recording.id]?.trim() ?? recording.title ?? "";
      const artist = metadataArtists[recording.id]?.trim() ?? recording.artist ?? "";
      const comment = metadataComments[recording.id]?.trim() ?? recording.comment ?? "";
      await backend.setRecordingMetadata(recording.id, {
        title: title || null,
        artist: artist || null,
        comment: comment || null,
        idempotencyKey: uiIdempotencyKey("recording-metadata"),
      });
      setRecordings((current) =>
        current.map((item) =>
          item.id === recording.id
            ? { ...item, title: title || null, artist: artist || null, comment: comment || null }
            : item,
        ),
      );
      setPreviewMessage("Recording metadata saved; the audio file was unchanged.");
    } catch (error) {
      setPreviewMessage(formatUiError(error, "Unable to save recording metadata."));
    } finally {
      recordingMutationBusy.current = false;
      setRecordingMutationBusyState(false);
    }
  };
  const removeRecordingEntry = async (recordingId: string) => {
    if (!window.confirm("Remove this library entry? The audio file will be preserved.")) return;
    if (!backend.connected || recordingMutationBusy.current) return;
    recordingMutationBusy.current = true;
    setRecordingMutationBusyState(true);
    try {
      await backend.removeRecordingEntry(recordingId, uiIdempotencyKey("recording-entry-remove"));
      setRecordings((current) => current.filter((item) => item.id !== recordingId));
      setPreviewMessage("Library entry removed; the audio file was preserved.");
    } catch (error) {
      setPreviewMessage(formatUiError(error, "Unable to remove recording entry."));
    } finally {
      recordingMutationBusy.current = false;
      setRecordingMutationBusyState(false);
    }
  };
  const renameRecording = async (recordingId: string, newPath: string) => {
    if (!backend.connected || recordingMutationBusy.current) return;
    recordingMutationBusy.current = true;
    setRecordingMutationBusyState(true);
    try {
      const result = await backend.renameRecording(recordingId, newPath.trim(), uiIdempotencyKey("recording-rename"));
      setRecordings((current) =>
        current.map((item) => (item.id === recordingId ? { ...item, path: result.path, missing: false } : item)),
      );
      setPreviewMessage("Recording renamed within the approved directory.");
    } catch (error) {
      setPreviewMessage(formatUiError(error, "Unable to rename recording."));
    } finally {
      recordingMutationBusy.current = false;
      setRecordingMutationBusyState(false);
    }
  };
  const revealRecording = async (recordingId: string) => {
    if (!backend.connected || recordingMutationBusy.current) return;
    recordingMutationBusy.current = true;
    setRecordingMutationBusyState(true);
    try {
      const result = await backend.revealRecording(recordingId);
      setPreviewMessage(
        result.revealed
          ? "Recording revealed by the operating system."
          : "Recording is missing; no operating-system action was performed.",
      );
    } catch (error) {
      setPreviewMessage(formatUiError(error, "Unable to reveal recording."));
    } finally {
      recordingMutationBusy.current = false;
      setRecordingMutationBusyState(false);
    }
  };
  const recycleRecording = async (recordingId: string, confirm: boolean) => {
    if (!backend.connected || recordingMutationBusy.current) return;
    recordingMutationBusy.current = true;
    setRecordingMutationBusyState(true);
    try {
      const result = await backend.recycleRecording(
        recordingId,
        confirm,
        confirm ? uiIdempotencyKey("recording-recycle") : undefined,
      );
      setPreviewMessage(
        result.fileAction === "recycled"
          ? "Recording recycled."
          : result.fileAction === "recycle"
            ? "Recycle preview loaded; confirmation is still required."
            : `Recording was not recycled: ${result.reason}.`,
      );
      if (result.fileAction === "recycled")
        setRecordings((current) =>
          current.map((item) => (item.id === recordingId ? { ...item, missing: true } : item)),
        );
    } catch (error) {
      setPreviewMessage(formatUiError(error, "Unable to recycle recording."));
    } finally {
      recordingMutationBusy.current = false;
      setRecordingMutationBusyState(false);
    }
  };
  return {
    previewMessage,
    recoveryMessage,
    recordingMutationBusyState,
    previewRecording,
    inspectRecovery,
    saveRecordingMetadata,
    removeRecordingEntry,
    renameRecording,
    revealRecording,
    recycleRecording,
  };
}
