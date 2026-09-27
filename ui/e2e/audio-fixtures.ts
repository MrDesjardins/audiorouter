/** Generated PCM only; never uses a microphone or a user's media file. */
export function syntheticWav(frames = 480, channels = 1): Buffer {
  const dataBytes = frames * channels * 2;
  const wav = Buffer.alloc(44 + dataBytes);
  wav.write("RIFF", 0); wav.writeUInt32LE(36 + dataBytes, 4); wav.write("WAVEfmt ", 8);
  wav.writeUInt32LE(16, 16); wav.writeUInt16LE(1, 20); wav.writeUInt16LE(channels, 22);
  wav.writeUInt32LE(48_000, 24); wav.writeUInt32LE(48_000 * channels * 2, 28);
  wav.writeUInt16LE(channels * 2, 32); wav.writeUInt16LE(16, 34);
  wav.write("data", 36); wav.writeUInt32LE(dataBytes, 40);
  for (let frame = 0; frame < frames; frame++) {
    for (let channel = 0; channel < channels; channel++) wav.writeInt16LE(Math.round(Math.sin(frame * 2 * Math.PI * 440 / 48_000) * 4096), 44 + (frame * channels + channel) * 2);
  }
  return wav;
}
