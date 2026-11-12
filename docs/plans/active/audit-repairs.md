# High and medium audit repairs — 2026-11-12

## Objective and authority

Owner requested all three high and twelve medium findings in
[the audit](bug-audit-2026-11-11.md), regression tests, a new branch and a PR.
Branch: `fix/audit-high-medium`, isolated at `target/audit-repair` from
`a2dd275e`. The original checkout's unrelated changes are preserved.
The owner's request explicitly authorizes opening this PR after validation.

Count correction (2026-11-12): the authoritative audit index has three high
and **eleven** medium rows, despite the earlier summary saying twelve medium.
Clarification requested asynchronously about the extra finding (suggested
SES-4 recorder cleanup). Continue every listed high/medium repair while the
owner decides; do not silently mark the original count achieved.

## Requirements and tasks

STATE-02/04/05/07/09/12, REC-01/07/08/09, graph atomicity and DSP dynamics
requirements govern these repairs. No release, driver installation or owner
audio changes are authorized or needed.

1. Reproduce SES-1 and fix deletion ordering (first independent repair).
2. Repair MEDIA-1, STO-5 and SES-2 with dispatcher/storage/failure coverage.
3. Repair DSP-1/2, ENG-1, REC-1, STO-1/2/3, SES-3 and SIM-1.
4. Reproduce STO-4 before changing path security; retain name-surrogate
   escape protections. Record a rejected finding if evidence disproves it.
5. Run focused tests before and after each fix, then formatting, relevant
   Clippy, workspace tests, shell checks, contracts drift and docs acceptance.
6. Inspect the complete diff, commit focused repairs, push and create PR.

## Prerequisites and validation

Windows is available. Builds use one job to limit interference with live
audio; no owner process or session is stopped. Tests use disposable data.
Real OneDrive placeholders, physical endpoints and attended shell evidence
must be distinguished from unit tests. Product compilation and dispatcher
tests do not prove live hardware behavior. Record exact results below.
API/schema changes require contracts and default MCP parity checks. Any
changed user controls require guide and three-theme visual qualification.

## Risks and rollback

Startup isolation must retain unreadable data and keep privacy/device latches
fail-closed. Stop must silence audio despite file failures. Route edits must
preserve ports, matrices, flags and Mixer volume. Backups must not overwrite
existing destinations. Rollback is reverting this branch's focused commits;
do not delete or alter the owner's original checkout or live database.

## Evidence and next action

Initial inspection: original checkout has unrelated tracked and untracked
plugin/UI work; isolated worktree created successfully at the recorded base.
SES-1: `cargo test -p audiorouter-control --lib
refused_delete_preserves_saved_version_notes -j 1` failed before the fix:
the saved timestamp became null. After moving cleanup past all guards and
durable deletion, `cargo test -p audiorouter-control --lib refused_delete -j 1`
passes 2/2, including persisted notes after reopen and comparison expiry.
An attempted native-free `nodes.compare` setup correctly refused an unattached
route; the comparison test seeds only the hold and uses real delete dispatch.
Both workspace/shell formatting commands and checks pass. Control all-target,
all-feature Clippy with `-D warnings` passes (Windows, Rust 1.96.0).

MEDIA-1: dispatcher deletion regression fails because used Soundboard media
was deleted; storage regression fails because pad expiry is retained.
Next: centralize exhaustive node media references and use them in all four
owners, then rerun these failures and broader media/bundle tests.

MEDIA-1 repair: `Node::audio_media_ids` exhaustively classifies all 37 kinds;
`Session::audio_media_ids` deduplicates references. Retention, deletion,
bundle export and missing-asset reporting share these helpers, including
Soundboard sets and disabled nodes, excluding installed-library references.
The catalog test walks described media parameters to prevent schema drift.
Both original failures now pass. The bundle test also proves two sounds
restore and one genuinely missing sound is reported (library assets excluded).
The first post-fix bundle run exposed an empty staging-directory cleanup
omission in the test; explicit non-recursive cleanup now passes.
Windows full library suites: control 367 pass / 11 explicitly ignored;
storage 107 pass. No native endpoint or live-owner session was exercised.
Next: validate Clippy/format and commit MEDIA-1, then SES-2 shutdown failure.

MEDIA-1 committed as `5614e9f8`; domain/storage/control all-target,
all-feature Clippy and both formatting checks passed.

SES-2: the injected node-finalize failure test and changed legacy Quit test
both failed before repair. Audio shutdown is now separate from finalization;
Quit stops every session before any recording finalization. Node and legacy
failures drop encoder ownership, retain failed recovery checkpoints and return
per-recorder failures. Repeated Stop and idempotent Quit remain usable.
Existing tests explicitly expecting a live route after recording failure were
updated to the STATE-09 safety contract. Output schemas and TypeScript now
describe failure outcomes. The default MCP `control_session` test injects a
disk failure and verifies stopped status and the recorder failure response.

Verification: Windows / Rust 1.96.0, one build job; CLI library 50/50 passes
(including default MCP reachability and the new failure case). Control library
368 pass / 11 hardware-dependent tests ignored before the final repeat-failure
adjustment; full rerun pending. Control/CLI all-target/all-feature Clippy,
workspace/shell formatting and contract drift (147 methods, 37 kinds, 20
processors, 22 event categories) pass. API-only and guide discovery catalogs
regenerated without changes. Full site generation revealed unrelated stale
tool-catalog content; that generated-only change was restored rather than
mixed into shutdown behavior. Docs acceptance initially caught the original
audit's link to the other checkout's untracked plugin plan; changed that
historical reference to plain text, without importing unrelated work.

No physical audio, built shell, OneDrive or deadline/finalization timeout
qualification is claimed. Next: finish SES-2 validation/commit, then STO-5
quarantine and the remaining eleven medium findings. Goal and PR scope remain
all fifteen findings; no PR has been opened yet.

SES-2 final local rerun: control library 368 pass / 11 ignored; CLI 50 pass;
control/CLI all-target/all-feature Clippy and both format checks pass.
Documentation acceptance passes 250 files / 1,449 links; diff whitespace check
passes. Full shutdown diff inspected. A bounded finalization-time test and
attended native Stop remain part of final branch qualification, not passing
evidence from these unit tests. Next implementation task: STO-5.

SES-2 committed as `e23b8c57`.

STO-5: a disposable SQLite database with a healthy session, malformed session
and malformed history entry failed `try_with_storage` before repair. Storage
now exposes a bounded usable inventory with quarantined row metadata and a
separate validated-history loader. Strict single-row reads remain strict;
SQL/I/O errors and privacy/device latches still fail closed. Startup uses the
validated current row even when history is damaged. Rows are never rewritten
or removed by quarantine. List, status and diagnostics remain available.
`sessions.export raw=true` and default MCP `export_session raw=true` recover
original text; explicit deletion removes the unavailable row. Contracts,
discovery and authorization describe the optional parameter and output union.

The UI's existing reserved message slot links to session recovery. The panel
exports unchanged text and requires a second deliberate delete press. It uses
existing app styles and does not add fields or alter tool controls.
Windows evidence: startup/dispatcher regression passes (including persistent
privacy mute, raw export, refused Play and deliberate delete). Control library
369 pass / 11 hardware-dependent cases ignored. UI recovery tests 2/2 and full
UI typecheck pass. Default MCP raw-export regression passes. Rebuilt the real
`e2e_backend` fixture; all 3 Edge recovery journeys pass in dark/light/high
contrast, through database fault injection and backend restart, original-byte
download, cancel/delete and healthy-session retention. Inspected all three
screenshots under `target/audit-evidence/recovery-<theme>.png`; controls and
text are readable and unclipped. This is real backend/browser evidence,
not a packaged-shell or hardware run. Playwright's NO_COLOR/FORCE_COLOR warning
is inherited runner environment conflict, not an application diagnostic.
Next: full storage/CLI/UI checks, format/lint/Clippy, contract drift and docs;
inspect complete diff and commit STO-5. Then repair the eleven medium findings.

STO-5 review found and reproduced a second data-loss path: the shell's default
session bootstrap could overwrite a quarantined default ID. Creation now
reserves quarantined IDs, and shell bootstrap uses a reusable, non-conflicting
recovery ID. A full 128-row quarantined inventory also reproduced a startup
failure at the durable session cap; it now permits an explicitly unsaved
in-memory recovery workspace without altering the 128 original rows. Both
fail-first tests pass. Shell `cargo check` passes. All-feature shell Clippy
first required `ui/dist`; the production UI build now passes and reports its
606.99 kB main-chunk advisory. Keep that advisory visible; chunk splitting is
outside this data-recovery repair and is not required for correctness here.
The full UI suite passed 1,003 tests but exposed synchronous exceptions from
the startup-order OBS mock. Fixed that test separately in `9ffc354b` to return
an asynchronous unavailability rejection; its two cases now pass without
React error-boundary noise. Storage 107/107, CLI 51/51, UI lint/typecheck,
contracts drift and docs acceptance pass. Latest control rerun: 370 pass,
11 ignored. Updated native autoplay regression is compiling: automatic Play
must not select another route while recovery is required; deliberate Play
remains available. Final shell Clippy/test evidence is still pending.

STO-5 final evidence: native autoplay regression failed before its guard and
passes afterward; automatic Play now pauses while any quarantined sessions
need recovery, leaving deliberate Play available. Native shell suite: 93 pass,
4 explicitly ignored; all-target/all-feature shell Clippy passes after the UI
build. Workspace/shell formatting checks, UI lint and diff checks pass.
The current-row test also covers well-formed JSON rejected by domain validation
(not only malformed JSON), invalid history, raw-byte preservation, create-ID
reservation, reusable recovery default, privacy mute and deliberate deletion.
No packaged executable or physical endpoint was run for this unit of work.
Next: commit STO-5 and implement DSP-1/2, ENG-1, REC-1, STO-1/2/3/4, SES-3
and SIM-1 (ten listed medium findings remain after SES-1). Extra finding count
clarification remains pending; the original requested goal remains intact.

Branch ancestry review: `origin/main..a2dd275e` contains three unrelated
virtual-cable commits (`ffb94197`, `7c88895c`, `a2dd275e`). After committing
STO-5, move only this branch's repair commits onto the current `origin/main`
so the requested PR does not include that separate driver work. Preserve the
original checkout/branch. Repeat affected validation on the resulting base.

Rebase completed onto `8794a7968` (current `origin/main` fetched 2026-11-12).
The four repair commits are now `37a19810e` (SES-1 coverage), `b47a63849`
(MEDIA-1), `fd928fd91` (SES-2), `b35628358` (STO-5). Original hashes above
describe pre-rebase evidence. Main already contained the SES-1 ordering repair
and a stronger asynchronous OBS fixture; retained the deletion regressions,
adapted the timeout regression to main's per-session comparison map, and
dropped the redundant OBS-only commit. Preserved upstream Copy error UI and
all active-plan entries while resolving conflicts. No driver commit remains
in this branch's ancestry beyond main. Rebased library results: control
382 pass / 12 ignored, storage 107 pass, CLI 52 pass. Reinstalled the new
lockfile's UI dependencies (zero audit vulnerabilities); typecheck passes.
Formatting changed only the adapted comparison test. Native/browser checks
will be repeated on this base before PR handoff.

STO-4 prerequisite found: this PC has a real OneDrive root carrying
`FILE_ATTRIBUTE_REPARSE_POINT` and cloud tag `0x9000701a` (`fsutil reparsepoint
query` metadata-only inspection). No user files were read, created or changed.
This allows a real policy reproduction instead of assuming cloud behavior.

Rebased recovery qualification: rebuilt `e2e_backend`, all three real-backend
Edge recovery journeys pass again (dark/light/high contrast). Contracts drift
passes at main's 148 methods / 37 kinds / 20 processors / 22 event categories.
Both format checks pass; docs acceptance passes 259 files / 1,464 links.
Next concrete repair: REC-1's counting-writer regression and bounded chunk
encoding, followed by DSP-1/2. Native shell qualification on the new base and
the full branch validation remain in the final PR gate.

REC-1: counting `Write + Seek` regression failed before repair with 4,096
writes for 4,096 samples. WAV encoding now reuses a construction-time 16 KiB
buffer in batches of at most 4,096 samples, preserving worker-side flush and
header boundaries. Successful batches advance frame/byte counters together.
The test checks bounded write calls and size, buffer reuse for a larger input,
and byte-for-byte equivalence across batch boundaries for PCM16/24/float32
with dither. Recording library suite: 48/48 pass including the larger-block
extension; all-target/all-feature recording Clippy, both format checks and
diff whitespace checks pass. Inspected the complete focused diff. No realtime callback work, API,
control setting or guide behavior changed. Next: commit REC-1, then DSP-1/2.

REC-1 committed as `5fda58555`.

DSP-1: mono/stereo release regression failed on unequal left/right gains
(`0.100278996` versus `0.100299776`). The actual compiled Limiter stage also
failed bitwise comparison with frame-major reference processing. Added linked
planar processing and frame-major interleaved detection: advance the shared
gain once for the maximum channel peak, then apply it to both delayed samples.
The engine uses this entry point for stereo; per-channel entry is mono-only
and invalid shapes silence rather than passing unprocessed audio. Both
original failures pass. The first engine test tried a private constructor;
replaced it with public `compile_session`, reproducing the failure through the
actual compiler/runtime entry. Full DSP: 81 pass / 1 explicitly ignored;
planar reference suite 7/7, source combinations 5/5 and tool combinations 4/4
pass. DSP/engine all-target/all-feature Clippy, both format checks and diff
whitespace checks pass.
No property schema or setting changed. Refresh Limiter production-audio guide
evidence after DSP-2 in the same PR; do not report guide documentation ready
until that workflow and Firefox playback check have been exercised.
