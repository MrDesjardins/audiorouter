# Release 0.0.26 - 2026-10-31

Objective: owner-requested next unsigned app-only experimental prerelease,
including all merged main changes through `73ac6f85`. Requirements: DIST-01-08;
GRAPH-12/15, UI-17/20/25, SND-12/13/14/17/19. No driver or storage migration.
The owner explicitly requested release publication; no PR creation is authorized.
Version metadata and release notes are the only product-source changes.

Prerequisites: Windows, pinned Rust 1.96.0 and Node 22.22.3, native VST3 SDK,
verified tunnel runtime, NSIS and GitHub credentials. Already merged behavior
has combined Rust/UI/Clippy/format/site evidence in the combined-build record.
The default control pipe is occupied; preserve the owner's process and data.
Private-pipe native editor/library/drop checks can run independently of it.

Tasks:
1. Bump all first-party package versions and locks to 0.0.26; write notes.
2. Check locked metadata, formatting, documentation, release-script regressions,
   UI build/typecheck and Stream Deck; inspect the complete release diff.
3. Commit/tag the exact source and build app-only assets with the release
   preparer; retain the previous local packages and avoid overwriting releases.
4. Verify provenance/checksums, extract actual NSIS contents, check library and
   companion hashes/static runtime imports; run the disposable native editor.
5. Upload a draft, download and re-verify its assets, publish on the owner's
   explicit release request and confirm public assets.

Validation: record Windows commands and results below. Installer execution,
clean-machine upgrade/uninstall, missing WebView2, physical continuity and
attended Stream Deck remain open; disclose them as prerelease limitations.
Do not claim M08 completion. Signing is intentionally absent; no driver ships.
Rollback: retain 0.0.24 and the local 0.0.25 test package plus configuration
backups; no schema migration intended. Published assets are immutable.

Local preparation checks passed on Windows: both Rust formatting commands and
checks, docs validation (221 Markdown files / 1172 links), four release-script
regression suites (preparation, artifact verifier, installer hooks, app-only),
UI format/lint/typecheck/production build and contracts typecheck/drift (136
methods, 37 node kinds, 20 processors, 22 event categories). The complete diff
contains version metadata and release documentation only; no dependency update.
Restricted-token access initially blocked npm lock writes, Cargo debug-cache
access and Vite dist cleanup; exact metadata edits and authorized normal-context
checks passed. Vite's chunk-size advisory remains: bundle splitting changes
behavior/build structure and is outside this version-only release preparation.
Logs: `target/release-026-ui-build.log`, `target/release-026-contract-drift.log`.

Next: commit/tag this source, then run the clean tagged release preparer.
