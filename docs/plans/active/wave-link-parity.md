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

## Decisions

- **UI-only first.** Listen-to-mix, Sound Check and app groups need backend
  or engine work. They stay future until the owner picks one.
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
