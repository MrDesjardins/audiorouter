# M04 effects and recording evidence

## 2026-09-11 - Explicit FLAC library handoff

`WavRecorderWorker`, `BufferedFlacRecorderWorker`, and
`StreamingFlacRecorderWorker` now accept an explicit `FileRecordingIdentity`
on the lifecycle thread. After successful sync and format inspection they
publish one bounded library row with format,
channels, sample rate, frames, file bytes, start time, and present-file state.
The default constructors remain path-unaware and therefore publish no
inferred rows. A streaming-worker regression verifies the row matches the
finalized file. Control tests (113), strict Clippy, formatting, and diff checks
passed. JSON-RPC automatic recorder construction, graph attachment, and
native endpoint ownership remain open.

The control plane now exposes `attach_recorder_worker_with_identity`, which
configures that identity before attachment and rejects workers that cannot
own one file. The WAV session-stop regression exercises this boundary; no
path or session ownership is inferred from an unconfigured worker.

The lifecycle-owned `create_file_recorder` factory now reuses
`RecordingPathPolicy` for exclusive WAV/FLAC creation, carries the requested
WAV dither setting into the writer, configures identity before attachment, and
supports `ControlPlane::create_and_attach_file_recorder`. A control regression
verifies creation, attachment, lifecycle stop, and one indexed row. No file
operation is available from the realtime callback.

The factory configuration is now represented by versioned
`FileRecorderConfig` data. It rejects unsupported versions, identity sizes,
channel/rate combinations, FLAC bit depths, and queue limits before path
creation. The factory regression confirms an invalid version creates no file.

The approved recording root is now persisted in `control_settings` and
hydrated during durable control startup. Save/load validates the existing
non-reparse local directory, and a regression proves round-trip plus failure
when the root disappears. `ControlPlane::configure_recording_root` replaces
the active policy only after durable save succeeds.

`recorders.create` is now an authoritative Record-scoped API method. It
accepts only the bounded versioned format configuration, creates under the
hydrated root, returns an unarmed recorder, and journals the result for
idempotent replay. The control regression confirms replay returns the same
path without creating a second file or arming recorder state.
If attachment fails after exclusive creation, the factory removes only that
newly created file; the regression also verifies duplicate attachment leaves
the original file as the sole destination.

The engine now provides a prebuilt bounded `AudioTapSet` and processor/
scheduler methods that notify it without constructing callback-time storage.
Control workers expose their queue tap through the worker boundary, and
`ControlPlane::recorder_tap_set` returns the prepared set for a runtime
adapter. Engine fan-out and tap-set regressions pass; this is portable graph
attachment evidence, not loaded-driver or native endpoint ownership evidence.

The domain registry and portable compiler now include an available `Recorder`
node kind. It participates in validated linear graph topology as a sink/no-op
processing stage; the runtime adapter supplies its actual queue tap through
the prepared `AudioTapSet`. Registry and compiler regressions pass. This does
not claim a loaded virtual endpoint or native device graph.

## Recorder node identity and generation binding (2026-09-11)

`RecorderTapBindings` now binds each validated recorder node ID to exactly one
prebuilt tap and runtime generation. It rejects empty or oversized IDs,
duplicate node bindings, capacity beyond eight observers, missing nodes, and
stale generations before a callback tap set is prepared. The focused engine
regression covers all rejection paths and successful preparation. This is
portable graph/runtime binding evidence; it does not claim native endpoint
ownership or loaded-driver activation.

The control plane now derives that binding from the stored session graph through
`ControlPlane::recorder_tap_bindings`. It requires exactly one enabled
`Recorder` node for the compatibility session-worker path and an attached
worker, then binds the node ID to the requested generation. A control
regression proves the prepared tap is available for the matching generation
and stale-generation preparation fails closed. Multiple nodes use the
node-worker path described below.

The control boundary now supports independent node-keyed workers through
`attach_recorder_worker_to_node`. A two-node/two-worker regression proves that
the binding builder returns two distinct taps and rejects a node absent from
the validated session. Node-keyed workers are currently a realtime attachment
boundary only; recorder lifecycle/API identity remains session-scoped.
Deletion now removes node-keyed workers for the deleted stopped session, and
the regression verifies no node-worker ownership remains after deletion.
Session stop now fails closed while a node-keyed worker is attached, because
the session-scoped lifecycle cannot yet finalize that worker safely. The
two-sink regression verifies this refusal; no recording is silently orphaned.

Node-keyed workers now also own independent `RecorderController` state and can
be driven through the control-plane `control_recorder_node` lifecycle boundary.
Arm, start, pause, resume, split, and stop use the node's worker and checkpoint
identity; successful stop persists finalized library rows when durable storage
is configured and removes only that node's ownership. The two-sink regression
arms, starts, and stops both workers independently before allowing session
shutdown. JSON-RPC lifecycle schemas and allowed-field validation now accept
an optional `nodeId`. Dispatch verifies the node belongs to the supplied
session, uses that node's controller/worker, and journals a request hash
containing both identities. The two-sink regression drives arm/start/stop
through JSON-RPC for both nodes. The create API remains session-worker based
until node-targeted creation is implemented.
`recorders.list` now includes node-targeted controller state with bounded,
deterministic ordering and an optional `nodeId`, while preserving the legacy
session entry shape. A regression confirms the node-created recorder is
discoverable before lifecycle control.
The same node-targeted create/lifecycle regression now queries durable
`recordings.list` after stop and verifies one finalized row with the expected
recorder identity and `completed` state.
The fixture now contains two node-targeted recorders with separate workers and
identities, and verifies two independent finalized library rows after both
JSON-RPC lifecycles complete. This confirms durable multi-sink metadata
ownership without claiming native graph fan-out.
Node-targeted `recorders.create` is now supported. It validates the enabled
session node before creating a file, attaches the worker and controller to that
node, and preserves exclusive-file rollback and idempotent replay. A control
regression creates, replays, arms, starts, and stops a node-targeted recorder
through JSON-RPC; the compatibility session-worker create path remains intact.

## 2026-09-09 limiter requalification

The guarded `safe-all.ps1` chain passed at pushed head `6d8e6ad2`. M04 ran 30
DSP tests and 30 recording tests, including the stateful limiter's bounded
lookahead/release regression; no audio device or persistent machine
configuration was changed.

## 2026-09-06 — Named conservative voice presets

Added `VoiceChainPresetId` and `voice_chain_preset` to the DSP boundary. The
`VoiceNeutral` preset uses flat EQ, no gate/compressor, and the documented
-1 dBFS sample-peak ceiling. `VoiceGateAndCompression` adds the specified
-45 dBFS gate and -18 dBFS, 3:1 compressor defaults. Both are preparation-only
configuration and never arm recording, enable monitoring, or mutate an active
graph. DSP construction and behavior tests plus strict Clippy pass.

The preset IDs also expose stable user-facing names and short descriptions so
clients can explain the starting point before applying it; the DSP layer does
not apply presets to a session by itself.

The control discovery document now publishes the same voice-chain catalog
(`voiceNeutral` and `voiceGateAndCompression`) with names and descriptions.
The TypeScript discovery contract mirrors this shape; discovery is read-only
and applying a preset still requires an explicit graph plan/commit.

`presets.list` and the CLI `presets list` command now provide the same catalog
as a direct read-only API result for headless clients.

The CLI regression now verifies both stable voice-chain IDs and the presence of
user-facing names/descriptions, keeping the headless catalog aligned with the
shared discovery contract.

The same catalog now includes the three built-in EQ starting points: voice
neutral, 50 Hz hum notch, and 60 Hz hum notch. Their stable IDs and explanatory
metadata are exposed through `system.describe`, `presets.list`, and the CLI;
the control and CLI regressions verify the complete set. Presets remain
preparation-only and are not applied automatically.

The checked-in `tests/acceptance/m04-dsp-recording.ps1` now packages the M04
portable gate: format check, DSP/recording tests, strict Clippy, and diff
validation. It does not open an audio device or change machine configuration.

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

StreamingFlacRecorder now connects the incremental writer to the bounded
RecordingQueue and the same lifecycle/checkpoint contract used by WAV. Its
regression persists two contiguous boundaries through the callback and
finalizes a valid FLAC stream. No native callback or audio device is involved.

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

## Bounded recording queue construction (2026-09-07)

`RecordingQueue::new` now rejects capacities above
`MAX_RECORDING_QUEUE_CHUNKS` (2,048) before constructing lock-free storage.
The existing overrun ownership behavior is unchanged for valid capacities;
regressions cover zero, over-limit, and `usize::MAX` requests. The recording
suite passed 30 tests, doc-tests, formatting, and strict Clippy. Native
realtime recorder integration remains open.

## Version retention in instantiated DSP presets (2026-09-08)

The instantiated DSP payloads now retain `version: 1` on `EqPreset` and
`VoiceChainConfig`, so version information is not lost between catalog lookup
and processor construction. DSP (27) and engine (75) tests, strict Clippy,
formatting, and diff checks passed; no audio or machine configuration changed.

Both constructors reject an unsupported preset version before allocating or
publishing processor state. The rejection regressions are covered by the DSP
suite and preserve the fail-closed processing boundary.

## Versioned built-in preset catalog (2026-09-08)

The built-in voice-chain and EQ preset descriptors now expose a required
numeric `version` field, currently `1`, through the DSP registry, `system.describe`,
and `presets.list`. The Rust control/CLI regressions and TypeScript contract
typechecks verify the field; the API reference documents it. This is metadata
only: preset discovery remains read-only and application still requires an
explicit graph plan/commit.

## Unpaged recording-list allocation bound (2026-09-07)

The legacy array response path for `recordings.list` now uses the bounded
500-record storage page query. If additional records exist, it returns an
actionable pagination error instead of loading or silently dropping the rest;
cursor-based callers retain the existing page contract. Control/storage tests
and strict Clippy cover the change without opening audio or changing files.

The unpaged `recordings.list` array schema now advertises the same 500-item
maximum as its bounded runtime path. Responses with more records require the
cursor form, preserving complete inventory semantics without unbounded
allocation or silent truncation.

Storage recording rows now enforce a shared 128-byte maximum for recording,
session, and recorder identifiers. The public recording item schema advertises
the same limit; path length remains unspecified because no owning validation
policy exists yet.

The recording path audit found no platform path-length limit enforced by the
owning storage or recording policy. Existing safety checks cover absolute
paths, approved roots, reparse-point escapes, file existence, and supported
extensions; path strings therefore remain intentionally unbounded in the API
until a documented Windows support ceiling is approved.

## Bounded recording chunk admission (2026-09-07)

`RecordingQueue::try_push` now rejects caller-owned chunks larger than 4,096
interleaved samples (2,048 stereo frames) before queue insertion. Oversized
rejections have a separate counter and return the original chunk to the
caller; queue-full overrun behavior remains distinct. Regression coverage is
included in the 30-test recording suite, which passes with strict Clippy,
formatting, and doc-tests. Native realtime recorder integration remains open.

## Stereo processor failure containment (2026-09-07)

The portable runtime now fails closed for stereo Parametric EQ, compressor,
gate, delay, and graphic-EQ stages when the right-channel state is missing or
cannot be acquired. It clears both channels rather than leaving the right side
dry after processing the left. A regression covers the missing-right-state
case; engine tests pass 64 cases with strict Clippy, formatting, and diff
checks. Native callback scheduling and hardware timing remain open.

## 2026-09-07 — Delay graph integration

The portable graph now also integrates `delay@1`, using a preallocated bounded
delay line per active channel. The configured range and finite processing
behavior are validated before graph publication; native scheduler timing and
hardware latency evidence remain open.

The portable graph also now integrates the bounded `gate@1` expander stage.
Its validated parameters are exposed through discovery and the UI library, and
prepared state is retained across blocks with fail-closed processing behavior.
This does not claim native realtime or hardware evidence.

The graph now also integrates `limiter@1`, with bounded ceiling validation and
finite sample-peak limiting on each prepared channel. This is portable graph
evidence only; true-peak/lookahead behavior and native hardware timing remain
open.

The dedicated engine regression also confirms that a prepared compressor stage
reduces sustained level over successive blocks while retaining finite output.

The portable graph now integrates `graphic-eq@1` with ten validated band-gain
parameters (`band0Db` through `band9Db`). All fixed-band filter state is built
before publication and processed per channel through the existing allocation-free
DSP implementation. This is portable graph evidence only; native callback timing
and hardware response measurements remain open.

The pitch DSP now has a prepared 128-frame streaming boundary with state
allocated at construction and caller-owned buffers, and the portable graph
exposes it as `pitch@1` using one mono state per active channel. The graph fails
closed when a block is not the declared quantum. Control-plane reset rebuilds
stream state for reconnects; measured realtime quality/latency evidence is still
required.

## 2026-09-07 — Compressor graph integration

The portable graph now includes the validated `compressor@1` node. Domain
validation enforces threshold, ratio, attack, release, and makeup bounds;
discovery advertises the processor as available; and the UI library can add a
configured compressor to a draft. The engine prepares stateful compressor
stages per active channel and fails closed if a state boundary is unavailable.
This is portable graph evidence only; native callback scheduling, stereo-linked
hardware routing, device activation, and production performance remain open.

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
enforces the published Gain (`gainDb`, −60..24 dB) and Mute (`muted`) contracts,
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

Successful recording mutations now publish session-scoped state events:
`recording.metadataChanged`, `recording.renamed`, `recording.entryRemoved`, and
`recording.recycled`. Preview and missing-file responses remain non-mutating and
do not emit events. Control coverage verifies the metadata event while storage
and file-safety behavior remain unchanged; native realtime integration remains
open.

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
to its declared ceiling and makes no true-peak claim. It now preallocates
per-channel lookahead storage and applies bounded exponential release.
The delay allocates its fixed ring at construction, bounds changes to the
declared maximum, preserves channel order, and supports reset. Thirty DSP tests
cover ceiling enforcement, finite repair, delay timing, bounds, and reset.
Graph/API integration for the limiter and delay, de-clicked automation, and
measured transfer vectors remain open.

The DSP crate now includes a stereo-linked gate/downward expander. It applies
bounded threshold, hysteresis, ratio, range, attack, hold, and release
parameters, exposes its open state, and performs finite-safe interleaved
processing without allocation. Tests cover quiet-signal attenuation, linked
loud-signal opening, and hysteresis behavior. Graph/API integration for the
remaining dynamics controls and measured transfer vectors remain open.

`ParametricEq` now turns the eight-band preset contract into a reusable
stateful processor. It constructs enabled `Biquad` state before processing,
supports per-band replacement and reset, and processes interleaved audio with
no allocation. Tests cover preset construction, active-band accounting, finite
processing, replacement, reset, and invalid band indices. Graph/API wiring
remains open; the separate ten-band graphic EQ is covered below.

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
coverage is now 25 tests with strict Clippy. Compression tuning and native
realtime integration remain open.

The streaming writer also accepts the bounded title, artist, and comment
metadata contract before writing the FLAC stream. A regression writes a
streaming file with all three tags, reopens it through the bounded metadata
reader, and verifies the tags and frame count. This is file-format evidence;
durable library/API mutation and native realtime integration remain separate.

The DSP compressor remains a tested portable component, but it is not yet
advertised as a graph node. Its detector state cannot safely be inserted into
the current immutable published `Arc<RuntimeGraph>` without a scheduler-owned
mutable state boundary. The attempted graph integration was rejected during
compile validation rather than weakening state continuity or introducing
unsafe aliasing; this remains an explicit engine/scheduler task.

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

## 2026-09-06 — Incremental FLAC recovery

## 2026-09-06 — Checkpoint flush ordering

Both the WAV and incremental-FLAC queue workers now flush their underlying
`Write` destination immediately after encoding each chunk and before invoking
the caller's durable checkpoint hook. A flush failure transitions the worker to
`Failed`, so a checkpoint cannot report a boundary that remains buffered in the
writer. The recording suite passes 27 tests with strict Clippy; OS-level
`sync_all` policy and native realtime integration remain separate work.

The regression suite includes a destination whose `flush` operation fails. The
WAV worker returns the I/O error, enters terminal `Failed` state, and does not
invoke the checkpoint hook; this prevents persistence from claiming a boundary
after a failed writer flush. Recording coverage is now 28 tests with strict
Clippy.

Added recover_streaming_flac_file for conservative crash recovery of the
incremental writer's verbatim layout. It validates the FLAC metadata chain,
scans complete bounded frames and CRCs without decoding the audio payload,
truncates an incomplete tail, and patches STREAMINFO with complete-frame
counts and size bounds. A temporary-file regression recovers three frames
after appended partial bytes; recording coverage is now 27 tests with strict
Clippy. Recovery of arbitrary third-party FLAC subframes remains outside this
specialized path.

Added a recording-library refresh regression for incremental FLAC files. When
an indexed FLAC is externally replaced, refresh now reloads its bounded Vorbis
title metadata rather than retaining stale values; the file path and safety
policy remain unchanged. Recording coverage is now 30 tests with strict
Clippy; native realtime integration remains separate work.

The M04 acceptance wrapper was requalified at clean revision `e34438f`.

At the current head, the checked-in wrapper passed 25 DSP tests and 30 recording
tests, including the 60-second pitch-duration extremes. The process-scoped
PowerShell execution-policy bypass was not persisted; no audio device or machine
configuration was accessed.

## Recording-row metadata bound (2026-09-08)

`Storage::save_recording` now validates title, artist, and comment values at
the persistence boundary: each is at most 256 Unicode characters and contains
no control characters. Direct-storage regression coverage rejects both forms
of invalid metadata. Storage coverage passes 42 tests with strict Clippy,
formatting, and diff checks; no audio endpoint or machine configuration was
accessed.
All 25 DSP tests and 30 recording tests passed, including the dedicated
sixty-second pitch-duration regression, incremental WAV/FLAC recovery, and
metadata/path safeguards. Formatting, strict Clippy, and diff checks passed.
The wrapper used temporary test state only and did not access audio devices or
change machine configuration.

## 2026-09-06 — Recording mutation idempotency

Recording metadata, rename, library-entry removal, and confirmed recycle
mutations now accept an optional idempotency key. When supplied, the control
plane scopes the key to the authenticated client and method, hashes the
normalized request, journals the JSON outcome for the configured retention
window, and replays it after a control restart. Reusing a key with a different
payload returns `idempotencyConflict`. Preview and missing-file outcomes do not
create mutation journal entries. The control suite (71 tests), full locked
workspace, formatting, and strict Clippy pass; no audio endpoint or machine
configuration was accessed.
## 2026-09-06 — Evidence reconciliation

The earlier checkpoint and FLAC paragraphs above are historical snapshots and
are superseded by the later implementation evidence in this file and the
active plan. Durable SQLite checkpoint persistence is now wired into the
control-plane recorder lifecycle, and the WAV/incremental-FLAC workers expose
flush-ordered checkpoint hooks with recovery coverage. The incremental FLAC
writer/recorder and bounded Vorbis metadata path are also implemented and
tested. Remaining M04 limitations are native realtime graph integration,
hardware timing, and production-scale compression/performance evaluation;
portable tests do not claim those gates.
## 2026-09-06 — Current-tip portable acceptance

The complete M04 acceptance was rerun after the recovery changes. All 25 DSP
tests, including the 60-second pitch-duration extremes, and all 30 recording
tests passed, along with formatting and strict Clippy. The suite used only
portable processing and temporary file-boundary state; it did not open an
audio device or change machine configuration.
## Current-tip portable qualification (2026-09-06)

The M04 acceptance wrapper passed all 25 DSP tests and 30 recording tests,
including the long-duration pitch boundary cases, with formatting and strict
Clippy. This validates portable processing and file-boundary behavior only;
native realtime integration remains open.

## Current-tip portable qualification (2026-09-07)

M04 was rerun at the current head: all 25 DSP tests and 30 recording tests passed,
including 60-second pitch-duration and recovery coverage, with formatting and strict
Clippy. This remains portable/file-boundary evidence; native realtime integration and
hardware timing remain open.

## 2026-09-07 — Parametric EQ graph integration

The portable graph now includes the validated `parametricEq@1` node. Domain
validation enforces its frequency, Q, and gain bounds; discovery advertises the
node and processor as available; the UI library can add it to a draft; and the
engine prepares a stateful peaking-EQ stage for each active channel. Processing
uses prebuilt state and a nonblocking state boundary, sanitizing or silencing on
failure. Engine regression coverage verifies finite transformed output and
stateful preparation. This is portable graph evidence only; native callback
scheduling, device activation, hardware timing, and production performance
remain open.

## Recorder checkpoint collection bounds (2026-09-07)

Recorder checkpoint restore now rejects more than 4,096 parts or 4,096 pause
intervals before validating or adopting lifecycle metadata. This protects both
the public restore API and JSON recovery path from oversized lifecycle
collections while preserving the existing ordering and frame-consistency
checks. The recording suite passes 30 tests, doc-tests, formatting, and strict
Clippy. This is portable recovery evidence; native realtime recorder
integration remains open.

## Recorder checkpoint JSON input bound (2026-09-07)

`RecorderController::restore_json` now rejects documents larger than 1 MiB
before invoking JSON deserialization. Together with the 4,096-part and
4,096-pause collection limits, this bounds the recovery input at the recorder
boundary while retaining existing checkpoint consistency checks. Recording
coverage remains 30 tests plus doc-tests, formatting, and strict Clippy. This
is portable recovery evidence; native realtime recorder integration remains
open.
## Recovery schema bounds (2026-09-07)

The public `recordings.recovery` schema now advertises the recorder's existing
4,096-part and 4,096-pause restore limits, sourced from the same constants
used by validation. Recording tests (30), control discovery tests (86), and
strict Clippy passed; native realtime recorder integration remains open.

## Recording checkpoint identity bound (2026-09-08)

Storage checkpoint save, load, and clear now reject empty or over-128-byte
recording IDs using the same identity ceiling applied to persisted recording
rows and API schemas. This closes the direct-storage bypass without changing
checkpoint contents or file behavior. Storage coverage passes 41 tests with
strict Clippy, formatting, and diff checks; no audio endpoint or machine
configuration was accessed.

## Direct session persistence validation (2026-09-08)

`Storage::save_session` and its atomic journal variant now revalidate the
domain session and enforce the existing 1 MiB serialized-document ceiling
before opening a transaction. Direct callers can no longer persist an invalid
session while bypassing the control layer. Storage coverage passes 44 tests
with strict Clippy and formatting; no audio endpoint or machine configuration
was accessed.

Graph-plan candidate persistence uses the same validated session serializer,
closing the equivalent direct-storage bypass for uncommitted candidates. The
regression suite passes 45 storage tests; native driver and endpoint gates
remain unaffected.

Recording-library lookup, paging, metadata, rename, missing-state, and removal
operations now enforce the same 128-byte recording identity bound as checkpoint
and row writes. Direct-storage regression coverage passes 50 tests with strict
Clippy, formatting, and diff checks; no recording file or audio endpoint was
accessed.

Paged recording-list discovery now advertises its 500-item page bound, matching
the control dispatcher and storage page request limit.

## Recorder transition schema bounds (2026-09-07)

The `recorders.arm`, `recorders.start`, `recorders.pause`, `recorders.resume`,
`recorders.split`, and `recorders.stop` response schemas now advertise the
same 4,096-part and 4,096-pause bounds used by recorder checkpoint validation.
Control discovery regression coverage and the 86 control plus 30 recording
tests passed, with no audio stream or machine configuration changes. Native
realtime recorder integration remains open.
## Recording counter-boundary validation (2026-09-08)

SQLite recording hydration now converts signed integer counters to unsigned
values only after rejecting negative values. Write validation also rejects
frame and file-byte values above SQLite's signed integer range before any row
mutation. Regressions cover a corrupt negative row and an oversized write;
recording (10), storage (62), and control (87) tests plus strict Clippy,
formatting, and diff checks passed. No recording file, audio endpoint, or
machine configuration was accessed.

## M04 DSP/recording acceptance requalification (2026-09-08)

`tests/acceptance/m04-dsp-recording.ps1` was requalified against the current
tree. The DSP suite passed 27 tests, including deterministic transfer curves,
finite-output repair, bounded state, and long-duration pitch behavior. The
recording suite passed 30 tests, including WAV/FLAC round trips, bounded queues,
metadata, checkpoint/recovery, worker failure, pause/split boundaries, and
library inspection. Formatting, strict package Clippy, and `git diff --check`
also passed. This is portable DSP/recording evidence; native realtime recorder
integration, endpoint ownership, and W1 hardware timing remain open.

## Delay storage bound hardening (2026-09-08)

`DelayLine::new` now rejects sample rates outside the supported 1–192 kHz
contract before converting the bounded delay size to storage units. This keeps
otherwise valid-looking finite inputs from requesting impractically large
allocations. The regression for an oversized sample rate passed; the DSP suite
passed 28 tests and strict package Clippy completed. This is portable safety
evidence only; it does not change native endpoint configuration.

## Processor catalog stale-availability cleanup (2026-09-08)

Removed an unused control-plane availability value that still referenced the
obsolete `requires M04 graph integration` state after built-in processors had
become available. The UI presentation fixture now uses an explicitly synthetic
unavailable reason, preventing stale milestone text from being mistaken for a
runtime capability report. Control tests (87), strict Clippy, the focused UI
processor-catalog suite (4), TypeScript typecheck, and diff checks passed.

## Direct recording-list pagination hardening (2026-09-08)

The storage boundary now shares the 500-record authority with control, probes
one extra row, and rejects an oversized unpaged result with an explicit cursor
pagination error. Single-record lookup uses a direct identity query and remains
available for larger libraries. A 501-record regression passed alongside 68
storage tests, 87 control tests, strict Clippy, formatting, and diff checks.
No recording files, audio endpoints, or machine configuration were accessed.
## 2026-09-09 — Eight-band parametric EQ contract

The portable graph now exposes all eight fixed-capacity parametric-EQ bands.
Each band supports explicit enable state, peaking/low-shelf/high-shelf/
low-pass/high-pass/notch selection, frequency, Q, and gain. The prior
`frequencyHz`, `q`, and `gainDb` fields remain accepted as band-0 compatibility
aliases. Domain validation, control discovery, and engine compilation share the
same bounded ranges; enabled bands are prepared before publication and disabled
bands allocate no active filter state. The full domain/engine/control suites,
formatting, and strict Clippy pass. This is portable DSP evidence; frequency
response reference vectors, UI curve parity, native callback timing, and
hardware/release gates remain open.

The DSP crate now exposes the combined response of all enabled parametric bands
through the same coefficient path used for audio processing. A deterministic
response vector covers peaking, low/high shelf, low/high pass, and notch shapes;
29 DSP tests, doc-tests, formatting, and strict Clippy pass. This provides the
portable response-vector evidence, but no UI curve transport or native timing
claim is made.

## Segmented WAV worker (2026-09-11)

`SegmentedWavRecorder` now provides a worker-side file boundary for REC-06.
It rotates caller-owned WAV destinations at a bounded frame threshold, accepts
an explicit manual split boundary, finalizes each prior segment before opening
the next, and processes a chunk crossing a boundary in exact frame slices.
The regression creates one six-frame mono chunk, requests a split at frame 2,
uses a two-frame automatic threshold, and verifies three finalized files with
two frames each and no duplicated/lost payload. Recording tests (35), strict
Clippy, formatting, and diff checks pass. This is portable file-worker
evidence; durable path allocation, UI/API automatic size/time configuration,
realtime graph attachment, and native endpoint ownership remain open.

## Segmented path-policy integration (2026-09-11)

The segmented worker regression now opens the initial and rotated WAV files
through `RecordingPathPolicy`, verifying sanitized names, root containment, and
exclusive creation across all three segments. The test also caught and fixed a
Windows compatibility defect where canonical local paths in the `\\?\C:\...`
extended form were incorrectly rejected as network roots; extended UNC roots
remain rejected. Recording tests (36), strict Clippy, formatting, and diff
checks pass. This closes the portable path-allocation slice, while durable
session/API integration, UI size/time controls, realtime graph attachment,
and native endpoint ownership remain open.

## Recorder timeline failure containment (2026-09-11)

WAV, buffered FLAC, and streaming FLAC queue workers now mark the recorder
`Failed` when frame-counter arithmetic or controller advancement fails, in
addition to existing write/flush/checkpoint failures. This prevents an
invalid timeline from remaining active after a bounded worker error. A
terminal arithmetic-failure regression passes; recording tests (39), strict
Clippy, formatting, and diff checks pass. Partial-file recovery presentation
and native graph attachment remain open.

## Segmented worker failure containment (2026-09-11)

Segmented WAV queue draining now marks the recorder `Failed` for rotation,
write, flush, arithmetic, controller, or checkpoint errors before returning
the error. This aligns segmented behavior with the existing single-file
workers and preserves the written prefix for recovery handling. A flush-failure
regression verifies the terminal failed state; recording tests (38), strict
Clippy, formatting, and diff checks pass. Disk-recovery listing and native
graph attachment remain open.

## Global recorder capacity (2026-09-11)

The control plane now enforces the REC-01 maximum of eight simultaneously
armed/active recorder controllers and advertises the same bound through
`system.describe`. A ninth arm request is rejected before changing recorder
state; completed/inactive controllers do not consume the active budget. The
regression covers eight accepted sessions and one rejected request. Control
tests (111), strict Clippy, formatting, and diff checks pass. Independent
recorder-node graph attachment and native resource qualification remain open.

## API stop finalization (2026-09-11)

`recorders.stop` now finalizes an attached worker before completing the
authoritative recorder state. A failed or incomplete finalization prevents
the state transition; a successful worker is removed after its file is
durably synced. The regression drives arm/start/stop through JSON-RPC,
verifies a two-frame WAV on disk, and confirms the worker is consumed. Control
tests (110), strict Clippy, formatting, and diff checks pass. Multi-recorder
durable library rows, automatic thresholds, graph attachment, and native
endpoint ownership remain open.

## Recorder lifecycle forwarding (2026-09-11)

The control `RecorderWorker` boundary now exposes lifecycle hooks for arm,
start, pause, resume, and split. Attached workers receive these operations
before the authoritative controller is mutated; unsupported worker splitting
fails closed. A dispatch regression verifies arm/start/split forwarding and
the resulting frame checkpoint. Control tests (109), strict Clippy,
formatting, and diff checks pass. Worker stop/finalization remains a separate
bounded operation, and durable recording rows, automatic threshold
configuration, graph attachment, and native endpoint ownership remain open.

## RIFF capacity guard (2026-09-11)

The WAV writer now rejects a write whose data payload would exceed the RIFF
32-bit size boundary before writing samples. `SegmentedWavRecorder` also
rejects a configured segment threshold that cannot fit for the selected format
and channel count. A regression covers an oversized threshold; recording tests
(37), control tests (108), strict Clippy, formatting, and diff checks pass.
This prevents an invalid threshold from being accepted, but does not change
the separate JSON-RPC configuration and durable library integration gates.

## Control-plane segmented worker (2026-09-11)

`SegmentedWavRecorderWorker` now connects the REC-06 segmented file worker to
the control-side recorder boundary. It allocates the initial and subsequent
files through `RecordingPathPolicy`, exposes the existing pooled audio tap,
uses bounded finalization passes, syncs every finalized segment, and reports a
completed outcome only after all segments are durable. The control regression
produced three two-frame WAV files from one six-frame queue item. Control tests
(108), strict Clippy, formatting, and diff checks pass. JSON-RPC configuration
for segment limits, durable recording-library rows, realtime graph attachment,
and native endpoint ownership remain open.

## Bounded recorder tap fan-out (2026-09-11)

The engine now exposes `process_with_taps` and
`process_once_with_taps`, allowing up to eight independent recorder/observer
sinks to receive the same processed quantum. Fan-out is performed over
borrowed observers after processing and retains the allocation-free,
nonblocking callback contract. Engine tests (97), strict Clippy, and
formatting pass. Native endpoint-owned graph attachment remains open.

The control recorder capacity now consumes the engine's shared tap bound, and
a scheduler regression verifies all eight sinks receive one processed
quantum. This is portable attachment plumbing only; it does not claim native
endpoint ownership or live-driver qualification.

## Bounded recovery discovery (2026-09-11)

`recordings.recovery` now supports a bounded cursor/limit listing of persisted
checkpoint IDs in addition to its existing single-ID lookup. Each entry is
reported as `available`, `missing`, or `invalid`; corrupt checkpoint JSON does
not hide other entries. Storage tests (81), control tests (112), strict
Clippy, formatting, and documentation validation pass. This is durable
checkpoint discovery only and does not claim native crash or power-loss
recovery guarantees.

## Recovery checkpoint contract alignment (2026-09-11)

Single-ID and paginated `recordings.recovery` responses now share one bounded
checkpoint schema. The schema describes part and pause entries, uses the actual
serialized snake_case checkpoint field names, and includes the bounded
`stop_frame` recovery boundary. Control coverage passed 112 tests and strict
Clippy; no audio endpoint or machine configuration was accessed.

## REC-06 default segment boundary (2026-09-11)

The recording layer now exposes the canonical default WAV segment boundary:
the earlier of a 2 GiB RIFF-safe payload budget and 24 hours at the negotiated
sample rate. `SegmentedWavRecorder::new_with_default_segment_frames` and the
control worker companion use the shared calculation, while explicit limits
remain available for deliberate configuration and tests. Recording tests (40),
control tests (112), strict Clippy, formatting, and documentation validation
pass. JSON-RPC recorder configuration and native graph attachment remain open.

The shared TypeScript contracts and UI backend adapter now represent the
single-item and paged recovery response shapes separately. UI tests (123),
typecheck, and a disposable production build pass; no file or audio-device
operation is performed by the adapter.

## Finalized segmented library rows (2026-09-11)

Policy-owned segmented WAV workers now return bounded finalized-file metadata
on the lifecycle thread. `recorders.stop` and session-stop finalization persist
one library row per completed segment after all outputs are synced; the realtime audio tap remains
independent of storage. A storage-backed JSON-RPC regression verified the
completed row, frame count, path, and present-file state. Control coverage
passed 113 tests with strict Clippy, formatting, and diff checks. FLAC/simple
file-worker metadata handoff, automatic recorder configuration, realtime graph
attachment, and native endpoint ownership remain open.

Segment identities include a per-start run identifier so a later recording
cannot replace an earlier segment row through SQLite upsert. The focused
regression verifies the bounded identity shape and remains green.
