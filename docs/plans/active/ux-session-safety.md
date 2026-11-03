# UX-01 session switch safety

Current status (2026-11-02): source integrated as `7b2bd937` in parent
`bf99cf97`, with combined cancellation regression evidence in
[current.md](current.md). Restrictions below describe the completed child
assignment. Parent final qualification now owns themed/keyboard/Narrator and
native recorder-finalization gates; these remain unrun in this slice.

Objective: disclose the existing stop-all policy before session navigation.
Requirements: UX-01, UI-08/09/10/14, UC-07/08, REC-04/12.
Authority: assigned implementation slice, source-only isolated worktree; the
owner is gaming. No live backend, real data, devices, native builds or app use.

Prerequisites/decisions: read AGENTS, documentation map, current execution
record, M05/M08, interface/workflow/recording and delivery decisions, and the
parent's read-only future UX plan. Preserve backend lifecycle authority and
stop-all policy; no new API, independent viewed session or storage migration.

Ordered tasks (recorded before edits):
1. Add app-style accessible confirmation at the common switchSession boundary.
2. Refresh running IDs before disclosure and again before stops; changed targets
   require renewed disclosure. Failed refresh cannot authorize interruption.
3. Preserve draft/selection on cancel; disclose recorder finalization and partial
   stop failures without claiming audio resumed or recordings completed.
4. Add focused mock regressions and run single-worker tests, typecheck, format
   and lint locally. Inspect diff and commit the slice only.

Validation matrix: mock navigation with unrelated active sessions, cancellation,
confirmation, changed running IDs, partial failure; dialog keyboard/focus tests.
Native recorder finalization, all-theme visual QA, Narrator and packaged M05/M08
gates deferred because this assignment prohibits app/native execution. Mocks
cannot establish audio continuity or durable recording outcomes.

Risks: external automation can change state after a refresh; no atomic backend
stop-set API exists. Report confirmed stop outcomes only; never auto-restart.
Rollback: revert this slice; no data migration. Bounded mock checks and source
integration are complete; attended qualification is next.

## Implementation and evidence (2026-11-02)

Common App.switchSession now obtains fresh status before prompting and after
confirmation. Changed running IDs require a new prompt; failed refresh cannot
trigger stops. Cancel performs no stops and does not change selection or reset
the draft. Existing dropdown, create/copy/import and other callers retain this
common boundary. Startup/external active-selection synchronization does not
stop audio and remains unchanged. No contracts/backend changes were needed.

SessionSwitchDialog uses the existing themed dialog/button classes, named and
described modal semantics, initial Cancel focus, Escape cancellation, trapped
Tab focus and focus restoration. Long lists/names can scroll/wrap. It explains
all listed sessions will stop, recording finalization/partial-file risk and
explicit Play. Stop failures preserve selection, name completed stops and warn
that other audio/recording state may have changed; never auto-resume.

Windows isolated worktree, copied local UI dependencies (no shared output):
`npm.cmd --prefix ui run typecheck:all` passed after correcting unsupported
Testing Library role options. `npm.cmd --prefix ui run format` and
`npm.cmd --prefix ui run lint` passed (no unrelated formatting changes).
`npm.cmd --prefix ui test -- App.sessionSelection.test.tsx --maxWorkers=1`
passed six tests before the additional failed-refresh regression; final rerun
recorded below. `git diff --check` passed. The initial npm PowerShell shim was
blocked by execution policy; npm.cmd worked without changing policy.

Integration: cherry-pick this slice and resolve overlapping App edits around
switchSession/imports/state/modal rendering. UX-02 navigation must await the
returned switched flag before selecting a node. No live/backend/native checks,
release builds, full suites, PR or push performed. The unavoidable automation
race between final status read and sequential stops remains a backend atomicity
limit; independent viewed/running sessions remain out of scope. Next: parent
disposable-profile themed/keyboard/Narrator and native
recording-finalization qualification under M05/M08.

Final verification: seven session-selection tests passed with one worker
(13.22 seconds), including stale-state refresh refusal and second-stop
recording failure. Final typecheck:all, format, zero-warning lint and diff
checks passed. Full changed-file diff inspected; only this slice is staged.
