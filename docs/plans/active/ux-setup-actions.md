# UX-02 setup actions

Objective: open affected tools by stable session/node ID and focus Properties.
Requirements: UX-02, UI-05/08/17/19/27, UC-06. No API changes.
Prerequisites: existing setup-check lifecycle and centralized switchSession;
UX-01 confirmation will be integrated by the parent. Preserve backend authority.

Steps before implementation:
1. Thread an optional finding action through SetupCheck and SetupWorkbench.
2. Validate connected state and current objects; preserve dirty drafts by refusing
   cross-session navigation until the user saves or discards deliberately.
3. Delegate cross-session transitions to switchSession, then select by ID after
   the target draft loads, and focus Properties. Explain stale/deleted targets.
4. Run typecheck promptly, focused mock tests with one worker, formatting/lint,
   and inspect the complete diff before committing.

Validation matrix: component actions and stale results via mocks; navigation
guards via focused regression tests. Native audio, real backend, browser/theme,
Narrator and packaged gates are deferred under the gaming restriction. No live
profiles, devices, builds, sidecars or running processes will be touched.
Risks: async refresh can remove a target; revalidate after session load. Feedback
findings without node IDs cannot identify a unique tool; explain this limitation.
Rollback: revert this slice; no persistence migration.

Implementation: SetupCheck/SetupWorkbench expose optional finding actions. App
checks connectivity, session presence and cross-session draft changes, delegates
to centralized switchSession, and resolves the node in the loaded draft. Missing
tools explain why navigation failed. Findings without a unique node open Session
for route inspection (including feedback); no automatic rewire. Focus moves to
the selected Properties tab. Existing fixed findings viewport and result lifecycle
are unchanged. Same-session draft inspection retains edits and states that saved
checks do not cover them. Parent must integrate UX-01 before enabling safe
cross-session use; this slice deliberately does not duplicate its stop policy.

Windows source-only validation: local npm ci --ignore-scripts succeeded; initial
commands could not run because dependencies were absent and PowerShell blocked
npm.ps1 (used npm.cmd). Typecheck passed; format and lint passed. Focused Vitest
SetupCheck and App.sessionSelection: 13/13 passed, --maxWorkers=1. The App mock
test exercises the real finding button, stale display names, Properties focus
and absence of stop calls for same-session navigation. Component tests cover
stable IDs, route inspection and disabled retained actions after disconnect.
An attempted npm exec formatting command had the wrong relative path; reran
the normal format script. No native binaries or live backend calls were made.
Final normal format, zero-warning lint, typecheck:all and git diff --check pass.

Remaining gates: cross-session confirmation/cancellation and partial-stop tests
with integrated UX-01; all-theme fixed geometry, Narrator/scaling and packaged
Windows checks. No API/schema/storage change; no REST/MCP regeneration needed.
Next: parent integrates UX-01 and runs combined navigation acceptance in a
disposable profile after gaming restrictions lift.
