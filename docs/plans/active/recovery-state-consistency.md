# Backend recovery-state consistency

## Objective and authority

2026-11-27: owner requested the next non-driver work after frontend/build
qualification. Investigate and repair the five recorded M07 recovery cases
A5-A9. The owner explicitly approved all five recommended repairs on 2026-11-27.
The decisions below supersede the pinned current-behavior tests.
Existing RES-3 permission for bounded automatic recovery of the exact saved
endpoint remains unchanged. No driver work, PR, release, live profile/device
changes, attended keyboard test or eight-hour soak is authorized here.

## Requirements and prerequisites

RES-3, STATE-03/09/10/11, CAP-11, API-07, AUTO-18, ENG-01/02/04.
Read M07, persistence/capture requirements, the November 13 resilience decision
and Q4.2 application-quality findings. Windows and pinned Rust 1.96.0 are
available. Fault-interleaving fixtures exercise Windows control logic with
simulated worker state; they do not establish physical unplug/sleep recovery.

Branch `fix/recovery-state-consistency` starts from main `2f24ad436`, independently
of the prior frontend branch. Preserve untracked `target-preview/`,
`target-preview2/` and `ui/tsconfig.tsbuildinfo`.

## Approved decisions (2026-11-27)

1. A5: a disqualified device recovery stops the stale runtime, finalizes any
   recorder safely and publishes a reason; it never starts the edited/faulted route.
2. A6: Start on a still-lost device route returns `nodeNotReady`; deliberate Stop
   then Play is required, with no successful silent no-op.
3. A7: repeated Sleep retains the union of eligible stopped-route resume lists,
   without silently starting portable routes.
4. A8: failed-plugin routes are excluded consistently from resume/recovery offers.
5. A9: one armed-or-recording predicate prevents sleep/recovery from re-arming a
   recorder. Preserve recorder finalization and microphone privacy.

## Ordered tasks

1. Reproduce the pinned behavior with fault-interleaving tests; inspect dispatcher,
   cancellation, recorder and event paths before changing expectations.
2. Record the owner's decisions here and in the applicable specifications before
   implementation; write failing dispatcher regressions for those choices.
3. Fix the owning backend layer in small checked steps. No adapter-only workaround.
4. Exercise REST dispatcher/default MCP entry points for changed public behavior;
   check drift and regenerate references if public contracts change.
5. Run focused and broader control/transport/CLI tests, both Rust formatting checks
   and relevant strict Clippy. Inspect the complete diff and update evidence.

## Validation matrix

- A5: revision edit, recorder start and failed plugin cancel stale recovery;
  stopped state, witness/event, recorder finalization and saved graph verified.
- A6: real dispatcher and default MCP Play cannot report success for silent audio.
- A7: two sleeps, explicit Stop/Start and delayed duplicate notification; retain
  only eligible routes, remain stopped, no duplicate worker ownership.
- A8: both native and portable plugin faults excluded without weakening the latch.
- A9: armed, recording, paused/stopping recorder cases share the safety predicate.
- Privacy/idempotency: existing fault scheduler, API-07 and cancellation suites.
- Hardware: no actual unplug, sleep/resume, audible continuity or long soak claim.

## Risks and rollback

Stopping a disqualified recovery can finalize a recording and change runtime
generation; old idempotency keys and session-scoped events must remain honest.
Retaining resume lists must not resurrect a deliberate Stop or failed plugin.
Rollback only this branch's commits; preserve saved graphs, recording files and
existing recovery policy. Do not roll back microphone fail-closed protections.

## Evidence and next action

Implementation and local verification are complete. See the
[evidence](evidence/2026-11-27-recovery-state-consistency.md) for regression
proof, command outcomes and physical-hardware limits. Next: short attended
exact-endpoint loss/resume qualification in a quiet device window; no long soak.

2026-11-27 baseline: Windows, Rust 1.96.0. `cargo test -p audiorouter-control
--lib fault_interleaving_tests --locked` passed all 29 tests (`baseline.log`),
including the five pinned current-behavior cases. `cargo test -p audiorouter-cli
--lib every_api_method_is_reached_by_a_default_mcp_tool_or_excluded --locked`
passed its reachability test (`mcp-baseline.log`). Contract drift passed for
156 methods, 38 node kinds, 20 processors and 24 event categories. Documentation
validation and diff whitespace checks passed. These baseline checks preceded
the owner's explicit approval; implementation then followed all five decisions.
