# M08 release evidence

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
