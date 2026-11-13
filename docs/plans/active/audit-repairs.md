# High and medium audit repairs — 2026-11-12

## Objective and authority

Owner requested all three high and eleven listed medium findings in
[the audit](bug-audit-2026-11-11.md), regression tests, a new branch and a PR.
Branch: `fix/audit-high-medium`, isolated at `target/audit-repair` from
`a2dd275e`. The original checkout's unrelated changes are preserved.
The owner's request explicitly authorizes opening this PR after validation.

Count correction (2026-11-12): the authoritative audit index has three high
and **eleven** medium rows, despite the earlier summary saying twelve medium.
Owner explicitly confirmed fixing only these fourteen listed findings.
SES-4 is excluded; earlier pending-clarification entries below are historical.

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

DSP-1 committed as `4a6671bcb`.

DSP-2: peak-gain probe and brute-force pending-window reference both failed
before repair (`0.5073836` applied instead of `0.5011872` at 8 kHz / zero
lookahead). Added a monotonic deque with construction-time storage for at
most lookahead+1 constraints. It includes the sample leaving this frame;
release is capped by every still-pending peak, including a smaller peak after
a larger one. The final clamp remains a numerical guard, not the limiting
mechanism. No callback allocation or lock is introduced. Both original
failures pass at zero/5/10 ms and 8/48/192 kHz; queue fill/pop, reuse and reset
checks added. Full DSP: 83 pass / 1 ignored; planar reference 7/7, source
combinations 5/5 and tool combinations 4/4 pass, including
`processing_never_allocates_for_any_tool_or_layout` with its counting allocator.
DSP/engine all-target/all-feature Clippy passes. Guide refresh remains pending.

Final shutdown qualification follow-up (SES-2): code inspection shows
`ThreadedRecorderWorker::call` still uses unbounded `recv`, and Drop joins the
writer unconditionally. Audio now stops first on an error, but a stalled file
writer can still exceed STATE-09's Quit budget. Add a stalled-worker regression
and a bounded lifecycle/retirement path before calling the branch ready. This
is actual remaining code work, not passing timeout evidence or an external
hardware blocker.

DSP-2 committed as `d4bcbaaf8`; DSP-1 is `4a6671bcb`, REC-1 is `5fda58555`.
Next task: reproduce stalled threaded-recorder shutdown with a bounded test
and repair SES-2's remaining wait/retirement budget. Then finish ENG-1,
STO-1/2/3/4, SES-3 and SIM-1, update Limiter guide evidence, run the full PR
gate and create the authorized PR. The scope-count clarification remains
open; no goal completion or PR readiness is claimed.

SES-2 stalled-writer follow-up: the original regression timed out waiting for
both finalization and Drop before releasing its fake writer. Threaded encoder
commands now have bounded reply waits and nonblocking queue submission; Stop
and Quit supply one shared ten-second finalization deadline across recorders
and sessions. Timeout closes admission, cancels pending commands and leaves
file ownership on the encoder thread until its current I/O returns. Drop waits
at most 50 ms (and never past the shutdown deadline) for ordinary cleanup;
only an already-finished thread is joined. No thread is forcibly terminated
and no recoverable file is deleted. All production file encoders use this
wrapper; custom nonblocking Rust test workers retain their direct interface.

The original stalled-finalize/Drop test passes. Additional real-dispatch tests
exercise sessions.stop and system.quit with two stalled encoders, using the
same deadline path with a 50 ms test budget. Both return stopped, no active
audio/owned encoders, and individual failed/recoverable outcomes before the
fake I/O is released. Full control: 384 pass / 12 explicitly ignored;
threaded-recorder suite 5/5 passes. All-target/all-feature control Clippy,
both format checks and diff checks pass. Clippy first caught a test-only
post-default field assignment; moved it into the initializer and reran clean.
Inspected the complete worker and deadline-routing diff. This proves
bounded encoder retirement; it is not a claim about an unresponsive OS driver
or a real disk unplug qualification.

SES-2 bounded-retirement follow-up committed as `29b51272e`.

STO-1: storage deletion regression failed because plugin-node bindings remained;
the real `sessions.delete` dispatcher regression failed on the same retained
rows. Deletion now removes bindings atomically with session/history/plans.
The control plane collects binding/current/history state references before
deletion, then reclaims unreferenced automatic captures and slider sidecars
through the same helper used when replacing an automatic capture. Other
sessions, saved versions and explicit reusable library states retain their
assets. Cleanup uses safe IDs inside the canonical owned plugin-state folder;
file/I/O failures or quarantined graphs conservatively retain metadata/assets.
No recording files are deleted. The storage test enumerates session_id tables
so a new owned table needs an explicit lifecycle decision. Both original
failures pass; the dispatcher test also covers shared/history references and
explicit-state preservation. Full control/storage, Clippy and formatting
validation pending. Next: commit STO-1, then STO-2/3 and remaining route fixes.

STO-1 final verification: Windows control library 385 passed / 12 explicitly
ignored; storage library 107 passed. Both workspace and shell formatting checks
and control/storage all-target/all-feature Clippy with denied warnings pass.
Inspected the complete production and regression diff. No live plugin or owner
session was touched. Next: commit STO-1 and reproduce STO-2 backup publication.

STO-1 committed as `a092ae1ab`. STO-3 regression reproduced the incorrect
CorruptDatabase result for version 99. The original migration guard conflated
an unsupported future format with malformed metadata. Storage now distinguishes
NewerSchema and checks it before journal-mode changes, including checked opens.
The regression verifies both open paths leave original bytes unchanged. Negative
versions remain corruption. Control and shell map the new category to an app
update instruction; the shell log category is newerSchema. Focused storage test
passes; shell message qualification and broader checks are pending. Rollback:
revert this focused repair; no data migration or contract shape changes.

STO-3 evidence: full control 386 passed / 12 ignored; storage 107 passed.
Storage/control all-target/all-feature Clippy and default-feature shell Clippy
pass. Shell update-message test passes. This is native shell-unit evidence,
not an attended rollback launch; no owner database was opened.

STO-2: injected write failure reproduced a partial final-name file. The old
online-copy helper wrote directly to the retention candidate and only cleaned
oversized copies. It now reserves a same-directory .partial file with create_new,
copies, checks size and SQLite integrity, syncs, then publishes without replacing
an existing destination. Windows uses MoveFileExW without REPLACE_EXISTING;
portable publication uses an atomic hard link followed by staging cleanup.
Ordinary errors remove only the owned staging name; crash leftovers cannot match
the completed-backup retention pattern. The first Windows run exposed that
sync_all needs write access; corrected the staging sync handle and reran.
All ten backup tests pass, including partial failure, invalid successful copy,
a competing destination appearing before publication, live roundtrip, restore
and a library exceeding the configuration-only budget. No API shape changes.
Next: finish broader checks and inspect/commit STO-2/3; then ENG-1, STO-4,
SES-3 and SIM-1, Limiter guides and the full PR gate.

STO-2/3 final local check: control 386 passed / 12 ignored; storage 109 passed.
Storage/control all-target/all-feature Clippy, both format checks, diff check
and docs acceptance (259 files / 1,464 links) pass. Inspected all changed code
and fault-injection cases. Updated the headless recovery runbook. These two
storage repairs share this commit; no schema migration or public API was added.
Full shell all-feature checks and attended behavior remain in the branch gate.

STO-2/3 committed as `9458e75c8`.

ENG-1: added a direct Mixer -> Duck -> output test through the named mixer
compiler for every flag pattern, including actual block processing. It passes
with the original post-convergence Duck omission too: the branch compiler
consumes the remaining processor. This disproves the proposed observable
failure for this route; retain the audit's authorized structural fix. All
three duplicate chain lists now call one exhaustive classification, so adding
a kind forces a deliberate compiler decision. Disabled-node handling is also
exhaustive: endpoints/generators/Mixers silence; processors, Recorder and
switches preserve their existing dry behavior. No tool setting changed.
The previous divergence arose from independently maintained topology lists;
compiler exhaustiveness prevents recurrence. Source/tool suites and engine
checks are running; full engine Clippy/format and diff review remain before
commit. No physical audio or native endpoint qualification is claimed.

ENG-1 verification: engine library 181 passed / 1 ignored; source combinations
5/5 and tool combinations 5/5 pass, including callback allocation coverage.
Engine all-target/all-feature Clippy, both formatting checks and diff checks
pass. Complete focused diff inspected. Next: STO-4 path policy, SES-3 startup
rollback and SIM-1 rewiring, then guide refresh and full branch qualification.

ENG-1 committed as `dd74a8c54`. SES-3 next: wrap every post-runtime-start
activation step in a single fallible boundary and run one rollback on error,
including application-capture maintenance. Reuse audio-only Stop to retire
all workers and bridges, clear taps, and preserve the original activation error.
Add a dispatcher fault-injection seam at this boundary to prove stopped status
and a usable retry without requiring an actual process-loopback endpoint.
This test does not qualify real Windows application restart behavior. Rollback
is reverting the focused change; no saved session or database format changes.

SES-3 fail-first dispatcher test observed activeSessionCount=1 after an injected
post-start activation error. It now passes with activeSessionCount=0, stopped
runtime and a successful retry with a new idempotency key. All post-start
fallible calls are inside the shared boundary, including the previously bare
application-maintenance `?`; removed hand-copied rollback blocks. Audio-only
Stop retires all workers and bridges even when an individual stop fails;
rollback also clears endpoint/render taps and returns the original error.
Control all-target/all-feature Clippy passes. Full library and formatting
results recorded next; no real application capture was opened for this test.

SES-3 final local results: control library 387 passed / 12 explicitly ignored;
both formatting checks and diff check pass. Inspected the entire activation,
rollback and test diff. Next: commit SES-3, finish STO-4 and SIM-1, refresh
Limiter guide evidence, and run the full branch gate before the authorized PR.

SES-3 committed as `46be4269e`. SIM-1 dispatcher reproduction fails because
inserting a Meter replaces a custom matrix with identity. Repair insertions
using the original source/destination ports and edge flags; retain the original
matrix at the compatible side of the inserted tool. Bridges compose both
edge matrices through the removed tool's dry channel conversion. Move a Mixer
input's volume key with its upstream identity. Refuse ambiguous between edges
rather than deleting several connections. Verify both after/between, enabled
and disabled links, Mixer 40%, Duck key and Input Switch b through dispatch.
Rollback: revert the focused edit; saved graphs need no migration.

SIM-1 repair: the combined dispatcher regression now passes for after/between,
enabled/disabled links, non-default source port, Duck key and Input Switch b,
custom matrix and Mixer input volume 40%, including removal with bridge. A
separate rectangular-matrix test verifies composition of both edges. Its first
fixture used disallowed coefficients above one; corrected to valid 0.2/0.3.
The real default-MCP assistant journey initially caught a mono->stereo->mono
insertion refusal; added dry channel conversion with the original mapping and
a focused mono 40% insert/remove regression. The MCP journey now passes.
No parameters or API shapes changed; updated the API reference behavior text.
Simple task tests 16/16 pass. Full control/CLI, Clippy and drift checks pending.

SIM-1 validation: control 390 passed / 12 ignored; CLI 52 passed including
the real mcp_tool_call assistant journey and default reachability check.
Control/CLI all-target/all-feature Clippy, both format checks, contract drift
(148 methods / 37 kinds / 20 processors / 22 event categories) and diff check
pass. Final review also prevents a stale new-ID Mixer volume from overriding
the old input's implicit 100% default. Focused simple tests pass again after
that adjustment. Complete helper/callsite/test diff inspected. Next: commit
SIM-1, then STO-4 (last listed repair), guides and full branch qualification.

SIM-1 committed as `b11fcdfae`.

### STO-4 qualification: not reproduced on this Windows environment

Real OneDrive root metadata reports cloud tag `0x9000701a` through fsutil and
ReparsePoint through PowerShell. However, Rust 1.96's symlink_metadata returns
attributes `0x31` for that same root, without the reparse bit. The unchanged
RecordingPathPolicy::new therefore accepts it. The suspected failure was based
on native attribute inspection, not the attributes the application receives.
Do not relax security checks based on that inference. Added opt-in, explicitly
ignored-by-default metadata-only qualification tests for recording, storage
backup/bundle paths, session export and credential ancestors. These inspect
the real configured root and never create/read user files or credentials.
Recording and storage probes pass; control/transport probes are running.
These results do not prove every cloud provider, cloud state, OneDrive-backed
known folder, attended record/rename workflow or Windows version. No owning
policy change is justified by the available reproduction. Keep junction and
symlink defenses unchanged. If a real owner failure is reported, capture the
exact Rust metadata and tag at that failing path before revisiting the policy.
This resolves the audit's explicitly reproduction-dependent finding as
unconfirmed, rather than claiming a fix to a demonstrated cloud defect.

STO-4 final evidence: all four opt-in policy probes pass. Session-export probe
first compared a canonical extended Windows path with its plain spelling;
fixed the test expectation, without changing production policy. Recording,
storage, control and transport all-target/all-feature Clippy passes. Both
format checks and documentation acceptance are checked before commit. No
real recording, personal file write, cloud upload or credential access occurred.
Next: refresh Limiter production-audio Properties clips with the existing
portrait capture pipeline, verify Firefox playback, then complete branch-wide
tests/builds/visual qualification and create the authorized PR.

STO-4 qualification committed as `30de5a039`.

## Final branch qualification — 2026-11-12

Scope accounted for: MEDIA-1, STO-5 and SES-2 (high); SES-1, DSP-1/2,
REC-1, STO-1/2/3, ENG-1, SES-3 and SIM-1 (medium) repaired, plus STO-4
qualified as unconfirmed above. SES-1's production ordering repair was already
on the rebased main; this branch retains its regression coverage. No SES-4 or
unrelated original-checkout changes were added.

Windows / pinned Rust 1.96.0, one Cargo build job:

- `cargo test --workspace --all-features -j 1 -- --quiet`: 1,404 passed,
  56 explicitly ignored; complete output `target/audit-workspace-tests.log`.
- `cargo clippy --workspace --all-targets --all-features -j 1 -- -D warnings`
  passes (`target/audit-workspace-clippy.log`). Both formatting checks pass.
- Shell all-target/all-feature Clippy passes; shell tests 101 passed / 4
  ignored (`target/audit-shell-clippy.log`, `target/audit-shell-tests.log`).
- UI format check, zero-warning lint, all-code typecheck and production build
  pass; 115 test files / 1,018 tests pass (`target/audit-ui-tests.log`). Vite
  reports a 610.70 kB main chunk. Keep the advisory: changing unrelated startup
  chunk architecture or raising its warning limit is outside these repairs;
  the actual fresh-profile backend startup check below passes.
- Rebuilt `e2e_backend`; Edge recovery journeys pass 3/3 in dark/light/high
  contrast. Inspected fresh `target/audit-evidence/recovery-<theme>.png` images:
  original-file export, deliberate deletion controls and text are visible.
- Built native debug shell with custom-protocol after the current UI build;
  executable timestamp 17:00:30 is newer than UI dist 16:56:18. Opt-in
  `fresh_install_shell` passes with isolated LOCALAPPDATA and WebView profile;
  real backend answers status.get in 63 ms, device/folder consent stays denied
  to raw callers, and no audio device opens. Disposable shell log reports ready.
  The killed test process leaves a partial backend log line for its deliberately
  denied permission request; do not interpret this as an absence-of-errors soak.
- Contract drift passes (148 methods / 37 kinds / 20 processors / 22 event
  categories); regenerated API-only and tool-discovery catalogs are unchanged.

Limiter guides: rebuilt production-engine fixtures and ceiling/lookahead/release
variants from this branch, recorded the actual Properties panel alone in portrait
with live readings, and refreshed the silent overview and representative demo.
Inspected control-change and late-playback frames: controls remain visible and
the live visual stays at the top. Capture metadata records no page errors.
All six Limiter clips pass real-click Firefox playback, including measurable
output for audible clips. Format/size checker passes all 202 published clips.
Property manifest and generated guide pages were regenerated; only Limiter media,
its guide hashes and preview entry change. Manifest coverage data is unchanged.
The first Firefox run failed because Python was unavailable and its server never
started; reran with a local Node server and all six passed. Guide regeneration
also requires social and CSP regeneration; after both, configured site checks
pass (38 guides / 146 anchors / 1,320 local links/assets and 13 guard tests).
The standalone full-property coverage check still reports `plugin.recovery`
pending. It is outside the explicitly limited audit scope and no plugin control
changed here; retain that honest pending row rather than invent an exemption.
Limiter's three property rows remain recorded with verified refreshed media.

Limits: no physical latency, actual disk unplug, signed driver, release package,
licensed plugin or live microphone qualification. Shutdown stalls use injected
encoder I/O; the native launch test exercises startup/consent, not every repaired
workflow in the packaged UI. Owner processes and personal audio/data are untouched.
Rollback remains reverting the focused commits; no migration is required.
Next: commit guide/evidence refresh, inspect complete branch diff, fetch main,
push `fix/audit-high-medium`, create PR and read its check annotations. Do not
mark merge-ready while required CI is unresolved.

Final integration: fetched and cleanly merged `origin/main` at `0f63709a6`
(the separate update-check improvement). No backend code changed in that merge.
Repeated UI format/lint/typecheck/build and full tests: 117 files / 1,052 tests
pass. Main chunk is now 621.35 kB; the advisory and scope decision above remain.
Repeated real-backend recovery journeys 3/3 pass. Rebuilt custom-protocol shell
after merged UI, and repeated isolated fresh-install native test: pass, first
status response 103 ms. Contracts drift, both format checks, diff check and docs
acceptance (261 files / 1,478 links) pass. Earlier workspace/backend and shell
unit/Clippy evidence remains applicable: that source was unchanged by the merge.
Guide refresh/evidence committed as `38a3e82d8`. No personal work was imported.
Local implementation and validation are complete; next action is the authorized
PR creation, then check annotations and required CI. The unrelated pending
plugin.recovery guide coverage is explicitly retained as a separate owner task.
