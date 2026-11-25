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

Consolidated candidate verification passed from clean source
`0af71ca715333ae6c08c6e58c809f8b1f7bf514e`. All three original remote branch
tips are ancestors of this commit. App-only preparation and artifact verifier
passed for `target/releases/0.0.32-consolidated/release-manifest.json`.
Installer: `AudioRouter_0.0.32_x64-setup.exe`, 24,472,180 bytes, SHA-256
`50BCDFC1F16E7CE4C8A04A6A73387C709B0D148D9AA00481CC3C58A86DB7CEED`.
Shell SHA-256:
`A68CCA0C27037E73FDE2A9FB9923AC2A2C49D901832AE97B664AE37C36AC8360`.
Shell timestamp 2026-11-23 18:26:22 UTC is newer than UI index 18:24:51 UTC.
Exact shell smoke passed: native bridge, save/restart, graceful quit, no
uncaught UI errors or cleanup failure; evidence
`target/release-0.0.32-checks/consolidated-smoke/result.json`.
The original failing shell entry-point regression
`chosen_save_destination_is_approved_and_exported_through_the_real_pipe`
also passed on consolidated sources (`consolidated-shell-entry-test.log`).
Final docs acceptance passed 295 files/1,641 links. This candidate supersedes
the earlier installer for testing; it remains unsigned, driver-free and
untagged. No live audio, installer lifecycle or publication is claimed.
Next: push combined history/evidence to PR #181, monitor current-head CI,
read its annotations and fix remaining failures. Retain original branch refs
until delivery to main; no separate PRs are needed for these three branches.

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

## 2026-11-24 - merged, lifecycle on the consolidated bytes, no tag

Verified state (read-only): PR #181 is **merged** at
`0fb41911bbbc850bc14e26dbcfde1ff72edd0b08` (2026-11-23T20:51:53Z); `main`
contains the 0.0.32 version/notes work after that merge. GitHub has no
`v0.0.32` tag and no 0.0.32 release (latest release is v0.0.31). The "verify and
push" next action above is therefore historical.

Installer lifecycle on the consolidated candidate (**these bytes only**):
`AudioRouter_0.0.32_x64-setup.exe`, 24,472,180 bytes, SHA-256
`50bcdfc1...ceed`, source `0af71ca71`, on the disposable release guest as a
standard user. Every lifecycle step passed (fresh install 6.8 s, production
recording, same-version repair, rollback to 0.0.29, upgrade, uninstall,
reinstall); the 440 Hz stereo 48 kHz recording was retained with SHA-256
`8592C312...E011`. Record:
[0.0.32 release qualification](evidence/2026-11-24-release-qualification-0.0.32.md).
Two deviations: the previous fixture was 0.0.29 because host Defender blocks
reads of the v0.0.31 installer, and offline operation was only partial (CLI,
schema, session read, discovery and MCP stdio passed offline; the window-based
generated-recording restart failed after reinstall and is an open finding).

Follow-up (2026-11-24, second VM session): offline operation now **passes** on
these bytes with the fixed lifecycle harness, including a production recording
after the reinstall; the absent-WebView2 baseline bootstrapped WebView2
`158.0.4370.33` and passed the full lifecycle after its intended offline-exit
failure; the unattended wrapper's cold-boot sign-in was fixed and proved. The
earlier recorder failure was a stale replayed `sessions.play` idempotency result:
the journal survives an app restart while the runtime does not. That product
defect is fixed in `crates/control`: a same-lifetime retry replays the recorded
outcome, and a runtime outcome recorded before a restart is refused with
`idempotencyConflict` instead of re-executing or claiming it is still playing.
Ten dispatcher regressions cover it, but **these installer bytes predate the
fix**; the next build must carry it. The UI Play path was never affected because
it prepares before start. See
[the 0.0.32 qualification record](evidence/2026-11-24-release-qualification-0.0.32.md#follow-up-post-reinstall-recorder-failure-absent-runtime-wrapper-sign-in-2026-11-24-second-vm-session).

Still open for 0.0.32, all human-gated or unreleased-bytes work: interactive
first-run and downloaded-file trust, real endpoint guidance, missing-WebView2 on
a fresh baseline, a second Windows build, a clean Defender verdict, and the
signed-artifact route. A tag, draft or publication is **not** authorized by this
plan and was not created. After a reviewed tag exists, rebuild with
`-ReleaseTag v0.0.32`, verify the tagged artifacts, and repeat the lifecycle on
those bytes; this run does not transfer to rebuilt or tagged bytes.
