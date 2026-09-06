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
