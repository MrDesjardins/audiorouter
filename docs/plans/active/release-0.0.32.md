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
Reuse ignored npm dependency directories through junctions; copy the pinned
SDK locally because its validator rejects junction parents. Clean worktrees
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
Rust format checks and `git diff --check`. Full workspace test results and
their log path are recorded below.
No runtime implementation or dependency version changed in this branch.

### Candidate results (Windows 11 Home build 26300, Node 22.22.3)

- Clean-input build source: `725ce5022c3bd06e1ad537112178cefb777a76e4`.
  `prepare-artifacts.ps1 -AppOnly -OutputDirectory
  C:/code/audiorouter/.worktrees/release-0.0.32/target/releases/0.0.32-candidate`
  passed. `verify-artifacts.ps1 -ManifestPath
  target/releases/0.0.32-candidate/release-manifest.json` passed.
- Installer: `target/releases/0.0.32-candidate/AudioRouter_0.0.32_x64-setup.exe`,
  24,472,783 bytes; SHA-256
  `2C288B169CA7686BE7809D5A80DD097A3C61B49EB114ED967D6788C04DC23C36`.
- Shell SHA-256:
  `A15DEA28213F0D65820E3E4813E17E4D7134D78CA067B22CC1FBC3E77EDC5DED`.
  Shell timestamp 2026-11-23 06:55:52 UTC is newer than UI index
  06:52:51 UTC. x64/x86 plugin workers and CLI passed portable-import and
  architecture checks in preparation. Stream Deck typecheck/tests/pack passed;
  its dependency install reported zero vulnerabilities.
- Manifest records Rust 1.96.0, x64, unsigned, `includesDriver: false`,
  `publicationReady: false`. No release tag has been created; `releaseTag`
  is null. Candidate assets include checksums, Cargo/npm SBOMs and notices.
  After the release metadata is reviewed/merged and tagged, rebuild with
  `-ReleaseTag v0.0.32` and verify the exact tagged artifacts before a draft.
- Full locked workspace tests: 1,518 passed, 69 guarded tests ignored. Shell
  tests: 131 passed, four ignored. Strict workspace and shell Clippy passed
  with all targets/features. Logs: `target/release-0.0.32-checks/`
  (`workspace-tests.log`, `shell-tests.log`, `workspace-clippy.log`,
  `shell-clippy-final.log`, `artifact-build.log`).
- Initial shell Clippy ran before UI assets existed and failed in
  `generate_context!`; after the UI production build, the same command passed.
  No source repair was needed. Build advisories: UI chunks above 500 kB and
  React runtime `use no memo` bundling; retained in this metadata-only branch
  rather than changing application bundling. The no-sign warning is intentional.
- Exact candidate packaged smoke refused before launching any child because
  the owner's shell PID 61952 owns the default pipe. Evidence:
  `target/release-0.0.32-checks/packaged-smoke/result.json`. This is a blocked
  startup check, not an application failure or a passing save/restart test.
  Requested owner closure; did not stop their process or touch their profile.

Next: rerun the exact candidate smoke after owner closure. Then report local
checks and branch head for explicit PR-opening approval. Publication, tags and
external uploads remain unperformed. Current-candidate installer lifecycle,
Defender, signing, live audio/plugin and broader M08 gates remain open.
