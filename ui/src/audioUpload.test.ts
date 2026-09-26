import { describe, expect, it } from "vitest";
import { audioUploadProblem, MAX_AUDIO_UPLOAD_BYTES } from "./audioUpload";

describe("audio upload validation", () => {
  it("accepts WAV and MP3 within the size limit and explains anything else", () => {
    const file = (name: string, size: number) => ({ name, size }) as File;
    expect(audioUploadProblem(file("room.wav", 1_000))).toBeNull();
    expect(audioUploadProblem(file("Cabinet.MP3", 1_000))).toBeNull();
    expect(audioUploadProblem(file("impulse.flac", 1_000))).toMatch(/\.wav or \.mp3/);
    expect(audioUploadProblem(file("huge.wav", MAX_AUDIO_UPLOAD_BYTES + 1))).toMatch(/64 MiB/);
  });
});
