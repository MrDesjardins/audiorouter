# UX-03 state clarity — 2026-11-02

Objective: distinguish saved routes, temporary draft playback and edits awaiting
Save in existing UI space. Requirements: UX-03, UI-04/05/07/09/17, UC-07.

Prerequisites/readings: AGENTS.md, documentation map, current execution record,
M05/M08, interface/workflow specifications and parent's read-only UX proposal.
Backend retains validation, persistence and playback authority; no API changes.

Ordered tasks (recorded before implementation):
1. Label file export as saved-version export, including progress/success/failure;
   clear obsolete success when a new operation starts or is cancelled.
2. Clarify header route Play versus source Play and draft/save state using
   existing control labels/tooltips and reserved run-state slot.
3. Typecheck promptly; add bounded mock component regressions; format/lint,
   inspect diff and commit this isolated slice.

Validation: Windows host, source-only typecheck/lint and single-worker mock
Vitest. No live backend, real profile, audio/device changes, native build,
running artifact writes or browser launch permitted while owner is gaming.
Native playback, concurrent assistant/save conflict journeys, three-theme
visual checks and attended M05/M08 gates remain unrun. No release claim.

Risks: a UI-started preview cannot establish externally changed runtime state;
avoid claiming that every running route is the saved revision. Export is a
backend snapshot at export time, not necessarily the revision seen before a
file dialog opened. No save-then-export path will be introduced.

Rollback: revert this slice; no storage or contract migration. Next: implement
and record bounded verification results here, then parent integrates.

## Implementation and verification

Implemented saved-version export wording (no save or commit is issued), cleared
old operation success before another file operation including cancellation,
and clarified header route/source transport scope in existing tooltips. The
reserved header status says Draft preview only for a remembered preview whose
generation matches fresh backend diagnostics; otherwise it says Route running,
without claiming audible signal or a saved revision. Saved/draft explanations
remain in the existing Save/status controls and Session file note.

Windows source checks (2026-11-02): `npm.cmd --prefix ui run typecheck` passed;
`npm.cmd --prefix ui run format` and `npm.cmd --prefix ui run lint` passed;
`git diff --check` passed. Initial typecheck identified nullable telemetry;
fixed with optional access and guarded generation matching. PowerShell blocked
the npm.ps1 wrapper; npm.cmd works. Missing local dependencies were populated
with offline `npm.cmd --prefix ui ci --ignore-scripts --offline --no-audit
--no-fund` in this worktree only, with no lockfile changes.

`npm.cmd --prefix ui test -- src/SessionFilePanel.test.tsx
src/routePlaybackLabel.test.ts --maxWorkers=1`: 2 files / 5 tests passed,
one concurrent worker. Covers saved-only export arguments, failure replacing
success, cancellation clearing success, preview/stop/busy states, stale or
externally replaced generation, missing telemetry and unknown runtime origin.
Vite cache was explicitly confined to this worktree.

Unrun: native/app/browser/audio/export round-trip, live save conflict and
concurrent assistant journeys, three-theme geometry, Narrator and scaling.
These remain parent integration/attended gates under the owner's gaming
restriction; no running application or real data was accessed. No tool
Properties controls or API/contracts changed, so no tool guide capture or
API catalog regeneration is needed. Next: parent cherry-picks this commit,
resolves narrow App.tsx overlaps, and schedules those gates when authorized.
