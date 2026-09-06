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

`RecordingQueue` adds a fixed-capacity nonblocking handoff for caller-owned
interleaved chunks. A full queue returns the chunk to its producer and
increments an overrun counter; queue operations perform no encoding or file
I/O. A fourth in-memory test covers capacity, ownership return, and counters.
The queue is not yet connected to a recorder worker.

`RecorderController` now models the REC-04 state transitions independently of
encoding: unarmed start is rejected, pause intervals are recorded by exact
frame, split closes one part and starts the next, stop finalizes the active
part, failures are explicit, and a completed recorder can be re-armed for a
new part set. Two additional in-memory tests bring the crate to six tests.
This remains control metadata only and does not claim file-worker, crash
recovery, or recorder API integration.

`WavRecorder` now joins the queue and state machine around the WAV writer. It
drains a caller-selected maximum number of contiguous chunks per worker call,
rejects non-recording drains and frame discontinuities, and finalizes the
caller-owned WAV destination only after a completed stop. A seventh in-memory
test verifies chunk draining and finalized data size. Filesystem policy,
crash recovery, FLAC, and public recorder APIs remain open.

`RecordingPathPolicy` validates an absolute local recording root, rejects UNC
roots, sanitizes reserved/invalid Windows filename components, confirms the
canonical parent remains under the approved root, and creates files with
exclusive `create_new` semantics. A temp-directory test verifies sanitization
and collision rejection. Reparse-point-specific Windows checks, allowlisted
token templates, library metadata, and recycle operations remain open.
