# UX onboarding and recovery — 2026-11-02

Current status: guidance source integrated as `c920fed7` in parent `bf99cf97`.
The gaming-time restrictions below record the completed child assignment;
parent final qualification follows [current.md](current.md). UX-04 remains a
prototype, and UX-06 measurements and attended M05/M08 acceptance remain open.

## Objective and requirements

Implement an additive UX-04 guided setup prototype and UX-06 recovery wording.
Requirements: UI-01/06/08/09/12/13/17/26, UC-01/02/03/06/08/09,
NFR-14; M05 and M08 attended gates remain open.

## Prerequisites and decisions

Owner is gaming with the current app. Source-only isolated worktree work;
never contact backend, profiles, devices, native app, audio, or shared outputs.
Read parent UX proposal at C:/code/audiorouter/docs/plans/future/ux-improvement-2026-11-02.md.
Reuse existing Discord draft action and Tools workflow. Existing template
builders are test helpers without a UI action; do not invent a second graph
commit path or replace the owner's draft. Preserve canvas and navigation.
No API/schema changes. Availability is not evidence of signal or app success.

## Ordered work / verification

1. Add goal-led guidance to existing first-run panel, using existing callbacks.
2. Show exact paired cable names and independent verification instructions in
   Discord setup; correct claims that presence proves audio reception.
3. Clarify Retry versus Stop/Play for voice protection.
4. Run prompt typecheck, focused mock tests with one worker, format and lint.
5. Inspect diff and commit only this slice.

## Validation matrix and deferred evidence

Mock components: goal transitions, callbacks, absent cable, exact names,
no false success, explicit monitor/recorder safety, Retry request semantics.
Deferred: packaged app, real signal, app closed/silent source, mic monitoring,
recording state, dark/light/high contrast, 1280x720/200%, Narrator, five fresh
participants (four finish in ten minutes; all identify Discord sources).
These require an attended session with disposable profile and supported hardware.

## Bounded attended measurement checklist (do not run while gaming)

Declare build/hash, OS, CPU/RAM, endpoint formats, plugin licences, route and
telemetry workload. Use a disposable profile, headphones and explicit consent.
Record five cold and five warm launches from invocation to usable canvas;
retain individual durations, median and maximum against NFR-14 (8s cold/3s
warm). Measure ten panel switches during fixed telemetry, noting response
times and stale-state visibility. Separately observe actual audio continuity
and redacted backend errors; UI timing does not prove audio correctness.
One attended unplug/replug, sleep/resume and plugin fault each: record affected
node, silence, recovery action and result; no substituted mic or auto-resume.
Stop at unexpected audio, recording, device change or loss of owner control.

## Risks, rollback, next action

Prototype usability is unproven. Revert this slice to restore prior guidance;
no storage migration or backend change. Parent must integrate and perform
attended tests before marking UX-04/06 or M05/M08 complete.

## Results

Implemented an additive collapsed goal guide in FirstRunGuide, exact cable-end
explanation in DiscordSetup, independent signal/app checks, and Retry/reload
wording in PluginPanels. No template application API was added: voice goals
reuse the existing guided draft route; app recording opens Tools with explicit
steps. This is a prototype, not completed participant-qualified onboarding.

Windows worktree verification, 2026-11-02:

- Initial npm.ps1 invocation blocked by PowerShell policy; used npm.cmd.
- Initial typecheck/format/lint could not start without local dependencies.
  Created an ignored local ui/node_modules junction to existing installed tools
  at C:/code/audiorouter/ui/node_modules. No install or shared output requested.
- `npm.cmd --prefix ui run typecheck` and `typecheck:all`: passed.
- `npm.cmd --prefix ui run format`: passed; only slice files changed.
- `npm.cmd --prefix ui run lint`: passed, zero warnings.
- `npm.cmd --prefix ui run test -- src/GoalSetupGuide.test.tsx src/FirstRunGuide.test.tsx src/DiscordSetup.test.tsx src/PluginNodeInspector.test.tsx --maxWorkers=1 --no-file-parallelism --no-cache`:
  4 files / 13 tests passed, 8.98 seconds, maximum one concurrent worker.
- `git diff --check`: passed. Complete slice diff inspected before commit.

No native build, backend access, audio, benchmark, packaged app or browser test
was performed. No API methods, node controls, persistence or sidecars changed.
Next: parent executes the deferred attended M05 UX-04/06 checklist; source
integration is complete. Do not infer participant success from the mock tests.
