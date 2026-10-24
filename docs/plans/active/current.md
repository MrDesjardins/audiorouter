# Active plan — post-0.0.8 maintenance

## Release 0.0.22 — 2026-10-24, owner request

The owner merged #67 (Discord and OBS, `dfc17cea`) and asked for a new
release with it. Same app-only unsigned prerelease profile as 0.0.21
(NFR-06, DIST-01–08); no storage migration, no session format change since
`v0.0.21`, so 0.0.21 stays the rollback.

Tasks: (1) release prep PR `release/0.0.22`: version bump by package name
(21 files, only `audiorouter-*` lock entries), 0.0.22 notes, the Apps page
version note, the background-inventory follow-up in
[OBS and Discord future work](../future/obs-discord-integration.md#follow-up-build-the-application-inventory-off-the-control-thread);
owner approves and merges. (2) Annotated tag `v0.0.22` on main; build in a
clean worktree with `create-draft-release.ps1 -Tag v0.0.22 -AppOnly`
(owner asked first: heavy build). (3) `fresh_install_shell` on the exact
packaged shell, download the draft and `verify-artifacts.ps1`, publish as
a prerelease, record evidence in `evidence/2026-10-24-release-0.0.22.md`.

CI notice kept on purpose: GitHub annotates every Linux job with "The
ubuntu-latest label will migrate to Ubuntu 26 beginning October 19, 2026".
Those jobs only run Node scripts on pinned `setup-node` versions, so
following the label is the intended behavior; pinning `ubuntu-24.04` would
only postpone the same move. Revisit if a Linux job fails after the move.

Two flakes fixed on the release branch (owner reports, 2026-10-24):

- **Edge, PR #68 run 40121219491:** "new connections keep both chosen
  sides when older geometry is undone" left the remove button present for
  10 s. A debug trace of a local failure (about 1 in 40–200 runs at 4
  workers) showed the test clicking Arrange about 70 ms before the session
  was adopted. The adoption reset the layout and refitted the view, so the
  button and handles moved under the pointer. The node-count fit also
  animated for 180 ms under reduced motion, unlike the other two fits; it now
  honours `prefers-reduced-motion`. The test waits for `canvas-readiness`
  (session adopted, geometry measured) and a still viewport before each
  interaction. Result: 500/500 runs at 4 workers with no retries, the three
  canvas suites (29 tests) pass, and lint, typecheck and 721 unit tests pass.
- **Coverage nightly run 40076776528 (main):**
  `sender_clock_drift_is_absorbed_without_gaps_or_growing_delay` counted 4
  steps with the 1.003 sender. It had one underrun, which must be mid-run:
  that sender finishes after the receiver stops reading, so there is no
  final drain. The instrumented runner held the sending thread back for most
  of the 40 ms buffer, and the gap that followed was correct behavior. The
  feeder now reports its worst lateness. An attempt where the sender ran
  more than 25 ms late is not judged and is repeated, up to 3 attempts; an
  on-time attempt is judged as strictly as before. Locally the sender runs
  about 1 ms late and the 13 network-audio tests pass.

## OBS and Discord premium experience — 2026-10-23, owner request

The owner authorized stages D1, O1, D2 and O2 of [OBS and Discord setup
help](../future/obs-discord-integration.md) (DEC-22) and asked for a UX plan
first. Tracked in its own plan: [OBS and Discord](obs-discord.md). X1
(Discord RPC) needs Discord's approval and is not authorized.

Review repairs authorized and implemented 2026-10-23: collection/UUID-bound
OBS writes, effective voice-path checks, correct OBS default-device roles,
and activity/track-aware duplication checks (SEC-15, UI-26, GRAPH-05/17).
36 focused UI tests, scoped lint/format, Rust formatting and docs checks pass.
Rust compile/tests, contracts drift and Edge remain on the owner's hold;
see the [repair evidence and exact next checks](obs-discord.md).

Before-manual-test follow-up: fresh OBS inspections, stable/validated repair
choices, visible Restore errors and stale-state handling; fixed conditional
Apps hooks that could crash startup. Focused UI/card/hook tests pass 55/55;
the delayed-startup App regression passes separately and fails with the old
hook placement. The OBS note's four-line layout still needs Edge theme QA.

Coverage audit: fixed another observed source-replacement/dialog identity
case after its regression failed. All six relevant UI/lifecycle files now
pass (84 tests). The integration plan records missing backend failure-path
tests as well as the compiled/native/browser checks still on hold; this is
not exhaustive verification or a closed O1/O2 gate.

Owner approved another review and opening a PR while still playing:
[draft #67](https://github.com/MrDesjardins/audiorouter/pull/67), branch
`docs/obs-discord-integration`, code commit `5de6565f`. Final review adds
worker shutdown, bounded file reads/connection queues, parallel-write
guards and backend failure-path tests. 86 focused UI tests and local
format/lint/docs/site checks pass. CI is running remotely; its current head
and results are on the PR. Keep it unmerged for the owner's manual test.

## Review follow-ups R1–R7 and a second review — 2026-10-23, owner request

The owner asked to improve the `site/compare.html` wording, fix every
remaining 0.0.21 review item, confirm the nine findings are fixed with no
new issues, then review again and fix what that finds. Branch
`fix/review-followups-0.0.21` from main `534d243e`. A PR needs the owner's
approval (AGENTS.md); this request did not include one.

- Outcomes per item: [review follow-ups](../future/review-fixes-0.0.21.md)
  (R1/R2 per-side switch gates, R3 not a defect plus a regression, R4 dead
  code removed, R5 one dialog helper, R6 wording, R7 one product bug and
  three test races fixed, CI retries once).
- Verification of the five 0.0.21 fixes: their regressions still pass; the
  switch-into-Mixer refusal is replaced by R1 support, and both new switch
  tests fail with the side gate disabled (run 2026-10-23, then restored).
- Second review: `/code-review high v0.0.20..HEAD` (after the R1–R7
  commits), 10 findings, all checked against the code and fixed:
  1. With the audio thread pumping, `nativeMultiInputs.pump` returned
     hard-coded zero counters, so the status line could not tell a healthy
     route from a stalled one. It now reports the thread's totals since Play
     (`PumpTotals`), and the late-gap warning reads the audio thread's gaps
     (`audioThread`, now in the API output schema and contracts; the schema
     had `additionalProperties: false` while the backend already sent it).
  2. VST3 worker: plugin-window edits were added to host events with no
     combined limit, so a quantum could exceed 256 parameters and stop the
     worker. Edits now share the limit; the rest wait a quantum, and a host
     event for the same parameter wins.
  3. Capture-sink driver leases were renewed only between control requests
     while the audio thread kept pumping. With such a lease the audio thread
     now pumps only within half a lease of the last successful checks.
  4. The App group picker kept the previous group's selection after Create.
  5. A switch side was faded twice (gain squared); the sink now only
     silences a fully closed side.
  6. The executable placeholder showed `C:\\Games\\game.exe`.
  7. Switch gates did per-sample work in steady state; now a no-op at gain 1
     and a clear at gain 0.
  8. Hidden devices used their own storage helper with no size bound; now
     the shared presentation helpers.
  9. Output Switch checks and side chains were copied in both compilers; now
     `output_switch_children` and `OutputProcessorChain::output_switch_side`.
     `domain::validate_session` keeps its own shape check (the backend's
     authority on saved sessions).
  10. Copying a copy nested IDs (`-copy-1-copy-1`) with no 128-byte bound;
      it now numbers from the original and truncates to fit.
- Checks after both rounds (2026-10-23, Windows 11, Rust 1.96.0): fmt and
  strict Clippy clean (workspace, shell); `cargo test --workspace --locked`
  (exit 0, 1,160 passed, 0 failed); UI format, lint (0 errors, 25 warnings),
  typecheck, 648 unit tests; contracts typecheck and drift; docs and site
  checks; Edge: compact-workspace, canvas-layout(-review), session-workflows,
  device-access-consent, drag-place, canvas copy/paste, clipboard,
  application-sources, device-binding (60 tests), the two de-flaked tests
  20/20 repeats; VST3 worker rebuilt (release) and
  `verified_native_vst3_worker_processes_an_opt_in_fixture` passed with
  mda-vst3. New regressions were run against the unfixed code for R1/R2
  and the App group picker (each failed, then passed with the fix).
- Not done: live audio on hardware (sine-continuity harness) for the switch
  gate and audio-thread changes; a UADx editor session with many edits.
- Rollback: revert the branch's commits; no storage migration. The
  contracts gained an optional `audioThread` field.
- PR #65 (owner-approved). CI's dependency policy then failed on
  GHSA-mrhx-c7xc-m3w5 (high: zod < 4.6.6 can exhaust memory validating
  oversized collections), reached through `@elgato/utils` 0.6.1 in
  `tools/streamdeck`, which still requires zod ^3. No zod 3 fix exists, and
  `npm audit fix --force` would downgrade `@elgato/streamdeck` to 1.4.1.
  Decision: an npm `overrides` pin of zod 4.6.7. The library imports only
  `zod/v4-mini`, which zod 4 still exports. Checks: `npm-audit` passes; the
  Stream Deck typecheck, its 26 tests and the build pass; Elgato's
  JSON-RPC request/response schemas accept valid messages and reject bad
  ones under zod 4.6.7. Remove the override once `@elgato/utils` depends on
  a patched zod.

## Overnight review, PR, merge and release 0.0.21 — 2026-10-23, owner request

The owner asked, before going to sleep, for: (1) a quick code review of all
changes since 0.0.20, (2) fixes for high-priority findings and the rest
recorded as future fixes, (3) one PR with all the work, (4) merge it when
ready, (5) a new release with every fix and feature. This message is the
explicit PR, merge and release approval (AGENTS.md "Pull request approval").

- Scope: branch `feature/vst3-plugin-windows` = plugin windows (f1eb76f2)
  + `feature/output-switch` merged locally (1015b5be) + review fixes
  (50aeaed9) + the 0.0.21 bump. Everything else since 0.0.20 is already on
  main (#58, #59, #60 and earlier).
- Review: `/code-review medium v0.0.20..HEAD`, 9 findings. Fixed: plugin
  sliders resent every quantum (editor knobs reset), Output Switch side into
  a shared Mixer (leak or wrong mute; now refused), Output Switch routes on
  the single-endpoint worker, latency query ending the worker, two misplaced
  doc comments. Open: [review follow-ups](../future/review-fixes-0.0.21.md)
  R1–R6.
- Checks on 50aeaed9: fmt and Clippy (workspace, shell) clean; plugin-host,
  engine and control tests pass (plugin-host lib 78, engine lib 161, control
  lib 264); UI format, typecheck, lint (0 errors, 25 warnings), 646 tests;
  contracts typecheck and drift; docs validation (191 files, 1056 links).
  The new switch-into-Mixer regression was not run against the old code
  (the guard-disabled run was blocked), so it is not yet proven to fail
  without the fix.
- Not changed: `site/compare.html` VST2 wording (R6, blocked by the
  permission check; owner decision).
- Merged: #61 `69ea5992` (squash), then #62 (`ctutils` advisory published
  overnight) and #63 (Edge test budget, flakes recorded as R7).
- Released: [v0.0.21](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.21)
  on `90b6ab2e`, published 2026-10-23 11:52 UTC as a prerelease
  ([evidence](evidence/2026-10-23-release-0.0.21.md)). Keep 0.0.20 for
  rollback.
- Next: the owner installs 0.0.21 and plays a route with a plugin window;
  then picks a review follow-up (R1–R7) or decides the VST2 site wording
  (R6).

## UAD plugin plan and bring-your-own-plugin guide — 2026-10-22, owner request

Objective: plan support for every UAD product family and give users one
guide for installing, scanning, adding and troubleshooting their own VST3,
VST2 and UAD plugins. Started as documentation only; U0 code followed (below).

- Requirements: PLUG-01–PLUG-07 (no change), DSP-01, UI-17.
- Decisions: the plan lives in `docs/plans/future/uad-plugin-support.md`
  (not authorized for implementation); proposed PLUG-01 (instruments) and
  PLUG-05 (latency change) changes are listed there as owner decisions. UAD
  facts not observed on a real install are marked "confirm in U1"; the user
  guide calls UAD-2 unqualified and every UA title untested until listed in
  the compatibility snapshot.
- Code facts recorded as gaps (read 2026-10-22): the VST3 worker takes no
  class ID and loads the first initializing audio-effect class; UI plugin
  nodes are mono; no `setBusArrangements`, no `getLatencySamples`, no VST3
  editor; no VST2 shell enumeration.
- Files: `docs/operations/bring-your-own-plugins.md` (new),
  `docs/plans/future/uad-plugin-support.md` (new), links from
  `docs/README.md`, `docs/plans/future/README.md` and
  `docs/operations/plugin-compatibility.md`.
- Validation: `tests/acceptance/docs.ps1` (links, anchors, fences) and
  `git diff --check`.
- Rollback: delete the two new files and the links.
- U1 first run (2026-10-22, owner installed UA Connect's nine free UADx
  titles): all load, process and save state offline; but the app cannot
  start any VST3 plugin (`VST2 load failed: MissingEntryPoint`; the native
  VST3 worker is not packaged or selected) and UADx stdout output corrupts
  the native worker's protocol pipe. See
  [UADx inventory evidence](evidence/2026-10-22-uadx-inventory.md); the
  guide was corrected to say VST3 cannot play yet.
- U0 implemented (2026-10-22, owner: "Do not change the website wording:
  MAKE IT WORK. I want VST3 VST2 AND UAD to work"). Site wording unchanged.
  - Control selects the worker by format (`PluginWorkerKind` in
    `crates/control/src/plugins.rs`): `audiorouter-vst3-worker.exe`
    (override `AUDIOROUTER_VST3_WORKER_PATH`) for VST3.
  - Packaging: `tools/release/prepare-artifacts.ps1` builds it with
    `tools/m06-vst3-worker/build.ps1 -Release`; both Tauri release configs
    bundle it.
  - Worker (`tools/m06-vst3-worker/main.cpp`): protocol pipes isolated from
    plugin stdout; mono/stereo negotiation plus in-worker adaptation;
    parameter descriptors filtered and capped at 256; instruments refused;
    `Latency` answered.
  - Host (`crates/plugin-host/src/lib.rs`): plain plugin path instead of
    `\\?\` (UA PACE rejected the verbatim path and passed audio through);
    a fixed Windows environment allowlist after `env_clear`; stderr
    diagnostic read after exit.
  - Checks: fmt; Clippy on plugin-host and control; their tests;
    `worker_process` with test-fixtures (35 passed); AGain and ReaPlugs
    acceptance; release self-tests; eight UADx effects processed mono and
    stereo ([evidence](evidence/2026-10-22-uadx-inventory.md#u0-results-in-the-app-worker-2026-10-22)).
    Not run: full-workspace Clippy, a shell Play on a live route.
  - Guide, plan, compatibility snapshot, quickstart and worker README
    updated.
- First attended Play (2026-10-22) failed: "native graph rejected:
  UnsupportedTopology". Cause, not VST3-specific: the UI starts a single
  route on the single-endpoint worker, whose compiler accepts only one
  channel width per chain (engine check: mic 1 → out 2, mic 2 → plugin 1
  → out 2 and mic 1 → plugin 1 → out 2 all rejected, multi-path compiler
  accepts all). Plugin blocks are mono, so any plugin between stereo
  devices failed. Fix: `needsNativePaths` (`ui/src/draft.ts`) sends a
  route with any width-changing connection to the multi-path worker;
  regression in `ui/src/draft.test.ts`.
- Second defect found while qualifying: after Stop, the next Play failed
  with `plugin state restore failed: StateUnsupported`. The VST3 worker's
  `StateStream::read` returned `kResultFalse` on a short read at the end,
  and UADx `setState` rejected the state. It now returns `kResultOk` with
  the count, as the SDK's `MemoryStream`; restore also uses the SDK order
  (component `setState`, then controller `setComponentState`).
  `native_vst3_fixture_signal_report` now round-trips state into the same
  and a fresh instance: all eight UADx effects pass mono and stereo.
- Checks after these fixes: `live_plugin_route_plays_again_after_stop`
  (new, opt-in) on a copy of the owner's database, privacy-muted: Play,
  Stop, Play of mic → `uaudio_ua_1176` → headphones, plugin `running`
  both rounds. AGain `m06-vst3-worker.ps1` passed; plugin-host
  `worker_process` (35) and control lib (262) passed; Clippy on both
  crates; UI 567 tests, typecheck, lint, Prettier on changed files.
  Release shell rebuilt 19:16 with a UI bundle built from HEAD plus the
  `draft.ts` change only (other sessions' uncommitted UI work excluded).
- Incident (my error, repaired): removing a temporary git worktree whose
  `node_modules` were junctions with `git worktree remove --force`
  followed the junctions and deleted `contracts/` (5 tracked files) and
  most of `contracts/node_modules` and `ui/node_modules`. Restored with
  `git checkout -- contracts/`, `npm ci` in `contracts` and `npm install`
  in `ui` (lockfile unchanged; a running site Vite server kept `npm ci`
  from replacing `ui/node_modules`). Never junction dependencies into a
  temporary worktree; remove junctions with `rmdir` first.
- Attended result (2026-10-22, owner, release shell 19:16 with VST3
  worker 19:20): mic → `uaudio_ua_1176` → headphones "works very well".
- Follow-up round (2026-10-22, owner: "the UI/UX must be consistent, drag
  and drop should work. Then, do: [plugin windows, latency display, licence
  errors, PolyMAX] Especially opening the plugin ui should be available
  along with the sliders"). Requirements: PLUG-03, PLUG-05, PLUG-06,
  SEC-07, UI-17.
  - Drag and drop: Plugins-list cards drag onto the canvas like tools,
    with the same live preview (`ui/src/libraryDrag.tsx`,
    `SessionFlowCanvas.tsx`, `App.tsx`, `PluginPanels.tsx`). Regression:
    `ui/e2e/drag-place.pw.ts` (plugin case, three themes; route harness
    gained `__routeFixturePluginInventory`).
  - PolyMAX: scans read VST3 `moduleinfo.json` sub-categories and UA
    `manifest.json` `is_synth`; instruments get the new compatibility
    `unsupportedInstrument` and leave the Plugins list (contracts, output
    schema, control). Remembered lists keep the old verdict until rescanned.
  - VST3 plugin windows (`tools/m06-vst3-worker/main.cpp`): the worker's
    main thread is now the plugin's UI thread (OLE STA, message loop,
    per-monitor DPI aware) and owns every controller/view call; a second
    thread reads the pipe and processes audio, handing every other request
    to the UI thread (`SendMessageTimeout`, 15 s, then the worker ends).
    `IPlugView` attaches inside the shell's window, sizes it to fit, follows
    resizes and applies the content scale; attach and detach are posted so
    the shell's window thread can be blocked in the close RPC. Edits in the
    window reach the processor with the next quantum; slider events reach
    the controller so the window follows the sliders. The worker now calls
    the module's `InitDll`/`ExitDll` (VSTGUI editors crashed with
    `0xC000041D` without it). UI "Open plugin editor" is enabled for VST3.
  - **Decision (owner, 2026-10-22): editor sandbox.** UADx editors are
    WebView2 and failed with `CreateCoreWebView2Controller ERROR=Not enough
    quota` (UA log) under the one-process job. Owner chose "only while
    editor open": the host raises the job's active-process limit to 16 when
    an editor opens and restores 1 when it closes; kill-on-close and the
    512 MB per-process cap are unchanged, helpers stay in the job. Recorded
    in SEC-07 (`docs/spec/13-security.md`). Rollback: set
    `WORKER_EDITOR_MAX_ACTIVE_PROCESSES` to 1 (UADx windows stay blank).
  - Sliders and window together: each state capture saves the slider values
    it contains (`<state>.sliders.json` beside the state); Play applies only
    sliders moved since that capture, so window edits are no longer undone
    by older slider values (`plugin_sliders_to_apply`, unit test).
  - Latency: the bridge asks the worker for the plugin's own delay at start
    and after its editor closes, and adds it to `latency_samples`, which the
    Properties delay already shows (`runtime_bridge_counts_the_plugin_reported_latency`).
  - Licence: the bridge flags a single plugin that has returned loud input
    (≥ −46 dBFS) unchanged for ~2 s and never changed it; telemetry
    `plugin.unchangedAudio`; the inspector shows a reserved-space hint
    (`ui/e2e/plugin-inspector.pw.ts`, three themes, no layout shift). It is
    a hint, not an error: a neutral effect does the same.
  - Checks: `native_vst3_editor_opens_in_a_pumping_parent` (new, opt-in)
    passed for AGain and all eight UADx effects (1176: window sized 966x250,
    VU moving, audio processed while open; no WebView2 error in UA's log); AGain acceptance; plugin-host lib (77) and
    `worker_process` (36); control lib (263); Clippy on both crates;
    contracts typecheck and drift; UI 567 tests, typecheck, lint, Prettier;
    the two e2e files above. Not run yet: the shell's own editor window on a
    live route, full-workspace Clippy.
- Rebased onto main e6e575ff (owner: "pull from main before") on branch
  `feature/vst3-plugin-windows`; there: fmt (workspace, shell), Clippy
  (workspace and shell with the built UI), plugin-host 77 + `worker_process`
  36, control 264, AGain editor and acceptance tests, contracts typecheck and
  drift, UI format, typecheck, lint, 642 tests, `drag-place` and
  `plugin-inspector` e2e. `plugin-inspector.pw.ts` now imports the guarded
  `./browser-test` fixture that main requires.
- Built for the owner (from the pre-rebase tree, UI from HEAD plus these
  UI files only): `src-tauri/target/release/audiorouter-shell.exe` 22:42 and
  `audiorouter-vst3-worker.exe` 22:41.
- Owner test 2026-10-22: the 1176 window opened from the app on a live
  route and "works very good". At the owner's request
  `feature/output-switch` (834a327d, f224856e) was merged locally into this
  branch before any PR.
- Next: with the owner's approval (AGENTS.md "Pull request approval"), push
  the branch and open the PR.

## Output Switch — active feature branch (2026-10-22)

The owner authorized a routing node with one input and two exclusive outputs,
toggled in the app or on Stream Deck. Work is isolated on
`feature/output-switch`; see the detailed
[Output Switch plan](output-switch.md) for contracts, decisions, tasks and
validation. Implementation and automated checks are complete. Local build:
`C:\code\audiorouter\.worktrees\output-switch\src-tauri\target\debug\audiorouter-shell.exe`
(worker and CLI sidecars are beside it). Local commit `834a327d` is on the
feature branch. Owner testing is pending. The requested local-main merge is
deferred while the shared `main` worktree has unrelated uncommitted changes
and is 32 commits behind `origin/main`; see the detailed plan.

## Wave Link parity #4 — application sources and groups — 2026-10-22

Objective: let a user add an application source from a selected `.exe` even
when that app is closed, and create a group of multiple app sources feeding a
Mixer. Preserve the existing per-process-tree selector contract and normal
graph planning/commit path.

- Requirements: CAP-05, CAP-06, CAP-10, UI-23; GRAPH-01/02 and UI-17 apply.
- Prerequisites/evidence: application sources already persist an executable
  basename/path and support `allVerifiedInstances`; running-app selection is
  currently limited to `applications.list`. The desktop has a native COM file
  dialog for session files. An app group can be represented without a new
  backend node kind: separate `applicationCapture` nodes connected to a Mixer.
- Decisions: persist only an executable selector for a picked `.exe`, never
  the current PID; use `allVerifiedInstances`, and let backend preparation
  resolve the verified live instance(s). A group is a creation convenience
  expanded into ordinary application nodes and one Mixer, so each app can be
  renamed/rebound/removed independently. Select multiple running app entries
  or add `.exe` paths; de-duplicate identical normalized executable paths.
  The Windows picker uses the existing isolated COM-dialog pattern and offers
  a typed full-path fallback in browser previews. Do not auto-start or reroute
  apps. Do not silently claim parent/child deduplication unless backend
  evidence establishes it; surface the CAP-10 limitation for candidate groups
  and record any required backend follow-up explicitly. Fix the global checkbox
  min-height so app-group switches match the existing 2.55 × 1.4 rem switch;
  review this shared switch change in all three themes.
- Ordered tasks: (1) add UI-23 picker/group contract and update this plan and
  future list; (2) add strict path-to-selector helper and atomic group graph
  expansion (app sources → Mixer); (3) add native `.exe` open dialog and
  browser path entry; (4) wire single and grouped flows into the canvas
  picker; (5) regress closed-app persistence, grouped topology, duplicate
  paths, malformed paths, and composition with existing sources/outputs;
  (6) run UI format, lint, typecheck, unit tests, production build, focused
  Edge tests in dark/light/high-contrast, relevant Rust format/Clippy/tests,
  documentation validation and diff review.
- Validation matrix: pure draft tests for executable selectors and atomic
  composition; component tests for app selection and error states; Windows
  Edge route-harness scenarios for closed apps, groups, existing sources,
  and no layout shift; Rust shell checks for the new Windows dialog command;
  docs links/anchors and `git diff --check`. CI supplies the final current-base
  check. No live audio is required to validate graph construction; do not
  claim runtime process capture from UI evidence.
- Risks: invalid, non-EXE, moved, protected or ambiguous applications must
  remain stopped/silent with backend errors; app groups can duplicate audio
  when one chosen process tree contains another until CAP-10 overlap detection
  is verified. Rollback: revert UI-23, the scoped picker/draft/dialog changes,
  tests and docs together; no session schema migration or audio-driver change.
- Implementation (2026-10-22): closed-app paths create `allVerifiedInstances`
  selectors with basename and full path, no PID; running single-app selection
  retains the existing exact-instance behavior. Group creation accepts running
  apps and/or executable paths, normalizes duplicate paths, and atomically
  expands them into ordinary capture nodes plus a Mixer. Added a Windows
  `.exe` Open dialog on its own COM STA thread. No session schema, backend API,
  graph node kind, or playback policy changed. The picker warns against
  parent/helper combinations and the action summary repeats the caution. The
  checked global switch sizing is 2.55 × 1.4 rem; it fixes existing checkbox
  switches too.
- Validation (Windows 11, 2026-10-22, Node 22.22.3, Edge; pinned Rust 1.96.0): UI format
  and format check pass; TypeScript 7 `typecheck:all` passes; ESLint exits 0
  with 25 existing exhaustive-deps warnings and no errors; all 72 UI test files
  pass (638 tests); production build passes (existing >500 KB chunk warning).
  Focused Edge route-harness tests pass 3/3: closed-app path selection,
  two-source Mixer expansion while preserving prior edges, and picker stability
  in dark/light/high-contrast. Screenshots reviewed from
  `%TEMP%/audiorouter-playwright-results/application-sources.pw.ts--669c1-e-and-stable-in-every-theme/`.
  The first run of the added three-theme test hit a non-unique test locator;
  using an exact accessible name fixed the fixture, and the complete rerun
  passed. Shell Clippy (`--all-targets --all-features -D warnings`) passes;
  workspace Clippy passes; `cargo test --manifest-path src-tauri/Cargo.toml
  --locked` passes (83 passed, 4 ignored); both Rust format checks pass.
  Documentation acceptance passes (186 Markdown files, 1,027 links), and
  `git diff --check` passes. Not run: live process-loopback capture, native
  file-dialog interaction in a packaged shell, or group behavior with
  parent/child process trees.
- Owner local-test build (Windows 11): `npm.cmd run build --prefix ui` passed,
  then `cargo build --release --manifest-path src-tauri/Cargo.toml
  --features custom-protocol`, `cargo build --release -p
  audiorouter-plugin-host --bin audiorouter-plugin-worker`, and `cargo build
  --release -p audiorouter-cli` passed with `CARGO_TARGET_DIR` set to
  `target/owner-local-test`. Shell artifact:
  `C:\code\audiorouter\.worktrees\wave-link-app-sources\target\owner-local-test\release\audiorouter-shell.exe`.
  The plugin worker and CLI are beside it. This is a local unsigned build,
  not an installer or release qualification.
- Deviation/blocker: current API does not expose process ancestry and backend
  CAP-10 does not yet detect parent/helper overlap between app-capture nodes.
  Group creation therefore warns clearly but cannot deduplicate or reject a
  selected parent/helper pair. Keep this as a CAP-10 follow-up before claiming
  full group-overlap safety.
- Review handoff: commit `53816c46` is pushed in isolated branch
  `feature/wave-link-app-sources`; [PR #59](https://github.com/MrDesjardins/audiorouter/pull/59)
  was opened before the owner's new PR-approval preference, later reopened,
  and merged by the owner on 2026-10-22.
  The owner tested the local build and reported that it is working. On
  2026-10-22 the owner authorized pushing these changes directly to `main`.
  GitHub rejected that push because branch rules require a PR and the
  `Quality gate` status check. The feature branch was pushed through commit
  `18e91b3f`; PR #59 was later merged by the owner, so no replacement PR was
  opened. The hosted CI run for PR #59 was cancelled before the owner merged
  it. Keep CAP-10
  overlap detection on the open list.
- Next: PR #59 was merged by the owner on 2026-10-22. The owner selected Sound
  Check (#10), now tracked in [Wave Link parity](wave-link-parity.md). The owner
  also reported occupied Advanced EQ inputs should add a Mixer rather than
  replace the existing microphone; the fix and the updated Sound Check
  comparison row are in PR #60, opened at the owner's explicit request on
  2026-10-22. The owner then reported that Audio File Loop was disabled during
  playback and Sound Check status text had a mojibake separator. The fixes
  are recorded in the detailed parity plan; the owner tested the local build
  and confirmed both fixes work. Fixes were pushed to existing PR #60 in
  commit `d8e4d5d7`. Await review and CI; the owner handles merge. Continue
  with Listen to any mix
  (#1 + #9) after review; CAP-10 overlap detection remains a separate
  app-group follow-up.

## Pull request approval preference 2026-10-22

The owner requests that no agent create or open a pull request until a human
explicitly approves the exact branch and changes. Complete local validation,
inspect the diff, and report the branch, commit and results before asking for
that approval. Authorization to implement, commit, push or run CI is not PR
approval. PR #59 was eventually reopened and merged after owner testing.

## Wave Link parity, first round — 2026-10-22, owner request

Tracked in its own plan, [Wave Link parity](wave-link-parity.md): level
target zone on the Meter (UI-21) and hidden devices in pickers (UI-22), both
UI-only. The candidate list is in
[Wave Link parity candidates](../future/wave-link-parity.md).

## CI path selection — 2026-10-22

Objective: keep website-only and Markdown-only changes out of AudioRouter's
Windows build, Rust test and Edge browser suites while retaining focused
validation for the changed surface.

- Acceptance: website changes run website checks; Markdown changes run
  documentation validation; product source changes continue to select their
  existing checks; mixed changes run the union. Main pushes classify the
  pushed diff, while manual dispatch remains a full run. The required
  `Quality gate` accepts only explicitly skipped unaffected jobs.
- Requirements: CI path-selection behavior; no product requirement or runtime
  behavior changes.
- Decisions: classify `site/` independently from app `ui/`; add explicit
  `docs` and `site` outputs; keep the policy test and aggregate gate
  unconditional; classify CI-policy edits through policy tests without
  running product quality suites.
- Ordered tasks: extend path classification and gate expectations; add a
  lightweight website check; make docs/site jobs conditional; classify push
  events by their before/after commits; test site, Markdown, code and mixed
  change sets.
- Validation: run the quality policy and verifier Node tests; check social
  metadata only on changed website pages plus the shared sitemap/robots files;
  run guide integrity checks and let the PR Quality gate execute.
- Risks: path omissions could skip a relevant suite; unknown executable paths
  continue to select all product areas. Rollback: revert the classifier,
  workflow and tests together.
- Evidence: policy tests passed; the first website check correctly exposed unrelated stale social tags on untouched guide pages, so the generator check is now scoped to changed pages. The updated gate is running.
- Next: merge after the narrowed Quality gate succeeds.


## Website comparison: AudioRouter and Elgato Wave Link — 2026-10-22

Objective: add an accurate, neutral Wave Link 3.0 comparison to
`site/compare.html`, alongside the existing Voicemeeter comparison.

- Requirements: public feature claims must match current AudioRouter
  compatibility evidence and cited vendor information; distinguish routing
  models, inputs/mixes, effects, device support, automation, cost, source and
  platform.
- Prerequisites/evidence: Wave Link works with third-party mics/interfaces,
  has up to 8 software/non-Wave channels and up to 5 mixes, is free, and
  supports VST3 on Windows ([Elgato Wave Link specs](https://www.elgato.com/ww/en/s/wave-link-app),
  [third-party microphone support](https://www.elgato.com/us/en/explorer/products/wave/which-microphones-work-with-wave-link/)).
  AudioRouter's plugin boundary is documented in
  `docs/operations/plugin-compatibility.md`; VST format support must not be
  described as a universal UAD or vendor-plugin guarantee.
- Decision: keep the Voicemeeter table intact and add a distinct Wave Link
  table and “when Wave Link is the better choice” guidance. State that both
  are free and that Wave Link's Stream Deck integration and device controls
  are strengths; compare AudioRouter's route canvas, built-in processing,
  isolated plugin workers, local automation API, and open source accurately.
- Ordered tasks: add this plan entry; update compare-page metadata and add
  the Wave Link comparison/source note; run social metadata check, local-link
  and HTML checks, inspect the responsive page in Edge, and review the diff.
- Validation: `node site/generate-social.mjs --check` after regeneration,
  `git diff --check`, and Edge visual review at desktop/mobile widths. The
  site has one dark theme. No product/runtime requirement ID changes.
- Risks: vendor specifications or software behavior can change; date and
  link the primary sources. Avoid implying every VST3 or UAD plugin works.
- Rollback: revert the page and this plan entry; no application or user data
  changes.
- Evidence (Windows, Node 22.22.3, Microsoft Edge): `node
  site/generate-social.mjs --check` passed (47 pages); `node
  site/check-guides.mjs` passed (36 pages, 894 links/assets); a jsdom audit
  found no missing local links or malformed rows in the 13 Wave Link feature
  rows; `git diff --check` passed. Headless Edge at 1440 px and 390 px had no
  horizontal overflow or page errors; screenshots are in the local temp
  directory (`audiorouter-compare-desktop.png`,
  `audiorouter-compare-mobile.png`). The full page was visually reviewed.
- Next: review the diff and follow the separately authorized site
  publication path.


## Quality Q4 - graph commit storage-failure recovery - 2026-10-22

Objective: cover the approved quality roadmap's graph-commit persistence failure
boundary (ENG-01/02, API-05/07, STATE-01). Main contains the merged Q1-Q3
baseline; this slice adds the missing deterministic control-plane regression.

Prerequisites and decision: use only a disposable SQLite fixture and an
injected trigger; do not touch the daily database or launch the shell. Keep
product behavior and APIs unchanged. The test checks rollback of the in-memory
graph and durable revision when persistence fails, then retries the same plan
with the same idempotency key and verifies exactly one durable commit.

Implementation: added the regression and a dev-only `rusqlite` dependency.
The fixture now ignores only `NotFound` when clearing a stale file and fails
on other cleanup errors. Rollback: revert the scoped test and dev-only
dependency commits.

Validation matrix (Windows 11 x64, pinned Rust 1.96.0): focused regression
passed after the cleanup fix; both workspace and desktop-shell Rust format
checks passed. The branch's earlier run recorded full workspace tests and
Clippy, UI type/build checks, and a mutation check; current-base GitHub CI is
the remaining gate. No hardware or audio endpoints are required. This does
not qualify native audio or packaged startup.

Next: finish the protected current-main PR gate, merge if green, then rerun
the merged-main checks and repair any failures.

## Quality Q3 - production browser smoke - 2026-10-21

Q2 storage slice is isolated in PR #53 on history PR #52 and feature PR #51.
This slice starts at 01e31b77 on quality/production-smoke. Requirements UI-02/11/17,
NFR-08/14/15, ENG-02/05. Extend the optimized Vite harness to include the real
backend adapter and the missing group/layout/history/clipboard/session/recording
scenarios. Preserve the full development suite. Make the ordinary e2e script
run both so Q1's shared browser check and CI automatically select the smoke;
retain separate failure outputs. Add an explicit session/connector readiness
condition using the existing bounded measurement repair, and regress cold/warm
loads and generation changes without arbitrary sleeps. Run optimized Edge,
format/lint/type/unit/build; triage unexpected page errors, keep traces and
three-theme/scaling evidence. Packaging smoke uses only an isolated profile and
the exact built shell/sidecars; do not call browser evidence packaged evidence.
Prerequisites: installed Edge, Windows build cache and no-device Rust fixture.
No live audio or default endpoint changes. Rollback: revert the scoped smoke
commit(s). Next: extend the existing production inputs and scenario selection.

Q1 status while Q3 proceeds: first full CI run 39710446637 passed all eight
jobs at 0a7cd2af. Main ruleset 25451476 is now active: PR/resolved threads,
strict current-base Quality gate (GitHub Actions app 15368), no bypass actors,
no deletion/force push, independent approvals=0 for the sole-owner repository.
PR #50 was updated to main 1be9ce3b and pushed at 10b666b2; current-base run
39713478156 remains pending. A separate docs-only PR #54 at fd2b4c65 proved
the legitimate-skip path in [run 39713611409](https://github.com/MrDesjardins/audiorouter/actions/runs/39713611409):
classification/docs/policy/gate succeeded; Windows/Edge/supply/Stream Deck were
explicitly skipped. Closed that temporary PR after retaining the evidence.
Its result does not replace the full run or waive any hardware/release gate.

Q3 baseline experiment: extended optimized acceptance input from route-only to
route plus real-backend adapter, and selected the missing history, group,
clipboard, session and synthetic-recording suites. `npm.cmd run
e2e:production-canvas -- --workers=1`: 72 passed in 1.8m. Both entry HTML files
and their optimized chunks were emitted; the normal desktop build still has
only index.html. Browser command now runs full development plus optimized smoke
through the shared package script, so Q1 inventory needs no duplicate command.
Separate production trace/screenshot output is included in CI failure uploads.

Readiness now reports the rendered session/revision and valid card/connector
measurements through a hidden DOM probe; it waits for fonts and reuses the
existing bounded repair, exposing exhausted rather than a false ready state.
Three pure readiness tests, TypeScript and seven development Edge scenarios
(including delayed first snapshot, cold repair in three themes, reload, new
session/revision and permanent missing bounds) passed. Output:
%TEMP%/audiorouter-q3-readiness/. This is not packaged WebView2 evidence.
The user's app is currently running (PID 28952, group-layout-test build), so
the ordinary single-instance packaged smoke cannot safely run locally until
that app is closed. No process or daily-use profile was changed. Continue
independent optimized/CI work and prepare an isolated packaged check.

Further Q3 evidence: the full guarded local pipeline completed successfully.
UI unit tests passed all 71 files/611 tests; the development Edge invocation
exited zero across its 326 collected scenarios (explicit optional skips remain
unrun), and all 76 optimized scenarios passed. Both Playwright `.last-run.json`
files report `passed` with no failed tests. The optimized directory is now
explicitly emptied before builds; verified its resolved dedicated target
beneath target/canvas-production-ui before cleanup. Wiring tests check both
build inputs, selected scenario existence and package scripts. The uncaught
error fixture is shared by all 45 formerly raw Playwright scenario modules;
real-backend scenarios retain their stricter existing guard. The initial
negative guard setup failed to import a file: URI on Windows and was correctly
rejected by the positive control/expected assertion markers. After fixing to an
absolute module path, the clean-page child exited 0 and intentional-error child
exited 1 at the intended guard assertion. Evidence: `%TEMP%/audiorouter-browser-guard-Tt7cbR/`
and `%TEMP%/audiorouter-browser-guard-szqJfN/`. Fixed five Node-script lint
errors through explicit process/log imports; lint has only the existing 25
hook warnings. Formatting, all-project TypeScript, Vite build and lint pass.

CI limitations retained: PR #53's Jev run 39712836614 failed with HTTP 422
while building a location-choice dictionary (291 entries, API limit 255).
It reported an unlocated provisional rule violation but posted no review;
this is not a successful review or a confirmed product bug. The pinned tool
lives in another repository, so no unapproved upstream edit or fabricated
finding was made. Actual UI/TypeScript/model/browser evidence remains separate.
Q1 current head is 31386905, run 39714997868: policy, docs, supply, Stream
Deck and Edge passed; Windows strict Clippy is still in progress. The prior
503/cancelled run is not counted as passing; registry audit recovered on this
new run.

Q5 baseline inspection (not a new coverage run): downloaded metrics-coverage
from [nightly 39570183964](https://github.com/MrDesjardins/audiorouter/actions/runs/39570183964)
at b4bbbef57f56598d50dcf3b5cabc25b7093a1e70: Rust workspace lines 78.3%,
shell lines 56.8%, UI lines 68.5%. Artifact copy:
%TEMP%/audiorouter-quality-baseline-39570183964/. These older global percentages
are not baselines for the new history modules and do not justify setting a
coverage floor yet. Continue measured module coverage before a ratchet.

## Quality Q2 - presentation persistence - 2026-10-21

History slice is isolated in PR #52 on top of group PR #51. This next slice
starts at bccd6249 on quality/presentation-storage. Requirements UI-02/16/17,
ENG-01/02: reject oversized local JSON before parsing and normalize decoded
geometry to its allowed fields. Current readLayout accepts extra fields; those
are later spread into CanvasRect, so a stored id can replace a graph identity.
Reproduce with a focused decoder regression; add bounded pure storage helpers,
canonical position/size/side decoders and malformed/generated corpus tests.
Retain valid data, legacy group behavior and optional write-failure handling.
Compatibility: existing ordinary shapes remain readable; invalid extra fields
are discarded; oversized presentation records fall back without changing audio.
Verify failing-before/passing-after tests, full UI checks and canvas browser
scenarios. No hardware required. Rollback: revert this slice; no graph/database
migration. Next: demonstrate the unexpected-field failure before implementation.

Confirmed two failures before fixing: the new layout unit regression returned
foreign id/group/width fields, and a denied window.localStorage getter crashed
AppContent in Edge (SecurityError, canvas absent). Before-browser artifacts:
%TEMP%/audiorouter-q2-storage-before/. Canonical decoders now construct only
allowed position/size/side fields, validate IDs against the backend's 128-byte
bound, and reject JSON above 1,048,576 characters before parsing. Shared safe
storage access protects App preferences, canvas, groups and diagnostics. Groups
also discard foreign fields while retaining legacy defaults. Oversized writes
preserve the prior record; optional faults leave in-memory editing usable.

Expanded evidence: 300 generated malformed/canonical persistence cases, 600
fixed-seed nested gestures checking frozen ownership/once-only movement/locks,
and nightly coverage retains fixed seeds plus a reproducible run-ID seed.
Replayed AUDIOROUTER_TEST_SEED=39710446637: 19 selected tests passed. Removed
the normalization, parse-bound, denied-getter catch and write catch in four
temporary experiments: all four made focused tests fail; originals restored.
Logs: %TEMP%/audiorouter-q2-storage-mutations-jK2Hgb/.

Final Windows checks: format, lint (zero errors, 25 existing Hook warnings),
all-project TypeScript and Vite build passed (existing chunk warning), all
69 files/604 UI tests passed. Full focused Edge set: 45 passed in 56.9s,
including poisoned saved identity and denied-storage startup/undo regressions.
Artifacts: %TEMP%/audiorouter-q2-storage-final/. Visually inspected cold-start
screenshots in dark, light and high-contrast: canvas/cards/controls/notifications
remain readable and positioned consistently. actionlint 1.7.12 validated the
nightly seed change. Docs and diff checks passed. No backend/audio changes or
database migration; local presentation compatibility consequences are specified
in UI-16. Next: scoped persistence PR, then Q3 production browser coverage.

## Quality Q2 - editor state boundaries - 2026-10-21

Owner approved the [application quality roadmap](application-quality.md).
Q1 is isolated on main in PR #50; current CI run 39710446637 is pending.
Its local intentional format/type/test/contract/classifier faults were all
detected by the real shared runner and restored in finally blocks; reports
are in %TEMP%/audiorouter-q1-faults-zplj4b/. This is failure-detection evidence,
not passing product evidence. Q2 starts at the reviewed group branch 6d9890e0
in its own worktree; it does not implicitly merge that feature or PR #50.

Scope UI-02/11/16/17, ENG-01/02: specify session/presentation/history invariants,
extract pure layout history/restore operations without moving backend authority,
and test generated action sequences plus malformed persistence/fault boundaries.
Prerequisites: Windows/Edge and deterministic backend fixture available, no
audio hardware needed. Steps: reproduce repeated Arrange no-op history issue;
extract and integrate history; keep Undo arrange on the same history path;
add fixed-seed bounded sequence tests, lock/session/storage guards and focused
browser regressions; run format/lint/type/unit/Edge and inspect diffs. Rollback:
revert the Q2 commit; no database or contract migration is intended. Keep
generated seed/action trace in failure output. Next: reproduce before fixing.

Q2 first evidence: Edge reproduced the redundant Arrange undo entry at
6d9890e0 (%TEMP%/audiorouter-q2-arrange-before/, expected original voice position
445,67 but undo left arranged 419,0). Added pure history model/tests first;
integration and the user-visible fix follow as a separate commit. Five seeds
each exercise 400 actions against an independent coordinate/connector model.
The model's lock, stale-session and redo-clear guards were each removed in a
temporary experiment: all three mutations made the focused suite fail; originals
restored in finally. Logs: %TEMP%/audiorouter-q2-mutations-u2e7FB/.
The integrated working tree passed 590 UI tests, type/build/lint (25 existing
Hook warnings, zero errors), and 28 focused Edge scenarios including the
formerly failing case. Final integration checks and the Undo arrange/redo
regression still follow. No Rust/audio or backend graph behavior is changed.

Q2 history integration completed: all geometry/side/Arrange history commands
now use the pure module. Repeating unchanged Arrange preserves undo/redo, and
Undo arrange delegates to the ordinary undo path, permitting Redo layout.
The model clones snapshots, rejects cross-session restore and locked changes
atomically, and touches only surviving graph identities and preferences.
Compatibility: local storage shapes are unchanged; history is transient and
still resets on session changes. No migration is needed. User guidance and
UI-16 contract now state the no-op/redo semantics.

Final Windows checks: `npm.cmd run format`, `npm.cmd run lint` (zero errors,
25 existing warnings), `npm.cmd run build` (TypeScript all-project check and
production build passed; existing chunk-size warning), `npm.cmd test`:
65 files/590 tests passed. Complete focused Edge set (review, layout, session,
lock/groups, smart-layout, cold-start, clipboard, copy/paste and drag-place):
43 passed in 56.3s, including dark/light/high-contrast scenarios and both new
Arrange regressions. Output %TEMP%/audiorouter-q2-history-final/. Earlier 28-case
integration run also passed; no diagnostic rerun is counted as first-pass
success. Existing React Flow handle/attribution warnings remain visible.
`node tools/docs/validate.mjs` and `git diff --check` passed. Q2 history is
ready for its own review on top of the unmerged group feature PR #51; next
Q2 slice is bounded/sanitized presentation persistence and generated geometry.

## Group/layout review round two - 2026-10-21

Scope: UI-02/11/16/17, review undo interleaving, connector-side presentation
preferences, nested movement, locks and persistence. Worktree initially clean
at 1de3a33a. No hardware needed. Candidate probes: Undo layout followed by
Undo arrange, connector side changes after geometry edits, invalid saved edge
preferences, and storage write failures. Reproduce before fixes; add focused
regressions, repair owning layout/history layer, run unit/Edge/format/lint/build
and record evidence. Rollback: revert this review fix commit.

Confirmed and fixed four findings at 1de3a33a:
1. Undo layout left the temporary Undo arrange action live. Using that stale
   shortcut could add a no-op snapshot and discard valid redo. New geometry,
   side edits and history restoration now retire the older Arrange shortcut.
2. Connector-side edits were omitted from history while geometry snapshots
   restored connector sides, so undo could discard a later choice. Side edits
   now record layout history and clear redo; no-op side selections do neither.
   Connection creation initializes both sides together without extra history
   entries; a synchronous ref preserves both updates. Restore touches only
   edges existing in the snapshot and current graph, preserving newer edges.
3. Saved connector preferences were unchecked JSON. A saved null crashed the
   canvas on opening. A validated reader rejects null/arrays/scalars/invalid
   side values and falls back to default sides, while retaining valid entries.
4. Connector preference saves ran inside a React state updater and an optional
   storage quota failure crashed the canvas. Writes now run outside updaters,
   catch storage failures, and leave the in-memory editor/undo usable. Arrange
   and undo use the same safe persistence path. Existing on-disk shape stays
   compatible; no migration or graph/backend change is needed.

Evidence (Windows, 2026-10-21): before fixes, all 3 initial browser probes failed
at their intended assertions, including an uncaught null preference exception.
Artifacts: %TEMP%/audiorouter-layout-review-round2-before/. The separate quota
probe reproduced an uncaught QuotaExceededError and canvas error boundary:
%TEMP%/audiorouter-layout-review-round2-quota-before/. After fixes all four
probes passed. Added ordered geometry/side undo-redo and new-connection-side
regressions. One expanded test initially compared the whole transient node
style (hidden versus visible); corrected it to compare geometry after node
visibility and reran the complete set. No product change was based on that
setup failure.

Final direct Edge command from ui/: node node_modules/@playwright/test/cli.js
 test --config=playwright.config.ts e2e/canvas-layout-review.pw.ts
 e2e/canvas-layout.pw.ts e2e/canvas-layout-session.pw.ts
 e2e/canvas-group-lock.pw.ts e2e/canvas-groups.pw.ts e2e/smart-layout.pw.ts
 e2e/canvas-cold-start.pw.ts e2e/canvas-clipboard-regressions.pw.ts
 e2e/canvas-copy-paste.pw.ts e2e/drag-place.pw.ts --workers=1
 --output %TEMP%/audiorouter-layout-review-round2-final: 41 passed.
Covers the six review regressions, nested/locked groups, bulk flags, alignment,
sizing/distribution, geometry undo/redo, session isolation, pointer connection
creation, clipboard and tool placement in all three themes. Existing resize
observer and intentional cold-connector warnings remain as documented below.

npm.cmd --prefix ui test: 64 files / 578 tests passed (including eight new
connector preference validation/storage cases). npm.cmd --prefix ui run format,
format:check, lint and build (TypeScript 7 typecheck:all included) passed.
Lint retains 25 existing warnings, no new warning; existing Vite chunk warning
remains. git diff --check passed. UI specification and Group guide source were
updated; node site/generate-guides.mjs, node site/generate-social.mjs,
node site/check-guides.mjs and social --check completed successfully.

Native shell build passed with the same locked release/custom-protocol command
recorded below; separate manual test app with matching sidecars/runtime:
%TEMP%/audiorouter-group-layout-review2-test/audiorouter-shell.exe.
SHA256: 837DD256499E0F97A9A93BF1FD48C6AC31B04B4AA0BD71C0FCF59BA638E042B3.
Original cached shell restored and its hash verified unchanged. These are UI
repairs, not an audio/driver qualification or signed release. Review found no
additional confirmed functional defect in the other inspected paths.
Next: owner acceptance of this second-review build on feature/canvas-group-layout.

## Group layout review - 2026-10-21

Review found a session-isolation risk in the older Undo arrange control:
its 15-second snapshot has no session key and survives a session switch,
allowing old positions/connector sides to be written into the next session.
Affected code predates the group feature and is present at 103802f9.
Requirements UI-02/16, local presentation only. Plan: reproduce with real
backend session switching, scope and clear the Arrange snapshot, regress
normal Arrange undo and session switching, typecheck/lint/format/build.
Rollback: revert this isolated fix. No audio hardware is needed.

Confirmed: real-backend Edge regression failed because Undo arrange remained
visible in the newly created session within its 15-second lifetime. Evidence:
%TEMP%/audiorouter-arrange-review/canvas-layout-session.pw.t-eb1f8-ded-when-switching-sessions/.
Fixed: snapshots store the layout/session key; rendering and execution check
that key, and session switching clears the snapshot. Returning to the original
session does not resurrect the old undo. No stored data migration is needed.
Validation: direct Playwright command for e2e/canvas-layout-session.pw.ts and
e2e/smart-layout.pw.ts, --workers=1, passed all 6 tests (Windows Edge), covering
this reproduction and normal Arrange undo in all three themes. Artifacts:
%TEMP%/audiorouter-arrange-fix/. npm.cmd --prefix ui run format, format:check,
lint and build (including TypeScript 7 typecheck:all) passed. Lint retains the
same 25 existing warnings; the existing Vite chunk warning remains.
Native shell build passed with the same locked release/custom-protocol command
recorded below; original cached executable restored with its verified hash.
Separate local app: %TEMP%/audiorouter-group-layout-review-fix/audiorouter-shell.exe.
SHA256: 69BA50CFE640DAA1E5CE10EE4657C1BEF4AE9AB4D689C4BC25C56886AC07D9DA.
Next: owner acceptance of the fix on feature/canvas-group-layout.

## Multi-group options and layout icons - 2026-10-21 owner feedback

Objective: UI-02/11/16/17 follow-up: edit Lock and Move contents for all
explicitly selected groups, with native mixed-state checkboxes. Mixed
selections apply these options only to groups; locked groups must remain
unlockable. Preserve geometry, graph and selection. Add conventional SVG
alignment/spacing/size icons with visible labels and accessible button names.
Steps: group-only batch option update; selection controls and icons; regress
mixed states, unlock, persistence and no geometry changes; format/lint/build,
unit and Edge tests with dark/light/high-contrast visual inspection. No audio
hardware is needed. Rollback: revert this follow-up commit.

Implemented: group-only batched option updates permit unlocking and changing
Move contents on locked groups, while geometry still uses the locked-aware
batch path. Selected group values come from saved group state. Mixed values
show a dash plus Mixed text; clicking sets all selected groups on, then off.
Tools and unselected groups stay unchanged. Icons accompany all 12 layout
actions and keep their visible/accessibility names. Updated UI specification
and generated Group guide from its source JSON. Review confirmed that locking
does not disable the option controls, and option edits never enter graph state.

Evidence (Windows, 2026-10-21): npm.cmd --prefix ui test passed 64 files / 570
tests. npm.cmd --prefix ui run format, format:check, lint and build passed;
build includes TypeScript 7 typecheck:all. Lint retains 25 existing warnings;
Vite retains the existing main-chunk size warning. The first typecheck caught
a test-only unsupported query option; corrected it and the build passed.
Direct Edge command from ui/: node node_modules/@playwright/test/cli.js test
--config=playwright.config.ts e2e/canvas-layout.pw.ts
e2e/canvas-layout-session.pw.ts e2e/canvas-group-lock.pw.ts --workers=1:
18 passed. The three bulk-option theme tests also passed again with screenshots
in %TEMP%/audiorouter-group-options-visuals/. Checks cover mixed states, lock
and unlock all, move-content on/off, persistence, unchanged rectangles and
graph Save state, and unselected group isolation in mixed tool/group selection.
Visually inspected dark/light/high-contrast controls and SVG icons. Existing
ResizeObserver resize notifications remain as recorded below.
node site/check-guides.mjs and node site/generate-social.mjs --check passed.

Native shell rebuilt with the same locked release/custom-protocol command as
below. Separate manual review app (same matching sidecars/runtime):
%TEMP%/audiorouter-group-layout-options-test/audiorouter-shell.exe.
SHA256: 2B650CF8A7D03D6CD3ADA42DE11C7B394EF6F3640180179E1C0BC69157E0CE0F.
Original cached shell restored with the same verified hash recorded below.
No Rust/audio behavior changed; no new audio qualification is claimed.
Next: owner review of the follow-up build on feature/canvas-group-layout.

## Group movement and selection layout — 2026-10-20, owner overnight request

Objective: optional group-content movement plus mixed tool/group selection,
alignment, equal sizing and equal spacing in Properties. Requirements UI-02,
UI-11, UI-16 and UI-17; presentation data only, no graph/API/audio changes.
Prerequisites: M05 canvas and local group/layout storage; installed Windows
Edge for real pointer/selection/themed verification. No audio hardware needed.

Decisions: full rectangle containment, with the smallest containing group as
the unique owner (stable ID breaks overlap ties). Freeze ownership at drag
start. Nested groups and their descendants move once with a carrying ancestor;
the inner toggle controls dragging that inner group itself. New groups carry
contents by default; old annotations without the flag keep their old behavior.
Locked rectangles stay fixed; a carrying group with a locked descendant cannot
move until unlocked or carrying is disabled. A locked rectangle does not freeze
individually edited audio tools. Partial overlaps are not membership.

Layout actions use selection bounds. Selected carrying ancestors subsume their
selected descendants for movement; sizing applies to explicitly selected items
without scaling contents. Locked groups remain fixed anchors. Distribute equal
edge gaps between the endpoints and any locked anchors; reject insufficient
space rather than create overlaps. Tool sizes retain usable content minima,
and persist separately from graph state. Align/sizing/distribution never rewire.

Tasks: (1) pure tested ownership/movement/layout math and validated sizes;
(2) group toggle and drag integration; (3) mixed selection and accessible
Properties controls; (4) regression tests for locks, nesting, overlap, resize,
session isolation and persistence; (5) format/lint/typecheck/build/unit tests,
headed Edge dark/light/high-contrast, full diff review and fixes. Include the
three existing clipboard findings in integration review because selection and
keyboard ownership are changing here. Record results and remaining limits.

Rollback: revert scoped UI changes and the new presentation size key; old
group/layout records remain compatible. Risks: double movement in multi-drag,
ownership drift mid-drag, stale selection across sessions, clipping after size
changes, and accidental changes to locked rectangles. Implementation and review
are complete; next is owner acceptance using the separate review executable.

Implementation: pure ownership/translation/layout math in canvasGeometry.ts;
batched group updates and persisted moveContents flag; mixed React Flow
selection and Properties layout controls; validated per-session tool sizes;
20-entry local Undo layout/Redo layout history for drag, resize, keyboard moves
and layout actions. History respects current locks and preserves connector-side
preferences. Group and size presentation preferences stay outside graph drafts.
The Group website guide and UI-02/UI-11/UI-16 wording were updated together.

Review fixes and regression evidence:
- NodeResizer end callbacks can see the starting render. Recording history at
  the first changed geometry fixes disabled resize undo, without recording
  no-op gestures. The real-pointer resize/lock/undo regression passes.
- Test Signal and Audio File had later fixed-width CSS overrides. Restrict
  those defaults to unsized cards; both now honor size matching, restore their
  defaults on undo, and keep their buttons inside the minimum-size card.
- Keyboard movement records layout history; undo restores connector-side
  preferences after Arrange; imported IDs cannot inherit presentation sizes
  or positions from Object.prototype. Ownership is computed once per render.
- All three clipboard review findings recorded below are resolved here:
  keyboard paste is brought into view; selected UI text keeps native copy;
  customizable playback/privacy shortcuts reserve Ctrl+C and Ctrl+V. The
  three focused browser regressions pass, including copying while playing.

Validation (Windows host, 2026-10-20; no hardware required for this UI slice):
- npm.cmd --prefix ui test: 63 files / 568 tests passed on the final code.
  Pure tests cover deterministic overlap ownership, partial containment,
  nested selected descendants, disabled carrying, locked descendants,
  coordinate bounds, six alignments, shared usable minima, unequal vertical
  sizes, edge-gap arithmetic, locked anchors and atomic insufficient-space
  rejection. Persistence tests cover legacy flags and invalid stored sizes.
- npm.cmd --prefix ui run format and format:check: passed; git diff --check
  passed. npm.cmd --prefix ui run lint: exit 0, the same 25 existing
  exhaustive-deps warnings, no added warning.
- npm.cmd --prefix ui run build (includes typecheck:all with TypeScript 7):
  passed. Existing Vite main-chunk warning remains (about 549 KB minified).
- Final consolidated Edge run: 47 tests passed, exit 0 (2.5 minutes).
  Command from ui/:
  node node_modules/@playwright/test/cli.js test --config=playwright.config.ts
  e2e/canvas-layout.pw.ts e2e/canvas-layout-session.pw.ts
  e2e/canvas-clipboard-regressions.pw.ts e2e/canvas-copy-paste.pw.ts
  e2e/canvas-groups.pw.ts e2e/canvas-group-lock.pw.ts
  e2e/smart-layout.pw.ts e2e/canvas-cold-start.pw.ts e2e/drag-place.pw.ts
  e2e/header-canvas-usability.pw.ts e2e/site-guides.pw.ts --workers=1.
  Covers dark/light/high-contrast, mouse and keyboard movement, mixed and
  marquee selection, exactly-once nested movement, locks, frame-only mode,
  resize, undo/redo, equal gaps, min/max sizing, reload/session isolation,
  media-card bounds, connector updates and unchanged graph Save state.
  Also covers existing clipboard, Arrange, tool placement, cold connectors,
  header behavior and every website guide (including offline control counts).
  Screenshots were visually inspected in all three themes; artifacts are in
  %TEMP%/audiorouter-playwright-results/canvas-layout.pw.ts-*/.
  The pre-existing ResizeObserver loop notification still appears during
  group resize, including the existing group tests; geometry assertions pass.
  The cold-start test deliberately hides connectors briefly and emits the
  expected missing-handle warnings before successful recovery.
- Guide generation: updated site/guide-content/routing.json, ran
  node site/generate-guides.mjs and node site/generate-social.mjs. Only the
  Group guide, guide index and manifest change. node site/check-guides.mjs
  passed: 36 pages, 139 anchors, 1,075 links/assets and 36 catalog links;
  generator social --check passed. After regeneration, the offline guide
  opening/control-count test and guide search/index test passed again (2/2).
- Native build: cargo.exe build --locked --release --features custom-protocol
  --manifest-path src-tauri/Cargo.toml --target-dir
  C:/code/audiorouter/src-tauri/target passed. Staged the final shell alongside
  the matching CLI/plugin worker from the preceding clipboard build and
  existing tunnel runtime/licenses at
  %TEMP%/audiorouter-group-layout-test/audiorouter-shell.exe.
  Shell SHA256: C22818302B0AA7A2F996BF6D3EB06ECA8FC558887776E0EA50D81F4AE2B98FEF.
  The original cache executable was restored and its SHA256 verified:
  EBDCA3FC1E11844DDAF87BFD4F938FA4CC9D0DE06D6EBE5FC07F97FCBCB90496.
  This is a manual review build, not a signed release or audio qualification.

Requirement evidence: UI-02/16 -> geometry and group/layout browser tests;
UI-11 -> modifier/marquee selection, keyboard nudge/undo and labelled controls;
UI-17 -> stable controls, local state, three-theme visual inspection. No backend
API, Rust source or audio callback changed. Existing driver/release gates are
unaffected and are not asserted by these UI checks.

Work is isolated on feature/canvas-group-layout, based on the already pushed
clipboard feature. Original C:/code/audiorouter user changes (including marketing,
Jev and website work) remain untouched, and .codex is never staged. Next task:
owner morning review, then PR/merge review; do not merge this branch automatically.
For existing groups, enable Move contents with group to try carrying behavior.


## Canvas node copy and paste — 2026-10-20 (owner request, in progress)

Objective: let users copy one configured canvas node and paste it as an
unconnected node, including between sessions in the same AudioRouter window.
Requirements: UI-02, UI-11 and UI-17. Preserve normal backend plan/commit
validation and local session layout ownership.

Decision: the app keeps a deep configuration snapshot in memory across session
switches. Pasting assigns a unique destination-session ID and copy name, copies
the node's settings and flags, and omits all graph edges and Duck's
`keyNodeId` side-chain association. Context menus provide Copy/Paste; Ctrl+C and
Ctrl+V act on the selected node/canvas and do not intercept editable fields.
Clipboard contents are window-local and are not persisted after app exit.

Prerequisites: UI and contract packages installed; inspect the current M05
canvas and layout ownership. No audio hardware is required. Windows Edge is
required for the themed canvas interaction run; jsdom covers data semantics
but does not establish visual behavior.

Tasks:
1. Add copy/paste draft helpers and regress configuration, flags, unique IDs,
   cross-session paste, detached graph edges and Duck references. Done.
2. Add canvas context menus and Ctrl+C/Ctrl+V. Pane paste is positioned at the
   pointer and clamped within the visible canvas; keyboard paste uses the
   session's next layout slot. Clipboard data survives session selection in
   this window. Done.
3. Update UI-02 and verify data semantics, TypeScript, lint/format and headed
   Edge behavior in dark, light and high-contrast themes. Done; results below.
4. Review the final diff and hand off the feature branch. Done.

Validation matrix: draft unit tests for same- and cross-session paste; focused
canvas/App tests for copy, paste, menu and shortcut behavior; `npm --prefix ui
run typecheck:all`, `npm --prefix ui run lint`, `npm --prefix ui run format`,
and targeted UI tests; headed Edge in the three themes for context menus and
keyboard paste. No Windows audio gate applies because this changes graph-edit
UI data only.

Risks: a copied node may carry a reference to another node through its
parameters; clear the known Duck side-chain reference and test it. Paste
remains an unsaved draft edit and is reversible through existing graph
undo/commit behavior. Rollback: remove the clipboard helpers, canvas menu and
shortcut bindings, and revert the UI-02 wording.

Evidence (2026-10-20, Windows host):
- `npm.cmd --prefix ui run format` and `npm.cmd --prefix ui run format:check`:
  clean.
- `npm.cmd --prefix ui run typecheck:all`: pass.
- `npm.cmd --prefix ui test`: 62 files, 549 tests passed.
- `npm.cmd --prefix ui run lint`: exit 0, 25 existing
  `react-hooks/exhaustive-deps` warnings; no new warning from copy/paste code.
- `npm.cmd --prefix ui run build`: pass; Vite retains its existing warning
  that the main JavaScript chunk exceeds 500 KB.
- `npm.cmd --prefix ui run e2e -- --grep "copy and paste a configured node
  across sessions"`: Playwright reported the targeted test passed (2.8 s).
  The local runner then remained in fixture teardown; it was interrupted after
  the passing result, and the Vite child was stopped. The assertion path had
  already verified shortcuts across a newly created session, both context
  menus, saved configuration, detached edges and in-canvas paste bounds.
  Dark/light/high-contrast screenshots are in the ignored temporary output
  directory `%TEMP%\audiorouter-playwright-results\canvas-copy-paste.pw.ts-co-6c9dc-ns-with-menus-and-shortcuts\`.
  Inspected all three theme screenshots; the context menu stayed in the canvas
  bounds, remained legible, and had a visible focus outline.
- `git diff --check`: pass. No audio device or Windows package gate applies.

### Owner-requested code review — 2026-10-20

Reviewed every feature diff and its selection, shortcut, layout, draft history
and session-switching callers. Implementation files were left unchanged during
review. Three P2 findings are confirmed in Windows Edge:

- Keyboard paste always sets `skipNextFitRef`, even when its next grid slot is
  outside the viewport. After panning right and pressing Ctrl+V, the new node
  started at screen x=1751.7 while the canvas ended at x=981.8. The node exists
  but is entirely hidden. Keep the current view only for a deliberate pointer
  paste, or place/bring keyboard paste into view.
- The global Ctrl+C listener checks editable elements but not selected UI text
  or canvas focus. Selecting header text and pressing Ctrl+C prevented the
  native copy and copied the previously selected tool instead.
- Clipboard shortcuts are outside the existing customizable shortcut conflict
  mechanism. With the saved session-toggle shortcut `Control+C`, selecting a
  tool and pressing Ctrl+C stopped the playing route instead of copying it.
  Resolve/reserve these bindings through the same shortcut dispatcher.

Review verification: full UI suite 62 files / 549 tests passed again;
`typecheck:all` and `git diff --check` passed. Fifteen existing Edge scenarios
passed, covering cross-session copy/paste, the three themes, dragging, visual
groups, session lifecycle/import/export, transactions, undo/revert and the
default playback shortcut. A separate focused probe confirmed non-default
gain preservation, paste undo/redo and editable-field shortcut exclusion.
The original copy/paste test plus that focused probe completed with exit 0
(2 passed). The three defect probes completed with exit 1, each failing at
its intended assertion. Two earlier audit attempts used a precise-gain field
absent from the browser fixture; those setup failures are not product findings.

The previous runner shutdown uncertainty is resolved: the same direct
Playwright/Vite command exited normally when run with Windows process-cleanup
permissions. The hang also reproduced without the real backend in the
sandbox, so it was not evidence of a copy/paste/backend teardown defect.
Temporary audit files were removed from the worktree. Probe source, temporary
config and failure screenshots/error contexts are preserved under
`%TEMP%\audiorouter-clipboard-review\`.

Owner requested commit and push of the tested feature on 2026-10-20, with the
three review findings recorded above still open. Next: fix those findings and
retain focused regression coverage before merging. Feature worktree:
`C:\Users\miste\AppData\Local\Temp\audiorouter-ts7-main`.
The original dirty worktree was not modified.

Manual test build (owner requested 2026-10-20): rebuilt the production UI and
the optimized native shell with `--locked --release --features custom-protocol`,
plus the matching workspace CLI and plugin worker. Staged these and the existing
OpenAI tunnel runtime/licenses at
`%TEMP%\audiorouter-copy-paste-test\audiorouter-shell.exe`.
Both native builds completed successfully. Used the existing build cache,
then restored the original three executables and verified their hashes against
the backups. The test app was not launched by the agent. It contains the current
feature and the three documented review findings; no fixes were applied.
## Application quality implementation - 2026-10-21

Owner approved the [quality roadmap](application-quality.md). Execute Q1-Q6
in small changes; Q1 begins with shared verification and merge gates.
Requirements ENG-01/02/04/05 first, then UI/QUAL/NFR gates in the roadmap.
Worktree was clean except the requested planning documents; unrelated original
C:/code/audiorouter user changes remain untouched. Branch quality/regression-gates
is based on main 5ff9de22. The reviewed group feature 6d9890e0 remains separate;
its product changes are not part of Q1 and no automatic feature merge occurs.

Initial audit: current account has repository admin access. Main had
no branch protection; ruleset 25451476 was disabled and only described deletion
and force pushes. The owner is the sole collaborator, so an independent formal
PR approval cannot be required without making owner-authored PRs unmergeable.
Require PRs and resolved threads, with owner review; record this limitation.
Latest successful full CI baseline: run 39543090377 at b4bbbef5 (main), about
32m37s total; Windows checks 31m09s, Edge 16m21s, docs 11s, dependency checks
33s. This is not evidence for the newer feature SHA. Cancelled runs remain
cancelled evidence, not passes. GitHub connector has no ruleset-management API;
use authenticated gh REST for that capability.

Tasks: Q1 shared classifier/command inventory, fast/full/guarded profiles,
classifier and aggregate negative tests, Stream Deck checks, reviewed metric
publication, then real PR CI and staged protection activation. Continue Q2
state invariants and Q3 production/packaged smoke; Q4 offline fault/contract
work and Q5 bounded reliability follow. Q4 hardware/endurance and Q6 actual
release install/rollback evidence require declared fixtures and isolated state.
Validation: node tests of policy/runner including intentional failures; actual
local checks with timing and full current PR CI; no skipped required job may
pass the aggregate. Preserve pinned toolchains and existing checks.
Rollback: revert each scoped quality commit; keep prior required protections
until a replacement gate is demonstrated. Back up rules before settings edits.
Risks: classifier under-testing, a green aggregate from skipped jobs, main
protection blocking the metrics bot, unavailable live endpoints, and a formal
review rule blocking the sole owner. Next: implement and test Q1 policy.

Q1 implementation evidence (Windows 11 x64, Node 22.22.3, 2026-10-21):
shared Node command inventory and runner now power local and CI checks. Fast,
full/change-scoped and explicitly guarded hardware profiles report actual
command exits and distinguish failed/blocked/not-run results. JSON evidence
includes dirty state so local edits are not misrepresented as commit evidence.
Classification tests include contracts' consumers, Stream Deck, unknown build
surfaces and invalid paths. The aggregate rejects missing, cancelled, failed or
unexpectedly skipped checks; real subprocess tests exercise nonzero exits.
Stream Deck now gets a conditional CI type/test/build job. Nightly metrics
produce a reviewed artifact at the measured SHA instead of pushing main.

Commands/results: `node --test tools/quality/policy.test.mjs
tools/quality/verify.test.mjs`: 50 passed. Shared `--profile fast --keep-going`:
8 checks passed (docs, policy, both Rust format checks, contract types,
UI format/lint/all-project types, Stream Deck types); 25 existing UI Hook
warnings, no lint errors. Evidence %TEMP%/audiorouter-quality-q1-fast.json
predates the final policy additions and dirty-state metadata. Shared
`--checks docs,policy,streamdeck-type,streamdeck-tests,streamdeck-build`:
all passed, including 26 Stream Deck tests; evidence
%TEMP%/audiorouter-quality-q1-tooling.json. Documentation: 181 files/993 links.
Official actionlint 1.7.12 Windows binary, archive verified against its published
SHA256 checksum: CI and nightly workflow validation passed. `git diff --check`
passed. No product code or real audio activity changed in Q1.

Q1 rollout evidence: [PR #50](https://github.com/MrDesjardins/audiorouter/pull/50)
is separate from the group feature. Its first clean full
[CI run 39710446637](https://github.com/MrDesjardins/audiorouter/actions/runs/39710446637)
passed at 0a7cd2af: all eight jobs including Windows/release preparation,
Edge, Stream Deck and Quality gate succeeded (28m45s total). Clean local fast
evidence at the same SHA: %TEMP%/audiorouter-quality-q1-clean.json, dirty=false,
all eight selected checks passed. Intentional format/type/unit/contract-drift/
classifier faults were all detected by real shared checks, then restored;
reports/logs: %TEMP%/audiorouter-q1-faults-zplj4b/.

After that demonstration, updated the branch to main 1be9ce3b (website/marketing
records preserved; resolved only the active-plan insertion conflict). Activated
ruleset 25451476 and verified effective main rules through the REST API:
PR required, blocking review threads resolved, Quality gate from GitHub Actions
integration 15368 required, strict latest-base checks, no bypass actors,
deletion/force-push restrictions retained. Independent approval count is zero
because the owner is the only collaborator; owner review remains an operational
requirement. Public settings evidence and rollback source:
[before](evidence/2026-10-21-quality-rules-before.json),
[active](evidence/2026-10-21-quality-rules-active.json),
[effective main rules](evidence/2026-10-21-quality-main-effective-rules.json).
No direct-push exception was granted to the nightly publisher. Its old main
workflow cannot push under these rules; PR #50 provides the reviewed artifact
replacement. The newly updated PR head must pass fresh CI before merge; the
earlier successful run does not qualify this later merge commit. Next: record
the new head's CI, demonstrate a docs-only declared-skip run and continue Q2/Q3
in their separate stacked branches. No PR is automatically merged.

Q1 follow-up review: shell Clippy inherited CI's omission of --all-features,
although AGENTS.md requires the same strict profile as the workspace. Verified
the all-feature shell check locally with current embedded UI: passed in 21.3s
(existing C:/code/audiorouter/src-tauri/target cache; no app launched/replaced).
Shared rust-clippy now builds current UI before both strict checks, and CI
installs locked UI dependencies before that step for backend changes too.
A policy regression pins the prerequisite/order and both all-feature flags.

Run 39713478156's dependency job failed on a registry HTTP 503 during Stream
Deck audit, after cargo-deny and the other npm audits passed. Retained log:
%TEMP%/audiorouter-quality-q1-supply-failure.log and its uploaded verification
artifact. A diagnostic local probe timed out on bulk audit then got 400 from
the deprecated quick fallback; it did not establish a dependency-tree defect
or a passing audit. The npm status page reports degraded install/publishing
since 02:33 UTC (Security Audit still lists operational), so attribution to a
broader registry incident remains an inference. GitHub refused to rerun the
completed job while the whole run was active. Do not waive or suppress it.
This source follow-up triggers fresh full CI; retain the failed/cancelled older
run as such. Next: require the final head's full check, including audit, to pass.

## Website: no localhost Swagger link, one header everywhere — 2026-10-21, owner request

Objective: the hosted API page must not link to `127.0.0.1` (that points at
each visitor's own PC), and the visual guides under `site/tools/` must show
the same header menu as the rest of the site.

- Decisions: the API page explains that the interactive Swagger view runs in
  the desktop app (API → Start API → Open Swagger documentation); the app's
  local Swagger flow is unchanged. `site/assets/site.js` resolves header links
  from the site root (the `site.js` script URL), so `generate-guides.mjs`
  emits `<div data-header></div>` and loads `../assets/site.js` instead of
  its own guide-only header; "Visual guides" is marked current on every
  `tools/` page. The latest-release lookup now runs only on pages that show
  release links, so browsing guides makes no GitHub API calls.
- Validation (Windows, Node 22.22.3): `node site/generate-guides.mjs`
  regenerated 36 guides plus the index; `node site/check-guides.mjs` passed
  (36 pages, 894 local links/assets); headless Edge screenshots of
  `tools/index.html` and `tools/gain.html` at 1280 px and 390 px (menu open),
  and `api.html`: same header, correct current item, links resolve to the
  site root, no horizontal scroll. The site has one (dark) theme.
- Follow-up (owner request): the header now stays at the top. Its
  `<div data-header>` wrapper was the sticky containing block, so it scrolled
  away on every page; `[data-header]{display:contents}` fixes that, and
  `html{scroll-padding-top:88px}` keeps anchor targets below it (manual
  sections' own margin drops 100 → 12 px; the guide TOC sticks at 104 px).
  Headless Edge: topbar at 0 after scrolling index, API, a guide and the
  390 px tools page (menu opens); `api.html#endpoints` lands at 91 px,
  `manual.html#api` at 127 px, a guide TOC jump at 116 px with the right
  item current. `site/README.md` documents a `uv` preview server (tested:
  200 for a guide and the catalog JSON).
- Rollback: revert the commits; no app or data change.
- Next: confirm the deployed pages after the Pages workflow runs.

## TypeScript 7 readiness — 2026-10-20, owner request

Objective: make every maintained TypeScript project typecheck with the latest
stable TypeScript compiler, currently 7.0.2, while keeping tools that need the
TypeScript compiler API on a compatible 6.x API.

- Requirements: ENG-02 and ENG-04.
- Prerequisites: PR #46 is merged. The TypeScript 7.0 announcement says 7.0
  has no programmatic API yet and recommends running TypeScript 7 beside
  TypeScript 6 for tools such as typescript-eslint; the current
  typescript-eslint dependency range is below 6.1.0. Evidence:
  [TypeScript 7 side-by-side guidance](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/#running-side-by-side-with-typescript-6-0),
  [typescript-eslint supported TypeScript versions](https://typescript-eslint.io/users/dependency-versions/),
  and the PR #48 CI typecheck failure at `contracts/src/index.ts:1205`.
- Decision: retain TypeScript 6.0.3 under the `typescript` package name for
  API-using tools, install TypeScript 7.0.2 as `typescript-7`, and invoke its
  compiler by explicit package path. Keep `tsc` scripts deterministic and
  typecheck the complete UI source, tests, end-to-end tests and configs.
- Ordered tasks: update this plan; add side-by-side compiler dependencies and
  explicit typecheck scripts to UI, contracts and Stream Deck; fix and regress
  the TypeScript 7 typed-array error; run TypeScript 7 across every TS/TSX
  file; run UI lint, formatting, unit tests, UI build, contracts drift check,
  and Stream Deck tests/build; inspect the diff; commit and push only these
  scoped changes to `main`.
- Validation matrix: `typecheck:all` in UI, contracts and Stream Deck under
  TypeScript 7.0.2; UI ESLint under TypeScript 6.0.3; UI unit tests and build;
  contract drift; Stream Deck tests and build; `git diff --check`; GitHub CI
  on the pushed main commit. No hardware gate is part of this tooling change.
- Risks: npm's binary link can select the TypeScript 6 compiler when both
  packages expose `tsc`; scripts will invoke the TypeScript 7 binary by path.
  TypeScript 7.1 prerelease behavior is outside the stable 7.0.2 target.
- Rollback: revert the dedicated TypeScript readiness commit; no runtime or
  persisted-data changes are intended.
- Evidence on 2026-10-20: TypeScript 7.0.2 passes for the full UI project
  (app, tests, end-to-end tests and configs), contracts, and Stream Deck.
  Expanding the UI check exposed existing errors in Playwright fixtures and
  configs; those are fixed and included in the full UI check. UI production
  build passed with the full check enabled; UI tests passed (547/547), format
  check passed, and ESLint passed with zero errors (25 existing hook warnings).
  Contracts drift passed (124 methods, 35 node kinds, 19 processors, 22 event
  categories). Stream Deck typecheck, 26 tests, and build passed. `git diff
  --check` passed. Commands were run on Windows with Node 22.22.3 in the
  isolated `codex/typescript-7-readiness` worktree based on merged `main`.
- Next: commit and push the scoped readiness change to `main`; inspect the
  resulting GitHub CI run before reporting the task complete.

## Jev paused — 2026-10-22

The owner asked to pause Jev because it is not providing enough value for the
time it adds. The workflow now supports manual dispatch only; pull-request
CI, formatter, test and Clippy checks remain unchanged. This is reversible by
restoring the `pull_request` trigger in
`.github/workflows/jev-review.yml`. No requirement or acceptance gate was
waived.

## Jev review scope

Objective: keep Jev focused on review findings that the configured compiler,
formatter, linter, or static analyzer cannot detect. Formatting remains owned
by rustfmt and the UI formatter; CI checks remain authoritative for their
configured diagnostics.

- Requirements: ENG-02 and ENG-04.
- Decision: Jev may report remaining correctness, safety, security,
  accessibility, concurrency, or performance risks. It must check the active
  tool configuration first and must not duplicate compiler/linter findings or
  comment on formatting and subjective style. Language-specific boundaries
  are recorded in each `jev/*.md` rule file; the duplicate Rust unsafe-comment
  rule was removed because strict Clippy already enforces it. The numeric cast
  rule now distinguishes narrowing integer casts from deliberate approximate
  integer-to-float metric formatting.
- Tasks: update rule boundaries and retain the preference as a validated
  lesson.
- Validation: inspect rule boundaries and the Clippy/rustfmt/UI lint workflow;
  run `git diff --check`. Re-run Jev through PR CI after pushing.
- Evidence: this entry, Jev rule files, and `AGENTS.md`.
- Risk: Jev may still miss context or overreport; use the configured tools and
  the concrete value range when triaging a future cast finding.
- Rollback: revert only the Jev policy and corresponding documentation.
- Next: allow PR Jev and AudioRouter CI to complete on the updated base.

## Dedicated audio thread for the multi-input worker — 2026-10-21, owner requested ("Go full")

Objective: requests and control-thread maintenance can no longer delay
audio processing. Today one thread (MMCSS "Pro Audio", measured base
priority 24, ~5 % of a core) both dispatches every request and pumps every
native worker, so a slow request stops all audio for its duration. Prevention
item (2) of the scratchy-voice defect below. Requirements NFR-06, ARCH-04,
the AGENTS.md audio-callback rule (never wait for UI, IPC, disk or
control-plane locks).

Evidence before the change (owner's live session, 133 min after a restart,
2026-10-21): longest gap between passes 7.6 ms (late threshold 8 ms), 2 slow
requests (`nativePaths.prepare`, 333 ms, at Play). So the change is
prevention; it is not expected to change a quiet session's numbers.

Scope: the multi-input worker only. It serves every route started from the
app (`nativePaths.prepare`). The single-endpoint (`nativeEndpoints.*`),
duplex and render-source workers and the cable route stay on the control
thread for now (legacy API and the unshipped cable); each can move later
with the same pattern.

Design:

1. **Audio thread.** The plane starts one `audiorouter-audio` thread (COM
   MTA, MMCSS "Pro Audio", 1 ms timer) the first time a multi-input worker
   starts, and keeps it for the process. Every 1 ms it calls
   `pump_and_process_outputs` (or `pump_available` when no output owner)
   on the shared worker. Nothing else runs there: no logging, no disk, no
   allocation in the steady state.
2. **Sharing.** The worker moves into a slot shared by both threads: a
   mutex around the worker plus atomics for `enabled`, pass statistics and
   the latest pump error code. The audio thread only `try_lock`s: when the
   control thread holds the slot, the pass is skipped and retried 1 ms
   later, so a control thread at normal priority can never block the
   audio thread (no priority inversion). The control thread holds the slot
   only for short worker calls (start, stop, binding taps, live graph
   updates, reading counters for diagnostics); a worker being released is
   taken out of the slot and dropped outside the lock.
3. **Thread safety.** WASAPI and MMDevice interfaces are free-threaded
   within the MTA, and both threads join the MTA, but the `windows` crate
   marks every COM interface `!Send`. A small wrapper asserts `Send` for the
   worker, with the invariants written at the `unsafe impl`: both threads
   are MTA, the mutex gives one thread access at a time, and no part of the
   worker is apartment-bound or reference-counted with `Rc`. Checked by
   reading every field the worker owns before relying on it.
4. **Gating.** The control thread keeps every check the pump made:
   capture-sink lease heartbeats, endpoint lifecycle invalidation, runtime
   state and generation. It sets `enabled` from them on each maintenance
   pass and synchronously at Stop, Play and rebinding; the audio thread
   pumps only while `enabled`. A pump error is counted, its category kept,
   and the control thread reacts as before (application inputs switched to
   silence via `maintain_multi_input_applications`).
5. **Control-thread maintenance stays where it is:** recorder draining
   (2 s queues), node recordings, network diagnostics, application probes,
   heartbeats. Its 1 ms loop is unchanged.
6. **Diagnostics.** `audioService` (passes, late gaps, max gap) and the
   `audioHealth` gap fields come from the audio thread; the record adds
   whether MMCSS was granted on it. The control-thread gap measurement
   becomes `controlMaxPassGapMs`, so a slow request still shows.

Order: wrapper and audio-thread host with unit tests (a fake pump: skip on
contention, enabled gating, error reporting, stop and drop outside the
lock) → move the multi-input slot and its ~36 call sites → diagnostics →
compile and the full test suite → live checks. Local builds run at
below-normal priority with limited jobs, and only when the owner agrees.

Validation: workspace tests and Clippy; `live_backend_service_keeps_a_routed_tone_continuous`
for the multi-input path (zero glitches on a clean reference); a stall test
that blocks the control thread for 500 ms with a request while a tone plays
and requires zero glitches (impossible before the change); muted start and
pump with a plugin and a Stats.cc Duck; then an attended session.
Rollback: revert the commit; no saved data or API change.

**Implemented (2026-10-21, branch `audio-thread`, not yet compiled
locally; CI compiles it).** `crates/control/src/audio_thread.rs` holds the
shared slot, the `Send` wrapper and the thread; the plane's
`native_multi_input_worker` is now a `MultiInputWorkerHandle` with the same
method names, so most call sites are unchanged (the network sender and
receiver, which returned references, go through `with`). Differences from
the design above:

- The thread starts with the backend service (`mark_audio_service_started`),
  not at the first Play, and only if it joins the COM MTA; otherwise, and
  in tests and the CLI, the control thread pumps inline as before.
- A handle drops only the worker it attached (attach number), so a stale
  handle can never release a newer worker.
- A panic on the audio thread stops its pumping and is re-raised on the
  control thread at the next pass, so the existing supervisor restarts the
  backend as it did for an inline panic.
- `nativeMultiInputs.pump` with the thread running applies the checks and
  returns zero new frames plus the worker's counters; its response (and
  every response carrying `audioService`) adds an `audioThread` object
  with the thread's counters.
- Diagnostics: the transport's `maxPassGapMs`/`maxPassMs` keep their names
  (they still measure the control thread); `audioHealth` gains an
  `audioThread` object (started, MMCSS granted, passes, late gaps, max gap,
  pump errors, passes skipped on contention, and interval maxima).
- The stall test already existed: `live_saved_session_tone_survives_starvation`
  with `AUDIOROUTER_STATIC_STALLS` suspends the control thread.

Validation, 2026-10-21 (owner's PC; AudioRouter and Discord closed at the
owner's go-ahead; release test builds at below-normal priority):

- Control crate: 261 tests passed (7 new in `audio_thread_tests.rs`);
  workspace Clippy `-D warnings` clean.
- Stall test, same day, same settings on `main` and on this branch: copy of
  the owner's database, session `patrick-main-siege-footstep`, tone helper
  on CABLE Input captured in place of Discord, result from the PD200X
  loopback, other outputs on Realtek Digital Output,
  `mixer-2/inputVolume:mixer-1=0`, control-thread stalls of 60, 90, 170,
  30, 120, 200 and 500 ms at 20–110 s, Stop/Play at 130 s, 150 s.

  | Run | Glitches during stalls | At Stop/Play | Silent samples |
  |---|---|---|---|
  | `main` | 18 (1–3 at every stall) | 4 | 68,063 |
  | branch, run 1 | 0 | 8 | 13,599 |
  | branch, run 2 | 0 | 10 | 14,111 |

  Run 1 also had one 30 ms burst of tiny residuals (0.05–0.14×) at 3.07 s
  that run 2 did not repeat (the same kind was seen once on old code,
  2026-10-19). Stop/Play glitches are expected; the higher count is noted,
  not explained.
- Muted start + pump (`live_native_paths_start_pump_and_report_signal_timing`
  with the new `AUDIOROUTER_LIVE_PATHS_AUDIO_THREAD=1`) on the same
  session: passed, 2,798 audio-thread passes in 4 s, no pump errors, every
  node (both Ducks, built-in Gate/Compressor/EQ, Pitch, Voice Changer)
  reported processing. The session has no VST plugins.

Next: CI green, then an attended session on a branch build, reading
`audioThread` in `backend.jsonl`, before merging.

## Cargo major updates by hand (supersedes Dependabot #41) — 2026-10-21, owner requested

#41 failed CI in `audiorouter-storage` (rusqlite 0.40 API) and paired
windows-core 0.100 with windows 0.62.2, whose COM types only implement
windows-core 0.62 traits (no windows 0.100 exists). Done on branch
`deps/cargo-major-2026-10`, without local compiles while the owner's
long-run observation of the scratchy-voice defect runs:

- rusqlite 0.32.1 → 0.40.2 (bundled SQLite via libsqlite3-sys 0.30 → 0.38).
  `DatabaseName::Main` became `MAIN_DB` (two backup calls). `u64`/`usize`
  `ToSql`/`FromSql` moved behind the `fallible_uint` feature, which is
  enabled: the same checked `i64` conversion as 0.32, so no stored value
  changes.
- tungstenite 0.26.2 → 0.30.0 (Stats.cc round feed client): changelog shows
  no API change for the client calls used; 0.30 only tightens server-side
  key checks.
- getrandom 0.3.4 → 0.4.3 (shell tokens and nonces): `fill` unchanged.
- windows-core stays 0.62.2; `.github/dependabot.yml` now ignores its major
  updates until a matching windows release exists.

Checks so far: both `cargo fmt -- --check` clean; `cargo deny check` (root
and `src-tauri`) advisories, bans, licenses and sources ok; lockfiles change
only these three crates and their dependencies. Compile, Clippy and tests:
Windows CI on the branch. Still to do before merging: open a copy of the
owner's session database with the new SQLite (sessions list, a backup) and
a muted start + pump of a Stats.cc Duck route, both when the owner is not in
a call. Rollback: revert the commit; no schema or file format change.

## Release 0.0.20 — 2026-10-19, owner requested ("potentially we should do a 0.0.20 since it fixes the issue with the buffer increasing")

**Published** 2026-10-20 04:38 UTC as an experimental prerelease from tag
`v0.0.20` (`a5850856`), same app-only profile as 0.0.19. Contents: inputs
skip stale capture packets after a stall (`abad73bf`, see "Backlog
recovery after stalls" below). All checks green before tagging (workspace
1,137 / shell 83 tests, UI 547, Edge 281, CI 39367752979 on branch
`release/0.0.20`); exact-package fresh install passed; draft assets
verified and byte-identical. Record: [release 0.0.20](evidence/2026-10-19-release-0.0.20.md).

**Dependency updates after the tag** (owner request, 2026-10-19): merged
Dependabot #28–#30; sha2 0.11/hmac 0.13, symphonia 0.6, vite 8/jsdom 30
upgraded by hand (`b0249e3a`, `ad055257`), superseding the other major
PRs; Jev skips Dependabot PRs. Auto-merge (owner chose it): Actions and
npm minor/patch plus Cargo patch merge after CI through
`dependabot-auto-merge.yml`; Cargo minor and all majors stay manual
(grouped per ecosystem). jsdom 30 needs undici 8.12.3 (`node >=22.19.0`),
so every vitest worker failed on CI's Node 22.14.0 (PR #40, run
39369458528) while the PC's 22.22.3 passed; the workflow pins are now
22.22.3. Next: #41 (cargo-major group) needs a manual upgrade and the
live continuity check. #40 auto-merged on 2026-10-20 (`fc6df9ab`) after
its CI passed on Node 22.22.3; the dispatched main CI 39372710384 passed.
Rollback: 0.0.19 opens the same sessions; no migration; never replace
published assets or move the tag.

## Release 0.0.19 — 2026-10-18, owner requested ("Can we have a new release with the new tool we did?")

**Published** 2026-10-19 05:03 UTC as an experimental prerelease from tag
`v0.0.19` (`8190fd32`), same app-only profile as 0.0.18. Contents: Fun
Voice Changer (DSP-20), Duck "In a round, playing", once-a-minute
`audioHealth` log records; known issue: lasting static after heavy load
(below). All checks green before tagging (workspace 1,137 / shell 83 tests,
UI 547, full Edge 281, CI 39196571264); three load-sensitive tests fixed on
the way (guide page walk timeout, undo race, Gate autosave plan count).
Exact-package `fresh_install_shell` passed (339 ms); draft and public assets
verified. Record: [release 0.0.19](evidence/2026-10-18-release-0.0.19.md).
Rollback: keep 0.0.18 and backups taken before using the new tool or Duck
option (0.0.18 rejects those sessions); no migration; never replace
published assets or move the tag. Next: owner installs 0.0.19.

## Fun Voice Changer tool — 2026-10-18, owner requested ("lets build it now") (in progress)

Promoted from [future/voice-effects.md](../future/voice-effects.md), which
holds the full design (signal chain, 36 parameters, 14 presets, runtime
state registry, test list T1–T8, risks). New requirement DSP-20; one new
`NodeKind::FunVoiceChanger`, existing tools unchanged. Owner decisions:
single tool with tweakable presets, approved preset set, ≤ 25 ms added
delay. Implementation finding: `microfft`'s `rfft_2048` compiles with the
existing `size-1024` feature but panics at run time (its twiddle table is
one entry short), so `crates/dsp/Cargo.toml` now enables `size-2048`
(additive; existing tools unchanged, all 80 DSP tests pass).

Progress: `crates/dsp/src/voice_changer.rs` done: McLeod pitch tracker,
pitch correction with note hysteresis and humanize, TD-PSOLA with grain
resampling for formants, ring modulator, bit crusher, saturation, LPC
whisper, tone filters, chorus/flanger, 8-line FDN space, existing
`PeakLimiter` as the −1 dBFS safety limiter, fixed 1,024-sample delay. 15
offline tests pass, including detection within 1 % (90–400 Hz), shifts within
10 cents (±12 st), correction within 10 cents, no note warble at a boundary,
formant peak moved by 2^(±5/12) within 8 %, exact neutral passthrough,
constant delay, hostile input finite and ≤ −1 dBFS, click-free preset change.
Lessons from the tests: PSOLA output is normalised by a 10 ms average of the
window overlap (an instantaneous divide reshaped the waveform and cancelled
the formant shift upward); a perfectly periodic vowel's glottal pulse
dominates its spectrum, so formant movement is tested on a single
resonance.

Implemented end to end (2026-10-18), with two deviations from the design:
- State handoff: not a global (session, node) registry like Time Shift's.
  Independently compiled graphs of the same session would share one
  processor (tests and any concurrent compile). Instead a live replacement
  carries the processor by node: `RealtimeMixerFanout::replace_paths`
  (every input, output and processing chain) and
  `RuntimePublication::publish`. A graph compiled on its own, or a node back
  from bypass/disable/Stop, starts fresh, so no reset flag is needed.
  Telemetry follows the existing per-node accessor chain.
- Parameter table and presets live in `crates/domain/src/voice_changer.rs`
  (domain does not depend on dsp); validation, the catalog schema,
  `presets.voiceChanger` and the engine all read it. Five presets carry an
  `outputDb` so every preset is within ±3 dB of the input on a synthetic
  voice (robot +7, alien +5, whisper +8, radio −3, megaphone −3).

Files: `crates/dsp/src/voice_changer.rs`, `crates/domain/src/voice_changer.rs`
and `lib.rs` (kind, registry, latency 1,024, validation), engine stage and
carry (`crates/engine/src/lib.rs`), catalog/schema/simple/cable routes/
telemetry (`crates/control`), CLI list, contracts, UI (`draft.ts`,
`library.ts`, `parameterText.ts`, `FunVoiceChangerEditor.tsx` with tests,
`NodeEditors.tsx` `only` filter, canvas/workbench/guide link tables,
`styles.css`), shell guide slug, site guide (content, tutorial, synthetic
sung-voice demo, catalog), spec DSP-20, API reference and quickstart.

Validation (Windows host, 2026-10-18): DSP 15 voice-changer tests (80 in the
crate); domain 3 new; engine `tests/voice_changer.rs` 6 (fixed delay, live
preset change without dropout and a fresh-processor control, no stale audio
after bypass, independent compiles isolated, presets within ±3 dB,
telemetry) plus the kind in `tool_combinations`, `tool_routes`,
`cable_route`, bypass-null and the bench; workspace and `src-tauri` Clippy
`-D warnings` and fmt clean; control 254, shell 83, UI 547 unit tests;
ESLint 0 errors, Prettier clean; Edge `fun-voice-changer` 4 (three themes
reviewed, no layout shift on edit, Reset, Custom); `site/check-guides.mjs`
36 guides; Firefox guide audio sweep for the new guide OK. Full Edge suite
(2 workers, below-normal priority, while the owner played): first run 3
failures, two stale for the new tool (`tool-workflows` drove the preset
select as a generic field, now skipped there since a preset rewrites every
setting; guide index count 35 → 36) and one load flake in `audio-tools`
(passed alone); rerun 281 passed, 9 skipped. Review build
`target/reviews/voice-changer-20261018/release/audiorouter-shell.exe`
(incremental from a copy of `audio-health-20261018`, `-j 6` below normal;
`ui/dist` rebuilt first and its bundle name `index-Y_5EWqXj.js` found in the
exe). Owner tried the review build on 2026-10-18: "Seems to work" (first
impression; no preset tuning feedback yet). Not done yet: the
sine-continuity harness through the tool on a live route (needs the owner's
route stopped), and the owner's attended listening and preset tuning (T8).

Order: DSP core with offline tests → domain/engine/control → UI → site and
docs → live checks and a review build (owner's shell is not stopped) →
attended preset tuning. Rollback: remove the kind; saved sessions holding it
would fail validation in builds without it.

## Scratchy Discord voices after an overnight run — 2026-10-18, owner report (open)

Report: after ~15 h of playing (review build `duck-in-round-20261017`,
session `patrick-main-siege-footstep`, window open, Siege off), people in
Discord sounded very scratchy. Stop/Play and the window's Reconnect did not
help; killing the shell and starting it again did. Requirements NFR-06
(long-run stability), OPS logging.

Evidence gathered (Windows host, 2026-10-18, on the restarted app):
- `backend.jsonl` for the overnight process (PID 50928, 20:10→11:02): no
  errors, no restarts, no device changes; edits until 20:22, then only the
  quit. The log held no audio-health data at all, so it cannot show when or
  where audio was lost.
- Clock drift ruled out: a scratch shared-mode probe (packet device position
  vs QPC, 120 s) measured mic, CABLE, CABLE-A, CABLE-B and Discord's process
  loopback within 0.2 ppm, Scarlett within 1–3 ppm (<0.1 s over 15 h).
- Discord did not restart overnight (running since 2026-10-17 13:17).
- The round Duck holds the last Stats.cc phase; no round overnight.
- Fresh run: requests ≤1 ms each, backend thread 1.3 % busy (35 req/s),
  shell memory/handles and WebView2 memory flat over minutes, diagnostics
  payload fixed at 13.6 KB. `devices.list` ~10 ms, `sessions.list` 9–26 ms
  (each blocks the audio thread; the 8 ms late-gap threshold).
- Stop/Play rebuilds the whole native worker, so the bad state survives
  outside it (control thread, process-wide state, or the WebView).
- Found: the backend audio pass discarded multi-input pump errors
  (`let _ =`); a persistently failing input would end every pass before the
  outputs drain, unseen.

Change (observability, not yet a fix): a once-a-minute `audioHealth`
record in `backend.jsonl` while a native route plays (written by the logger
thread): longest gap between service passes, longest pass, slowest request
and its method, requests over 8 ms, pump error count and last category, per
input pickup delay and `DATA_DISCONTINUITY` packets, per physical output
queued ms, underruns and blocks dropped at a full ring (newly counted in
`AudioBlockRing::full_drops`). Tests: logger thread writes the record
(`backend_logger_thread_writes_commits_and_skips_routine_read_successes`),
full-ring drops counted (`mixer_fanout_runs_bound_plugin_before_every_branch`),
discontinuities counted (`multi_input_feeder_submits_coherent_sources_to_both_outputs`),
error categories drop private text, no record while nothing plays.

Reproduced 2026-10-18 ~17:00 (owner in a Discord call, Siege running,
review build `audio-health-20261018`, PID 50896):
- Trigger: CPU starvation. From 16:30, only in minutes when the agent was
  compiling/testing (all cores busy), `system.diagnostics` took 15–170 ms
  (normally ~1 ms) and the longest service-pass gap matched it; outputs
  logged underruns and full-ring drops, Discord's process-loopback capture
  (`application-capture-1`) lost nothing. The audio pump shares the
  normal-priority control thread, so CPU load starves it.
- Lasting fault: after the builds stopped, counters were flat again (no
  late gaps, underruns or drops) yet the owner still heard static on
  Discord voices. Stop/Play reset every delay to normal (Discord pickup
  22.6 → 10.9 ms, Siege 98 → 12.5 ms) but not the static. Quitting and
  restarting the shell fixed it, as overnight.
- Located: a 10.6 s Recorder take of `mixer-3` (Discord capture + raw mic,
  before `duck-2` → `mixer-2` → `voice-monitor`) had no clicks, gaps or
  clipping (one noise burst was an "s", 4–8 kHz). So the static is added on
  the output side, by state that survives Stop/Play but not a process
  restart. Ruled out: Duck gain carry and `NodeLevel` (weak entries freed on
  Stop), `RingOutputPump` (per worker), Siege phase flapping (steady
  `action`), clipping (no clipped samples on the path).

Offline reproduction attempt (2026-10-18 evening, owner's route stopped,
debug harness build): `live_backend_service_keeps_a_routed_tone_continuous`
gained `AUDIOROUTER_CONTINUITY_SOURCE=selfApplication` (a helper copy of the
test binary plays the tone and the route application-captures it; capturing
the test process itself fed the route's own output back and produced
regular false glitches), `AUDIOROUTER_CONTINUITY_STRESS=<start>,<s>,<threads>`
(normal-priority busy threads) and `AUDIOROUTER_CONTINUITY_RESTART_AT=<s>`
(Stop, detach, prepare, start, as the UI's Play), plus glitches per 5 s.
CABLE-B Input is now 8-channel (Siege surround), so the run used tone →
CABLE-B Input, route app capture → Duck → Mixer (+ silent Test Signal) →
CABLE Input, result from CABLE Output. Baseline 30 s: 0 glitches. 150 s with
48 busy threads (24 logical CPUs) at 20–65 s and Stop/Play at 85 s: 6
glitches plus short silent runs during the load, 2 at the restart, none
before or after. The lasting fault did not reproduce on this simple route.
Not yet covered: several sources on different clocks (the 8-channel
`siege-in` with surround-to-headphones), a round-mode Duck, connected Mixers
and the Spectral Gate. Note `siege-in` pickup sat at ~98 ms from 12:28 until
the owner's Stop/Play, after a 31 ms `applications.list` stall, so a short
stall can leave an input in a lasting state.

Code and log review (2026-10-18 evening, no audio run):
- The incident process (PID 50896) logged a clean lifecycle: one prepare at
  11:57, the owner's Stop/Play at 17:05 (stop, detach, prepare, start), a
  Recorder take, quit. No errors, pump errors or extra prepares.
- Its `audioHealth` series: Discord's capture had 0 discontinuities and a
  steady ~11 ms pickup throughout; mic and `siege-in` discontinuities and
  all three outputs' underruns/full drops rose only in the minutes with slow
  requests (16:30–17:01).
- The whole route is one connected component (Discord feeds `mixer-3`,
  `duck-2` and `mixer-2` directly), so `connected_mixers` compiles it as a
  single path. Both Discord copies into `mixer-2` are summed inside one
  graph pass, so path-to-path drift cannot misalign them.
- The control/audio thread already enters MMCSS "Pro Audio" and 1 ms timer
  resolution (`AudioServiceThreadGuard`, `transport/src/lib.rs`); the live
  shell's busiest thread runs at priority 24. The "own MMCSS thread"
  prevention idea is therefore already in place. A normal-priority build
  cannot preempt it, so the 15–170 ms request stalls more likely came from
  memory pressure (page faults on a trimmed working set: that evening an
  Edge run was killed for low memory) than from CPU priority. Unproven.
- Process-wide engine state is limited to the Time Shift, `NodeLevel`
  (including `<duck>::duck-gain`) and Siege round registries, all weak
  entries freed when the graph drops on Stop; no drift or resampler state is
  remembered per endpoint.

Saved-session reproduction (2026-10-18 ~20:10, owner's app quit, nobody in
a call): new `live_saved_session_tone_survives_starvation` runs the owner's
`patrick-main-siege-footstep` from a database copy in a release harness
build, retargets `application-capture-1` at a tone helper process and
`voice-monitor` at the PD200X headphone output (Firefox was playing on the
Focusrite, which polluted its loopback), and records that output's loopback.
Baseline 30 s: 0 glitches, tone at the expected 0.074 peak (41 % direct plus
Duck 2). 240 s run with 48 busy threads plus a 14 GB memory hog
(30–150 s) and Stop/Play at 200 s: a 22 ms gap at the stress start, one
small glitch during it, a 63 ms gap at 179 s followed by ~0.5 s of ±40–50 Hz
sidebands, the Stop/Play gap; clean everywhere else, including the 30 s
after the stress and after Stop/Play. No lasting fault. Second run, 600 s
beside the afternoon's real trigger (a clean release build of the workspace
and shell in a scratch target dir, then the UI unit tests, from t≈30 s to
~3.5 min) and Stop/Play at 560 s: glitches only in the first 15 s of the
build (52 ms gap, slowest `system.diagnostics` 54 ms) and at Stop/Play,
small level bumps at 478–525 s consistent with room sound through the live
mic chain, clean otherwise. No lasting fault. So the lasting static needs
something this copy lacks: real Discord as the captured process, the
Focusrite render endpoint, a live call, or a much longer intermittent load.

Code and log re-analysis (2026-10-19, no audio run, owner on the
`db7a784e` local build): every piece of audio state (graph, DSP, rings,
WASAPI clients, capture and render workers) is rebuilt by Stop/Play; the
only state that outlives it is the control/audio thread, the control
plane, the registries above and the UI. Two facts weaken "survives
Stop/Play": the load continued after the 17:05 Stop/Play (126 ms gap at
17:07, `siege-in` 12.5 → 65 ms), and the clean `mixer-3` take ran at
17:08, after the Stop/Play had put Discord's pickup back to 10.9 ms.
Leading hypothesis (unproven): the multi-input feeder is lockstep
(`stage_path_inputs` mixes only when every input has a block) with no
drift correction or backlog trimming, so a stall leaves each input with a
lasting backlog in its WASAPI capture buffer (the health log shows new
plateaus after every stall: Discord 10.7 → 22.4 ms at 17:01, `siege-in`
pinned at ~100 ms, the 100 ms polling buffer). An input pinned at a full
buffer loses audio inside Windows on every jitter or burst. Discord's
process-loopback stream never set `DATA_DISCONTINUITY`, even across
170 ms stalls that its small event-driven buffer cannot have survived, so
those losses are invisible to every counter, and they are only audible
while someone speaks: static on Discord voices. Prediction to test: under
the static, Discord's `waitMs` sits above its ~11 ms baseline; a
harness stall (inject a 100–200 ms sleep in the service pass) followed by
a bursty application source shows lasting drops at the capture while all
counters stay flat. Fix direction if confirmed: trim each input's backlog
back to a target after a stall (discard whole packets at a quiet point,
counted), and count capture frames against the QPC clock to expose losses
Windows does not flag. The DSP thread sets no FTZ/DAZ and `RealtimeDsp`
busy-clears are not counted; neither fits the flat `maxPassMs` (1–5 ms).

Stall injection (2026-10-19, owner's app running, owner consented): the
saved-session harness gained `AUDIOROUTER_STATIC_STALLS=<at s>:<ms>,...`
(suspends the audio service thread), `AUDIOROUTER_STATIC_SPARE_RENDER_ID`
(other outputs to the unused Realtek digital output) and
`AUDIOROUTER_STATIC_SET=<node>/<parameter>=<n>` (here `mixer-2`'s
`mixer-1` input at 0, so only the tone path is measured). Release test
build, database copy `static-db`, tone helper → process loopback, result
from the PD200X headphone loopback. Run 1 (tone on Realtek; stalls 150,
200, 120 ms; Stop/Play at 150 s) and run 2 (tone on CABLE Input like
Discord; stalls 60, 90, 170, 30, 120, 200 ms; Stop/Play at 145 s): the
stalls reproduce the incident's lasting plateaus exactly (`siege-in`
11.6 → 17 → 79 → 89 ms, output queues 15 → 40–48 ms, held until
Stop/Play, which restored 11.6/15.5 ms), but the application tone stayed
at 10.6 ms pickup and clean between stalls: glitches only at each stall
(2–3 per stall), 0 lasting. One unexplained 30 ms burst of tiny residuals
(0.06×) at 79.0 s in run 2, before any stall. So: lasting latency after a
stall is reproduced and is a real defect (no backlog trimming), the
Discord-at-cap static is not; the application input appears to pace the
graph here, while in the incident Discord's pickup itself plateaued
(22 ms), which needs a real bursty or starved Discord. Not yet measured:
the mic and Siege paths (muted in these runs), where an input pinned near
its 100 ms polling buffer could lose audio.

Recurrence (2026-10-19, local build of `db7a784e`, PID 25792): the owner
heard crackle from about 19:03. YouTube in the browser, played straight
to the default device (Focusrite USB Audio) and not through AudioRouter,
crackled too, and kept crackling after Stop in AudioRouter. At 19:08:
audiodg 0.6 % of a core, AudioRouter 7 %, total CPU 22 %, DPC 0.9 %,
interrupt 0.4 %; no System-log USB, audio, PnP or WHEA events in the
previous 90 minutes; every Focusrite device reported OK. `audioHealth`
counters had been flat since 18:52 (siege-in pickup about 97 ms since a
15:42–15:52 series of 80–85 ms service gaps, Discord 11 ms). So this
occurrence is below AudioRouter (Focusrite device, driver or Windows
endpoint), and the earlier "clean `mixer-3` take" could not have shown it,
since `mixer-3` never passes through the Focusrite. The Siege backlog
plateau is still a separate real defect. Fix: changing the Focusrite's
format in Sound settings → Properties → Advanced (and back) cleared the
crackle at once, without unplugging; the endpoint then stored 48 kHz
24-bit stereo (engine mix format 48 kHz 32-bit). A format change restarts
the endpoint's audio engine and the device stream, so the fault lives in
that stream (driver or device clock state), not in AudioRouter's graph.
When it recurs, before resetting: note Focusrite Control's sample rate and
buffer size versus Windows', try YouTube with AudioRouter stopped, and run
LatencyMon for a minute.

Backlog recovery after stalls (owner-approved 2026-10-19, "Yes please
do"). Defect: after a service stall, a multi-input route keeps the extra
delay until Stop/Play (reproduced by `AUDIOROUTER_STATIC_STALLS`; live
siege-in 97 ms and outputs 26–50 ms). Two causes: (a) the lockstep feeder
reads each capture at the graph's rate, so unread packets left in an
input's WASAPI buffer never drain; (b) every underrun deepens the render
cushion by 5 ms up to 40 ms and it never shrinks, and nothing removes
audio already queued past it.

Implemented (uncommitted): `AudioCaptureSource::has_queued_packet`
(physical, binaural and process-loopback captures; `GetNextPacketSize`).
`WasapiMultiInputFanout::pump_inputs` skips a packet older than 40 ms at
pickup (`INPUT_BACKLOG_TRIM_AGE_100NS`) while a newer one is queued, and
keeps skipping until none is (never the newest), counted per input as
`trimmedPackets` in `audioHealth`. Regression in
`multi_input_feeder_submits_coherent_sources_to_both_outputs`.

Stall harness (run-2 settings: tone on CABLE Input, stalls 60/90/170/30/
120/200 ms, Stop/Play at 145 s; logs `stall-run3…8.log` in the session
scratchpad), run beside the owner's live app with Firefox and Discord
busy:

| Run | Change | siege-in after stalls | Glitches |
|---|---|---|---|
| 6 | trimming off (A/B) | 20 → 32 → 85 → 95 → 105 ms | 23 |
| 3 | inputs + output trim + cushion decay | 20 ms | 43 |
| 4 | inputs + output trim | 20 ms | 47 (bursts at 47, 87, 112 s) |
| 5 | inputs only | 20 ms | 133 (83 after Stop/Play, no stall) |
| 7 | inputs, no app loopback | 20 ms | 23; app 11 → 18 → 28 ms, lasting |
| 8 | inputs incl. app loopback (final) | 20 ms | 32, at or beside stalls |

Rejected: output trimming at cushion + 20 ms cut healthy audio, because
CABLE-A's ordinary queue (about 32 ms) already exceeds it; each device
buffers differently. Timed cushion decay dropped audio and caused fresh
underruns about 60 s after the last one. Excluding app loopback moved the
backlog onto the app input instead. Run 5's burst did not recur in run 8
with the same code, so it is attributed to load (not proven). Output
queues still keep their deepened cushion (up to 40 ms after repeated
underruns) until Stop/Play; a later fix would need inaudible catch-up
(slight resampling), not block drops. Checks: fmt (workspace, shell),
strict Clippy, workspace tests 1,138 passed / 0 failed / 29 ignored
(before the last two narrowing edits), then windows-audio, control and
transport 429 passed / 0 failed. Rollback: revert the commit. Full
record: [lasting static and stall backlog](evidence/2026-10-19-lasting-static-and-stall-backlog.md).

Next: (0) when the static is heard again, before any restart: record
`mixer-2`'s output and a loopback of the Focusrite endpoint at the same time
(engine clean + device dirty = render/OS side; both dirty = engine), and
bypass `duck-2` briefly; log page faults and working set in `audioHealth`.
(1) reproduce offline, without the owner: route a test tone (or the
sine-continuity harness) through an application-capture → Duck → Mixer →
output path, saturate the CPU for a few minutes, stop the load, and check
whether the output stays corrupted after Stop/Play; diff process-wide state
(audio service thread, timer resolution, WASAPI sessions, static
registries). (2) Prevention regardless of the root cause: run the audio
service on its own thread with MMCSS "Pro Audio" priority so requests and
CPU load cannot starve it. (3) Agent rule meanwhile: no heavy builds while
the owner is in a call on the running app. Rollback: revert; the records are
additive.

## Duck: "In a round, playing" phase — 2026-10-17, owner requested

Owner: "The Duck tool for Siege should have one option to Duck when In a
Round Playing." Requirement DSP-19. A round-following Duck could only turn
audio down outside rounds (menu, preparation, between rounds); `action`
always released. New boolean `duckAction` (default **false**, so saved Ducks
keep their behavior): engine mask bit `ROUND_DUCK_ACTION`, domain
validation, catalog/schema, Properties checkbox "In a round, playing"
(listed in game order), spec 07, quickstart and the Duck website guide.
Unknown, stale or disconnected feed state still releases.

Validation (Windows host): `round_duck_follows_…` extended with an
action-only Duck (ducks in action; releases in prep, menu, stale and
unknown); workspace tests passed except one unrelated
`sender_clock_drift_is_absorbed…` timing failure, which passed on rerun;
Clippy `-D warnings` and fmt clean; UI 541 unit tests, ESLint 0 errors,
Prettier clean; Edge `duck-siege-round`, `duck-widget`, `tool-workflows`
36 passed; dark/light/high-contrast screenshots reviewed; contract drift,
`site/check-guides.mjs` and docs validation passed. Not done: an attended
match with Stats.cc. Rollback: revert; a saved `duckAction` would then be
rejected by validation, so untick it before downgrading.

## Release 0.0.18 — 2026-10-17, owner requested ("publish a new version with all these fixes")

**Published** 2026-10-17 19:55 UTC as an experimental prerelease from tag
`v0.0.18` (`54b948f1`), same app-only profile as 0.0.17. Contents: several
custom themes, steady sidebar tabs, coloured node status, public guide
links, gated Virtual Cables panel, cable route reconnect, cable status
cache, lighter backend logging. All checks green before tagging (workspace
1,110 / shell 83 tests, full Edge 275, CI 39062192939); exact-package
`fresh_install_shell` passed (303 ms); draft and public assets verified.
Record: [release 0.0.18](evidence/2026-10-17-release-0.0.18.md).
Rollback: keep 0.0.17 and backups; no migration; never replace published
assets or move the tag. Next: owner installs 0.0.18. The silent Bass & Treble guide was a
Firefox-only website defect, fixed after the release (website only).

## Tool guide animations follow the audio — 2026-10-17, owner requested

Owner findings: the playhead and the sound did not match, Visual Group
animated although it is only visual, changing a setting (Gain) stopped the
audio, and examples should loop under one Play/Stop button.

Causes and changes (`site/assets/guide-audio.js`, `guide-animation.js`):
- Pictures were schematic (fixed humps, a Duck "control" band at 2.4–5.8 s
  while the trigger voice is heard at 1.1–3.3 s and 4.6–7.3 s, Delay drawn in
  a one-second window), yet a playhead swept them over the 8 s audio. Time-axis
  pictures (Gain, Volume, Compressor, Limiter, Gate, Duck, Delay, Audio File)
  now draw IN/OUT from the rendered example; Duck's band comes from where its
  output is ducked; Limiter shows its real ceiling around the output; the
  linear-scale threshold lines that sat on the baseline (Compressor, Gate)
  were removed. Diagrams without a time axis (EQs, FIR, Dehum, Input Switch,
  Pitch, Tone, routing, Group) have no playhead. Time Shift's pointer follows
  the example (live, history 2–6 s, live); Network pairing aligns with 2.4 s.
- Group and Physical Input have no motion at all and no transport.
- A setting change rebuilt the picture through `seek`/`play`, which stopped
  the audio source until a fresh render. The picture now restores its
  position without touching audio, and the audio swaps to the new render at
  the same position with a 30 ms crossfade. Input Switch re-renders the
  selected input for the whole loop once its fade finishes.
- Examples loop (`AudioBufferSourceNode.loop`, infinite WAAPI iterations)
  under "Play example"/"Stop example" ("Play/Stop animation" when silent);
  Stop returns to the start. Replay was removed. Recorder keeps its once-only
  take flow.

Validation (Windows host, Edge): `ui/e2e/site-guides.pw.ts` 8 passed,
including a new test (gain change starts exactly one crossfaded looping
source and keeps playing; wrap at 7.8 s; Stop; Duck band at the voice
onsets; Group static); `site/check-guides.mjs` passed; Prettier clean.
Screenshots of Gain, Volume, Compressor, Limiter, Gate, Duck, Delay, Audio
File, Time Shift, Network Receive, Input Switch, Plugin and Group reviewed.
Not done: listening test by ear.

Follow-up the same day (owner: "Bass-treble does not have any audio … check
every tool"). Bass & Treble was not reproduced as silent: an output-tap sweep
in Edge with the default autoplay policy measured audible output on every
audible guide, locally and on the published site, including while dragging a
slider and after scrolling to the last slider. Review of all 35 pictures
found and fixed: footnote overlapping the bottom caption (Bass & Treble,
Advanced EQ, Declick, Spectral Gate), Meter "dBFS" over the +6 label, Speech
Denoise output label on its waveform, Recorder stage names outside their
boxes, Pitch wrongly switched to real-audio traces (it keeps its schematic
cycles; only time-axis pictures use audio traces), routing packets sliding
under the boxes (now on the wires), and the page note promising a "moving
playhead" on pictures without one (generator now words it per picture kind;
29 pages regenerated, one line each). New Edge regressions: every audible
guide must reach the output above 0.01 peak after Play and above 0.001 after
a control change; no guide picture may have overlapping or clipped labels
(Network stage swap excepted). `site-guides.pw.ts` 10 passed. Resolved the same day: the owner's default browser is Firefox 158
(the earlier sweeps ran only Edge). `filtered()` in `guide-signal.js`
assigned the input `AudioBuffer` to its source before filling it; Firefox
takes the contents at assignment, so Bass & Treble, Graphic EQ, Advanced EQ,
Dehum and FIR Filter rendered silence there. Driving stock Firefox over
WebDriver BiDi with a real click: the published site had exactly those 5
silent (peak 0.000), the fix plays all 27 audible guides and keeps them
playing through a control change; no label overlaps in Firefox either.
`tools/site/firefox-guide-audio.mjs` keeps that check; Edge
`site-guides.pw.ts` 10 passed after the fix.

## Workbench polish, several custom themes, cable section gated — 2026-10-17, owner requested

Requirements UI-17 (no layout shift), UI-10 status colours, spec 17 §7.4
(cable panel), DIST (app-only releases ship no cable driver).

1. **Sidebar tabs moved.** On pages long enough to scroll, the desktop
   scrollbar (~10 px) narrowed the tab grid, so at some sidebar widths
   (reproduced in headed Edge at 420–428 px) "UI Options" wrapped on Tools
   but not on Properties and the whole tab row changed height. Fix: the
   sidebar reserves its scrollbar gutter (`scrollbar-gutter: stable`) and
   every tab button has a fixed two-line height. Headless Edge overlays
   scrollbars, so the regression asserts the gutter and a single button
   height across tabs at 404/428/520 px; it fails on the old CSS.
2. **Help links opened local files.** Repository builds opened
   `site/tools/<tool>.html` from the checkout. `tool_guides::destination`
   now always returns `https://audiorouter.org/tools/<slug>.html`.
3. **Selected-node status was grey.** The Properties status now uses the
   canvas `node-state` tones (Active/Ready green, Bypass amber, Failed and
   Disconnected red, Off neutral) at a fixed width.
4. **Cable section.** The shell reports whether the installation ships the
   driver helper and package (`virtual_cable_management_available`). App-only
   releases do not, so the Virtual Cables panel shows "AudioRouter cables are
   not available yet" with the VB-Cable alternative and keeps every
   management control disabled. Tauri rejects commands with a string, which
   the panel turned into a generic grey "The cable operation failed."; the
   host now wraps it in an `Error`, and outcomes go to the workspace
   notification area (errors red via `markErrorMessage`).
5. **Several custom themes.** `audiorouter.ui.custom-themes` stores up to 24
   named themes plus the selection; the previous single theme is migrated.
   UI Options lists every custom theme, "Create a new theme" adds one (a copy
   of the active custom theme, else the default palette), import adds rather
   than replaces, names stay unique, and "Delete this theme…" asks inline
   before removing only that custom theme (falling back to Dark when none
   remain). Built-in themes cannot be deleted.
6. **Website.** Home page "Make it yours" section with `assets/themes.svg`;
   manual section 05 "Choose or create a color theme" (Quick fixes is now 06).

Validation (Windows host, 2026-10-17): UI unit tests 541 passed; ESLint 0
errors (25 existing warnings); Prettier and TypeScript clean; focused Edge
tests (workbench organization incl. tabs and themes, inspector status, three
themes) passed; full Edge suite 272 passed, 9 skipped; tab regression failed on the old CSS and passed headed with
classic scrollbars; `src-tauri` fmt, Clippy `-D warnings` and the
tool-guide/cable shell tests passed; docs validation (173 files) and
`site/check-guides.mjs` passed. Screenshots of the status pill, cable notice,
tabs (dark, light, high contrast), theme editor and the site sections at
1280/390 px were reviewed. Not done: an attended shell launch of the new
cable probe (a release build would show the notice; a build with the driver
payload beside it would not).

Rollback: revert this change set; the old single-theme key is left in
storage, so a revert reads the previous theme again.

## Release 0.0.17 — 2026-10-16, owner requested ("create a new release version and push it")

**Published** 2026-10-16 15:20 UTC as an experimental prerelease from tag
`v0.0.17` (`6ed54e91`); exact-package fresh-install passed (95 ms), draft and
public assets verified. CI (run 38923536567 on `05b00352`) and a manual
nightly quality run (38923751714) passed; the two Edge flakes recorded in
the release record were fixed 2026-10-16 (below).

Scope: unsigned experimental Windows x64 **app-only** prerelease, same
profile as 0.0.16 (no driver/helper payload, `-AppOnly`). Contents: the
endpoint-ID leak fix (confirmed by the 8 h owner soak), canvas GPU cut,
startup active-session flip fix, routine-read log filtering, workbench UX.
Requirements NFR-06, DIST-01–08, UI-17. Record:
[release 0.0.17](evidence/2026-10-16-release-0.0.17.md).

Prerequisite CI repairs (same day): `zip` 2.x was yanked in full on
crates.io, so `cargo deny` failed (run 38852616585). Moved to `zip` 9.0.4
with only the `deflate` and `time` features (the defaults pulled
`libbz2-rs-sys`, whose bzip2-1.0.6 license the policy does not allow, plus
zstd/lzma/AES we never use); entry names became fallible (`name()?`). The
nightly coverage job (run 38871837715) never wrote
`ui/coverage/coverage-summary.json`: under PowerShell, `npm … test --
--coverage` drops the `--`, so the flag reached npm instead of Vitest
(confirmed: `-- --version` ran the suite). New `test:coverage` script.

Tasks: bump only AudioRouter packages to 0.0.17; full Windows workspace,
shell, UI, contract, Edge and release-script checks; tag; build from an
isolated clean checkout of the tag with `create-draft-release.ps1 -AppOnly`;
`fresh_install_shell` on the packaged shell with no other AudioRouter
running; verify the uploaded draft; publish as prerelease (owner request
covers publication). Rollback: keep 0.0.16 and backups; no migration; never
replace published assets or move the tag.

## Repair CI run 38835254406 — local fix complete (2026-10-15)

Objective: repair the Windows build/test workflow failure without changing
product behavior. Evidence: [GitHub Actions run 38835254406](https://github.com/MrDesjardins/audiorouter/actions/runs/38835254406).
Affected requirements: UI-17 (parameter autosave and explicit Save state),
MCP tunnel process containment (SEC-01 / local child-process lifecycle).
Prerequisites: Windows Edge, Node dependencies and the pinned Rust toolchain;
the CI run is Windows evidence, while local checks are only supplementary.

Findings: Edge had stale assertions for the retired “Unsaved…” text and an
outdated MCP missing-install message. The live dynamics editor now debounces
parameter-only saves while playing; stopped device selection marks Save
required. The Windows tunnel test relied on nested PowerShell startup to
print a descendant PID; CI timed out waiting for it, then panicked parsing an
empty line. Keep the product contract, update the browser assertions, and use
a deterministic Rust test-binary child fixture to exercise job containment.

Implementation: the device-binding test now checks the enabled header Save
action and canvas marker. The dynamics test checks that a rejected debounced
save on its intentionally disconnected fixture leaves Save required; valid
live autosave is covered by compact-workspace. Updated the MCP status copy
assertion and replaced PowerShell with a direct test-binary descendant.

Validation on the Windows host: Edge Playwright, focused to the affected
desktop workspace, three-theme device binding, and three-theme compressor/gate
cases: 10 passed. The tunnel job-owner/descendant test passed. UI format check,
TypeScript and production build passed; ESLint passed with 0 errors and 25
existing hook warnings. Documentation validation passed (171 Markdown files,
949 local links). `cargo fmt --all -- --check`, the `src-tauri` format check,
and `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
--all-features -- -D warnings` passed. The full GitHub workflow has not rerun;
its Windows runner result remains the authoritative check after this fix is
pushed.

Rollback: revert only this plan, test and fixture change set. Risks: Windows
job nesting can depend on runner policy; CI rerun remains required. Next:
commit and push this fix, then confirm both previously failing Actions jobs.

## Publish automated quality status in the root README — implemented 2026-10-15

Owner requested visible code coverage and status for the existing performance
and quality automation. README now has CI and quality-workflow badges. After
successful Rust/UI coverage, engine soak and backend-startup jobs, a separate
job with only `contents: write` refreshes the marked summary table and appends
one idempotent row to a durable CSV on `main`. It generates a four-panel SVG
timeline for the last 90 successful runs (engine p99/p99.9, engine heap,
Windows process memory and backend readiness); the CSV retains the complete
history. Coverage remains informational and has no threshold.

Scope: CI/quality README badges, quality workflow, a dependency-free Node
summary/update/chart script, README, durable quality-history CSV/SVG and this
plan. No runtime or user data changes.
The task-specific script and workflow were reviewed in source; execution on
Windows Actions was not performed locally. The first successful main-branch
quality run is the required end-to-end evidence. Risk: repository branch
protection or Actions settings may reject the bot's README/history push.
Rollback: remove the publisher job, script, README block and generated history
files; existing Actions summaries remain.

Next: after these changes reach `main`, dispatch `quality.yml` and confirm the
coverage, soak and startup jobs succeed and the publisher updates README.md.

## Graphic EQ help playback level — completed 2026-10-15

Raised synthetic guide monitor gain to 50% with the peak-protection compressor
retained. Source PCM was healthy; the prior 16% monitor gain made all examples
too quiet on some systems. Regression passes on Graphic EQ and Mixer: generated
sample RMS is nonzero, AudioContext is running, and the connected monitor gain
is 0.5. TypeScript, focused ESLint/Prettier, 35-guide audit, and diff checks
pass. See the [execution record](../archived/2026-10-15-graphic-eq-guide-audio-level.md).
No microphone/device or desktop build was required; static site publication
was not performed. Next: reload the local page with Ctrl+F5 and try Play Example.

## Tool teaching and sidebar spacing — implemented 2026-10-15

Requirements UI-11/12/17/20 and the documented DSP/recording/routing contracts.
All 35 guides now teach tool-specific setup, expected results, fault checks and
example limitations. Relevant examples use deterministic PCM, named input/output
sample waveforms and reference listening through one transport. Mixer/Switch
have distinct tracks; Duck has a control-only key; Delay, cleanup, pitch timing,
Test Signal level, Meter readings and early Recorder take length are corrected.
Tool cards (including app capture, Group and VST) use compact keyboard-accessible
question marks without triggering Add. Properties reserves actual wrapped tab
height plus 12 px, keeping all bottom tabs visible/clickable.

Windows Edge combined suite: 10/10 pass in 1.5 minutes, including every guide,
numerical sample checks, mobile/reduced motion and dark/light/high-contrast at
1440/1120/980 px. Static coverage, TypeScript, focused ESLint, formatting, docs
and UI production build pass. Initial failed checks and their fixes are in the
[execution record](../archived/2026-10-15-tool-teaching-and-sidebar-spacing.md).
No website publication or production DSP/hardware qualification occurred.

Packaging handoff: desktop PID 73136 was running at inventory; it was not
stopped or replaced. UI assets are rebuilt. Owner has been asked to close the
app so its executable can be rebuilt; this is the exact remaining task.
Rollback: revert only this follow-up's listed files, preserving earlier shared
work. No database/session migration is needed.

## Workbench organization, themes and visual tool guides — completed 2026-10-14

Owner expanded the active UI task: replace Setup with Status (six readiness
checks and this-PC audio devices), Virtual Cables (all cable management/guidance)
and UI Options (header theme moved here, create/import/export a local theme).
Move device access/version preferences into Advanced. Integrate the Recording
empty state into its library card. Make local assistant setup a step-by-step
flow. Build a premium static website guide per tool, explaining every exposed
configuration with examples and accessible animated illustrations, linked
from tool Properties/Tools. Continue the compact-workspace scope above.

Requirements: UI-01/05/07/11/12/17/18 plus new owner-authorized UI-19 (organized
workbench/custom themes) and UI-20 (visual tool guides). Existing static
GitHub Pages source is site/ for audiorouter.org; no new hosting, publication,
driver, cloud inference or remote-control capability is authorized by this slice.

Ordered phases: (1) finish compact workspace regressions; (2) split settings,
add validated bounded JSON theme import/export and theme token editor, tidy
Recording/MCP; (3) research every exposed tool property from source, generate
individual static guide pages and on-demand animated examples, wire safe
contextual guide access; (4) verify coverage, links, keyboard/reduced motion,
three-theme responsive UI and visual website QA; (5) build an owner-testable
desktop artifact when no executable lock prevents staging.

Use Sites workflow for the existing local checkout. Its explicit bounded
asset/research delegation instruction permits three agents: processing property
research, routing property research, and an isolated animation runtime asset.
Root owns UI, integration and source contracts. HyperFrames guidance routes
the short instructional animation assets; deliver on-demand HTML/WAAPI with
static/reduced-motion alternatives, not a bulk video download.

Theme schema must allow only fixed color-token keys and hex colors, a bounded
name and versioned app format; reject oversized/malformed files without changing
current theme. Export only theme data, never app/session settings or secrets.
Guide illustrations are conceptual and must never imply measured response,
latency, live audio capture or universal VST support. Use actual source ranges
and defaults; mark dynamic/plugin/device-dependent properties honestly.

Validation and rollback evidence are archived in
[the completed workbench and guide execution record](../archived/2026-10-14-workbench-organization-and-visual-guides.md).
Current result: UI and static guide implementation are complete, local tests
and the custom-protocol desktop build pass, and no external publication was
performed. The source website is ready for owner review; publication remains a
separate authorized action.

The 2026-10-15 follow-up reviewed all 35 guide kinds, corrected FIR/EQ/network
scenes, and added opt-in synthetic audio. Its static audit and five Edge guide
checks pass; see the [execution record](../archived/2026-10-15-all-tool-guide-audio.md).

Roll back by reverting the archived task's listed
files; theme import/export has no app/session data migration.
Next action: owner reviews the built desktop app and local visual guides; then
decides whether to authorize website publication.

## Synchronized tool guide playback — completed 2026-10-15

Owner request completed for UI-20/UI-17, DSP-01/02 and REC-01/02/07. The static guides now use one Play/Pause, Replay and seek timeline for each audio-relevant tool; controls have reserved widths. Device/application capture, physical output, arbitrary VST and Visual Group pages remain animation-only. Recorder uses Record → Stop → Play recording across Recorder and Audio File stages; Mixer combines two distinct synthetic tracks with independent controls; Meter follows the app's stereo dBFS RMS/current-peak/peak-hold/clip layout; each of the ten Graphic EQ bands explains purpose, examples and cautions.

All sound is locally generated after a user click. Recorder stores only an in-memory synthetic example. No microphone, user's file, session audio, network connection, disk recording, live DSP measurement, desktop app or published site is involved.

Verification: generator and static audit passed (35 pages, 101 properties, 941 local links/assets and UI/native kind parity); TypeScript, focused ESLint, Prettier and diff whitespace checks passed. Microsoft Edge Playwright suite passed 6/6 with offline pages, shared playback/seek/pause, responsive/reduced-motion, Mixer, Meter, Recorder, EQ and Network Send/Receive scenarios. The execution record contains commands, environment, results, screenshot path, limitations and rollback: [synchronized tool guide playback](../archived/2026-10-15-synchronized-tool-guide-playback.md). No hardware/audio-device gate applies. No commit or publication performed.

Next action: owner reviews the updated local guide pages. Website publication remains a separate authorized action.

## Compact workspace follow-up — 2026-10-14 (owner request)

Objective: move notifications into a bottom-right popover without reserving
workspace height; tighten Canvas heading/selection spacing; use consistent
question-mark help in every tab; replace the inspector change-summary line
with clear Save/node markers; rename through the inspector title.
Requirements: UI-04/05/07/09/11/12/17 (M05 maintenance). Prior header/tray
changes are uncommitted and must be preserved. Read interface/M05 contracts
and existing lifecycle/autosave behavior; Windows Edge and dependencies are
available. No hardware, driver or credentials are needed for UI qualification.

Decisions: notifications remain accessible, nonmodal and dismissible; warnings
and recovery actions stay visible without moving the workspace. Stopped setup
guidance moves into help. Retain the existing 400 ms parameter-only autosave
while running; unsaved topology/name/stopped edits require explicit Save.
Node indicators distinguish pending autosave/saving from Save required and
clear only after authoritative save. Use an identical help disclosure/icon;
rename title supports pointer, keyboard, Enter/blur commit and Escape cancel.

Ordered tasks: update notification layout and canvas spacing; add reusable help
and inline naming; derive node save markers from saved/draft content and edge
changes; update contracts/docs and focused regressions; format, lint, typecheck,
build and run meaningful component/Edge checks in all themes. Measure canvas
bounds across messages and saves, plus live autosave/manual Save behavior.
Rebuild the embedded desktop UI for owner testing when its executable is free.

Rollback: revert only this follow-up's edits atop the previous UI slice; keep
prior desktop binaries. Risks: notifications obscuring controls, stale save
indicators, focus loss on selection, confidential parameter values in labels.
No secret values may be displayed in save markers. Evidence: archived with the
workbench and guide execution record; 535 UI unit tests, 42 sequential Edge
checks plus four focused Virtual Cables checks passed across dark/light/high-
contrast themes. Custom theme round-trip, guide interactions, and cable route
layout all passed. Next action: owner review of the rebuilt desktop app.

## Header and canvas usability — 2026-10-14

Implementation and browser verification complete for the four requested fixes
and the follow-up removal of header/tray mute toggles (UI-04/09/10/11/12/15/17).
The [execution record](../archived/2026-10-14-header-canvas-usability.md) records
files, decisions, reproduction, rollback and evidence: 527 UI tests, 80 shell
tests (4 ignored), 13 Windows Edge checks, three-theme visual review, both
Rust format checks, both Clippy checks and UI lint/typecheck/build pass.

Owner-requested test build completed (2026-10-14): optimized custom-protocol
shell, plugin worker and staged MCP CLI at `src-tauri/target/release/`. Shell
is newer than the current UI bundle; staged/source CLI hashes match. Previous
shell and CLI preserved for rollback; hashes and commands are in the record.
No shell was running at inventory, and the agent did not launch the rebuilt app.
Next M05 task: owner tests the actual tray menu and embedded UI using the
rebuilt audiorouter-shell.exe.
Check for an existing shell before launching. Browser fixtures do not qualify
real audio or the packaged app. No implementation blocker remains; the
independent stability task below still awaits the owner's overnight soak.

## Stability round — 2026-10-14 (user request)

Objective: find and fix stability problems before users hit them, while the
driver signing waits on funding. Scope: the app (shell, backend, UI); the
frozen driver is out of scope. Requirements: ARCH-04, NFR-06, UI-17, SEC-01
(diagnostics), AUTO-15. Method, in order: field logs from the owner's
install; full local suites as a baseline; long-run behaviour; a code sweep
for long-uptime failure classes; each fix with a regression test.

Baseline (Windows host, before fixes): Rust workspace 1,105 passed / 0
failed / 25 ignored (live-device tests); UI Vitest 523/523; lint 0 errors
(25 reviewed warnings); CI on `main` green at `96fa215f`. Nightly engine
soak (run 38536990496): 20 min = 10.7 h of audio, heap +0.0 KiB, p99
202 µs/quantum (budget 1,333 µs). The engine soak covers the engine only;
nothing soaked the shell, backend service loop, WebView or logs.

Findings and fixes:
1. **Logs kept ~30 minutes of history.** Owner's 0.0.15/0.0.16 logs:
   `backend.jsonl` + previous (2 × 5 MB) covered 33 minutes, 9.5 lines/s,
   ~330 MB/day of writes, because `is_routine_read_rpc` named only
   `system.diagnostics` and `events.subscribe` while the window now polls
   `meters.levels`, `sessions.summary`, `recorders.list` and
   `virtualCable.status`. Fix: the class comes from the API table
   (`SideEffectClass::ReadOnly`), so new polls cannot drift; failures always
   log, verbose keeps everything; `sessions.get`, `sessions.list` and
   `devices.list` stay logged (they follow user actions and carry the graph
   and device counts support needs); HTTP reads stay unlogged (Stream Deck
   polls `sessions.list`). Regression
   `polled_read_successes_stay_out_of_the_backend_log`; transport 32/32,
   shell 80/80 (its request-ID test caught a first draft that would have
   logged Stream Deck's polls again).
2. **Startup flipped between two sessions and overwrote the saved active
   session.** Seen in a soak dry run with a fresh WebView profile (first
   launch, cleared data): 24 `sessions.active.set` writes and full reloads
   in 0.4 s, alternating the owner's 4-node and 19-node sessions. Cause: the
   initial session list replaced the demo placeholder with the first listed
   session before the startup snapshot ran; the window wrote it as active
   (overwriting the backend's saved choice); every write emits
   `session.selectionChanged`, which the event poll answered by reading the
   active session back, and a read between two in-flight writes returned
   the other session. Risk: autoplay plays the active session about a
   second after launch. Fix (`App.tsx`, `useWorkspaceEvents.ts`): with
   nothing remembered, the first snapshot opens the backend's active
   session; list refreshes keep the placeholder while connected; no write
   of a value the backend already has, none while the placeholder is
   selected; backend changes are not adopted while this window's own write
   is in flight. Regression `App.sessionSelection.test.tsx` (fails before
   the fix: the saved active session ended as `desktop-session`); UI
   525/525.
3. **Sweep, no defect found:** no `lock().unwrap()` in product code (mutex
   poisoning cannot cascade); frame/sample counters are 64-bit (no 24.8 h
   wrap); `expect`s in thread modules guard length checks.
4. **Shared-mapping aliasing (P3-1 audit):** `NativeBridgeRegion` now uses
   `memmap2::MmapRaw` with volatile load/store (`f02aa839`), requalified by
   the r10 VM `-Session all` (no regression; virtual-cable plan).
   `SharedAudioRegion` (plugin host) still writes through `MmapMut`'s
   shared slice: open, same fix.

5. **Developer hygiene (noted, not fixed):** `cargo test --workspace` and
   the e2e backend write records into the developer's real
   `%LOCALAPPDATA%\AudioRouter\logs` (`graph.commit` lines from test
   processes in `backend.jsonl`, MCP tests in `mcp-activity.jsonl`), so log
   evidence and support bundles from a development PC mix in test records.
   Follow-up: give tests a temporary log directory.

6. **The playing canvas used up to ~2 cores of WebView GPU and renderer
   time.** First overnight soak attempt (owner's database, window open
   behind Firefox): WebView CPU rose from ~6 % to bursts of 130–170 % of a
   core (GPU process ~103 %, renderer ~62 %) while the shell stayed at
   ~6 %; DOM, listeners and heap were flat (no leak). Bursts followed audio
   activity, not uptime, so live-route measurements could not separate the
   causes (two attempts gave contradictory answers). Controlled measurement:
   headed Edge on the route harness (4 paths, 12 nodes, 8 live links,
   sinusoidal talk), per-process CPU from browser CDP
   `SystemInfo.getProcessInfo`, layers switched off by injected CSS, 3
   rotating rounds (scratch spec, not committed). Before: 135 % GPU + 78 %
   renderer; canvas hidden 1 %. Causes: every live connection shares one
   canvas-sized SVG, and (a) SMIL `animateMotion` comets (six per link,
   hidden ones animating at opacity 0) repainted it at 60 fps (~40 points),
   (b) 160 ms colour/width transitions restarted by every 20 Hz tick kept
   it repainting (~50 points), (c) node meters animated `height`. Fixes
   (`flowLine.tsx`, `SessionFlowCanvas.tsx`, `DuckEditor.tsx`,
   `styles.css`): comets are HTML in React Flow's label layer moved by
   Web Animations transform keyframes (compositor; same look, under the
   nodes); connections draw the level in 3 dB steps without transitions;
   live bars scale with `transform` and step at 20 Hz; Animated
   connections defaults to "while AudioRouter is the active window"
   (stored choices kept). After: 46 % GPU + 43 % renderer with lights
   moving, 17 % + 24 % paused (the remaining cost is compositing a
   60 fps or 20 fps window). Tests: `flowLine.test.ts` (level steps,
   constant-speed keyframes, no 360° spin), Animated connections default;
   UI 527/527, e2e 238 passed + 1 timing flake in an unrelated top-bar test
   (passes alone and 69/69 with `--repeat-each 3`), 9 skipped; dark, light
   and high-contrast canvas screenshots reviewed. Real-app check: the
   rebuilt shell runs and selects the new default, but no audio flowed
   during the check, so moving comets were seen only in Edge (same engine).
   `run-app-soak.ps1` gained `-DebugPort` (diagnosis only) and a 5.1 fix
   for its default shell path.

7. **The shell leaked ~9.3 MiB/h while a session played.**
   [Evidence record](evidence/2026-10-15-shell-endpoint-id-leak.md). Two owner
   overnight soaks (8 h each, owner's database copy, `-AutoPlay`, build
   `87545f1b`): every check PASS except shell private bytes, REVIEW at
   9.31 and 9.26 MiB/h, linear for 8 h (~220 MiB/day for a tray app);
   WebView flat, handles 509, threads stable. Attribution (scratch probes,
   database copies, a separate worktree): idle shell flat;
   777,000 `system.diagnostics` calls through the real window path while
   idle +1.4 MiB then flat (request path ruled out); a dhat build showed
   live Rust heap flat at ~6 KB while the process grew 8 MiB/h, so the leak
   was native. Two `cdb -pv` heap snapshots of the process heap 25 min
   apart while playing: only the 128-byte LFH bucket grew (15,112 →
   43,039 blocks, 18.7/s ≈ 8.2 MiB/h), and the blocks were endpoint ID
   strings (`{0.0.x.00000000}.{…}`), active and inactive endpoints. Cause:
   all six `IMMDevice::GetId` call sites in `windows-audio` copied the
   CoTaskMem `PWSTR` with `to_string()` and never freed it (two comments
   claimed the `windows` crate frees it; it returns a bare pointer).
   Standalone measurement per call on the owner's PC: active walk 2,263 B,
   display info 2,064 B, defaults 809 B, all-state walk 8,251 B;
   application walks 0. Which caller repeats about once a second while
   playing is not proven; likely the endpoint monitor, which re-walks on
   every `OnPropertyValueChanged` (follow-up below). Fix: `take_endpoint_id`
   copies then frees, used at every site. Regression (live, ignored on
   CI) `endpoint_walks_free_every_endpoint_id`: fails before (5.6 MB left
   after 500 walks of 62 endpoints), passes after; windows-audio 141/141,
   Clippy and fmt clean. Real-app check, patched `87545f1b` worktree build,
   full session playing 40 min: 128-byte blocks 141 → 141 between minutes
   10 and 35, private bytes 32.10 → 32.46 MiB over 30 min (before: 8–10
   MiB/h). Follow-ups: (a) the endpoint monitor re-enumerates every
   endpoint, opening an `IAudioClient` on each, whenever any property
   changes; check the notification rate while playing and filter keys;
   (b) the 20 Hz refresh makes ~82,000 short allocations/s (dhat), and the
   backend logger decodes every response again even for unlogged routine
   reads; (c) one diagnostic `cdb -pv` session left the probe shell's
   threads suspended when killed: keep heap commands short.
   **Confirmed 2026-10-16 by the owner's 8 h overnight soak** on the fixed
   build (`87545f1b` plus this fix, SHA-256 `4B9C16DF14707527…`, 951
   samples, session playing): every check PASS; shell private bytes
   0.11 MiB/h after warm-up (30.9 MiB at the end; before 9.31 MiB/h and
   ~105 MiB), WebView −0.22 MiB/h, handles flat at 509, thread spread 2,
   0 error lines, shell CPU 2,257 s (unchanged).

New tool: `tools/quality/run-app-soak.ps1` runs the release shell for hours
on a copy of the owner's database with its own `LOCALAPPDATA` and WebView2
profile (`WEBVIEW2_USER_DATA_FOLDER`; Tauri resolves its WebView folder
through the known-folder API and ignores `LOCALAPPDATA`, so the first two
dry runs used the real WebView profile; the remembered last session there
reads `patrick-main-siege-footstep`, the database's active session). It
checks that the real AudioRouter and WebView folders were not modified, has
optional `-AutoPlay`, and samples shell and WebView private bytes, handles,
threads, CPU and log growth every 30 s; verdicts on slopes after a
30-minute warm-up and on log history at the measured rate. 6-minute idle
dry runs: on 0.0.16 logs would hold 9.5 h (backend) and 12.4 h (shell);
on the fixed build ~500 h each, real folders untouched, and a fresh
profile starts with 0 active-session writes and opens the saved 19-node
session (before: 24 writes, 11 flips). Workspace and shell Clippy and
`cargo fmt --check` clean; UI format, lint (0 errors) and `tsc` clean;
Playwright e2e 239 passed, 9 skipped.

Follow-up (a) done 2026-10-16: the endpoint monitor gets no notifications
while playing; the walk that repeated every ~4 s was `virtualCable.status`
(UI polls 2 s, backend cache 2 s). Its cache now lasts until an endpoint
change, with a 30 s backstop: cable walks once per 30 s instead of 7.5
([record](evidence/2026-10-15-shell-endpoint-id-leak.md#repeating-caller-found-2026-10-16)).
Follow-up (b) done 2026-10-16: the backend logger no longer decodes the
responses of routine-read successes it does not log (session-sized
`system.diagnostics` 61 → 6 µs per record on the logger thread; same
record).
Edge flakes from the 0.0.17 full suite, 2026-10-16:
- `undo-eq-points`: reproduced with `--repeat-each=12 --workers=18`
  (8 of 36 failed at the gain Ctrl+Z, value still `-6`). Cause: the test
  pressed Ctrl+Z right after the "Saved (revision N)" message, but the save
  keeps `graphBusy` until its post-save `refresh()` ends, and history is
  unavailable while busy (the Undo button is disabled too; clicks wait for
  it, the shortcut does not). Test now waits for Undo to be enabled: 72 of
  72 passed over two stress runs. A product change tried first (window
  listener calling the latest handler through a ref) only moved 8 → 6
  failures and was dropped.
- `virtual-cables` status pill: not reproduced (old 5 s wait passed every
  run at 24 workers). The pill changes only on the window's 2 s poll, so
  the wait per status is now 10 s; 24 of 24 passed under stress.

## Cable route lost connection: retries, then a message — 2026-10-16

Owner decision (2026-10-16, "Let's have retries and then messages") on the
r10 finding (virtual-cable plan, results): a cable-only route whose lease
heartbeat or worker fails now closes (cable silent), reports
`reconnecting` and reopens after 1, 2 and 4 s with the session still
playing; then it stops with a plain message. A loss within 30 s of a
reconnect continues its count. Format changes keep their own stop message.
Spec 17 §7.3, `crates/control/src/cable_routes.rs`, contract and output
schema (`cableRoute.state` gains `reconnecting`). Rollback: revert the
commit.
- Checks: fmt clean (both workspaces); workspace Clippy clean; control
  tests 252 passed, 9 ignored (two new: retry count and messages);
  contracts typecheck and drift pass; UI typecheck and format clean;
  `VirtualCablePanel` tests 17 passed.
- Not run: the reopen itself needs the cable driver, which is not
  installed on the host; qualify it in the next VM bundle (freeze the guest
  over 2 s during a cable route and check `reconnecting` then `running`).

Next action: none queued; take the owner's next request.
Open decisions: none.

## Release 0.0.16 published — 2026-10-12

The owner-requested review/commit/push/experimental app-only release is
complete; [execution record](../archived/2026-10-12-release-0.0.16.md)
archives source, tests, hashes, known gates and rollback. Full local suites,
exact packaged fresh-install and public asset verification pass. Qualified
0.0.16 app and matching companions staged at the existing launch path; old
binaries retained. No driver payload or default-device changes.

CI maintenance follow-up: `7baa1cd6` repairs the stale standalone probe lock
and selects app-only M08 acceptance in CI. Locked probe and release regressions
pass locally; replacement run 38333587240 was in progress at publication.
Next: observe its result and repair substantiated failures. Remaining local
Cowork and ChatGPT interoperability/security tasks are below; full M08
consumer/hardware/signing gates are still open.

## Local Cowork plugin — 2026-10-12, owner authorized

Objective: export a ready-to-upload plugin for the owner's **local** Cowork
deployment. Requirements AUTO-06/10/13 and UI-17; backend grants remain the
authority. Prerequisites: Windows, matching installed CLI, running backend,
and a Cowork deployment whose policy permits local plugin MCP servers.
The owner selected Local. Anthropic's [plugin guidance](https://support.claude.com/en/articles/13837440-use-plugins-in-claude)
and [architecture overview](https://support.claude.com/en/articles/14479288-claude-cowork-architecture-overview)
distinguish local plugin MCP from cloud tasks and Desktop chat configuration.
This supersedes earlier blanket statements that all Cowork needs remote MCP.

Local implementation completed and archived in the
[execution record](../archived/2026-10-12-local-cowork-plugin.md): plugin ZIP
export, separate Observer enrollment preserving existing grants, real CLI
preflight, conditional controls and stable feedback. Claude plugin validator,
real stdio/Windows pipe/denied-write checks, native export fixture and four
three-theme UI checks pass. Updated app staged at the existing release path.
The [runbook](../../operations/cowork-mcp.md) gives the upload steps.

No binaries, keys, database contents or audio are bundled. Plugin configuration
is specific to this PC. No remote listener, tunnel or cloud hosting is added.
Validation must distinguish CLI/plugin checks from actual Cowork import and
tool execution: no running Claude Desktop was found, so that attended gate
remains open until observed. Risks: Claude deployment/policy changes and moved
installation paths. Rollback: remove the exported plugin in Cowork, revoke
`audiorouter-cowork-local`, and revert the adapter/UI change. Next action:
attended local Cowork import, session-list read, owner-approved stopped-session
creation and post-revocation denial; capture matching MCP activity and versions.

## ChatGPT MCP setup accepted — 2026-10-12

Documentation example (AUTO-13/AUTO-16, owner request) completed: the supplied
`Mcp_Result.png`, `Mcp_Result2.png`, `Mcp_AudioRouter_Setup.png` and
`Mcp_ChatGPTPlugin.png` were reviewed and copied unaltered to stable image paths.
The [worked example](../../operations/chatgpt-mcp-example.md)
explains AudioRouter/ChatGPT setup, session listing, voice route creation and stopped playback; README
and runbook links make it discoverable. Source/copy hashes match and local
documentation links validate. Rollback removes the copied assets and paired
links; the Downloads originals and app/session state are untouched.

The user confirmed successful connection and supplied the created
[ChatGPT plugin](https://chatgpt.com/plugins/plugin_asdk_app_6acd77c0830881919e8b719807f198a8).
The local compatibility/tunnel setup repair is complete and archived in
[the execution record](../archived/2026-10-12-chatgpt-mcp-setup-repair.md),
with [live repair evidence](evidence/2026-10-12-chatgpt-tunnel-repair.md).
The user subsequently confirmed tool use and supplied matching ChatGPT/app
screenshots: sessions were listed and **PD200X Radio Voice** was created with
PD200X → Advanced EQ → CABLE-A and Scarlett headphones. This is user-supplied
read/write evidence. No AudioRouter MCP tools are exposed in this Codex chat,
so the agent does not claim to have performed those calls.

Next task: qualify denied writes, reconnect/revocation and credential/privacy
behavior under the integration plan below. The owner's example read and
permitted session/graph write are confirmed; the broader qualification gates
remain open. Generic remote
MCP for cloud Cowork remains [future work](../future/generic-remote-mcp.md);
the owner's local Cowork plugin is the active task above.


## ChatGPT MCP integration plan (2026-10-11, active)

OpenAI Secure MCP Tunnel transport only; generic cloud MCP is tracked in
[future work](../future/generic-remote-mcp.md).

### Objective and scope

Make the AudioRouter MCP tools usable from the user's ChatGPT Windows app with
the fewest setup steps. AudioRouter should install/manage the official OpenAI
Secure MCP Tunnel client, supervise its local stdio MCP connection, and show
plain-language connection health. The app must retain the existing backend
enrollment, per-client scopes, graph validation and audit. This is remote
control-plane access to the local app, not audio streaming. Requirements:
AUTO-16, SEC-14, UI-17; decision DEC-20. No public/LAN MCP listener, raw audio,
automatic tunnel startup, or change to existing local MCP clients is in scope.

### Verified constraints and unresolved account dependency

AudioRouter currently has local stdio MCP. OpenAI's current official
Secure MCP Tunnel guide says the setup requires a `tunnel_id`, a runtime API
key, and a local stdio or HTTP MCP server. Tunnel CRUD needs Tunnels Read +
Manage; running the daemon and selecting the tunnel in ChatGPT needs Tunnels
Read + Use. Tunnel visibility also depends on associating the tunnel with the
target ChatGPT workspace. ChatGPT separately requires the user to add a
custom MCP app, choose Tunnel, review its warning, and create the app. The
owner's successful custom-plugin setup was confirmed on 2026-10-12. Account
plan type and administrative workspace policy were not inventoried. Sources checked 2026-10-11: [OpenAI Secure MCP Tunnel guide](https://developers.openai.com/api/docs/guides/secure-mcp-tunnels),
[official tunnel-client configuration](https://github.com/openai/tunnel-client/blob/master/docs/configuration.md),
[official end-user guide](https://github.com/openai/tunnel-client/blob/master/docs/end-user-guide.md).

The OpenAI docs require a tunnel ID and runtime key for the daemon. The
restricted runtime key is created in Platform's Runtime API keys UI with
Tunnels Read + Use; admin keys are only for tunnel CRUD. The current OpenAI
guide describes ChatGPT's separate add-custom-MCP-app, choose-Tunnel,
review-warning and Create steps. Thus a fully silent, zero-setup flow is not
supported: at minimum the owner must supply an existing tunnel ID and
restricted runtime key and approve the ChatGPT connector. AudioRouter will
automate local runtime packaging, credential protection, process lifecycle,
and status, and open the right account screens. No admin-key flow is in this
slice. Initially the development environment lacked a runtime key and the
Platform page was logged out; the owner later configured the existing tunnel
and successfully created the ChatGPT plugin. Never scrape browser cookies or
ask for a secret in chat.
No remote resource or key is created by local implementation.

The official release page currently marks `tunnel-client` v0.0.16 as latest
(2026-10-06); its repository is Apache-2.0 and release assets include Windows
amd64 archives plus license reports, SPDX SBOMs, checksum manifest and
Sigstore provenance. Review the exact Windows artifact and verification
procedure before packaging. The tunnel-client version is not hardcoded in the
app until release evidence records the artifact hash and verification.

### Intended user flow

1. The MCP tab now separates the standard local stdio server from the optional
   OpenAI tunnel. Keep other clients, permissions and activity secondary.
   Cloud Cowork requires a future remote endpoint; local Cowork uses its
   separate plugin export and is not configured by this OpenAI-specific path.
2. On Connect, check supported Windows architecture, packaged tunnel-client,
   current-user storage availability, and the target ChatGPT custom-MCP
   capability. Explain only the specific item that prevents setup.
3. Reuse an already authorized AudioRouter tunnel if its workspace association
   and credentials validate; otherwise prepare the minimum-scope Platform
   authorization flow. Get explicit confirmation immediately before remote
   resource/key creation. Do not persist the admin credential.
4. Use an existing OpenAI tunnel associated with the intended ChatGPT
   workspace. The owner creates a restricted runtime key with Tunnels Read +
   Use in Platform and enters the tunnel ID/key in Setup → MCP. AudioRouter
   does not create remote resources in this slice. Persist the tunnel ID and
   runtime key together in a current-user DPAPI-protected file.
5. Enroll a distinct `audiorouter-chatgpt` MCP client and obtain the same
   owner-approved scopes used by other clients. Launch the official
   `tunnel-client` with a fixed AudioRouter CLI stdio target and the local
   named pipe/database. Supply runtime credentials through a child-only
   environment or a more protective supported input; never arguments,
   browser storage, logs, crash text, or support bundles. No arbitrary command
   execution surface.
6. Supervise readiness, connection loss, process exit, app shutdown and
   explicit Disconnect. Disconnect stops the local process; Forget saved
   details removes the local DPAPI copy. OpenAI resource/key revocation
   remains an explicit Platform action. Display bounded states such as Checking, Needs
   authorization, Connecting, Connected, and Action needed; reserve dimensions
   so live status does not shift layout. Disconnect stops the process and
   offers remote revocation where the official API supports it. Startup stays
   off unless the user separately enables it.
7. Open the official ChatGPT connector settings at the point the tunnel is
   ready. Guide the user to add a custom MCP app, select this tunnel, review
   access, and create it. Explain that the connector must then be selected in
   the ChatGPT conversation's add-tools menu. The default Observer grant is
   read-only; creating or editing a session requires the owner to change the
   `audiorouter-chatgpt` grant to Editor in Client permissions. Afterward,
   verify a read-only capability call and surface actionable errors without
   exposing credentials.

### Implementation sequence and progress

1. **Pin and package the official client — implemented.** OpenAI
   `tunnel-client-runtime` v0.0.16 is pinned to archive SHA-256
   `639fdfe10d91c672d8f8a49386ebf39acaac4e560a88134735b1739a02ddb82d`.
   Release staging verifies the digest and GitHub attestation before extracting
   the binary and licensing/SBOM data. Current restricted-network re-verification
   could not initialize Sigstore; release staging fails closed.
2. **Build the native tunnel service — implemented.** A Windows-only supervisor
   stores configuration under current-user DPAPI, starts only on user action,
   polls loopback readiness, reaps/stops the child and supports local Forget.
   The OpenAI runtime key is not placed in arguments or logs; the trusted MCP
   child clears its inherited key variables immediately.
3. **Authorization/provisioning — local portion implemented; remote part gated.**
   An explicit first Connect grants the distinct ChatGPT client Observer access.
   Owner-supplied restricted Platform key and existing tunnel are required.
   UI idempotency keys are compact so the backend's caller/method scoping stays
   under the 128-byte durable journal limit. No admin credential or remote
   resource is created by AudioRouter.
4. **Wire the MCP UI — implemented and visually verified.** Connect, Disconnect,
   Forget, saved-details entry, local status, permission disclosure and the
   ChatGPT connector handoff are present. Setup-page actions open the fixed
   OpenAI destinations through the Windows shell and report errors in the UI.
   Layout reserves status/message height.
   Three-theme screenshot/layout evidence is in the archived UI flow correction.
5. **Qualify and document.** Add portable state-machine, redaction and grant
   tests; Windows DPAPI/process supervision tests; mocked tunnel-client
   protocol tests; and three-theme Edge tests for no layout movement. Then,
   on an authorized Windows account, qualify tunnel creation, workspace
   association, ChatGPT discovery, read success, denied write, approved write,
   disconnect/restart, key revocation, and no key/audio leakage. Record account
   plan, versions, commands, results and redacted evidence. No real audio
   content is used.

Implementation and build evidence: [managed ChatGPT MCP evidence](evidence/2026-10-11-chatgpt-mcp-managed-runtime.md).

### Acceptance, risks, rollback, next action

Complete only when the packaged app has reproducible install-free local
prerequisites, actual tunnel supervision, recoverable connection status,
least-privilege grants, redaction/process-shutdown tests, three-theme UI
evidence, and authorized ChatGPT end-to-end evidence. Until the OpenAI account
and workspace gates pass, label the feature as unconnected/incomplete; mocks
and local builds are not proof of ChatGPT connectivity. Risks include OpenAI
changing the tunnel-client contract, organization/workspace mismatch, account
role restrictions, and revocation behavior. Roll back by disconnecting the
local process, revoking the runtime key/tunnel association when authorized,
then reverting the native/UI integration; local stdio MCP remains available.

The user now prefers a vendor-neutral remote MCP solution for ChatGPT and
Cowork over relying on the OpenAI-only tunnel. The MCP UI flow was updated and
verified in the browser harness in all three themes; this does not verify the
user's ChatGPT account or tunnel. The generic remote plan recommends an
AudioRouter-managed outbound relay as the future product architecture and
records the hosting, privacy, OAuth, reliability and cost decisions it needs.
No public listener, relay or remote resource has been deployed. Keep the
OpenAI tunnel as an optional ChatGPT transport, not the cross-client solution.
Next for this OpenAI integration: qualify denied permissions and lifecycle
behavior through the created plugin. The owner's supplied screenshots now
document a successful read and permitted route creation. Move the generic remote MCP architecture from future plan into an implementation
milestone only after choosing the service/hosting and privacy operating model.
The one stale idempotency test failure and detailed UI evidence are recorded in
[the archived flow correction](../archived/2026-10-11-generic-mcp-ui-flow.md).


## Release 0.0.15 — 2026-10-10 (in progress)

Driver r16 preparation: bounded opt-in timer/notification/fetch records,
timing model and disabled/enabled short guest wrapper implemented. Independent
fresh GPT-6 Luna High WP-04 source review finds no blocker. Host models,
concurrency, 738 driver checks, 58 wrapper checks and normal/diagnostic WDK
builds pass. Source `8679e6e7` has a clean test-signed 0.1.1 diagnostic
package; 33 package checks and all 41 immutable bundle hashes pass. No audio
repair or qualification claimed. First guest attempt: install passed;
comparison refused by its `C:\ar\IS_TEST_VM` marker guard before running
(nothing measured). Second attempt (marker created): disabled baseline
valid (61 lost, p95 19.314 ms, jitter 8.296 ms, r15-like); enabled run
invalid. A 4.77 s guest-wide pause, which the driver's own timer records
also show, expired the bridge lease and the route failed with 0x80070016.
Records were complete (0 dropped/exhausted). Third, WPR-traced attempt
valid: no pause; instrumentation did not worsen continuity (41 vs 70 lost).
Every one of the probe's 119 flagged capture packets is a packet the driver
skipped: sample `GetReadPacket` returns the newest packet, not the next
unread one, and 115 of the skipped packets were still intact. r17 repair
(return oldest intact unread packet, MoreData) implemented uncommitted:
Independent review: no blocking defect; findings applied (88,167 checks).
Committed `70e0d829`; clean test-signed 0.1.2 diagnostic package (33 checks)
and immutable bundle `diagnostics-20261011-cable-capture-read-r17`
(41/41 hashes). r17 guest result (0.1.2 verified loaded): 0 intact skips,
but still 119 skips and 118 flagged capture packets. The audio engine
ignores MoreData 136/150 times and waits for the next notification, so it
stays one packet behind and the loss moves. Gate not met. r18 (re-signal
the notification event on MoreData) implemented and reviewed (no blocker;
findings applied; pre-existing unlocked list registration recorded).
r18 guest (0.1.3 verified, SysMain off after a traced 3.45 s whole-guest
freeze during a SysMain burst): the re-signal works (141/143 immediate
follow-ups); skips 119 → 50, flagged capture packets 119 → 51 (57 %), lost
impulses 28/29, 0 corrupted. Gate criterion 2 (≥ 80 %) not met. The rest
are late first notifications. r19 rejected before implementation: the model
shows frame-granular position worsens skips, and guest data shows the VM
batches all high-resolution timers on one late interrupt (93–97 % of
capture ticks within 0.05 ms of render ticks), so an extra timer cannot help.
Controlled comparison with SysMain off (r18 ×2 against r17): flagged capture
packets 42 vs 80 on average (−47 %), driver skips −51 %, lost impulses
−27 %, 0 corrupted. r18 is a real repair but below the 80 % criterion. The
user chose the documented deviation plus one hour of r18. One-hour r18 VM
deviation run (valid, 0.1.3 loaded, SysMain off; not qualification): 409 /
1,440,000 impulses lost (0.028 %) against r12's 0.101 % (−72 %), 0
corrupted, loss runs 705 → 284, capture dropped frames −72 %. Losses
cluster in minutes 1–6 and 59–60, alongside whole-guest pump gaps; minutes
16–58 lost 3 impulses across four routes. Not controlled against r12
(SysMain was on, older runner). A native-timer environment is not
available (user, 2026-10-11): VCAB-24/25 qualification is blocked, not
waived. r20 (commit `6b2b915a`, 0.1.4) locks notification-list
register/unregister against the timer walk; reviewed (no blocker, findings
applied), host checks pass, bundle
`diagnostics-20261011-notification-lock-r20`. Guest: 10 starts under
Driver Verifier with no bugcheck and 0 corrupted; a 10-minute continuity
rerun (the first attempt was invalid because Verifier was still running
without a restart) lost 416 / 240,000 (0.17 %), 0 corrupted, the same
regime as r18's first 10 minutes. r20 (0.1.4) is the current driver;
SysMain restored in the guest. Driver work in the VM is complete. Open:
VCAB-24/25 qualification (blocked: no native-timer environment). See
[r16 evidence](evidence/2026-10-10-m03-cable-driver-timing.md).
This supersedes the r15 next-task/pending preparation below.

Latest driver evidence: r15 focused scheduler archive verified and reviewed.
Zero ETW lost events; 84/1,000 impulses lost, p95 19.076 ms but jitter
7.390 ms. Probe capture waits up to 8.083 ms before readiness and only
0.504 ms after; associated Windows audio-engine capture waits up to
7.888 ms before readiness. Generic DPC events do not identify the driver
notification callback. Next agent task: design bounded notification/fetch
timestamps, model their interpretation, and obtain required WP-04 review
before kernel instrumentation. No unchanged guest rerun. Details:
[r15 review](evidence/2026-10-10-m03-cable-latency-preparation.md#r15-returned-focused-scheduling-review-2026-10-10).
This supersedes the pending r15 instructions below.

Latest driver evidence: r14 archive verified. COM/MMCSS and trace completeness
pass, but low-latency loss persists (35/27/20 impulses); default is lossless
in all three short runs, with two p95s above 40 ms. The probe registration
repair did not solve continuity. Agent next reviews packet notification and
Windows fetch/wake timing; no unchanged guest rerun or hour test. See
[r14 review](evidence/2026-10-10-m03-cable-latency-preparation.md#r14-returned-capture-reader-review-2026-10-10).

Next step prepared: offline packet/slot model reproduces skipping after a
small notification/fetch delay and shows client timestamps alone cannot
distinguish the causes. A guest-only wrapper records the proven minimal WPR
profile around one 1,000-impulse low-latency engine run; logs, raw traces,
worker IDs and ETL are retained. Driver unchanged. Verify r15 bundle before
the [focused scheduling command](../../operations/virtual-cable-latency.md#focused-capture-scheduling-trace-after-r14).

User-requested release. Version bump and notes are committed as
`aeba0f95` (`Release 0.0.15`); local annotated tag `v0.0.15` points there.
The preceding documentation-anchor repair is `a6dc4697`. Do not push or
publish without asking the user.

Checks: the user's handoff reports shell tests, contract drift/typecheck,
504 UI tests, UI format and lint passing. The previously saved workspace log
had no completion summary. A complete rerun stopped at
`crates/cli/tests/mcp_stdio.rs`: 1 passed and 2 failed. The bounded-backend
case reported `Windows("Access is denied. (0x80070005)")`; its dependent MCP
proxy case returned `null` instead of `false`. A direct retry reproduced
both failures. Cargo could not replace `target/debug/audiorouter-cli.exe`
(`Access is denied`), preventing a Cargo retry. The isolated-target retry
could not build `lzma-sys`, so it provided no test evidence.

`node tools/docs/validate.mjs` initially found two stale explicit anchors in
the cable evidence; commit `a6dc4697` added those IDs and validation now passes
(146 Markdown files, 823 links). The three release-tool self-tests pass.
Artifact preparation first built CLI/plugin worker and UI but could not
replace the shell while the user app held it open. After the user closed the
app, another shell process was identified and the user closed it; a separate
orphaned CLI process from the failed pipe integration test was stopped. The
custom-protocol shell and unsigned NSIS installer then built successfully.
`verify-artifacts.ps1` passes. The installer is newer than `ui/dist` and the
shell, and the rebuilt bundle contains the session-switch/reconnect code. The
manifest records source revision `ca0b6e90`, version 0.0.15, x64 MSVC and
unsigned status. npm first failed under network-restricted execution with DNS
`ENOTFOUND`; lockfile-pinned `npm ci` passed with network access (133 packages,
0 vulnerabilities), then preparation passed.

The Windows workspace suite remains incomplete: `mcp_stdio` failed twice at
backend pipe startup with access denied and then a dependent assertion. The
failed test left an `audiorouter-cli.exe` child open; this agent stopped that
test-owned process. Do not claim the workspace suite passed. The exact-package
fresh-install gate is deferred while the user uses AudioRouter for game audio;
do not stop it. No `AUDIOROUTER_ALLOW_DEVICE_ADMIN` override, host driver
installation or host setting change occurred.

Task 2 host preparation is complete: `7802c9b2` adds the runner to the bundle
preparer and documents the procedure. All 153 host acceptance checks pass.
The r12 bundle is ready at
`C:\VMs\ar-share\diagnostics-20261010-cable-continuity-r12`; its 38-file
manifest verifies, and SYS SHA-256 is unchanged. The one-hour inside-VM
run completed. The returned archive's SHA-256 matches the user's hash. It
reported 1,456 lost impulses over four routes, no corruption, and
`unexplained-events` (`Qualification=false`). Loss events overlap across
multiple routes even when the strict all-four-route ≥20 ms gap test does not
classify them VM-wide; see the detailed analysis in
[r12 continuity evidence](evidence/2026-10-10-m03-cable-latency-preparation.md#one-hour-guest-result-r12-2026-10-10).
Do not treat the VM-wide label as root-cause attribution. Next, inspect the
r12 raw event timing and counter deltas, then locate the standing delay from
r11 data before designing recovery (spec 17); do not rerun the unchanged
one-hour test.

Driver work is organized in the
[execution queue](virtual-cable.md#execution-queue-after-r12-2026-10-10):
agent analyzes r12 timing/counters and localizes r11 standing delay on the
host, then prepares a verified short VM diagnostic or recovery check. The
saved r11 replay reproduces the 34.379→64.446 ms shift at impulse 148. Both
r11 archives are present and their hashes match; the user has no new guest
command to run until a bundle measuring or repairing the identified issue is
ready. Recheck the classifier's nominal event dating before attributing r12
losses, since saved QPC clocks are available.

Driver follow-up: offline QPC review reproduced all losses and found about
4.8 s nominal-date drift; only 119 lost impulses lack nearby all-route gaps
under the corrected correlation. The saved Cable A bridge WAV places a
20 ms payload shift before the engine, while end-to-end dating reports
30 ms. Submitted payload frames may have slipped relative to device-clock
frames, so discarding a guessed driver backlog is not yet justified. The
runner dating error is fixed; an opt-in, preallocated route/submission trace
and short r13 runner are prepared to resolve the missing measurement. Next:
verify the r13 manifest and ask the user to run the two-minute engine trace
from the latency runbook, then review its archive before recovery changes.

r13 preparation complete: clean source `a4949be8`, verified 38-file bundle
at `C:\VMs\ar-share\diagnostics-20261010-cable-stage-trace-r13`; signed SYS
hash unchanged. The user can now run the short trace block in AR-DriverTest.
23 tone tests, 2,203 probe checks, 189 continuity checks, 233 repeated-start
checks, QPC review regressions, formatting, Clippy and docs validation passed;
this is host diagnostic evidence, not a new driver audio qualification.

For release 0.0.15, fresh-install qualification remains deferred while the
user uses AudioRouter for game audio. When they are done, ask them to close it
and run the exact packaged fresh-install gate. Resolve the Windows workspace
pipe-test environment issue. Then ask before pushing the tag or
creating/publishing a GitHub release. Rollback: keep 0.0.14 and compatible
configuration/recording backups; the local release tag is unpushed.

r13 returned and reviewed: SHA-256 matches the user. Six runs retain 53
lost impulses: default 0/1/0 and low-latency 19/16/17. Default median latency
is 37.35–38.09 ms; the r11 persistent +30 ms jump did not recur. Independent
source-WAV pulse codes agree with the render trace, and render/capture
publication pulse order agrees in every run. Fourteen losses precede the
observed route input; 39 follow sink publication. This narrows the boundary
without establishing a kernel cause. Next agent task: inspect capture
packet retention/delivery and probe worker scheduling before another guest
command; no unchanged hour run or guessed backlog drop. Details:
[r13 result](evidence/2026-10-10-m03-cable-latency-preparation.md#r13-returned-stage-trace-review-2026-10-10).

Capture investigation: probe std::threads lacked COM/MMCSS setup while route
workers had it. The corrected probe now registers both workers, waits for
readiness and rejects worker API/scheduling errors. Short raw traces add
capture device positions, frames, flags, packet timestamps and reader/wake
QPC observations. Driver GetReadPacket returns the newest completed packet;
an older physical slot may be overwritten, so returning guessed old data is
not a safe recovery. Host static build, 2,208 offline probe checks, 253
repeated-start checks and 189 continuity checks pass. Next: verify a new r14
bundle with identical SYS, then the short reader comparison; no hour run.

Driver-agent resumption: read the [Claude / next-agent handoff](virtual-cable-agent-handoff.md)
first. It records the prepared candidate, exact next guest step, completed
checks, open defects and host/VM safety boundaries as of 2026-10-10.

Updated 2026-10-07. Completed work through the code review P0/P1 fixes is in the
[2026-10-04 to 2026-10-07 execution record](../archived/2026-10-07-maintenance-0.0.12-to-code-review.md);
the current work is the code review P2/P3 follow-ups below.

## Objective and scope

Support users of the published releases and close the remaining qualification
gates. Preserve sessions, local API credentials, device formats and published
assets. Requirements DSP-19, GRAPH-08/14/15, UI-05/08/11/12/13/17, AUTO-15,
ARCH-04, SEC-01/10 and DIST-01–08 stay traceable through the release evidence.

## Where things stand

- [v0.0.7](https://github.com/MrDesjardins/audiorouter/releases/tag/v0.0.7)
  (tag `ac29df97`): Rust workspace 979/0, shell 44, UI 458, full development
  Edge 202/0, exact-package fresh install and published-asset verification
  pass. Unsigned.
- Completed work of 2026-10-03 (0.0.6/0.0.7) is recorded in the
  [execution record](../archived/2026-10-03-releases-0.0.6-0.0.7.md). Earlier
  history is indexed in [archived plans](../archived/README.md).
- Other active plan: [app/installer signing](release-signing-and-publication.md),
  blocked on the user's provider choice and enrollment.

## Decision (2026-10-05, user): GPL-3.0-only with commercial licensing (DEC-19)

The user asked to stop others from profiting from AudioRouter while staying
open source and keeping the option to earn money later. CC BY-NC was
considered and rejected: it is not an open-source license and Creative Commons
advises against it for software. Chosen: `GPL-3.0-only` for versions after
0.0.13, with commercial licenses offered by the sole copyright holder
(Patrick Desjardins, author of every commit). Releases up to 0.0.13 stay MIT.

- Changed: `LICENSE` (standard GPL-3.0 text from GitHub's license API, since
  gnu.org was unreachable), workspace and shell `Cargo.toml` license fields,
  README License section, new `CONTRIBUTING.md` (contributors grant the
  copyright holder a right to relicense, which dual licensing needs), and
  website wording ("GPL-3.0 open source", footer).
- Compatibility checked: shipped Rust dependencies are MIT/Apache-2.0/BSD/
  ISC/Zlib/Unicode/MPL-2.0 or LGPL-3.0 (LAME), all GPL-3.0 compatible; VST2
  uses AudioRouter's own ABI declarations, not Steinberg's VST2 SDK; the VST3
  SDK is MIT. The MS-PL driver stays a separate program under its own license.
- Not legal advice; a lawyer should review the commercial-license terms
  before any are sold. Rollback: restore the MIT `LICENSE` and fields.
- Next: the next release ships under GPL-3.0; no code change is needed.

## Decision (2026-10-05, user): ship AudioRouter's own virtual cable (DEC-18)

**Current goal (2026-10-12): complete the driver.** The definition of done,
the Stage A gap table, the critical path and the next action are in the
[driver completion goal](virtual-cable.md#goal-complete-the-driver-set-2026-10-12-user-request).
The history below is earlier execution state.

Current driver task (2026-10-09): qualify sustained tone continuity after the
render-publication repair. Latest packet-clock r2: preparation and 30 seconds
pass, but the five-minute attempt failed and exposed a 38.5-minute synchronous
reporting block that extended active recording past its deadline. The harness
now uses bounded deferred reporting and a process watchdog; host regressions
and the checksummed bounded-tone update are complete. Its 30-second guest
check passes: leases stop at 30,001 ms, child exit 0 without timeout, zero
error counters. Its five-minute run ends on time but loses audio: both audio
loops stall about 310 ms and the control loop about 317 ms in the same
observation interval. Root cause remains unidentified; next is an opt-in guest
scheduler trace, preserving WSL and the failed continuity gate. Its initial
GeneralProfile startup failed with 0x80070032 before tone began. The minimal
profile's two-second guest probe now starts/saves successfully; the copied
trace decodes with scheduling/DPC/ISR event families and zero reported lost
events. The 300-second traced run now fails with 7,056 capture underrun and
14,832 render overrun frames. Saved trace analysis finds worker waits up to
35.7 ms, predominantly before readiness; the longest wake runs at priority 24
within 30.2/161 us for capture/render. An offline reader interprets the common
version-5 context-switch fields and matches independent event counts. Guest
timer versus host/VirtualBox attribution remains open. Next: prepare paired
host/guest timing diagnostics before another audio run; do not repeat the
unchanged guest-only test or alter buffers/counters/WSL based on correlation.
Paired recorder/clock probe scripts are prepared and local checks pass. Real
host recording remains unavailable to the agent's non-administrator Windows
token. The user has now run that host probe successfully: saved event coverage and
zero-lost counts pass review, and WPR is idle afterward. The paired two-second
probe also passes trace coverage/loss review. Guest UTC drifts relative to QPC;
normalized QPC brackets remain compatible within 22.45631 ms during this short
probe. Attribution must use raw QPC with that uncertainty, not UTC alignment.
The separate copy-only paired-tone update is published and independently
verified (35 files, all 32 base files unchanged). The paired 30-second run now
passes counters/deadline, both trace loss checks and compatible QPC brackets
(23.0877 ms). The user reports continuous hiss mixed with the beep through
Cable B, ending with the script. The saved Cable A WAV does not verify that
path. Hold longer runs; the [automatic direct audio diagnostic](../../operations/virtual-cable-direct-audio.md)
generates Cable A audio and records Cable B without default/microphone fallback
or Audacity installation. The first guest attempt passed status but the new
recorder exited before readiness with empty output. PE inspection confirms a
dynamic Visual C++ runtime dependency missed by the original packaging.
Repair uses static linking, a PE import gate and saved numeric/hex startup
exits; no redistributable or driver installation. The r2 recorder now succeeds,
but the run fails with a 288-ms gap shared by all three audio workers and
12,912 lost frames in each bridge direction. Both active WAVs span 29.73 s.
Direct Cable B samples are clean between the single loss event; the user's
continuous hiss points to further Listen/speaker playback investigation.
Cable A also has separate phase breaks at 12.037–12.050 s. The offline
analyzer now retains waveform metrics when duration fails, with unchanged
pass criteria; both saved WAVs still fail. Hold further guest runs while
reviewing these separate failures. See the
[direct waveform evidence](evidence/2026-10-09-m03-direct-audio-preparation.md#direct-r2-recordings-and-reporting-repair-2026-10-09).
Sustained continuity, audible quality and exact cause remain unresolved.
User also hears hiss on saved-file replay through Speakers; that device only
offers PCM16 at 16/22.05/44.1 kHz. Keep its supported 44.1-kHz setting and
Cable B at 48 kHz. The clean recorded section quantizes identically to a
generated reference. New offline PCM16 reference files are published.
The user now reports static in all three sections of the independently
generated 17-second **44.1-kHz** speaker reference. Offline reinspection
confirms its header, clean sine samples and silent separators. Read-only VM
configuration/log review shows HDA, HostAudioWas and default Focusrite speaker
output; it does not identify a defective component. With Cable B Listen off,
the user now hears much less static but several distinct crackles remain in
the early/final portions, and the middle is not perfectly clean either. Keep
Listen off. The bounded guest speaker-loopback recording is now reviewed:
after a 64-frame alignment, the first 749,260 stereo frames (16.990023 s)
match the reference exactly, including both reported noisy sections and
silent separators. Audible crackles in those sections arise after this
capture boundary; the exact downstream component is unproved. The final
440 reference frames differ and further tone sections continue; packet
position gaps/flags are retained, not silently waived. No whole-recording
or driver pass is claimed. Next is read-only VirtualBox playback/backend
review before preparing a reversible comparison, as documented in the
[direct audio runbook](../../operations/virtual-cable-direct-audio.md#completed-supported-format-speaker-reference).
Installed-version source review now confirms a VM-specific DirectSound
comparison requires both backend selection and an override to avoid silent
WAS substitution. The powered-off guard, saved baseline, startup-log check
and exact rollback are prepared in the runbook. User applied the reviewed
host block and booted normally. Fresh startup log confirms DSoundAudio.
The guest recorder now stops at 25.111494 s after receiving 34.996825 nominal
seconds of samples and reaching its storage limit. Its first 16.990023 s of
samples remain exactly equal to the reference. User reports quiet separators,
scratching during capture and clean sound after capture ends. The subsequent
capture-off playback also scratches variably, sometimes six to eight times,
with more scratching as playback continues. This supersedes the provisional
capture-only explanation; DirectSound is not a demonstrated improvement.
The current boot log also reports a 248.755-second virtual-clock catch-up
failure and a 249-second guest heartbeat gap. Installed-version source shows
HDA uses the synchronous virtual clock, whose recovery can accelerate DMA.
This is a concrete timing hypothesis, not aligned proof of the audible cause.
Host power-event inspection was denied; no sleep or host cause is established.
User has now shut the VM down and completed the guarded rollback. XML confirms
default/WAS, retained HDA and no backend override; VM processes are absent.
The closed-session log records 11 HDA output transfers skipped while a
completion interrupt remained pending; source confirms this pauses DMA while
the host consumer continues. Whole-session totals do not identify individual
scratches. Later overnight HostSuspend/HostResume and host endpoint changes
are also logged and must not be attributed to the earlier listening run.
Next engineering step: source review confirmed that event render DMA can replay
uncommitted circular-buffer bytes without incrementing bridge counters. The
owning reader now tracks accepted absolute packet identities, silences/counts
missing frames and commits atomically with position progress. Fresh kernel
review found no remaining source blocker in this scoped repair; 620 offline
helper checks and x64/ARM64 WDK/catalog/source acceptance pass. Legacy clients before/without packet commits and
one-slot continuity remain unqualified. This finding does not prove the saved
phase-break cause or explain the independent HDA scratches. Keep the VM off
and further recorder/bridge runs held. The clean-source `28b989f5` candidate
is ready at `C:\VMs\ar-share\repair-20261010-render-commits`: 33 package checks
and all 32 manifest hashes pass. Both clean guest smokes and install/status
passed 2026-10-10; the r3 direct audio run failed: the repair's silence
substitution removed ~15 % of Cable A audio. The approved follow-up (slot
provenance plus packet outcome counters) passed two clean smokes and a clean
30-second direct run on 2026-10-10; a bounded 5-minute run is next.
No installed driver or host setting is changed by the agent. See the
[repair and validation record](evidence/2026-10-10-m03-render-commit-validity.md) and the
[clock review](evidence/2026-10-09-m03-direct-audio-preparation.md#directsound-playback-with-capture-off-and-clock-review-2026-10-10).
Preserve WSL and the separate measured scheduling/loss failures.
Commands are in
[Administrator PowerShell](../../operations/virtual-cable-paired-trace.md).
See the
[packet-clock review](evidence/2026-10-09-m03-packet-clock-review.md) and active
virtual cable plan. The [earlier review record](evidence/2026-10-09-m03-render-publication-review.md)
and [virtual cable execution plan](virtual-cable.md) record the guest's passing
smoke cycles, four-endpoint format inventory, and 30-second tone run. The
first 10-minute attempt failed with earlier audio counter errors and a
14m55s VirtualBox guest-execution stall. VirtualBox reports NEM “Snail” mode
under the active Hyper-V hypervisor. The user requires WSL to remain
available; do not disable Hyper-V-backed features. The guide's 4-vCPU/8-GB
settings are already in use. A supervised 300-second run with the VM kept
awake and foreground completed without a long pause, but recorded four bursts
of audio errors (16,704 capture underrun frames / 348 ms; 19,152 render
overrun frames / 399 ms; maximum pump gaps about 163 ms). The user confirms
the VM did not sleep and no screen saver ran. This rules out those proposed
causes, but does not identify why the VM was intermittently descheduled. Keep
the continuity gate failed; review the run evidence and scheduling diagnostics
before any further long run. Source review confirms the counters track actual
loss: the recorded WAV is short by 40 ten-millisecond blocks, closely matching
the render-overrun total. The driver's bounded queues provide only tens of
milliseconds of headroom; absorbing a 163 ms pause would require comparable
buffer headroom and transient latency, conflicting with the virtual-cable
latency target. The deeper review subsequently found packet-count drift,
unbounded capture catch-up, a one-packet timestamp error and fractional carry
retained across STOP, plus long-uptime timestamp conversion overflow.
These are repaired in the
[packet-clock review](evidence/2026-10-09-m03-packet-clock-review.md).
Host regressions/builds pass; fresh VM continuity is still pending. Next:
use the [bounded retest procedure](../../operations/virtual-cable-packet-clock-retest.md).
No test driver has been installed on the host; WSL and Hyper-V remain enabled.

AudioRouter will ship its own signed virtual cable so users do not need
VB-Cable; VB-Cable/Voicemeeter stay supported. Lowest-cost signing
(attestation, EV certificate bought only for driver-release windows).
Recorded in [15-delivery DEC-18](../../spec/15-delivery.md); full plan,
costs and phases in the [driver track](../future/M03-driver-signing.md).

- Requirements: VDEV-01–12, SEC-08, NFR-16, DIST-01/03/07 (amended).
- Prerequisites for phase 1: a Hyper-V VM (Windows 11, Gen2) or a spare PC,
  created by the user; test signing only inside it. Nothing is installed or
  changed on the daily workstation.
- **Execution plan with work packages WP-00…WP-15:**
  [virtual cable plan](virtual-cable.md). Design: [17 Virtual
  cable](../../spec/17-virtual-cable.md). Signing runbook:
  [virtual cable signing](../../operations/virtual-cable-signing.md).
  WP-01 host baseline passed; WP-02 package tooling complete (2026-10-05),
  with [build/signing evidence](evidence/2026-10-05-virtual-cable-wp02.md).
  No driver has been loaded; VM evidence is pending.
- Test procedure (stages A–E, VM to main PC): [virtual cable
  testing](../../operations/virtual-cable-testing.md).
- WP-03 smoke tooling is prepared and WP-04 host bridge hardening is reviewed;
  their VM gates (install/remove, Verifier/fuzz, second-user denial) remain
  pending. WP-05 host endpoint, precision and format work is in progress; see
  its execution record and evidence in the [virtual cable plan](virtual-cable.md).
  Current WP-05 cable descriptors compile for x64/ARM64 and pass portable
  INF/conversion checks, but remain non-loadable until timing/cleanup work,
  WP-06 bridge/Rust compatibility and the VM gates are complete. Host builds
  are not runtime or sound-quality evidence.
- Needs a separate user go-ahead: buying the EV certificate (phase 5) and
  the first public driver release.
- Cables: up to 8 (A–H), user enables 1–8, default 2; develop and test with
  2, qualify all 8 before signing (user, 2026-10-05).
- Open choices: see WP-00 D2–D6 in the [virtual cable plan](virtual-cable.md).
- Rollback: the driver is a separate optional package; releases without it
  keep today's VB-Cable workflow. Revert DEC-18 doc changes to return to
  DEC-16.

## Decisions (2026-10-03, user)

- Signing: keep publishing unsigned prereleases while the project builds the
  public track record SignPath Foundation expects; apply later. The signing
  plan stays parked.
- Authorized now, in this order after items 1–2: Siege compressor
  simulation (item 3); Surround to headphones for 44.1/96 kHz 5.1/7.1 devices
  by resampling; the API request builder from the
  [external app integrations plan](../future/external-app-integrations.md);
  spatial speaker mode and distance/room controls from the
  [spatial audio plan](../future/spatial-audio.md). Each gets its own section
  here with requirement IDs, steps, validation and rollback before coding.

## Open work

### Code review P2/P3 follow-ups (2026-10-07, user request)

- **P2-1 (control-plane split).** `crates/control/src/lib.rs` went from
  33,117 to 1,290 lines. Code moved, without behavior change, into 22
  domain modules. The largest is `api_output_schema` at 2,045 lines. Tests
  moved to `<module>_tests.rs`, with shared fixtures in `test_support.rs`.
  Checks: 244 tests before and after, an identical `pub` item list, and
  item-by-item equivalence. Workspace Clippy (windows-gnu, `-D warnings`)
  and fmt are clean. The tests were not run here (they need Windows), and
  the test binary does not link in the container (`-lSwdevice` is
  missing). The typed method table was not done. Module list in the
  [review](../future/code-review-2026-10-07.md#p2-1-the-control-plane-is-one-21500-line-file).
  Next action: a Windows CI run of `cargo test -p audiorouter-control`.

- **P2-7 (DSP loops).** Engine block kernels (fixed delay, gain ramp,
  interleave/PCM16 conversions, channel matrix, `map_from`,
  `mix_mapped_from`, linear and streaming resamplers, voice-chain
  interleave) borrow planar channel slices once and run channel-major with
  `zip`/`chunks_exact`; their per-sample `unwrap`s are gone. The fixed
  per-route cost was `BlockMeter::observe` (80% of a bare Gain route in
  callgrind: eight meters per quantum, each making about eight passes); it now
  makes one vectorizable pass plus the sequential f64 sums. Output is
  bit-identical (`tests/planar_kernels_match_reference.rs` passes on the old
  and new code). `cargo bench --bench realtime` (Linux container, release):
  Gain 23.4 → 3.5 µs/quantum, ParametricEq 24.3 → 3.7, Denoise 66.3 → 46.4,
  route-32-tools 680.0 → 507.6. Continuity harness on Windows not run.
- **P2-2 (UI lint, format, App.tsx).** ESLint (typescript-eslint,
  react-hooks) and Prettier added to `ui/`; one mechanical format commit
  (`2697774b`); CI runs `format:check` and `lint`, the pre-commit hook
  formats staged UI files. 0 lint errors, 25 reviewed `exhaustive-deps`
  warnings. A proven stale closure is fixed: Ctrl+Alt+S after Save
  started a temporary preview instead of the saved session
  (`e2e/shortcut-after-save.pw.ts`). `App.tsx` 8,543 (formatted) → 3,069
  lines: helpers, four polling hooks and seven panels moved verbatim to
  their own files. Linux container: tsc clean, Vitest 56 files / 498
  tests before and after, build OK, harness-only Playwright (Linux
  Chromium) 152 → 153 passed, HTML of every tab and node inspector
  identical in three themes. Not run: backend-harness Playwright, Windows
  CI. Next: moving AppContent's action handlers (state flow) into hooks.

**P3-1 unsafe audit.** Every `unsafe` block and impl now has a
`// SAFETY:` comment with its invariants (pointer and handle validity,
owner and lifetime, buffer lengths, COM thread). Before: 188 missing
(windows-audio 98, transport 27, plugin-host 19, engine allocator test 4,
shell 26, `tools/m00-wasapi-probe` 14); after: 0. The lints
`clippy::undocumented_unsafe_blocks` and `clippy::missing_safety_doc` are
`deny` in `[workspace.lints.clippy]` (each crate has
`[lints] workspace = true`) and in `[lints.clippy]` of `src-tauri` and the
probe, so all targets, tests and examples included, are covered.
Soundness fixes: `SharedCapture::next_packet_into` and
`SharedRender::submit_bytes` reject `bytes_per_frame` above the stream's
`nBlockAlign` (a larger value read or wrote past the WASAPI buffer);
`created_callback` clones its sender before sending, so `create` may free
the context while `send` runs; `token_user_sid_string` reads `TOKEN_USER`
with `read_unaligned` from its byte buffer; the OS-transition and plug-in
editor windows no longer free their boxed context in `WM_NCDESTROY`, which
could double-free when `CreateWindowExW` fails after `WM_NCCREATE`; the
probe's `raw_capture_initialize` no longer calls `CoUninitialize` before
its interfaces are released.
Open: `NativeBridgeRegion` and `SharedAudioRegion` write through pointers
taken from `MmapMut`'s shared slice (`map.as_ptr()`), which Rust's aliasing
model does not allow; move them to `memmap2::MmapRaw`. The `enumerate_*`
helpers return a `windows::core::Error` (which may hold COM error info)
after `CoUninitialize`.
Local evidence (Linux container, pinned 1.96.0, `x86_64-pc-windows-gnu`):
workspace and shell Clippy with `-D warnings` clean; probe Clippy clean;
`cargo fmt` checks clean; engine, storage, domain, dsp, recording,
protocol and plugin-host tests pass. Windows CI and hardware runs not done
here; the Jev rule check was not run (no tool checkout or key).
Rollback: revert the commit.

- **P2-4 supply chain:** Dependabot, `deny.toml` and a `supply-chain` CI
  job (cargo deny for both workspaces, `npm audit --omit=dev` for ui,
  contracts and Stream Deck); actions bumped to Node 24 releases. Five
  unmaintained rust-unic advisories in `src-tauri` are ignored until
  2027-01-07 (needs a Tauri update checked on Windows). Local: both
  `cargo deny` runs and all three audits pass. Details in the
  [review](../future/code-review-2026-10-07.md#p2-4-dependency-and-supply-chain-checks-are-missing).
  Next action: first CI run of the `supply-chain` job.
- **P2-8 coverage and soak:** nightly/manual `quality.yml` (Windows):
  `cargo llvm-cov` and Vitest coverage (not gated), a 20-minute engine soak
  (`crates/engine/examples/soak.rs`, gates on heap growth, allocations and
  p99 per quantum), and the backend part of NFR-14 timed by
  `fresh_install_shell`. Local: 10 s soak passes; UI coverage 74.5 % of
  lines; portable-crate coverage 86.5 %. The root README now shows current CI
  and quality-workflow badges; successful main-branch coverage/soak/startup
  runs publish linked measurements there. UI-ready startup time is not
  measured yet. Next action: after this change reaches `main`, dispatch
  `quality.yml` once and verify the first README refresh.

- **P2-5 network pairing key** (SEC-13, GRAPH-16; branch
  `worktree-agent-ae33fb03459795579`). Network Send/Receive take an
  optional `pairingKey` (domain validates blank or 16–128 printable ASCII).
  Wire format: version 2 = version 1 plus a 16-byte truncated HMAC-SHA256
  tag over header and samples; key = HMAC-SHA256(pairing key, fixed
  context). Tagging runs on the send I/O thread and verification plus the
  replay window (`crates/protocol/src/network_audio.rs`, portable) on the
  receive thread, never on the audio thread. Counters `authFailures`,
  `authProblem` and `replayedPackets` reach telemetry, `network.jsonl` and
  the inspector; the key is never logged (no log writes parameter values;
  MCP activity also drops `pairing*` argument names, and the inspector's
  change summary hides the value). A key change applies in place on
  both nodes. Decision: version 1 and 2 are mutually exclusive, so a paired
  node cannot talk to an older AudioRouter (documented).
  Evidence (Linux): protocol unit tests (pinned tag computed with Python
  `hmac`); domain validation tests; Windows-target Clippy clean for
  protocol, domain, windows-audio and control; the `network_audio.rs` socket
  tests run on Linux in a throwaway harness (stubs for the crate types):
  all pass except the pre-existing IPv6-loopback test, which also fails on
  the unchanged file in this container; Vitest and the new
  `ui/e2e/network-pairing.pw.ts` in Chromium, three themes, no layout
  shift. Windows CI must still run `windows-audio` and the extended
  `network_settings_corrected_while_playing_take_effect_without_restart`
  control test. Rollback: revert the commit; sessions with a key then fail
  validation until the key is removed.
  Next action: Windows CI on the branch, then a two-PC attended check with
  the same key, a wrong key, and one side blank.

- **P2-3 request IDs, verbose mode, support bundle** (commit `00ecb1ed`).
  Protocol: optional top-level `requestId` member, which older backends
  ignore. The logic is portable and unit-tested in
  `crates/protocol/src/diagnostics.rs`. New methods are
  `diagnostics.getVerbose` (`read`) and `diagnostics.setVerbose`
  (`sessionControl`, one hour, expires by itself). The Logs tab has a Verbose
  switch with a fixed-width countdown (UI-17) and "Copy support bundle"
  (`export_support_bundle`, a ZIP in the logs folder, path-filtered, local
  only). Decision: the ID travels beside `JsonRpcRequest` in wrappers
  (`CorrelatedRequest`/`IncomingRequest`), not as a new struct field, to
  leave its ~290 literal constructions unchanged. The HTTP forward signature
  now takes the ID.
  Verified on Linux: `cargo test -p audiorouter-protocol` and `-p
  audiorouter-domain`, the support-bundle tests in a scratch crate,
  `npm --prefix ui test` (493 passed), `tsc`, the Logs e2e in Chromium in
  dark, light and high-contrast, Windows-GNU Clippy for the workspace and
  `src-tauri`, and `tools/docs/validate.mjs`.
  **Next:** let Windows CI run the transport, shell (`http_api`,
  `support_bundle`, shell log) and `mcp_stdio` tests and
  `check-drift.mjs`. Then, in an attended run, click Play with verbose on,
  confirm one ID in the client row, `shell.jsonl` and `backend.jsonl`, and
  open a support bundle.

### Release packaging: one download (2026-10-06, user request)

User found the 0.0.13 release page confusing (16 assets) and asked for one
package containing the app, backend, frontend and Stream Deck plugin.
Finding: the installer already held the shell (UI embedded, backend in
process), CLI and plugin worker; the loose `.exe`s, `audiorouter-ui.zip`,
`run-vb-cable-desktop.ps1` and examples ZIP were duplicates or developer
files. Changes (requirement area M08 distribution, HTTP/Stream Deck):

1. `tauri.release.conf.json` bundles the packed Stream Deck plugin;
   `prepare-artifacts.ps1` and `m08-installer-smoke.ps1` pack it before NSIS.
2. Shell command `install_streamdeck_plugin` opens the bundled file with
   `ShellExecuteW` (Stream Deck asks to confirm); API tab gains
   `StreamDeckPluginPanel` with a reserved status line (UI-17).
3. Published assets: installer, `.streamDeckPlugin`, `SHA256SUMS.txt`,
   `release-manifest.json`, SBOMs, notices. `verify-artifacts.ps1`, its test
   (which still lacked the 0.0.12 Stream Deck entries) and `m08-release.ps1`
   updated; `m08-release.ps1` now rejects loose copies of bundled files.
4. `create-draft-release.ps1` prepends a "Which file do I download?" section.

Checks run (Linux container, 2026-10-06): `vitest` for
`StreamDeckPluginPanel`/`ApiPanel` pass; UI typecheck clean;
`npm run pack` in `tools/streamdeck` produced a package with
`com.mrdesjardins.audiorouter.sdPlugin/manifest.json`.
Not run (need Windows): shell compile, PowerShell release scripts and
tests, NSIS build, installing the plugin from the installed app, three-theme
screenshots of the API tab. Next: on Windows run `test-verify-artifacts.ps1`,
`m08-installer-smoke.ps1`, `m08-release.ps1`, then install the 0.0.14
candidate and press **Install Stream Deck plugin** with and without the
Stream Deck app. Rollback: revert the commit; earlier releases are unchanged.

### Defect 2026-10-05: backend dead after resume ("Backend refresh failed")

Reproduction (v0.0.12 release shell, PID 17188): sign-in start at
2026-10-04 19:05 with `patrick-main-native` running; lock + sleep at 19:20
(only `lock` was journaled); resume 2026-10-05 16:30:01, when the backend
still ran endpoint inventory (`discovery.jsonl`, `getDevicePeriod`
0x88890008). Opening the window at 19:39 showed "Unable to load your saved
session: Backend refresh failed", and every shell RPC was `transportError`.
The `\\.\pipe\audiorouter-control` pipe no longer existed, the process's live
threads totalled ~1.5 min CPU of 2 h 23 min (the 1 ms service thread had
exited), and `recovery_crashes` was empty, so the supervisor's error path
never ran: the control thread **panicked**. A panic bypassed the
`BackendSupervisor` loop, and release builds have no console, so the panic
location was lost.

Fix: `serve_control_connections_forever_observed` catches a control-plane
panic, wakes and joins its I/O thread (which otherwise keeps the only pipe
instance; max instances is 1), and returns an error. The shell supervisor
also wraps each attempt in `catch_unwind`, so a panic is counted and
restarted like any other failure (safe mode after repeated failures). A
panic hook writes a `shell.panic` record (thread name, file and line; no
message text) to `shell.jsonl`. Regressions:
`stopped_control_plane_releases_the_pipe_for_a_restarted_server` (fails with
the wake disabled) and `shell_panic_log_keeps_location_and_thread_but_no_message`.
Checks 2026-10-05: `cargo test -p audiorouter-transport --lib` 26/26,
`cargo test --manifest-path src-tauri/Cargo.toml` 61 passed, 1 ignored.

Released in 0.0.13 (2026-10-05): see the
[execution record](../archived/2026-10-05-release-0.0.13.md) and
[release evidence](evidence/2026-10-05-release-0.0.13.md).

Still open (next action): the panic site itself is unknown. After the next
occurrence, read the `shell.panic` record, fix the owning code and qualify a
sleep/resume with a native route running. A restarted backend leaves routes
stopped until Play.

## Risks, evidence and rollback

Stats.cc's undocumented feed can change; unknown state releases Duck.
Independent clock drift and queue latency remain limits. Keep the previous
installer and compatible configuration/recording backups for rollback; no
migration or driver install. Published assets are immutable; repairs use a new
version.

## Tool-guide animation asset — 2026-10-14 (bounded website task)

Objective: accessible, locally hosted explanatory animation runtime for the per-tool guides (UI-04/07/09/12/17; M05 documentation). Preserve launch film and unrelated work. Use deterministic native WAAPI, the HyperFrames supported adapter, with static input/output comparisons, finite eight-second motion, Play/Pause/Replay, seek API, offscreen pause and reduced-motion defaults. Source-only website delivery: no publication, dependency changes or MP4 batch. Node 22.22.3 and Windows Edge/Chrome exist; cached HyperFrames 0.8.107 exists; ffmpeg/ffprobe are absent from PATH.

Tasks: add isolated site/assets/guide-animation.js and an embed contract; inspect browser output and controls across representative visual families, keyboard and reduced motion; parent integrates guide pages and runs site checks. Validation is explanatory UI evidence, not actual DSP or latency evidence. Rollback: remove this runtime and guide script references only. Risks: illustrations must remain explicitly illustrative, never claim measured DSP behavior. Next: build and inspect runtime.

Animation runtime evidence (2026-10-14, Windows host): JavaScript parsed with Node vm.Script; headless installed Edge via explicit executablePath exercised all 16 visual types, seek(4) yielded 4000 ms paused state, repeated parameter updates/destroy left no stale figure. Eight parameter families changed actual SVG path geometry (gain, compressor, gate, limiter, duck, EQ, delay, pitch); destroy left zero browser animations. Reduced-motion initially showed static state; deliberate Play/Pause and keyboard ArrowRight on seek worked. Zero browser page errors. At 390 px EQ controls remain in bounds. Reviewed compressor image; scratch browser images at ui/test-results/guide-animation/{compressor,eq-mobile}.png (not committed documentation evidence). Playwright channel discovery initially missed Edge; explicit Program Files (x86) path resolved it. Library source contains no standalone HyperFrames composition; no video render or HyperFrames composition check was performed.
