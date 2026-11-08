# UI quality review — 2026-11-03

## Round 28 — attended session and scores (2026-11-07)

Evidence: the owner's attended session on the recorded and rebuilt exes
(disposable profile), the fixes listed in [the active plan](current.md), PR
#122, and a full browser run on head `720a6d9d` (688 passed, 15 skipped,
0 failed). A category rises only where today's evidence closes the gap the
iteration-15 table recorded for it; nothing reaches 10 (no independent study).

| # | Category | Score | Basis / what still keeps it from 9 or 10 |
|---|----------|-------|------------------------------------------|
| 1 | Contrast | 8 | No new pixel audit of gradients and boundaries |
| 2 | Typography/size | **9** | Owner checked Windows 150 % and 200 %: nothing clipped or overlapping; new Settings nav checked in three themes |
| 3 | Clarity | 8 | Test Signal Play/Stop and lost-device wording fixed; still no first-time-user test |
| 4 | Consistency | 8 | Custom themes unchecked |
| 5 | Spacing/alignment | 8 | Plugin "?" crowding fixed; no full inventory |
| 6 | Navigation | **9** | Owner's Narrator pass (names read, Tab order sound); Settings search with Ctrl+K hint; keyboard-only route building |
| 7 | Errors/recovery | 8 | Lost device now a red, lasting, plain message; owner has not yet seen it on a rebuilt exe |
| 8 | Inputs | 8 | Copy/paste both ways passed; shortcut capture works; no full field inventory |
| 9 | Buttons | 8 | Compact targets and pressed states not inventoried |
| 10 | Loading/progress | 8 | Messages now close by kind; slow-action matrix still partial |
| 11 | Menus/dropdowns | **9** | Physical keys on the native list (arrows move, Escape behaves as the platform's plain select, nothing applies before Save); tray menu cut to five items with a self-naming Play/Stop, owner-confirmed |
| 12 | Icons | **9** | Owner comprehension check: only Spectral Gate unclear; redrawn and owner-confirmed; Settings pages gained distinct icons |
| 13 | Drag/drop | 8 | Escape cancels card and connection drags; Enter on a Tools card now adds it selected and focused, so the arrow keys place it (`e2e/keyboard-placement.pw.ts`); owner has not yet tried this path natively |
| 14 | Motion | 8 | Scripted scroll honours reduced motion; no attended live-route motion check |

Total 116/140 (was 112). Next, in order: owner check of the lost-device
message on a rebuilt exe (Errors/recovery); a first-time-user walk-through
(Clarity); owner check of keyboard placement of a new tool on a rebuilt exe
(Drag/drop; browser path implemented: added cards were focused but not
canvas-selected, so React Flow ignored the arrow keys).

## Round 27 — dropdown, density and loading evidence (2026-11-07)

**Final disposition:** software phase implemented and verified; the real-key
blocker was cleared by the owner. The [execution record](../archived/2026-11-07-ui-round-27.md)
contains the completion audit and retained native/attended limits. Earlier
pending statements below record intermediate results, superseded by that
record. Scores remain 112/140; M05/M08 and the wider 9/10 goal remain open.

Owner requested the next improvement phase. Round 26's completed fixes and
112/140 rating remain the baseline. Execution prerequisites, ordered tasks,
requirements and rollback are in current.md under UI quality Round 27.

First slice: `native-select-keyboard.pw.ts` exercises ArrowDown on the actual
output-device select, Alt+ArrowDown/Escape, retained focus, Tab departure,
Unsaved indication and absence of any backend commit. At 640 px the field
remains within the viewport. Windows Edge passes 3/3 across dark, light and
high contrast. The initial three failures expected the wrong first option:
the channel-first list puts surround-preview first; correcting that fixture
expectation made the actual keyboard checks pass without product changes.

Inspected all three `target/ui-quality-review/round-27/device-picker-*.png`
captures: field, selected text, guidance and next focused action remain
contained and readable. This proves closed-field rendering and keyboard
behavior in Edge, not the appearance of the OS popup or native WebView2.
Typecheck:all, docs acceptance (246 files/1430 links) and diff check pass.
No production source or tool setting changed; guides/API/MCP are unaffected.

Next: long device-name/duplicate-name fixtures and dropdown cancellation
with a changed provisional option, then remaining menu/density and measured
loading work. Narrator, actual Windows display scaling, attended live audio
and user/icon studies remain open. No score increase, commit or PR claimed.

Second slice results (Windows 11 Home 10.0.26300, Edge/WebView2
157.0.4341.42, Node 22.22.3):

- Long duplicate endpoint names remain two distinct keyboard-selectable IDs.
  All six keyboard/long-name cases pass in development and optimized builds.
  Six narrow/ordinary long-name captures are retained; narrow captures were
  inspected in all themes. Fields contain their contents geometrically, but
  selected long text truncates. This does not establish full-name readability
  in a native popup.
- A stronger provisional-choice cancellation probe failed in the app and in
  a plain HTML select, both headed and headless. Space, Alt+Down and a pointer
  opening did not establish a native popup in that automation path. Removed
  the temporary diagnostic test and narrowed the permanent claim to Escape
  retaining value/focus; native provisional cancellation remains unverified.
- The density suite derives Settings destinations from SETTINGS_PAGES. Its
  22 destinations at 1280x720 and 640x360 pass across three themes, including
  horizontal field containment, 24px button targets and unclipped button
  captions. The expanded-panel extension also passes for every addable tool
  at both widths/themes. JSON inventories are in round-27/density-*.json and
  tool-density-*.json. This is not a full spacing, popup or Windows-scaling
  qualification. Development/optimized initial suites passed 9/9; expanded
  tool development checks pass 3/3. Final combined optimized rerun is pending.
- Added report-bundle.mjs; production entry is 599,241 bytes. Largest module
  transformed-length contributors include App, SessionFlowCanvas, Workbench,
  draft and ToolVisuals. No chunk threshold or boundary changed. The warning
  stays because these samples do not identify a slow startup bottleneck;
  moving bytes without measured improvement would not close UX-06.
- Five native baseline samples on the unchanged E06ADFA6 release hash pass,
  using a stopped three-card route, no device access, separate database and
  WebView profile, standard-user token. Fresh-profile times (ms): 1718,1191,
  1099,1136,1109; restart: 1104,1342,1147,1240,1162. Medians: 1136/1162;
  maxima: 1718/1342. Machine: Core Ultra 9 285K, 24 logical processors,
  33,579,425,792 bytes RAM. These are local fresh-profile/restart samples,
  not OS-cold distributions or an optimization claim. Evidence:
  round-27/native-baseline[-2 through -5]/result.json. All ten shells quit 0.

Native shortcut repair (UI-11/12; F27): actual Windows input through the
computer-use skill reproduced Ctrl+F Find UI, Ctrl+P Print UI and F5 resetting
Session to Tools in the old release. Canceled both browser UIs, never printed
or played audio, and quit the isolated process 0. Native Settings3 now disables
browser accelerators without consuming app keyboard events. Only existing
WebView2 COM ABI packages became direct Windows dependencies; no version changed.
The COM interfaces stay on Tauri's UI thread, and setting read-back emits a
bounded shell diagnostic. Failure keeps the editor usable and is recorded.

The packaged regression requires that record from each current shell PID,
including after restart. Old hash E06ADFA6 fails exactly at the new assertion
(native-keys-old/result.json), then quits gracefully. Current release hash
BB23FDD598A14D719255EA62437771C9D364F83CE985BA929C3C79BE94D54EC4 passes
(native-keys-new/result.json). Shell tests: 91 passed/4 explicit skips; root
and shell strict all-target/all-feature Clippy, both Rust format checks, UI
typecheck/lint/format and helper tests 7/7 pass. New native review initially
hit another app's screen-picker overlay: `point (1003, 196) is over
PickerHost.exe`. Refreshed/retried once and stopped input; asked owner to
dismiss/defer the picker. Cleaned up the owned stopped test shell through its
native quit transport (exit 0), leaving the other app alone. Real-key recheck
and full smoke fault qualification are pending; no repaired-key behavior claim
is made from read-back alone. No public API or tool control changed, so no
contract/MCP/Properties guide update is required. No score increase or PR.

Final available checks: combined optimized suite passes 12/12, including 32
expanded tool kinds in every theme/width; wiring tests 5/5. All eleven native
smoke fault/cancellation cases pass on BB23FDD5. Policy tests pass 61/61;
docs acceptance 246 files/1430 links, final typecheck/lint/format and both Rust
format checks pass. The native record guard was then tightened to require a
timestamp at/after the current launch as well as PID, rejecting reused-PID
stale evidence. Its regression and native-keys-final baseline pass. The full
fault qualification above precedes only this timestamp refinement.

Re-observation showed a Windows network-permission prompt covering the app,
explaining the earlier PickerHost interference. Do not automate security or
privacy permission prompts (computer-use skill). Owner was asked to dismiss
or defer it. The owned review process (4760) was quit through its isolated
native transport, exit 0, without interacting with that prompt. No native
key verification is claimed for the repaired build yet. Next: after the prompt
is clear, use a fresh disposable profile to check F5/Ctrl+R retain the current
page/edits, Ctrl+F/P show no browser UI, custom/app shortcuts and text editing
still work, and native dropdown full-name/cancellation behavior. The goal
remains active, with no category re-rating, commit, PR or release claim.


## Round 26 — fresh evaluation and complete initial canvas (2026-11-07)

Objective: re-evaluate the fourteen categories against the current dirty working
tree, record actionable findings, and repair them in priority order. Preserve
all existing work. Requirements UI-02/11/17/36, ENG-02/04; M05.

Owner decision: opening must show the complete route. This supersedes F20's
readability-first initial crop and updates UI-36. Card positions and audio stay
unchanged. Reproduction: new complete-route browser assertions fail at 1280
and 1000 px before the repair; the 1440 px three-theme cases pass.

Ordered actions:
1. Fit the entire measured route on opening, cap zoom at 100%, verify narrow
   windows and reloads; preserve deliberate zoom during telemetry updates.
2. Rerun focus, shortcuts, toggle states, Settings language, target sizing,
   context menus and SVG text checks in dark/light/high contrast. Re-rate
   categories only against inspected evidence and record any new defects.
3. Repair demonstrated issues, then typecheck, lint, format, run relevant
   unit/browser suites and build. Inspect changes and retain evidence.
4. Record remaining slow-action, packaged-shell and external evidence gates
   explicitly; do not award unverified 9/10 scores.

Prerequisites: Windows, local Node dependencies and Edge available. Harness
uses disposable data and simulated audio; no driver or credentials needed.
Rollback: revert only Round 26 framing/test/spec edits, retain earlier work.
Risks: large routes have smaller labels; zoom-in remains available. No new
tool control or guide recording is introduced.

First evaluation results (Windows Edge harness):
- 43/46 selected category probes passed initially. Focus, navigation shortcuts,
  help, toggles, minimum button targets, ordinary Settings wording and SVG label
  floors passed across dark/light/high contrast. Three context probes failed:
  their first paragraph was collapsed help text, producing an empty selection.
  Target visible support prose and assert a nonempty selection; all three then
  pass. No production context-policy change was needed.
- Complete-route assertions failed before repair at 1280, 1000 and 640 pixels.
  After repair, first-view and canvas aids passed 15/15, including a 15-card
  route and reload after zoom. Screenshot review found the minimap obscured
  the final card at 640; fit now reserves top space for the map, with explicit
  non-overlap assertions. Canvas/context combined checks then passed 18/18.
- Full unit suite passed 983/983 (108 files). Final typecheck, zero-warning
  lint, format:check and UI production build pass. Docs acceptance passes
  (245 files, 1428 links); diff whitespace check passes.
- Build warnings remain actionable: 599.47 kB main chunk and ineffective
  dynamic import of PluginPanels (also statically imported). Do not suppress
  these: investigate chunk ownership in the loading/progress work, measuring
  cold-load behavior before changing boundaries. Browser logs also report
  NO_COLOR/FORCE_COLOR and React Flow attribution/license notices; license
  entitlement has not been established by this evaluation.

Provisional Round 26 scores (out of 10; bounded browser re-evaluation):
Contrast 8; Typography/size 8; Clarity 8; Consistency 8; Spacing/alignment 8;
Navigation 8; Errors/recovery 8; Inputs 8; Buttons 8; Loading/progress 8;
Menus/dropdowns 8; Icons 8; Drag/drop 8; Motion 8. Total 112/140 (8.0).
Spacing and Navigation recover from 7 based on complete opening geometry,
focus and shortcut evidence. Other categories retain the prior ceiling;
this is not a fresh pixel-level contrast audit or native recovery study.

Remaining ordered work: slow/error action matrix and chunk-loading warnings;
native menu/dropdown checks; full contrast/spacing inventory; packaged-shell
startup on disposable route data; Narrator and real Windows display scaling;
attended real-route/device-loss and first-time-user/icon comprehension checks.
The owner's shell is running (PID 7336 at inspection); it was not stopped,
rebuilt or driven. The earlier exe therefore does not contain this repair.
Evidence: target/ui-quality-review/round-26/first-view-*.png and the named
Playwright suites. Next: finish final framing regression after control gutter
refinement, then continue the slow-action/loading matrix. Goal remains active.

Slow-action slice (2026-11-07; UI-04/05/13/17, ENG-02/04): add isolated
commit/lifecycle delays and failures to the route harness, leaving initial load
fast. Exercise real Save, Play and Stop buttons and shortcut requests in all
three themes; check busy accessibility, repeat exclusion, retained edits,
stable canvas bounds, rejected-result feedback and deliberate retry. Rollback
is removal of these fixture hooks/tests only. Save passes 3/3 with a 1200 ms
rejected commit then successful retry. Lifecycle checks are in progress.

Loading warning decision: PluginPanels is already statically loaded by the
tool shelf, Properties and SignalFlowPanel. Make App's PluginScanPanel import
static too, removing an ineffective lazy boundary rather than claiming a split
that cannot occur. This does not increase the dependency closure. Typecheck
passes immediately; verify Plugins navigation and a fresh production build.
The main-chunk budget warning remains under investigation; do not raise its
threshold to hide it. Packaged smoke refuses concurrent production instances,
so the owner's running shell remains an external prerequisite for that check.

Slow-action results: delayed/rejected Save, delayed Stop and rejected Play all
pass across the three themes (6 new scenarios). Each asserts aria-busy,
disabled repeat actions (including keyboard requests), stable canvas bounds,
retained edits/state and successful retry. Plugin inspector plus these checks
pass 15/15 in development. The optimized production bundle passes 12/12
opening/slow-action checks; these three files now belong to the permanent
production-browser suite. Full UI suite again passes 983/983. Lint, format and
build pass; ineffective dynamic-import warning is gone, desktop main chunk is
599.18 kB. The warning remains recorded: splitting an eagerly needed editor
without measuring load cost would only move bytes, not prove faster startup.

Fresh cross-page capture/contrast-candidate/loading checks pass 12/12 at
1440 and 1280 across all three themes (12 pages per capture). Light has no
solid-background contrast candidates. Dark/high contrast each report the
gold-gradient Play caption and Properties 0 dB button as approximate
candidates, because the collector uses the background behind the gradient.
These are not established contrast defects; inspect actual gradient colors
before changing the palette. Evidence is in round-26/*-contrast-candidates.json
and matching screenshots. Remaining menu/icon/navigation review is running.

Menu/icon/navigation review passed 24/24: enlarged/resized menu containment,
Escape focus return, warning dismissal without movement, save vocabulary,
quarantined-plugin contrast, built-in icons and intact navigation captions.
Visual review of the cross-page captures exposed a test-evidence defect:
API's screenshot still showed MCP, because a lazy page transition retained
the preceding committed panel after click. The workbench navigation helpers
now await aria-selected=true for the requested tab before returning. Repeat
the six capture cases and inspect the actual API page; the initial capture
set is not proof of every requested page. This is test synchronization,
not a claimed product defect or a reason to weaken page assertions.

The gold-gradient candidate colors are #1b2026 on #ffd166 to #e7971d in
styles.css. Calculated endpoint contrasts are 11.37:1 and 6.90:1, both above
4.5:1; the earlier solid-ancestor candidates do not establish a defect.
Actual rendered screenshots were inspected for the affected Play and 0 dB
buttons. Scores stay 112/140; no native/user evidence has been substituted.

Corrected cross-page captures pass 6/6; inspected the high-contrast 1280 API
image and confirmed the API heading/controls and selected API tab (not MCP).
Final lint, format:check and diff whitespace checks pass. No commit or PR was
created. PID 7336 remains live; no packaged build or native smoke was attempted
against it. Next: continue delayed/failure checks for scan/import/reconnect,
then package and test when the desktop instance is no longer running. Retain
the recorded external accessibility/usability gates and main-chunk warning.

Scan/reconnect slice (2026-11-07; UI-05/11/13/17, ENG-04): reproduced
PluginScanPanel's pending scan leaving Scan/Load/Retry/Inspect visually enabled
and no aria-busy state. New PluginScanPanel.test.tsx fails on the first enabled
button while its backend request is held. Disable competing actions and mark
the panel busy; preserve directory input after failure. Add browser delayed
failure/retry checks in three themes and verify stable geometry. No API or
tool setting changes; no guide control recording required. Rollback only this
panel feedback change and its regressions. The owner's PID 7336 remains live.

Scan checks pass 3/3 in Edge and the new unit regression passes; screenshots
scan-{theme}.png inspected in all three themes. Startup reconnect passes 3/3
with a failed snapshot then a 1200 ms delayed recovery, no demo graph/Play
exposed before readiness. Full unit run passes 984/984 (109 files).

Audio-import check: slow-audio-import.pw.ts reproduces missing progress text
in dark and light before repair: the picker disables but its status still
describes the preceding file. Set an Importing message in the existing reserved
status area before upload; verify failure retains the old media and retry
replaces it. Requirement UI-05/17; no setting/control semantics changed.
Rollback only that status sentence and fixture/test additions. Rebuild the
UI and embedded debug shell again after this repair before recording provenance.

Final browser results for this slice: optimized production opening, audio
import, scan and startup reconnect pass 15/15. Audio import's three cases
failed first on missing Importing text; after the one-line status repair they
pass, retaining first.wav after a rejected replacement and accepting the
replacement on retry, with unchanged status bounds. Import status screenshots
inspected in dark/light/high contrast. Added all three new failure suites to
the permanent production-browser profile. Typecheck, lint, formatting, UI and
optimized builds, docs acceptance (246 files/1429 links) and diff check pass.
The desktop main-chunk warning remains documented; no threshold was changed.

Fresh embedded debug artifact (not native-run-qualified):
`C:\code\audiorouter\src-tauri\target\debug\audiorouter-shell.exe`, built
2026-11-07 14:05:43 after ui/dist/index.html at 14:05:08, 36,584,448 bytes,
SHA256 `00D37F86A289DA9B75E67A8CDD9A6FBF1323190C1072A3D96403A74C2D512397`.
Command: cargo build --manifest-path src-tauri/Cargo.toml --features
custom-protocol --locked. No Rust source changed. The release exe and owner
process remain untouched. Native smoke requires no concurrent default-pipe
instance. Asked the owner whether to close normally and verify with isolated
database/WebView state, close it themselves, or defer. Await the answer before
closing their app. Scores remain 112/140. Next: final full-unit outcome, then
native startup verification if permitted; attended gates stay separate.

Final full-unit outcome: 984/984 tests passed in 109 files (64.79 seconds).
Available software checks for this slice are complete. The native startup gate
has remained blocked across three consecutive goal turns by the owner's live
desktop instance. No independent repeat checks would add evidence now. Resume
with the pending close/defer decision; never infer approval from elapsed time.

Native continuation: owner chose to close the app themselves. Process inspection
now confirms no shell running. Ran the embedded debug shell through the native
packaged smoke with isolated database, LOCALAPPDATA/APPDATA and WebView2 profile:
baseline passed, native bridge works, audio stopped/device access false before
and after save/restart, both owned processes quit gracefully with exit 0.
Evidence: target/ui-quality-review/round-26/native-smoke/result.json.

Strengthen the smoke's opening-view assertion (UI-02/36): without Fit or manual
zoom, wait for every measured card to be within the native canvas and outside
the minimap, capture bounds and screenshots on initial load and restart. The
debug build passes that check at the actual 1280x800 window: three cards fit
inside a 796x572 canvas. Evidence: native-framing/result.json and
initial-opening-view.png / restarted-opening-view.png; initial image visually
inspected. Qualify against the pre-fix release exe as a negative control before
calling the viewport evidence complete. Existing unrelated onboarding-helper
work in the shared smoke script is preserved. Rollback only the opening-view
helper/calls. No live audio or owner's saved database/profile was used.

Negative-control qualification passed: the old release exe fails exactly at
verifyOpeningView after 10 seconds; failure.png visibly clips the output card
at the right canvas edge. It still quits gracefully with exit 0. Old hash:
ed79695389239d9dce75935ef7582622ac35b2465898c45c64b75a614b2995e1.
The new debug exe passes the same assertion at initial load and restart.
Evidence: native-framing-old/result.json and failure.png (inspected). This is
a verified regression discriminator, not a timeout attributed to the product
without a picture. Syntax check and onboarding-helper test pass. Backend logs
from the positive run contain no outcome:error entries. No shell remains live.

Now build the release exe with the current embedded UI, then apply the same
isolated native smoke and geometry checks to that exact release binary before
handoff. Current-round completion means a new evidence-based rating/list,
the demonstrated fixes and their verification; it does not award 9/10 or waive
the broader attended accessibility/live-route/user-study gates listed above.

### Round 26 outcome and completion audit

The requested new evaluation round is complete: fourteen scores and an ordered
action list are recorded above; demonstrated opening-view, scan-busy and import
feedback defects were repaired and exercised through real user entry points.
UI-02/36: old native binary visibly crops the last card and fails the new
assertion; current embedded release passes without Fit on load and restart.
UI-05/11/13/17: delayed/rejected user actions, retained edits/media, retry,
busy controls, navigation/focus and stable bounds have focused browser evidence.
ENG-02/04: typecheck, lint, format, 984/984 unit tests, optimized browser checks,
native shell builds and docs checks pass as recorded. No new API method or
tool setting exists, so no contract/MCP schema change was required.

Final release artifact for local testing:
`C:\code\audiorouter\src-tauri\target\release\audiorouter-shell.exe`
was built at 2026-11-07 14:16:15, after the embedded UI bundle at 14:05:08.
Size 25,790,976 bytes; SHA256
`E06ADFA6B1F270023260ADA0AAAC6E5AB60E6A229C5B70D1EA06FDD13476F456`.
Exact command: cargo build --manifest-path src-tauri/Cargo.toml --release
--features custom-protocol --locked. Native smoke on this exact hash passes;
evidence: target/ui-quality-review/round-26/native-release/result.json,
initial-opening-view.png and restarted-opening-view.png. Restart image inspected.
Both owned shell processes exit 0 through graceful quit; no uncaught UI errors,
no backend outcome:error entries, and no shell remains running. Standard-user
and onboarding helper tests pass 6/6; docs acceptance passes 246 files/1429 links.
Only disposable data/profile paths were used. Owner's saved route was untouched.

Final bounded rating remains 112/140: all fourteen categories at 8. No 9/10 is
awarded by this round. The broader M05/9-to-10 gate remains open: Narrator,
real Windows scaling, attended live-route/device-loss and first-time-user/icon
comprehension evidence, native dropdown/menu checks, remaining density audit
and measured desktop main-chunk work are the next tasks. They are not implied
complete by this round's fixes. No PR, commit, publication or release was made.

## Objective and authority

Owner requests iterative evaluation and improvement to 9/10 in every category,
preserving AudioRouter's look and palette in dark and light. Capture the actual
rendered UI before/after with browser automation; document criteria, findings,
changes and verification each iteration. Independent model counter-evaluation
is a subsequent owner gate, not evidence this review can award itself.

Branch: `improve/ui-quality-review`, based on the existing
`fix/ci-test-timing-races` checkout. Existing documentation consolidation edits
and prior commits are unrelated and must be preserved. No PR authorization.

Checkpoint: final status inspection found the shared checkout had switched
externally from `improve/ui-quality-review` to `new`; both point to baseline
`33a7a17b5d374637392be2a5ec1d2aca0c5d063a`. No work was committed or lost.
Leave that externally selected branch untouched; before the next source
iteration, establish isolation without moving or discarding the shared dirty
tree. Current evidence refers to the inspected working files, not a commit.

## Requirements and prerequisites

M05; UI-02/03/04/05/07/08/09/11/12/13/15/17/19 and ENG-02/04/05.
Windows host, installed Node/UI dependencies, Edge/Playwright and browser preview
are available. Use deterministic disposable route/backend harnesses; never
contact the owner's route, database or audio endpoints. Harness evidence proves
rendering and interaction only. Native audio, Narrator, Windows scaling and
first-time-user studies remain distinct from browser screenshots/zoom.

Read M05, interface requirements, delivery decisions, production browser runbook
and previous accessibility slice before implementation. Respect global flat
9px fields, beveled buttons, stable live slots and backend-owned validity.

## Scoring criteria (defined before baseline evaluation)

Score each category separately, for each theme. 5 = usable with repeated
friction; 7 = sound foundation with visible or interaction defects; 8 = polished
but a material gap remains; 9 = professional, coherent and verified across the
matrix; 10 = exceptional, independently validated. Unknown evidence is pending,
never a passing score. An average cannot hide a category below 9.

1. Contrast: normal text >=4.5:1, large text >=3:1; meaningful boundaries,
   status graphics and focus >=3:1. Check actual composited colors and states.
2. Typography/size: readable hierarchy, consistent type scale, legible secondary
   labels, units and values; no clipping at normal and enlarged layouts.
3. Clarity: explicit action/result/scope, useful empty states, unambiguous saved,
   draft, running, disconnected and silent states; no false audio claims.
4. Consistency: shared component treatments, vocabulary, alignment, state and
   theme semantics across all workbench tabs and dialogs.
5. Spacing/alignment: predictable rhythm, full-width stacked fields, balanced
   density, no accidental overlaps, overflow or telemetry-induced layout shift.
6. Navigation: discoverable tabs, selected state, keyboard traversal, focus
   restoration, session/Properties transitions and narrow-layout access.
7. Errors/recovery: readable actionable messages with affected object, retained
   input/draft, safe retry/recovery and announced outcomes; no silent failures.
8. Inputs: labels/units/help, precise entry, validation, focus/disabled/read-only
   states, themed file/select controls and correct text editing behavior.
9. Buttons: clear hierarchy/scope, sufficient targets, hover/focus/pressed/
   disabled/busy states, no accidental duplicate actions.
10. Loading/progress: immediate truthful feedback, retained last good content,
    bounded stable placeholders, accessible busy state, retry on failure.
11. Menus/dropdowns: readable selected/options, keyboard operation, containment,
    dismissal, restored focus, long names and all themes.
12. Icons: coherent size/stroke/alignment, familiar meaning, visible labels or
    accessible names/tooltips; statuses never depend on color alone.
13. Drag/drop: clear origin/target/provisional/invalid states; cancel/reject
    preserves graph; equivalent keyboard path; grouping and placement usable.
14. Motion: purposeful, restrained, no false activity/layout shift; reduced
    motion respected; transitions do not restart on telemetry or obscure focus.

## Ordered tasks and validation matrix

1. Capture baseline: Tools, selected Properties, Session, Status, recording,
   settings/API/MCP/logs, dialogs, menus, failure/loading and canvas gestures.
2. Record category-specific findings and scores with screenshot references.
3. Fix demonstrated issues in bounded iterations; typecheck immediately after
   source edits and add focused regressions for defects.
4. Repeat matching captures and compare; verify dark/light/high-contrast at
   1440x1000 and 1280x720, plus 640x360 effective 200% layout and long names.
5. Exercise keyboard/focus, loading/error/disabled states, drag/keyboard
   connection, reduced motion, and stable geometry with telemetry.
6. Run UI format/lint/typechecks, relevant unit/browser suites, production build,
   docs validation and diff inspection. Broaden checks when findings require it.
7. Audit all categories against current evidence before awarding 9; prepare
   reproducible reviewer instructions. Keep goal active while evidence is missing.

Keep screenshot output in `target/ui-quality-review/` with iteration/theme/
viewport/state names; retain a durable evidence record here. No tool parameter
or behavior changes are planned initially; if one changes, follow AUTO-18 and
the guide capture/coverage/Firefox requirements in the same iteration.

## Risks and rollback

Shared CSS has broad impact. Prefer owning selectors/tokens and verify all
themes rather than layering unexplained overrides. Do not let cosmetic work
alter audio behavior or conceal disabled/error states. Revert only this task's
source/plan edits for rollback; no schema migration or device changes.

## Iterations and evidence

### Iteration 0 — baseline

Windows Edge, Node, isolated route harness. Six capture journeys passed:
13 tabs x 2 viewports x 3 themes = 78 screenshots in
`target/ui-quality-review/baseline/`, plus computed-color candidate JSON.
Preview navigation also captured dark/light at 1280x800. No live API/audio.

Confirmed defects before fixing: navigation regression failed in all three
themes (Properties, Recording and Advanced exceed 53.2px available word width).
Secondary-label regression failed in dark/light, passed in high contrast.
Dark app eyebrow on solid topbar = 3.10:1; light Properties badge = 2.41:1;
light guide link = 2.33:1. Candidate audit flags gradients/opacity as approximate
and is not a full WCAG claim. Baseline navigation/typography = 6/10, contrast
= 6/10; other categories remain unscored until their interaction states run.

### Iteration 1 — whole navigation labels and theme contrast

Changed owning CSS: tab columns fit minimum whole-word widths, captions use
12px text and normal word wrapping; preserved three-row layout at 400px panel
width. Separate text/focus/field-border tokens from decorative accent fills;
readable light icons/guide links, themed badge, darker light node metadata and
Ready text. Preserve amber primary buttons and cyan/green/amber identities.

Verification: `npm.cmd run typecheck:all` passed immediately; Windows Edge
`UI_REVIEW_PHASE=iteration-1 npx.cmd playwright test
--config=playwright.review.config.ts` passed 12/12 in 18.5s. Includes six
repeated full-tab capture journeys and six label/contrast regressions. Matching
screenshots/JSON are in `target/ui-quality-review/iteration-1/`. Initial baseline
runner emitted NO_COLOR/FORCE_COLOR environment warnings; removed NO_COLOR
from subsequent runner environment, clean output. No application warning waived.

Remaining: gradients require actual pixel verification; complete dynamic-state,
narrow-layout, long-name, reduced-motion and all-category review before any
9/10 claim. Next: empty search/loading/error/menu/drag interaction captures.

### Iteration 2 — loading and empty search

Baseline initial-session loading lacked aria-busy and sat at the window edge;
the three-theme regression failed before repair. An unmatched search retained
Group/Application/plugin actions and empty headings without saying the search
found nothing (confirmed through browser preview). Added scoped built-in-tool
empty feedback, Clear tool search with restored search focus, centered themed
startup card, truthful loading/error guidance and busy semantics on startup,
lazy panels, Save and route transport. No new audio/API behavior.

Immediate typecheck passed. Three loading/search browser checks passed in 8.3s;
screenshots in `target/ui-quality-review/iteration-2/`. Visual inspection found
the initial centering rule lost to a later app-shell declaration; increased its
owning selector specificity and repeated captures in iteration 3. Custom themes
map the new presentation tokens back to their existing palette keys.

Gradient endpoint analysis found dark node metadata at only 3.63:1 against
the bright gradient stop. Raised its token to yield 5.05:1 there; added a
regression checking both gradient endpoints in every built-in theme. This
complements the approximate solid-background collector, not a full pixel audit.

### Iteration 3 — menus, warning interference and verification quality

Context menu baseline checks failed in all themes: undefined legacy variables
selected a dark fallback rather than the active surface. Replaced them with
owned theme tokens. Escape, Copy and menu Paste return focus to the invoking
node/canvas; session reset retires that focus target. The three menu checks
passed after repair (16.0s), with baseline/iteration-3 screenshots.

The 64-scenario exploratory matrix exposed narrow 640x360 navigation blocked
by a floating unfed-route warning. Its noninteractive text no longer intercepts
gestures; a labelled dismiss action hides this warning until the session or
affected-node set changes. Dismissal never moves the workspace. Existing
combined UX checks reproduce the original blocked Advanced click.

Verification experiment failed: concurrent Playwright commands shared an
artifact directory, causing ENOENT during context teardown. Do not count that
run's 48 passes/9 skips/7 failures as clean evidence. Separate focused/matrix
directories by iteration and run browser suites sequentially. That run also
found an asynchronous selection assertion sampling the previous selected tab;
the test now waits for Properties to be selected after ArrowRight before
reading its panel association. No production wait or timeout was increased.

UI format initially exited 1 with truncated output; targeted CSS check and a
full logged repeat passed. Full UI lint and immediate typechecks passed.
The clean 86-scenario matrix is running; record its result before handoff.

Development matrix result: 76 passed, 9 skipped (kinds without numeric fields),
1 failed at initial Declick fixture loading. Trace network proves the module
`src/backendHarness.tsx` failed with `net::ERR_NO_BUFFER_SPACE`; no rendered
app or backend error. Preserve that trace in iteration-3/matrix-results.
All six combined UX cases, including every 640x360 theme, now passed, as did
all 21 review scenarios. Follow the existing production UI runbook: compiled
fixtures delivered by Playwright routing, with no ESM/HMR network dependency.
Built browser acceptance assets successfully; repeat the same 86 scenarios
using playwright.ci.config.ts and a distinct iteration-3-bundled evidence path.
Do not increase timeouts or call the development run clean.

Optimized matrix passed: **77 passed, 9 skipped, 0 failed in 1.7m** on Windows
Edge. All 21 new review scenarios passed, as did both window sizes of combined
UX in every theme, source/plugin drag, named-node Play recovery, copy/paste,
three-theme fields/bounds and numeric-inspector stability. Nine skips are
explicit kinds with no numeric setting, not successful numeric-field coverage.
Screenshots and contrast candidates: `target/ui-quality-review/iteration-3-bundled/`.
This is compiled browser/disposable-backend evidence, not packaged WebView2,
physical audio, Narrator or attended Windows scaling.

Final typecheck:all and zero-warning UI lint passed. Prettier flagged the new
review config (outside npm's src/e2e formatter scope); formatted it and repeated
the full src/e2e/config check successfully. Documentation validation passed
(238 files, 1,408 links); git diff --check passed. Full unit tests and ordinary
production build are recorded separately in target/ui-quality-review logs.

Full unit result: **821 tests / 93 files passed** in 95.96s with one worker.
The runner prints only its final summary, so an initially quiet log was not a
hang; no test process was stopped or retried. Ordinary `npm.cmd run build`
passed, with a **524.29 kB application-chunk warning**. Do not raise its limit:
remaining startup/performance work should split the production entry where
ownership allows, then verify initial canvas readiness again. This is recorded
as remaining work, not a warning-free production-build claim. Complete UI diff
and the added capture/config files inspected; unrelated documentation work
remains preserved. No commit, push, PR or native artifact handoff in this slice.

Development console warnings are tracked, not waived: React Flow emits an
attribution warning because the established canvas sets hideAttribution=true;
subscription/attribution authority is not established here, so preserve that
owner choice pending disposition. Transient zero-size canvas warnings appeared
during narrow resize/HMR; investigate in optimized static captures before
claiming a warning-free or 9/10 canvas. Vite's incompatible-export HMR message
belongs to development hot refresh, not production runtime evidence.

## Category review after the first three iterations

Provisional assessments, not final acceptance; each needs the full matrix.

1. Contrast: baseline 6, current 8. Proven label/link/metadata fixes; remaining
   colored status graphics, meaningful boundaries and gradients need coverage.
2. Typography/size: baseline 6, current 7. Whole tab words restored; tiny canvas
   metadata and some workbench help still need a deliberate readable scale.
3. Clarity: current 7. Search/loading clearer; stale Plan/Commit language still
   conflicts with the primary Save action (Properties, messages, advanced help).
4. Consistency: current 8. Shared tokens/fields hold; context menu repaired;
   audit all secondary panels and custom-theme behavior next.
5. Spacing/alignment: current 8. Baseline panels are balanced at desktop sizes;
   narrow notification obstruction repaired, full narrow/long-name sweep pending.
6. Navigation: baseline 6, current 8. Whole captions and roving keyboard paths
   work; narrow/long-name paths and focus restoration need final clean evidence.
7. Errors/recovery: current 8. Named-node refusal, preserved geometry and visible
   Play failure are exercised; initial snapshot failure/reconnect still to cover.
8. Inputs: current 8. Global flat fields and bounds/precise entry are exercised;
   full disabled/read-only/focus/native option contrast checks remain.
9. Buttons: current 8. Amber primary hierarchy preserved, busy semantics added;
   compact canvas icon targets and pressed/disabled theme states need review.
10. Loading/progress: baseline 7, current 8. Explicit busy, truthful text and
    stable lazy-panel slot; verify failure recovery and slow action geometry.
11. Menus/dropdowns: baseline 6, current 8. Context theme/focus repaired;
    keyboard dropdown options and screen-edge placement remain to inspect.
12. Icons: current 7. Contrast improved; mixed text glyphs versus SVGs, tiny
    actions and accessible names require systematic inventory.
13. Drag/drop: current 8. Tool and plugin preview/placement tests pass in all
    themes; cancellation/invalid target/keyboard equivalence still to qualify.
14. Motion: current 8. Existing reduced-motion and telemetry design is sound;
    run active/stale/idle and reduced-motion browser captures before scoring 9.

Next iteration: finish clean matrix and regressions; then consistent Save
language, typography/icon target audit and optimized dynamic-state captures.
No category is yet declared final 9/10; goal remains active.

Confirmed next-iteration candidate: `Workbench.tsx` maps icon strings by library
ID but omits `output-switch`, leaving that tool's icon empty. Replace the
unchecked icon dictionary with exhaustively covered kind-based rendering and
consistent SVG strokes, preserving the custom Duck glyph and adjacent labels.
Capture all library groups before/after and add a regression over every library
entry. No NodeKind/API/DSP changes are required for that presentation repair.

### Iteration 8 — plugin quarantine status contrast

The plugin quarantine badge on canvas nodes used a hard-coded pale red
foreground and translucent fill. It failed the review's normal-text contrast
threshold in the light theme and was too small to be comfortably scanned. The
badge now uses explicit semantic tokens: deep red text on a pale red surface in
light mode, the existing warm warning palette in dark mode, and white on black
in high contrast. Its text size is 0.72rem and the border remains visible as a
secondary status cue.

Verification: the initial targeted check reproduced the defect before the CSS
change; the matching post-change browser regression passed dark, light and
high-contrast. The plugin inspector's existing UI-17 fixed-height tests also
passed in all three themes. Evidence is under
`target/ui-quality-review/resume-plugin-before/` and
`target/ui-quality-review/resume-plugin-after/`, including rendered captures.

### Iteration 9 — library icon completeness

The documented icon audit found that `output-switch` was present in the tool
library but missing from `TOOL_ICONS`, leaving an empty visual slot. Added the
same familiar switch glyph used by Input Switch and added a three-theme browser
regression that checks every rendered built-in tool card has either text or an
SVG icon. This preserves the existing palette and card layout while removing a
single-card consistency defect.

Verification: six targeted browser checks passed (plugin quarantine contrast and
all-card icon coverage in dark, light and high-contrast) in 6.6s. Typecheck,
zero-warning lint, the four-test `PluginNodeInspector` unit suite, and the
production UI build passed. The build still reports the existing 524 kB entry
chunk advisory; it remains a performance follow-up, not waived as a clean-size
claim. Evidence is under `target/ui-quality-review/iteration-9-after/`.

Current category update: Contrast advances to 9 for the tested plugin-status
states; Icons advances to 9 for tool-library coverage. Other categories remain
provisional below 9 until the planned loading/error, input, dropdown, drag/drop,
motion, narrow-layout, and full screenshot matrix evidence is complete. The
goal remains active.

### Final review follow-up — bundled fixture route cleanup

Code review found that the compiled-browser fixture's teardown used
`context.unrouteAll`, which removed routes registered by individual scenarios
as well as its own bundle route. Cleanup now unregisters only the matching
fixture handler, waits for that handler's in-flight requests to settle, and
leaves scenario-owned API/network routes intact. The full UI unit suite passed
(821 tests), and the focused browser review passed 27/27 on Windows Edge in
dark, light and high-contrast themes. Typecheck, lint and formatting passed.
The route cleanup does not change application behavior.

Review limitation: the optional Bugbot reviewer could not start because its
configured subagent type is unavailable in this environment; the full diff was
reviewed locally instead. Pull request CI remains the next independent review
gate.

### PR #112 CI repair — bundled route teardown race

The first PR #112 hosted run reported two flaky UI scenarios and failed the
quality gate because the bundled-fixture handler attempted `route.fulfill` after
another page-level route or browser teardown had already claimed the request.
The failure was in `ui/e2e/browser-test.ts`, not application behavior. The
bundled responder now treats only Playwright's explicit `Route is already
handled` teardown error as benign; missing assets and all other routing errors
still fail the fixture.

Verification on Windows: typecheck, zero-warning lint, the related unit suite,
and the two original failing scenarios passed. The exact CI-mode six-test
regression set also passed. A full CI-mode browser run reached 396 passes and
14 intentional skips; it exposed unrelated timing/artifact flakes, and a
serial rerun of all five reported cases passed. The previous UI browser failure
and downstream quality-gate failure are therefore addressed by the route-race
repair; hosted CI remains the final authority after the push.

### PR #112 CI repair — help disclosure activation

The next hosted run reached 400 browser passes but still marked the Quality
gate failed because `compact-workspace.pw.ts` was flaky in the shared help
regression. It used a page-level keyboard Enter after focusing a native
`details > summary`; Edge sometimes retained focus on the replaced lazy panel,
so the disclosure stayed closed in dark/high-contrast runs. The test now
activates the same summary directly and waits for the native `open` attribute
before checking content visibility, then clicks it again to close it.

Verification: the three-theme CI-mode regression passes dark, light and
high-contrast in 8.1s, with typecheck and zero-warning lint passing. This is a
test synchronization repair; no application behavior changes.

### Iteration 10 — Save vocabulary and action clarity

The category audit confirmed a user-facing clarity defect: the top bar used
Save while secondary copy told users to Plan and Commit. That split vocabulary
made the same workflow sound like two different actions. The visible UI now
uses Save consistently for graph changes, imported sessions, Audio File and
Soundboard guidance, application selection, plugin removal, connection edits,
and draft status messages. Backend method names and the plan/commit protocol are
unchanged; only user-facing wording and the import action label changed.

Verification: the focused unit suites passed **108 tests**; typecheck and
zero-warning lint passed. The fresh compiled-browser three-theme Save-language
regression passed **3/3**. The combined UI-quality and session workflow suite
passed **37/37**. The full set covered dark, light, high-contrast, 1280×720,
1440×1000, keyboard focus, loading/search, menu focus, plugin status, tool-card
icons, draft save wording, and stopped-session import. Evidence is under
`target/ui-quality-review/iteration-10/`. The production build passed with the
existing 524.30 kB entry-chunk advisory; it remains a performance follow-up.

Category update: Clarity advances from 7 to **8 for the exercised workbench
and draft-save surfaces**; Consistency advances from 8 to **9 for the exercised
Save vocabulary and import/draft surfaces**. Neither category is final overall:
the remaining 9/10 audit still needs narrow/200% scaling, full error/reconnect,
input/disabled/native-dropdown, drag-cancel/invalid-target, motion-state and
attended accessibility evidence.

## Current-head category audit — 2026-11-04

PR #112 is merged at `e59a1e2c`; its hosted Windows build, Edge browser suite,
and Quality gate all passed. The next review branch starts from that current
`origin/main` so the score audit does not rely on a pre-merge checkout.

The 14-category evidence status is:

1. **Contrast — 8, with a scoped 9 for plugin-status badges.** Solid text,
   links, node metadata and quarantine badges have measured theme regressions;
   gradients, status graphics and all meaningful boundaries are not fully
   pixel-audited.
2. **Typography/size — 7.** Tab words and badge size are covered, but tiny
   canvas metadata, help text, long-name wrapping and 200% scaling need a
   dedicated matrix.
3. **Clarity — 8 for exercised workbench and draft-save surfaces.** Loading,
   search, error and save explanations are clearer; reconnect/retained-draft
   recovery and attended first-use evidence remain.
4. **Consistency — 9 for exercised Save vocabulary/import/draft surfaces.** The
   tested secondary panels now use the top-bar Save vocabulary; action/status
   icon inventory, custom themes and untested panels remain.
5. **Spacing/alignment — 8.** Desktop, narrow warning, inspector and dynamic
   slots have evidence; long-name and full 640×360 state coverage remains.
6. **Navigation — 8.** Tabs and keyboard focus are covered; focus restoration,
   narrow access and the complete dialog path remain open.
7. **Errors/recovery — 8.** Node-not-ready, plugin recovery and search/loading
   states are covered; backend reconnect and retained-draft failure paths need
   matching theme evidence.
8. **Inputs — 8.** Bounds, precise numeric entry and global field styling are
   covered; disabled/read-only/native-option and focus-state inventory remains.
9. **Buttons — 8.** Save hierarchy and disabled/busy states are covered;
   compact target, pressed and action-label consistency remain.
10. **Loading/progress — 8.** Startup busy and lazy-panel slots are covered;
    failure/retry and slow-action geometry need a complete matrix.
11. **Menus/dropdowns — 8.** Context-menu theme/focus behavior is covered;
    native dropdown keyboard, long option and screen-edge evidence remains.
12. **Icons — 9 for tool-library coverage, 7 overall.** Every tool card now
    has an icon, but action/status icon inventory and target consistency remain.
13. **Drag/drop — 8.** Tool/plugin placement passes in themes; cancellation,
    invalid-target feedback and keyboard equivalence remain.
14. **Motion — 8.** Existing reduced-motion and telemetry design are covered;
    active/stale/idle and reduced-motion captures need a unified matrix.

Ordered next work: (1) audit action labels, status icons, disabled/pressed
   states and native dropdowns; (2) add narrow/200% and reduced-motion state
   captures; (3) qualify reconnect, retained-draft and drag-cancellation
   recovery; (4) arrange attended scaling/Narrator evidence; (5) only then
   reassess each score against the 9/10 definition. Unknown evidence remains
   below 9 rather than being averaged away. Rollback is limited to the review
   copy/test/plan edits.

## Continuation — 2026-11-05

Objective: improve the documented menu/navigation/spacing categories through
screen-edge and enlarged-text evidence (UI-02/11/12/17, ENG-02/04/05).
Branch: `improve/ui-quality-2026-11-05`, from `0a5d7677`; preserve the owner's
existing `current.md` edit. This branch isolates the source iteration from main;
use disposable browser fixtures only, with no owner backend or audio access.

Ordered tasks: reproduce menu containment at canvas edges with enlarged text
and narrow viewports; fix demonstrated placement constraints; immediately
typecheck; repeat dark/light/high-contrast captures and keyboard/focus checks;
run formatting, lint, related browser suites, production build and docs checks;
record bounded category changes. Windows Edge is available; browser text/viewport
tests do not replace attended Windows scaling or Narrator. No credentials,
driver or hardware required. Rollback: revert only this iteration's source,
test and plan edits. No API, processing settings or schema changes planned.
Next action: add a failing edge-containment regression before source changes.

### Iteration 11 — measured context-menu placement

Confirmed defect: at 1280x720 with 32px menu text, Paste tool extended below
the canvas in dark/light/high-contrast. All three baseline checks failed the
bottom-bound assertion. The previous placement subtracted fixed 190px/64px
estimates rather than measuring the rendered menu. `CanvasContextMenu.tsx` now
measures its own menu/container before paint and observes their size changes;
the owning CSS bounds large content and permits wrapping/scrolling. It no
longer queries the first canvas in the document. Graph/API behavior is unchanged.

Evidence: `target/ui-quality-review/iteration-11-before/edge-results/` and
matching after captures under `iteration-11-after/` and
`iteration-11-resize-corrected/`. Inspected dark/light disabled Paste and
high-contrast focused/enabled Paste captures. The new six browser regressions
cover enlarged text, disabled Paste, live text/canvas resizing, retained focus,
Enter activation and Escape restoration in every built-in theme. Existing
real-dispatcher copy/paste coverage also passed. Initial 640x360 probes clicked
outside the visible canvas and are not defect evidence; the first resize test
copied the initially selected microphone while asserting a gain-copy ID. The
corrected test deliberately selects Gain before copying; no product change
was made for either test-fixture mistake.

Windows Edge verification: immediate `npm.cmd run typecheck:all` passed;
fresh `npx.cmd vite build --config vite.browser-acceptance.config.ts
--configLoader runner` passed; first combined browser run passed 34/34 and the
added corrected resize checks passed 3/3. `npm.cmd run format` and zero-warning
lint passed. Ordinary `npm.cmd run build` passed with a 529.42 kB entry-chunk
advisory; startup chunk splitting remains a performance task, with no increased
warning threshold. Final combined compiled-browser run passed **37/37 in
43.6s**, with evidence under `target/ui-quality-review/iteration-11-final/`.
Final format:check, zero-warning lint and typecheck:all passed. Docs validation
passed (241 Markdown files/1,410 links); diff whitespace passed. The full unit
suite and native/packaged gates were not run for this presentation-only slice.

Category disposition: **Menus/dropdowns advances to 9 for canvas context-menu
containment, theme/focus and keyboard activation**, including enlarged text and
an open-menu resize from 1280x720 to 1024x720. Overall Menus/dropdowns remains
8 pending native dropdown/long-option evidence. Spacing/alignment and
Navigation gain regression evidence but retain their overall 8; typography's
7 and the other category scores above are unchanged. This does not qualify
full 200% text/scaling, 640x360 navigation, Narrator, native audio or packaged
WebView2. No native executable, commit, push or PR was produced.

Next task: complete native dropdown keyboard/long-option and compact-action
state inventory, then the narrow typography matrix. Preserve the owner route
and the unrelated `current.md` and release-note edits observed in the shared
checkout. Rollback only the iteration-11 files/edits.

Handoff checkpoint: final inspection found an external checkout switch to
`release/0.0.27`. The iteration's source/test/plan edits remain present and
uncommitted. Leave that branch selection and release work untouched; the
review branch above records where this iteration began, not the final selected
branch. Re-establish source isolation before the next iteration.

### Iteration 12 — owner-reported slider spacing

Objective: reproduce and repair excessive separation between slider captions,
values and tracks, starting with populated Soundboard slots (UI-05/11/12/17;
review categories Spacing/alignment, Inputs and Consistency). Preserve all
iteration-11 edits and external release work on the shared checkout; no branch
switch, commit, PR, owner-profile or live-audio operations.
Tasks: reproduce in disposable Soundboard fixture; inspect shared range layout;
add a failing geometry regression; repair the owning layout; immediately
typecheck; compare dark/light/high-contrast at desktop/narrow widths, keyboard
value changes and stable row geometry; refresh affected guide media/coverage;
run relevant suites, format/lint/build/docs and inspect the diff. Windows Edge
is available. Browser evidence is not Narrator/native scaling qualification.
Rollback only iteration-12 edits; no API/schema/audio changes are planned.

Reproduction: populated Soundboard slots in the disposable route harness show
an excessive caption-to-track gap in every theme. All three baseline checks
failed: the input box began 11.98px below its caption, and a more-specific
legacy `.node-binding-editor input:not([type="checkbox"])` rule imposed the
39.2px text-field minimum on the range. The visible native track was centered
inside that unnecessarily tall box. The more-specific inspector range rule
also overrode Soundboard's zero-margin intent.

Repair: the owning inspector range rule now sets height/min-height to 1.5rem
(24px at the default type size) and zero margin; the label/control owns the gap.
This keeps a usable pointer area while bringing the track closer to its value
caption. Existing parameter-row and vertical Graphic EQ overrides remain
effective. No settings, parameter limits, API, contracts or DSP behavior changed.

Windows Edge evidence: fresh compiled acceptance assets and immediate
typecheck:all passed. `npx.cmd playwright test --config=playwright.ci.config.ts
soundboard-editor.pw.ts --workers=1` passed **8/8 in 17.6s**. New regressions
cover dark/light/high-contrast, 1280x720 and 640x360, caption/input gap <=8px,
24px input height, Home/End reaching -24/+6 dB with the correct accessible
value label, and unchanged slot height. Existing playback-state, file-drop and
picker journeys passed. Evidence/screenshots:
`target/ui-quality-review/iteration-12-before/` and
`target/ui-quality-review/iteration-12-soundboard-final/`. Visually inspected
dark/high-contrast desktop and light narrow captures. First margin-only repair
passed the gap assertion, but screenshot inspection caught the remaining
text-field minimum; added the 24px regression before completing the repair.

Related compiled-browser run (UI review, inspector stability and canvas groups)
passed **62 tests, 9 intentional nonnumeric-kind skips, 0 failures in 1.3m**;
evidence: `target/ui-quality-review/iteration-12-related/`. Format,
format:check, zero-warning lint, typecheck:all and production build passed.
The production entry remains 529.42 kB with the recorded chunk advisory; do
not raise its warning limit. Full unit suite, native app/audio, Narrator and
attended Windows display scaling were not run.

Guide refresh: recorded the actual Soundboard, Duck and Group Properties
panels as 720x1280 silent configuration tours using the existing browser
recording pipeline, returning to the top after showing lower controls. Updated
their overview MP4/JPGs and the coverage manifest's presentation-refresh record;
preserved existing audible clips. Regenerated guides, social metadata and CSP;
only the three affected guide pages change, through their media content hashes.
`check-guides.mjs` and `check-tool-guide-property-coverage.mjs` passed (38 kinds,
252 properties). `GUIDE_KIND=soundboard,duck,group node
tools/site/check-tool-guide-video-firefox.mjs` passed **all 14 selected clips**,
including the three refreshed overviews; evidence:
`target/tool-guide-firefox-results.json` and
`target/tool-guide-overviews/{soundboard,duck,group}/capture.json`.
Docs validation passed (243 files/1,416 links); diff whitespace passed. No site
publication, native build, commit or PR. The disposable Vite server was stopped.

Category update: Spacing/alignment and Inputs now have **9/10 evidence for
Soundboard slider caption/track proximity, keyboard bounds and stable rows**;
their overall scores remain 8 pending the broader inventory. Consistency gains
the shared-rule repair; no other overall score changes. Next: resume native
dropdown/long-option and compact-button inventory, then narrow typography.
No owner input blocks that independent work. Preserve unrelated shared release
edits and iteration-11 work; undo only this iteration's CSS hunk, test addition,
guide media/pages and manifest/plan entries if rollback is needed.

## Whole-interface continuation — owner correction, 2026-11-05

The owner rejects treating the slider as an excellent experience and explicitly
requests continued work across the entire UI. The iteration-12 scoped 9 is
superseded: geometry checks establish only a repaired gap, not overall slider
quality. No component or overall category is awarded 10 by this agent; the
rubric requires independent validation and the owner's experience is evidence.

Objective: complete an actionable pass through all 14 categories across Tools,
Properties, Session, Status, Apps, Cables, Recording, Settings, MCP/API/Logs,
dialogs and canvas states. UI-02/03/04/05/07/08/09/11/12/13/15/17/19,
ENG-02/04/05. Preserve shared uncommitted work and external release changes.
No owner profiles/audio, publication, PR or native build implied.

Ordered work: (1) improve slider readability, exact entry and discoverable reset;
(2) audit/fix small canvas metadata and compact action targets; (3) cover native
dropdown keyboard/long names, all-panel narrow layout, loading/errors/recovery,
drag cancellation and reduced motion with current compiled fixtures; (4) run
related/full browser and useful unit checks, inspect screenshots/themes and
complete diff; (5) refresh affected guide recordings/manifest and Firefox checks;
(6) update each category with evidence, defects and remaining gates. Follow
findings through repairs rather than ending after the first passing slice.

Prerequisites: Windows/Edge and dependencies available. Disposable backend and
browser-only checks can proceed independently; Narrator/attended Windows
scaling, first-time-user evaluation and live audio remain separate. Rollback:
only this continuation's source/test/docs/media edits; no schema migration.

### Iteration 13 — whole-interface findings and repairs (verification running)

The previous gap-only slider assessment is withdrawn. Soundboard now pairs its
full-width caption with a slider and exact-value field on one row, dB units,
track-aligned minimum/maximum labels, and a visible Reset to 0 dB action. It
retains the app's global flat field style. Invalid out-of-range exact entry
does not change the slider; reset and keyboard bounds are exercised. Shared
numeric captions place their value next to the caption rather than across the
entire panel. This is a design iteration for owner evaluation, not a 10 claim.

Additional reproduced defects:

- Canvas family/kind/status/port text used 8.96–10.72px type. The three-theme
  baseline failed the 11px review floor; owning rules now use 11.2–11.52px.
  Delete targets grow from 21.6px to 28px before canvas zoom.
- Long titles contributed intrinsic width and overlapped the next node,
  obstructing clicks. Bound title intrinsic sizing, permit wrapping and expose
  the full name as a tooltip. The desktop long-name journey now passes.
- At 640x360 the canvas was 300px high but React Flow's renderer was 0px high;
  zoom controls appeared behind the legend. Override React Flow's inline
  percentage-size defaults with an inset renderer that fills its canvas.
  The regression measures the renderer itself, not merely node dimensions.
- Failed reconnect kept the draft but still said Audio stopped and allowed
  Save/start against a stale snapshot. Retain draft/snapshot, label status
  unavailable, disable graph save/start/flags and dependent editors, cancel
  pending autosave and resume after successful refresh. Refresh and emergency
  Stop/privacy actions keep their original transport. No API/DSP changes.
- Escape during a Tools-origin drag left a provisional canvas node. Clear the
  preview on the document's dragend, where a native cancellation actually
  finishes. Three-theme cancellation now preserves the graph without a ghost.

Current bounded evidence: 15 whole-interface checks passed in 21.7s (all 13
tabs in three themes at 1280x720 and 640x360, long names, metadata/target audit,
native dropdown keyboard/focus and retained-draft reconnect). Six motion/drag
checks passed in 5.0s after fixing a test selector that incorrectly selected
the hidden legend's sample edge. The baseline drag checks failed in all
themes before the source repair. Evidence directories are
`target/ui-quality-review/iteration-13-{before,narrow-geometry,recovery-before,
whole-interface,motion-drag,motion-drag-after}/` (individual directories,
not a literal brace-named path). Narrow content captures are being tightened
to scroll the selected panel into view rather than treating off-screen content
as a visual pass. Document scrolling remains part of the stacked layout.

823 unit tests / 93 files passed in 110.87s; typecheck, zero-warning lint and
contract drift passed (138 methods, 37 kinds, 20 processors, 22 categories).
The 448-scenario compiled browser run has since been superseded; see iteration
15. Two existing tests clicked node centers to select/focus; changed card metrics put that click on
the embedded gain slider. They now select the title and focus the node before
clipboard/movement commands. Preserve the initial failures and recheck them;
do not call the initial full run clean. Formatting an extra context provider
initially indented a large JSX subtree; storing that subtree as a local value
keeps the final behavioral diff focused instead of mixing mass indentation.

Next: finish the full run, investigate every failure, test connection-menu
theme/keyboard behavior, refresh affected Properties tours and Firefox playback,
then update all 14 category dispositions with remaining gaps. No overall 9/10
or 10/10 completion is being asserted.

### Iteration 14 — one slider, one type scale, audit repairs

All range inputs now render through `ui/src/Slider.tsx`, which a source test
enforces. It gives one track, thumb and fill rule, exact entry, unit, a reset
that keeps its slot, and a logarithmic track for wide Hz and ms ranges. Other
repairs: toasts moved off the Properties panel, about 14 font sizes reduced to
one scale with an 11 px floor, jargon removed, the unclear driver line
replaced, Delete session styled as dangerous, and disabled card hints made
readable. Evidence: `ui/e2e/slider-control.pw.ts`,
`ui/e2e/ui-typography-inventory.pw.ts`, `ui/src/Slider.test.tsx`.

Full compiled browser run during a parallel cargo build: **442 passed, 10
failed, 14 skipped** (`target/ui-quality-review/iteration-14-full.log`). The
test build handed to the owner at 18:57 predates the repairs below.

### Iteration 15 — full-run failures and the owner's window-gap report

Failures from the iteration 14 full run:

- Compressor and Gate editors, six scenarios: a real 4 px layout shift (UI-17),
  caused by iteration 14. The `.muted` placeholder took the 12.8 px scale while
  measured text stayed at 12.16 px, so `6em` differed. The slot now holds the
  longest text in an invisible `::before` sharing its grid cell, so it fits at
  any panel width. The first version showed a large empty area, so the quiet-voice
  advice was shortened.
- Canvas menu, three scenarios: the right-click in the test landed on the Dismiss
  button of the "copied" toast, which now floats over the canvas corner. The test
  dismisses the toast first. A button keeping its own clicks is intended; a
  right-click on that button does not open the canvas menu.
- Audio file import, one scenario: "an audio upload is already in progress" once
  under load. Not reproduced alone or in 16 of 16 repeats at 6 workers. Each test
  has its own backend and a failed import releases the slot, so **the cause is
  unexplained**. Watch it in the next full run.

Owner report, 2026-11-05: the five gaps framing the workspace were unequal.
Measured at 1280, 1440 and 1920 px: left 20, right 20, top 16, middle 16, bottom
32. All now use `--workspace-gap: 1rem`. `ui/e2e/workspace-margins.pw.ts` failed
before the change and measures all sides on two tabs in three themes.

Also found in the owner's screenshot:

- A drag across the tab strip highlighted tab names as text. Reproduced by
  `ui/e2e/sidebar-resize-selection.pw.ts`, fixed with `user-select: none`. The
  panel handle was tested and was not the cause.
- The Soundboard help had a second scrollbar, because iteration 14's larger text
  no longer fit its fixed 4.8rem slot. It now has a minimum height and grows.

Verification on Windows Edge: 120 of 120 targeted browser scenarios at 2 workers
(`target/ui-quality-review/iteration-15-targeted.log`), 847 of 847 unit tests in
94 files, format, zero-warning lint and typecheck. **Not rerun after these fixes:
the full browser suite, the release build, and packaged smoke**, deferred because
the owner was using live audio. Guide videos are still not refreshed, docs
validation is not run, and the 529 kB chunk advisory stays open.

Category scores after iteration 15. These are this agent's scores against the
criteria above, and no theme scored differently from another. Custom themes are
not covered. A 10 needs independent validation, so none is awarded.

| # | Category | Score | What keeps it from the next level |
|---|----------|-------|-----------------------------------|
| 1 | Contrast | 8 | Gradients and all boundaries are not pixel-audited |
| 2 | Typography/size | 8 | Windows 200% scaling untested; zoomed-out canvas cards unreadable |
| 3 | Clarity | 8 | No first-time-user test |
| 4 | Consistency | 8 | Owner still finding inconsistencies; custom themes unchecked |
| 5 | Spacing/alignment | 8 | Gaps just fixed and unconfirmed by owner; orphan row in tab strip |
| 6 | Navigation | 8 | 13 tabs on 3 rows take about 150 px; Narrator untested |
| 7 | Errors/recovery | 8 | Live device failures not exercised in this review |
| 8 | Inputs | 8 | Owner has not yet judged the new slider |
| 9 | Buttons | 8 | Compact targets and pressed states not inventoried |
| 10 | Loading/progress | 8 | Slow-action geometry is not a complete matrix |
| 11 | Menus/dropdowns | 8 | Canvas menu is 9; long native options and Narrator open |
| 12 | Icons | 7 | No action/status icon inventory yet |
| 13 | Drag/drop | 8 | Cancel fixed; no complete keyboard equivalent for placement |
| 14 | Motion | 8 | No attended check on a live route |

Later on 2026-11-05, with the app closed, the full browser suite ran on this code
(**463 passed, 1 failed, 14 skipped**,
`target/ui-quality-review/iteration-15-full.log`), a new exe was built (SHA-256
`8ce9779e…1004a`) and packaged smoke passed on it.

The one failure, "a processing output can feed more than one destination", passed
5 of 5 alone and failed 12 of 12 at 6 workers, with the old gaps restored as well,
so the gap change did not cause it. Logged positions show the input card 10 to 21
px from Gain alone and 0 px under load, which puts both connection dots on one
spot. The test now spaces its cards widely and passes 8 of 8 at 4 workers. Why a
dragged card lands a few pixels short under load is not explained.

### Iteration 16 — group sides are hard to grab (owner report, 2026-11-05)

Each side's grab zone was the library's line, one canvas unit wide, so less than
a screen pixel when zoomed out. All 16 side-drag scenarios failed before the fix.
Sides are now a 16 px zone on screen at any zoom (9 px outside, 7 px inside, so a
side stays reachable beside a neighbouring group), and the line thickens on hover.
The caption stays above the top zone and corners above the side zones. Tool cards
stay on top by design, so a side is not grabbable where a card overlaps it. Help
text and `docs/operations/quickstart.md` now say "any side or corner".

Evidence: `ui/e2e/canvas-group-resize.pw.ts`, 22 of 22 (each side from outside and
inside at two zoom levels, opposite side and tools unmoved, caption and corner
drags, locked group, hover in three themes, screenshots inspected). The 26-scenario
group run before the test's card-avoidance change passed its other 24, and
`CanvasGroups.test.tsx` passed 5 of 5. The Properties panel's controls did not
change, only the collapsed help text, so no property clip needs a new control.

Not run after iteration 16: full browser suite, full unit suite, a new exe, and
live testing on the owner's route. Drag/drop stays 8: keyboard placement is open.

With the app closed, the full browser suite on iteration 16 passed 486 with 14
skipped, and all 847 unit tests passed.

### Iteration 17 — grey stub at the left of a slider (owner report, 2026-11-05)

The fill started at the thumb's lowest center, half a thumb from the rail's end,
leaving about 9 px of unfilled rail once the thumb moved away. Introduced by
iteration 14. A fill starting or ending at a limit now runs to the end of the rail.
`ui/e2e/slider-control.pw.ts` reads real pixels; it failed in all three themes
before the fix and passes after. Canvas card faders were not affected.

### Iteration 18 — "UI error (unknown)" in the Logs tab (owner report, 2026-11-05)

Reproduced in Edge: dragging the panel handle fires `ResizeObserver loop completed
with undelivered notifications` with no error object, which was logged as `UI error
(unknown)`. In the packaged app the page address has no file name, so the location
was missing too. The owner's rows hold no message text, so their exact cause cannot
be proved; the backend and shell logs show no errors from 19:16 onward.

The notice is no longer logged. Other uncaught errors now show type, place in the
bundle and a fixed description for known browser wording. Raw text stays out, per the
log privacy rule, so an unrecognized error shows only type and place. Identical rows
collapse to one per 30 seconds, and old `UI error (unknown)` rows are dropped on read.
Evidence: `ui/src/clientDiagnostics.test.ts`, `ui/src/App.accessibility.test.tsx`, and
`ui/e2e/sidebar-resize-selection.pw.ts`, which requires the real notice to fire and
failed on the old bundle. The ResizeObserver source itself was not removed.

### Iteration 19 — resize several groups; undo for resizing (owner request, 2026-11-05)

Probe before changes: with two groups selected, only the dragged one changed. After
a group resize, Undo layout worked but Ctrl+Z did nothing, and right-panel resizing
was in no history. "Panel" in the request could mean a group or the right panel, so
both are covered.

Decisions, recorded in `docs/operations/editor-invariants.md` and spec
`docs/spec/09-interface.md`:

- The top-bar Undo and Redo and Ctrl+Z/Ctrl+Y take the newest change, route or layout
  (`ui/src/editOrder.ts`). The two histories stay separate and each restores only its
  own data, so a route undo is never replaced. Undo layout and Redo layout stay
  layout-only. **Behavior change:** Ctrl+Z after moving a card now undoes the move first.
- Dragging one selected group moves the same edges of all selected groups by the same
  distance, one undo step (`resizeGroupsTogether`). Locked groups stay; a group at its
  minimum stops with its other edge in place. Equal distance is this agent's choice over
  proportional scaling; the owner can overrule it.
- Right-panel width joined layout history: one step per drag, key press or double-click.

Evidence: `ui/e2e/canvas-layout-undo-resize.pw.ts` (5 of its 6 scenarios failed before
the change; 18 of 18 after with 3 repeats at 4 workers), `ui/src/editOrder.test.ts`,
and additions to `canvasGeometry.test.ts` and `canvasHistory.test.ts`. No backend, API or
MCP change: all of this is local presentation state. Guide videos are still not refreshed.

Defect found by the full suite: `api-request-builder.pw.ts` failed once under load with
the send applied (canvas at 3.0 dB) but no "Sent" message. Not caused by this iteration
and not a flake: the builder cleared its status and reset its value box whenever a
backend refresh delivered a new `node` object, so a refresh landing after the send erased
the message, and any refresh replaced a value being typed. It now compares plain values:
the status clears only when the chosen setting changes, and the box reloads only then or
when the saved value changes. Regression: `ui/src/ApiRequestBuilder.test.tsx` forces the
order, so it does not depend on load (2 of 4 failed before the fix).

Next: refresh guide videos,
inventory icons and compact buttons, and review the tab strip. Rollback: revert
only this review's uncommitted UI, test and plan edits.

## Round 20 — re-rating and 10/10 gap discovery (owner request, 2026-11-05)

Objective, changed by the owner: find what stands between the current UI and a 10/10
experience in every category, and record it to tackle later. No product code changed in
this round. The earlier 9/10 target in "Objective and authority" is superseded by 10/10.

**Method.** Compiled route harness at 1440×1000 and 640×360: all 13 tabs in three themes,
axe-core 4 (WCAG 2.0 to 2.2 AA plus best-practice, run from a scratch folder, not added to
the project), control and type inventory, Properties for the 33 tools that can be added,
keyboard walk, 11 expected shortcuts, first-task and playing-route journeys, and a source
read of each finding. Screenshots and JSON: `target/ui-quality-review/round-20/` (not
committed). The audit specs were temporary and are deleted.

**Not covered, so no score below rests on them:** the real exe on real devices, Narrator,
Windows display scaling, custom themes, Firefox, and any first-time user. Harness audio is
simulated. Findings marked *verify in exe* come from the harness or source only.

### Scores

These are this agent's grades. The rubric reserves 10 for independent validation, so no
10 can be awarded here. Most scores fell from the previous table. The UI did not get
worse: the earlier 8s meant "no known defect in what was tested", and this round tested
what a user meets rather than what had just been fixed.

| # | Category | Was | Now | Main reason | Findings |
|---|---|---|---|---|---|
| 1 | Contrast | 8 | 8 | axe clean on all 13 tabs in 3 themes; graphics not pixel-audited | F21 |
| 2 | Typography/size | 8 | 7 | Chart labels 8.5 px; canvas text under 11 px at normal zoom | F20 |
| 3 | Clarity | 8 | 6 | READY beside "Choose a device"; mute unseen; developer words | F1, F2, F7, F8 |
| 4 | Consistency | 8 | 7 | Two meanings of Undo; four names for one tool; native dialogs | F5, F7, F6, F19 |
| 5 | Spacing/alignment | 8 | 7 | Connection controls cover cards; toast covers tabs when narrow | F9, F10 |
| 6 | Navigation | 8 | 6 | 13 flat tabs; panel state lost; 85 Tab stops; session switch hidden | F3, F11, F12, F13 |
| 7 | Errors/recovery | 8 | 7 | Play error is good but offers no action; warning never leaves | F14, F15 |
| 8 | Inputs | 8 | 7 | Toggles 22 px, state unlabeled; New session is a browser prompt | F6, F16 |
| 9 | Buttons | 8 | 7 | Reconnect always shown; Stop plain; button reads as a state | F2, F17, F18 |
| 10 | Loading/progress | 8 | 8 | No new defect; slow-action matrix still incomplete | F27 |
| 11 | Menus/dropdowns | 8 | 8 | Canvas menu strong; native lists and long names unaudited | F27 |
| 12 | Icons | 7 | 6 | No icon set: tool and action icons are Unicode characters | F22 |
| 13 | Drag/drop | 8 | 7 | Keyboard connecting is a form inside Advanced; no alignment help | F23 |
| 14 | Motion | 8 | 8 | Comets, stepped meters and 8 reduced-motion rules hold; not attended | F27 |

### Findings, most important first

Each has a done-when test. "Effort" is S (hours), M (a day or so) or L (several days).

**P0: the app says something untrue, or hides something that matters**

- **F1. READY contradicts the card (S).** Every tool except an application capture is
  labeled `ready` whatever its state (`SessionFlowCanvas.tsx:1554`), so a card shows
  green READY beside red "! Choose a device", "No device chosen" and "no input reaches".
  Done when the chip is derived from real state (no device, no input, unsaved, running,
  failed) and a test covers each state.
- **F2. Privacy mute cannot be seen (M).** With mute on, the top bar still says "Audio
  running" in green. The only visible words are at the bottom of a tool's Properties, so
  with no tool selected, nothing says the mic is muted; connection labels say "muted"
  only to a screen reader. Mute starts true until the backend answers (`App.tsx:343`). The
  control is a global safety latch placed among per-tool actions, and its button reads
  "Privacy mute enabled", which is a state and not an action. Someone can talk on Discord
  with no voice going out and see only green. Done when mute shows in the top bar and on
  affected cards in every theme, with one clear on/off control. *Verify in exe.*
- **F3. Session switching is hidden (S).** The top-left session name is a borderless
  button whose only hint is the tooltip "Open Session", and it opens a tab rather than a
  list. Done when the name looks like a control and switches sessions directly.
- **F4. Unsaved work at quit (M, *verify in exe*).** Closing the window goes to the tray
  (`main.rs:2339`), and `useReportUnsaved` only logs. No prompt was found on the path that
  really exits. Done when quitting with unsaved edits is exercised, and either warns or
  keeps the draft for next launch.

**P1: repeated friction in everyday use**

- **F5. Undo means two things (S).** Top-bar Undo and Session tab Undo are different
  histories with one name; in the probe the Session one was enabled while the top-bar one
  was disabled. With Undo layout, that is three sets on screen. Done when the Session
  actions are renamed for what they do (restore a saved version) and the canvas pair is
  reviewed now that the top bar covers layout too.
- **F6. Browser dialogs (M).** New session uses `window.prompt` (`App.tsx:1543`), and
  Delete session, Unload plugin, Recycle recording and Remove library entry use
  `window.confirm`. They ignore the theme and offer no name check or undo. Done when all
  five use the app's dialog with focus handling, in three themes.
- **F7. One tool, four names (M).** The tool card says "Input device", the new card
  "Physical input 1", its header "PHYSICAL INPUT", and Play's error "Physical Output
  Properties". Messages also say "draft" (over 50 strings in the source), and panels say
  "capture endpoint", "virtual capture bus", "Native" and "node". Done when a written
  glossary is applied everywhere and a test blocks the retired words in visible text.
- **F8. Cards show developer detail (S).** Each card carries an always-on "1 port ·
  enabled · output: out (1ch)" and a NATIVE or PHYSICAL pill the user cannot act on, while
  "Save required" appears even though Play can preview unsaved edits. Done when the card
  shows what the user needs, with port detail on demand.
- **F9. Connection controls cover the cards (S).** At the default spacing the pause,
  remove and add strip sits over Microphone's delete button and the first letters of
  "Voice gain". Done when the strip never overlaps a card, or shows only on hover or
  selection, with no accidental delete possible.
- **F10. Toast covers the tabs at 640 px (S).** Done when no toast hides navigation at
  any supported size. *Seen on a full-page capture; confirm while scrolled.*
- **F11. Thirteen flat tabs (L).** Three rows of about 130 px mix daily work (Tools,
  Properties, Status) with setup (Apps, Virtual Cables, UI Options) and developer items
  (MCP, API, Logs), and Advanced holds 14 more unranked sections. Logs, API and MCP sit
  alone on the last row. Done when tabs are grouped by task, the daily ones stay one
  click away, and a first-time user finds a setting without hunting. Needs an owner
  decision on structure; this is the largest single gain for Navigation.
- **F12. Panels forget where you were (S).** Switching tabs resets the scroll (Tools 700
  → 0) and closes open Advanced sections. Done when both survive tab switches.
- **F13. Keyboard path (M).** Crossing Tools to reach the canvas takes 85 Tab presses,
  and there is no skip link. Ctrl+S, Ctrl+A, Ctrl+D, F2, F1, `?`, Esc to deselect and
  Ctrl+0/+/− all did nothing. Arrow keys did not move a card after a mouse click.
  Shortcuts are listed only inside Advanced. Done when the standard shortcuts work, a
  shortcuts list is one key away, and regions can be skipped.
- **F14. Errors name the fix without offering it (S).** "Select the speaker or headphone
  device in Physical Output Properties" highlights the card well, but has no button to go
  there and does not name "Headphones". Done when each actionable error names the object
  and offers the action.
- **F15. The unconnected warning never leaves (S).** It stays over the canvas with a
  long sentence, returns as state changes, and "connected routes can still play" reads
  poorly for a route with no connections. Done when ongoing state lives on the affected
  cards or a status area, and toasts report only events.
- **F16. Toggles (S).** They are 41×22 px, under the 24 px target, and only Verbose
  logging shows On or Off in words. Done when all share one 24 px or larger control with
  a visible state.
- **F17. Top-bar hierarchy (S).** Reconnect is always visible at full strength even when
  connected, with more weight than a disabled Save. Save and Play become identical amber
  together. Stop is a plain button, so stopping is less obvious than starting. Done when
  Reconnect appears only when needed, and Save, Play and Stop are clearly ranked.
- **F18. Heavy actions in Properties (S).** "Duplicate", "Remove" and privacy mute are
  full-width equal buttons, and Remove is not styled as destructive although Delete
  session is. Done when actions are ranked and destructive ones look it.

**P2: polish a discerning user will notice**

- **F19. Style sprawl (M).** Across all tabs: 17 button heights (21 to 133 px, cards
  included), 14 radius values, 267 distinct hex colors, 49 `!important`. Done when sizes
  and radii come from a small token scale and a test holds the line.
- **F20. Small text (M).** Chart labels 8.5 px; canvas text 9.9 to 10.9 px at the default
  zoom, because the 11 px floor is applied before canvas zoom. 20 sizes render in all.
  Done when charts meet the floor and the canvas opens at a zoom that keeps text legible.
- **F21. Boundaries and non-text contrast (S).** Disabled top-bar button borders measure
  1.77:1 dark and 2.86:1 light (disabled is exempt, but they read as missing). Meter and
  chart graphics are not pixel-audited. axe flagged color-contrast and target-size only
  with 20 or more cards stacked in the sweep, probably overlap; re-check on spaced cards.
- **F22. No icon system (L).** There is no icon library: tool icons and × ＋ ⏸ are
  characters drawn by whatever font Windows picks, so weight and size vary, and related
  tools reuse a glyph (⇄ for both switches, ▥ for Meter and Spectral Gate). Done when one
  drawn set covers tools, actions and status, each with a label or name.
- **F23. Canvas aids (M).** No snap-to-grid, alignment guides or minimap. Keyboard
  connecting is a form inside Advanced. Done when drag placement has guides and a large
  route stays navigable, tested on the owner's real route.
- **F24. Long panels (M).** 28 of 33 tool panels need scrolling at 1000 px (median 924 px,
  up to 1470 in a 747 px view). Done when the main controls sit in the first screen and
  rarely used ones collapse, with the choice remembered.
- **F25. Tools list (M).** 36 cards as tall as 133 px make a 4226 px list, about 4.6
  screens. There are no recents, favorites or collapsible groups. Search does work: all 18
  everyday terms tried returned results, including "volume", "headset" and "autotune".
  Disabled cards (Cable input, Cable output, Endpoint loopback) sit in the main list with
  long explanations. Done when common tools are reachable without scrolling.
- **F26. Accessibility rules (S).** axe's only failures on the 13 tabs are `aria-label`
  on the connection dots (`aria-prohibited-attr`, 16 with 3 cards). In the Gain visual,
  focusable handles sit inside an image (`nested-interactive`). Done when axe is clean and
  kept in the suite. A clean axe is necessary, not proof of usability.
- **F27. Desktop feel, *verify in exe*.** The browser context menu is not suppressed on
  the top bar, panels or tabs, and interface text can be selected. Check F5, Ctrl+P and
  Ctrl+F in WebView2. Also still open: native dropdown long names, a slow-action matrix for
  Loading, and an attended live-route check for Motion.

**P3: what would make it exceptional (proposals for the owner to accept or reject)**

- **Start from a goal, not an empty graph.** Only Discord has a guided setup. Add
  templates such as "clean up my mic", "game + voice for streaming", "record a podcast".
- **"Is it working?" in one place.** A mic test with an honest result ("no signal: the
  mic may be muted in Windows") and a plain-language route check, instead of reading meters.
- **Compare by ear.** A held A/B for a tool or the whole chain, and named presets per tool.
- **Find anything.** A command palette (Ctrl+K) across tools, settings and sessions.
- **Visible history.** A list of recent changes to jump back to, since Undo now spans
  route and layout.
- **First-minute tour.** Short, skippable, and only for what the current screen needs.

### What a 10 needs in every category

No code change can award these, which is why they are gates and not tasks:

1. A first-time-user test: someone new routes a mic to Discord, unaided and timed.
2. Narrator through the main flows, and Windows scaling at 125%, 150% and 200%.
3. An independent reviewer scoring against this rubric, per the owner gate above.
4. Attended use on the owner's real route, including device loss and sleep or resume.

### Suggested order

1. P0 F1, F2, F3, then verify F4: small work that fixes claims the app gets wrong.
2. Quick P1 wins: F5, F9, F12, F14, F16, F17, F18.
3. The glossary (F7, F8), since later copy depends on it, then F6, F13, F15.
4. Owner decision on tab structure (F11), then F24 and F25 inside the new layout.
5. Foundations: icon set (F22) and tokens (F19, F20), which touch many screens.
6. The external gates above, then re-rate. Refresh guide videos after the visible controls
   settle (still outstanding from iterations 14 to 19), and verify them in Firefox.

**Probe correction, kept so it is not re-raised:** connecting appeared to move the Voice
gain card onto Headphones. The audit grabbed a hidden dot through `.first()` and dragged
the card itself. Not a product defect.

**Incident during this round, no product impact:** at about 20:25 another thread's
worktree cleanup followed `node_modules` links and emptied `contracts/` and
`ui/node_modules` in this checkout. That thread stopped, restored `contracts/` from git
and reinstalled packages by 20:36. This review's uncommitted work and the 20:06 exe were
unaffected. The interrupted audit steps were rerun and passed.

### Iteration 20 — the small fixes from Round 20 (owner request, 2026-11-05)

The owner asked for the easy wins first, mockups of the bigger changes for approval (in a
separate thread, writing only to `target/ui-mockups/`), and an answer on fonts.

**Lesson from this round:** three findings above were written without checking earlier owner
decisions, and would have undone them. Check `docs/spec/09-interface.md` and
`docs/plans/archived/` before changing behavior a finding describes.

| Finding | Result | What changed |
| --- | --- | --- |
| F1 | Done | Card and Properties chips say "no device", "no input" or "muted" from real state (`nodeAttention.ts`). |
| F2 | Partly done, by owner decision | The owner removed "Mute mic" from the header and tray on 2026-10-14 (spec, UI-10), so there is no top-bar button. The status now reads "Audio running · Mic muted", affected cards show "muted", and the control stays in Properties, now worded as an action. |
| F3 | Partly done, by owner decision | The owner asked that the name opens the Session tab, and the spec keeps switching there. Added a "›" and a clearer tooltip, no dropdown. |
| F4 | Confirmed from code, not fixed | Closing the window keeps the draft, because it only hides when edits are unsaved. Tray "Quit and stop audio" and the window's Quit button exit without checking `UiUnsaved`, so unsaved edits are lost. Not run in the exe. Warn or keep a draft: owner decision. |
| F5 | Done | The Session tab pair called the same undo as the top bar, so it was a duplicate and not a second history. Removed. |
| F9 | Done | When two cards are under 100 canvas units apart, the connection bar hides until the line is hovered, selected or reached by keyboard. |
| F10 | Deferred | A floating message must cover something at 640 px. Goes with the layout mockups. |
| F12 | Done | Each tab keeps its scroll and open sections while the window is open (`usePanelMemory.ts`). Properties always starts at the top, because it follows the selected tool. |
| F14 | Done | The "No audio started" device messages offer "Go to {tool}", which selects that tool and opens Properties. Writing the test found the first version left the old tool selected. |
| F15 | Reworded only | The spec asks for this nonmodal bottom-right popover, so it stays. The new text drops "connected routes can still play". |
| F16 | Partly done | Toggles are now 24 px high. Visible On and Off words are not added. |
| F17 | Done | Reconnect shows only when the connection needs it, and arrives without moving the status. Play is the one filled button, Save is outlined, Stop is red with a square. |
| F18 | Done | Duplicate and Remove share a row, and Remove looks destructive. |
| F26 | Fixed, axe not rerun | Connection dots have `role="img"`, and the parametric EQ curve is a `group`, since its points are focusable. (The finding called this the Gain visual; the code is the EQ curve.) |

Also fixed: section rows in Advanced respond across the whole row. The missing-device
message no longer says "Physical Output Properties", part of F7. Hiding the connection bar
means close cards need the line hovered first; `ui-round-20-fixes.pw.ts` runs that real
mouse path. Watch this one in attended use, because the line between close cards is short.

Mistakes caught while testing this round: the "›" went into the session name's accessible
name (123 browser failures; now hidden with `content: "›" / ""`), and removing a sentence
from the Properties note undid wording from the save-language change (restored).

**Fonts:** no download is needed. The CSS names `Inter` first, which is neither bundled nor
installed here, so the app already falls back to Segoe UI, which ships with Windows.
The mockup thread compares options; bundling Inter would be optional.

**Regression tests:** `ui/e2e/ui-round-20-fixes.pw.ts`, `ui/src/nodeAttention.test.ts`, and
new cases in `NodePropertyStatus.test.tsx`, `routePlaybackLabel.test.ts`,
`SessionFlowCanvas.test.ts` and `App.lifecycle.test.tsx`.

**Results (2026-11-05, this PC, uncommitted tree):** typecheck, `format:check` and lint clean;
unit 879 passed; browser 507 passed, 14 skipped, 1 failed with "browser has been closed", and
that test passed when rerun alone. Dark, light and high-contrast top bar checked by eye from
the test screenshots. Not rerun: axe, so F26 stays unconfirmed. Guide videos are still not
refreshed, and Firefox guide playback is not rechecked.

**Build:** `src-tauri/target/release/audiorouter-shell.exe`, 23:13, SHA-256 `2ba412d3…ef60`,
built from a `ui/dist` of 23:12; the new strings were found in those assets. The packaged
smoke test passed, but no one has pressed these controls in the exe. The first build this
round kept a stale `ui/dist` because `npm run build` failed on a test-file type error that
`tsc -p .` does not report, and the smoke test still passed. Check the build's exit code and
the `ui/dist` time. The exe also contains another thread's uncommitted security changes.

Mockup thread corrections to Round 20: Advanced has 12 sections, not 14; there are 37 tool
cards, not 36; `GoalSetupGuide.tsx` offers three goals, of which only Discord builds anything.

Rollback for Iteration 20: the UI files named above; nothing is committed. F4 is still the
owner's decision.

## Round 21 — build every approved mockup (owner request, 2026-11-05)

Objective: the owner said "I want you to do ALL the recommendations and proposals" from the
13-item approval list (`target/ui-mockups/index.html`, git-ignored). Work continues on top of
the uncommitted tree on `release/0.0.27`. No commit, branch change or PR is authorized.

Requirements: UI-01, UI-05, UI-07, UI-09, UI-10, UI-11, UI-17, UI-18, UI-19, UI-21, UI-26,
AUTO-18; findings F10, F11, F13, F19, F20, F22, F23, F24, F25 and the six P3 proposals.

Decisions:

- **DEC-UI-21a, tab structure (F11 option C).** Five daily tabs and a Settings page that fills
  the workspace. Timing and Apps become views of Status; Advanced is removed and each section
  moves as listed in the mockup table. UI-18, UI-19 and UI-26 change with it.
- **DEC-UI-21b, A/B compare.** The owner's "ALL" includes A/B, which the list left to the owner.
  It is built with the visible warning that Discord hears the change while held. Releasing,
  losing focus or leaving the panel always restores the prior state. A preview path only the
  user hears is not built.
- **DEC-UI-21c, privacy mute stays in Properties.** UI-10 requires it and Iteration 20 kept it,
  so F24 keeps it in the panel. The mockups' assumption that it would leave was wrong.

Ordered tasks (each is typechecked and tested before the next starts):

1. F11 tabs and Settings page, then spec and test updates.
2. Fonts and the type and spacing scale (F19, F20).
3. Icons (F22): a copied Lucide file with four custom icons; ISC notice beside it.
4. Tools list (F25): recents, pins, compact groups.
5. Properties (F24): collapsible sections with values shown while closed; guide videos.
6. Canvas aids (F23): snap and minimap, then alignment guides.
7. Goal templates, mic test, A/B and presets, Ctrl+K, change history, tour; backend parts follow
   AUTO-18 (`API_METHODS`, contracts, MCP tool or exclusion, recipes, generated docs).

Validation: UI typecheck (`typecheck:all`), lint, format, unit and browser suites; three-theme
screenshots at 1440x1000 and narrow sizes; Rust tests, fmt, Clippy and contract drift for
backend changes; a release exe run through the changed paths when no one is using the app.
Heavy builds only when `audiorouter-shell` is not running.

Risks: a broad CSS scale can regress small panels; canvas hidden under Settings must keep its
state; A/B changes live audio. Rollback: revert the Round 21 edits; no database migration is
intended unless recorded below.

### Round 21 progress

Baseline before any edit (2026-11-05 23:15, this PC): typecheck and lint clean; 879 unit tests
passed. Other results are recorded per task below as they run.

Task 1 (F11): built; five daily tabs, gear, Settings page, Status views, Session saved versions, canvas
toolbar keyboard panels. Text in specifications, documents and the site is still to change.

Tasks 2 (F19/F20 and fonts): the type and spacing scale is mapped. Segoe UI Variable is the app font; the
11 px canvas opening zoom waits for the canvas step.

Task 3 (F22): `ui/src/iconShapes.ts` holds 85 shapes copied from lucide-static 1.69.0, with
`ui/src/icons-LICENSE.txt` (ISC) beside it, plus four AudioRouter drawings. `NODE_KIND_ICONS` is a
`Record<NodeKind, IconName>`, so a new kind fails to compile without an icon. Defect found by the browser
suite: the card's kind icon widened cards by 9 px, saved positions overlapped, and connection dots were
covered ("a removed line can be dragged back" failed). Fixed by reducing the kind label's letter spacing;
regression: `ui/e2e/card-width.pw.ts`.

Task 4 (F25): `ToolShelf.tsx` and `toolGroups.ts`. First-run height is under the 1454 px of the design;
fully open is under 2400 px against 4226 px. The design's 1454 px counted tool rows and headings only;
Recent, Needs setup reasons, plug-in buttons, search and the description slot add the rest.
Deviation: the description slot is 80 px, not 66 px, so every tool's help text fits without clipping.
A file named `toolShelf.ts` next to `ToolShelf.tsx` collided on Windows, so the logic is `toolGroups.ts`.

Task 5 (F24) in progress: `PropertySection.tsx` remembers open sections per tool type on this PC.
Decisions: privacy mute stays a visible button on every tool, not in a section (DEC-UI-21c). Compact
one-row sliders apply to all generic tool settings, not only the two designed panels; each explanation moves
to the tooltip and `aria-description`. Fun Voice Changer's Custom preset now starts with sections closed
(it was 4161 px with all nine open). Heights measured on the real-backend harness at 1440x1000, where 831 px
is the visible panel: 14 tools now fit it; Compressor 1177, Duck 1418, Meter 1331, Network Receive 1321,
Advanced EQ 1273 and Network Send 1250 still scroll. Guide videos are not yet re-recorded.

Task 5 finished: F24 sections are in place. Guide videos re-record at the end of the round because Compare and My
presets also joined Properties.

Tasks 6 and 7 (F19/F20, F23): the canvas opens at a readable zoom; a route too wide opens where its signal begins
(`openRouteView`) rather than centred, which hid its source and its output. Snap (24 px), map, guides and arrow keys:
`e2e/canvas-aids.pw.ts`. Default card spacing moved from 280x230 to 336x264 because cards at 14 px overlapped.

Tasks 8 to 13: backend in `goals.rs`, `compare.rs`, `tool_presets.rs`, `saved_versions.rs`, reachable through
REST and the default MCP tools except `nodes.compare`, excluded with its reason. UI in `GoalDialog`, `MicTest`,
`ToolCompare`, `CommandPalette`, `ChangeHistory`, `FirstMinuteTour`. Spec: UI-28 to UI-36.

Defects the tests and screenshots found in this round, each with a regression:

| Defect | Regression |
| --- | --- |
| "Without the chain" included a tool that also fed another branch, so another feed would have changed | `compare rules` in `roundTwentyOne.test.tsx` |
| "Since last save" kept listing saved changes, because the undo list outlives a save | `does not list changes that are already saved` |
| Mic test said "Good" beside "Peak −∞" | `judges the loudest moment` and the browser mic test |
| Settings › Help was an empty page, so the tour could not be replayed | `goals-and-tour.pw.ts`, `tab-structure.pw.ts` |
| A goal route opened with cards stacked on one another | overlap check in `goals-and-tour.pw.ts` |
| A wide route opened centred, with its source and output off-screen | Microphone position check in `goals-and-tour.pw.ts` |
| Goal dialog kept empty placeholder cards after a failed load | screenshot review; placeholders now end when the load fails |
| Compressor card's threshold line was near-white on the light theme (`.flow-node-content small` was a fixed colour) | computed colour probe and three-theme screenshots |
| A running comparison could survive Stop for up to 30 s and affect the next Play | `stopping_a_route_ends_its_comparison` |

`crates/control/examples/e2e_backend.rs` now allows `toolPresets.*`. `e2e/session-file.pw.ts` approves the
destination through `sessions.approveExport`: the security repair requires the Save dialog's approval, and the
stale example binary had hidden that from this test.

Limits to report: hold-to-compare has not been heard on live audio unless the handoff says otherwise. A hold on a
protected voice path is audible to listeners by design (DEC-UI-21b), with the warning shown; a listener-only preview
is not built.

Running the release exe (disposable database and `WEBVIEW2_USER_DATA_FOLDER`, virtual cables, no real microphone)
found what the harness could not:

| Defect | Regression |
| --- | --- |
| A remembered route that no longer exists (removed by a script, older database restored) stopped startup at "session not found"; Reconnect asked for the same route | `opens the active route when the remembered one no longer exists`, and the exe run |
| While that startup failed, the first listed route was written as active, replacing the backend's saved choice | same test; `keeps offering Reconnect when the remembered route still exists but will not load` keeps the other path |
| During a hold the Compressor and Duck panels reported "sends no readings… run only one window" | `hold to compare shows its warning and lets go` |
| The compare warning named Discord on routes without it | wording in UI-30, UI-31 and the tests |
| Zoom buttons stayed dark on the light theme | three-theme screenshots |

`crates/cli/tests/mcp_stdio.rs` was out of date and is corrected: 98 default tools, and the pipe test now serves the
database its client is enrolled in, as the 2026-11-05 security repair requires. `check-tool-guide-video-firefox.mjs`
takes `GUIDE_SITE_PORT`, because a server left on 4187 made every clip fail with "NotSupportedError".

The browser suite is load-sensitive on this PC: at default workers (24 logical CPUs) 9 tests timed out that pass
alone, and at 6 workers 1 did. Run it with `--workers=6`.

Incident, 2026-11-06: the first exe run and the old `fresh_install_shell` shared the owner's WebView profile. The first
run replaced the remembered route `patrick-main-siege-footstep` with `desktop-session`. With Auto-play on, the next launch
would have opened and played the wrong route. Restored through the app's own `localStorage` with a throwaway database and
confirmed in the LevelDB log; the owner's `state.sqlite` was not touched (last written 2026-11-05 20:13). Prevention:
`fresh_install_shell` now sets `WEBVIEW2_USER_DATA_FOLDER`, and the lesson is in `AGENTS.md`.

The owner approved a PR on 2026-11-06: [#118](https://github.com/MrDesjardins/audiorouter/pull/118), branch
`improve/ui-round-21`, stacked on #117. CI then found a check not run in this round: the regenerated guide pages had
stale social metadata and CSP hashes. Regenerated, and `verify.mjs --checks site` now passes.

## Round 22 — re-rating after Round 21 (owner request, 2026-11-06)

**Method.** Same rubric and method as Round 20, on the code of #118: route harness in Edge at 1440×1000 and 640×360;
all 5 daily tabs, 2 Status views and 15 Settings pages (22 screens) in three themes; axe-core 4 from a scratch folder;
control and type inventory; Properties for 33 tools; keyboard walk; 11 shortcuts on a route with an unsaved change.
Screenshots and JSON: `target/ui-quality-review/round-22/` (not committed).

**Limits.** This agent graded its own fresh work. Harness audio is simulated. No Narrator, Windows scaling, custom
themes or first-time user, so no score rests on them. The real exe was exercised in Round 21 with a silent cable and no
real microphone. SVG chart labels are not covered by the text-size figure, because computed size ignores SVG scaling.

### Scores

| # | Category | Round 20 | Now | Main reason | Still holding it back |
| --- | --- | --- | --- | --- | --- |
| 1 | Contrast | 8 | 8 | axe clean on all 22 screens in 3 themes | Selected switch text 1.24:1 (N1); graphics not pixel-audited (F21) |
| 2 | Typography/size | 7 | 8 | Nothing under 11 px, canvas included; 20 sizes down to 6 | Windows scaling; chart labels unmeasured |
| 3 | Clarity | 6 | 7 | True chips, goals, mic test | NATIVE and PHYSICAL pills, port lines, "draft" (F7, F8) |
| 4 | Consistency | 7 | 7 | One Undo, one icon set | Native dialogs (F6), one tool with several names (F7), 273 colours (F19) |
| 5 | Spacing/alignment | 7 | 7 | Connection bar no longer covers cards (F9) | Message covers tabs when narrow (F10); tight tab labels |
| 6 | Navigation | 6 | 8 | 13 tabs to 5 plus Settings; Ctrl+K; canvas in 10 Tab stops, was 85 | Narrator; no direct route switcher (F3) |
| 7 | Errors/recovery | 7 | 8 | Errors offer the fix; missing route recovers | Quit loses unsaved edits (F4); live device loss untested |
| 8 | Inputs | 7 | 7 | Compact sliders, 24 px toggles | Browser prompt for New session (F6); most toggles lack On/Off (F16) |
| 9 | Buttons | 7 | 8 | Ranked Play, Save, Stop; 17 heights down to 12 | 2 targets under 24 px |
| 10 | Loading/progress | 8 | 8 | No new defect | Slow-action matrix incomplete |
| 11 | Menus/dropdowns | 8 | 8 | Palette added | Native lists; browser menu on panels (F27) |
| 12 | Icons | 6 | 8 | One drawn set, compile-checked per tool kind | No separate check of icon meaning with users |
| 13 | Drag/drop | 7 | 8 | Snap, guides, map, arrow keys | Not used on the owner's real route (F23) |
| 14 | Motion | 8 | 8 | Unchanged | No attended check on a live route |

Total 99 to 108 of 140 (average 7.1 to 7.7). Lowest score 6 to 7; ten categories at 8. None reaches 9, and the rubric
does not let an average stand in for that. No 10 can be awarded: all four external gates in Round 20 remain unmet.

### Findings retested

- **Done (11):** F1, F5, F9, F11, F12, F14, F17, F18, F22, F25 (first screen 1060 px, all open 2305 px, was 4226 px), F26
  (no `aria-prohibited-attr` or `nested-interactive` left).
- **Partly (10):**
  - F2, F3: limited by owner decisions.
  - F7: "added to the draft" is still visible.
  - F13: Esc and arrow keys work. Ctrl+S, Ctrl+A, Ctrl+D, F2, F1, `?` and Ctrl+0/+/− still do nothing, and there is no skip link.
  - F15: reworded only.
  - F16: height only.
  - F19: heights and type sizes fell, but colours (273 distinct in `styles.css`) and `!important` (49) did not.
  - F20: chart labels unmeasured.
  - F23: real route not used.
  - F24: 9 of 33 tools fit without scrolling in an 831 px panel, against 5 of 33 in 747 px; the median is 1013 px. Round 21's
    "14 fit" no longer holds after Compare and presets joined the panel. Closed sections do keep the main controls near the top.
- **Open (6):** F4, F6 (5 native dialogs), F8, F10 (seen covering the tabs at 640×360), F21, F27 (browser menu on the top bar,
  panels and tabs in the harness; text selectable).

### New or newly explained

- **N1.** Selected Input Switch and Output Switch buttons: `small` text at 1.24:1 on the amber fill, because
  `.flow-node-content small` overrides the button's dark text. Round 20's axe data has the same count on these tools, so
  this is not new; Round 20 had put such flags down to stacked cards.
- **N2.** "Review before saving" covers the right-side tabs while its message points to Session. Seen in the exe; this
  audit's probe did not open it, so there is no new measurement.

### Next, by expected gain

1. Decide and fix F4 (unsaved edits at quit), the most serious open defect.
2. F7 and F8 (plain words on cards and messages), for Clarity and Consistency.
3. F6 (app dialogs), for Consistency and Inputs.
4. F10, N1, N2 and F16: small fixes for Spacing, Contrast and Inputs.
5. The rest of F13, then F19 and F24.
6. The four external gates, which every 9 and 10 depends on.

## Round 23 — fixing what held the score back (owner request, 2026-11-06)

The owner asked to fix the holding issues without choosing between a quit warning and a kept copy, so the recommended
option was built: nothing blocks quitting and the edits come back. This is not a re-rating; the Round 22 scores stand
until someone grades the result.

### Changes

- **F4, unsaved edits at quit.** While edits are unsaved the page writes a copy to `unsaved-route.json` beside the
  database (`src-tauri/src/ui_draft.rs`, `ui/src/draftRecovery.ts`), 400 ms after each change and at once when the
  window's Quit is pressed. Tray Quit and a crash therefore lose at most the last 400 ms. On the next launch the edits
  return with Discard edits beside the message. If the route was saved in between, nothing is applied until the user
  picks Use my edits or Keep saved route. A copy for a deleted route, or one that fails decoding, is ignored. Saving or
  discarding removes the file. The file holds one route: a copy for another route waits for that route, but new edits on
  the current route replace it, and the message says so.
- **F6, browser dialogs.** All five use `ui/src/AppDialog.tsx`: focus trap, Esc cancels, Enter submits, empty names refused
  in the dialog. Destructive questions start on Cancel.
- **F7, wording.** Glossary below, enforced by `ui/src/plainWords.test.ts`, which first found 27 more strings.
- **F8, cards.** The port line and the Native badge are gone; port detail moved to the kind label's tooltip. "Save required"
  became "Unsaved", because Play can preview unsaved edits.
- **F10, message over tabs.** In a narrow window the message moves to the top, below the top bar, while it would cover the
  tabs (`ui/src/notificationPlacement.ts`).
- **N1.** Selected switch text inherits the button's dark colour. Axe also found the unselected text at 4.31:1 in light; fixed.
- **N2.** "Review before saving" now sits over the canvas in the two-column layout.
- **Found on the way.** Dialog labels were nearly invisible in the light theme, including the Connect dialog; fixed.
- **CI.** `tools/m00-wasapi-probe/Cargo.lock` was stale from #117's new dependencies, which failed #118's Windows job. It
  would fail #117 alone too. One line added; `verify.mjs --checks probe-build` passes.

### Glossary

| Say | Not | Meaning |
| --- | --- | --- |
| route | graph | What is on the canvas |
| tool | node | A card on the canvas |
| unsaved edits | draft | Changes not saved yet |
| input device, output device | physical input, physical output, endpoint | A real device |

Scope: the daily tabs, cards and messages. `NativeDevicePanels.tsx` and `AdvancedPanels.tsx` are developer panels in
Settings and keep engine terms. Identifiers and the API are unchanged. The test reads source strings, so it can miss text
built in other ways.

### Verified

Run on the Windows PC with no shell running, at below-normal priority.

| Check | Result |
| --- | --- |
| Format, lint, typecheck, `cargo fmt --check` for both manifests | Clean |
| Full UI unit suite | 967 of 967 |
| Every changed browser file plus `e2e/quit-keeps-edits.pw.ts`, one worker, in Edge | 218 passed, 1 skipped, 1 failed (the 640×690 message defect below) |
| `e2e/quit-keeps-edits.pw.ts` three times after that repair, with its new regression test | 33 of 33 |
| `audiorouter-control` lib, `audiorouter-cli` lib, `src-tauri` tests | 359, 49 and 91 passed; 11, 0 and 4 ignored |
| Clippy, workspace and `src-tauri`, `-D warnings` | Clean |
| `check:drift`, `verify.mjs --checks site`, `verify.mjs --checks probe-build` | Passed |
| Axe on the canvas with both switch kinds, New session and Delete session, three themes | No violations |
| Release exe, new database, isolated `LOCALAPPDATA` and `WEBVIEW2_USER_DATA_FOLDER`, driven over CDP | Edit then forced kill, relaunch: edits back with their message. Edit then window Quit at once: last edit kept. Discard removes the cards and the file; next launch is clean. Backend log: 0 errors |
| `fresh_install_shell` with `--ignored` against that exe | Passed |

Defects found before handoff, each with a test: Discard right after a restore left the file behind, so the edits would
have returned; the tab row was measured through a zero-height element, so the message never moved; Undo after a restore
returned the startup placeholder; the message placement was only measured on a scroll, a resize or a change of text, so
it stayed on the tabs when they appeared after loading (now rechecked every 250 ms in narrow windows); a new install still
named its first cards "Physical input" and "Physical output", which the UI word check could not see, because the shell
seeds them (also fixed in the API's default names and an MCP recipe).

Removing the card's port rows made 33 browser tests fail. They clicked cards past the canvas edge at readable zoom and had
been passing by scrolling a clipped container, a problem already on `HEAD`. They now press the fit button first.

### Limits

- **Tray Quit itself was not clicked.** A forced kill after the 400 ms write stands in for it, and is the harsher case.
- **The last browser run predates the message recheck.** After it, only `quit-keeps-edits.pw.ts` and the unit suite ran.
- **Not run locally:** browser files this work did not change, Rust integration suites beyond those listed, and real audio
  or real devices. The disposable exe run used no microphone.
- **Guide clips are not re-recorded** for Input device, Output device, Plugin and Audio file, and the website and quickstart
  still say "Physical input" and "Physical output" in places, so F7 is not finished there. See the current plan.
- **A second card can still be clipped at 1000 px.** Geometry is the same on `HEAD`; a person pans or uses the fit button.
- **Still open from Round 22:** the rest of F13, F16 (On and Off words), F19, F21, F23, F24, F27, and the four external gates.

Rollback: revert the Round 23 commit. A leftover `unsaved-route.json` is harmless and is ignored by an older build.

Next action: the owner tests the exe; then re-record the four guide clips and finish the website wording.

## Round 25 — one-point improvement in every category (owner goal, 2026-11-06)

Objective: raise each Round 24 category by at least one point. Spacing/alignment
and Navigation must reach 8; Contrast, Typography/size, Clarity, Consistency,
Errors/recovery, Inputs, Buttons, Loading/progress, Menus/dropdowns, Icons,
Drag/drop and Motion must each reach 9. The existing rubric and external gates
remain in force. No score is raised by starting this work. No PR is authorized.

Requirements: M05, UI-02/03/04/05/07/08/09/11/12/13/15/17/19, ENG-02/04/05.
Prerequisites: Windows, installed UI dependencies and Edge are available;
disposable harnesses must isolate the owner's route, devices and database.
Narrator, Windows display scaling, first-time-user testing, independent review
and attended real-route/device-loss/resume evidence remain unverified.

Ordered work and validation:

1. N3: reproduce invisible keyboard focus, add a regression for dark, light and
   high contrast, draw a theme-aware focus ring and inspect rendered results.
2. N4/N6: simplify repeated card labels and exercise first-view geometry at
   1920, 1440, 1280 and 1000 px without reducing readable text size.
3. Contrast/Typography: audit graphics, focus, scaled chart labels and enlarged
   layouts. Clarity/Consistency: finish N5, the glossary, guides and token audit.
4. Inputs/Buttons: visible toggle states, target sizes, shared sizing and all
   disabled/busy/validation states. Navigation: skip links and remaining F13.
5. Errors/Loading/Menus: exercise slow, failed, retry, long-name, keyboard and
   focus-restoration paths through their real entry points.
6. Icons/Drag/Motion: check icon comprehension, complete keyboard equivalence,
   real-route placement and attended playback/reduced-motion observations.
7. Run relevant checks and three-theme visual verification for each slice;
   record exact evidence and limits. Re-rate all 14 categories only after the
   per-category matrix and required external evidence have been inspected.

Initial inspection: branch `improve/ui-round-23`; the review document already
has unrelated uncommitted Round 24 text, and release evidence is untracked.
Preserve both. No source changes were present. Round 24 is the authoritative
baseline (110/140). Shared CSS risks cross-theme regressions; rollback each
bounded slice by reverting only its source/test hunks, preserving prior work.

First slice evidence (Windows Edge, 2026-11-06):

- `npx playwright test --config=playwright.config.ts e2e/canvas-focus.pw.ts
  --workers=1` reproduced N3 in all three themes: expected solid outline,
  received none. The focused wrapper already matched `:focus-visible`.
- `ui/src/styles.css` now draws a 3 px theme-token outline, offset 3 px,
  on keyboard-focused card wrappers. It changes no layout or audio state.
- After the fix, the focus and canvas-delete-alignment suites passed 6/6
  with one Edge worker. Focus tests cover visible outline, unchanged card
  bounds and removal on Tab away. `npm run typecheck:all` passed; scoped
  Prettier reported unchanged files; `git diff --check` passed.
- Inspected `target/ui-quality-review/round-25/card-focus-{dark,light,
  high-contrast}.png`: amber, teal and white outlines visibly distinguish
  the focused Voice gain card. No packaged shell or live audio was exercised.
- Browser output retains the previously recorded NO_COLOR/FORCE_COLOR
  environment notice and React Flow attribution notice; no new warning
  was introduced by this CSS slice. Full `npm run lint` passed with zero
  warnings.

Second slice evidence (Windows Edge, 2026-11-06):

- The new `canvas-first-view.pw.ts` reproduced the clipped Headphones card at
  1440x1000 in dark, light and high contrast before the repair (3 failures).
- Removed family badges from canvas cards: the existing kind label already
  says Input Device, Output Device or the exact VST format. Unbound device
  cards retain the honest "no device" status, without repeating "No device
  chosen" in their reserved app-usage footer. Bound-device app labels and
  warnings are unchanged. Footer space remains reserved for UI-17.
- First-view, focus, card-width and canvas-aids checks passed 16/16 with one
  Edge worker. Device-used-by and quit-keeps-edits checks passed 12/12,
  including app-label geometry, three-theme dialogs and narrow notifications.
- Reviewed first-view screenshots in all three themes under
  `target/ui-quality-review/round-25/first-view-{theme}.png`: three cards fully
  visible, kind labels legible, no repeated family or missing-device line.
  Regression requires initial zoom >=1; the fix does not shrink text.
- `npm run typecheck:all`, full lint and format:check passed. Initial full
  unit run: 966/967, with the sole failure requiring the intentionally removed
  Input family badge. Updated that assertion to require each kind once,
  preserving kind, VST3 format and missing-device status checks. The focused
  SessionFlowCanvas suite then passed 31/31. Final full rerun with
  `npm test -- --maxWorkers=2`: 967/967 tests, 105/105 files passed.
  Documentation acceptance passed (245 Markdown files, 1421 local links);
  final diff whitespace check passed.

Scope limit: the new first-view assertion covers 1440x1000; narrower windows
and existing saved positions may still require panning or Fit. No new shell,
audio, Narrator, Windows scaling or attended route evidence is claimed. No
Properties controls changed, so this slice does not alter guide controls.

Third slice plan (UI-02/11/17, ENG-02/04): add keyboard skip links to the
canvas and workbench, and Ctrl+S through the same guarded save action as the
header. Respect text editing, open modal dialogs, key repeat, backend readiness
and warning acknowledgement. Add failing browser regressions first; verify
focus, geometry, saved state and absence of accidental saves. Exercise three
themes, then typecheck/lint/format and focused/full UI checks. Rollback only
the shortcut and skip-link hunks. No API or audio behavior change is intended.

Fourth slice plan (UI-02/11/17, ENG-02/04): canvas-scoped Ctrl+A selects all
tools and groups; Ctrl+D duplicates exactly one selected tool via the existing
action, and Ctrl+0/+/minus operate the existing viewport. Ignore text controls,
handled events and repeated keys. Add browser regressions before source edits,
then typecheck, focused browser/unit checks, lint/format and theme screenshots.
Zoom must preserve tool positions and reduced-motion behavior. Rollback these
bounded key-handler and callback hunks; keep the prior keyboard work.

Fifth slice plan (UI-02/11/17, ENG-02/04): F2 renames one selected tool
using the themed text dialog and backend-owned draft validation. F1 or ?
opens a readable shortcut reference with current configurable bindings,
trapped focus, Escape and focus restoration. Ignore text fields and existing
modal dialogs; retain cancellation and empty-name validation. Reproduce with
browser tests, then verify three themes, narrow layouts, keyboard focus,
typecheck/lint/format and full unit tests. Rollback only this slice's handlers,
help component and tests. No API/audio/tool parameter change is planned.

Sixth slice plan (UI-04/11/17, ENG-02/04): finish N5 on ordinary Settings
pages. Test rendered Device access, Plug-ins, startup, keyboard and support
copy rather than trusting source-string extraction. Keep raw diagnostic
records, API fields and developer panels exact. Replace user-facing engine
terms with route, tool and device; preserve privacy/authorization facts.
Check five-page navigation and three-theme captures, focused/full unit tests,
typecheck/lint/format and documentation. Rollback only the wording/test hunks.

Seventh slice plan (UI-08/11/17, ENG-02/04): begin F16 with startup toggles.
Show fixed-size confirmed On/Off, Loading or Unavailable text without changing
accessible labels. Preserve confirmed values during saves/failures; use the
shell's returned value for success text. Add regressions for unavailable,
pending, rejected and differing acknowledged values, then exercise the real
Settings page in three themes with unchanged geometry and keyboard controls.
Typecheck/lint/format and focused/full unit tests remain required. Rollback
only this slice's component, styling and tests. Remaining tool toggles and
their guide refresh requirements remain part of F16, not silently waived.

Eighth slice plan (UI-08/11/17/18, ENG-02/04): show update-check On/Off
explicitly, retain action/message space when a release arrives, and avoid
claiming a successful version check when no result exists. Exercise real
Settings with injected release data and no network. Repair the touching
setup/scan buttons observed in slice six using the shared action row, with
geometry regressions and three-theme/narrow screenshots. Preserve privacy
copy, permission identifiers and existing API methods. Validate typecheck,
lint/format and focused/full tests. Rollback only the scoped markup/style/copy
and tests. Tool toggles and their guide refreshes remain pending separately.

Ninth slice plan (2026-11-07; UI-08/11/17, AUTO-18, ENG-02/04): shared
Enabled/Bypass flags in Properties get visible On/Off text and accessible
descriptions without changing their labels or backend acknowledgement path.
Exercise stopped/live/rejected/pending flags and geometry in three themes.
Record a silent portrait Properties-only configuration clip of the shared
controls on Gain, explicitly identified as the example, and reference it from
each applicable tool guide. Add explicit shared-control coverage to the
manifest instead of misclassifying flags as processor parameters. Regenerate
guide pages and verify Firefox playback before documentation is called ready.
Preserve existing audible effect clips. Rollback the bounded label/test/guide
hunks; no new API method or parameter is introduced.

Tenth slice plan (2026-11-07; UI-08/11/17, SEC-03/05, ENG-02/04):
qualify Device access loading, failed reads, pending saves and uncertain save
outcomes. Keep controls and message space reserved. Show actionable Check
again after a failed read or write; never claim permission was unchanged when
its response may have been lost. Preserve the last confirmed value, disable
changes until a fresh read, reject duplicate writes and ignore stale results
after a backend change. No new permission or API method is authorized.
Add regressions before the fix, then exercise the real Settings entry point
with controlled faults, geometry and three-theme captures. Rollback only
this component's error/loading handling, styles and tests.

Eleventh slice plan (2026-11-07; UI-04/11/17, ENG-02/04): measure SVG
label sizes after their actual screen transform, rather than computed font
size alone. Walk the addable tool inventory through the Route list, avoiding
off-canvas card clicks. Retain a per-tool inventory, check the 11 CSS-pixel
floor and clipping at normal/narrow layouts, then fix only reproduced issues.
Keep hierarchy, curve interactions and stable layouts. Repeat three-theme
visuals and relevant browser/unit checks. Chart changes require matching
guide follow-up before documentation is declared ready. Windows display
scaling and Narrator remain separate attended gates. Rollback the bounded
typography/layout changes, never lower the floor to make the test pass.

Eleventh slice guide follow-up plan (2026-11-07; UI-11, ENG-04): refresh
the existing silent Properties overviews for all 13 affected chart kinds,
using the real panel in the established portrait recorder. Inspect representative
frames, retain capture metadata, publish only the reviewed overview assets,
then run the coverage/video checks and installed Firefox playback. Preserve
audible demonstrations and their production-DSP evidence until separately
refreshed; a silent overview cannot substitute for them. Rollback restores
only the replaced overview assets. No live owner session is involved.

Eighteenth slice plan (2026-11-07; UI-04/11/17/22, ENG-04): distinguish
known 128-frame/other cable periods as On/Off and missing periods as
Unavailable. Preserve disabled and management guards, backend-owned values,
helper operation names and the cable-restart warning. Add missing-period and
known-period regressions before the UI change. Use opt-in route fixtures for
three-theme keyboard and geometry checks; no real driver or device changes.
Rollback only the state presentation and fixtures, never the driver settings.

Seventeenth slice plan (2026-11-07; UI-04/11/17/22, ENG-04): verify and
clarify persistent picker visibility and plugin pass-through states using
BooleanField. Preserve device-specific accessible names and local hiding,
plugin enable/remove dispatch, disabled behavior and privacy explanations.
Reproduce missing descriptions first, then check keyboard/caption actions,
fixed geometry and three themes. No owner devices, plugins or permissions
are changed: use isolated fixtures. Add Plugin to pending guide refreshes;
readiness requires its updated portrait capture and Firefox playback.
Rollback only field substitutions, keeping saved preferences and grants intact.

Seventeenth slice results (Windows Edge route harness, 2026-11-07):

- Picker visibility and plugin pass-through regressions failed before the
  substitutions because neither switch exposed an On/Off description.
  Both now use BooleanField with their original accessible names, disabled
  conditions and handlers. Device hiding still changes only local picker
  preferences. Turning plugin pass-through off still removes its stored key
  when removal is supported, preserving older-build route compatibility.
- Hidden device identity/format text was faded to 60% opacity. With an
  explicit Off state now available, that fading was unnecessary for status
  and reduced readability. A browser assertion reproduced the old opacity;
  the rule was removed, keeping name/format at full opacity while preserving
  the preference class and picker filtering. This is not a new numerical
  claim of full color-contrast coverage.
- Both focused suites pass 18/18. All 12 browser checks pass, covering narrow
  device rows, keyboard/caption activation, unchanged geometry, and the
  existing plugin licence/unchanged-audio/mute/retry paths in all themes.
  Six final `device-switch-{theme}.png` / `plugin-switch-{theme}.png`
  captures were visually inspected. No owner device or plugin was touched.
- The full UI suite passes 982/982 in 108 files; typecheck, zero-warning lint,
  formatting and diff whitespace checks pass. The later fading-only CSS
  change was verified by the final three-theme browser run.
- Plugin's silent portrait overview was refreshed. On/Off and return-to-top
  frames were visually inspected; capture completed with 120 frames and zero
  page errors. Final encoded media passed installed Firefox playback. Exact
  [hash, metadata and playback evidence](evidence/2026-11-07-plugin-switch-guide.json)
  identify it. Guide generation, coverage, media budgets and 13 site tests
  pass. Owned test servers were stopped; cached HTTP DEP0066 remains noted.
- Next: VirtualCablePanel latency must distinguish unavailable from Off;
  then verify SignalFlowPanel's advanced state controls. The pending seven
  guide-kind set is now six (Mute, Duck, Fun Voice Changer, Audio File,
  Recorder and Group); renamed-tool wording remains open separately.
  No score or attended evidence gate is closed by this slice.

Sixteenth slice plan (2026-11-07; UI-04/11/17, ENG-04): verify the custom
group and recording switches. Reuse BooleanField for single-group lock/carry
and recording dither, preserving accessible names and existing format rules.
Retain the multi-group Mixed/On for all/Off for all presentation. Add state
regressions before changes and exercise keyboard/caption activation plus
geometry in three themes. Record affected Group-guide refresh work alongside
the preceding five tool-guide kinds; recordings are not ready until refreshed.
Rollback these substitutions independently of the existing saved group data.

Sixteenth slice results (Windows Edge route harness, 2026-11-07):

- Group lock and recording-dither state-description regressions both failed
  before the substitutions. Single-group lock/carry and manual-recorder
  dither now use BooleanField. Optional accessible-name override preserves
  "Lock group position and size" while its visible caption stays shorter.
  Saved group data, lock behavior and recorder requests remain unchanged.
- Multi-group options already show Mixed/On for all/Off for all; their
  checked/indeterminate behavior remains covered by CanvasLayoutInspector
  tests and was not replaced with a two-state control. Float32 WAV/MP3 still
  disable dither and force it Off; returning to PCM re-enables it without
  silently turning it On or starting recording.
- Visual review found two integration defects after the first substitution:
  unequal help lengths put group switches at different heights, and an old
  recorder checkbox rule placed the caption beside the field. New browser
  assertions reproduced both. Shared control width, the group's row allocation
  and removing the old inline recorder rule fix them without per-panel field
  appearance overrides. Final six theme captures were visually inspected:
  `group-states-{theme}.png` and `dither-states-{theme}.png` under Round 25.
- All 16 browser cases pass (new group/dither, existing locked-group behavior
  and the prior BooleanField consumers). Focused group/layout/component tests
  pass 9/9, focused recorder tests 2/2, and the full UI suite 982/982 in 108
  files. UI typecheck, zero-warning lint, formatting and diff checks pass.
- Remaining switch-state candidates from source inspection: ThisPcDevices
  picker visibility, PluginPanels pass-through, VirtualCablePanel latency and
  SignalFlowPanel's advanced enabled/bypass controls. LogFilesPanel already
  has explicit verbose status; startup/update/shared flags were covered earlier.
  OBS fix choices, Discord setup inclusion and save-warning acknowledgments
  are selections/consent, not persistent On/Off settings. Preserve their
  checked semantics and descriptive review text in the remaining audit.
- Add Group's guide to the pending Mute/Duck/Fun Voice Changer/Audio File/
  Recorder refresh set. No guide readiness, category score or attended gate
  is claimed. Next: verify those remaining persistent settings, then perform
  the consolidated guide refresh after control layouts stop changing.

Fifteenth slice plan (2026-11-07; UI-04/05/11/17, ENG-04): extend explicit
switch state beyond shared Enabled/Bypass. Introduce one shared BooleanField
for generic boolean properties, Duck's phase switches and Audio File Loop,
with a stable accessible name, separate On/Off description and fixed state
width. Reproduce missing descriptions before the fix; exercise caption and
keyboard activation, disabled behavior and geometry in three themes. Refresh
affected guide media after the controls stabilize. Keep the remaining custom
checkbox inventory open rather than claiming every toggle is covered.
Rollback only these field substitutions and their shared layout component.

Fifteenth slice results (Windows Edge, 640 by 720, 2026-11-07):

- The generic boolean regression failed before implementation: no Off
  description existed. `BooleanField` now supplies the setting caption,
  existing help, themed switch and a reserved 3ch On/Off state. State is a
  description, so it does not rename the checkbox. Generic bool properties,
  Duck's four game phases and Audio File Loop use it; parameter semantics
  and backend requests are unchanged.
- Two component regressions cover caption activation and disabled behavior;
  generic-editor rerendering covers Off/On descriptions. Four focused suites
  passed 14 tests. All 982 unit tests in 108 files pass, as do UI typecheck,
  zero-warning lint and formatting.
- Nine browser cases exercise Mute, Audio File Loop and Duck phase switches
  in three themes, using Space and caption clicks, stable names/descriptions,
  and exact field/header/inspector geometry. One initially failed: editing
  Mute in high contrast made Save's border grow from 1 to 2 px, increasing
  the header by 2 px. The owning pending-Save rule now adds the heavier stroke
  inside its existing border. The geometry regression then passed.
- All 12 final boolean/target browser cases pass. Nine
  `boolean-{kind}-{theme}.png` field captures were visually inspected under
  `target/ui-quality-review/round-25/`. The earlier unit test's unavailable
  jest-dom matcher and its invalid RTL exact option were corrected before
  accepting the reproduced missing-description failure.
- Documentation is not ready yet: refresh affected Mute, Duck, Fun Voice
  Changer, Audio File and Recorder guides after the rest of the custom-switch
  audit stabilizes. Their current published clip counts are 2/8/37/2/1.
  The Mute parameter help still says "node"; include that in the pending
  wording review. Other custom checkboxes (plugin consent, groups, recording
  format and app/device settings) remain outside this slice's coverage.
  No category score or attended evidence gate is closed by this work.
- Next: finish that custom-switch inventory, then refresh the affected
  portrait guides and verify Firefox before claiming this control change ready.

Fourteenth slice plan (2026-11-07; UI-11/17, ENG-04): pixel-audit visible
canvas connection handles in idle/output and connecting/input states across
all themes. Compare actual composited control pixels with the same scene
with handles hidden; do not infer contrast from CSS colors alone. Retain
measurements and screenshots, identify the sampled scope, and repair any
reproduced failure against a 3:1 graphical threshold. Verify no geometry
shift and preserve hidden/inactive receiving handles and connection flow.
Rollback only evidenced handle styling and its bounded regression.

Fourteenth slice results (Windows Edge, 1440 by 1000 CSS pixels, 2026-11-07):

- Added `handle-contrast.pw.ts`, using the PNG decoder bundled with the
  pinned Playwright runtime (no new dependency). It records actual rendered
  fill/border pixels, compares with a no-handles scene, and samples each
  cardinal boundary's rasterized border/adjacent neighborhood. The reported
  value is the minimum of those four boundary maxima, not text contrast or
  a claim that every antialiased edge pixel has 3:1 contrast.
- The initial center-only probe flagged handles over colored card outlines;
  it omitted contrast supplied by the dark handle border. That measurement
  was rejected rather than changing styling on a false-positive inference.
  The refined probe also requires a visible pixel change versus the hidden
  scene, unchanged geometry, and zero opacity on the inactive direction.
- All 48 handle/state samples pass this bounded 3:1 boundary probe. Minimum
  observed ratios: dark 4.03, light 3.76, high contrast 3.93. Six state/theme
  screenshots were visually inspected, including original-resolution review
  of the high-contrast connecting state. No style change was warranted.
  [Measurements and screenshot paths](evidence/2026-11-07-handle-contrast.json)
  preserve the exact sampled scope: eight sending handles at rest and eight
  receiving handles during a real output drag in the disposable demo route.
- Five browser tests pass, including sender/receiver affordances and planning
  and committing a connected multi-input route. Typecheck, zero-warning lint,
  formatting and diff whitespace checks pass. No production source changed
  in this slice, so the prior 979-test unit result was not needlessly repeated.
- F21 remains open for other graphics, live-state curves/meters, custom
  colors and Windows scaling. This evidence narrows that audit; it does not
  establish all graphical contrast or raise a category score. Next: audit
  live chart curves and remaining custom-toggle states before guide refreshes.

Thirteenth slice plan (2026-11-07; UI-11/17, ENG-04): reproduce F27's
unhandled workspace context menu. Suppress the browser menu on application
chrome while retaining editing, link actions and copying selected text.
Preserve existing canvas menus and their keyboard path. Add unit policy
regressions and three-theme real-handler browser checks before implementation;
verify text selection/copy eligibility rather than disabling selection
globally. Native WebView2 F5/Ctrl+P/Ctrl+F remain separate unverified gates.
Rollback the bounded context-menu policy and root handler only.

Thirteenth slice results (Windows Edge route harness, 2026-11-07):

- Three browser regressions first failed because top-bar context events were
  not canceled. A bounded root handler now suppresses browser commands on
  workspace chrome. The shared policy preserves text inputs, textareas,
  editable content, links, diagnostic pre/code and intersecting selected text.
  It does not disable text selection or override the canvas's own menus.
- Three policy unit tests cover chrome/non-text controls, editing/link/code
  actions and selected text versus unrelated chrome. Range intersection is
  used for selected contents; the initial containsNode check did not cover
  the selected paragraph in jsdom. No exception was mocked away.
- All three expanded browser cases pass, including an actual top-bar right
  click with cancellation observed after React bubbling, existing card-menu
  right click/Escape, an editable Session name field, a guide link, selected
  support prose and cleared-selection panel chrome. The added field checks
  initially targeted a numeric field absent from the route fixture, then an
  intentionally read-only shortcut field; the final case uses the existing
  editable session field. These were test-setup failures, not waived checks.
- Visually inspected `workspace-menu-{theme}.png` in
  `target/ui-quality-review/round-25/` for dark/light/high contrast. UI
  typecheck, zero-warning lint, formatting and all 979 unit tests in 107
  files pass. Documentation acceptance and diff whitespace checks also pass.
- F27 remains open for native WebView2 F5/Ctrl+P/Ctrl+F, native dropdown
  long-name behavior, slow-action and attended-motion evidence. This slice
  closes only the browser-harness workspace-menu behavior. No score is raised.
  Next: contrast graphics and remaining custom-toggle/guide wording audits.

Twelfth slice plan (2026-11-07; UI-11/17, ENG-04): reproduce the
Round 24 session-name and map-close targets below 24 CSS pixels. Add a real
browser regression before changing shared CSS; reserve at least 24 px in
both dimensions, preserve header/canvas geometry during state changes,
and exercise pointer edges plus keyboard activation in all three themes.
Inspect normal and narrow layouts. Rollback only the two target rules and
their regression; do not shrink labels or waive the size floor.

Twelfth slice results (Windows Edge route harness, 2026-11-07):

- `control-targets.pw.ts` failed before the fix in all three themes on the
  session button's measured 21 px height; the map-close rule was 20 by 20 px.
  Both controls now reserve at least 24 CSS pixels in each dimension. The
  session heading lets its child's focus outline remain visible while the
  button itself still truncates long text.
- Extending the regression found that keyboard map closure left focus on
  neither the removed close control nor its surviving toggle. It failed
  before the fix. Closure now returns focus to Map through a button ref.
- All 12 target/canvas-aid browser checks passed. The target tests exercise
  1280 and 640 px in three themes, Enter activation, pointer clicks two pixels
  inside the lower-right edges, Session-tab navigation, restored Map focus,
  and unchanged header/canvas bounding boxes when the map closes. All 12
  `targets-{header,map}-{width}-{theme}.png` captures under
  `target/ui-quality-review/round-25/` were visually inspected.
- UI typecheck, zero-warning lint, format check and all 976 unit tests in
  106 files passed. This is browser-harness evidence; desktop/Narrator/Windows
  scaling and attended-route gates remain open. No score has been raised.
- Both separate Duck configuration tours (level and game) were also refreshed
  in portrait with zero page errors. Four middle/end frames were visually
  inspected, final encoded clips passed Firefox playback, and hashes/capture
  metadata are in the [configuration evidence](evidence/2026-11-07-duck-configuration-guides.json).
  They remain explicitly silent, with no live Stats.cc or device claim. Site
  generation/13 tests, property coverage and all video-budget checks pass
  (202 references, 53,986,439 bytes). Owned test servers were stopped.
- Next: inspect remaining renamed-tool guide wording and continue F21/F27
  and custom-toggle state audits before another full category assessment.

Eleventh slice audible-guide plan (2026-11-07; UI-04/11, ENG-04):
recapture the 13 representative audible chart demos with the current real
Properties UI and their existing production-DSP output/telemetry fixtures.
Validate capture provenance and audio samples, encode to the unchanged site
budget, inspect changed-control frames, and verify final Firefox playback.
Keep the per-property refresh explicitly open until its own clips are updated.
Rollback only the representative demo media; preserve all fixture audio.
The same capture/verification sequence also covers the 51 existing per-property
render entries for these kinds. Preserve their property IDs, explicit silent
exceptions and fixture hashes; do not substitute a representative demo for
the per-property coverage requirement.

Eleventh slice audible-guide progress (2026-11-07):

- All 13 representative demos were recaptured in the real Properties UI,
  preserving production output PCM and telemetry. Each has 120 frames,
  zero page errors, a matching telemetry SHA-256 and a parameter action
  within 0.1 s of the fixture's 3.2 s change. Changed-control and return-to-top
  frames were inspected across all 13 kinds.
- All source PCM files contain eight seconds of finite stereo samples and
  non-zero audio. Final encoded demos pass the unchanged format/size check;
  all 13 pass pointer-click playback and non-zero audio-output measurement
  in installed Firefox. This is synthetic production-DSP guide evidence,
  not attended microphone/device or physical audio evidence.
- [Exact demo hashes, capture provenance and Firefox results](evidence/2026-11-07-chart-guide-demos.json)
  identify the refreshed media. All 51 per-property captures also completed;
  their source fixtures pass finite-sample/duration checks and are non-silent
  where marked audible. All 51 final encoded property clips passed Firefox
  playback, including non-zero output for audible clips. Shared-flags and
  existing Duck configuration clips also passed that sweep, but those are
  not counted as newly refreshed per-property captures.
- Visual inspection rejected Gate's initial Hold capture: an unanchored
  case-insensitive caption regex matched the end of "Threshold". The audio
  fixture changed Hold while the UI incorrectly showed Threshold = 600.
  `guideControlName` now matches the whole caption with only the standard
  "precise value" suffix allowed. Its regression failed on the old matcher
  and passes after the fix; it also protects Gain/Makeup gain and Bass/Bass
  frequency. The nine guard tests are now part of normal site verification.
  Gate Hold was recaptured and visually verified at Hold = 600 ms with
  Threshold unchanged at -35 dB, replacing the rejected local artifact.
- [Final property hashes, capture provenance and Firefox results](evidence/2026-11-07-chart-guide-properties.json)
  retain the exact 51 artifacts. Representative changed-field frames were
  inspected for Bass frequency, EQ filter type, Compressor ratio, Gate hold,
  Spectral Gate learning and Duck release, in addition to all 13 representative
  demos. Final media check: 202 references, 53,964,819 bytes; coverage: 38 kinds,
  252 properties. Regenerated guides/social/CSP, site checks (13 tests), docs
  acceptance and diff whitespace checks pass. Owned recording and HTTP
  servers were stopped; cached HTTP server DEP0066 remains recorded above.
- Next guide work: the two separate Duck configuration captures and the
  remaining renamed-tool wording review. Next UI finding: remeasure the
  session-name and map-close targets (Round 24 measured 21 and 20 px).
  No score or external gate is closed by this synthetic guide work.

Eleventh slice guide results (Windows Edge and installed Firefox, 2026-11-07):

- All 13 affected kinds have refreshed eight-second, silent, Properties-only
  portrait overviews. Each capture completed with 120 frames and zero page
  errors. Representative start/middle/end frames and final encoded chart
  frames were inspected. Existing audible demonstrations were preserved.
- Direct recorder exports initially failed the 1 MB media budget on five
  files. Applying the existing final encoding settings (H.264, CRF 26,
  700 kbit/s maximum) fixed the failure without relaxing the budget.
  Final checker: 38 tools, 202 video references, 53,909,736 bytes; all meet
  format, eight-second duration and size checks. All 13 final encoded
  overviews passed actual pointer-click playback in installed Firefox.
- [Capture metadata, final media hashes and Firefox results](evidence/2026-11-07-chart-guide-overviews.json)
  identify the exact artifacts. Manifest, guide pages, social metadata and
  CSP were regenerated; the site quality check and property/catalog coverage
  passed. The first site check caught missing social regeneration; the full
  pipeline passed after that generation. The cached HTTP test server emitted
  DEP0066; no dependency was added, and both owned test servers were stopped.
- Next: refresh affected audible demo/per-property clips from their production
  DSP fixtures, including the remaining renamed-tool clips. The silent tours
  alone do not close documentation readiness or any attended evidence gate.

Eleventh slice progress (Windows Edge route harness, 2026-11-07):

- The baseline rendered-size regression failed: nominal 11 px SVG labels
  rendered at about 9.84 px in 13 tool kinds. A shared Properties observer
  now compensates for each chart's actual viewBox scale on resize, without
  changing chart geometry on telemetry ticks or maintaining a kind allowlist.
- The added clipping assertion then found seven labels outside their SVG
  bounds in Duck, Compressor, Gate and Limiter. Axis-label insets and the
  transfer curve's left margin now reserve room; the compressor zone label
  anchors inside the plot's right edge. Controls and audio behavior are unchanged.
- The inventory covers 216 visible labels at each of 1280 and 640 px, in
  dark, light and high contrast. The three browser cases passed the 11 px
  transformed-size floor and clipping assertions. Evidence is under
  `target/ui-quality-review/round-25/svg-type-inventory-{theme}.json` and
  `svg-{kind}-{width}-{theme}.png`; representative chart-only captures are
  added for closer visual inspection.
- `npm run typecheck:all`, zero-warning UI lint, `npm run format:check`,
  all 976 UI unit tests, 14 focused Dynamics/Duck tests and documentation
  acceptance passed. After the final margin adjustment, all three browser
  cases and 14 focused tests passed again; 18 chart-only captures (three
  representative tools, two widths, three themes) were visually inspected.
  Axis titles, ticks and curves remain legible inside their chart bounds.
  Browser output also reports conflicting NO_COLOR /
  FORCE_COLOR environment flags and React Flow's hidden-attribution notice;
  neither is a test failure or evidence of licensing entitlement. Do not
  represent the branch as ready while these notices remain unexplained.
- Next: inspect the final chart captures and refresh affected portrait guide
  recordings with Firefox playback evidence. Existing four renamed-tool guide
  refreshes, Windows scaling, Narrator and attended audio gates remain open.
  No category score has been raised from the Round 24 baseline.

Tenth slice results (Windows Edge, 2026-11-07):

- Four regressions failed before the fix, including an actual unhandled
  rejection from a lost permission-change response. Device access no longer
  disappears after a failed read or silently drops a write failure. Controls,
  state, description and status message retain reserved geometry.
- Check again retries the existing read. A write failure labels the previous
  value "Last known", disables further changes, and asks for a fresh read
  instead of claiming permission was unchanged. The test adapter simulates
  an applied withdrawal whose response is lost; a fresh read correctly
  recovers Ask first before Allow can be pressed again.
- Five unit regressions cover read retry, uncertain writes, duplicate pending
  writes, a late read and a late write acknowledgement after backend changes.
  Generation guards discard stale results; synchronous errors also enter the
  handled read/write paths. No permission scope or API method changed.
- Final Device-access/Settings-wording browser suites passed 6/6. Tests use
  the actual Settings entry point and controlled fixture faults, including a
  keyboard-confirmed successful change after recovery. Group heights remain
  equal throughout failed reads, recovery, lost responses and confirmation at
  640 px in all three themes. Reviewed all error/recovery captures under
  `target/ui-quality-review/round-25/access-{error,recovered}-{theme}.png`;
  final header alignment was rechecked with refreshed recovery captures.
- Full UI suite passed 976/976 in 106 files. Typecheck, zero-warning lint,
  formatting, docs acceptance (245 files/1421 links) and diff checks passed.
  After the final header-only alignment, formatting and the six browser
  checks passed again. No unhandled rejection remains in these scenarios.

Evidence scope: permission calls were mocked; the owner's grants, Windows
devices, microphone and running audio were untouched. This qualifies these
UI recovery paths, not native permission revocation or device-loss behavior.
Next: continue the slow/error matrix and tool-specific toggle audit, then
measure contrast/typography gaps and re-evaluate all categories against the
still-open native/attended gates. No new score is asserted yet.

Ninth slice results (Windows Edge, 2026-11-07):

- The Properties-state regression failed before the change in all three
  themes (Enabled had no accessible state description). Enabled and Bypass
  now have separate On/Off slots, keeping their checkbox names and handlers.
  Reviewed three `flags-{theme}.png` crops: clear state, focus and unchanged
  flag-row height. No API method, authorization or audio path changed.
- Final flag/live-control browser suites passed 7/7: stopped keyboard flags,
  all three themes, live source/processor/output flags, rejection/restart
  messages and unsaved topology retention. The state words describe the
  displayed settings, not signal or native playback proof. A dedicated held
  acknowledgement/state-description probe remains in the slow-action matrix.
- Recorded the actual Gain Properties panel at 480x854, encoded to 720x1280,
  eight seconds, no audio stream. Enabled and Bypass change, then the view
  returns to the top. Frame review found "node" in the shared Off/Bypass
  explanation; changed it to "tool" and re-recorded before publication.
  Added the text assertion to the flag regression. Capture evidence:
  `target/tool-guide-configs/gain--shared-flags/`; published clip and poster:
  `site/assets/tool-guides/shared-flags.{mp4,jpg}`. MP4 SHA256:
  `0ace7cf97f787eebe8588fd379560f0610df202be254998928781c3eaaa9d032`.
- `shared-controls.json` labels the silent Gain example; `guide-recordings.mjs`
  appends it to every audio tool without replacing any existing effect clip.
  A domain-inventory regression requires coverage for every NodeKind and
  preserves original clips. Group stays excluded because it has no audio
  flags. The manifest now tracks the two shared flags separately from its
  252 parameter rows; the coverage checker validates media and all 37 kinds.
- Regenerated guides, social metadata and CSP. Catalog, property coverage,
  video format/size checks and `verify.mjs --checks site` pass. The format
  checker counts 202 tool/clip references, including shared references to the
  same new asset; this is not 202 unique video files. Site's normal check
  inventory now runs the new shared-recording regression. Guard/family/runner
  tests passed 25/25, and docs/diff checks pass.
- Firefox plays the new shared-flags clip and the existing Gain property
  clip. The first attempt used an unrelated existing server on 4187 and got
  media errors/404; preserved it, served this checkout on 4189, verified the
  new MP4's HTTP response and reran successfully with GUIDE_SITE_PORT=4189.
  Python's Store alias was unavailable, so the temporary server used the
  already-installed Node http-server. That tool reported DEP0066; no project
  dependency was added. Both owned recording/test servers were stopped.
- Full UI suite passed 971/971 before the final two-word-message refinement;
  final owning accessibility/glossary suites passed 104/104. Typecheck,
  zero-warning lint and UI formatting passed. No packaged exe, Narrator,
  actual audio or the owner's settings/devices were exercised.

Shared main flag labels and their guide coverage are addressed. The earlier
four renamed-tool guide refreshes remain open; this shared clip does not
claim to replace those demonstrations. Next: audit remaining tool-specific
boolean controls and the slow/error matrix, then re-evaluate the 14-category
matrix and its outstanding native/attended gates. Scores remain unraised.

Eighth slice results (Windows Edge, 2026-11-06):

- `settings-controls.pw.ts` reproduced all six failures before the change:
  missing update-check state and touching action targets in each theme.
  New versions now shares the fixed-width On/Off slot and accessible
  description. Its summary does not claim a successful check without evidence.
- Summary and release-action space remain reserved. The browser test toggles
  the actual setting with Space, injects v99.0.0 without network access,
  waits for the release action, and proves equal panel heights before/after
  the asynchronous result and again when switching off at 640 px.
- Setup and plugin-scan buttons now use the existing wrapping `.actions`
  layout. Geometry checks require at least 7.5 px separation, exercising
  all three actions in both pages at 640 px. No field styling was introduced.
- Final browser checks passed 6/6, full UI suite 971/971 (105 files),
  typecheck, zero-warning lint, formatting, docs and diff checks passed.
  Removed the now-unused direct-child update-button CSS rule afterward;
  it matched no element after the action wrapper change.
- Reviewed all nine `updates-{theme}.png` and
  `actions-{theme}-{Device-access,Plug-ins}.png` screenshots under
  `target/ui-quality-review/round-25/`: clear state, distinct buttons, no
  overflow in dark/light/high contrast. Backend actions and release-page
  authorization are unchanged. No live network, devices or audio were used.

F16's ordinary Settings toggle labels now cover startup, API auto-start and
version checks; verbose logging already had explicit state. Properties and
other tool toggles remain pending, including required guide updates. Next:
audit those controls against the property coverage manifest, implement shared
state presentation and exercise/refresh the affected guides. Other category
gaps and external gates remain open; no score is raised by this slice alone.

Seventh slice results (Windows Edge, 2026-11-06):

- Startup autoplay/API auto-start now show fixed-width On/Off, Loading or
  Unavailable text, associated with the native checkbox by description.
  Confirmed values stay visible through pending/rejected saves. Feedback now
  follows the shell's returned value rather than the requested value.
- Two new unit regressions failed before the change. Tests now cover pending
  reads/writes, unavailable reads, synchronous bridge errors and a differing
  acknowledgement. Existing save-failure and outside-desktop tests remain.
- Three-theme browser geometry assertions exposed 33.59 px growth after
  saving the two settings. Reserved message height fixes it; tests now cover
  keyboard changes, acknowledged state, failed saves retaining On, and an
  unchanged group height at 1440 and 640 px. Reviewed six wide/narrow images:
  `target/ui-quality-review/round-25/toggle[-narrow]-{theme}.png`.
- Resizing Settings to 640 px also exposed NaN SVG coordinates from the hidden
  canvas. An explicit console-error regression reproduced the actual failure,
  including pattern/minimap errors. `openRouteView` now leaves a zero-size
  canvas alone; it reframes when shown again. All three themes pass with no
  non-finite-coordinate errors, and returning to Tools then Fit stays finite.
  No console suppression or warning allowance was added.
- Final startup/canvas-aids browser suites: 12/12. Earlier startup plus
  Settings-wording check: 6/6 (that run exposed the now-fixed NaN warnings).
  Full unit suite passed 971/971 before the hidden-canvas guard; final owning
  unit suites passed 37/37 (AutoplaySetting and SessionFlowCanvas). Typecheck,
  final zero-warning lint, formatting,
  docs acceptance and diff whitespace check passed. Existing environment and
  React Flow attribution notices remain, as previously recorded.

Scope: shell commands are mocked in browser tests; no Windows sign-in, live
audio or the owner's settings were changed. F16 is only partly addressed:
other Settings/tool toggles and relevant Properties guide refreshes remain.
Next: apply confirmed-state presentation to the remaining controls, exercise
their errors/geometry and continue the full category matrix. Scores stay at
the Round 24 baseline until re-evaluation supports an increase.

Sixth slice results (Windows Edge, 2026-11-06):

- Added `settings-plain-words.pw.ts` to inspect rendered copy on five ordinary
  Settings pages in all themes, including text inside collapsed disclosures.
  The first probe raced a tab transition; waiting for the destination heading
  corrected the probe. It then reproduced N5 consistently in all three themes.
- Device access now says tool/device, including the goal disclosure. Plugin
  scanning says route; startup explains Windows sign-in setup; shortcut scope
  is stated as app-only; support explains route checkpoints. Exact permission
  identifiers and diagnostic records remain intact. The test excludes `code`,
  `pre` and raw diagnostic lists explicitly, not ordinary page prose.
- A second probe found the remaining "endpoints" in the collapsed setup
  guide. Replaced it and "draft" there; the final rendered check passed 3/3
  (five pages per theme). This covers N5's ordinary-page explanation text,
  not every backend error, log record, website page or guide video.
- Updated the two startup status assertions to the new caption, retaining
  their observed-registration behavior. Typecheck, zero-warning lint, full
  968/968 unit tests (105 files), final format:check, documentation acceptance
  (245 files/1421 links) and diff whitespace check passed. The first format
  check found the new browser file; formatted it and reran successfully.
- Visually inspected all 15 `settings-{theme}-{page}.png` captures under
  `target/ui-quality-review/round-25/`: text remains readable and contained in
  dark/light/high contrast. The route harness lacks native startup support;
  its startup unavailability is not packaged-shell evidence. No live devices,
  microphone, Windows registration or audio were changed.

N5 is repaired within that tested prose scope. F7 still has website/guide
follow-up. Next: F16 toggle states, their error/loading states and geometry;
also retain the observed touching setup/scan action buttons for the spacing
audit. Scores remain unraised and the full goal remains active.

Fifth slice results (Windows Edge, 2026-11-06):

- Before source changes, F2 failed to open Rename tool; F1 failed to open
  help in each theme (4 reproduced failures). F2 now opens the existing
  themed text dialog for exactly one selected tool. It keeps the target ID
  and applies to the latest draft only if the session is still the same.
  Empty names remain refused, Escape cancels and focus returns to the card.
- Added `KeyboardHelp.tsx`: F1 or ? opens the reference, with the current
  custom audio bindings and explicit disabled labels when they conflict.
  Focus stays within the scrollable reference and Close button; keyboard
  scrolling reaches the rest of the list, and closing restores the origin.
  Existing modal dialogs and text fields do not open a second help dialog.
- Help plus navigation browser suite passed 12/12. After strengthening Home
  scroll settling and the conflict caption, help passed 5/5 plus the new
  conflict regression 1/1. Checks cover 640x360 containment, custom bindings,
  no nested modal, empty-name validation, rename cancellation/commit and focus.
- Reviewed refreshed `target/ui-quality-review/round-25/help-{theme}.png`
  in dark/light/high contrast: readable labels, contained scrolling and a
  visible Close button. Typecheck, final lint/format:check, docs acceptance
  (245 files/1421 links) and diff whitespace check passed. Full UI suite
  passed 968/968 before the final conflict-caption-only refinement; that
  refinement was exercised by the focused browser regression above.

F13's listed shortcuts now have implementation and browser evidence across
these slices. Packaged-shell, Narrator and attended usability evidence remain
open. No category is re-rated by these changes. Next: N5 ordinary Settings
wording and F16 visible toggle states, followed by the remaining category
matrix, narrow-layout work and external gates. The durable goal stays active.

Fourth slice results (Windows Edge, 2026-11-06):

- New browser regressions failed before the fix: Ctrl+A left one of three
  cards selected, and Ctrl+equal did not increase zoom. Canvas-scoped key
  handling now selects tools/groups, duplicates one tool and controls the
  existing React Flow viewport. Field editing, handled events and key repeat
  are excluded. The key handler never rewires audio or changes positions.
- Duplication reuses the Properties action's backend-owned draft path and
  now selects the new copy consistently. A mixed/group/multiple selection
  cannot duplicate through Ctrl+D. The browser test exercises actual keys,
  requires four selected items for three tools plus a group, and verifies
  no extra tool appears while typing in the rename field.
- Navigation suite passed 7/7. Canvas aids, first-view and focus suites
  passed 15/15. Full UI unit run passed 968/968 in 105 files; typecheck,
  lint, formatting, docs acceptance and diff whitespace checks passed.
- Settings now lists the canvas keys. Its final three-theme browser checks
  passed 3/3, including containment; inspected
  `target/ui-quality-review/round-25/shortcuts-{theme}.png`. No new controls
  or Properties styling was introduced. UI-02/11/17 and ENG-02/04 covered.

Still pending: F2 rename and one-key shortcut help, then the remaining
non-keyboard findings and evidence gates. Scores remain at the Round 24
baseline; these harness results do not establish Narrator, packaged shell,
Windows display scaling or attended real-route behavior.

Third slice results (Windows Edge, 2026-11-06):

- New `navigation-shortcuts.pw.ts` reproduced missing skip links in three
  themes. Its first Save probe omitted the Rename step and was corrected;
  the corrected probe then failed at the actual Ctrl+S save assertion.
- Added focus-only links before the header, with named focusable canvas and
  workbench targets and theme focus outlines. Enter moves focus; the next
  Tab from the workbench reaches its selected tab. Canvas geometry is unchanged.
- Header and Ctrl+S now call the same guarded save action. The shortcut uses
  `useEffectEvent`, so it sees current edits without a telemetry-driven effect.
  It ignores repeated keys, text editing and modal dialogs. Ctrl+S is reserved
  from custom audio bindings, with a regression against accidentally starting
  audio while saving. Settings lists Save, Find and skip navigation.
- During screenshot review the skip link background was transparent because
  it referenced an undefined token. Corrected to `--ambient-surface`, added an
  opacity assertion, and inspected refreshed screenshots in all three themes.
- Navigation, start-after-save and card-focus browser checks passed 8/8;
  after the background repair, navigation passed another 4/4. Save regression
  uses a connected fixture route and requires exactly one backend commit,
  with no save from an editing field or an open New session dialog.
- Typecheck, zero-warning lint and format:check passed; full UI suite passed
  968/968 across 105 files. Docs acceptance (245 files/1421 local links) and
  diff whitespace check passed. Screenshot evidence:
  `target/ui-quality-review/round-25/skip-{theme}.png`.

Limits: browser harness only; no packaged shell, Narrator, real Windows scaling
or native audio exercised here. The fixture does not persist across reload,
so saved-state evidence is its backend commit and refreshed snapshot, not a
restart/persistence claim. F13 still needs Select all, Duplicate, Rename,
keyboard zoom and one-key shortcut help. No category has been re-rated yet.

F16 toggle-state slice (Windows Edge, 2026-11-07):

- Every switch built on `BooleanField` now gives its state as On or Off in the
  accessible description and as visible text. The cable Low latency switch says
  On for a 128-frame period and Off for any other reported period. With no
  reported period it says Unavailable and stays disabled, so a click sends nothing
  (`VirtualCablePanel.test.tsx`, which failed first).
- The route harness can now fake the cable status and record latency requests.
  It never calls the shell or driver. New `boolean-states.pw.ts` cable case: Space
  sends exactly one `setLowLatencyOn`, the description changes Off to On, and the
  field stays in place, in all three themes.
- Screenshot review found the state word pushed to the far edge of the row, away
  from its switch. It now sits beside the switch, with a fixed width so the row
  does not move. Inspected `target/ui-quality-review/round-25/boolean-*-{theme}.png`.
- Browser checks: boolean-states, property-flag-states, settings-controls and
  device-access-recovery passed. Typecheck, zero-warning lint, format:check and
  the full UI suite (983/983, 108 files) passed.

Limits: harness only. The real driver's latency switch was not changed.

N3 and N4 are repaired in the harness, and N6 is repaired at 1440. Category
scores remain unchanged pending the remaining matrix. Next action: narrower
first-view geometry and the remaining keyboard paths, then N5/glossary,
toggle labels and the slow-action matrix. The durable goal remains active.

## Round 24 — re-rating after Round 23 (owner request, 2026-11-06)

**Method.** Round 22's scripts rerun unchanged on `32072ee0` (#120): route harness in Edge, 22 screens in three themes,
axe-core 4.14, inventory, Properties for each tool, keyboard walk, shortcuts on an unsaved route. Added: a real message
against the tabs at 5 window sizes and 5 scroll positions, both app dialogs in three themes, jargon on each screen, first
view at 4 window sizes, layout and animation during simulated playback, and keyboard focus on cards by comparing pixels.
Screenshots and JSON: `target/ui-quality-review/round-24/` (not committed).

**Limits.** This agent wrote most of the fixes it is grading. Harness audio is simulated. The owner's shell was running,
so the exe was not driven this round; the last exe run is Round 23's, on `ec1df0e5`, and the exe on disk predates the
later UI repairs. "Review before saving" was measured with an inserted element using the real class, because the harness
returns no warnings. No Narrator, Windows scaling, custom themes, first-time user or real device. Hosted Windows and Edge
checks on this head were still running when these scores were set.

### Scores

| # | Category | Round 22 | Now | Main reason | Still holding it back |
| --- | --- | --- | --- | --- | --- |
| 1 | Contrast | 8 | 8 | Axe clean on 22 screens in three themes and 36 tool panels in dark; N1 fixed | Graphics not pixel-audited (F21); no focus mark on cards (N3) |
| 2 | Typography/size | 8 | 8 | Nothing under 11 px; 6 sizes | Windows scaling; chart labels unmeasured |
| 3 | Clarity | 7 | 8 | No port lines, Native or "draft" on the daily screens; "Unsaved" | Card repeats itself (N4); engine words on 5 Settings pages (N5); route and session both used |
| 4 | Consistency | 7 | 8 | App dialogs replace browser ones (2 of 5 exercised here, 3 rest on Round 23's tests); one name per tool in the app | 274 colours, 49 `!important`, 12 button heights (F19); website still says "Physical input" |
| 5 | Spacing/alignment | 7 | 7 | F10 fixed, 0 of 25 positions cover the tabs | Third card cut off by default at 1440 and 1280 (N6); 5 px tab padding |
| 6 | Navigation | 8 | 7 | Discovery, not a regression: card focus has never been visible (N3) | N3; no skip link; Ctrl+S and others still do nothing (F13) |
| 7 | Errors/recovery | 8 | 8 | F4 fixed: edits survive quit and forced kill (Round 23's exe run on `ec1df0e5`, not rechecked here) | Tray Quit not clicked; live device loss untested |
| 8 | Inputs | 7 | 8 | Browser prompt gone; empty name refused in the dialog | 0 of 5 toggles measured say On or Off (F16) |
| 9 | Buttons | 8 | 8 | Delete icons centred | 2 targets under 24 px; 12 heights |
| 10 | Loading/progress | 8 | 8 | Session stays usable while history loads, 336 ms here | Slow-action matrix incomplete |
| 11 | Menus/dropdowns | 8 | 8 | Palette and card menu work by keyboard (Shift+F10 opens it, Esc closes) | Native lists; browser menu on panels (F27) |
| 12 | Icons | 8 | 8 | Unchanged | No check of icon meaning with users |
| 13 | Drag/drop | 8 | 8 | Arrow keys still move a card 24 px | Real route not used (F23); N3 makes the keyboard path harder |
| 14 | Motion | 8 | 8 | No endless animation; layout steady in 30 samples while card values changed; under reduced motion only 0.01 ms meter updates run | No attended check on a live route |

Total 110 of 140 (average 7.9), against 108 for Round 22's table. Lowest 7, twelve categories at 8, none at 9. Clarity,
Consistency and Inputs each rose by one. Navigation fell by one only because N3, which was already there, was found, so
judged like for like the work gained 3. No 10 can be awarded while the four external gates in Round 20 are unmet.

### Findings retested

- **Done:** F4 (Round 23's exe run), F6, F8, F10, N1, N2 (inserted element at 3 window sizes). F7 is done on the daily
  screens only. F25 still holds: first screen 1060 px, all open 2305 px.
- **Unchanged:** F13 (Ctrl+S, Ctrl+A, Ctrl+D, F2, F1, `?`, Ctrl+0/+/− do nothing, no skip link), F16, F19, F21, F23,
  F24 (9 tool panels fit, median 1013 px, the same as Round 22), F27.

### New or newly measured

- **N3. Keyboard focus on a canvas card shows nothing.** Tab reaches each card and `:focus-visible` matches, but React Flow's
  stylesheet sets `outline: none` on it and `styles.css` draws nothing in its place, at Round 22 as well. Dark: the card's
  pixels are identical focused and unfocused. Light and high contrast: the image bytes differ, but side by side nothing visible changes. Axe cannot detect this, so
  every earlier round missed it. WCAG 2.4.7, and rubric category 1 requires visible focus. Not fixed: this round only grades.
- **N4. The card says the same thing twice.** "INPUT", "INPUT DEVICE", "NO DEVICE" and "No device chosen" share one card.
  This width is also why N6 happens: Headphones is 282 px against 218 px for Gain.
- **N5. Engine words remain on ordinary Settings pages.** Device access ("node Properties", "endpoint"), Plug-ins ("graph"),
  When AudioRouter starts and Keyboard shortcuts ("Native"), Logs and support ("graph", "nodes"). The Round 23 word check
  excludes developer panels, but these are not all developer pages.
- **N6. The third card of a three-card route is cut off in the first view.** Not new: Round 22's picture shows it, and
  Round 23 noted it at 1000 px. Measured now. Cards fully visible: 3 of 3 at 1920×1080, 2 of 3
  at 1440×1000 and 1280×720, 1 of 3 at 1000×700. At 1440 it misses by 32 px, and its delete button and status are clipped.
  Readable zoom (F20) is why it does not shrink to fit, so narrower cards are the better repair.
- **Seen, minor:** at 640×360 the message moved to the top sits over part of the map's close button.

### Next, by expected gain

1. N3: draw a focus ring on cards. Small, and it returns Navigation to 8.
2. N4 then N6: remove the repeated labels, which narrows cards so three fit at 1440. Lifts Spacing.
3. N5, then the guide clips and website wording, to finish F7.
4. F16, the rest of F13, F27.
5. The external gates (Narrator, Windows scaling, first-time user, attended real route), which every 9 depends on.
