// Checks and import of the owner's own sound file for a Soundboard slot. The picker and
// drag and drop (SND-13, SND-19) use the same functions, so the limits are the same.
import { audioUploadProblem, uploadAudioMedia } from "./audioUpload";
import type { UiBackend } from "./backend";

/** Longest user sound a slot keeps ready (owner decision 2026-10-29). */
export const MAX_USER_SOUND_SECONDS = 120;

export type ImportedSound = { mediaId: string; fileName: string; durationMs: number };

/** Whether a drag carries files (and not, say, a tool dragged from the library). */
export function isFileDrag(dataTransfer: DataTransfer | null): boolean {
  return Array.from(dataTransfer?.types ?? []).includes("Files");
}

/** The single dropped sound file, or the reason it cannot be used, checked before any upload. */
export function droppedSoundFile(dataTransfer: DataTransfer | null): { file: File } | { problem: string } {
  const files = Array.from(dataTransfer?.files ?? []);
  if (files.length === 0) return { problem: "Drop a WAV or MP3 file." };
  if (files.length > 1) return { problem: "Drop one sound at a time." };
  const problem = audioUploadProblem(files[0]);
  return problem ? { problem } : { file: files[0] };
}

/**
 * Uploads a file that passed `audioUploadProblem` and refuses one longer than a slot keeps;
 * the decoded length is known only after the upload.
 */
export async function importSoundFile(backend: UiBackend, file: File): Promise<ImportedSound> {
  const problem = audioUploadProblem(file);
  if (problem) throw new Error(problem);
  const media = await uploadAudioMedia(backend, file);
  if (media.durationMs > MAX_USER_SOUND_SECONDS * 1000)
    throw new Error(
      `${file.name} is longer than ${MAX_USER_SOUND_SECONDS} seconds, so it cannot be used on a Soundboard.`,
    );
  return { mediaId: media.mediaId, fileName: media.fileName, durationMs: media.durationMs };
}
