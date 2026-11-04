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
