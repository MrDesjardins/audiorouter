# Release qualification and recovery checklist

AudioRouter is not a releasable Windows installer yet. The portable control
plane, CLI, MCP adapter, DSP, recording, plugin-worker, crash-recovery
orchestration boundaries, and the shell-owned control backend are implemented
and tested, but native routing, the owned virtual-device driver, production
signing, and clean-machine qualification remain open.

## Current verified artifacts

The reproducible preparation flow is:

    powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\release\prepare-artifacts.ps1 -OutputDirectory <new-absolute-directory>
    powershell -NoProfile -ExecutionPolicy Bypass -File .\tools\release\verify-artifacts.ps1 -ManifestPath <prepared-directory>\release-manifest.json

Preparation requires a clean worktree, locked Cargo inputs, and a new output
directory. It produces unsigned x64 CLI, native-shell, and disposable-worker
artifacts, the disposable VB-Cable desktop launcher, a disposable zipped UI bundle, locked Cargo SBOM metadata, the authoritative UI
`package-lock.json` plus a deterministic CycloneDX npm SBOM generated from the
lockfile, and checksums. Before copying the three executables, preparation reads
bounded DOS/PE headers and rejects malformed, reparse-point, or non-x64 files.
The standard preparation flow does not install a
driver or generate an installer. An optional transient unsigned NSIS smoke can
exercise the native bundler without installation:

    npm.cmd exec --yes --package @tauri-apps/cli@2.11.4 -- tauri build --debug --no-sign --ci --bundles nsis --config src-tauri/tauri.conf.json

That smoke writes only to the native Cargo target directory and must be
cleaned afterward; it is not production signing, installer, or clean-machine
qualification.

The checked-in wrapper `tests/acceptance/m08-installer-smoke.ps1` runs this
smoke, verifies that Tauri does not modify `src-tauri/Cargo.toml`, and removes
its exact disposable NSIS bundle directory on completion.

## Current qualification snapshot

At the current repository revision, the safe, repository-local qualification
surface is green:

- The locked Rust workspace passes the current workspace suites and all doc-tests,
  formatting,
  and strict workspace Clippy.
- M04 passes 34 DSP and 40 recording tests, including the 60-second pitch
  boundary cases.
- M05 passes TypeScript typecheck, 234 UI tests, and a disposable production
  build.
- M06 passes with the pinned local VST3 SDK: 51 SDK self-tests, 1,598 official
  validator tests with zero failures, and the offline native loader.
- M07 passes 36 CLI tests, 3 MCP stdio tests, 173 control tests (2 guarded live
  tests ignored), 70 plugin-host tests, 13
  worker-process tests, 26 shell tests, and strict Clippy, including durable
  shell safe-mode supervision.
- M08 unsigned artifact preparation, provenance/SBOM, checksums, exact-content
  verification, and cleanup pass in a disposable output directory.
- M00 native validation includes read-only endpoint-format inventory and
  compile-only probe checks on this machine. Visual Studio Community 2026,
  MSVC 14.51.36231, Windows SDK 10.0.28000.0, and WDK 10.0.28000.0 are
  available; the SysVAD sample also passes x64 package/API validation through
  the 64-bit MSBuild host. No stream, probe runtime, or driver is run by this
  gate.

The VST3 SDK is source-distributed and installed only at the ignored local
path `third_party/vst3sdk`; it is not a system SDK or plugin registration.
The documented unsigned NSIS smoke also produces and verifies a debug x64
installer bundle without installing it; the output is removed afterward. These
checks do not establish native end-to-end routing, a production driver,
production signing, or clean-machine qualification.

## Before any installation

1. Back up the SQLite database and recording files to separate, new
   destinations using the headless runbook.
2. Stop AudioRouter sessions through the authorized control surface.
3. Confirm the package, driver, and configuration versions are compatible.
4. Review the install preview and the required elevation scope.
5. Keep unrelated audio drivers and devices out of the change set.

The current repository has no production installer or production-signed
virtual-device driver package to install. It does contain an AudioRouter-owned
x64 prototype and a guarded, build-qualified lifecycle entrypoint for an
isolated test system; neither is a releasable driver artifact. Do not treat
VB-Audio, Voicemeeter, Sonar, or another existing virtual device as an
AudioRouter release artifact.

The current qualification workstation reports Secure Boot enabled and VBS/
Memory Integrity active. WDK `signtool.exe` is available under the installed
Windows SDK, but read-only verification of the generated x64 and ARM64
prototype `.sys` files reports `No signature found`. The packages therefore
remain unsigned development artifacts; certificate submission, production
catalog signing, and clean-machine installation evidence are still required.

## Recovery and uninstall expectations

If a future installer fails, retain the SQLite database and recordings, restore
only to a new validated database destination, and do not delete unrelated
audio devices. A future uninstall must preview owned endpoints, startup
registration, privileged helpers, and control-pipe cleanup separately from
recording retention. Permanent recording deletion is never an implicit
uninstall action.

For the current prototype-only driver lifecycle, use the guarded
`drivers/audiorouter-virtual/manage.ps1` entrypoint only on an isolated test
system. Keep its exact published-package state file with the package; the
script refuses ambiguous cleanup and attempts compensating package removal if
it cannot publish that state after installation. This prototype procedure is
not a substitute for the future signed installer and does not authorize
changes to test-signing, Secure Boot, HVCI, endpoint defaults, or volumes.

The non-mutating preview is available before that isolated operation:

```powershell
.\drivers\audiorouter-virtual\manage.ps1 -Install -Preview -Inf .\path\to\AudioRouterVirtual.inf
```

It emits a bounded JSON plan and does not invoke `pnputil`. The plan includes
the validated driver-binary and catalog paths, and refuses an incomplete,
reparse-point, or oversized package before describing installation. A preview
of an uninstall similarly reports the tracked package or the fail-closed
blocker.

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
