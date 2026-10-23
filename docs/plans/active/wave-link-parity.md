# Active plan — Wave Link parity, first round (2026-10-22, owner request)

Owner request (2026-10-22): record the Wave Link comparison as a future
entry, then implement the easiest missing features without breaking anything
existing; test each feature alone and combined with existing features, with
extra care for performance, stability and resiliency. Other agents work in the
same tree, so this plan lives in its own file and changes stay inside the files
listed below.

## Objective

Deliver the two smallest Wave Link gaps, both UI-only so the engine, backend,
API contracts and saved sessions are unchanged:

1. **Level target zone (UI-21)** — the Meter inspector shades a voice target
   zone (peaks from −18 to −6 dBFS) and shows a level verdict (waiting, too
   quiet, in the zone, loud, clipping) in a reserved slot.
2. **Hidden devices (UI-22)** — each active device in Status › Audio devices
   on this PC has a "Show in pickers" switch. Hidden devices leave the Input
   Device and Output Device pickers, except a node's own saved device, which
   always stays visible (lesson 2026-10-03, "a control must show the node's own
   saved value").

The full candidate list is in [Wave Link parity candidates](../future/wave-link-parity.md).

## Current task — Sound Check (#10)

Owner selected the next Wave Link parity task after merging the application
picker/groups (#4) to `main` in PR #59. Implement a clear, bounded Sound Check
flow using the existing temporary recording and Audio File playback paths.

- Requirements: UI-05, UI-17, REC-04, REC-08, REC-12, SEC-03, SEC-06 and
  SEC-09. Add UI-24 for the user-visible Sound Check behavior.
- Baseline: the Audio File inspector already records from an existing
  Recorder node on a running route, imports the completed WAV into 24-hour
  temporary media, and has Play/Pause/Stop transport. It currently permits
  120 seconds and gives a generic “plan and commit before playback” message.
- Decision: cap Sound Check at 10 seconds, retain explicit manual stop, use
  existing expiring media, and explain that the Audio File source must be
  planned/committed before playback. Audio File playback follows its ordinary
  downstream graph path, including effects. Require an already-routed
  Recorder node; do not open a microphone, use browser capture, or mutate a
  saved/running route implicitly. No new backend API or media format.
- Ordered tasks: (1) update this plan and UI-24/spec/candidate status; (2)
  revise Audio File inspector labels, 10-second timer and replay guidance;
  (3) add focused UI regression coverage for the Sound Check controls and
  timer; (4) fix the reported route-authoring defect: when a second source is
  connected to an already-fed single-input tool, insert a Mixer before that
  tool and preserve both sources instead of offering replacement only; (5)
  add draft and route-harness regression coverage for the mixer topology;
  (6) run UI format, lint, typecheck, relevant tests and production build;
  (7) inspect the complete diff and build the local shell in this isolated
  worktree for owner testing. No PR before explicit human approval.
- Validation matrix: UI component/unit coverage for the bounded timer,
  explicit stop, and playback guidance; UI lint/format/typecheck/tests/build;
  Edge dark/light/high-contrast visual review if layout changes; Windows
  desktop build. Live microphone playback is owner-run because it depends on
  the owner's device and endpoint preparation.
- Risks: a user may expect Sound Check to capture any microphone or play
  without committing. The control therefore says it only records from the
  selected Recorder branch and makes the plan/commit step clear. Existing
  recorder persistence means closing the UI does not stop recording (REC-04).
  The occupied-input Mixer insertion must preserve the prior edge's channel
  matrix and remain a draft-only topology change until the user saves it.
- Rollback: revert the scoped UI, spec, candidate list and plan changes; no
  schema migration or backend lifecycle change.
- Implementation (2026-10-22): the Audio File inspector calls the existing
  temporary-take workflow “Sound Check,” limits its auto-stop to 10 seconds,
  keeps explicit stop, and tells the user to plan/commit before playing the
  sample through downstream effects. The status area has a reserved fixed
  height. No backend, permission, persistence or route behavior changed.
- Validation (Windows 11, Node 22.22.3, Edge): UI format, format check,
  typecheck, lint (0 errors; 25 existing exhaustive-deps warnings), full UI
  suite (73 files, 640 tests), production UI build, and documentation
  acceptance (186 Markdown files, 1,029 links) passed. The focused UI test
  covers automatic stop at 10 seconds, temporary media import and replay
  guidance. The three Edge Sound Check theme cases each reported passed and
  screenshots were reviewed in dark, light and high-contrast. The Playwright
  process did not exit after reporting all cases passed and was interrupted;
  treat its command exit as incomplete, and rerun with the next browser
  qualification if needed. `git diff --check` passed after final updates.
- Owner local-test build: the release UI, shell, plugin worker and CLI were
  built from this worktree with output under `target/owner-local-test/release`.
  Shell path:
  `C:\code\audiorouter\.worktrees\wave-link-sound-check\target\owner-local-test\release\audiorouter-shell.exe`.
  This is an unsigned local build, not a release. No live microphone or native
  effect playback was tested.
- Limitation: this closes the 10-second recording and replay-guidance portion
  of Sound Check. A user still needs an existing Recorder branch and an
  Audio File source in the desired route, then must plan/commit before Play;
  full one-button parity remains open.
- Next action: the owner authorized committing and pushing this branch on
  2026-10-22. Push the feature branch, but do not open a PR until the owner
  explicitly approves it. The owner should validate the occupied-input Mixer
  behavior in the local build. After review, continue with Listen to any mix
  (#1 and #9); full one-button Sound Check parity remains open.

## Owner-reported route authoring defect — occupied processor input

The owner reports that connecting an Audio File to Advanced EQ when the EQ
already receives a microphone offers only input replacement. The expected
behavior is to insert a Mixer before the EQ and connect both existing and new
sources to it, matching AudioRouter's occupied-output Mixer insertion.

- Requirements: UI-03; preserve GRAPH-01/02 and UI-17 behavior.
- Ordered tasks: insert a draft Mixer into the occupied input edge, add the
  new source to that Mixer, preserve the existing downstream edge matrix,
  select the Mixer and explain the draft action; add unit and Edge regression
  coverage for microphone + Audio File -> Mixer -> Advanced EQ; format, lint,
  run focused and full UI checks, and rebuild locally.
- Validation: verify exact draft topology, matrix preservation, undo, route
  planning/commit on the harness, and the owner's local live playback.
- Rollback: revert the shared occupied-input routing helper, tests and UI-03
  clarification together. No backend or session schema change.
- Implementation: `appendDraftConnection` rejects a second source on a regular
  occupied input. The canvas already converted this collision into a Mixer
  only for Physical Output; other inputs offered replacement. The handler now
  inserts a draft Mixer before the occupied destination and adds the new
  source to it. The original edge's enabled state and downstream channel map
  are preserved. Added unit coverage for microphone + Audio File and updated
  the Edge three-theme occupied-input scenario to assert topology and Undo.
- Validation (Windows 11, Node 22.22.3, Edge): the focused draft suite passes
  (21 tests); full UI suite passes (73 files, 640 tests); typecheck, lint (0
  errors; 25 existing warnings), format and docs acceptance pass. All 6
  `banner-layout.pw.ts` Edge cases reported passed, including all three themes,
  but the Playwright process remained alive after the final pass and was
  interrupted (command exit 1); treat browser command completion as
  incomplete. Production UI build passes. Native release shell, plugin worker
  and CLI builds pass in `target/owner-local-test/release` after the owner
  closed the existing shell; the first rebuild attempt while it was running
  failed with Windows access denied on the locked executable. Live audio was
  not exercised by automation; owner should verify microphone + Audio File
  through the inserted Mixer and Advanced EQ.

## Decisions

- **UI-only first (historical).** The owner later selected Sound Check (#10).
  This bounded UI workflow reuses existing recording/playback APIs; a
  temporary monitor/capture path for full one-button parity remains future.
- **No Stream Deck fades.** Every `nodes.set` is a saved plan/commit revision,
  so a client-side fade would add many revisions per press and delay the audio
  pump (lesson 2026-09-26, "Native audio must be pumped by the backend").
- **Hidden devices are a per-PC view preference**, stored in `localStorage`
  (`audiorouter.ui.hidden-devices`) like theme and layout. They never change
  graph data, saved bindings, Windows or the backend. Bounded to 256 IDs of
  at most 512 characters each; corrupt or unavailable storage means "nothing
  hidden". Endpoint IDs change on driver updates (lesson 2026-10-12); a
  changed ID only makes the device visible again, which is the safe direction.
- **Target zone uses the held peak**, which already has display ballistics, so
  the verdict does not flicker at 20 Hz. The verdict line has a fixed height
  and swaps only its text (UI-17).
- Advanced Devices-tab endpoint panels keep listing every endpoint (diagnostic
  surfaces).

## Tasks

1. [x] Future entry `docs/plans/future/wave-link-parity.md` and a row in the
   future README.
2. [x] UI-21 and UI-22 in `docs/spec/09-interface.md`.
3. [x] `ui/src/hiddenDevices.ts`: bounded storage plus a small external store
   (`useSyncExternalStore`), unit tests for corrupt, oversized and throwing
   storage.
4. [x] Pickers (`PhysicalInputBinding`, `PhysicalOutputBinding`) filter hidden
   devices, always keep the bound device, and show a disabled "N hidden" option
   (no layout change). Component tests, including "all hidden" and "bound
   device hidden".
5. [x] `ThisPcDevices` gets the switch per device; component tests.
6. [x] `MeterInspector` target zone and verdict, pure `levelVerdict()` with
   unit tests; component tests.
7. [x] Combination checks: the existing device-binding, meter, input-channels,
   spatial (surround loopback picker) and workbench e2e suites, plus a new e2e
   for both features in three themes, including a layout-shift sample.
8. [x] Quickstart user documentation.
9. [x] Full UI checks: typecheck, ESLint, Prettier, the complete vitest suite,
   and the relevant Playwright files on the route harness (no cargo build).

## Validation matrix

| Check | Covers | Environment |
| --- | --- | --- |
| `npm --prefix ui test` (full suite) | No regression in existing UI behavior | Windows PC, Node 22 |
| New unit and component tests | Storage bounds and failures, filter rules, verdict thresholds | jsdom |
| Playwright `device-binding`, `meter-inspector`, `input-channels`, `spatial-audio`, `workbench-organization`, new `wave-link-parity` | Pickers, meter, surround picker, Status tab combined with the new features; three themes; no layout shift | Edge, route harness (no backend) |
| typecheck, lint, format:check | CI gates | Windows PC |

Not covered: the real desktop shell and live audio. Neither feature reaches
the backend, so a live audio check adds no evidence. The owner can check the
features in the next build.

## Risks and rollback

- Risk: a hidden device makes a user think it vanished. Mitigation: the picker
  says how many are hidden and where to show them; a node's own device is never
  hidden.
- Risk: the meter verdict suits voice, not music. Mitigation: it is labelled
  as a voice guide.
- Rollback: revert the files listed above. No data migration;
  the stale `audiorouter.ui.hidden-devices` key is ignored.

## Evidence

The work was first written and checked against a local `main` that was 29
commits behind `origin/main` (the canvas alignment work in PR #51 was
missing). It was then moved onto `origin/main` (`2228aa32`) on branch
`feature/wave-link-parity`, in its own worktree, and every check below was
re-run there on 2026-10-22: the owner's PC, Windows 11, Node 22.22.3, Edge, with
a clean `npm ci` from the committed lockfiles.

- `npm run typecheck:all` (TypeScript 7.0.2): clean.
- `npm run lint`: 0 errors. The 25 warnings are existing `exhaustive-deps`
  warnings in other files; the changed files lint clean.
- `npm run format:check`: clean.
- `npm test`: 72 files, 630 tests passed. 22 are new or extended: storage
  bounds, corrupt/throwing storage, write failure, picker filtering,
  bound-device retention, all hidden, surround group, live switch from Status,
  verdict thresholds and band position. A deliberate break of the bound-device
  rule and of the too-loud threshold made 3 tests fail each, so the tests detect
  those regressions. The new e2e file uses the guarded `./browser-test` fixture
  (`verificationWiring.test.ts`).
- `npm run e2e` (all three stages, `e2e_backend` built from the branch with
  `cargo build -j 4`): browser guard qualified; development suite 324 passed,
  9 skipped, 0 failed; production canvas suite 76 passed. One
  `ResizeObserver loop completed` log line came from the existing
  `inspector-stability` and `shortcut-after-save` tests, not these features.
- `e2e/wave-link-parity.pw.ts` (7 passed, three themes): the Meter showed
  in-zone, loud and too-loud verdicts during simulated talk; the meter below kept
  one offset and the verdict line one height for 6 s (UI-17). Hiding a device
  left the device list's height unchanged and the picker showed "1 device
  hidden"; the node's own hidden device stayed selected and listed; the hidden
  list survived a reload; corrupt storage hid nothing. Screenshots from the
  first pass were reviewed in dark, light and high-contrast.
- `tests/acceptance/docs.ps1`: passed (186 files, 1027 links).

Observations, not changed here (existing behavior):

- Every checkbox switch in the app renders about 41×38 px (nearly round), not
  the 2.55×1.4 rem pill defined in `styles.css`. The global field rule
  `input { min-height: 2.35rem }` also applies to checkboxes. The new switch
  matches the others. A one-line `min-height: 0` on `input[type="checkbox"]`
  would fix them all, but it changes every switch and needs its own visual pass.
- Workspace notifications float over the bottom of the sidebar. The last
  device in Status can sit under a toast, so a pointer click there is
  intercepted until the toast closes; the keyboard works. The e2e test uses
  the keyboard.

## Next action

Owner review of the two features in the next build. Then pick the next
candidate from the [Wave Link parity list](../future/wave-link-parity.md).
Recommended: the not-running app picker (#4), then Listen here (#1).
