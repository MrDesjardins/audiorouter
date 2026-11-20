import { useRef, useState, type Dispatch, type SetStateAction } from "react";
import type { RecordingRow } from "@audiorouter/contracts";
import { askConfirm } from "./AppDialog";
import { formatUiError, type UiBackend } from "./backend";
import { uiIdempotencyKey } from "./idempotency";
import { tx } from "./i18n";
import { recordingStateLabel } from "./recordingStateLabel";

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
    setPreviewMessage(tx("workflowActions.inspect"));
    try {
      const result = await backend.previewRecording(recordingId);
      if (request === previewRequest.current)
        setPreviewMessage(
          tx("workflowActions.previewLoaded", { state: recordingStateLabel(String(result.preview.status)) }),
        );
    } catch (error) {
      if (request === previewRequest.current)
        setPreviewMessage(formatUiError(error, tx("workflowActions.previewError")));
    }
  };
  const inspectRecovery = async (recordingId: string) => {
    const request = ++recoveryRequest.current;
    setRecoveryMessage(tx("workflowActions.inspectRecovery"));
    try {
      const result = await backend.getRecordingRecovery(recordingId);
      if (request === recoveryRequest.current)
        setRecoveryMessage(
          result.status === "missing"
            ? tx("workflowActions.noCheckpoint")
            : tx("workflowActions.checkpoint", { state: recordingStateLabel(result.checkpoint.state) }),
        );
    } catch (error) {
      if (request === recoveryRequest.current)
        setRecoveryMessage(formatUiError(error, tx("workflowActions.recoveryError")));
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
      setPreviewMessage(tx("workflowActions.metadataSaved"));
    } catch (error) {
      setPreviewMessage(formatUiError(error, tx("workflowActions.metadataError")));
    } finally {
      recordingMutationBusy.current = false;
      setRecordingMutationBusyState(false);
    }
  };
  const removeRecordingEntry = async (recordingId: string) => {
    const confirmed = await askConfirm({
      title: tx("workflowActions.removeTitle"),
      message: tx("workflowActions.removeHelp"),
      confirmLabel: tx("workflowUi.removeEntry"),
      destructive: true,
    });
    if (!confirmed) return;
    if (!backend.connected || recordingMutationBusy.current) return;
    recordingMutationBusy.current = true;
    setRecordingMutationBusyState(true);
    try {
      await backend.removeRecordingEntry(recordingId, uiIdempotencyKey("recording-entry-remove"));
      setRecordings((current) => current.filter((item) => item.id !== recordingId));
      setPreviewMessage(tx("workflowActions.removed"));
    } catch (error) {
      setPreviewMessage(formatUiError(error, tx("workflowActions.removeError")));
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
      setPreviewMessage(tx("workflowActions.renamed"));
    } catch (error) {
      setPreviewMessage(formatUiError(error, tx("workflowActions.renameError")));
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
      setPreviewMessage(result.revealed ? tx("workflowActions.revealed") : tx("workflowActions.revealMissing"));
    } catch (error) {
      setPreviewMessage(formatUiError(error, tx("workflowActions.revealError")));
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
          ? tx("workflowActions.recycled")
          : result.fileAction === "recycle"
            ? tx("workflowActions.recyclePreview")
            : tx("workflowActions.notRecycled", { reason: result.reason }),
      );
      if (result.fileAction === "recycled")
        setRecordings((current) =>
          current.map((item) => (item.id === recordingId ? { ...item, missing: true } : item)),
        );
    } catch (error) {
      setPreviewMessage(formatUiError(error, tx("workflowActions.recycleError")));
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
