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
