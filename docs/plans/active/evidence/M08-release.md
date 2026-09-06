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

## 2026-09-06 — Headless operations runbook

Added `docs/operations/headless-runbook.md` and linked it from the documentation index. It documents commands that exist in the current CLI, the versioned plan-file workflow, non-overwriting recovery backups, staged bundle import, MCP stdio/pipe launch, stale-plan recovery, and explicit limits around native routing, drivers, signing, and installation. The runbook does not present portable tests or an MCP response as evidence of a configured audio system.
