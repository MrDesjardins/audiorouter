# M04 effects and recording evidence

## 2026-09-06 â€” Worker checkpoint boundary

`WavRecorder` and `BufferedFlacRecorder` now expose validated controller
checkpoints. After each successfully encoded queue chunk, the worker advances
the checkpoint to that chunk's end frame, so recovery metadata cannot lag the
bytes already committed by the worker. The snapshot contains no samples,
encoder buffer, or file handle. Recording tests (18) and strict Clippy pass.
Durable scheduling of these snapshots and true incremental FLAC output remain
open.

## 2026-09-06 — Streaming FLAC metadata

Streaming FLAC now supports the same bounded title, artist, and comment
metadata through a Vorbis-comment block written before audio frames. A
metadata regression reads the resulting file back through the library
inspector and verifies the exact values and frame count. Recording coverage is
now 26 tests with strict Clippy. The streaming writer uses deterministic
verbatim frames; compression tuning and native realtime integration remain
open.

## 2026-09-06 â€” FLAC metadata tags

The bounded batch FLAC encoder now optionally inserts a standards-shaped
Vorbis-comment metadata block containing title, artist, and comment fields.
The existing no-metadata stream layout is preserved, and the implementation
retains the ten-minute batch limit. The buffered FLAC worker exposes the same
finalization seam. Compilation and strict Clippy pass; Windows Application
Control blocked the rebuilt test executable with OS error 4551, so runtime
tag/decoder evidence remains pending.

## 2026-09-06 â€” WAV metadata tags

WAV finalization now optionally writes bounded UTF-8 title, artist, and
comment values as RIFF `LIST/INFO` chunks. Invalid control characters and
values over 256 characters are rejected before finalization. The worker-facing
`finish_with_metadata` method keeps metadata writing off the realtime path;
the default no-metadata output remains byte-compatible. Recording coverage is
20 tests with strict Clippy. FLAC tags and durable metadata scheduling remain
open.

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
and collision rejection. File creation now also accepts only the supported
`wav` and `flac` extensions, rejecting unsupported formats before opening a
file. Root symlinks/reparse points are rejected before canonicalization; the
Windows suite covers the reparse attribute path and the portable suite covers
a Unix symlink root. Nested reparse-point-specific checks, allowlisted token
templates, library metadata, and recycle operations remain open.

`recover_wav_file` provides an in-place crash-recovery primitive for WAV
destinations. It validates the known format, truncates trailing partial sample
frames, rebuilds the RIFF and data sizes, and leaves the same file handle and
path in place. A temporary-file regression verifies recovery of a ten-byte
stereo PCM16 payload to two complete frames; journal/startup integration and
FLAC recovery remain open.

`WavRecorder` now transitions to `Failed` when a worker observes a frame
discontinuity or encoder/I/O error. Failed recorders cannot be finalized, while
their caller-owned destination remains available for the recovery or quarantine
policy. A regression verifies the terminal state and finalization refusal.

`inspect_wav_file` supplies the first recording-library metadata boundary. It
validates the canonical RIFF/WAVE header, supported format/rate/channel shape,
block alignment, and data bounds, returning exact frame, data-byte, and file-byte
counts. A temporary-file regression verifies PCM24 metadata and rejects a
truncated payload; missing-file, rename, user metadata, and recycle operations
remain open.

`audiorouter-engine` now provides `VoiceChainBlockProcessor` for the worker
boundary. It preallocates interleaved scratch for a declared channel/frame
shape, copies planar blocks into `VoiceChain`, copies results back, and rejects
shape changes without allocating during processing. Engine tests verify stereo
limiter output and mismatch handling; 36 engine tests and strict Clippy pass.
Native callback scheduling and live graph publication remain open.

`VoiceChain` now provides a reusable worker-side composition boundary for the
implemented built-ins. It prepares optional parametric EQ, gate, compressor,
delay, mandatory sample-peak limiter, and signal meter, then processes in the
declared EQ → gate → compressor → delay → limiter → meter order without
allocation. A regression verifies finite output, limiter ceiling, telemetry,
and reset. Seventeen DSP tests and strict Clippy pass; live graph publication,
parameter API wiring, and scheduler integration remain open.

`GraphicEq::magnitude_db_at` now exposes the aggregate magnitude response by
summing the ten exact band responses used during processing. This keeps the
future response curve tied to effective coefficients; regression coverage
checks flat response and a configured +18 dB high-frequency band. Sixteen DSP
tests and strict Clippy pass.

`SignalMeter` now supplies a per-channel telemetry primitive for mono/stereo
paths. It accumulates finite-safe sample peaks and RMS sums, counts sample
clipping, exposes linear and finite dB values with a documented -120 dBFS
silence floor, and resets without allocation. A regression verifies channel
separation, RMS values, clipping, non-finite repair, and silence. Sixteen DSP
tests and strict Clippy pass; configured RMS windows/peak hold, graph/API
integration, and live publication remain open.

The DSP crate now exposes fixed, explainable EQ starting points: an all-disabled
`VoiceNeutral` preset and Q8 notch presets at 50 Hz and 60 Hz. Presets return
the same bounded `BiquadParams` values used by processing and response-curve
calculation; reference tests verify target frequencies and more than 40 dB
rejection at each hum frequency. Twelve DSP tests and strict Clippy pass.

`Biquad::magnitude_db_at` now computes a control-plane magnitude response from
the exact normalized coefficients used by the audio path, avoiding a separate
UI curve model. Reference tests verify flat response, the configured peaking
gain, notch attenuation, and invalid out-of-band frequencies. Eleven DSP tests
and strict Clippy pass; ten-band/8-band preset schemas and graph integration
remain open.

`inspect_recording` wraps that metadata boundary for library listings. It
returns `Present`, `Missing`, or `Invalid` for the expected file conditions,
while propagating unrelated I/O failures. A regression confirms deleted and
malformed paths remain representable without terminating enumeration.

## FLAC batch encoding boundary

The recording crate now depends on pure-Rust `flac-io` 0.1.1 (Rust 1.74+
metadata, MIT/Apache-2.0) and exposes `FlacBufferEncoder` for completed
in-memory segments. It validates mono/stereo 44.1/48 kHz and FLAC 16/24-bit
contracts, converts finite interleaved `f32` samples to exact integer planes,
and returns a native FLAC stream. A round-trip test decodes PCM16 output and
checks sample identity. This dependency API is batch-oriented, so bounded
streaming FLAC worker integration, partial-file recovery, and metadata blocks
remain open.

## Root-scoped library index

`RecordingLibrary` is the first non-destructive library boundary. It is created
from an approved `RecordingPathPolicy`, validates each file's canonical parent,
stores session/recorder/path/status entries, refreshes status, filters by
session, and removes only an index entry. A temporary-file regression confirms
valid and missing entries, traversal rejection, and that removing an entry
leaves the recording bytes present. Durable persistence, rename/title/artist/
comment metadata, preview, and separately authorized recycle remain open.

`RecordingMetadata` now provides title, artist, and comment fields for library
entries. Updates reject control characters and values over 256 Unicode
characters before mutating the entry, and the metadata remains independent of
the underlying file and remove-entry operation. The library regression covers
valid metadata and rejection of invalid text; fourteen recording tests and
strict Clippy pass. Durable metadata/file tags and API wiring remain open.

## 2026-09-06 — Graph parameter wiring

Graph nodes now carry a serde-defaulted parameter map. Domain validation
enforces the published Gain (`gainDb`, −60..12 dB) and Mute (`muted`) contracts,
and the engine compiler prepares those values instead of hard-coding unity gain
or mute-on behavior. Invalid and unknown parameters are rejected before
preparation; the portable domain/engine suites pass with strict Clippy.

## 2026-09-06 — Recording controller checkpoints

`RecorderController` now exports and restores a versioned JSON checkpoint
containing state, part boundaries, pause intervals, and frame cursors. Restore
revalidates ordering, bounds, state/pause consistency, and checkpoint version.
The checkpoint excludes queued samples and file handles, so it is safe as a
control-plane crash-journal payload. Recording coverage is 18 tests with
strict Clippy; durable worker journal persistence remains open.

Added a cross-crate recovery regression that drives the live WAV worker's
per-committed-chunk checkpoint hook into SQLite storage. Two contiguous queued
chunks are drained, the latest durable boundary is reloaded, and frame 103 is
recovered without persisting samples or file handles. Storage coverage is now
30 tests with strict Clippy; true incremental FLAC encoding and native
realtime integration remain open.

## Initial built-in DSP slice

The new `audiorouter-dsp` crate provides an allocation-free, caller-owned
interleaved biquad processor for mono and stereo. It implements peaking,
low/high shelf, low/high pass, and notch coefficient forms, validates the M04
frequency/Q/sample-rate/channel bounds, repairs non-finite samples, and
supports state reset and coefficient updates. Four tests cover unsafe parameter
rejection, flat neutrality, finite output across all shapes, reset, and update
behavior. This is portable DSP groundwork; graph compilation, typed node
schemas/presets, dynamics, limiter, delay, and measured transfer-function
vectors remain open.

The DSP crate now also includes a stereo-linked feed-forward compressor with
the M04 threshold, ratio, attack, release, knee, and makeup bounds. Detection
uses one peak envelope for both channels so stereo balance is preserved; sample
processing is allocation-free and repairs non-finite values. Six DSP tests
cover neutral below-threshold behavior, linked reduction, contract rejection,
and finite repair. Gate, limiter, delay, graph/API integration, and reference
transfer vectors remain open.

The DSP crate now provides a conservative sample-peak `PeakLimiter` and a
bounded interleaved `DelayLine`. The limiter clamps every emitted finite sample
to its declared ceiling and explicitly makes no true-peak or lookahead claim.
The delay allocates its fixed ring at construction, bounds changes to the
declared maximum, preserves channel order, and supports reset. Ten DSP tests
cover ceiling enforcement, finite repair, delay timing, bounds, and reset.
Graph/API integration, de-clicked automation, and measured transfer vectors
remain open.

The DSP crate now includes a stereo-linked gate/downward expander. It applies
bounded threshold, hysteresis, ratio, range, attack, hold, and release
parameters, exposes its open state, and performs finite-safe interleaved
processing without allocation. Tests cover quiet-signal attenuation, linked
loud-signal opening, and hysteresis behavior. Limiter, delay, graph/API
integration, and measured transfer vectors remain open.

`ParametricEq` now turns the eight-band preset contract into a reusable
stateful processor. It constructs enabled `Biquad` state before processing,
supports per-band replacement and reset, and processes interleaved audio with
no allocation. Tests cover preset construction, active-band accounting, finite
processing, replacement, reset, and invalid band indices. Graph/API wiring and
the separate ten-band graphic EQ remain open.

`GraphicEq` now provides the required ten fixed bands at 31.5, 63, 125, 250,
500, 1k, 2k, 4k, 8k, and 16k Hz. It validates +/-18 dB gains and sample-rate
eligibility, uses the shared peaking biquad path, prebuilds all state, and
processes without allocation. Tests cover flat response, gain updates, band
bounds, and invalid gain rejection. Fourteen DSP tests and strict Clippy pass;
graph/API wiring and parameter smoothing remain open.

`Biquad::set_params_ramped` now supplies a de-clicking coefficient transition
primitive. It validates the new parameters off the audio path, interpolates
the five normalized coefficients over a caller-selected frame count, and
snaps to the target exactly without allocating during processing. A reference
test verifies target response, finite output, and bounded sample-to-sample
change. Fifteen DSP tests and strict Clippy pass; higher-level automation and
graph/API integration remain open.

`GraphicEq::set_gain_db_ramped` now applies the shared coefficient transition to
an existing graphic band, avoiding abrupt replacement while preserving the
fixed ten-band storage model. The graphic-EQ regression exercises the ramp and
checks finite output and final gain state. Fifteen DSP tests and strict Clippy
pass; graph/API integration and higher-level automation policy remain open.

The compressor now exposes its current gain-reduction meter from the same
stereo-linked envelope and reduction calculation used for output samples.
Reset clears the meter, and regression coverage distinguishes quiet
below-threshold audio from compressed audio. Fifteen DSP tests and strict
Clippy pass; broader per-channel RMS/peak telemetry and graph/API integration
remain open.

`audiorouter-storage` now persists recording-library rows in SQLite. The schema
stores session/recorder identity, path, format, channels, sample rate, frame
and byte counts, start/state/missing fields, and bounded title/artist/comment
metadata. Save/list/update/remove-entry methods preserve row-only removal; a
file-backed test reopens the database and verifies persistence and ordering.
Storage coverage is 21 tests with strict Clippy. Control/API integration,
embedded file tags, rename, preview, and recycle remain open.
FLAC library inspection is now implemented alongside the existing WAV
inspection. `inspect_flac_file` reads and validates the FLAC marker and
STREAMINFO block without decoding audio frames, and `inspect_recording` reports
valid `.flac` entries with channels, sample rate, bit depth, frame count, and
file size. The recording suite has 15 passing tests and strict Clippy passes;
streaming FLAC worker integration remains open.

The temporary sample buffer in `FlacBufferEncoder` now has a ten-minute frame
limit and returns `TooManyFrames` before extending, keeping the explicitly
batch-only path bounded. The 15-test recording suite and strict Clippy remain
green; this does not claim incremental FLAC output.

Added `BufferedFlacRecorder`, which joins the bounded recording queue and
recorder lifecycle to the batch encoder. It handles arm/start/stop, contiguous
frame validation, terminal errors, bounded accumulation, and emits a valid
FLAC stream on finish. The recording suite now has 16 passing tests with
strict Clippy clean. This is deliberately not claimed as incremental FLAC file
output; a true streaming encoder remains open.
# 2026-09-06 — Recording preview API parity

Exposed the existing non-decoding WAV/FLAC header inspector through the
`recordings.preview` API, `recordings preview` CLI command, and MCP
`preview_recording` tool. Results distinguish present, missing, and invalid
files and include format/frame metadata where available. The operation is
read-only and requires recording scope; 16 recording, 46 control, and 9 CLI
tests pass with strict Clippy.

## 2026-09-06 — Recording metadata editing parity

Added the typed `recordings set-metadata` CLI command and MCP
`set_recording_metadata` tool over the authorized `recordings.setMetadata`
method. The CLI reads the existing metadata first so omitted fields are
preserved, while the backend continues to enforce 256-character/control-byte
limits and never changes the recording path or audio content. Control and CLI
tests pass with strict Clippy.

## 2026-09-06 — Safe recording rename

Added `recordings.rename`, `recordings rename`, and MCP `rename_recording`.
The storage boundary requires an absolute WAV/FLAC destination in the same
canonical parent as the existing regular source file, refuses destination
collisions, performs the filesystem move before updating the library row, and
attempts rollback if the row update fails. The regression confirms the source
disappears, the destination exists, and the durable path is updated; 25 storage
tests, 46 control tests, and 9 CLI tests pass with strict Clippy.

Added authorized `recordings.reveal`, `recordings reveal`, and MCP
`reveal_recording`. The operation obtains the path only from the persisted
recording identity, returns an explicit missing result without spawning a
process, and on Windows launches `explorer.exe` with a separate `/select,`
argument for an existing regular file. It does not modify the file or library
row. Domain/control/CLI tests and strict Clippy pass.

Added separately authorized `recordings.recycle`, `recordings recycle`, and
MCP `recycle_recording`. Requests without `confirm: true` return a preview and
never touch the file. On Windows, confirmed requests use the OS Recycle Bin
through the `trash` library, then mark the persisted row missing; missing
files and unsupported platforms return explicit non-destructive results. No
permanent-delete fallback exists. Storage/domain/control/CLI tests and strict
Clippy pass.

The control regression now verifies that preview mode leaves a generated
recording file in place and that a confirmed request for an already-missing
file returns a non-destructive `missing` result. Control coverage is 49 tests;
the full affected-crate validation and strict Clippy remain green.

WAV library registration now reads RIFF `LIST/INFO` title, artist, and comment
tags without loading the audio data chunk. Malformed or oversized tag values
are ignored while the valid recording remains indexable. The regression proves
metadata survives writer finalization and library registration; recording
runtime coverage is 21 tests with strict Clippy.

FLAC registration now follows the same boundary: it reads only bounded Vorbis
comment blocks before the audio frames, maps title/artist/comment values, and
ignores malformed comments without rejecting the recording. Encode, parse, and
registration regression coverage passes in the 21-test recording suite.

The WAV and bounded FLAC recorder workers now expose
`drain_queue_with_checkpoint`, invoking a caller-owned persistence hook after
each contiguous chunk has advanced the validated lifecycle boundary. A hook
failure transitions the worker to `Failed` before more audio is accepted;
ordinary draining remains unchanged. The recording suite passes 23 tests with
strict Clippy and formatting checks. This is the durable scheduling seam; true
incremental FLAC encoding and native realtime integration remain open.

Implemented StreamingFlacWriter and StreamingFlacRecorder. The writer emits
bounded verbatim FLAC frames on each off-thread chunk, keeps only one bounded
frame in memory, and patches STREAMINFO frame-size and total-sample fields on
finish. The queue worker preserves contiguous-frame validation, lifecycle
states, and per-chunk checkpoint hooks. Interoperability tests decode the
resulting stereo stream and verify a three-frame worker output; recording
coverage is now 25 tests with strict Clippy. Compression tuning, metadata
insertion for this streaming path, and native realtime integration remain
open.

Added deterministic compressor transfer-curve reference vectors covering below
threshold, the soft-knee center and boundary, and the hard-knee ceiling. The
vectors exercise the same `compression_reduction` function used by processing;
the DSP suite now has 21 tests and strict Clippy/formatting pass. Dynamic
detector timing and native scheduler integration remain separate acceptance
work.

Added bounded `WindowedSignalMeter` telemetry with configurable nonzero
windows (up to ten seconds), default 300 ms RMS and 1 s peak-hold constants,
finite-sample repair, clipping counts, and allocation-free processing after
construction. Regression tests verify rolling expiry, reset, and bounds; the
DSP suite passes 24 tests with strict Clippy and formatting. Existing
allocation-free block-meter compatibility is preserved.

Integrated `WindowedSignalMeter` into `VoiceChain`; callers retain the existing
`MeterSnapshot` interface while receiving rolling RMS and peak-hold behavior
with the documented defaults. DSP library tests and strict Clippy pass. Graph
publication and native realtime scheduling remain separate acceptance work.

Added deterministic gate/expander transfer vectors for below-threshold
attenuation, threshold crossover, ratio response, range clamping, and the
open-state pass-through. The named `gate_target_gain_db` helper is the same
equation used by processing; the DSP suite now has 22 tests with strict
Clippy/formatting green. Hysteresis timing and native scheduling remain
separate acceptance work.
