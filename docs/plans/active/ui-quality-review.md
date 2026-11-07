# UI quality review — 2026-11-03

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

Next action: owner reviews #118 and the #117 order, and chooses from the list above.
