# M08 release evidence

## 2026-09-08 - Complete safe acceptance at `2c8ab54d`

The elevated `tests/acceptance/safe-all.ps1` chain passed at the pushed head:
native toolchain discovery/compile and read-only 34-endpoint format inventory,
disposable pinned SysVAD x64 compile/package/API qualification, M01 CLI, M04
DSP/recording, M05 UI, M06 SDK/VST3, M07 headless, unsigned M08 preparation,
158 normative requirement mappings, and documentation validation (51 Markdown
files, 160 local links). Temporary outputs and the reference checkout were
removed. No driver was installed or loaded, no signing or registration changed,
and no machine audio configuration was modified.

## 2026-09-08 - Complete safe acceptance at `c4995aae`

The elevated `tests/acceptance/safe-all.ps1` chain passed at the current head:
toolchain discovery, native compile, read-only 34-endpoint format inventory,
disposable pinned SysVAD x64 compile/package/API qualification, M01 CLI, M04
DSP/recording, M05 UI typecheck/tests/temporary production build, M06 SDK/VST3,
M07 headless, unsigned M08 preparation, 158 normative requirement mappings,
and documentation validation (51 Markdown files, 160 local links). Temporary
artifacts and the reference checkout were removed. No driver was installed or
loaded, no signing mode or registration changed, and no machine audio
configuration was modified.

## 2026-09-08 - Full workspace requalification after `1b782fd3`

`cargo test --workspace --all-features --quiet --locked` passed all workspace
unit, integration, and doc-test suites after durable plan expiry hydration was
hardened, including control (93), engine (78), Windows audio (30), storage
(80), DSP (27), recording (30), plugin host/worker, transport, and CLI/MCP
coverage. Strict workspace Clippy with `-D warnings`, formatting, and
documentation validation also passed. No audio endpoint or machine
configuration was accessed.

## Safe-chain requalification after backup write-boundary fix (2026-09-08)

At pushed revision `cbf52f4`, the complete safe acceptance chain passed:
native WASAPI compile and read-only 34-endpoint inventory, disposable pinned
SysVAD x64 qualification/package/API validation, M01 CLI, M04 DSP/recording,
M05 UI, M06 SDK/VST3, M07 headless, unsigned M08 artifact preparation and
verification, and documentation acceptance (51 Markdown files, 160 local
links). The storage backup write-boundary regression is included in this
tree. Temporary outputs and checkouts were removed; no driver was installed
or loaded, no signing mode changed, and no plugin/startup registration or
machine audio configuration was changed.

## Safe-chain requalification after `95fab8d` (2026-09-08)

The repository-wide safe acceptance chain passed at the pushed percentile
telemetry head. Native compile and read-only 34-endpoint inventory passed;
disposable pinned SysVAD x64 qualification completed with the installed
VS/WDK toolchain; workspace, M01, M04, M05, M06, M07, unsigned M08 artifact,
and documentation checks passed. Temporary outputs/checkouts were removed.
This is qualification evidence only: no driver was installed or loaded, no
signing mode changed, and no plugin/startup registration or machine audio
configuration changed. Production driver ownership, signing, installer,
clean-machine, physical-latency, and manual UI/accessibility gates remain open.

## Workspace regression after journal hardening (`b571e24`, 2026-09-08)

The locked workspace unit/integration tests and doc-tests passed at the
current head, and strict workspace Clippy passed with `-D warnings`. This
requalifies the shared storage change through the control, transport, CLI,
MCP, engine, plugin, recording, and Windows-audio crate boundaries. No driver
was installed or loaded, signing mode was unchanged, no plugin or startup
entry was registered, and no machine audio configuration changed.

## Safe-chain requalification after discovery self-consistency (`3d01a6e`, 2026-09-08)

The elevated non-mutating command `powershell.exe -NoProfile
-ExecutionPolicy Bypass -File tests/acceptance/safe-all.ps1` passed at the
latest pushed tree. Native compile and 34-endpoint read-only inventory passed;
disposable pinned SysVAD x64 package/API qualification completed with the
installed VS/WDK toolchain; M01/M04/M05/M06/M07, unsigned M08 preparation, and
documentation validation passed. Temporary outputs/checkouts were removed.

No driver was installed or loaded, signing mode was unchanged, no plugin or
startup entry was registered, and no machine audio configuration changed.
Production driver/signing, installer, hardware/manual UI, native callback
deadline, and physical-latency gates remain open.

## Safe acceptance after workspace regression (`09494d7`, 2026-09-08)

The elevated `tests/acceptance/safe-all.ps1` wrapper passed at the current
head. Native compile and read-only 34-endpoint format inventory passed;
disposable pinned SysVAD x64 build/package/API qualification passed with the
installed VS/WDK toolchain; M01, M04, M05, M06, M07, unsigned M08, and
documentation acceptance also passed. The restricted-shell first attempt was
blocked by PnP inventory access denial; the elevated rerun completed without
changing configuration. Temporary outputs/checkouts were removed.

No driver was installed or loaded, signing mode was unchanged, no plugin or
startup entry was registered, and no machine audio configuration changed.
Production driver/signing, installer, hardware/manual UI, native callback
deadline, and physical-latency gates remain open.

## Workspace regression after plugin-state read validation (`03c58d9`, 2026-09-08)

The locked workspace test suite and strict workspace Clippy passed after the
M06 plugin-state read-boundary hardening. Formatting/diff checks and
documentation acceptance also passed (51 Markdown files, 160 local links).
No driver was installed or loaded, signing mode was unchanged, no plugin or
startup entry was registered, and no machine audio configuration changed.
Production driver/signing, installer, hardware/manual UI, native callback
deadline, and physical-latency gates remain open.

## Safe-chain requalification after versioned preset metadata (`390262f`, 2026-09-08)

The elevated non-mutating command `powershell.exe -NoProfile
-ExecutionPolicy Bypass -File tests/acceptance/safe-all.ps1` passed at the
current pushed head. Native compile and 34-endpoint inventory, disposable
pinned SysVAD x64 package/API qualification, M01/M04/M05/M06/M07, unsigned M08
preparation, and documentation validation (51 Markdown files, 160 local links)
all passed. Temporary outputs/checkouts were removed.

No driver was installed or loaded, signing mode was unchanged, no plugin or
startup entry was registered, and no machine audio configuration changed.
Production driver/signing, installer, hardware/manual UI, native callback
deadline, and physical-latency gates remain open.

## Safe-chain requalification after control schema alignment (`e22a89f`, 2026-09-08)

The elevated non-mutating command `powershell.exe -NoProfile
-ExecutionPolicy Bypass -File tests/acceptance/safe-all.ps1` passed at
`e22a89f`. Native compile, 34-endpoint read-only inventory, disposable pinned
SysVAD x64 package/API qualification, M01/M04/M05/M06/M07 validation, unsigned
M08 preparation, and documentation validation (51 Markdown files, 158 local
links) all passed. Temporary outputs and checkouts were removed.

No driver was installed or loaded, signing mode was unchanged, no plugin or
startup entry was registered, and no machine audio configuration changed.
Production driver/signing, installer, hardware/manual UI, native callback
deadline, and physical-latency gates remain open.

## Safe-chain requalification after durable journal hardening (2026-09-08)

The guarded `tests/acceptance/safe-all.ps1` chain completed at the current
`719f0a8` head after the durable plan, plugin-state, enrollment, and
idempotency-boundary changes. Native compile/inventory and disposable SysVAD
x64 package/API qualification completed, followed by the repository's
portable M01–M08 checks and documentation validation; temporary outputs and
checkouts were removed. No driver was installed or loaded, signing mode and
startup/plugin registration were unchanged, and no machine audio
configuration was changed. Production driver, signing, installer,
clean-machine, physical-latency, and manual UI gates remain open.

## Release artifact completeness (2026-09-08)

`verify-artifacts.ps1` now requires the complete unsigned preparation set:
the CLI and plugin-worker executables, UI archive, cargo and npm SBOMs, npm
lock snapshot, and third-party notices. The M08 acceptance wrapper asserts the
same required names before inspecting archive/SBOM structure. PowerShell
parsing, unsigned release acceptance, and diff checks pass. This remains
preparation-only evidence; signing, driver, installer, clean-machine, and
native-audio gates remain open.

## Full safe-chain requalification after `f86f870` (2026-09-08)

The repository safe acceptance chain passed: native compile and 34-endpoint
read-only format inventory, disposable pinned SysVAD x64 compile/package/API
qualification, workspace checks, M01 CLI, M04 DSP/recording, M05 UI, M06
SDK/VST3, M07 headless, unsigned M08 preparation, and documentation validation
(51 Markdown files, 158 local links). Temporary outputs and checkouts were
removed. No driver was installed or loaded, signing mode and startup/plugin
registration were unchanged, and no machine audio configuration changed.

## Unsigned release requalification after `ca4e8a4` (2026-09-08)

The complete `tests/acceptance/m08-release.ps1` wrapper passed after the
required-artifact verifier change. The optimized Rust binaries, UI production
archive, cargo/npm SBOMs, lock snapshot, notices, manifest checksums, and
unsigned blocker assertions all passed in a disposable temporary directory;
the directory was removed afterward. No signing, installer, driver, audio
endpoint, or machine configuration was accessed.

## Safe-chain requalification after `0162438` (2026-09-08)

The canonical safe acceptance chain passed again after telemetry overflow
hardening. Native compile and read-only 34-endpoint inventory, disposable
pinned SysVAD x64 qualification, workspace, M01, M04, M05, M06, M07, unsigned
M08 artifact, and documentation checks all passed. Temporary outputs/checkouts
were removed. No driver was installed or loaded, signing mode was unchanged,
and no plugin/startup registration or machine audio configuration changed.
Production driver ownership, signing, installer, clean-machine,
physical-latency, and manual UI/accessibility gates remain open.

The post-qualification artifact audit found two stale disposable SysVAD
reference checkouts in the user temp directory. Their exact paths were
validated under the temp root and removed recursively; the repository probe
directory contained only checked-in `build.ps1` and `main.cpp` afterward. No
driver was installed or loaded, and no machine audio configuration changed.

## Current-tip unsigned qualification (2026-09-07)

The complete release-preparation acceptance passed at the current head. It
built the locked optimized artifacts and verified the manifest, third-party
notices/SBOM, sanitized provenance, checksums, explicit blocker assertions,
and artifact verification in a disposable temporary directory. The temporary
directory was cleaned afterward. This remains unsigned preparation evidence;
driver packaging, production signing, installer/upgrade/rollback, clean
machine validation, and native audio gates remain open.

## 2026-09-07 — Current-head unsigned requalification

The unsigned release-preparation wrapper passed again at the current head.
Optimized locked artifacts, manifest, notices/SBOM, provenance, checksums, and
verification were created and checked in a disposable directory, then removed.
This does not provide production signing, installer, driver, clean-machine, or
native-audio evidence.

## 2026-09-06 — Reproducible unsigned artifact preparation

Added `tools/release/prepare-artifacts.ps1`, which:

- requires a new output directory and refuses to overwrite an existing release directory;
- runs `cargo build --release --locked -p audiorouter-cli -p audiorouter-plugin-host`;
- copies the x64 CLI and disposable plugin-worker executables;
- records locked Cargo package metadata in `sbom.cargo.json`;
- writes per-artifact SHA-256 hashes and sizes in `release-manifest.json`;
- marks the result `signed: false` and `publicationReady: false`, with explicit production certificate, driver package, and clean-machine installer blockers.

PowerShell parser validation, `cargo fmt --all -- --check`, and `git diff --check` pass. An actual release build was attempted, but Windows Application Control blocked the generated `getrandom` build helper with OS error 4551. No signed artifact, installer, driver, or machine configuration was produced or changed.

## 2026-09-06 â€” Complete SBOM provenance

Release preparation now runs locked full `cargo metadata` (including the
dependency graph) and records the exact Git source revision in the release
manifest. The PowerShell script parses successfully and the complete locked
metadata command passes after the local Cargo cache is available. This still
does not claim signed artifacts, an installer, driver packaging, or clean-
machine qualification.

On 2026-09-06, unsigned release preparation completed successfully with the
installed Visual Studio 2026 toolchain: locked optimized CLI and plugin-worker
builds completed, `release-manifest.json` and `sbom.cargo.json` were generated,
and `verify-artifacts.ps1` accepted the output. The output directory was under
the user temp path and was removed after verification. This does not close
production signing, driver packaging, installer, or clean-machine gates.

The preparation script also refuses any tracked or untracked working-tree
changes before invoking Cargo, so the manifest and SBOM cannot describe an
uncommitted source state. PowerShell parsing and diff validation pass.

Added `tools/release/verify-artifacts.ps1`, a read-only verifier for prepared
directories. It validates the pinned manifest schema/x64 source revision,
requires explicit unsigned/publication blockers, parses the Cargo SBOM, and
checks every artifact's safe filename, existence, SHA-256, and byte count.
PowerShell parser validation and the missing-input failure path pass; signing,
installer, driver, and clean-machine qualification remain release blockers.

## 2026-09-06 — Headless operations runbook

Added `tools/release/test-verify-artifacts.ps1`, which validates the verifier
against a temporary valid manifest and then confirms tampering is rejected.
The harness cleans its temporary directory in `finally`; the positive and
tamper cases pass without modifying release outputs or machine configuration.

Added `docs/operations/headless-runbook.md` and linked it from the documentation index. It documents commands that exist in the current CLI, the versioned plan-file workflow, non-overwriting recovery backups, staged bundle import, MCP stdio/pipe launch, stale-plan recovery, and explicit limits around native routing, drivers, signing, and installation. The runbook does not present portable tests or an MCP response as evidence of a configured audio system.

Added docs/operations/release-qualification.md and linked it from the
documentation index. The checklist gives the reproducible unsigned
prepare/verify commands, requires clean inputs and new destinations, separates
recording/database recovery from future installation actions, and lists the
remaining driver, signing, native-routing, sandbox, and clean-machine gates.
It makes no installer, driver, or machine-configuration claim.

## 2026-09-06 — Unsigned artifact qualification

Ran tools/release/prepare-artifacts.ps1 and
tools/release/verify-artifacts.ps1 from clean revision
5258346cf4002367723f05efd11a9bf1692507c0, using a newly generated
temporary output directory. The locked release build completed successfully,
the manifest and Cargo SBOM parsed, and verification matched all recorded
SHA-256 hashes and byte counts:

- audiorouter-cli.exe — 4,768,768 bytes —
  fcc87ae280f799ac768d616a08eb739fac82c82b93e5ffcc1c7c02804cd0f326
- audiorouter-plugin-worker.exe — 415,744 bytes —
  94e4d25813b8c53c0e1ed02ca618b482d1d679a5e6c8cb5f30f56dfbcec4406a
- sbom.cargo.json — 674,254 bytes —
  42558e44ebe7cdb0314151f8ed2f30a7e9fb15f9068942d95c75867ddc397dd4

The temporary output directory was removed after verification. This validates
the unsigned preparation workflow only; production signing, driver package
and signing, installer, and clean-machine acceptance remain open. No audio
endpoint or machine configuration was changed.

## 2026-09-06 — Post-change workspace validation

After the backup-retention, incremental-FLAC, bounded-worker-read, and
Windows Job Object changes, the complete locked workspace validation passed:
14 CLI unit tests plus the MCP process test, 52 control, 24 domain, 24 DSP,
38 engine, 26 plugin-host plus 4 worker-process, 5 protocol, 26 recording,
30 storage, 14 transport, and 8 Windows-audio tests. All doc tests, strict
workspace Clippy, formatting, and diff checks also passed. Tests used
temporary/local fixtures only; no audio endpoint or machine configuration was
changed.

The same safe chain was rerun at pushed head `8fd56c1` and passed. This
requalification remains unsigned and non-release evidence: no driver was
installed or loaded, no live audio or startup/plugin registration was used,
and signing, installer, clean-machine, hardware, and manual acceptance gates
remain open.
## 2026-09-06 — Workspace validation after hardening

The complete `cargo test --workspace --locked` suite passed after the
incremental-FLAC recovery and plugin state reparse-point changes. This includes
all unit, integration, worker-process, MCP, and doc tests. Workspace
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
format checking, and `git diff --check` also passed. No audio endpoint,
default device, volume, mute, privacy, driver, or other machine configuration
was changed by this validation.

The same workspace validation was rerun at the later current tip after the
control-pipe singleton and UI event-cursor reliability changes. The complete
locked test suite and strict workspace Clippy, format, and diff checks remained
green; no audio endpoint or machine configuration was touched.

## 2026-09-06 — Current-tip unsigned artifact requalification

The locked release preparation was rerun from the clean current revision using
the installed Visual Studio 2026 toolchain. Optimized `audiorouter-cli.exe` and
`audiorouter-plugin-worker.exe` builds completed; the script generated the full
locked Cargo SBOM and provenance manifest, and `verify-artifacts.ps1` accepted
the recorded SHA-256 hashes and byte counts. The temporary output directory was
removed afterward. The manifest continues to state `signed: false` and
`publicationReady: false`; production signing, driver packaging/install,
installer, and clean-machine acceptance remain open. No audio endpoint or
machine configuration was changed.

Release preparation now also emits `THIRD-PARTY-NOTICES.txt`, listing every
locked Cargo package, version, declared license, and source/workspace origin.
It is generated from the exact metadata used for `sbom.cargo.json`, included in
the manifest's checksum/byte-count list, and therefore cannot be omitted from
the prepared artifact set without verifier failure.

The updated flow was executed from the clean current revision. Both optimized
binaries built successfully, `verify-artifacts.ps1` accepted the manifest and
all hashes/byte counts, and an additional check confirmed the notice contains
the expected workspace package entries. The temporary output contained
`THIRD-PARTY-NOTICES.txt` (15,231 bytes) and was removed afterward. The result
remains an unsigned preparation set; signing, driver, installer, and
clean-machine gates are unchanged.

The verifier regression now creates and verifies both a binary artifact and a
`THIRD-PARTY-NOTICES.txt` artifact, then confirms tampering with the binary is
rejected. Its temporary fixture is removed in `finally`, and no release output
or machine configuration is changed.

The complete locked workspace was also rerun at the current revision after the
storage hardening and release qualification. All unit, integration,
worker-process, MCP, and doc tests passed, as did strict workspace Clippy,
formatting, and diff checks. The validation used temporary/local fixtures only;
it did not open a live audio stream or change machine audio configuration.

Corrected the release-qualification command example: `verify-artifacts.ps1`
accepts `-ManifestPath`, not the previously documented `-ReleaseDirectory`.
The new `tools/release/test-runbook-command.ps1` regression checks the exact
invocation and rejects the obsolete parameter, keeping the unsigned artifact
workflow executable as documented.

The artifact verifier now rejects reparse-point paths before hashing and
requires `THIRD-PARTY-NOTICES.txt` alongside the SBOM. Its temporary regression
fixture covers the existing tamper check and conditionally exercises a symbolic
link artifact when the host permits link creation; no release output is kept.
The fixture also verifies that a manifest omitting the notice entry is rejected.

The Windows CI job now runs both PowerShell regressions on every push and pull
request, so release-path safety and runbook parameter parity are continuously
validated without producing a release package.
## 2026-09-06 — Qualification regression rerun

The artifact verifier regression and release qualification command-documentation
regression both passed at the current branch. These checks use disposable
fixtures and do not produce a signed package or alter audio/driver state.
## 2026-09-06 — Current unsigned qualification

The actual preparation flow was rerun from the clean current revision. Locked
optimized CLI and plugin-worker binaries, `sbom.cargo.json`,
`THIRD-PARTY-NOTICES.txt`, and `release-manifest.json` were generated in a
disposable temporary directory. `verify-artifacts.ps1` accepted the manifest,
hashes, and byte counts, after which the temporary directory was removed.
This is unsigned artifact evidence only; no driver, installer, signature, or
audio configuration was created or changed.

## 2026-09-06 - Current-tip qualification after SDK verification

The preparation flow was rerun from clean revision
`4aa534fcc781e3245f3d861270c108279c11cd39`. Locked optimized CLI and plugin
worker binaries, the complete Cargo metadata SBOM, dependency notices, and
the provenance manifest were generated in a disposable temporary directory.
The artifact verifier and runbook-command regression passed, and the
temporary directory was removed. The manifest remains explicitly unsigned
and not publication-ready; driver/signing, installer, and clean-machine gates
remain open.

The complete unsigned M08 preparation wrapper passed at the current head.
Optimized locked artifacts, SBOM, third-party notices, sanitized provenance,
checksums, manifest verification, and unsigned blocker assertions all passed in
a disposable directory, which was removed afterward. Signing, driver,
installer, clean-machine, and native audio gates remain open.

Requalified the unsigned release-preparation acceptance at the current
revision. Optimized locked CLI and plugin-worker artifacts, the Cargo SBOM,
third-party notices, manifest hashes, and byte counts were generated and
verified in a disposable temporary directory. The manifest retained
`signed: false`, `publicationReady: false`, and the required release blockers.
The temporary directory was removed afterward; no installer, driver, signing,
or machine audio configuration action occurred.

## 2026-09-06 - Qualification after UI test configuration

The complete unsigned preparation flow was rerun from revision
`ff5d9e3c71d5f22c59a8b8d2af488b051f2c72dc`. The optimized CLI and plugin
worker, locked Cargo SBOM, third-party notices, and provenance manifest were
created in a disposable temporary directory. The artifact verifier accepted
all four artifacts, and the manifest explicitly reported `signed: false` and
`publicationReady: false`; the temporary directory was removed afterward.
No signing, driver installation, installer action, or audio configuration was
performed.

## 2026-09-06 - Current-tip qualification after preset and UI alignment

From source revision `d035bf8458e48255102865a816951d7d8fa5f92b`, the unsigned
preparation scripts were rerun with the installed VS2026 toolchain in a new
temporary directory. Optimized x64 CLI and plugin-worker binaries, locked Cargo
SBOM metadata, third-party notices, checksums, and the release manifest were
generated; `verify-artifacts.ps1` passed. The manifest remains explicitly
`signed: false` and `publicationReady: false`, with driver/signing, installer,
and clean-machine blockers retained. The temporary output was removed after
verification.

The checked-in `tests/acceptance/m08-release.ps1` now reproduces this flow,
asserts that unsigned artifacts retain at least the three known release
blockers, and removes its uniquely named temporary output in a `finally`
cleanup. It does not install an artifact or change machine configuration.

The wrapper passed from clean revision `ff2fd5e` using the installed VS2026
toolchain. Both preparation and verification completed, and the temporary
directory was confirmed removed by the wrapper's cleanup path.

The wrapper was re-run successfully after the plugin scan-root hardening
revision on 2026-09-06. Optimized CLI/plugin-worker artifacts, SBOM, notices,
checksums, and manifest were prepared and verified, then the temporary output
was cleaned up. No installer, driver, signing action, or audio configuration
was involved.

After the verifier was tightened to reject unlisted package entries, the full
`tests/acceptance/m08-release.ps1` wrapper passed again from clean revision
`1b5094e`. The generated package's listed binaries, SBOM, notices, and
manifest were verified and the disposable directory was removed.

After preparation was tightened to require an existing non-reparse output
parent, `tests/acceptance/m08-release.ps1` passed again from clean revision
`ddfa192`. Artifact generation, exact-content verification, unsigned blocker
assertions, and temporary-directory cleanup all succeeded.

The release manifest now also records sanitized build provenance: release
profile, Rust target, compiler release, and Cargo version. The full wrapper was
rerun from clean revision `f59fe5c`; provenance validation and artifact checks
passed, and the temporary output was removed.

At clean revision `5670dde`, the broader locked workspace verification passed
325 unit/integration tests, all doc-tests, strict Clippy, and formatting. The
M08 wrapper then prepared and verified the optimized unsigned artifacts again,
asserted the publication blockers, and removed its temporary output. No
installer, driver, signing action, or audio configuration was involved.

The complete wrapper was requalified at clean revision `e5f240b` with the
installed Visual Studio 2026 toolchain. Optimized locked CLI and plugin-worker
artifacts, the full dependency SBOM, third-party notices, sanitized toolchain
provenance, checksums, and exact-content verification all passed in a unique
temporary directory. The wrapper asserted that the result remains unsigned
and not publication-ready, then removed the temporary directory. No installer,
driver, signing action, or audio configuration was involved.

The M08 wrapper was requalified again at the current tip. Optimized locked
CLI/plugin-worker artifacts, SBOM/notices, sanitized provenance, hashes, exact
content verification, and unsigned publication-blocker assertions passed in a
unique temporary directory, which was removed afterward.
## 2026-09-06 — Current-tip unsigned release preparation

The M08 release wrapper was rerun after the portable recovery-supervisor
change. Optimized pinned workspace artifacts, the manifest, notices/SBOM,
provenance, checksums, and byte-count verification passed in a disposable
temporary directory. The manifest correctly remained unsigned and not
publication-ready with the required blockers. The temporary output was
removed; no installer, driver, signing operation, audio endpoint, or machine
configuration was used.
## Current-tip unsigned qualification (2026-09-06)

The M08 wrapper prepared and verified optimized unsigned artifacts in a
disposable directory, then cleaned the output. Signing, driver, installer, and
clean-machine gates remain open; no audio configuration was changed.
## Artifact-root reparse protection (2026-09-06)

The release verifier now requires the manifest's parent directory to be a
regular non-reparse directory before validating the manifest, SBOM, notices,
and artifacts. This closes redirected-root verification paths while retaining
the existing per-file checks; no installer, signing, driver, audio, or machine
configuration action is performed.
## Lexical release-root reparse protection (2026-09-06)

The verifier now audits the manifest path and each lexical parent before
`Resolve-Path` canonicalization, preventing a symlinked artifact root from
being silently normalized into an apparently safe directory. The disposable
verifier regression covers this path when symbolic-link creation is available.
## Full output-parent reparse protection (2026-09-06)

Artifact preparation now checks every existing parent from the requested output
directory up to the filesystem root, not only the immediate parent. This
prevents a redirected grandparent from receiving unsigned release artifacts;
the existing disposable path-safety suite continues to pass.
## Post-hardening release qualification (2026-09-06)

The complete M08 wrapper passed after lexical and parent-chain reparse
hardening. Optimized unsigned artifacts, manifest provenance, notices/SBOM,
checksums, explicit blocker assertions, and verification succeeded in a
disposable directory; the directory was removed afterward.

## Current-head unsigned qualification (2026-09-07)

The wrapper passed again at the current head. Optimized locked artifacts,
manifest, third-party notices/SBOM, sanitized provenance, checksums, exact
content verification, and unsigned publication-blocker assertions succeeded
in a disposable temporary directory, which was removed afterward. Signing,
driver, installer, clean-machine, and native-audio gates remain open; no
machine configuration was changed.

## Disposable UI release artifact (2026-09-07)

Release preparation now builds the UI with the locked local npm inputs into an
isolated temporary directory and packages the resulting static bundle as
`audiorouter-ui.zip` beside the CLI and plugin-worker binaries. The artifact is
included in the same SHA-256 manifest and exact-content verification; the
temporary UI build directory is removed in the preparation `finally` path.
The release acceptance passed with the unsigned status and driver/signing,
installer, and clean-machine blockers preserved. No UI was installed, and no
audio or machine configuration was changed.

The release set now also carries the validated UI lockfile as
`sbom.npm.package-lock.json` and a deterministic `sbom.npm.json` CycloneDX
document generated directly from that lockfile. This avoids depending on npm's
installed `node_modules` tree, whose SBOM command is blocked here by the local
file dependency's incomplete nested dev-dependency installation. Both files
are included in normal manifest hashing and exact-content verification.

The M08 acceptance wrapper now explicitly requires exactly one
`audiorouter-ui.zip` manifest entry and opens the archive to verify its
`index.html` entry. This keeps the application artifact requirement regression-
protected rather than relying only on generic checksum coverage.

The corrected clean-tree wrapper run passed this archive assertion together
with locked CLI/worker builds, UI typecheck/build, manifest checksums, exact
content verification, and cleanup. The artifact remains unsigned and the
driver, signing, installer, and clean-machine blockers remain explicit.

The subsequent clean-tree run also passed after npm lockfile validation was
moved to Node: CLI/worker/UI artifacts, lockfile provenance, archive contents,
hashes, exact-content verification, unsigned status, and cleanup all passed.
No installer, driver, signing, or machine configuration action occurred.

The acceptance wrapper now structurally validates `sbom.npm.json` as a
nonempty CycloneDX 1.5 document and requires both npm provenance files in the
manifest, in addition to their generic hash and exact-content checks.

## Current-head unsigned release requalification (2026-09-07)

The M08 acceptance wrapper passed at the current head. It built the locked
optimized CLI, plugin worker, and UI bundle; generated and verified npm
lockfile/CycloneDX provenance, manifest hashes, and UI archive contents;
asserted unsigned driver/signing/installer blockers; and removed temporary
output. No installer, driver, signing, plugin registration, or audio
configuration action occurred.

## Full safe-chain requalification (2026-09-07)

At the origin-gated UI head (`e597840`), the sequential safe acceptance runner
passed every stage, including native compile/format inventory, disposable
SysVAD qualification, M01/M04/M05/M06/M07 checks, this clean-tree M08 release
preparation, and documentation validation. The M08 artifacts were unsigned as
required and were removed after verification; no installation, signing-mode,
plugin-registration, startup, or audio-configuration action occurred.

## Aggregate safe-chain requalification (2026-09-07)

The new sequential `tests/acceptance/safe-all.ps1` runner passed from a clean
working tree. Its M08 stage again built and verified the unsigned release
artifacts, including locked CLI/plugin-worker binaries, UI output, npm
provenance, manifest hashes, archive contents, and explicit blocker state.
Temporary output was removed and no installer, driver, signing, or audio
configuration action occurred.

## Current-head aggregate safe-chain requalification (2026-09-07)

The elevated sequential `tests/acceptance/safe-all.ps1` runner passed at the
current head. Read-only native inventory reported 31 endpoints; disposable
SysVAD x64 build/package validation, M01, M04, M05, M06, M07, unsigned M08
artifact preparation/verification, and documentation all passed. Temporary
checkouts and artifacts were removed. Live audio, driver installation,
signing-mode changes, plugin/startup registration, and machine audio
configuration remained excluded.

## Latest-head aggregate requalification (2026-09-07)

At pushed head `ed988a2`, the elevated sequential safe acceptance runner passed
all stages again. Read-only native format inventory reported 34 endpoints;
disposable SysVAD x64 qualification, M01 CLI, M04 DSP/recording including
bounded chunk admission, M05 UI, M06 SDK/worker checks, M07 headless checks,
unsigned M08 artifacts, and documentation all passed. Temporary outputs were
removed, and live audio, driver installation, signing-mode changes,
plugin/startup registration, and machine audio configuration remained excluded.

## Current pushed-head aggregate requalification (2026-09-07)

The elevated sequential `tests/acceptance/safe-all.ps1` runner passed again at
the current pushed head. Native compile and read-only 34-endpoint format
inventory, disposable pinned SysVAD x64 qualification, M01/M04/M05/M06/M07,
unsigned M08 artifact preparation/verification, and documentation all passed.
Temporary outputs and the disposable SysVAD checkout were removed. No driver
was installed or loaded; signing-mode changes, plugin/startup registration,
and machine audio configuration remained excluded.

## Pushed-head safe-chain requalification (2026-09-07)

At pushed head `15499db`, the elevated sequential safe chain passed native
compile and read-only 34-endpoint format inventory, pinned disposable SysVAD
x64 build/package/API qualification, M01/M04/M05/M06/M07, unsigned M08
artifact preparation/verification, and documentation acceptance. Temporary
outputs and checkouts were removed. No driver was installed or loaded, and no
signing-mode, plugin/startup registration, or machine audio configuration
changed.

## Pushed-head safe-chain requalification at `19f86e8` (2026-09-07)

The elevated sequential `tests/acceptance/safe-all.ps1` runner passed at the
new pushed head. Native compile and read-only 34-endpoint format inventory,
disposable pinned SysVAD x64 build/package/API qualification, full locked
workspace tests, M01 CLI, M04 DSP/recording, M05 UI, M06 SDK/VST3, M07
headless, unsigned M08 artifact preparation/verification, and documentation all
passed. Temporary outputs/checkouts were removed. Driver installation/loading,
signing-mode changes, plugin/startup registration, live audio, installer,
hardware, and manual UI/accessibility gates remained excluded.
## Safe-chain requalification at `0e28f3a` (2026-09-07)

The complete safe acceptance chain passed at the current pushed head: native
compile, read-only endpoint format inventory (34 endpoints), disposable SysVAD
x64 compile/package/API qualification, workspace tests, M01 CLI, M04 DSP and
recording, M05 UI, M06 SDK/VST3, M07 headless, unsigned M08 artifact
preparation, contracts, and documentation. Temporary build/checkouts were
removed by the acceptance flow. Live audio, driver installation/loading,
signing-mode changes, plugin/startup registration, installer, hardware, and
manual UI/accessibility gates remain explicitly unattempted.
## Current-head safe-chain requalification (2026-09-07)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/safe-all.ps1` at pushed head `f741be5`.

The complete non-mutating qualification chain passed: native M00 compile and
34-endpoint format inventory; disposable pinned SysVAD x64 build/package/API
validation; locked workspace tests; M01 CLI; M04 DSP/recording; M05 UI
typecheck, 88 tests, and temporary production build; M06 SDK provenance and
VST3 build/validator/offline loader; M07 headless checks; unsigned M08 release
preparation; and documentation validation (51 Markdown files, 157 local
links). Temporary build outputs and the SysVAD checkout were removed.

The chain did not install or load a driver, change signing mode, register a
plugin or startup entry, or modify machine audio configuration. Native driver
installation/signing, installer, hardware/manual acceptance, and physical
latency remain open release gates.

## Current-head safe-chain requalification (2026-09-08)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/safe-all.ps1` at pushed head `419c368`.

The complete non-mutating chain passed again: native compile and 34-endpoint
read-only inventory; disposable pinned SysVAD x64 build/package/API
qualification; M01 CLI; M04 DSP/recording; M05 UI with 88 tests and temporary
production build; M06 SDK/VST3 validation; M07 headless checks; unsigned M08
artifact preparation; and documentation validation (51 Markdown files, 157
local links). Temporary outputs/checkouts were removed.

No driver was installed or loaded, signing mode was unchanged, no plugin or
startup entry was registered, and no machine audio configuration was changed.
Production driver/signing, installer, hardware/manual UI, and physical latency
gates remain open.

## Safe-chain requalification after rate-domain bridge (2026-09-08)

The full non-mutating safe chain passed after commit `1264fa4`: native compile,
34-endpoint read-only inventory, disposable pinned SysVAD x64 build/package/API
qualification, locked workspace checks, M01/M04/M05/M06/M07 validation,
unsigned M08 preparation, and documentation validation (51 Markdown files,
157 local links). Temporary outputs/checkouts were removed.

The guarded live process-loopback acceptance also passed separately for both
include and exclude modes with media-device snapshot equality. No driver was
installed or loaded, signing mode and plugin/startup registration were
unchanged, and no machine audio configuration changed. Production driver,
signing, installer, hardware/manual UI, and physical-latency gates remain
open.

## Safe-chain requalification after documentation handoff (2026-09-08)

The elevated non-mutating command `powershell.exe -NoProfile
-ExecutionPolicy Bypass -File tests/acceptance/safe-all.ps1` passed at pushed
head `b181406`. It completed native compile and 34-endpoint read-only
inventory, disposable pinned SysVAD x64 build/package/API qualification, M01
CLI, M04 DSP/recording, M05 UI (typecheck, 88 tests, temporary production
build), M06 SDK/VST3 validation, M07 headless checks, unsigned M08 artifact
preparation, and documentation validation (51 Markdown files, 158 local
links). Temporary outputs/checkouts were removed.

No driver was installed or loaded, signing mode was unchanged, no plugin or
startup entry was registered, and no machine audio configuration changed.
Production driver/signing, installer, hardware/manual UI, callback deadline,
and physical-latency gates remain open.

## Safe-chain requalification after raw timing histogram validation (`f0d1b03`, 2026-09-08)

The elevated non-mutating command `powershell.exe -NoProfile
-ExecutionPolicy Bypass -File tests/acceptance/safe-all.ps1` passed at
`f0d1b03`. It completed native compile and 34-endpoint read-only inventory,
disposable pinned SysVAD x64 build/package/API qualification, M01 CLI, M04
DSP/recording, M05 UI (typecheck, 88 tests, temporary production build), M06
SDK/VST3 validation, M07 headless checks, unsigned M08 artifact preparation,
and documentation validation (51 Markdown files, 158 local links). Temporary
outputs/checkouts were removed.

No driver was installed or loaded, signing mode was unchanged, no plugin or
startup entry was registered, and no machine audio configuration changed.
Production driver/signing, installer, hardware/manual UI, callback deadline,
and physical-latency gates remain open. The next authorized work remains
native production-style scheduler callback period/deadline evidence when that
callback owns an endpoint stream.

## Safe-chain requalification after scheduler deadline telemetry (`801bd50`, 2026-09-08)

The elevated non-mutating command `powershell.exe -NoProfile
-ExecutionPolicy Bypass -File tests/acceptance/safe-all.ps1` passed at
`801bd50`. It completed native compile and 34-endpoint read-only inventory,
disposable pinned SysVAD x64 build/package/API qualification, the full locked
workspace regression, M01 CLI, M04 DSP/recording, M05 UI (typecheck, 88 tests,
temporary production build), M06 SDK/VST3 validation, M07 headless checks,
unsigned M08 artifact preparation, and documentation validation (51 Markdown
files, 158 local links). Temporary outputs/checkouts were removed.

No driver was installed or loaded, signing mode was unchanged, no plugin or
startup entry was registered, and no machine audio configuration changed.
Production driver/signing, installer, hardware/manual UI, native callback
deadline, and physical-latency gates remain open.

## Safe-chain requalification after deadline-lateness distribution (`d0461a8`, 2026-09-08)

The elevated non-mutating command `powershell.exe -NoProfile
-ExecutionPolicy Bypass -File tests/acceptance/safe-all.ps1` passed at
`d0461a8`. Native compile and 34-endpoint inventory, disposable pinned SysVAD
x64 package/API qualification, full workspace checks, M01/M04/M05/M06/M07
validation, unsigned M08 preparation, and documentation validation (51
Markdown files, 158 local links) all passed. Temporary outputs/checkouts were
removed.

No driver was installed or loaded, signing mode was unchanged, no plugin or
startup entry was registered, and no machine audio configuration changed.
Production driver/signing, installer, hardware/manual UI, native callback
deadline, and physical-latency gates remain open.

## Safe-chain requalification after shared-mode deadline qualification (`29951c3`, 2026-09-08)

The elevated non-mutating command `powershell.exe -NoProfile
-ExecutionPolicy Bypass -File tests/acceptance/safe-all.ps1` passed at
`29951c3`. Native compile, 34-endpoint read-only inventory, disposable pinned
SysVAD x64 package/API qualification, full workspace regression, M01/M04/M05/
M06/M07 validation, unsigned M08 preparation, and documentation validation
(51 Markdown files, 158 local links) all passed. Temporary outputs and
checkouts were removed.

No driver was installed or loaded, signing mode was unchanged, no plugin or
startup entry was registered, and no machine audio configuration changed.
Production driver/signing, installer, hardware/manual UI, native callback
deadline, and physical-latency gates remain open.
## Safe-chain requalification after `95fab8d` (2026-09-08)

The repository-wide safe acceptance chain passed at the pushed percentile
telemetry head. Native compile and read-only 34-endpoint inventory passed;
disposable pinned SysVAD x64 qualification completed with the installed
VS/WDK toolchain; workspace, M01, M04, M05, M06, M07, unsigned M08 artifact,
and documentation checks passed. Temporary outputs/checkouts were removed.
This is qualification evidence only: no driver was installed or loaded, no
signing mode changed, and no plugin/startup registration or machine audio
configuration changed. Production driver ownership, signing, installer,
clean-machine, physical-latency, and manual UI/accessibility gates remain open.
## Current unsigned release requalification (2026-09-08)

The disposable `tests/acceptance/m08-release.ps1` wrapper passed at the current
implementation head. Locked optimized CLI and plugin-worker builds, the UI
production archive, Cargo/npm SBOMs, npm lock snapshot, third-party notices,
provenance, required-artifact checks, and SHA-256/byte-count verification all
passed. The temporary output directory was removed by the wrapper. The result
is unsigned preparation evidence only: no installer, driver, signing action,
plugin registration, or audio configuration change occurred.

## Full workspace requalification (2026-09-08)

At the current implementation head, the locked workspace passed 466
unit/integration tests across all crates and process targets, all doc-tests,
strict workspace Clippy with `-D warnings`, formatting, and diff checks. This
run did not install or load a driver, change signing mode, register plugins or
startup entries, or alter machine audio configuration.

## Complete safe-chain requalification at `9d6b267` (2026-09-08)

The elevated `tests/acceptance/safe-all.ps1` chain passed after the M04
evidence update: native probe compile, read-only 34-endpoint format inventory,
disposable pinned SysVAD x64 compile/package/API qualification, M01 CLI, M04
DSP/recording, M05 UI, M06 SDK installer and VST3 SDK, M07 headless, unsigned
M08 release preparation, and documentation validation. Temporary outputs and
checkouts were removed. No driver was installed or loaded, signing mode
changed, plugin/startup entry registered, or machine audio configuration
altered. Production driver ownership, signing, installer, clean-machine,
physical-latency, native callback deadline, and manual UI gates remain open.

## Complete safe-chain requalification after SDK check (2026-09-08)

The elevated `tests/acceptance/safe-all.ps1` chain passed after verifying the
official Windows SDK prerequisite: native compile, read-only 34-endpoint format
inventory, disposable pinned SysVAD x64 compile/package/API qualification, M01
CLI, M04 DSP/recording, M05 UI, M06 SDK/VST3, M07 headless, unsigned M08
preparation, and documentation validation. Temporary outputs and checkouts were
removed. The installed matching `10.0.28000.0` SDK/WDK remained in place; no
driver was installed or loaded, signing mode changed, plugin/startup entry
registered, or machine audio configuration altered. Production driver ownership,
signing, installer, clean-machine, physical-latency, native callback deadline,
and manual UI gates remain open.

## Clean safe-chain requalification with toolchain guard (2026-09-08)

After committing the read-only toolchain guard, `tests/acceptance/safe-all.ps1`
passed from a clean tree. It verified Visual Studio/MSVC, the matching
Windows SDK/WDK line, native compile and endpoint inventory, disposable SysVAD
x64 package/API qualification, M01/M04/M05/M06/M07 acceptance, unsigned M08
preparation, and documentation validation. Temporary outputs/checkouts were
removed. No driver, signing-mode, plugin/startup registration, or machine
audio configuration action occurred.

## Requirement-map coverage acceptance (2026-09-08)

The range-aware `tests/acceptance/m08-traceability.ps1` check passed for all 158
normative requirement IDs extracted from the specification. Every ID is covered
by the delivery traceability table. This verifies documentation coverage only;
it does not claim that every requirement has implementation or release evidence.

## Clean safe-chain requalification with traceability guard (2026-09-08)

The clean `tests/acceptance/safe-all.ps1` chain passed with the read-only
toolchain and range-aware traceability checks enabled. It covered native
compile/inventory, disposable SysVAD x64 qualification, M01/M04/M05/M06/M07,
unsigned M08 preparation, 158 normative requirement-map checks, and
documentation validation. Temporary outputs/checkouts were removed. No driver,
signing-mode, plugin/startup registration, or machine audio configuration action
occurred; production and hardware gates remain open.

## Locked workspace regression after SDK documentation (2026-09-08)

The locked workspace regression passed with 466 unit/integration tests, all
workspace doc-tests, strict Clippy using `-D warnings`, and documentation
validation. This was a portable/build verification only; no driver, signing
mode, plugin/startup registration, or machine audio configuration changed.

## Safe chain at UI-pagination tip (2026-09-08)

At pushed tip `b120b87`, the complete safe chain passed: M00 toolchain/native
compile/format inventory and disposable pinned SysVAD x64 qualification, M01,
M04, M05 with 91 UI tests and a temporary production build, M06 SDK/VST3,
M07, unsigned M08 artifacts, traceability, and documentation validation.
Temporary outputs/checkouts were removed. Production driver, signing,
installer, hardware, and manual UI gates remain open.

## Safe chain after scheduler lifecycle fix (2026-09-08)

At pushed tip `76e3852`, the elevated `tests/acceptance/safe-all.ps1` chain
passed all stages: read-only native toolchain/34-endpoint inventory and
compile, disposable SysVAD x64 qualification, M01/M04/M05/M06/M07, unsigned
M08 preparation, 158 normative requirement-map checks, and documentation
validation. Temporary outputs/checkouts were removed. Driver installation or
loading, signing-mode changes, plugin/startup registration, and machine audio
configuration were excluded and unchanged.
# M08 release evidence

## 2026-09-08 - Complete safe acceptance at `125676ee`

The elevated `tests/acceptance/safe-all.ps1` chain passed at the current head:
toolchain discovery, native compile, read-only 34-endpoint format inventory,
disposable pinned SysVAD x64 compile/package/API qualification, M01 CLI, M04
DSP/recording, M05 UI (typecheck, 91 tests, temporary production build), M06
SDK/VST3, M07 headless, unsigned M08 preparation, 158 normative requirement
mappings, and documentation validation (51 Markdown files, 160 local links).
The initial non-elevated attempt stopped at the expected PnP access-denied
boundary; the elevated retry completed successfully. Temporary artifacts and
the reference checkout were removed. No driver was installed or loaded, and
no signing mode, plugin/startup registration, or machine audio configuration
changed.

## 2026-09-08 - Workspace requalification after linear topology guard (`a8a9fc14`)

`cargo test --workspace --all-features --quiet --locked` passed all workspace
suites, including engine (78), control (92), domain (57), DSP (27), plugin
host (47) and worker (20), recording (30), storage (80), transport (18), and
Windows audio (29). The focused engine Clippy and documentation validation also
passed. This verifies the portable compiler change across workspace consumers;
native driver ownership, signing, clean-machine, physical-latency, and manual
UI gates remain open. No audio endpoint or machine configuration was changed.

## 2026-09-08 - Safe acceptance requalification at `4d9ce1f4`

The complete `tests/acceptance/safe-all.ps1` chain passed at the current
pushed head. Native toolchain/compile and 34-endpoint format inventory,
disposable pinned SysVAD x64 build/package/API validation, M01/M04/M05/M06/
M07 acceptance, unsigned M08 artifact preparation, 158-ID traceability, and
documentation validation (51 Markdown files and 160 local links) all passed.
Temporary outputs and checkouts were removed. No driver was installed or
loaded, signing mode or startup/plugin registration was changed, and machine
audio configuration remained unchanged. This is not signed release evidence;
production driver, signing, installer, clean-machine, physical-latency, and
manual UI gates remain open.
