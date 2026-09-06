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
