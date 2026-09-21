# Archived plans

The UI redesign sub-plan below completed its own implementation scope and is
archived as such; the superseded VB-Cable-first rebaseline is retained as an
execution-history record, not as a completed milestone. Neither entry closes
the attended Windows accessibility/hardware/release gates tracked in the
active plan.

This directory stores dated execution records, not copies of the entire specification. Use filenames such as `2026-09-05-M00-feasibility.md` when a milestone actually finishes; the example date is not a claim that M00 is complete.

Each archived plan shall include original objective and scope, requirement IDs, implementation summary, decisions and superseded assumptions, test environment/commands/results, evidence/artifact links, unresolved known limitations, rollback/migration notes, and handoff. A plan abandoned or superseded before completion must say so explicitly and link to its replacement.

Keep an index below with date, milestone/task, outcome, and path. Do not move stable files from `docs/spec/` or `docs/milestones/`; other documents link to them. Never archive a blocked task as completed or use an archive entry as a substitute for missing Windows evidence.

## Index

| 2026-09-20 | VST plugin hosting completion | Implementation scope complete: shelf-reachable plugin add/remove/manage, canvas VST-vs-native visual identity, mixed physical/application multi-input mixing, isolated-worker health visibility, edge quick-insert, and a dispatch-blocking allow-list bug found and fixed during review. No attended Windows evidence; portable verification only. | [2026-09-20-vst-plugin-hosting.md](2026-09-20-vst-plugin-hosting.md) |
| 2026-09-19 | UI redesign (M05 visual editor) | Implementation scope complete: node cards, telemetry, EQ preview, four-sided connection handles, and three defect fixes (target-side persistence, handle overlap, clobbered safety notices). Attended Narrator/scaling/drag-drop verification is not closed and is tracked in the active plan. | [2026-09-19-ui-redesign.md](2026-09-19-ui-redesign.md) |
| 2026-09-17 | VB-Cable-first rebaseline | Superseded before completion; preserves prior execution history and records the driver/signing deferral. | [2026-09-17-vb-cable-rebaseline.md](2026-09-17-vb-cable-rebaseline.md) |

See the [active plan](../active/current.md) for current status and [future backlog](../future/README.md) for deferred scope.
