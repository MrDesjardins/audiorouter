# Issue 149 supplied-session verification

## Objective and scope

Verify issue #149 against latest main (`8f42bf8de`), using the supplied
Session.audiorouter and backend.jsonl read-only. Requirements: GRAPH-08/09,
UI-07, STATE-02/05/06, PLUG-01/03/04; M06/M07. Source files remain outside
the repository because they contain personal device identities and plugin state.

## Prerequisites and decisions

- Windows development machine; preserve existing untracked build directories.
- Latest origin/main fetched 2026-11-16; investigation branch
  `fix/reported-autosave-vst-2026-11-16`.
- Logs contain 3,937 records from 0.0.28 and 1,058 from 0.0.29, including
  64 graph.plan revision conflicts, 11 editor-open errors and 26 invalid-parameter
  nativePaths.prepare errors. Error messages/arguments are not logged, so these
  categories alone do not establish their precise cause.
- Session revision 413 contains Clear (VST3), ERA6 VoiceAutoEQ (VST3), ERA6
  DeEsser (VST2), Mute and Fun Voice Changer, with a two-output fan-out.
- Reporter plugin binaries are absent at their saved paths. Exact live
  plugin/hardware continuity cannot be claimed from substitute fixtures.
- Read issue body and comments through GitHub CLI. PR #150 addresses both
  interruption and edits made during saves; verify the latter at commit time
  as well as the already-covered planning-time race.

## Ordered tasks and validation

1. Inspect archive/session and correlate logs with recent repairs.
2. Exercise existing backend reuse/classification and UI autosave regressions.
3. Add focused regression for rapid edits while commit is in flight; fix if it
   demonstrates a defect. Validate supplied bundle through the real dispatcher
   on disposable in-memory storage where feasible.
4. Record exact commands/results, inspect diff, and report remaining live gates.

## Risks and rollback

Never import into the owner's live database or bind their microphone. Synthetic
tests establish behavior only within their stated scope. Revert investigation
changes if necessary; no saved-data migration or external publication is planned.

## Evidence and next action

Initial evidence: supplied files and issue #149; PR #150 archived live parameter
autosave plan and PR #152 VST experience work. Next: execute targeted regressions.

## Verification results (2026-11-16, Windows)

- `cargo test -p audiorouter-control --lib`: 442 passed, 22 opt-in tests
  ignored, zero failures. Includes dispatcher, plugin state, revision and
  audio-publication coverage.
- Focused `reuse` and `live_edit_classification` filters: 7 and 1 passed.
  Unchanged bridges do not start a worker; slider changes wait for publication.
- `npm --prefix ui test -- src/App.lifecycle.test.tsx
  src/LiveAutosaveSetting.test.tsx`: 26 passed. Expanded the existing regression
  into planning-time and commit-time races. Both pass without production changes:
  an edit during commit is retained and saved by a second serial commit.
- UI format, typecheck and lint passed (zero warnings). Complete diff inspected;
  no production behavior, API, controls or styles changed.
- Built and ran current CLI `import-bundle` into the disposable
  `%TEMP%/audiorouter-issue149-verify.sqlite`, using existing staging directory
  `%TEMP%/audiorouter-issue149-staging`: supplied bundle accepted, all 10 nodes,
  9 edges and 3 plugin states retained at revision 413. Initial invocations
  corrected the binary name and created the required staging root.
- Attempted exact `node set` operations for Mute and shiftEnabled on that copy.
  After adding the required idempotency keys, all were explicitly refused:
  `plugin placeholder requires a current explicit scan result`. This is the
  existing unscanned-plugin guard, not evidence of an autosave regression.
  No microphone, endpoint or live owner database was opened.

## Assessment and remaining gate

No remaining issue-149 defect reproduced in available tests. Main contains
PR #150's change-aware bridge reuse, continued pumping during preparation,
coalesced autosaves and Live autosave switch. Historical error records omit
request arguments and detailed messages; they cannot prove that every editor
or nativePaths error has the same cause or is now fixed.

Exact next task: on the reporter's machine with current main/0.0.30 code,
scan/resolve the three original plugins, load a disposable copy of this session,
play and record both outputs while rapidly toggling Mute and Fun Voice Changer,
including edits during a save. Confirm no gap, no new worker for unchanged
plugins, no revision-conflict lock, and editor open/close while stopped and
playing. This gate remains unrun here because those binaries and endpoints
are unavailable. The owner subsequently authorized adding this verification
to existing PR #160 and merging latest main into that branch. The verification
was committed as `c3302b0af` and cherry-picked as `f01ef57b6`; main `8f42bf8de`
was merged, preserving the consolidated plan and its new shortcut decision.
No release was created.

## Portable coverage re-check (2026-11-24)

Issue #149 is closed (`CLOSED`, 2026-11-20). Re-read the case the manager
named: toggling Mute and changing the Fun Voice Changer while an automatic
save is in flight. The portable parts are now covered; the exact-plugin live
gate is unchanged and belongs to the reporter/owner.

| Case | Portable evidence | What it proves |
| --- | --- | --- |
| Edit arrives while a save is planning or committing | `coalesces edits during automatic save planning/committing`, [App.lifecycle.test.tsx:532](../../../ui/src/App.lifecycle.test.tsx) | The second edit does not start a concurrent save, produces no "edits changed during review" warning, and the final planned value is the later one. |
| Mute toggle + Fun Voice Changer change, then a save from the new revision | `a_second_save_based_on_the_first_saves_revision_commits_without_a_conflict`, [graph_tests.rs:850](../../../crates/control/src/graph_tests.rs) | Tone → Mute → Fun Voice Changer → Meter started through `ControlPlane`; `graph.plan` and `graph.commit` go through `ControlPlane::dispatch` as the window sends them. Save 1 at revision 0 with `muted` and a preset commits at revision 1; save 2 built from revision 1 commits at revision 2; a plan still built from revision 0 is refused with `revisionConflict`. Both edits are saved and the first classifies as `ParametersOnly` with sources retained. This is two serial saves, not an overlap. |
| Flag and parameter edits stay live | `live_edit_classification_distinguishes_parameters_flags_and_topology`, [graph_tests.rs:806](../../../crates/control/src/graph_tests.rs) | A bypass flag, a parameter and both together are live edits; a rename, rewire or dropped edge is structural. |
| No new worker for an unchanged plugin | `merged_preparation_reuses_worker_and_defers_slider_changes_until_publication` [plugins.rs:1232](../../../crates/control/src/plugins.rs), `only_requested_plugin_groups_start_and_fully_reused_groups_are_skipped` [plugins.rs:1532](../../../crates/control/src/plugins.rs), `plugin_reuse_accepts_slider_and_flag_changes_but_not_identity_or_state` [plugins.rs:1448](../../../crates/control/src/plugins.rs) | A reused bridge is not reloaded and starts no worker; only a changed identity or saved state restarts a plugin. These use the in-repo `ReuseProbe` test double, not a vendor binary. |

The new test is portable (no audio device, no plugin process, in-memory
storage). It sends the real `graph.plan` and `graph.commit` requests through
`ControlPlane::dispatch`, not a hand-built stage. The backend serves one
request at a time, so it proves the serial fallback the window uses when a save
is still in flight — not two saves actually overlapping; that overlap is the
App regression above. The stale-revision plan makes the case able to fail. It
does not stand in for the exact Clear/ERA6/Fun Voice Changer binaries:
`plugin_parameter_events`, `plugin_identity_matches` and the reuse suite are
what prove worker reuse, and the reporter's licensed VST2/VST3 binaries are
still absent.

The exact live gate below is unchanged and remains with the reporter/owner.
Nothing here authorizes a release or changes the saved-data format.
