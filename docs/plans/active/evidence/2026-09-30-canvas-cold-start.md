# 2026-09-30 canvas cold-start connection recovery

Affected build: source `8a2a01e0`, shell SHA256
`1F4C300842EA3F51BB2C7779A63B4678D2883EFE07A0DB03304FA8243355E11D`.
Requirements: UI-02/03/04, GRAPH-01/02, ENG-01/04/05.

## Observed failure and limits

User reports missing links and inability to drag connections after launching
the rebuilt desktop. Right-click Refresh restores links and blue connectors.
Read-only SQLite inspection found the selected session at revision 130 with
13 nodes and 11 edges. Shell logs show successful `sessions.get` responses
containing those same counts. Saved graph data was not erased.

Normal optimized browser fixtures render/reconnect correctly. A sanitized
copy of the actual node/edge shapes, saved layout and groups also renders,
including delayed fixture responses. These checks do not reproduce the exact
native cold-start race. Refresh recovery suggests initialization of connector
geometry, rather than persistence. The precise native timing remains unconfirmed.

The prior handoff checked builds, artifacts and ordinary theme rendering,
but lacked a cold-start production connector regression. That validation gap
allowed this user-visible failure to reach manual testing.

## Repair and focused regression

A first card-size measurement with unavailable connector elements leaves
React Flow with empty handle bounds. Without another resize, edges cannot
render and connection hit testing cannot finish. The deterministic production
test suppresses only the first source/target connector lookup per card while
preserving later DOM queries. Before repair it fails on the missing saved
edge; local log `target/canvas-cold-before.log` (exit 1).

`CanvasConnectorMeasurements` checks expected handle identities and valid
finite, nonzero geometry. It forces measurement only for missing/invalid
bounds, with at most eight animation-frame retries per geometry change and
one bounded retry sequence after fonts finish loading. Cleanup cancels its
pending frame. It runs on node/port geometry changes, not telemetry polling;
audio, persistence and node coordinates remain unchanged.

The regression asserts existing SVG edge paths, then removes a connection
and reconnects with an actual blue-to-orange mouse drag without refreshing.
Runs in dark/light/high contrast. Existing two-path playing-line and drag tests
run against optimized assets too. This reproduces and fixes a concrete
initialization failure class; the user's exact native first launch still needs
confirmation on the new executable.

## Verification

Windows x64, Edge headless; no microphone capture or audio endpoint opened.
From `ui`:

```powershell
npm.cmd run e2e:production-canvas
npm.cmd run test -- --run src/SessionFlowCanvas.test.ts src/CanvasGroups.test.tsx
npm.cmd run typecheck
```

Initial post-fix browser assertions reached successful drag, but an edge-count
selector also counted six decorative legend SVGs. Corrected it to actual
`.react-flow__edges .react-flow__edge`; no behavior threshold was lowered.
Five optimized browser tests then pass (three cold-start themes plus two playing
canvas tests). Thirty focused unit tests and TypeScript checking pass.
A pre-existing group test expected opacity 101 to be rejected despite the
previously implemented import clamping; corrected its expectation to opacity 100.
Local browser log: `target/canvas-cold-after.log`; screenshots under
`%TEMP%/audiorouter-playwright-results/canvas-cold-start*/cold-start-*.png`.
Final optimized regression rerun: `target/canvas-cold-final.log`, five pass,
exit 0. All three screenshots inspected; saved lines and reconnected line are
visible. Docs acceptance passes: 80 Markdown files, 423 local links; diff check
passes.

Production assets use the same Vite manual graph/vendor chunk configuration as
the desktop. The deterministic fixture backend does not claim native backend
or hardware qualification. Read-only temporary reproductions stay under target,
not source-controlled; no user DB, layout, app process or endpoint was modified.

## Corrected manual artifact

`C:\code\audiorouter\target\canvas-startup-20260930\release\audiorouter-shell.exe`
was built with locked dependencies, optimization and `custom-protocol` after
the final UI build. `ui/dist/assets/index-I8X_Go-x.js` includes the repair;
the shell timestamp (2026-10-01 00:41:14 UTC) is newer than that bundle.
Size: 22,534,144 bytes. SHA256:
`FEAAF86327D2E13E0803EA03D14E351E4BB66A871631C4860C8A6B7EEB4423FF`.
All three sibling executables pass the repository x64 PE validator. The worker
and CLI are copied from the preceding verified build because their sources
and protocol did not change; hashes match the active plan's previous artifact
record. The running app was left untouched, and the new shell was not launched.

Rollback: run the preceding executable; reverting the renderer recovery and
test configs requires no storage migration. Next: first native launch and drag
confirmation by the user, followed by remaining native audio qualification.
