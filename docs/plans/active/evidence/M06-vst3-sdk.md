# M06 VST3 SDK boundary

## Runtime multi-bus quantum-shape hardening (2026-09-09)

`RuntimeBusGeneration::process` now validates that every present input bus and
every output bus belongs to the same exact frame quantum before mutating any
destination. A malformed auxiliary bus therefore returns `BlockShape` without
partially publishing the main or auxiliary output. The focused command
`cargo test -p audiorouter-engine --all-features --locked` passed 85 tests;
strict engine Clippy and `git diff --check` also passed. This is portable graph
contract evidence only; native VST3 worker execution and realtime scheduling
remain separate gates.

## Normal-build multi-bus owner API (2026-09-09)

The negotiated multi-bus `WorkerProcess` and `SupervisedWorkerProcess` paths
are now available in normal builds, including bounded deadline reads, result
validation, heartbeat updates only after success, and layout-preserving
restart. Fixture-only constructors and controlled hang modes remain gated to
test builds. `cargo test -p audiorouter-plugin-host --locked` passed 66 library
tests and 13 default worker-process tests; the all-features run passed 66
library tests and 29 worker-process tests with six expected fixture skips.
Strict Clippy passed in both feature modes. This makes the process-owner seam
available to a future native VST3 backend, but the current executable's
multi-bus path remains an echo implementation and is not native plugin
execution or realtime graph evidence.

## VST2 single-stream enforcement (2026-09-09)

The supervised multi-bus constructor now rejects a `PluginFormat::Vst2`
identity before executable validation or process launch. This prevents the
legacy VST2 adapter from being flattened into or accidentally presented as an
auxiliary-bus worker. The focused plugin-host library tests passed 67/67 in
both default and all-features builds, worker-process tests passed 13 default
and 29 all-features cases with six expected fixture skips, and strict Clippy
passed in both modes. VST2 native processing remains the existing bounded
single-stream `processReplacing` path.

The same rule is enforced at the supervised multi-bus process-owner launch
boundary, where a VST2 identity is rejected before executable validation or
spawn with `VST2 workers support only the single-stream protocol`. This closes
the format confusion path introduced when multi-bus ownership became available
in normal builds.

## Multi-bus parameter validation plumbing (2026-09-09)

Negotiated multi-bus quanta now validate parameter events against the current
quantum frame count and accept bounded events through the framed process path.
The fixture still echoes audio and does not apply events to a native effect.
Default and all-features plugin-host tests passed 67/67, worker-process tests
passed 13 default and 29 all-features cases with six expected fixture skips,
and strict Clippy passed in both modes. This removes a protocol-level blocker
for a future VST3 backend without claiming native parameter automation.

The locked all-features workspace was then requalified successfully, including
all crate integration tests and doc-tests. This confirms the normal-build
parameter API did not alter the existing VST2 single-stream or native-gated
boundaries.

## Native VST3 parameter-event processing (2026-09-09)

The native loader was rebuilt with VS2026/MSVC 14.51 and Windows SDK
10.0.28000.0 after adding bounded `IParameterChanges` and
`IParamValueQueue` host objects. It sends a normalized value of `0.5` at sample
offset zero for the first exposed controller parameter before processing. The
mda class-0 probe, AGain main class, and AGain side-chain class-2 probe with
`--multi-bus` all completed with finite output; the side-chain result reported
two input buses and one output bus. No audio device was opened and no machine
configuration changed. This is native offline parameter-delivery evidence, not
the supervised realtime VST3 worker or graph scheduler.

The complete guarded `tests/acceptance/m06-vst3-sdk.ps1` wrapper then passed
with the rebuilt probe: SDK/validator compilation, 51 validator self-tests,
mda and AGain validators, AGain main and side-chain processing, explicit
single-bus rejection, and the five-class mda matrix. The wrapper removed the
generated loader executable/object afterward. No plugin registration, audio
device, or machine configuration action occurred.

## 2026-09-09 - Deadline-bounded missing-result containment

The typed multi-bus worker client now limits response waiting to the quantum's
declared deadline, capped by the existing five-second IPC bound. A controlled
feature-gated worker that accepts a quantum and then emits no result returned a
bounded `WorkerMessageError::Io` at the 100 ms deadline; the worker was then
dropped and terminated by the test. The supervised expired-quantum regression
also remains fail-closed and records the worker failure. The feature-enabled
`worker_process` suite passed 26 tests with six expected fixture-dependent
skips; strict Clippy, formatting, and diff checks passed. This is containment
evidence only: no production VST3 worker, realtime callback, plugin
registration, audio stream, or machine audio configuration was used.

The supervised result handoff was also exercised end to end through caller-owned
staging blocks and `RuntimeBusGeneration`: both main and auxiliary output buses
were published with the original sequence identity. The feature-enabled
worker-process suite then passed 27 tests with six expected fixture-dependent
skips. This remains an echo-fixture integration check, not production VST3 or
realtime callback evidence.

The negotiated worker fixture also accepted an asymmetric two-input/one-output
layout matching the pinned AGain side-chain shape, retained the main input as
the declared output, and rejected output cardinalities it could not synthesize
safely. The feature-enabled worker-process suite passed 28 tests with six
expected fixture-dependent skips; strict feature-enabled and default workspace
Clippy passed. This does not claim that the fixture executes a native VST3
processor.

Startup validation also rejects a multi-bus layout whose output side has more
buses than the available input side, preventing the fixture from inventing
audio. The feature-enabled worker-process suite passed 29 tests with six
expected fixture-dependent skips, and documentation validation passed.

The guarded `safe-all.ps1` chain was requalified after this handoff work:
VS2026/WDK/native compile, read-only 31-endpoint inventory, disposable SysVAD
package/API/signability checks, M01/M04/M05, VST3 validators and auxiliary
probe, VST2 modern/legacy/fault fixtures, M07, unsigned M08 preparation,
traceability, and 51-file/163-link documentation validation passed. Strict
all-features and no-feature workspace Clippy also passed after test-only
symbols were correctly feature-gated.

The subsequent asymmetric-output regression also passed in the focused suite:
an output layout larger than the input layout is rejected during worker startup
instead of synthesizing an audio bus. The feature-enabled worker-process suite
remained at 29 passing tests with six expected fixture-dependent skips.

The complete guarded acceptance chain was requalified after these regressions:
the pinned VST3 validators and AGain auxiliary-bus probe passed alongside the
VST2 modern/legacy/fault matrix and all other M00-M08 checks. No driver,
plugin registration, audio stream, signing-mode, or persistent machine audio
configuration action occurred.

The available native VST2 qualification was also rerun with the installed
ReaPlugs directory: six local x64 audio effects passed isolated worker
load/process checks at 44.1, 48, and 96 kHz (18 combinations). The acceptance
wrapper restored both fixture-related environment variables and did not open
an audio stream or register plugins. This strengthens PLUG-07 compatibility
evidence but does not close rights, editor, or release gates.

The guarded M00–M08 chain was requalified at this tip as well. It passed the
native VS2026/WDK and read-only endpoint checks, portable workspace and UI
checks, pinned VST3 validator/auxiliary probe, VST2 modern/legacy/fault matrix,
M07, unsigned M08 preparation, 159 requirement mappings, and 51-file/163-link
documentation validation. This does not close the production VST3 worker or
realtime-driver gates.

`WorkerLatency::total_samples_with_pipeline` now provides the bounded accounting
seam for combining plugin-declared latency with a graph scheduler's fixed worker
pipeline delay. Tests cover normal addition, overflow, and the ten-second cap;
the helper does not infer physical or device latency. Plugin-host unit tests
(66), feature-enabled worker-process tests (29 with six expected fixture skips),
and strict Clippy passed.

The locked all-features workspace requalification also passed: all unit,
integration, and doc-tests across the workspace were green, including the
engine, plugin-host, worker-process, control, transport, and Windows-audio
coverage. Formatting, strict Clippy, and diff checks passed as well.

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

The loader accepts both established VST2 export spellings: preferred
`VSTPluginMain` and legacy `main`. The fallback uses the same ABI signature and
does not relax x64 identity verification, header validation, worker
containment, or cleanup. The existing ReaPlugs fixtures all use
`VSTPluginMain`; the scanner also recognizes a `main`-only binary as VST2
without loading it.

The repository-owned acceptance builds a second ignored x64 fixture with only
the legacy `main` export and successfully loads/processes it through the
verified worker. This is runtime fallback evidence, not third-party
compatibility evidence.

The same legacy-main fixture passes the chunk-state behavioral round trip, so
the fallback is verified for stateful processing rather than only loading and
finite audio output.

The fixture build also emits a deliberate non-finite-output variant. Its
verified worker acceptance receives a bounded `vst2Processing:NonFiniteOutput`
failure and records the fault without serializing invalid samples.

The native fixture also has deliberate crash and hang variants. Both passed
the supervised worker acceptance: the worker was contained and reaped within
the bounded policy, and the host retained a failure record without exposing
invalid audio.

A focused scanner regression also classifies a minimal x64 PE containing only
`main` as VST2, preventing the inspection boundary from regressing to the
preferred export name only.

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
strict Clippy pass. Worker messages now delegate to this owner, but the
control-plane parent-window authorization token is not implemented, so editor
controls remain gated.

The worker protocol now carries bounded editor open/close requests and returns
`editorUnavailable` as an optional unsupported feature when no VST2 editor
thread is present. The generic worker regression verifies that this response
does not terminate the worker or interrupt subsequent processing. Actual
authorized HWND integration remains gated by the missing control-plane
authorization token and native third-party editor behavior.

The editor-open wire contract now carries a bounded opaque authorization token
and expected owner PID with the parent HWND. The worker rejects malformed
authorization data, and the Windows editor thread checks the live HWND owner
PID before entering `effEditOpen`; a focused native regression rejects a
mismatched owner before plugin dispatch. `EditorParentAuthorizationIssuer`
derives the opaque token from a control-plane-held key, HWND, and owner PID;
key storage and native-shell issuance are not exposed by the preview WebView
and remain the integration gate.

## Native editor probe and ABI correction (2026-09-08)

The VST2 editor capability flag was corrected from bit 2 to the VST2 ABI's bit
0, with a Windows regression locking the flag values and dispatcher opcodes.
The correction made all six local ReaPlugs report editor capability. A hidden
parent probe then showed that each binary entered `effEditOpen` but failed to
return within the five-second dedicated-thread bound. The ignored acceptance
test records that bounded timeout and does not claim a successful native editor
window. This is a third-party editor-hosting compatibility blocker, not an
audio-device-use or endpoint-configuration failure; no persistent audio state
was changed.

The supervised-worker acceptance also passed against the ReaComp fixture: the
editor timeout terminated the contained worker and preserved a single failure
in the supervisor ledger. This confirms native editor hangs remain bounded at
the process boundary rather than becoming an audio-host hang.

The repeatable `tests/acceptance/m06-vst2-editor.ps1` wrapper now runs both
editor-thread and supervised-worker timeout checks for each local ReaPlugs
binary, restoring any pre-existing fixture variable. It records containment
evidence only; native editor-window compatibility remains unqualified.

The native processing boundary now independently rejects non-finite output
before returning from `process_replacing`; a focused Windows regression uses a
synthetic callback that writes NaN and verifies `NonFiniteOutput`. The worker's
existing pre-framing finite check remains defense in depth, preserving the
failure/quarantine and protected-voice silence policy.

That focused regression also supplies invalid-layout evidence: a native output
channel with a frame count different from the input is rejected before the
plugin callback is invoked.

`WorkerFailureDiagnostic` now also reports whether the latest failure was an
immediate worker fault or a heartbeat timeout. Focused supervisor regressions
cover both categories while retaining the verified plugin identity and
quarantine count.

The supervised wrapper now forwards this diagnostic snapshot directly to its
owner, and the process-level heartbeat regression verifies the timeout reason
through that public boundary.

The complete locked workspace regression then passed: workspace tests and
doc-tests, strict Clippy, formatting, diff checks, and documentation
validation. The VST2-specific package suite passed 56 unit tests and 20
worker-process tests; the six-fixture processing matrix passed independently.
No driver, plugin registration, audio stream, or machine audio configuration
was changed.

## Installed x64 VST2 fixture and x86 negative control (2026-09-08)

Read-only CLI inspection classified the explicitly selected installed
`C:\\Program Files\\Common Files\\VST3\\Pitchproof\\pitchproof-x64.dll` as
an x64 VST2 binary, despite its VST3-named directory. Its 1,077,760-byte
SHA-256 is
`1974a3033b53ae72da5f419a9f37056d44c1610591bfd11a615ace0c448cf050`.
The disposable worker acceptance passed parameter/state-capability,
finite-processing, latency, editor-capability, and shutdown checks against
the original installed file. The wrapper restores the prior fixture
environment variable and performs no copy or registration.

The sibling `pitchproof.dll` was rejected by the same read-only inspection as
`unsupportedArchitecture` and was never loaded. This confirms that the x64
VST2 gate does not bridge or execute x86 binaries. These results are
fixture-specific compatibility evidence, not a rights determination or a
release qualification; native editor-window integration and the broader
rights/compatibility matrix remain open.

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
## Optional native VST2 matrix requalification (2026-09-08)

At pushed head `2905572a`, all six ignored local ReaPlugs x64 VST2 effects
passed verified worker processing at 44.1, 48, and 96 kHz (18 combinations).
The installed Pitchproof x64 VST2 binary also passed processing at all three
rates, followed by dedicated and supervised editor-containment tests. Its
SHA-256 remained
`1974a3033b53ae72da5f419a9f37056d44c1610591bfd11a615ace0c448cf050`.
The wrappers restored both VST2 environment variables and did not copy,
register, or alter plugins or audio configuration. This is compatibility and
containment evidence only; rights, successful editor integration, and release
qualification remain open.
## Installed plugin inventory follow-up (2026-09-08)

A read-only inventory of the available Windows plugin locations found the six
ReaPlugs effect binaries already used by the local VST2 matrix and the
installed Pitchproof x64 VST2 binary. The remaining ReaPlugs DLLs are
standalone/MIDI/JS assets rather than an additional rights-cleared native
audio-effect vendor fixture; the x86 Pitchproof sibling remains an explicit
negative control. No second-vendor VST2/VST3 fixture was available for this
run, and no plugin was copied, registered, loaded, or modified.

This confirms that the next independent-fixture gate depends on a user-supplied
rights-cleared x64 VST2 or VST3 binary. Until then, the existing worker,
failure-containment, and multi-rate evidence remains valid but cannot be
promoted to broad compatibility or release qualification.
## All-features workspace requalification (2026-09-08)

`cargo test --workspace --all-features --locked` passed 466 unit/integration
tests and all workspace doc-tests at pushed head `479a7ad7`. The feature
enabled worker-process suite passed 21 tests with six expected native-fixture
tests ignored, and the Windows-audio suite passed 32 tests. This confirms the
VST2 worker, failure-containment, and Windows identity contracts compile and
remain green together. No plugin registration, audio stream, driver, signing
mode, or machine configuration changed.

## Current installed-fixture inventory (2026-09-08)

A read-only recursive inventory of the common Windows VST2/VST3 directories
found 11 DLL candidates: the known ReaPlugs standalone/MIDI/utility assets and
the already-qualified Pitchproof x64/x86 pair. No additional x64 VST3 bundle or
second-vendor x64 audio-effect fixture was available. No plugin was loaded,
copied, registered, or modified, and no audio or machine configuration changed.
The independent-fixture gate therefore remains externally blocked.

## AGain lifecycle compatibility and independent-fixture follow-up (2026-09-09)

The canonical Steinberg `AGain` sample from the pinned SDK was built as an
x64 repository-local bundle with plugin-link creation disabled. The official
validator reported 94 tests passed and 0 failed. AudioRouter's native offline
loader initially received `0x80004001` (`E_NOTIMPL`) from
`IAudioProcessor::setProcessing(true)`; the SDK's `AudioEffect` base
implementation returns that result when the hook is not overridden, and the
SDK processing tests do not treat it as a processing failure. The loader now
accepts only `kNotImplemented` for that lifecycle call and still requires
successful processing, finite output, parameter automation, and state
round-trip. AGain's main stereo effect class passed all of those checks.

AGain's side-chain class was intentionally not counted in the one-input/
one-output probe because its bus layout is different and was rejected before
processing. The existing mda validator and five-class matrix passed after the
loader change. A read-only inventory of the checked machine roots found no
independent VST3 bundle; no plugin was registered, copied, or loaded beyond
the explicitly selected offline fixtures, and no audio configuration changed.
The independent rights-cleared fixture, broader bus-layout, editor, dynamic
latency, and release gates remain open.

The acceptance harness was then corrected for the installed Windows
PowerShell/.NET runtime, which promotes native stderr to a terminating error
when a nonzero command is captured. It now uses an explicit bounded process
capture for the intentional AGain side-chain rejection. The M06 VST3 script
passed with exit code 0: both official validators, AGain main-class processing,
the specific one-input/one-output rejection, and the five-class mda matrix all
passed. Generated loader outputs were removed; no system plugin link,
registration, audio stream, or machine configuration was changed.

## Bounded multi-bus control-plane contract (2026-09-09)

`WorkerAudioBusLayout` now records a bounded effect topology without changing
the existing single-stream worker wire format. It requires a main input and
output, permits at most four buses per direction, limits each bus to mono or
stereo, and caps aggregate channels at eight per direction. Three plugin-host
regressions cover a valid main-plus-side-chain layout, missing-main and
unbounded/invalid shapes, and preservation of the current single-stream
boundary. The plugin-host suite passed 60 unit tests, 21 worker-process tests,
doc-tests, formatting, and strict Clippy. Actual side-chain transport and
graph ownership remain intentionally open.

`WorkerAudioBusFrames` extends that boundary with one validated quantum per
declared bus. It rejects missing or extra buses, channel mismatches, unequal
frame counts, and mixed sequence/deadline identities before a future
multi-bus worker message can be serialized. The plugin-host suite passed 61
unit tests, 21 worker-process tests, doc-tests, formatting, and strict Clippy.
The current runtime and single-stream wire path are unchanged; actual
side-chain scheduling and shared-memory ownership remain open.

`SharedAudioBusTransport` now owns one explicit mapped slot per declared input
or output bus. Creation/opening requires caller-supplied absolute non-reparse
paths with bounded cardinality; writes validate the complete bus set before
publishing each slot, and reads return `MissingBus` or an incoherence error
until all slots share identity and quantum shape. A transport regression
round-trips a main-plus-side-chain set in both directions and verifies the
pre-publication missing-bus and alias boundaries. The transport is not yet
wired into `WorkerProcess` or realtime graph scheduling.

The worker protocol now carries bounded `ProcessBuses` and `ProcessedBuses`
message shapes. Encode/decode validation rechecks the serialized bus layout,
requires one valid frame per declared bus, and rejects mismatched channels,
quantum sizes, or sequence/deadline identity. A regression round-trips a
main-plus-side-chain request and response and rejects a misaligned request.
The current worker session and shared-memory runtime remain single-stream, so
these messages are a transport contract only and do not enable side-chain
execution.

`WorkerBusSession` now provides the matching state boundary for the new
messages. It binds the expected plugin fingerprint and exact bus layout during
`HelloBuses`/`Ready`, then accepts a `ProcessBuses` request only when every
declared input bus is coherent and within the frame deadline. Existing
single-stream `WorkerSession` behavior is unchanged. A handshake regression
passed with the plugin-host suite at 63 tests, alongside 21 worker-process
tests, doc-tests, formatting, and strict Clippy. Fixed shared-memory bus-slot
ownership and actual side-chain scheduling remain open.

`RuntimeBusLayout` and `RuntimeBusGeneration` add the graph-owned staging
boundary without importing worker implementation types into the engine. A
nonzero generation binds the bounded mono/stereo layout; processing validates
caller-owned blocks without allocation, passes through only the main bus, and
silences every output when the required main input is absent. Engine tests
passed 83 cases with formatting and strict Clippy. Auxiliary effect execution
and sequence/deadline result handoff remain open by design.

The multi-bus transport `open` path also now compares canonical paths and
native file identity for every existing slot pair, closing the hard-link alias
case already covered by the single-stream transport. The plugin-host suite
passed 64 unit tests, 21 worker-process tests (six fixture-dependent tests
ignored), doc-tests, formatting, and strict Clippy.

The engine now accepts a caller-owned `RuntimeBusWorkerResult` at that
generation boundary. `RuntimeBusQuantumIdentity` carries sequence, deadline,
and frame count; a result with a late/different identity or any missing bus
clears every destination and reports an explicit silence outcome. A matching
complete result copies all declared outputs without allocation, locking,
waiting, or I/O. Engine tests passed 84 cases with formatting and strict
Clippy. The engine remains independent of plugin-host, so this is the typed
handoff contract rather than a claim of completed auxiliary effect execution.

The result boundary was hardened after review found that source shapes were
not all checked before copying. `RuntimeBusGeneration::accept_worker_result`
now validates every present source bus before mutating any destination; the
regression proves a malformed later bus leaves earlier destinations unchanged.
Engine tests remain at 84 cases with formatting and strict Clippy passing.

`plugin-host` now depends one-way on `engine` and exposes
`stage_engine_worker_result`. It copies validated `WorkerAudioBusFrames` into
caller-prepared `AudioBlock` storage and caller-prepared reference slots,
constructing the engine identity envelope without allocating at the handoff.
The integration regression verified sequence/deadline/frame-count preservation
and successful delivery of main plus auxiliary outputs. Plugin-host passed 65
unit tests, 21 worker-process tests (six fixture-dependent tests ignored),
doc-tests, formatting, and strict Clippy. No realtime callback, plugin
registration, audio stream, or machine configuration was used.

The worker executable now has a separately negotiated `--input-buses` /
`--output-buses` fixture mode. It emits `HelloBuses`, requires `Ready`,
validates complete `ProcessBuses` sets, and echoes them as `ProcessedBuses`
only when the declared input/output layouts are symmetric. A Windows process
regression exercised the actual framed stdin/stdout executable path, including
main-plus-auxiliary identity preservation and clean shutdown: 22 worker-process
tests passed and six fixture-dependent tests were ignored. This is protocol
execution evidence only; it does not load VST2 or claim auxiliary effect
processing. The fixture now applies the same monotonic sequence and deadline
guard as the single-stream worker before echoing a result. The production VST2
path remains single-stream.

The process regression also sends an expired multi-bus quantum. The fixture
returns a bounded `Failure` with a `multiBusIdentity` code and exits
non-successfully rather than producing stale audio. This verifies the worker
side of the late-result fail-closed boundary; supervised restart/quarantine
policy and a real auxiliary-bus effect remain separate gates.

## Multi-bus session result supervision (2026-09-09)

`WorkerBusSession` now records the exact identity of its outstanding
`ProcessBuses` quantum. `accept_result` accepts only a matching
`ProcessedBuses` layout and sequence/deadline/frame-count identity; unsolicited,
misidentified, and malformed results are rejected before they can reach
graph-owned storage. `expire_pending_result` drops an overdue pending identity
so a late response cannot be paired with a newer quantum. A focused regression
covers result mismatch, duplicate/unsolicited response, deadline expiry, and
late-response rejection. Plugin-host passed 66 unit tests, 13 ordinary
worker-process tests, doc-tests, formatting, and strict Clippy. This is a
protocol/session supervision boundary only: it does not claim a real auxiliary
effect, realtime scheduling, or VST2 side-chain support.

## Native VST3 auxiliary-bus activation (2026-09-09)

The offline loader now has an explicit `--multi-bus` mode. It validates and
activates every declared mono/stereo audio bus (bounded to four per direction),
supplies all buffers to `IAudioProcessor::process`, and checks every output for
finite samples. The default probe still rejects non-single-bus effects. Against
the pinned SDK's AGain class 2 (`AGain SideChain VST3`), the new mode processed
a genuine two-input/one-output layout successfully; the single-bus invocation
continued to fail with the expected layout diagnostic. The M06 acceptance also
passed the pinned validators, main AGain probe, mda matrix, and documentation
checks. This is native offline VST3 effect evidence, not worker/realtime
scheduling or VST2 side-chain evidence.

The Windows process regression now drives the same `WorkerBusSession` state
machine around the framed fixture: it accepts Hello/Ready, records the
outstanding request, validates the returned `ProcessedBuses`, and verifies
that the expired-quantum path remains a bounded worker failure. The
feature-enabled worker-process suite passed 22 tests with six expected
fixture-dependent tests ignored; no realtime callback or machine audio path
was used.

The fixture-gated `SupervisedWorkerProcess` path now owns a multi-bus worker
using the same heartbeat, immediate-failure, restart, and quarantine ledger as
the legacy worker. It refreshes heartbeat only after `WorkerProcess` returns a
validated complete bus set, terminates and records protocol failures, and
restarts with the exact negotiated layout while retaining failure history. A
Windows feature-enabled regression passed 24 worker-process tests with six
expected fixture-dependent tests ignored. This is supervised fixture evidence;
the production VST3 plugin loader and realtime graph scheduler remain separate
gates.

After the supervised multi-bus lifecycle change, `cargo test --workspace
--all-features --locked` passed all workspace unit/integration tests and
doc-tests, including 84 engine, 98 control, 33 Windows-audio, and 30
feature-enabled plugin-host worker-process cases (six fixture-dependent tests
ignored). Strict all-target Clippy with `-D warnings`, formatting, and diff
checks also passed. No native driver or machine audio configuration action was
performed.

The complete guarded `tests/acceptance/safe-all.ps1` chain was requalified at
the supervised-worker implementation head on 2026-09-09. VS2026/MSVC/SDK/WDK
discovery and native compile, read-only 31-endpoint inventory, disposable
pinned SysVAD x64 package/API/signability checks, M01/M04/M05, VST3 validators
and auxiliary-bus probe, VST2 modern/legacy/fault fixtures, M07, unsigned M08
artifacts, 159 traceability mappings, and documentation validation (51 files,
163 links) all passed. Temporary checkouts and artifacts were cleaned; no
driver, signing, plugin registration, audio stream, or persistent machine
configuration action occurred.

The supervised process regression also sends an expired multi-bus quantum
through the typed owner. The worker returns the bounded `multiBusIdentity`
failure, the owner transitions to `Failed` and terminates the process, and its
diagnostic retains the verified plugin hash and one failure count. The
feature-enabled worker-process suite passed 25 tests with six expected
fixture-dependent tests ignored. This verifies propagation of deadline failure
into supervision; it does not claim realtime callback behavior.

The plugin-host API now exposes a fixture-gated `WorkerProcess` multi-bus
client. It launches the separately negotiated bus worker, validates the exact
`HelloBuses` layout, exchanges a complete `ProcessBuses` quantum, validates
the `ProcessedBuses` output, and performs bounded shutdown. A Windows
feature-enabled process regression passed 23 tests with six expected
fixture-dependent tests ignored; ordinary single-stream constructors remain
unchanged. This is a production-shaped process-owner contract, not yet the
production VST3 plugin worker or realtime scheduling path.

## Guarded acceptance after native parameter delivery (2026-09-09)

The complete guarded `tests/acceptance/safe-all.ps1` chain passed after the
native offline loader began sending a bounded `IParameterChanges` event:
VS2026/MSVC/SDK/WDK discovery and native compile, read-only inventory of 31
audio endpoints, disposable pinned SysVAD x64 qualification, M01/M04/M05,
VST3 validator and AGain auxiliary-bus/parameter probes, VST2 modern/legacy/
fault fixtures, M07, unsigned M08 artifacts, 159 traceability mappings, and
documentation validation covering 51 Markdown files and 163 local links.
The wrapper cleaned temporary outputs. It did not install or load a driver,
register a plugin, change signing mode, open an audio stream, or persist a
machine audio configuration.

## Native transformation assertion (2026-09-09)

The loader now has an opt-in `--require-output-change` check and a bounded
`--parameter-value` option. The AGain main-class probe passed with its first
parameter (`Gain`) set to normalized 0.75 and confirmed that at least one
finite output sample differed from the 0.25 probe input. This supplements
parameter-interface evidence with a real repository-local sound-transformation
check; it is still offline evidence and does not claim supervised realtime
hosting. Generated native artifacts were removed after the run.

## Native single-stream VST3 worker (2026-09-09)

`tools/m06-vst3-worker` now loads a verified x64 VST3 bundle in a separate
Windows process and speaks the existing length-prefixed JSON worker protocol.
The supervised Rust launch forwards the plugin path for VST3 identities, and
the end-to-end AGain regression completes Hello/Ready, sends a normalized
parameter events at sample offsets 0 and 64, processes a 128-frame stereo block, and
accepts finite output that differs from the input before bounded shutdown.
The worker intentionally supports only one input and one output bus; auxiliary
buses remain on the separately negotiated contract. This is worker-process
evidence, not realtime graph scheduling or physical-latency evidence.

## Native auxiliary-bus VST3 worker (2026-09-09)

The worker now selects a native effect whose initialized bus counts match the
requested bounded layout, activates each declared bus, and processes the
existing `HelloBuses`/`ProcessBuses` protocol. The supervised end-to-end test
uses AGain's actual `[stereo main, mono side-chain]` input layout and stereo
output: coherent two-bus input reaches `IAudioProcessor::process`, the output
bus is finite and differs from the main input, and the worker shuts down cleanly.
A channel mismatch is rejected before activation. This proves native
auxiliary-bus worker execution, not realtime graph scheduling, physical-latency
performance, or release rights.

The same acceptance now stages the native output into caller-prepared engine
blocks and publishes it through `RuntimeBusGeneration`, preserving the worker
sequence/deadline identity. This confirms the production-shaped worker-to-graph
ownership handoff; it does not prove callback scheduling, physical latency, or
long-run quarantine behavior.

The bounded `SupervisedBusWorkerLoop` now exercises that handoff asynchronously
with the fixture worker: the callback-facing side submits only preallocated
bus slots, while the owner thread performs IPC and sample conversion. The
fixture regression passed with clean shutdown and output publication. This is
worker-thread scheduling evidence; native callback timing, soak, and physical
latency remain open.

The same asynchronous path was then run with the native AGain side-chain
worker. A bounded normalized gain event was delivered on the owner thread, and
the `[stereo, mono]` input produced finite output different from the main input
through the graph scheduler. The opt-in Windows acceptance passed and restored
its environment. Crash/restart recovery, callback timing, soak, and physical
latency remain open.

The owner failure path was also exercised with the controlled hanging worker:
the quantum deadline terminated the exchange, the owner latched failure, and a
missing output was converted to scheduler silence. The regression completed in
120 ms. Deliberate restart/quarantine integration and native callback timing
remain open.

The native single-stream acceptance also records deliberate replacement: after
the first AGain worker produced finite transformed output, the test marked the
worker failed, restarted it through `SupervisedWorkerProcess::restart`, and
processed another transformed frame. The replacement retained the exact
verified bundle path rather than falling back to the generic echo worker. The
focused native acceptance passed; automatic restart policy, quarantine
integration, callback timing, soak, and physical latency remain open.

The asynchronous owner now accepts a caller-selected finite restart budget,
bounded to two replacements and defaulting to zero. A replacement is attempted
only after the supervised process reports failure and is created through the
existing `restart` path, preserving plugin identity and the failure ledger;
exhaustion or quarantine leaves the owner failed and the graph scheduler's
missing result fail-closed. The all-features plugin-host suite passed 67
library and 32 worker-process tests (nine expected skips), strict Clippy and
formatting passed, and the native M06 acceptance passed. Repeated native fault
soak, callback timing, and physical latency remain open.

The controlled hanging multi-bus fixture then submitted three sequential
quanta through the one-slot scheduler. Two bounded owner-thread replacements
were attempted with the fixture mode preserved; the third failure reached the
existing three-failure quarantine threshold. The owner exposed terminal
failure, and the final missing result was published as scheduler silence. This
is deterministic containment evidence, not a native third-party fault soak.

The owner timing regression then crossed 16,384 consecutive `[stereo, mono]`
fixture quanta through the fixed two-slot scheduler. Every output was finite,
the worker remained healthy, and the measured per-quantum staging/worker round
trip stayed below 100 ms in the guarded run. This is short worker-thread
evidence only; it does not qualify the realtime callback, the NFR-11 eight-hour W2
soak, or physical latency.

The guarded ReaPlugs VST2 matrix was also rerun after the worker changes. All
six supplied x64 effects passed isolated processing at 44.1, 48, and 96 kHz
(18 runs), and the wrapper restored `AUDIOROUTER_VST2_FIXTURE` and
`AUDIOROUTER_VST2_SAMPLE_RATE`. This remains local compatibility evidence;
rights, native editor, and release gates remain open.

The guarded `tests/acceptance/safe-all.ps1` chain was rerun after this policy
change and passed its native toolchain/endpoint and disposable SysVAD checks,
portable M01/M04/M05/M07 checks, native VST3 and available VST2 fixture
coverage, unsigned M08 preparation, 159 requirement mappings, and 51-file/
163-link documentation validation. Temporary outputs were removed; driver
installation/loading, signing-mode changes, plugin registration, audio streams,
and persistent machine audio configuration remained excluded.

The full guarded `tests/acceptance/safe-all.ps1` chain was rerun from clean
commit `eb0ad978` with this native worker acceptance included. Native
toolchain/endpoint checks, disposable SysVAD qualification, portable/UI
milestones, VST3/VST2 fixture coverage, M07, unsigned M08 preparation, 159
traceability mappings, and documentation checks all passed. Realtime graph
scheduling, physical-latency, production signing, and release-rights gates
remain open.

## VST2 replacement state coverage (2026-09-09)

The VST2 state-fixture acceptance now deliberately fails each worker, replaces
it through the supervised owner, restores the saved chunk through
`restart_with_state`, and verifies the restored parameter for both the modern
`VSTPluginMain` and legacy `main` export fixtures. This extends state evidence
across process replacement without changing the VST2 single-stream boundary.

## Native VST3 replacement state coverage (2026-09-09)

The real AGain single-stream acceptance now saves its opaque state after native
parameter automation, deliberately marks the supervised worker failed, and
uses `restart_with_state` to restore the saved asset before processing the next
frame. The replacement produced finite transformed output without a second
automation event, proving native state continuity across the verified worker
boundary. This remains offline worker-process evidence; native editor,
realtime callback, soak, physical-latency, and independent-plugin gates remain
open.

The first attempt exposed that the native worker had no state-message branch:
the Rust client correctly sent `StateSave`, while the worker treated it as an
invalid process request and closed its pipe. The fix adds a bounded local
`IBStream`, component and edit-controller `getState`/`setState` calls, a
versioned envelope for both streams, CNG SHA-256 calculation, and
`StateSave`/`StateRestore` JSON framing. Component-only assets remain accepted
for compatibility. The corrected native M06 acceptance
passed AGain state restoration across replacement; the generated executable and
object were removed afterward. This is worker-process evidence only, not
realtime callback or physical-latency qualification.

## VST2 intra-block parameter timing (2026-09-09)

The native x64 VST2 worker now sorts validated parameter events and splits each
block at event offsets, applying changes between `processReplacing` segments.
This prevents a nonzero sample offset from being silently moved to the block
boundary. The ReaPlugs acceptance matrix exercises offsets 0 and 64 in a
128-frame block for all six local effects at 44.1, 48, and 96 kHz (18 isolated
worker runs); finite output passed and the fixture environment was restored.

The complete guarded M00-M08 acceptance chain was then rerun after this worker
change. Native toolchain/endpoint checks, disposable SysVAD qualification,
portable and UI milestones, VST3, VST2 modern/legacy/state/fault fixtures,
M07, unsigned M08 preparation, 159 traceability mappings, and documentation
validation all passed. No driver installation/loading, plugin registration,
audio stream, signing-mode change, or persistent machine audio configuration
was performed.

## Validated state restoration across worker replacement (2026-09-09)

`SupervisedWorkerProcess::restart_with_state` now provides a control-plane
replacement operation that preserves the verified plugin path, then restores a
caller-owned `PluginStateAsset` only after version, size, and hash validation.
If restoration fails, the replacement supervisor is returned so failure and
quarantine accounting is not silently discarded. The fixture regression seeded
opaque bytes, deliberately failed the worker, restarted it with version 7, and
verified that the exact bytes could be saved again. A companion regression
confirmed that an invalid version is rejected before replacement spawn and
preserves the existing failed supervisor ledger. The all-features host suite
passed 67 library and 34 worker tests with nine expected fixture-dependent
skips; strict Clippy and documentation validation passed. This is fixture and
control-plane evidence only; native third-party state/editor, callback-timing,
and physical-latency gates remain open.
