# Final UX browser qualification — 2026-11-02

Scope: isolated branch `t3/ux-final-qualification`, Windows host, Node
22.22.3, installed Microsoft Edge, disposable Playwright contexts and the
in-memory `route-harness.html`. Requirements: UX-01–05; UI-05/08–12/17/27;
M05/M08 browser portions only. No native backend, real profile, device,
driver, production audio, owner process, push, PR or merge was used.

## Change and regression

The existing `workbench-organization.pw.ts` regression failed at all three
sidebar widths (404, 428, 520): rounded tab heights alternated between 38
and 39 px. Two-line content plus padding/borders exceeded the 2.4 rem
minimum. Reserve 2.5 rem for every tab. The unchanged regression now passes;
the new scenario additionally requires equal heights in each theme/viewport.
Rollback: revert this slice; no API, storage or audio semantics changed.

The route harness accepts optional saved routes and active-session IDs so
navigation can be exercised without a native fixture. Defaults are unchanged.
The new six-scenario test covers two active sessions in the confirmation,
initial Cancel focus, Escape cancellation with zero lifecycle calls, confirmed
stop of both sessions, stopped destination, finding navigation and Properties
tab/panel keyboard entry, Home/End/Right tab keys, roving tab index, panel
relationships, saved-version export label, and recording-goal guidance with
zero lifecycle calls. Tests run in dark/light/high contrast at 1280×720 and
640×360 CSS pixels. The latter is the viewport equivalent of 200% at 1280×720,
not an attended OS scaling or native WebView2 check.

## Commands and outcomes

- `npm.cmd --prefix ui ci --ignore-scripts`: passed (261 packages).
- `node ui/scripts/qualify-browser-guard.mjs`: positive passed; deliberate
  negative failed at the guard as required. Evidence:
  `%TEMP%/audiorouter-browser-guard-pKGDOu`.
- `npm.cmd --prefix ui run format`, `format:check`, `lint`, and
  `typecheck:all`: passed; zero ESLint warnings. Only intended files changed.
- Final Playwright batch: **23 passed, 27.4 s, one worker, no retries**:
  `ux-final-qualification`, `setup-check`, `workbench-organization`, and
  `header-canvas-usability`. The six strengthened scenarios separately
  passed in 9.9 s before this final batch.
- `git diff --check`: passed.

Reproduction from the repository root:

```powershell
node ui/node_modules/@playwright/test/cli.js test --config ui/playwright.config.ts ux-final-qualification setup-check workbench-organization header-canvas-usability --workers=1
```

The first normal-config run completed assertions but stayed in dev-server
cleanup in this restricted environment. Sent Ctrl+C only to that owned exec
session. For subsequent runs, retained the known owned Vite fixture and used
a temporary config importing `playwright.config` with `webServer: undefined`,
`workers: 1`, and `outputDir: "../target/ux-final-browser"`. These runs exited
normally; the temporary config was removed afterward. Finally stopped the
owned original fixture session with Ctrl+C. No existing server was adopted.

Final raw output: `target/ux-final-browser-run.log`; screenshots:
`target/ux-final-browser/`. These local ignored artifacts remain in this
worktree for parent inspection. Used `view_image` to inspect confirmation,
finding/Properties, goal-guide, and failed tab-regression screenshots across
all three themes, including the small viewport. Dialog text/buttons are
contained at both sizes; focus indication and app-themed fields are visible.
Existing setup-check tests prove unchanged heading/tab bounds as findings
change; header tests prove stable canvas/status bounds during run-state changes.

## Limitations and warnings

Persistent notifications can overlap content/tabs in the small viewport.
The success notification's Dismiss control is reachable; the test deliberately
dismisses it before pointer navigation. Guide content remains scrollable.
Do not represent this as a complete no-overlap scaling audit.

Vite logged React Flow attribution and transient zero-size startup warnings
at 640×360. The new test waits for a settled graph height greater than 100 px;
all scenarios pass with no uncaught UI exceptions. These console warnings
are not native rendering or performance evidence. Playwright also prints
the runner's NO_COLOR/FORCE_COLOR environment conflict; no source change
was made for that environment warning.

Not run: native/packaged UI, actual 200% desktop scaling, Narrator, live audio,
recorder finalization/partial stop failure, export file round-trip through the
native shell, full keyboard route construction, five-participant onboarding,
hardware/recovery/performance measurements, full unit suite or release build.
Parent owns those gates and integration. No release-ready claim.

Next: parent cherry-picks the qualification commit, reruns combined checks,
then builds and smokes the packaged app in a disposable profile. Preserve
these explicit M05/M08 attended and audio gaps in the final handoff.
