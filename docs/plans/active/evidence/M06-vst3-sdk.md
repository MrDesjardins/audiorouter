# M06 VST3 SDK boundary

## 2026-09-08 - Feature-enabled worker containment qualification

The opt-in `test-fixtures` worker suite passed with
`cargo test -p audiorouter-plugin-host --features test-fixtures --locked`:
47 plugin-host unit tests and 20 worker-process tests passed. The process
fixtures exercised bounded crash reaping, hang termination, invalid-output and
state-failure rejection, dynamic latency updates, descriptor transport, and
quarantine history across replacement generations. Feature-enabled strict
Clippy also passed. These are deterministic AudioRouter containment fixtures,
not third-party plugin/editor or OS-sandbox evidence; no plugin was registered
or executed and no audio or machine configuration was accessed.

## Bundle binary enumeration bound (2026-09-08)

VST3 bundle resolution now retains only the information needed to decide
whether `Contents/x86_64-win` contains exactly one regular binary: it returns
`notPe` as soon as a second binary is found. This prevents malformed bundles
with large file counts from causing unbounded temporary path retention during
scan. The regression and 47-test plugin-host suite pass with strict Clippy;
no plugin was executed and no audio or machine configuration was accessed.

## Current installation verification (2026-09-07)

The repository-local installer downloaded or verified the official Steinberg
checkout at revision `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96` and initialized
all seven recursive submodules. The VS2026 acceptance then rebuilt the SDK,
passed 51/51 SDK self-tests and 1,598/1,598 official validator tests, and
passed the offline loader with 68 classes and finite processing. This remains a
source checkout under `third_party/vst3sdk`; no system SDK/plugin registration,
driver, audio stream, or machine audio configuration was changed.

## Current-tip SDK qualification (2026-09-07)

The pinned repository-local SDK was rebuilt with VS2026. All 51 SDK self-tests and 1,598
official validator tests passed; the offline loader verified 68 classes, finite processing,
five-parameter automation, and a 180-byte state round-trip. Generated outputs were cleaned;
no system plugin registration, audio stream, driver, or machine configuration was changed.

## Stable scan diagnostics (2026-09-07)

Plugin scan and inspect responses now retain the existing human-readable error
and also expose a stable `errorCode`. The codes distinguish
`unsupportedExtension`, `notPe`, `unsupportedArchitecture`, `missing`,
`cancelled`, `deadlineExceeded`, `outsideConfiguredRoot`, `tooLarge`, and
`io`. This prevents clients from parsing Rust debug formatting to determine
whether a candidate is unsupported or whether a scan should be retried.
Control coverage (82), plugin-host coverage (33), strict Clippy, contracts
typecheck, and documentation validation passed. No plugin code was loaded or
executed.

The installer now validates recursive submodule status after updating it: all
seven declared submodule paths must be present and initialized, and Git's
uninitialized or commit-mismatch markers are rejected. The check uses Git for
Windows Bash because this host's PowerShell Git launcher does not reliably
provide the POSIX helper commands to direct `git submodule` calls. The real
installer and disposable origin/reparse provenance acceptance both passed.

## UI discovery parity (2026-09-07)

The UI now exposes the existing bounded discovery boundary through a
connected-only plugin scan panel. It accepts an explicitly entered directory,
forwards `plugins.scan` through the typed backend adapter, and renders either
identity/compatibility metadata or the stable `errorCode`; disconnected mode
cannot scan. UI typecheck, 70 UI tests, contracts typecheck, and the
disposable M05 production build passed. The panel does not load plugin code or
change files, audio, or machine configuration.

The same panel also exposes explicit single-candidate `plugins.inspect` for
paths selected by the user, displaying either bounded identity metadata or the
stable diagnostic code. This remains read-only and disconnected-safe; it does
not execute plugin code.

The public contract now constrains `errorCode` to the nine diagnostic values
emitted by inspection plus `null` for a successful identity result. This keeps
clients from accepting or inventing undocumented diagnostic strings; the Rust
schema and TypeScript contract remain aligned.

Directory-level scan failures now use typed application errors instead of the
generic `invalidRequest`: `invalidRoot`, `tooManyCandidates`, `cancelled`,
`deadlineExceeded`, and `io` are surfaced with stable remediation/retry
metadata. The new control regression and existing plugin-host/worker suites
pass with strict Clippy; no plugin code is loaded or executed.

The UI also exposes the existing authenticated `plugins.retry` operation for
an explicitly selected directory. Each retry uses one generated idempotency
key and remains bounded, read-only discovery; disconnected mode cannot invoke
it. UI typecheck, 70 UI tests, and the disposable production build passed.

The panel also exposes `plugins.list` as an explicit “load last scan” action.
It reads the backend's retained result for the entered directory and does not
start a new filesystem scan. UI typecheck, 70 UI tests, and the disposable
production build passed; no plugin code, audio stream, or machine configuration
was accessed.

## Failed-worker cleanup hardening (2026-09-08)

Supervised worker heartbeat, processing, latency, and externally reported
failures now terminate the failed child immediately. Shutdown detects an
already-terminated child before writing to its pipe. Plugin-host (39) and
worker-process (8) tests passed with strict Clippy and formatting. This closes
resource-retention behavior at the process boundary without claiming full OS
filesystem/network sandboxing or third-party plugin execution.

Each scan result now also offers an explicit path-selection action for the
inspection field. Selecting a result only copies the already returned path;
the separate inspect action is still required, so discovery cannot implicitly
load plugin code. M05 typecheck, 73 UI tests, and the disposable production
build passed.

The UI processor catalog now uses the dedicated read-only `processors.list`
method rather than relying solely on the broader system snapshot. Connected
catalog failures are shown as unavailable and disconnected mode does not
invent processor data. Contracts typecheck, UI typecheck, 75 UI tests, and the
disposable M05 production build passed; no processor or plugin was executed.

The UI now also consumes the dedicated read-only `presets.list` method and
displays authoritative voice-chain and EQ metadata separately from local
templates. Catalog failures remain visible and no preset is applied during
refresh. Contracts typecheck, UI typecheck, 76 UI tests, and the disposable
M05 production build passed; no plugin code or audio was executed.

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
but no `cmake.exe` was found on PATH or inside that installation. The repository
provides an ignored portable CMake 4.4.0 cache, so the SDK CMake
configure/example build is available through the project-local toolchain
documented below; this remains build/validator evidence, not proof of general
plugin compatibility.
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

## 2026-09-07 — Current-head SDK requalification

The complete M06 acceptance passed at the current head after retrying the
native build with elevated build-tool access. The pinned repository-local SDK
built successfully; its 51 self-tests passed, the official validator reported
1,598 passed and 0 failed, and the offline loader enumerated 68 classes and
processed a finite stereo block with five parameters and 180 bytes of state.
The initial non-elevated attempt failed before validation in MSBuild's
FileTracker with `E_ACCESSDENIED`; no source or machine configuration was
changed by that failure or the successful retry. No system plugin registration
or live audio stream was used.

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

## 2026-09-06 — Native SDK probe revalidated

Rebuilt and ran tools/m06-vst3-loader/build.ps1 with the installed Visual
Studio Community 2026/MSVC and Windows SDK toolchain against the pinned local
SDK's mda-vst3 bundle. The probe loaded the x64 module, enumerated 68 factory
classes, processed a bounded 48 kHz/64-frame stereo block with finite output,
verified all five normalized parameter read/write/restore operations, and
completed the 180-byte component state round trip. Generated executable and
object files were removed afterward. The probe opened no audio device and
changed no machine configuration; production worker integration, sandbox
enforcement, and realtime scheduling remain open.

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

The disposable worker now drives WorkerSession at runtime, including an
explicit worker-side Hello-sent transition before accepting Ready and process
messages. A process regression confirms that a duplicate sequence is rejected
with a session failure rather than echoed. The full plugin-host runtime suite
passes 25 unit tests plus 3 process tests with strict Clippy. The worker still
uses a placeholder zero-based clock and does not yet enforce OS sandbox policy.

Worker deadline validation now uses a shared Unix-epoch millisecond clock
instead of a constant zero. Future-dated frames continue to round-trip, while
the process regression confirms an expired frame returns DeadlineExpired and
is not processed. The worker suite passes 25 unit tests plus 4 process tests
with strict Clippy; OS sandboxing and nonblocking frame reads remain open.

WorkerProcess now reads child stdout on a dedicated bounded-channel reader
thread. Control calls receive responses with a five-second timeout, converting
EOF, disconnect, and timeout into stable worker-message errors instead of
blocking indefinitely. A timeout regression and the complete 26-test
plugin-host suite pass with strict Clippy; OS sandbox policy and production
plugin execution remain open.

WorkerProcess now attaches every Windows child to a fail-closed Job Object
configured with KILL_ON_JOB_CLOSE. The job handle is owned by the process
wrapper, so normal shutdown and Drop cannot leave a worker process running
after the wrapper is gone; job creation/assignment failure rejects the spawn.
The native worker process tests pass with this containment active. This is
process-lifetime containment only; filesystem/network sandbox policy and real
plugin execution remain open.

## 2026-09-06 — Plugin state path hardening

Hardened plugin state file safety by checking root and asset symlink metadata
before canonicalization. Symlink roots are rejected and a symlink asset cannot
be followed into an approved directory; the existing exclusive-write and hash
validation rules remain intact. The Windows plugin-host suite passes 26 unit
tests plus 4 worker-process tests with strict Clippy. Reparse-point coverage
and full filesystem/network sandbox policy remain separate native work.

## 2026-09-06 — Windows reparse-point state boundary

Plugin state roots and assets now reject the Windows `FILE_ATTRIBUTE_REPARSE_POINT`
attribute before canonicalization. This covers junctions and other Windows
reparse objects in addition to ordinary symlinks; non-Windows builds retain
symlink detection through the portable metadata API. The plugin-host unit and
worker-process suites pass with strict Clippy, formatting, and diff checks.
Nested reparse-point traversal and a full filesystem/network sandbox remain
open native work.

## 2026-09-06 — Nested state path traversal hardening

State reads now inspect the requested asset and every existing ancestor on the
path back toward the approved canonical root. Any symlink or Windows reparse
component is rejected before canonicalization and file opening, closing the
nested junction/symlink traversal case. Plugin-host unit and worker-process
tests pass with strict Clippy. This remains path hardening, not a complete OS
filesystem/network sandbox for plugin execution.

The regression suite also creates a linked state directory where the host allows
it and verifies that a file beneath that directory is rejected. This covers the
case where the leaf itself is regular but an intermediate component redirects
outside the approved root; link creation is conditional because Windows may
require a separate developer-mode or privilege setting.

State reads additionally require the canonical asset to be a direct child of
the approved root, matching the flat layout produced by `write_state_asset`.
This removes nested path ambiguity and reduces the state-file TOCTOU surface;
full OS filesystem/network sandboxing for plugin execution remains open.

## 2026-09-06 — Worker executable provenance

`WorkerProcess` now validates its executable before spawning: the path must be
absolute, resolve canonically, identify a regular file, and carry no symlink or
Windows reparse metadata. This prevents an accidental relative/PATH resolution
from selecting an unintended worker binary. The plugin-host suite passes 27 unit
tests plus 4 worker-process tests with strict Clippy; this is launch provenance,
not full OS sandbox enforcement or actual plugin execution.

Worker setup failures are now fail-closed as well: if Job Object attachment or
either piped handle cannot be obtained after spawn, the child is explicitly
killed and reaped before the error returns. `WorkerProcess::Drop` uses the same
cleanup helper. The plugin-host suite passes 27 unit tests plus 4
worker-process tests with strict Clippy; this does not establish full OS
filesystem/network sandboxing or plugin execution.
## 2026-09-06 — Current native loader qualification

The native loader was rebuilt and run with the installed Visual Studio Community
2026/MSVC and Windows SDK toolchain against the pinned local mda-vst3 bundle.
The real x64 module loaded and enumerated 68 classes; offline stereo processing
of 64 frames produced finite output; all five controller parameters were read,
changed, and restored; and a 180-byte component state payload round-tripped.
The generated executable/object were removed afterward. No audio endpoint or
machine configuration was accessed; worker integration, full OS sandboxing,
and production plugin execution remain open.

## 2026-09-06 - SDK installation verification

Ran the repository-local SDK setup script after the Visual Studio/WDK update.
The pinned Steinberg source checkout is present at `third_party/vst3sdk` at
revision `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`, recursive submodules are
initialized, and the required hosting headers are available. This is a
project-local source SDK, not a global installer; no system plugin, audio
configuration, or machine-wide SDK setting was changed.

## 2026-09-06 - Supervised worker lifecycle bridge

Added `SupervisedWorkerProcess`, which couples the existing worker protocol
client to `WorkerSupervisor`. Successful process, shared-process, and latency
exchanges refresh the bounded heartbeat; spawn and protocol failures enter the
existing failure/quarantine policy. The wrapper exposes explicit polling and
shutdown but deliberately does not restart workers, load plugin code, or open
audio. The plugin-host suite passes 29 unit tests and 5 process/integration
tests with strict Clippy.

The supervised wrapper now fails closed after a heartbeat timeout or prior
worker failure: process, shared-process, and latency calls are rejected until
an outer supervisor deliberately creates a replacement. A regression covers
the timeout boundary; the plugin-host suite passes 29 unit tests and 6
process/integration tests with strict Clippy.

The same supervised boundary now covers the shared-memory transport path via
`spawn_shared`; its integration regression exchanges a mapped stereo frame,
refreshes the heartbeat, and shuts down cleanly. This remains process and
transport evidence only: plugin loading, automatic restart, and full OS
sandboxing are not claimed.

`SupervisedWorkerProcess` also detects an exited child during `poll` and
accepts explicit failure reports from an outer adapter. Both paths enter the
same fail-closed state and prevent subsequent processing; an integration
regression covers the explicit process-failure report. The suite passes 29
unit tests and 7 process/integration tests with strict Clippy.

The lifecycle documentation was corrected to distinguish the portable
`WorkerSession` handshake state machine from the native `WorkerProcess` and
its Job Object containment; neither layer claims actual plugin execution.

Replacement spawning now accepts and returns an explicit `WorkerSupervisor`
ledger, preserving failure history across worker generations. A regression
exercises three deliberate generations and confirms the third failure enters
quarantine rather than resetting the count. The suite passes 29 unit tests and
8 process/integration tests with strict Clippy.

`SupervisedWorkerProcess::restart` provides deliberate replacement for both
pipe-only and shared-memory workers. It drops the failed process before
spawning, carries the caller-owned mapped transport into a shared replacement,
retains the supervisor ledger, rejects a running worker with an explicit
protocol error, and returns the ledger alongside spawn errors. The
three-generation and shared round-trip regressions confirm quarantine history
and transport continuity. No plugin is loaded and no audio endpoint or machine
configuration is touched.

## Native loader compile revalidation (2026-09-06)

The checked-in `tools/m06-vst3-loader/build.ps1` compiled successfully with
Visual Studio Community 2026 (`MSVC 14.51.36231`) and Windows SDK
`10.0.28000.0`, using the pinned local SDK headers. The invocation required a
process-scoped PowerShell execution-policy bypass because the machine policy
does not permit direct script execution; the persistent policy was not
changed. Generated executable/object outputs were removed immediately.

This is compile evidence only. The loader was not executed, no plugin binary
was downloaded or loaded, and no audio endpoint or machine configuration was
changed.

## Native loader fixture run (2026-09-06)

The loader was then rebuilt and run against the existing local SDK fixture
`third_party/vst3sdk-build/VST3/Release/mda-vst3.vst3`. It loaded the bundle's
factory and enumerated 68 classes, including audio-module/controller pairs. It
initialized the first audio component, processed a bounded 64-frame stereo
offline block with `finite=true`, verified five normalized parameter
set/readbacks, and completed a 180-byte component state save/restore.

This is controlled local-fixture evidence, not production compatibility
evidence: it covers one SDK sample/vendor and offline processing only. The
three-available-effects/two-vendor gate, realtime worker integration, crash/
hang/NaN/dynamic-latency fixtures, and editor lifecycle remain open. No audio
device was opened and no machine configuration was changed. Generated loader
outputs were removed after the run.

The probe also now checks the return value from each audio-bus activation and
from every normalized parameter write and restore. This makes plugin-side
argument failures visible instead of silently accepting a partial probe. The
fixture was rebuilt and rerun successfully after this change; no endpoint or
machine configuration was touched.

The negative path was also exercised with a nonexistent bundle target. The
probe returned exit code 1 with `resolved plugin binary is not a regular file`
before calling the loader, confirming fail-closed target validation. Generated
outputs were removed after the check.

The loader now validates that the resolved target is a regular file and uses
`LoadLibraryExW` with `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR` and
`LOAD_LIBRARY_SEARCH_DEFAULT_DIRS`, avoiding the broader legacy DLL search
behavior. The local fixture was rebuilt and rerun successfully after this
change. This remains a user-space probe; no audio endpoint or persistent
machine configuration was touched.

The probe was hardened to require that at least one factory class actually
passes the compatible audio-effect path. A bundle containing only non-audio
classes can no longer produce a success-shaped result. The rebuilt probe was
rerun against the same local fixture and passed; generated outputs were again
removed.

## Class selection and native result diagnostics (2026-09-06)

The loader accepts an optional `--class-index` selector and reports the exact
VST3 result code and operation when a selected effect rejects a step. Against
the local mda bundle, class indices 0 (Ambience), 4 (BeatBox), 6 (Combo), and
12 (Delay) passed offline processing, automation, and state round-trip. Class
2 (Bandisto) and class 32 (Limiter) rejected processor activation with
`0x80004001` (`E_NOTIMPL`), demonstrating a fixture-specific unsupported
activation path rather than an `E_INVALIDARG` from the loader. No failure was
converted into a success result.

The selector also makes the class list and compatibility result reproducible
for future plugin fixtures. All runs were offline, used the local SDK fixture,
and removed generated probe outputs afterward.

Selector rejection paths were also verified. Selecting the controller at index
1 and a nonexistent index 999 both returned exit code 1 with
`factory exposes no compatible audio effect`; neither path loaded a component
or produced a success-shaped result.

## Acceptance revalidation with VS2026 (2026-09-06)

After rerunning with elevated build-tool access to permit MSBuild file
tracking, `tests/acceptance/m06-vst3-sdk.ps1` passed the pinned checkout and
Release SDK build, all 51 SDK self-tests, the official validator (1,598 passed,
0 failed), and the offline native loader. The run used Visual Studio Community
2026, MSVC 14.51.36231, and Windows SDK 10.0.28000.0. Generated outputs were
cleaned; no system plugin registration or audio configuration changed.

The same acceptance was rerun from the current post-hardening tip on
2026-09-06. The repository-local checkout remained at revision
`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`; the Release build, 51 SDK tests,
1,598/0 official validator result, and offline loader all passed again.

The acceptance was revalidated at repository revision `22da8a2` after the
native compile acceptance was added. The repository-local SDK build and
offline fixture run passed again with the same VS2026/MSVC/Windows SDK
toolchain. The loader executable and object were removed in the acceptance
cleanup; the ignored SDK build directory is retained for repeatable local
builds. No system plugin registration, driver installation, endpoint change,
or other audio configuration was performed.

## Installer verification on the current tip (2026-09-06)

The repository installer was run with a process-scoped PowerShell execution
policy bypass because the host policy blocks direct script invocation:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tools\m06-vst3-sdk\install.ps1
```

It confirmed the source checkout at
`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96` and required hosting headers. The
subsequent M06 acceptance passed with 51 SDK self-tests, 1,598 official
validator tests, and the offline fixture loader. This remains a
repository-local source SDK setup, not a global Windows SDK or system plugin
installation; no audio configuration was changed.

The installer and full acceptance were rerun again at the current repository
tip. The first build attempt was stopped by the restricted host with
`E_ACCESSDENIED` from MSBuild file tracking before compilation. With authorized
native build access, the same acceptance passed: 51 SDK self-tests, the
official validator at 1,598 passed and 0 failed, and the offline loader over
the local `mda-vst3` fixture. Temporary generated probe outputs were cleaned;
no global SDK/plugin registration or audio configuration changed.
## Current-tip SDK qualification (2026-09-06)

The M06 acceptance wrapper rebuilt the pinned repository-local SDK with
Visual Studio Community 2026, passed all 51 SDK self-tests and 1,598 official
validator tests with zero failures, and completed the offline VST3 fixture
loader (68 classes, finite stereo block, parameter automation, and 180-byte
state round trip). Build outputs were cleaned. This does not claim realtime
plugin execution, full sandboxing, or system plugin installation.
## Installer provenance hardening (2026-09-06)

The SDK installer now rejects reparse-point destinations and verifies that an
existing checkout's `origin` is the expected Steinberg VST3 SDK repository
before accepting the pinned revision or applying `-Force`. This is a
repository-local provenance check and does not alter the installed checkout or
machine configuration.
## Installer provenance regression (2026-09-06)

`tests/acceptance/m06-sdk-installer.ps1` creates a disposable Git checkout
with an unrelated origin and verifies that the installer fails with the
explicit origin-mismatch diagnostic. Windows CI runs this check; the fixture
is removed afterward and no SDK, plugin, driver, or audio configuration is
changed.
## Installer parent-chain protection (2026-09-06)

The installer now audits the complete existing destination-parent chain before
clone or checkout operations. Its disposable regression rejects both an
unrelated Git origin and a destination below a symbolic-link parent when the
host permits link creation; fixtures are removed afterward.
## Missing-parent compatibility correction (2026-09-06)

The installer preserves its original ability to create missing ordinary
destination parents. It walks upward to the nearest existing ancestor before
checking the full parent chain for reparse points, so redirected ancestors are
rejected without imposing a pre-existing-parent requirement.

The M06 acceptance wrapper was requalified at the current head with the
authorized VS2026 toolchain. The pinned SDK built successfully; 51 SDK
self-tests and 1,598 official validator tests passed, and the offline loader
verified 68 classes, finite processing, five-parameter automation, and a
180-byte state round-trip. Generated outputs were cleaned; no system plugin
registration, audio stream, driver, or machine configuration was changed.

## Current-tip acceptance after global topology validation (2026-09-07)

The acceptance was rerun at the current head using the installed VS2026
MSVC/Windows SDK toolchain and the pinned repository-local checkout. The SDK
build passed, all 51 SDK self-tests passed, the official validator reported
1,598 passed and 0 failed tests, and the offline loader verified 68 classes,
finite stereo processing, five-parameter automation, and a 180-byte state
round-trip. Temporary loader outputs were removed. No system plugin
registration, audio stream, driver, or machine configuration was changed.

## Project-local SDK installation confirmation (2026-09-07)

The repository-local installer was run successfully from PowerShell. It
confirmed the pinned checkout at
`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`, initialized all seven SDK
submodules, and found the required hosting header. The complete M06
acceptance then passed: 51 SDK self-tests, 1,598 official validator tests,
and the offline loader's 68-class, finite-processing, automation, and
180-byte state checks. The checkout is intentionally project-local under
the ignored `third_party/vst3sdk` path; no global SDK/plugin registration,
driver, audio stream, or machine audio configuration was changed.

## Safe Bash path argument handling (2026-09-07)

The installer previously interpolated the destination into the Bash command
used for recursive submodule operations. It now passes the converted path as a
positional argument, so valid Windows destinations containing apostrophes are
not interpreted as shell syntax. The disposable origin/reparse-point
provenance acceptance and the real SDK installer plus offline M06 acceptance
pass after this change; no SDK, plugin, driver, audio stream, or machine
configuration was changed.

## Editor lifecycle policy groundwork (2026-09-08)

The plugin-host library now models the optional editor as a bounded control-plane
state machine: `Closed`, `Open`, and `Failed`, with explicit close and retry
transitions. Its processing-generation token is immutable, and regression tests
prove that open, close, failure, and retry do not restart processing. This is
policy evidence only: no native window was created, no plugin editor code was
loaded, and native UI-thread ownership remains an open Windows acceptance gate.

## Dynamic latency session boundary (2026-09-08)

`WorkerSession` now retains the latest bounded latency report and permits
dynamic sample-count changes only when the negotiated sample rate remains
constant. A sample-rate change is rejected and leaves the previous report
intact. This protects downstream compensation from silently adopting an
incompatible rate. The regression is protocol-state evidence, not a measured
plugin latency or realtime graph-compensation result.

The process-level worker regression then exercised an unsupported runtime
sample-rate change. The worker rejected it, and `WorkerProcess::report_latency`
now preserves the structured `session:InvalidLatency` failure for callers;
the adapter no longer collapses that result into an unclassified response.

Worker failure-code payloads are additionally capped at 128 bytes before
serialization and after decoding. Empty or oversized codes are rejected as
`InvalidFailureCode`, keeping failure diagnostics bounded independently of the
larger worker message envelope.

## Controlled worker failure fixtures (2026-09-08)

The opt-in `test-fixtures` feature adds deterministic worker modes for a
post-handshake crash, a nonresponsive hang, and malformed processed output.
The process suite passed 13 tests with the feature enabled: the crash was
reaped, the hang was killed by bounded shutdown and also contained by the
supervisor heartbeat path, and malformed output was rejected at the reader
boundary. The normal build does not enable these modes; no third-party plugin
code or audio device was involved.

## Discovery-to-launch identity revalidation (2026-09-08)

`PluginIdentity::verify_current` now reinspects the exact selected path against
the caller's configured roots and compares canonical path, binary path, format,
architecture, byte count, and SHA-256. Changed bytes and a root-grant mismatch
are rejected without substitution. This protects a future worker launch from
stale scan results; it does not claim plugin execution or sandbox completion.

`SupervisedWorkerProcess::spawn_verified` now composes revalidation with
supervised creation. The process regression used a temporary copied executable
as a VST3 fixture, revalidated it, processed one frame through the supervised
worker, and removed the temporary directory. This is launch-boundary evidence;
it is not third-party VST3 execution or full sandbox evidence.

A second process regression verifies that a missing scanned identity fails
before worker creation with a typed inspection error. The ordinary worker
suite has 13 passing tests; the feature-enabled fixture suite has 17.

## Opaque worker state transport (2026-09-08)

Worker messages now support explicit state restore/save and an opaque state
response. Every asset is non-empty, capped at 512 KiB, and SHA-256 checked at
the encode/decode boundary. The process regression restores an asset and saves
it back byte-for-byte; bounded and corrupt assets are rejected. This is worker
transport evidence only and does not claim vendor-specific VST3 state support.

Both worker wrappers now expose version-aware restore helpers. A mismatched
version is rejected before IPC with a typed state error, and the worker remains
usable; this preserves the version contract independently of vendor-specific
native serialization.

## Typed worker parameter descriptors (2026-09-08)

The worker protocol now validates bounded typed parameter descriptors: no more
than 256 unique IDs, 128-byte non-empty titles, finite normalized ranges, and
defaults inside those ranges. The ordinary disposable worker retains its
explicit empty catalog, while the opt-in `descriptors` fixture returns two
valid entries across the subprocess boundary. The process regression verifies
their IDs, titles, and normalized bounds; native VST3-to-worker mapping
remains open.

State version `0` is now rejected consistently during asset construction,
restore verification, and worker-wire validation, matching the existing
storage contract. The protocol regression covers a correctly hashed but
invalid-version asset.

The negative process path also requests a save before restore. It returns the
bounded `stateUnavailable` protocol failure and terminates the disposable
worker, proving that missing state is not synthesized as an empty asset.

The supervised process regression also performs version-aware restore, verifies
that a mismatched version does not restart or kill the worker, then restores and
saves a compatible asset successfully.

The opt-in `latency` fixture returns two bounded same-rate latency updates
across the process boundary. The feature-gated process regression observes 128
samples becoming 192 and then 256 at 48 kHz before clean shutdown. This proves
dynamic protocol transport only; plugin-reported latency and graph compensation
remain native/runtime gates.

The supervised wrapper regression now exercises the same fixture paths: it
observes a dynamic latency response while the supervisor remains `Running`,
then requests a two-entry descriptor catalog through the supervised process
adapter before clean shutdown. This covers wrapper heartbeat/error handling in
addition to raw IPC framing.

The all-features workspace requalification passed after this change. The
plugin-host package contributed 46 library tests, 20 feature-enabled process
tests, and the complete workspace suites passed; this remains fixture and
control-plane evidence rather than native third-party realtime execution.

## Cross-vendor loader matrix (2026-09-08)

## Native controller descriptor discovery (2026-09-08)

The offline loader now applies the worker descriptor ceiling while inspecting
each controller: negative or over-256 counts fail closed, defaults must be
finite normalized values, and each bounded record reports the VST3 parameter
ID, an ASCII-safe title, normalized default, step count, and flags. The
acceptance wrapper asserts that the default mda run emits the descriptor-count
field rather than only relying on the process exit code.

The rebuilt loader and pinned SDK acceptance passed with the installed
VS2026/MSVC/Windows SDK toolchain. The mda matrix emitted 5, 13, 8, 4, and 7
descriptor records for Ambience, BeatBox, Combo, DeEsser, and Degrade. The
ChowMatrix controller emitted a valid zero-entry catalog. This is native
offline discovery evidence and does not yet map vendor-specific VST3 metadata
into the Rust worker catalog or claim third-party realtime execution.

After the zero-parameter probe correction, the native loader was rebuilt and
run against five distinct classes in the pinned mda bundle. These all passed
the same bounded offline stereo processing, finite-output, parameter
automation, and component-state checks:

| Vendor | Effect | Class | Result |
| --- | --- | ---: | --- |
| mda | Ambience | 0 | pass; 5 parameters; 180-byte state |
| mda | BeatBox | 4 | pass; 13 parameters; 180-byte state |
| mda | Combo | 6 | pass; 8 parameters; 180-byte state |
| mda | DeEsser | 8 | pass; 4 parameters; 180-byte state |
| mda | Degrade | 10 | pass; 7 parameters; 180-byte state |

Combined with the ChowMatrix result recorded below, this is six
loader-compatible
effects from two independently sourced vendors. It is not a blanket VST3
compatibility claim: ChowMatrix's official SDK validator still reported 45
passed and 2 failed (`Valid State Transition 32bits` and `Bus Activation`),
and worker/editor containment has not yet been exercised with third-party
code. The matrix run used only disposable/local fixtures and did not install,
register, or alter audio configuration.

The reproducibility path is now part of `tests/acceptance/m06-vst3-sdk.ps1`:
after the standard mda validator and loader checks, it invokes the loader for
class indices 0, 4, 6, 8, and 10. The wrapper still removes its generated
loader executable/object in `finally`; the ChowMatrix source fixture remains a
separate disposable qualification because it is not redistributed by the
repository.

## Outbound worker-message validation (2026-09-07)

`encode_worker_message` now validates the complete message before producing a
wire frame, preventing locally constructed invalid handshakes, failures,
frames, parameter events, or latency values from crossing the worker boundary.
Existing negative cases were moved to assert sender-side rejection, while
valid round trips remain covered. The plugin-host suite passed 36 unit tests and
8 worker-process tests with doc-tests, formatting, and strict Clippy. Native
plugin execution and full OS sandboxing remain open.
## 2026-09-07 — Current-head SDK requalification

The repository-local installer repaired/verified the pinned Steinberg VST3 SDK
checkout at revision
`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`. The M06 acceptance wrapper then
completed with the installed Visual Studio Community 2026 toolchain (MSVC
14.51.36231, Windows SDK 10.0.28000.0): all 51 SDK self-tests and 1,598
official validator tests passed, and the offline loader successfully opened
the built x64 fixture and processed a finite block.

The generated SDK build tree is repository-local and ignored. No system-wide
SDK installation, plugin registration, driver action, audio stream, or machine
audio configuration change occurred.

The full safe sweep also exercised M06 after other acceptance jobs had
completed. A concurrent first attempt encountered an MSBuild `FileTracker`
access conflict in the shared ignored build directory; it did not indicate an
SDK or source failure. A sequential rerun passed the same 51 self-tests, 1,598
validator tests, and offline loader checks, after which the exact generated
build directory was removed.

## Clean acceptance configuration fix (2026-09-07)

The acceptance wrapper previously attempted `cmake --build` without creating
the ignored build tree, so a clean checkout failed before compilation. It now
configures `third_party/vst3sdk-build` with the installed `Visual Studio 18
2026` x64 generator and `SMTG_CREATE_PLUGIN_LINK=0`, preventing SDK example
plugin-link creation outside the repository. A clean rerun passed 51 SDK
self-tests, 1,598 official validator tests with zero failures, and the offline
loader. The generated repository build tree was removed; no user plugin
directory, system registration, or audio configuration was changed.

## Per-frame parameter offset validation (2026-09-07)

Worker protocol validation now checks automation offsets against the actual
audio frame count for both inline and shared-memory process messages. A focused
regression rejects an offset equal to the frame count in both forms while the
existing valid in-frame round trip remains accepted. The plugin-host suite
passed 36 unit tests and 8 worker-process tests with strict Clippy, formatting,
and doc-tests. Native plugin execution and full OS sandboxing remain open.

## Bounded queue construction (2026-09-07)

Worker frame and parameter queue constructors now clamp requested capacities to
the existing protocol limits before allocating their `VecDeque` storage. A
regression using `usize::MAX` confirms frame queues remain capped at 2,048
entries and parameter queues at 128 entries. The plugin-host library tests and
strict Clippy pass. Worker subprocess tests remain blocked by the host's
Application Control policy (OS error 4551), not by the queue change.

## Bounded state restore reads (2026-09-07)

Plugin-state restore now caps the post-open read at 16 MiB plus one byte, so a
file-growth or replacement race cannot turn `read_to_end` into an unbounded
allocation. Oversized files return `TooLarge` after the bounded read, and a
regression covers the existing oversized-file path. The plugin-host library
suite passed 38 tests with strict Clippy and formatting; no plugin was loaded
or executed.

## State verification enforces size invariants (2026-09-07)

`PluginStateAsset::verify_for_restore` now rejects empty or oversized public
asset values before version/hash checks. This closes the bypass where callers
could construct the public fields directly without using the bounded
constructor. A regression with a correctly hashed 16 MiB-plus-one-byte asset
returns `TooLarge`; the plugin-host library suite passed 39 tests with strict
Clippy and formatting.

## Local SDK reinstall and acceptance (2026-09-07)

The repository-local installer downloaded/repaired the pinned Steinberg VST3
SDK at revision `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96` under the ignored
`third_party/vst3sdk` directory. Dedicated M06 acceptance then passed the 51
SDK self-tests, 1,598 official validator tests, and the offline loader checks
(68 classes, finite stereo processing, five parameters/automation, and a
180-byte state round-trip). The installed Visual Studio Community 2026 host
resolved Windows SDK `10.0.28000.0`; no global SDK/plugin registration, driver,
audio stream, or machine configuration was changed.

## Plugin inventory response bound (2026-09-07)

The `plugins.scan`, `plugins.list`, and `plugins.retry` output contract now
advertises the plugin host's existing 256-candidate ceiling. Discovery
regression coverage preserves the read-only boundary: scanning inspects
metadata without loading plugin code, and no plugin registration or machine
audio state is changed.

The SQLite plugin-state write boundary now enforces the shared 128-byte record
identity limit before persisting opaque state metadata. A direct-storage
regression rejects oversized IDs on both save and removal; storage coverage
passes 50 tests with strict Clippy and formatting. Plugin execution and OS
sandbox gates remain open.

## Plugin binary-size schema alignment (2026-09-08)

The `fileBytes` field in `plugins.scan`, `plugins.list`, and `plugins.inspect`
now advertises the enforced 256 MiB `MAX_PLUGIN_BYTES` ceiling. Control (86)
and plugin-host (39 plus 8 worker-process) tests, strict Clippy, formatting,
diff checks, and documentation validation passed. Plugin execution and OS
sandbox gates remain open; no plugin was loaded by this change.

Plugin-state storage now applies the same nonempty 128-byte plugin identifier
bound on writes and filtered reads as on removal. The regression prevents a
direct SQLite caller from bypassing the identity contract; storage (52),
plugin-host (39), and worker-process (8) tests plus strict Clippy and
documentation validation passed. Plugin execution and OS sandbox gates remain
open.

## Offline SDK/loader requalification (`656fdb3`, 2026-09-08)

The elevated repository-local M06 acceptance passed with the pinned SDK:
51 SDK self-tests and 1,598 official validator tests passed. The x64 offline
loader discovered 68 classes and verified finite stereo processing, five
parameters with automation, and a 180-byte state round trip. The build used
the installed Visual Studio Community 2026/MSVC and Windows SDK
`10.0.28000.0`; no plugin was globally registered and no audio or machine
configuration changed. This does not satisfy the required independent
multi-vendor fixture, editor, or full OS-sandbox execution gates.

## Plugin-state numeric read validation (2026-09-08)

Persisted plugin-state versions now use checked SQLite integer decoding. A
negative version regression fails closed rather than wrapping to a large
nonzero `u32` value that could bypass the version invariant. The focused
storage regression and strict Clippy passed; no plugin was executed and no
audio or machine configuration was accessed.

## Plugin-state read-boundary validation (2026-09-08)

Plugin-state list hydration now revalidates persisted record IDs, plugin IDs,
SHA-256 identities, versions, absolute paths, bounded sizes, and reparse-point
ancestry before returning opaque metadata. A corrupt-row regression fails
closed. Storage (60) and control (87) tests, strict Clippy, formatting, and
diff checks passed; no plugin was executed and no audio or machine
configuration was accessed. Plugin execution and OS sandbox gates remain open.
## Plugin-state SQLite numeric boundary (2026-09-08)

Persisted plugin-state `size_bytes` values now use checked signed-to-unsigned
decoding, matching the existing version boundary. Negative legacy or corrupt
values fail before metadata reaches plugin consumers. Storage coverage increased
to 67 tests; strict Clippy, formatting, and diff checks passed. No plugin was
executed and no audio or machine configuration was accessed.

The repository-local SDK installer was also rerun successfully on 2026-09-08
at the pinned revision, confirming the source-distributed SDK checkout and all
seven recursive submodules remain available. This setup action does not install
a system SDK, register plugins, or change audio configuration.

## Native M06 acceptance requalification (2026-09-08)

The checked-in `tests/acceptance/m06-vst3-sdk.ps1` wrapper passed with Visual
Studio Community 2026/MSVC 14.51.36231 and Windows SDK `10.0.28000.0`: 51 SDK
self-tests and 1,598 official validator tests passed. The offline x64 loader
found 68 classes and verified finite stereo processing, five parameters with
automation, and a 180-byte state round trip. Outputs were repository-local and
temporary. No plugin was globally registered, no driver was installed, and no
audio or machine configuration changed.

## Plugin-host containment regression requalification (2026-09-08)

The complete `audiorouter-plugin-host` target passed 39 unit tests plus
doc-tests, and its worker-process integration target passed 8 tests. Coverage
includes bounded worker frames/deadlines, handshake identity, quarantine and
heartbeat policy, explicit worker termination, shared-memory epoch/sequence
guards, state-file integrity/reparse protection, and bounded scanner behavior.
Strict Clippy passed. This is portable/process-boundary evidence; actual
third-party plugin execution and full OS filesystem/network sandboxing remain
open.

## Elevated SDK acceptance after FileTracker access failure (2026-09-08)

An initial non-elevated requalification failed in MSBuild FileTracker with
`E_ACCESSDENIED` while evaluating the existing `ZERO_CHECK.vcxproj`; disabling
tracking did not remove the access failure. Rerunning the same repository-local
acceptance elevated passed: 51 SDK self-tests, 1,598 validator tests, 68
classes, finite stereo processing, five-parameter automation, and a 180-byte
state round trip. No global plugin registration or audio configuration change
occurred. The elevation requirement is an environment prerequisite, not an
SDK or plugin compatibility result.

## Fixture-gate inventory (2026-09-08)

The standard system and user VST3 locations were inspected read-only and did
not contain additional `.vst3` bundles. Only the repository-local mda fixture
is available for the current offline probe, so the required three-effect,
two-vendor M06 matrix remains open. No plugin was downloaded, installed,
registered, loaded, or executed during this inventory.

## Independent ChowMatrix fixture (2026-09-08)

The official [ChowMatrix source repository](https://github.com/Chowdhury-DSP/ChowMatrix)
was cloned with submodules into a disposable temporary directory and built at
commit `40d8e0ef1f752a6843099ff3dfc3d99132b332eb` with repository-local CMake
4.4.0, Visual Studio Community 2026/MSVC 14.51.36231, and Windows SDK
`10.0.28000.0`. The source is BSD-3-Clause licensed. No source, binary, or
plugin registration was added to this repository or the system.

The produced bundle was:

`ChowMatrix.vst3/Contents/x86_64-win/ChowMatrix.vst3`, 6,294,016 bytes,
SHA-256 `9ed07c61c3ddba6504b7307a92087ee37ec6236e2989954b4cc2e8053ee0d457`.

The read-only AudioRouter scanner identified the bundle as a supported x64
VST3. The native M06 loader initially rejected it because its controller has
zero automatable parameters. That was a loader-probe defect: zero parameters
is a valid VST3 surface. After removing that assumption, the rebuilt loader
passed against the same bundle:

`classes=2`, audio effect `ChowMatrix`, finite offline stereo processing at 64
frames, `parameters=0`, and a 3364-byte component-state round trip.

The official SDK validator was also run against the same disposable bundle.
It reported 45 tests passed and 2 failed: `Valid State Transition 32bits` and
`Bus Activation`. The remaining failures are fixture compatibility findings,
not AudioRouter scanner or loader failures, and are retained rather than
waived. This adds one independently sourced vendor/effect to the evidence
set, but the M06 requirement for three compatible x64 effects from at least
two vendors remains open until the full fixture matrix and worker/editor
containment checks pass. The disposable checkout and generated outputs were
removed after capture; no driver, plugin registration, or machine audio
configuration was changed.

The updated `tests/acceptance/m06-vst3-sdk.ps1 -SkipBuild` wrapper was
requalified after this addition. The existing pinned SDK validator and default
loader passed, followed by successful loader runs for mda class indices 0, 4,
6, 8, and 10. The wrapper removed the generated executable and object after
the run; no system plugin registration or audio configuration was changed.

## Bounded VST3 bundle metadata (2026-09-08)

The scanner now reads at most `MAX_PLUGIN_METADATA_BYTES` (1 MiB) from an
optional VST3 `Contents/Resources/moduleinfo.json` file. It tolerates the
trailing commas emitted by the official SDK's module-info tool, extracts
bounded vendor/version strings, and retains at most 256 deduplicated class
IDs. Missing or malformed optional metadata is represented by empty fields;
the scanner still relies on the PE and bundle checks for compatibility and
never loads plugin code.

The metadata is exposed as `vendor`, `version`, and `classIds` in both
`plugins.scan` and `plugins.inspect`, with matching Rust output schemas and
TypeScript contracts. The fixture regression verifies extraction, duplicate
class-ID removal, and the supported x64 identity without creating a native
plugin process. Plugin-host (39) and control (90) tests, strict Clippy,
contracts typecheck/drift, formatting, diff checks, and documentation
validation passed. No audio or machine configuration was accessed.

## SDK installer provenance acceptance (2026-09-08)

`tests/acceptance/m06-sdk-installer.ps1` passed: a disposable checkout with
the wrong origin was rejected, and a destination below a reparse-point parent
was rejected when link creation was available. Disposable Git fixtures were
removed. This validates installer provenance checks only; no SDK, plugin,
driver, or audio configuration was changed.

## Plugin-state inventory hardening (2026-09-08)

SQLite plugin-state listing now reads at most 501 rows, validates every
returned record, and fails explicitly when more than the shared 500-item
inventory bound exists. A 501-state regression passed with 69 storage tests,
87 control tests, strict Clippy, and formatting. This changes metadata
handling only; no plugin was executed and no audio or machine configuration
was accessed.

## VST2 ABI contract (2026-09-08)

Added a portable, non-executing VST2 ABI contract containing the VST 2.4
`AEffect` layout, `VSTPluginMain`/callback signatures, and validation for the
magic, required callbacks, replacing processor, bounded mono/stereo channels,
and parameter count. Two focused ABI tests passed. This does not load or call
any DLL; the Windows worker adapter must still own the lifetime and enforce the
callback/audio-buffer policy before runtime support can be enabled.

The ABI module now also provides a Windows-only RAII library handle with
bounded format setup, replacing-process buffer checks, documented FFI safety
invariants, and deterministic cleanup. Package compilation and strict Clippy
pass. It remains deliberately disconnected from worker startup; no third-party
DLL was loaded or called by this change.

## VST2 worker integration and fixture matrix (2026-09-08)

The verified identity path now passes the canonical VST2 binary path to the
Windows job-contained worker. The worker loads `VSTPluginMain`, performs the
VST2 open/format/mains lifecycle, supplies bounded planar buffers including
zeroed sidechain inputs, and returns interleaved output to the protocol. The
opt-in native test passed for `reacomp-standalone.dll` and
`reagate-standalone.dll`. `readelay-standalone.dll` and
`reaxcomp-standalone.dll` exceeded the five-second worker response deadline;
`reaeq-standalone.dll` and `reafir_standalone.dll` terminated the worker during
processing. The latter four remain unsupported and quarantinable fixtures;
this is not blanket VST2 compatibility evidence. No audio device or machine
configuration was accessed, and the ignored fixture DLLs were not committed.

The host callback supplies only bounded VST2 version, sample-rate, and block
size responses; it performs no blocking or audio-device work. ReaEQ remained
non-responsive under the five-second worker deadline with both short and
128-frame blocks, while ReaComp passed with a 128-frame block. This preserves
the distinction between a qualified fixture and an explicit unsupported one.

The ReaComp run additionally requested its parameter descriptors and applied a
bounded event for the first returned parameter before processing. This passed
through the worker protocol and VST2 setter. ReaComp advertises no VST2 program
chunks, so state save returns an explicit `UnsupportedFeature` response and
the worker remains usable; state chunk translation for chunk-capable plugins,
native editor containment, and plugin-reported latency are not yet qualified.

The ReaComp run now also reports its bounded `AEffect::initial_delay` through
the worker latency response and passes the existing latency bound. Native
editor containment remains open; no editor window was opened.

ReaGate independently passed the same opt-in worker acceptance. The worker
rejects a VST2 parameter event whose sample offset is outside the current
block before invoking the plugin setter; in-block VST2 events are applied at
the worker block boundary. Sample-accurate VST2 automation remains outside
this adapter evidence.

The worker `DescribeEditor` query passed for ReaComp and ReaGate, including the
bounded VST2 editor flag and preferred rectangle response. This is discovery
only: no HWND was supplied, no editor window was opened, and native UI-thread
containment remains unqualified.

## VST2 dispatcher opcode correction (2026-09-08)

The Windows VST2 adapter had used shifted dispatcher constants for setup:
sample-rate and block-size setup overlapped the VST2 chunk-state operations,
and mains lifecycle was also offset. The adapter now uses the VST2 2.4 values
`effSetSampleRate=10`, `effSetBlockSize=11`, and `effMainsChanged=12`; a Windows
unit test locks these values alongside the existing editor and chunk opcodes.

The complete ignored local ReaPlugs matrix was rerun individually after the
correction. `reacomp-standalone.dll`, `readelay-standalone.dll`,
`reaeq-standalone.dll`, `reafir_standalone.dll`, `reagate-standalone.dll`, and
`reaxcomp-standalone.dll` each passed the verified x64 worker acceptance:
contained load, parameter discovery and bounded automation, finite stereo
processing, editor-capability discovery, bounded latency, expected state
handling, and clean shutdown. This is stronger adapter/fixture evidence and
explains the prior four failures as an AudioRouter ABI bug, but it is not
blanket VST2 compatibility. No native editor HWND was created, no audio device
was opened, and the ignored DLLs were not committed.

## Per-binary worker failure diagnostics (2026-09-08)

`WorkerSupervisor` now retains the verified `PluginIdentity` alongside its
failure ledger and exposes the identity, failure count, and quarantine state as
a bounded diagnostic. A regression verifies that the canonical and binary paths
and SHA-256 fingerprint remain attached after a worker failure. This keeps
future crash, hang, invalid-sample, and layout reports attributable to the
exact binary under test; it does not relax quarantine or protected-voice
silence policy.

The public `supportedVst2X64Gated` compatibility result is also guarded by the
Windows target. Non-Windows discovery remains `unsupportedFormat`, matching the
platform-specific worker-start boundary; the plugin-host regression covers
both target branches.

The worker processing path now rejects any VST2 NaN/Inf output before
interleaving it into the protocol frame. The rejection is returned as a worker
failure so the supervisor can silence protected paths and count the binary
toward quarantine; no non-finite sample crosses the worker boundary.

The repeatable wrapper `tests/acceptance/m06-vst2-reaplugs.ps1` now runs the
same ignored worker acceptance once per local DLL and restores the caller's
fixture environment variable. It does not register plugins, open audio
devices, or include the ignored binaries in source or release artifacts.

The repository-owned `tests/fixtures/vst2-state-fixture.c` provides a minimal
stereo VST2 effect with one bounded parameter and `effFlagsProgramChunks`.
`tests/acceptance/m06-vst2-state-fixture.ps1` compiled it as an ignored x64 DLL
with the installed VS2026 toolchain and the verified worker test passed its
load, parameter, finite-processing, editor-capability, latency, opaque
state-save/restore, and shutdown checks. This is chunk-state contract evidence
without using a third-party binary for that capability; native editor
open/close and full rights/release review remain open.

The behavioral chunk regression also caught and fixed a real ABI argument bug:
`effSetChunk` now receives the actual bounded state length rather than a
hard-coded value. The fixture test proves changed mix output, restores the
saved opaque chunk, and then observes the original mix output again.

The adapter also caches the active VST2 processing format. Repeated blocks at
the same rate and size no longer invoke setup or mains lifecycle callbacks;
when a bounded format change is requested, the effect receives one
mains-off/reconfigure/mains-on transition. This reduces legacy-plugin lifecycle
reentrancy while keeping all calls on the contained worker thread.
The Windows unit regression verifies the exact dispatcher-call count for a
repeated format and one subsequent format change.

VST2 block handling now performs format setup before parameter setters, so
automation observes an initialized effect. The six ReaPlugs and repository
chunk-state fixture acceptance runs passed with this order.

The ABI layer now includes bounded native-editor open, close, and idle dispatch
primitives with one-editor-at-a-time state and close-before-effect teardown.
Focused Windows tests lock the VST2 editor opcodes and lifecycle state. No
editor window was created by this change; dedicated UI-thread ownership and
explicit parent authorization remain the integration gate.
Parent handles are now checked with Windows `IsWindow` before editor dispatch,
so an arbitrary nonzero integer cannot reach a plugin. This remains only a
primitive-level guard; cross-process authorization and the dedicated UI thread
are still required for actual editor integration.

The native layer now includes a disposable `Vst2EditorThread`. It owns a
separate VST2 instance, runs the Windows message pump, and serializes editor
open/close/idle calls away from the processing instance. Package tests and
strict Clippy pass. It is not wired to worker messages or a parent-window
authorization token, so no editor window was opened by this change.

The worker protocol now carries bounded editor open/close requests and returns
`editorUnavailable` as an optional unsupported feature when no VST2 editor
thread is present. The generic worker regression verifies that this response
does not terminate the worker or interrupt subsequent processing. Actual
authorized HWND integration remains open.

## Initial ReaPlugs compatibility inspection (2026-09-08)

The scanner now identifies the six x64 DLLs as `vst2` from the PE export
`VSTPluginMain`, while retaining `unsupportedFormat` compatibility. This is
format identity evidence only; no DLL was loaded or executed. The VST2 worker
adapter, rights review, and runtime compatibility matrix remain open under
`PLUG-07`. The CLI scan passed with six entries; plugin-host (47), DSP (28),
and strict package Clippy passed. No audio endpoint or machine configuration
was accessed.

The worker supervisor has an explicit `Vst2AdapterUnavailable` fail-closed
result for identified x64 VST2 binaries. It does not start a worker or execute
the DLL until the adapter is implemented and qualified. Plugin-host coverage
is 48 unit/integration tests plus 13 worker-process tests; strict Clippy and
formatting passed.

Six user-installed ReaPlugs effect DLLs were copied to the ignored repository
fixture directory for a read-only compatibility scan. The scanner identified
all six as x64 but classified them as `unsupportedFormat` with no VST3 class
IDs. A read-only PE export inspection found `VSTPluginMain` in each binary,
confirming the legacy VST2 entry-point boundary rather than a VST3 bundle. No
DLL was loaded or executed, and the files are not part of the source
tree or release artifacts. This confirms that the built-in DSP path—not a
VST2/standalone compatibility assumption—is the native basic-transformation
path. DSP coverage (27) and engine coverage (78) passed with strict Clippy;
no audio endpoint or machine configuration was accessed.
