# M04 effects and recording evidence

## Initial recorder slice

The new `crates/recording` crate provides a seekable WAV writer for PCM16,
PCM24, and float32 samples at 44.1 or 48 kHz, with one or two channels. It
accepts only a caller-provided `Write + Seek` destination; it does not open
paths, create files, access devices, or run a realtime callback. Non-finite
samples become zero, integer formats support optional deterministic TPDF
dithering, and `finish` patches RIFF/data sizes after all frames are written.

Three in-memory tests cover header finalization, 24-bit byte packing and
non-finite sanitization, and invalid rate/channel/sample shapes. Strict Clippy
passes. This does not yet claim REC-01 independent sinks, bounded recorder
workers, pause/split state, filesystem safety, crash recovery, or FLAC.
