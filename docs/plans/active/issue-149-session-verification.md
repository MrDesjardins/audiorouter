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
