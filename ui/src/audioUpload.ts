import type { UiBackend } from "./backend";

/** Largest WAV/MP3 accepted by the backend media store. */
export const MAX_AUDIO_UPLOAD_BYTES = 64 * 1024 * 1024;

/** Validation message for a chosen file, or null when it can be uploaded. */
export function audioUploadProblem(file: File): string | null {
  if (!/\.(wav|mp3)$/i.test(file.name)) return "Choose a .wav or .mp3 file.";
  if (file.size > MAX_AUDIO_UPLOAD_BYTES) return "Files are limited to 64 MiB.";
  return null;
}

/** Upload a WAV/MP3 to the backend media store in bounded base64 chunks. */
export async function uploadAudioMedia(backend: UiBackend, file: File) {
  const upload = await backend.beginAudioUpload(file.name, file.size);
  const bytes = new Uint8Array(await file.arrayBuffer());
  const chunkSize = Math.max(3, Math.floor(Math.min(upload.chunkBytes, 96 * 1024) / 3) * 3);
  for (let offset = 0, chunkIndex = 0; offset < bytes.length; offset += chunkSize, chunkIndex += 1) {
    const chunk = bytes.subarray(offset, Math.min(offset + chunkSize, bytes.length));
    let binary = "";
    for (let start = 0; start < chunk.length; start += 0x8000) binary += String.fromCharCode(...chunk.subarray(start, Math.min(start + 0x8000, chunk.length)));
    await backend.uploadAudioChunk(upload.uploadId, chunkIndex, btoa(binary));
  }
  return backend.finishAudioUpload(upload.uploadId);
}
