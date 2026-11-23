# Release 0.0.32 candidate preparation

## Objective and authority

Prepare an unsigned, experimental app-only 0.0.32 candidate after the owner
tested the local main build and reported it works on 2026-11-22. Publication
is a separate final human action (DIST-05). The owner approved opening PR #181
on 2026-11-23, then requested CI repair and branch-state reconciliation.
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

### Authorized consolidation (2026-11-23)

The owner selected consolidation of the verified pipe-guard fix
`d4290f8d8`, keybinding-delete recovery `eb759a522`, and latency/quality
documentation `3936293bc` (including `3774f43aa`) into PR #181.
Requirements: ENG-02, NFR-15, UI-10/11/17/29 and the existing Q3.2 evidence.
Plan: merge the branches preserving ancestry; immediately check transport
compilation and UI types; resolve only documentation overlaps; run transport
regressions, keybinding component/browser checks and lint/format, docs/summary
checks and strict Clippy; inspect the combined diff. Commit clean inputs,
rebuild app-only artifacts into a new candidate directory, verify and smoke
the exact shell with isolated profiles, update PR #181 and monitor CI.
No live endpoint, driver, signing or publication action is authorized.
Rollback: revert the consolidation merges and any integration fix without
discarding the original branches. Preserve them until the release PR reaches
main, then use the prescribed merged-worktree cleanup mechanism.

All three branches merged cleanly, preserving ancestry. Windows verification
passed: transport `cargo check` and `cargo test -p audiorouter-transport --locked`
(42 passed across unit/integration suites, 13 guarded tests ignored), including
the deterministic stolen-guard regression; workspace strict Clippy; UI types,
format/lint and all 1,166 tests, including immediate-save-failure selection
recovery; contract drift; six Edge keybinding scenarios. Reviewed the bulk
selection screenshots in dark, light and high contrast: controls, selected
states and text remain visible without clipping. Evidence:
`target/keybinding-ux/delete-bulk-{dark,light,high-contrast}.png` and
`target/release-0.0.32-checks/consolidated-*.log`.
The health-summary script passes syntax checking and a synthetic fixture
covering run resets, startup-counter discounts, input-stall attribution,
non-stall deltas and a torn final JSON line (`check-summary.mjs` in that
evidence directory). No private audio or live endpoint was used. Docs pass
(295 files/1,640 links). Formatters introduced no unrelated changes.
The browser runner reports its NO_COLOR/FORCE_COLOR environment notice;
no product change is justified by that runner setting.
Next: rebuild clean consolidated candidate inputs, verify artifacts and exact
shell smoke, then update and push PR #181 once with the final combined result.

### PR #181 CI repair and branch reconciliation (2026-11-23)

Observed head `c8f6fbe90`: mergeable, no unresolved review threads, dependency
advisory check failed on `cookie 0.18.2` (RUSTSEC-2026-0594; fixed in 0.18.3).
Plan: update only that shell transitive dependency, run the exact cargo-deny
checks and scoped shell compilation/tests, inspect all PR check annotations,
then push and monitor checks. Do not suppress the advisory or weaken CI.
Rollback: revert the dependency repair commit; earlier candidate hashes remain
historical evidence and do not qualify newly built dependency bytes.

Three recent remote branches have unpublished-to-main work: pipe-guard race,
failed keybinding-delete recovery, and latency/quality documentation. Asked
the owner whether to consolidate them into 0.0.32 or preserve them separately.
No unmerged branch will be discarded. The cleanup-script dry run identified
seven clean merged worktrees and preserved twenty others. Ran the prescribed
script with `-Apply`: removed those seven and their local branches where
present; all twenty others and active candidate artifacts were preserved.

Repair results: only `cookie` version/checksum changed in the shell lockfile
(0.18.2 to 0.18.3); no policy/configuration changes. Both exact cargo-deny
commands passed advisories/licenses/sources/bans, production npm audits for
UI/contracts/Stream Deck reported zero vulnerabilities, all 131 shell tests
passed (four guarded tests ignored), and strict all-target/all-feature shell
Clippy passed. Logs: `target/release-0.0.32-checks/ci-repair-*.log`.
Read all 23 original-head check runs and their one annotation: the advisory
check's exit-code failure (`pr181-annotations-original.json`). Cargo-deny
also reports duplicate dependency versions permitted by `deny.toml` because
the app and Tauri use different upstream API generations; the root graph
reports the five documented shell-only advisory exemptions as unmatched.
These diagnostics remain per the existing explicit policy, not as advisory
waivers for cookie. No new dependency policy exception was added.
Existing installer/smoke hashes below identify the earlier candidate; rebuild
and repeat the exact smoke before final release from repaired/tagged sources.

Version and lock metadata updated without dependency resolution changes.
Complete version/notes diff inspected. Windows checks passed: locked workspace
and shell `cargo check`; UI typecheck, lint, format:check and all 1,165 tests;
contract typecheck/drift (156 methods, 38 kinds, 20 processors, 24 event
categories); docs acceptance (294 files/1,632 links); app-only profile,
artifact-verifier and preparation path-safety regressions; workspace/shell
Rust format checks and `git diff --check`. Full workspace test results and
their log path are recorded below.
Those initial version-only checks predate the transport/UI consolidation and
cookie patch described above; current combined checks are recorded above.

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

After the owner closed AudioRouter, reran
`node tools/quality/packaged-shell-smoke.mjs --shell src-tauri/target/release/audiorouter-shell.exe --output target/release-0.0.32-checks/packaged-smoke-final --timeout-ms 60000`.
Passed: exact candidate launch, native bridge, session save, restart persistence
and graceful quit, with isolated database/WebView profile and no uncaught UI
errors. Evidence: `target/release-0.0.32-checks/packaged-smoke-final/result.json`.
This supersedes the earlier pipe-ownership refusal; candidate shell bytes
remain unchanged. No owner profile or audio was used.

Next: verify and push the consolidated candidate to the already approved
PR #181 and monitor its current-head checks. After merge, create the reviewed
tag and rebuild/verify tagged artifacts for the draft; final publication is
a separate human action. Publication, release tags and release-asset uploads
remain unperformed. Current-candidate installer lifecycle,
Defender, signing, live audio/plugin and broader M08 gates remain open.
