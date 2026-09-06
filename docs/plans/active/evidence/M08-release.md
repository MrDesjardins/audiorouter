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

The preparation script also refuses any tracked or untracked working-tree
changes before invoking Cargo, so the manifest and SBOM cannot describe an
uncommitted source state. PowerShell parsing and diff validation pass.

## 2026-09-06 — Headless operations runbook

Added `docs/operations/headless-runbook.md` and linked it from the documentation index. It documents commands that exist in the current CLI, the versioned plan-file workflow, non-overwriting recovery backups, staged bundle import, MCP stdio/pipe launch, stale-plan recovery, and explicit limits around native routing, drivers, signing, and installation. The runbook does not present portable tests or an MCP response as evidence of a configured audio system.
