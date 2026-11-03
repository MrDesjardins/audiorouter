# UX improvement plan — 2026-11-02

## Objective and authority

Owner request: analyze the current project for user-experience obstacles and
make an improvement plan. This is a source-based assessment and proposed M05/M08
follow-up, not authorization to implement, change audio settings, or open a PR.
Reviewed on `feature/setup-check`; working tree was clean before this document.
No application code changed. No live audio, device changes, builds or user study
were performed. Findings below distinguish observed code from usability hypotheses.

## Evidence and existing strengths

Sources: [interface requirements](../../spec/09-interface.md),
[workflows](../../spec/02-workflows.md), [delivery decisions](../../spec/15-delivery.md),
[M05](../../milestones/M05-visual-editor.md), [M08](../../milestones/M08-release.md),
[current execution record](../active/current.md), and
[setup-check implementation and verification](../active/setup-check.md).
The long execution history was sampled; this is not a comprehensive release audit.

Preserve existing backend validation, source meters, templates, node-specific
Play errors, protected microphone failure behavior, keyboard graph controls,
three themes, local support bundles and default MCP parity. Setup checks already
refresh outside Status and reserve an attention-dot slot. Do not propose those
as missing features. Their latest native visual check and attended audio evidence
are still qualified by the setup-check plan.

## Prioritized findings and acceptance

### UX-01 — Session navigation can interrupt unrelated audio (first)

Observed: `switchSession` in `ui/src/App.tsx` iterates through
`snapshot.status.activeSessionIds` and stops every session before selecting the
new one. It reports the interruption afterward. Inspecting a different saved
route can therefore interrupt a call or another route started through automation.
This is existing deliberate behavior, not a newly reproduced defect.

First improvement: show the consequence and affected running sessions before
switching; cancellation must leave audio and selection intact. Review existing
session/device ownership decisions before changing this policy. Separating
viewed session from running sessions is a later architectural option, not a
UI-only change and not automatically approved by this plan.

Acceptance: two active sessions, one unrelated to the visible canvas; inspect
another session; cancel produces no stop calls. Confirmed switch accurately
lists its effects. Verify partial stop failure and recording finalization.
Requirements: UI-08/09/10/14, UC-07/08. Effort: medium for disclosure; large for
independent navigation. Dependency: settle stop-policy scope first.

### UX-02 — Diagnostics explain problems but leave navigation to the user (first)

Observed: `ui/src/SetupCheck.tsx` renders severity, message, fix, session name
and node name as text. It has Check again but no action for opening the affected
node. With many routes the user must find the location manually.

Add an Open affected tool action using the finding's stable IDs, select the
node and focus Properties. For cross-session findings, obey UX-01 before
navigation can stop audio. For feedback, offer inspection of the contributing
path rather than an automatic rewire. Preserve last-result and incomplete-check
wording; a saved-route result must never certify an unsaved draft.

Acceptance: missing microphone, wrong cable format, three-session feedback,
deleted node during refresh, disconnected backend, and draft edits. Every
action either reaches the right object or explains why it cannot. No silent
device substitutions or Windows format changes. Requirements: UI-05/08/17/19/27,
UC-06; AUTO-18 if backend methods change. Effort: medium.

### UX-03 — Saved, previewed and audible state require too much interpretation (first)

Observed: `ui/src/App.tsx` has live flag commits, autosave, explicit Save,
temporary unsaved-route playback and source-specific Play controls. The header
primarily says Audio running/stopped. `ui/src/SessionFilePanel.tsx` warns about
unsaved edits but still exports the saved version with a Save to file button.
The mechanisms are intentional; misunderstanding their combination is a
usability hypothesis to test, not proof of lost data.

Use consistent language for saved route, draft preview and changes awaiting
Save. Explain whether header Play starts a route or an individual source next
to the relevant control. Name export as exporting the saved version; offer a
deliberate save-then-export flow only through existing validated commit logic.
Failed/conflicting saves must never export a file represented as the new draft.
Use reserved status space rather than another persistent route-status panel.

Acceptance: live gain change plus unsaved wiring; Play preview; Stop/restart;
failed Save; concurrent assistant edit; export with unsaved changes. The user
can identify what is audible, what survives restart and what the file contains.
Requirements: UI-04/05/07/09/17, UC-07. Effort: medium.

### UX-04 — Setup knowledge is spread across too many destinations (next)

Observed: `ui/src/Workbench.tsx` defines 13 peer tabs. Setup guidance appears
in Tools help, Status, Apps and Virtual Cables (`SetupWorkbench.tsx`). Existing
templates and app checks already help; this finding does not claim onboarding
is absent. The hypothesis is that novices must learn the internal organization
before completing a simple processed-microphone task.

Test a short goal-led path using existing templates: choose what to do, choose
source and destination, verify local signal, then verify the receiving app.
Explain CABLE Input versus CABLE Output with the exact selected endpoint names.
Distinguish device availability, signal received and external-app verification.
Missing external prerequisites should lead to instructions, never a false
success. Keep the canvas and advanced workflows available; test grouping less
frequent tabs before changing navigation globally.

Acceptance: processed microphone, gaming plus Discord, app recording; missing
cable; app closed; silent source. Monitoring starts muted and recording unarmed.
Requirements: UI-01/06/09/12/26, UC-01/02/03/08. Effort: medium to large.

### UX-05 — Keyboard and scaling require an attended gate (next)

Observed: Workbench declares a tablist and tabs but its tab-rendering loop has
no roving tab index, arrow-key handler or tab/panel association. Existing
keyboard graph controls do not establish that navigation works well with
Narrator. M05 explicitly retains attended accessibility/scaling evidence gaps.

Implement and test the standard tab interaction, stable focus after changing
panels, accessible panel relationships and focus restoration after errors.
Audit long endpoint names and actionable findings at 1280×720 and 200% scaling.
Do not infer screenshot or assistive-technology results from source inspection.

Acceptance: keyboard-only build/save/play/stop/export/recovery; Narrator can
identify selected tab and panel; no clipped essential control or focus trap;
dark/light/high contrast and reduced motion. Requirements: UI-11/12/17, M05/M08.
Effort: small for tab behavior; medium for full attended qualification.

### UX-06 — Performance and recovery should be measured as user journeys (then)

Evidence: the setup-check plan records a 633.03 kB main JS chunk warning and
remaining native testing limitations. This warning alone does not prove slow
startup. The current plan also records plugin licence/recovery caveats and
version-sensitive saved settings; those deserve understandable recovery text.

Measure cold/warm usable startup, panel response under telemetry and recovery
after unplug/replug, sleep/resume and plugin failure. Split expensive optional
panels only if measurements identify loading/render cost. Retain the last
valid state while clearly indicating stale/disconnected data. Explain when
Retry suffices and when Stop/Play is necessary; never auto-resume private audio.

Acceptance: meet NFR-14's 3-second warm/8-second cold ready limits on declared
hardware; record distributions and workload. Check backend logs and actual
audio continuity separately. Requirements: UI-08/12/13/17, NFR-14, UC-06/09.
Effort: medium investigation; remediation depends on results.

## Ordered execution and prerequisites

1. Baseline: observe five first-time users on UC-01, recording and failure
   recovery. Record completion, time, wrong turns, assistance and understanding
   of the receiving app's source set. Keep private audio out of evidence.
2. Confirm UX-01 policy, then implement UX-01/02 as separate reviewable slices.
3. Implement UX-03 with state-transition tests and export round-trip evidence.
4. Prototype and test UX-04; implement the successful variant. Complete UX-05
   alongside it, not as post-release polish.
5. Run UX-06 measurements and fix demonstrated bottlenecks. Repeat the same
   first-time-user tasks with fresh participants and retain raw observations.

Before each slice, read its complete linked specs and relevant decision records,
record the exact implementation/rollback steps in the active plan, and preserve
unrelated changes. Prerequisites: Windows packaged app, disposable profile,
supported endpoints, headphones, optional licensed plugin fixture, Narrator,
and participant availability. Owner's real database/audio remain untouched
unless separately included in the authorized test session. No driver install,
credential purchase, telemetry service or remote control is proposed.

## Verification and completion gates

- Component and backend regression tests exercise the actual action and failure
  boundary; REST dispatch/default MCP tests and contract drift for changed APIs.
- Browser tests verify navigation, focus, stale requests, long names and stable
  geometry in all three themes. These are not native audio evidence.
- Packaged Windows tests verify saved/exported content, device recovery,
  audible continuity and recorder finalization with reproducible fixture routes.
- Narrator, keyboard-only, 200% scaling and fresh-user observation are attended
  gates. M08 requires at least four of five first-time users to complete setup
  within ten minutes and all to identify Discord's source set.
- Record command/build, machine, result, evidence location and unrun cases.
  Run required formatting, lint, typecheck and relevant regression checks.
  Refresh guides and real portrait Properties videos if tool controls change;
  validate their Firefox playback. Do not call this release-ready from mocks.

## Risks, rollback and next action

Navigation changes can accidentally stop recording; simplification can hide
important safety information; autosave changes can discard drafts or commit
unrelated edits. Preserve backend authority and existing permission boundaries.
Prefer UI-only additive slices with no storage migration. Roll back each slice
independently while retaining compatible session files; any backend/storage
change needs its own compatibility and migration rollback plan.

Next proposed task: M05 UX baseline and UX-01 session-switch consequence design,
followed by UX-02 actionable setup findings. Implementation remains unstarted.
Analysis verification: source inspection only; documentation validation results
are recorded in the active plan. No usability scores or measured improvements
are claimed by this assessment.
