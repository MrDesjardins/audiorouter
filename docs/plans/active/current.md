# Active plan — post-0.0.8 maintenance

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
