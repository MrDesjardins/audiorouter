# Release 0.0.32 candidate preparation

## Objective and authority

Prepare an unsigned, experimental app-only 0.0.32 candidate after the owner
tested the local main build and reported it works on 2026-11-22. Publication
is a separate final human action (DIST-05). No PR opening is authorized.
Requirements: DIST-01/05/06/07/08, ENG-04/05, M08; existing qualification
gates remain open and cannot be replaced by a build or this owner report.

## Prerequisites and decisions

Base: main `ba34371ad190776193a630626bc27a117372b755`, matching fetched
origin/main. Previous published prerelease: v0.0.31. The owner-tested shell
SHA-256 is `F39C926411367131532573B0F0ACD0E868D03CDFAADF53099935370538B621C7`.
That build passed isolated native bridge/save/restart smoke, retained at
`C:/code/audiorouter/target/local-test-build-2026-11-22/result.json`.
The owner's shell is running, so use `.worktrees/release-0.0.32` and separate
build outputs; preserve the main checkout and all unrelated untracked files.
Reuse ignored dependency/SDK directories through junctions; clean worktrees
only with the prescribed worktree-cleanup script after merge.
Windows native build tools and the pinned Rust toolchain are available.
No VM credentials, live endpoint changes, driver install or signing purchase
is part of candidate preparation.

## Ordered tasks and validation matrix

1. Update all app/package versions and lock metadata together; write release
   notes covering UI repairs, API/MCP compatibility changes and security fixes.
2. Validate metadata, locked Cargo resolution, contract drift, UI checks,
   documentation and release-script regressions. Inspect the complete diff.
3. Commit the reviewed candidate on `release/0.0.32`; build app-only NSIS
   artifacts from clean inputs, verify hashes/provenance/SBOMs/portable imports.
4. Exercise the exact built shell using an isolated database/WebView profile
   and native bridge/save/restart smoke. Record exact candidate hashes and
   distinguish that evidence from the owner's earlier local build test.
5. Present the candidate, checks and remaining gates for publication approval.
   No publication or PR creation before the corresponding human approval.

## Risks and rollback

Unsigned v0.0.31 was quarantined by Defender in a fresh guest; Microsoft's
false-positive verdict remains pending in the signing plan. Do not claim a
clean Defender result for new bytes without checking them. Exact 0.0.32
installer lifecycle, WebView2, accessibility, endpoint/plugin and performance
gates require their own evidence. The candidate excludes the managed driver.
Rollback: retain v0.0.31 and owner data; abandon the unmerged candidate branch
if preparation fails. Never replace published assets in place.

## Execution and next action

Version and lock metadata updated without dependency resolution changes.
Complete version/notes diff inspected. Windows checks passed: locked workspace
and shell `cargo check`; UI typecheck, lint, format:check and all 1,165 tests;
contract typecheck/drift (156 methods, 38 kinds, 20 processors, 24 event
categories); docs acceptance (294 files/1,632 links); app-only profile,
artifact-verifier and preparation path-safety regressions; workspace/shell
Rust format checks and `git diff --check`. Full workspace tests are running,
with output in `target/release-0.0.32-checks/workspace-tests.log`.
No runtime implementation or dependency version changed in this branch.
Next: commit clean candidate inputs, prepare app-only artifacts, then qualify
the exact candidate shell and retain the remaining installer gates.
