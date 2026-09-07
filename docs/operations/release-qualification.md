# Release qualification and recovery checklist

AudioRouter is not a releasable Windows installer yet. The portable control
plane, CLI, MCP adapter, DSP, recording, plugin-worker, and crash-recovery
orchestration boundaries are implemented and tested, but native routing, the
owned virtual-device driver, production signing, packaging, and clean-machine
qualification remain open.

## Current verified artifacts

The reproducible preparation flow is:

    powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\release\prepare-artifacts.ps1 -OutputDirectory <new-absolute-directory>
    powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\release\verify-artifacts.ps1 -ManifestPath <prepared-directory>\release-manifest.json

Preparation requires a clean worktree, locked Cargo inputs, and a new output
directory. It produces unsigned x64 CLI and disposable-worker artifacts,
locked Cargo SBOM metadata, and checksums. It does not produce an installer or
install a driver.

## Current qualification snapshot

At the current repository revision, the safe, repository-local qualification
surface is green:

- The locked Rust workspace passes 379 tests across all targets, formatting,
  and strict workspace Clippy.
- M04 passes 25 DSP and 30 recording tests, including the 60-second pitch
  boundary cases.
- M05 passes TypeScript typecheck, 76 UI tests, and a disposable production
  build.
- M06 passes with the pinned local VST3 SDK: 51 SDK self-tests, 1,598 official
  validator tests with zero failures, and the offline native loader.
- M07 passes 25 CLI tests, MCP stdio/named-pipe interoperability, 83 control
  tests, 33 plugin-host tests, 8 worker-process tests, and strict Clippy.
- M08 unsigned artifact preparation, provenance/SBOM, checksums, exact-content
  verification, and cleanup pass in a disposable output directory.
- M00 native validation is compile-only on this machine. Visual Studio
  Community 2026, MSVC 14.51.36231, Windows SDK 10.0.28000.0, and WDK
  10.1.28000.2526 are available; the SysVAD sample also passes x64 package/API
  validation through the 64-bit MSBuild host. No probe executable or driver is
  run by this gate.

The VST3 SDK is source-distributed and installed only at the ignored local
path `third_party/vst3sdk`; it is not a system SDK or plugin registration.
These checks do not establish native end-to-end routing, a production driver,
production signing, an installer, or clean-machine qualification.

## Before any installation

1. Back up the SQLite database and recording files to separate, new
   destinations using the headless runbook.
2. Stop AudioRouter sessions through the authorized control surface.
3. Confirm the package, driver, and configuration versions are compatible.
4. Review the install preview and the required elevation scope.
5. Keep unrelated audio drivers and devices out of the change set.

The current repository has no production installer or owned virtual-device
driver to install. Do not treat VB-Audio, Voicemeeter, Sonar, or another
existing virtual device as an AudioRouter release artifact.

## Recovery and uninstall expectations

If a future installer fails, retain the SQLite database and recordings, restore
only to a new validated database destination, and do not delete unrelated
audio devices. A future uninstall must preview owned endpoints, startup
registration, privileged helpers, and control-pipe cleanup separately from
recording retention. Permanent recording deletion is never an implicit
uninstall action.

## Known release blockers

- Production-signed virtual-device driver and normal Secure Boot/Memory
  Integrity qualification.
- Native end-to-end routing, latency, drift, restart, and hardware evidence.
- Signed binaries/packages, installer elevation behavior, upgrade/rollback, and
  clean-machine testing.
- Full plugin worker sandbox enforcement and the tested compatibility matrix.
- Accessibility/usability and first-time-user qualification on the declared
  reference hardware.

See the [headless runbook](headless-runbook.md), [SDK setup](sdk-setup.md), and
[M08 evidence](../plans/active/evidence/M08-release.md) for commands and
measured boundaries.
