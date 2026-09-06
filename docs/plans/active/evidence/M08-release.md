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
