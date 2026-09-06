# M06 VST3 SDK boundary

## Selected dependency

AudioRouter uses the official Steinberg `vst3sdk` repository as the M06
hosting boundary. On 2026-09-05 it was downloaded with all submodules into the
ignored local path `third_party/vst3sdk` at commit
`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`. The checkout includes
`pluginterfaces`, `public.sdk`, and the official helper/tutorial submodules.

`third_party/vst3sdk/LICENSE.txt` identifies the checkout as MIT licensed. The
boundary is VST3 x64 only; VST2, x86 bridging, instruments/MIDI, and arbitrary
plugin scripts remain unsupported by the product specification. No plugin
binaries were downloaded or installed.

## Verification

The repository checkout and every declared submodule resolved successfully.
The installed Visual Studio 18 Community tree contains MSVC 14.51 and MSBuild,
but no `cmake.exe` was found on PATH or inside that installation. Therefore the
SDK CMake configure/example build is explicitly pending CMake installation;
this is a toolchain gap, not evidence that the SDK or plugins are compatible.
No audio endpoint, driver, default, volume, mute, or persistent machine audio
setting was changed.

## Portable inspection boundary

`crates/plugin-host` now provides non-executing binary inspection for the
future disposable worker: configured-root containment after canonicalization,
VST3/VST2 extension classification, bounded file size, PE signature and x64
architecture checks, SHA-256 identity fingerprints, and a three-failure
quarantine ledger requiring deliberate retry. Three crate tests and strict
Clippy pass. Loading, scanning code execution, worker IPC, and plugin state
remain intentionally separate follow-up work.

The same crate now defines a bounded `WorkerFrame` and `WorkerFrameGuard`:
only mono/stereo finite frames up to 2048 frames are accepted, sample shape is
checked, sequence regressions are rejected, and expired deadlines are
reported. Four plugin-host tests and strict Clippy pass. This validates the
message boundary without claiming shared-memory transport or plugin execution.

`scan_directory` now performs a bounded, explicit-root enumeration of VST3/DLL
candidates and returns invalid entries visibly alongside valid identities. It
does not recurse or execute binaries and rejects more than 256 candidates.
Five plugin-host tests and strict Clippy pass.

The scanner now accepts a shared `ScanControl` with an explicit deadline and
atomic cancellation flag. Both controls are checked at the root boundary,
while enumerating candidates, and before inspection; an empty root therefore
honors cancellation and expiry too. Six plugin-host tests and strict Clippy
pass.

The candidate-budget regression creates 257 temporary VST3-shaped entries and
confirms scanning returns `TooManyCandidates` before binary inspection. Seven
plugin-host tests and strict Clippy pass.

The post-integration `cargo test --workspace` run passes across CLI (5),
control (41), domain (23), DSP (17), engine (36), plugin-host (3), protocol
(5), recording (14), storage (21), transport (14), and Windows-audio (8)
tests, plus all doc tests. Strict plugin-host Clippy remains green.

`WorkerFailurePolicy` now makes the protected-path rule executable: worker
failure selects silence for protected microphone paths, while dry fallback is
available only when the path is explicitly unprotected. Eight plugin-host
tests and strict Clippy pass.

`PluginStateAsset` now bounds opaque state at 16 MiB, records a schema version
and SHA-256, and verifies both before restore. Empty, oversized, mismatched,
and tampered state are rejected. Nine plugin-host tests and strict Clippy pass;
durable asset storage and plugin-specific state serialization remain open.

The SQLite storage boundary now persists validated `PluginStateRecord` metadata
(plugin identity/hash, state version/path/hash, and size), filters by plugin,
and removes only the metadata row without touching the asset path. Twenty-two
storage tests pass, including reopen/persistence coverage, and strict Clippy is
green.

The durable plugin-state API now rejects relative asset paths and requires an
absolute path before persistence. The storage regression verifies the rejection
without creating files; storage tests (23) and strict Clippy pass.

`FailureLedger` now expires its rolling failure count after ten minutes, with
an injectable clock for deterministic tests. The new test compiles and strict
Clippy passes, but Windows Application Control blocked launching the generated
test executable twice with OS error 4551 before test execution. The prior nine
plugin-host tests passed before this policy block; this new runtime regression
remains pending execution-policy remediation.

`WorkerSupervisor` now models launch eligibility (VST3/x64 only), heartbeat
refresh, timeout-to-failure, quarantine integration, and deliberate retry
reset. The new lifecycle test compiles and strict Clippy passes; runtime test
launch remains blocked by Windows Application Control OS error 4551.

The scanner no longer labels arbitrary `.dll` files as VST2. DLL candidates
are retained with `PluginFormat::Unknown`, while only the `.vst3` extension
receives VST3 classification; this prevents false ReaPlugs/legacy support
claims. Nine plugin-host tests and strict Clippy pass.

`BoundedFrameQueue` now preallocates a fixed-capacity FIFO for worker frames,
returns the rejected frame to its caller on overflow, and exposes an overflow
counter without waiting. The queue test compiles and strict Clippy passes;
runtime execution remains blocked by OS error 4551.

`write_state_asset`/`read_state_asset` now provide the bounded asset-file
boundary: safe IDs, approved-root canonical containment, exclusive creation,
flush-to-disk, size limits, and version/hash verification on read. The new
asset-file test compiles and strict Clippy passes; plugin-host runtime launch
remains blocked by Windows Application Control OS error 4551.

`PluginIdentity::compatibility` now returns an explicit capability result:
only an inspected VST3/x64 identity is supported, while unknown/legacy
formats remain unsupported even if their PE structure is valid. The new test
compiles and strict Clippy passes; runtime launch remains blocked by OS error
4551.

## Portable CMake and SDK build correction (2026-09-06)

The official portable CMake 4.4.0 Windows x64 ZIP was extracted into the
ignored `third_party/cmake-4.4.0` cache. It configured the SDK with the
`Visual Studio 18 2026` x64 generator, MSVC 19.51.36256.0, and Windows SDK
10.0.28000.0. The Release build completed successfully after reconfiguration;
the SDK validator self-test reported 51 tests passed and the built VST3
validator reported 94 tests passed.

The build emitted upstream VSTGUI deprecation/narrowing warnings and noted
optional EXPAT/LIBJACK/AAX components were unavailable or embedded; these did
not fail the build. The SDK examples and validator binaries are local build
artifacts only and were not installed as system plugins. No audio endpoint,
driver, default, volume, mute, or persistent machine audio setting was
changed.

The inspector now resolves VST3 bundle directories to their single
`Contents/x86_64-win` binary, while preserving both bundle and binary paths in
identity. The bundle-layout regression compiles and strict Clippy passes; the
plugin-host test executable remains blocked by Windows Application Control OS
error 4551.

`ParameterEvent` and `BoundedParameterQueue` now provide bounded worker-side
automation: normalized values must be finite and within 0..1, sample offsets
must fit the 2048-frame block, and queue overflow returns ownership with a
counter. The new test compiles and strict Clippy passes; runtime plugin-host
execution remains blocked by OS error 4551.

The locally built `mda-vst3.vst3` sample bundle was also run through the
official SDK validator. It completed with exit code 0 and reported 1,598 tests
passed and 0 tests failed. The sample was validated in its build directory
only; it was not installed as a system plugin and no audio endpoint or machine
audio configuration was touched.

The state-file restore API now requires the durable expected SHA-256 from the
metadata record instead of comparing a freshly computed digest with itself.
Wrong-hash and post-write tamper regressions pass. The plugin-host suite has
16 passing tests, and strict Clippy passes.

State-file writes now validate the asset's own versioned digest before creating
the file, rejecting caller-side mutation of the public asset fields. The
corruption-on-write regression passes; the plugin-host suite remains at 16
passing tests with strict Clippy clean.

Added a bounded framed worker-control protocol with Hello/Ready, Process,
Processed, Shutdown, and Failure messages. Decode validates frame shape,
channel/sample bounds, and parameter bounds before a future worker can accept
audio. Round-trip, malformed, and oversized-frame regressions pass; the suite
now has 18 passing tests and strict Clippy is clean. This is protocol evidence
only; no worker process or plugin execution is claimed.

Worker `Hello` negotiation now requires protocol version 1, a 64-character
hexadecimal plugin fingerprint, and mono/stereo channel count; empty failure
codes are rejected. Capability-negotiation regressions pass, bringing the
plugin-host suite to 19 tests with strict Clippy clean.

Added a stateful `WorkerSession` gate requiring the expected plugin identity
and channel negotiation before `Ready`, then enforcing monotonic, non-expired
process frames until orderly shutdown or failure. The handshake, identity,
and sequence regression passes; the plugin-host suite now has 20 passing tests
with strict Clippy clean. This remains a portable protocol/lifecycle proof;
native process creation and OS isolation are still open.

Added validated `WorkerLatency` reports and a `Latency` control message. The
boundary accepts only 8--192 kHz rates and at most ten seconds of declared
latency, exposes sample-to-millisecond conversion, and permits dynamic reports
while active. Round-trip and bound regressions pass; the plugin-host suite now
has 21 passing tests with strict Clippy clean. These are declared worker
values, not measured native plugin latency.

For built-in pitch, `pitch_shift` 2.1.0 was resolved from crates.io and
identified as MIT-licensed, with repository provenance recorded in its package
metadata. `PitchShifter` uses its documented phase-vocoder block API, exposes
the required semitone/cent bounds, preserves offline frame count, reports a
fixed 1,024-frame warmup/latency boundary, sanitizes non-finite samples, and
supports bypass. Deterministic tests pass for +12 semitones, exact duration,
range rejection, and bypass. This implementation is currently an offline
allocation API; realtime streaming integration and 60-second/voice quality
acceptance measurements remain open.

The dedicated 60-second duration check has now executed for both -12 and +12
semitones: the combined test completed in 34.6 seconds, and each run preserved
the exact 2,880,000-frame length with finite output. This proves the offline
duration invariant but does not replace realtime streaming, speech listening,
or native W2 latency measurements.

Added exact-stream worker I/O helpers. `read_worker_message` validates the
declared payload size before allocation and uses `read_exact` for fragmented
pipe reads; `write_worker_message` writes one encoded frame and flushes the
control stream. A one-byte chunked-reader regression passes, bringing the
plugin-host suite to 22 tests with strict Clippy clean. This is transport
evidence only; native process creation and shared-memory audio transport remain
open.

Added `audiorouter-plugin-worker`, a disposable process protocol fixture. It
negotiates the plugin fingerprint and channel count, accepts `Ready`, validates
and echoes framed process buffers, echoes latency reports, and exits on
`Shutdown`; it does not load plugin code or open audio devices. A process-level
integration test passes alongside the 22 library tests and strict Clippy.
This establishes stdio process IPC only; native plugin loading, sandbox policy,
and shared-memory audio transport remain open.

Added the reusable `WorkerProcess` control-plane client. It validates the
configured hash/channel identity during Hello/Ready negotiation, rejects frame
channel or sequence mismatches, forwards bounded latency reports, supports
graceful shutdown, and kills an unfinished child on drop. The process
integration test now exercises this client against the disposable fixture;
the 22 library tests, process test, and strict Clippy pass. This is still
stdio IPC with an echo fixture: plugin loading, OS sandbox policy, and actual
shared-memory audio transport are not claimed.

Added `SharedAudioLayout`, a fixed-size versioned slot contract for future
shared memory. It bounds slots to the supported mono/stereo frame capacity,
stores sequence/deadline/channel/frame metadata, encodes samples in
little-endian form, and rejects bad magic/version/counts, non-finite samples,
and malformed frames. The round-trip/corruption regression passes; the
plugin-host suite now has 23 tests with strict Clippy clean. OS mapping,
cross-process synchronization, and plugin execution remain open.

Added `SharedAudioRegion`, a file-backed memory mapping over the fixed slot.
Creation requires an absolute caller-selected path and refuses an existing
target; reopening validates the minimum mapping size. Explicit write, read,
and flush operations support a future worker pair without machine-wide names
or audio access. The reopen/round-trip and relative-path regression passes;
the plugin-host suite now has 24 tests with strict Clippy clean. A complete
cross-process ownership/synchronization protocol and plugin execution remain
open.

The mapped region now carries an acquire/release epoch state word. Writers
reserve an even slot as odd while copying, publish the next even epoch only
after the frame is complete, and restore the prior state on validation error.
Readers reject empty or busy slots and compare the epoch after decoding to
detect a concurrent overwrite. This establishes single-writer/torn-read
semantics for the mapping; bounded worker queue integration, OS security
policy, and plugin execution remain open.

Writers now also compare the incoming frame sequence with the published slot
sequence and reject regressions before replacing a valid frame. The state is
restored after rejection, so a stale producer cannot silently displace newer
audio data. The regression and full 24-test plugin-host suite pass with strict
Clippy clean.

Added caller-owned `read_into` decoding for both the raw layout and mapped
region. It validates the version, channels, frame bounds, and finite samples,
fills preallocated sample storage, returns sequence/deadline metadata, and
retains the acquire/release epoch comparison for mapped reads. The regression,
24 library tests, worker integration, and strict Clippy pass. This removes
sample-vector allocation from the transport read boundary but does not claim
realtime scheduler integration or plugin execution.

Added SharedAudioTransport, a paired caller-owned input/output mapping that
lets a host and worker open the same two explicit slot files with opposite
directions. The exchange regression verifies metadata and caller-owned sample
buffers across both endpoints, and rejects aliased paths. All plugin-host test
targets compile and strict Clippy passes; generated test execution remains
blocked by Windows Application Control OS error 4551. This is shared-memory
transport wiring, not plugin loading or native realtime scheduling.

Hardened duplex transport construction so a failed output-slot creation removes
the newly created input slot after its mapping is dropped. This prevents
partially initialized IPC resources from surviving a failed setup; the
regression verifies the cleanup path. Formatting, all-target compilation, and
strict Clippy pass.

Extended the worker protocol with validated shared-frame metadata messages
(ProcessShared/ProcessedShared). WorkerProcess::spawn_shared now passes
explicit slot paths to the disposable worker, which opens the paired mapping,
reads the host input slot, and writes the output slot before acknowledging the
frame. The process-level integration suite passes both inline and shared-frame
round trips (2 tests); all plugin-host targets compile with strict Clippy.
The worker remains an echo fixture and does not load plugin code or open audio
devices.

Superseding runtime correction: the plugin-host library suite was rerun after
the shared-frame integration and all 25 tests passed. This includes the
previously blocked quarantine/state/transport cases; the generated library test
binary was executable on the current host. The remaining M06 gaps are actual
VST3 plugin loading, sandbox policy, and production worker integration.

Added and executed tools/m06-vst3-loader, a native non-audio loading probe
against the pinned SDK and locally built mda-vst3 bundle. The probe resolved
the bundle's x86_64-win binary, loaded it with LoadLibraryW, obtained the
GetPluginFactory export, enumerated 68 factory classes, released the factory,
and unloaded the module successfully. The generated executable and object file
were removed afterward. This proves module/factory loading only; it does not
claim processor activation, plugin DSP, editor behavior, or OS sandboxing.

The loader probe was then extended to instantiate the first audio-effect class
through IPluginFactory, initialize the component with the null host context,
inspect one input and one output audio bus, terminate/release the component,
and unload the module. This completed successfully against mda-vst3 without
opening an audio device or processing live/user audio. It is component
activation evidence only; audio-process callbacks, parameter/state behavior,
worker integration, and sandbox enforcement remain open.

The same probe then configured the real component for offline 32-bit processing
at 48 kHz with a 64-frame block, submitted a synthetic stereo buffer, verified
all 128 output samples were finite, and shut down processing cleanly. This is
bounded plugin-DSP execution evidence only; parameter automation, state/editor
behavior, worker integration, failure containment, and sandbox enforcement
remain open.

The probe also resolved the component's controller class through
getControllerClassId, created the IEditController, initialized it with the
null host context, and observed 5 parameters before terminating/releasing the
controller. This confirms a real VST3 parameter surface without opening an
editor or audio device; host-side parameter automation/state fidelity and
sandbox enforcement remain open.

The controller probe exercised every one of the 5 discovered parameters by
reading its normalized value, setting 0.5, validating a finite 0..1 response,
and restoring the original value. The complete synthetic automation pass
succeeded before controller shutdown. This does not yet prove AudioRouter
parameter-event forwarding, state persistence, editor behavior, or sandboxing.

The probe then supplied an in-memory IBStream to the real component. getState
emitted a 180-byte opaque payload; seeking back to the beginning and calling
setState succeeded before normal termination and unload. No state file or user
data was written. This is native component state round-trip evidence, not yet
durable AudioRouter state persistence, editor lifecycle, worker failure
containment, or sandbox evidence.

WorkerProcess shutdown now has an explicit timeout boundary: it waits for
graceful exit, kills an unresponsive child after the caller-selected deadline,
reaps it, and returns a stable Timeout error. The existing default uses five
seconds. The complete plugin-host suite (25 unit tests plus 2 process tests)
passes with strict Clippy and formatting. This does not yet make frame reads
nonblocking or establish OS sandbox restrictions.
