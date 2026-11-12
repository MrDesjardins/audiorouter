# Plugin worker packaging and copyable errors — 2026-11-11

Objective: repair the owner's missing-worker failure and make visible errors
copyable (PLUG-04/05/07, UI-08/11/17, ENG-05; M05/M06/M08). Issue
[#136](https://github.com/MrDesjardins/audiorouter/issues/136) reports all
plugins failing but supplies no names, versions or logs; its cause is unconfirmed.

Reproduction: backend.jsonl records plugins.parameters failures for process
51084, requests REF945K4/J7K2CVFZ, followed by nativePaths.prepare failure
AWECAZ1E. The running shell is in .worktrees/audio-latency/src-tauri/target/release;
neither worker is beside it or in resources. Pitchproof has x86 and x64 DLLs
installed under Common Files/VST3/Pitchproof. Preserve the live route and all
unrelated virtual-cable edits on fix/virtual-cable-review.

Prerequisites: Windows, Node, Rust and native compiler available. Installed
plugins may be exercised offline without touching live audio. No permission
to open a PR, publish a release, change drivers or interrupt the owner's audio.

Tasks:
1. Inspect saved session and reproduce offline parameter/state requests.
2. Add Copy error to global and panel errors, preserving original text on
   clipboard failure and reserving feedback space (no layout shift).
3. Provide one complete local desktop build command that builds and stages both
   workers; validate all inputs before staging. Repair the running build's
   missing worker files without restarting or changing its session.
4. Exercise clipboard success/denial and message replacement, offline installed
   VST2/VST3 state round trips, packaging guards, UI typecheck/lint/build and
   three-theme browser rendering. Record exact evidence and limits.

Rollback: revert only this task's source/docs changes; remove only newly staged
worker files if necessary. No database migration or plugin redistribution.
Existing installer resources already include both workers; do not infer the
reporter's installer is broken from the owner's incomplete local build.

## Implementation and Windows evidence

The saved PD200X Radio Voice revision 15 contains pitchproof-x64 1, an enabled
mono VST2 node between Mixer 1 and the stereo Advanced EQ, fanning out to two
outputs and Recorder. Its DLL hash matches the saved fingerprint:
`1974a3033b53ae72da5f419a9f37056d44c1610591bfd11a615ace0c448cf050`.
Read the live database through Node SQLite in read-only mode; no owner data
was modified. The file has no Windows product/file version resource.

Both rebuilt workers were copied only into the previously absent paths beside
the running audio-latency shell, with source/destination SHA256 equality checked:
VST2 `9378D6BDB835C839B8005E1B2365808BD3E9A48A5450B57DC90109F139C151C0`;
VST3 `707DBAD88A12BC6E54DB988036BB28B64D2C825D75EAAFEF7477D255BA7DABC9`.
No shell restart, endpoint preparation, driver change or route mutation was
performed. Existing installer configurations already package both workers.

`tools/build-desktop.ps1` now builds UI, CLI, shell and both workers together,
checks every executable, and prints hashes. The disposable desktop launcher's
Build option uses it, and the quickstart documents it as the local build path.
`tools/test-build-desktop.ps1` runs the real orchestration with mock builders:
missing VST3 output and failed Cargo are rejected, complete output passes.
The first test attempt exposed a PowerShell mock-variable scope mistake; fixed
the test's scope and reran successfully. This is packaging guard evidence,
not a substitute for the real build, which also passed (log below).

`CopyErrorButton` is shared by panel errors, the global error bar and startup
load errors. The full original text remains available; blocked/unavailable
clipboard exposes a selectable read-only field. Keyed state prevents a delayed
copy of an old error from labelling a replacement as copied. Stable button
width preserves geometry when feedback changes. Tests cover success, missing
clipboard, denied clipboard and an in-flight message replacement.

Windows 11 / Node 22.22.3 / pinned Rust 1.96.0:

- `cargo test -p audiorouter-control --lib`: 363 passed, 12 explicitly ignored.
- `installed_effect_describes_processes_and_restores_state -- --ignored
  --nocapture`: passed separately for installed Pitchproof x64 VST2 and UADx
  1176 VST3 with the staged release workers. Each invokes `plugins.scan` and
  `plugins.parameters` through ControlPlane::dispatch, then processes 200
  128-frame blocks at 48 kHz in mono and stereo, requires finite, non-silent,
  changed output, saves/restores state and restores into a fresh worker.
  Pitchproof peak 0.24434868; 1176 peak 0.18334171 for 0.25-peak input.
  This test is opt-in because it requires user-owned plugin binaries; generic
  unchanged/flat effects are intentionally unsuitable fixtures.
- `npm --prefix ui test`: 113 files / 1,004 tests passed before the final
  denied-clipboard and startup regressions; final focused tests 7/7 passed.
- `npx playwright test --config playwright.config.ts copy-error.pw.ts
  --workers 1` in ui: 3/3 Edge scenarios passed, dark/light/high-contrast,
  full-text copy and unchanged error geometry. Screenshots inspected in light
  and high contrast; artifacts in the local TEMP/audiorouter-playwright-results.
- UI typecheck:all, zero-warning lint, format:check and production build pass.
- Workspace and shell strict all-target/all-feature Clippy pass. Both Rust
  format checks pass. Contract drift: 147 methods / 37 node kinds, unchanged.
- Documentation acceptance: 249 Markdown files / 1,447 links, passed before
  this evidence update; diff check passes.
- Real `tools/build-desktop.ps1` passed; log `target/plugin-desktop-build.log`.
  The final startup-copy change required a subsequent UI/shell rebuild.

Warnings retained with disposition: Vite's 606 kB main chunk is outside this
localized error-copy/worker-packaging repair (no dependency changes); no startup
performance claim is made. Browser runner emits NO_COLOR/FORCE_COLOR and React
Flow hidden-attribution notices; no license or attribution policy change is
included. No new API method, NodeKind setting or tool property changed, so API
catalogs and Properties-only guide recordings do not change in this task.

Native experiment: an isolated shell with a private nonexistent backend pipe
exposed that the initial saved-session-load error bypasses the global bar.
The first probe timed out looking for Copy error. Added startup copy plus a
regression that fails without it; rebuilt the embedded UI. The owned test shell
was cleaned up. Native clipboard recheck passed after the final rebuild:
`node target/plugin-native-copy-check.mjs` launched the real release shell
against a private nonexistent backend pipe with isolated database, LOCALAPPDATA,
APPDATA and WEBVIEW2_USER_DATA_FOLDER. Clicking Copy error put the complete
startup error on the actual Windows clipboard (verified with Get-Clipboard).
Screenshot inspected; no audio endpoint opened. The test cleaned up only its
owned shell/process tree. Evidence:
`target/plugin-native-copy-check-final/{result.json,native-copy.png}`.
Final shell SHA256:
`DD4E76ED36EFBC47270021589C7184F15A82633634A091B571FA8A0442619419`.
Its 16:43:41 timestamp is newer than ui/dist/index.html (16:43:10).
Final full UI run: 113 files / 1,006 tests passed; shell strict Clippy passed
again after embedding the startup-copy repair. Documentation acceptance and
both Rust format checks passed again. Both new PowerShell scripts are ASCII.

Limits: no installed-plugin editor, attended PD200X playback, hardware latency,
soak, new installer qualification, hosted CI or reporter retest is claimed.
Issue #136 remains open and unconfirmed; no issue comment or PR was posted.
Changes remain uncommitted on fix/virtual-cable-review, beside preserved user
driver work. The locally built shell includes that existing work and is not a
release candidate for this isolated repair.

Next: owner retries Pitchproof parameter loading, Save plugin settings and
playback on PD200X Radio Voice. Keep issue #136 open pending its
plugin list/version/support bundle. A later release remains a separate gate.

## VST2/VST3 host audit (2026-11-11, owner request; read-only, no fixes yet)

Scope: `crates/plugin-host` (scan, spawn, sandbox, protocol, runtime bridge,
`vst2.rs`, VST2 worker), `tools/m06-vst3-worker/main.cpp`,
`crates/control/src/plugins.rs`, the Tools-tab scan UI and release packaging.
Requirement IDs: PLUG-01..07, UI-08. No code changed; no build run (the built
release workers and CLI from 16:34/16:40 were used).

Evidence, Windows 11, offline, no audio endpoint opened
(`target/plugin-audit/`): `sweep.mjs` drove both built workers with the host
protocol and the host's cleared environment (no job object) for every
supported installed plugin, mono and stereo, 200 blocks each, then Latency
and StateSave. 17 of 18 pass (8 UADx VST3, Pitchproof, 8 ReaPlugs). UADx
loads take about 1.0 s warm against the 5 s limit.

Reproduced:

- R1 ReaJS (`reajs.dll`): StateSave on an empty chunk ends the worker
  (`VST2 state asset invalid: Empty`). In the app this is Save plugin
  settings, Stop, and closing the editor while playing.
- R2 `plugins scan` (real CLI) fails the whole folder with `InvalidRoot` when
  the folder is a junction, `TooManyCandidates` over 256 `.dll`/`.vst3`
  files, `TooManyDirectories` over 256 subfolders (one valid plugin present);
  a junction inside the folder is skipped with no entry; a bundle with a
  second file beside its binary is `notPe`. "Scan standard folders" discards
  every such error and reports "Found 0 supported plugins".

Found by reading, not yet reproduced (no such plugin installed here):

- A1 The plugin is loaded before Hello; the host waits 5 s
  (`WORKER_RESPONSE_TIMEOUT`). VST3 UI-thread requests (state restore) have 4 s.
- A2 State: the JSON frame limit (1 MiB) caps a state near 290 KB, below the
  512 KB constant. Any save failure other than `StateUnsupported` makes
  `SupervisedWorkerProcess::save_state` kill a playing worker. A rejected
  restore fails Play for the whole route.
- A3 VST2 header check rejects more than 256 parameters, more than 2 outputs
  or 4 inputs. `LoadLibraryW` does not search the plugin's own folder. The
  host callback answers 0 to everything but version, sample rate and block
  size (`audioMasterGetTime` is null). Plugin stdout shares the protocol
  pipe. The editor runs on a second thread while the loading thread never
  pumps messages.
- A4 VST3: host objects answer `kNoInterface` to every query and
  `createInstance` is not implemented (no `IMessage`/`IAttributeList`);
  `ProcessData.processContext` is null; a single-component effect gets no
  controller (no sliders, no editor); only the first effect class of a
  module is reachable (shell modules); `require_result` ends the worker on
  `kResultFalse` from `activateBus(false)`, `setProcessing`, `process`,
  `getState`, `getParameterInfo`, and on duplicate parameter IDs; the load
  error has no Win32 code; the audio thread has no MMCSS registration.
- A5 Runtime: one non-finite sample or one failed exchange ends the plugin
  until the next Play (no restart on this bridge); one plugin that fails to
  start blocks Play of the whole route; errors reach the user as Rust
  `Debug` text.
- A6 Identity: node and saved state are bound to the binary SHA-256, so a
  vendor update needs a rescan and a re-add and drops the saved settings.
- A7 Sandbox: 512 MB commit limit and one process per worker (a VST2 chain
  shares both); environment allowlist has no `PATH`.
- A8 `tests/acceptance/m08-release.ps1` checks the VST2 worker in the bundle
  but not `audiorouter-vst3-worker.exe` (`prepare-artifacts.ps1` checks both).

Limits: only Universal Audio, ReaPlugs and Pitchproof are installed; no
JUCE, iPlug2, Waves, iZotope or large-state plugin was run, and no editor was
opened. Issue #136 is still unexplained. Next: owner decides the fix order;
proposed first batch is R1, R2, A1, A2, then a compatibility run with a set
of widely used free plugins.

## Audit repairs — 2026-11-11 owner authorization

The owner requested every finding be fixed. Requirements PLUG-01..07,
ARCH-04/09, SEC-07/09/12, UI-08/17, ENG-01/05 apply. Preserve unrelated driver
and prior copy-error edits; no PR/publication, driver or live-route changes.

Ordered work: reproduce and repair R1/R2; repair coherent state/frame bounds
and operation-specific deadlines (A1/A2); repair VST2 dependency search,
descriptor/channel adaptation, timing, protocol isolation and message pumping
(A3); repair VST3 COM interfaces/messages, timing, controller/class selection,
return handling, diagnostics and scheduling (A4); qualify runtime failures and
safe explicit recovery (A5), vendor-update settings recovery (A6), compatibility
resource/environment policy (A7), and both installer workers (A8).

Safety decisions: the audit mixes defects with intentional protections. Do not
automatically bypass a failed microphone processor, accept unverified state or
binary replacements, remove resource bounds, or silently restart after repeated
faults. Any retained protection needs an explicit finding disposition, regression
and actionable recovery. Scan junction traversal must retain canonical root
grants and loop/escape bounds. Installation of test effects is authorized by
this repair request, but prefer repository-owned native fixtures and local SDK
samples before third-party machine-wide installers.

Validation: focused unit and native fixture regressions fail before fixes;
compile/typecheck immediately after each edit group; real dispatcher and default
MCP/REST paths for new API; native workers mono/stereo state/parameters/audio
and editor lifecycle; UI three themes; source/tool layout suites for graph
changes; installer resource/acceptance checks; strict fmt/Clippy/lint/contracts/
docs. Record exact binaries and distinguish fixtures from installed products.
No claim about issue #136's cause without the reporter's diagnostics.

Rollback: revert only audited repair hunks, retain prior compatible state and
binary assets; no database migration unless explicitly recorded below. Next:
R1/R2 and the transport/timeout regressions.

### Repair results (2026-11-11)

Dispositions, all in the working tree on `fix/virtual-cable-review`, not committed:

- R1: an empty VST2 chunk saves as a parameter bank (`vst2.rs` test); ReaJS now saves 291 bytes.
- R2: scan caps raised (8192 candidates, 65536 directories); an explicit junction root is canonicalized, inner reparse points are listed as skipped `OutsideConfiguredRoot` (grants unchanged); scan errors reach the panel.
- A1: 30 s control deadline for Hello/restore/save, 35 s bridge request; `slow-load` fixture regression.
- A2: worker message bound `4 × state + 1 MiB` (const-asserted), half-megabyte worst-case JSON test; a refused save is a recoverable `UnsupportedFeature`, the worker stays alive (`state-refused` fixture).
- A3: 64 channels / 65536 parameters (4096 descriptors), `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR|DEFAULT_DIRS`, `audioMasterGetTime` and opcodes 23/34, protocol pipes duplicated and plugin stdio sent to NUL, owner messages pumped while waiting.
- A4: host QI for every host interface, `IMessage`/`IAttributeList` (bounded), `ProcessContext`, single-component controller, `--class-id` selection, `kResultFalse` tolerated for optional calls, Win32 error text on load failure, editor-attach failure text, MMCSS "Pro Audio" on the audio thread. `--self-test-host` passes.
- A5: launch/restore failures are `NodeNotReady` with a user sentence (Display, not Debug); "Start from defaults" is explicit and one-shot (see the review below); microphone processors are not auto-bypassed.
- A6: new `plugins.prepareUpdate` (pluginScan scope) re-links a node to the rescanned binary, copying its state under the new hash; MCP `prepare_plugin_update` and a recipe; dispatcher test `plugin_update_preparation_preserves_state_and_wiring_through_dispatch`.
- A7: Job limit 16 processes / 2 GB; `PATH` passed through.
- A8: `m08-release.ps1` and `test-installer-hooks.ps1` check the VST3 worker too.

Checks (Windows 11 Home, this PC): `cargo fmt --check` both manifests clean; Clippy workspace and `src-tauri` `-D warnings` clean; `cargo test` plugin-host (all features) 84+41 pass, control 364 pass, CLI 49 + `mcp_stdio` 5 pass (tool count 98→99); `check:drift` passes (148 methods); UI format/lint clean, 5 affected test files 149 pass. Offline sweep (`target/plugin-audit/sweep.mjs`, release workers rebuilt today): 36 of 36 runs (18 installed UADx/ReaPlug effects × mono/stereo) load, process 200 blocks, report latency and save state; Pitchproof (32-bit) and Polymax (instrument) are correctly refused.

Not run: the full UI suite and Playwright, three-theme screenshots of the new Properties controls, a live route through the built shell, the installer build, and the engine combination suites (no graph-compiler change). Issue #136's cause is still unknown without the reporter's diagnostics.

Next: owner review of the diff, then a commit staged by name (driver files excluded); no PR without approval.

### Code review of the repairs (2026-11-11, owner request; by reading the code)

Defects found in the repair diff and fixed, each with a regression test:

1. The UI transport gave every request 5 s, so a slow scan, plugin load at Play, state save or editor open showed "request timed out" while the backend's new 30 s allowance was still running. Plugin and `*.prepare` requests now get 60 s (`requestTimeoutMs`, `host.test.ts`).
2. A whole-library scan still had 10 s in total and failed completely at that point or at one unreadable subfolder. It now has 45 s, keeps verified results and lists the rest as `deadlineExceeded`; only an unreadable root fails (`a_scan_that_runs_out_of_time_keeps_its_candidates_listed`).
3. A scan resolved every root and remembered it under the resolved folder, so `plugins.list` with the typed folder could look empty, and on a PC whose temp or profile path has an 8.3 alias (the CI runner) three tests failed. The caller's spelling is now kept unless the folder is reached through a junction or link; only then is it resolved (ordinary drive form, no `\\?\` prefix) and listing accepts the link's spelling (`resolve_scan_root`, `an_ordinary_scan_root_keeps_the_callers_spelling`, `resolves_an_explicit_reparse_point_scan_root`, `a_scanned_folder_behind_a_link_is_listed_under_the_link_spelling`). Verified with `TEMP` set to an 8.3 path.
4. "Start from defaults" wrote a permanent `factory-default` marker that discarded every later editor or Stop capture at each Play. It now removes `stateId` and forgets the latest capture once (`Storage::clear_plugin_node_state`); state files are kept.
5. The VST2 channel rule refused graphs wider than stereo that worked before (6 in, 6 out). Equal widths pass again.
6. VST3 class selection compared one spelling of the class ID and reported "no audio effect class" on a mismatch. Both spellings match; a class that is not in the binary (stale `moduleinfo.json`) loads the first usable effect and says so on stderr (worker self-test).
7. A VST3 parameter with an out-of-range default failed the whole parameter list; it is clamped. Parameter and editor requests use the 30 s control deadline. VST2 process level is "realtime" only inside a process call. Skipped linked folders are bounded (256) and exclude linked files.

Reviewed without change: wire bounds (65 MiB message, 16 MiB state on both sides), VST2 time info layout and flags, DLL search flags, stdio isolation, host COM reference counting, single-component controller lifetime, authorization and schema entries for `plugins.prepareUpdate`.

Checks after the review: fmt and Clippy clean (workspace and `src-tauri`); plugin-host 86+41, control 365, storage 106, CLI 49+5 pass; drift, docs validation and site catalog check pass; UI format, lint, typecheck clean, 165 tests in 5 files pass; both release workers rebuilt, self-test passes, sweep 36 of 36.

Still not run: full UI suite, Playwright, three-theme screenshots, built shell on a live route, installer build. Residual risks read from the code, not exercised: a library that needs more than 45 s to hash is only partly listed per scan; the plugin job now allows 16 processes and 2 GB at all times.

### Verification run (2026-11-11, owner request)

- Full UI unit suite: `npm --prefix ui test`, 113 files, 1012 tests pass. `e2e:guard` qualified.
- Full Playwright (`e2e:development`, Edge, all themes): 731 passed, 15 skipped, 4 failed. `first-run-mic-compare` passed on rerun (load flake). `slow-scan` (three themes) found a real UI-17 regression in the Copy-error change: the button made an error line about 2 px taller than the "Scanning" line, moving the footer. Fixed in `styles.css` (the button stays inside the text line box); `slow-scan` is the regression guard and now passes.
- New `ui/e2e/plugin-recovery.pw.ts`: the recovery controls in dark, light and high contrast keep their size when an error appears, and the Copy error button is shown; screenshots in `target/ui-quality-review/vst-fixes/`.
- Installer: `tools/release/prepare-artifacts.ps1 -AppOnly` refuses a dirty tree, so the same steps were run by hand (both workers, MCP CLI, UI build, custom-protocol shell, Stream Deck pack, `tauri build --no-sign --bundles nsis --config tauri.app-only.release.conf.json`). Result: `AudioRouter_0.0.26_x64-setup.exe`, 21.67 MiB, unsigned; its generated NSIS script installs and uninstalls both `audiorouter-plugin-worker.exe` and `audiorouter-vst3-worker.exe`. The shell is newer than `ui/dist` and embeds "Start from defaults". The installer was not run: that would rewrite the owner's per-user install entry.
- Built shell, fresh install: `fresh_install_shell` with `AUDIOROUTER_SHELL_EXE` set to the release shell passes.
- Built shell, isolated (disposable database, `LOCALAPPDATA` and WebView profile, loopback debug port, no other shell running), driven over CDP: "Scan standard folders" found 18 supported plugins; added UA 1176 (VST3, 8 sliders) and Pitchproof x64 (VST2, 2 sliders); the Plugin recovery section is present for both. The backend log shows only two `invalidRoot` scans, for standard folders that do not exist on this PC.
- Real packaged workers through `ControlPlane::dispatch` (`installed_effect_describes_processes_and_restores_state`, mono and stereo, 200 blocks, state save and restore into a new worker): UA 1176, UA Pure Plate (VST3) and ReaFIR (VST2) pass. ReaEQ and ReaJS return their input unchanged at default settings, so that test's "must change audio" check does not apply to them; they pass the offline sweep.
- Not run: pressing Play on a route with a plugin through real audio endpoints (it would touch the owner's devices while Discord is in use); the installer on a second PC or VM.

### Hosted E2E stack-overflow repair (2026-11-12, PR #139)

Symptom: the hosted Edge suite repeatedly left "Loading your saved session";
the real fixture exited with 0xC00000FD and `thread 'main' has overflowed its
stack`. Runs 43147902058 and 43157972751 failed four/one tests respectively,
with further flaky retries caused by the same crash. The Quality gate was only
downstream of the browser failure.

Reproduction and evidence: the failing test files (58 scenarios), the full
CI-config Playwright suite (727 passed), and 100 startup-burst fixture runs
passed on Windows 11, including an 8.3 TEMP alias. A focused CI job then ran
the fixture under `cdb` on the hosted runner. On its second iteration,
`sessions.active.get` completed and `presets.list` raised a first-chance
0xC00000FD. The stack was `dispatch` → `describe` → `method_output_schema`;
there were two consecutive schema frames, not an unbounded recursion. Job
131001582828 contains the captured stack. The trace's initial RPC burst agrees
with that method sequence.

Cause: the comprehensive output-schema match has a 0x64B40-byte (402.8 KiB)
debug stack frame. The same prologue exists in the base revision, so this is a
latent stack-budget defect exposed by the hosted runner, not unbounded schema
recursion. `plugins.inventory` calls `method_output_schema("plugins.list")` in
its `json!` result; the outer match's frame stays live while a second equally
large schema frame is built. An ordinary local run has enough remaining stack,
but the hosted runner does not.

The first wrapper-only attempt (`eac2409c`, job 131007030416) proved this
distinction: it reduced the wrapper to 0x28 bytes but still overflowed with
`impl → wrapper → impl → wrapper` on the debugger stack. Moving the schema
implementation alone therefore did not fix the nested allocation.

Fix: `method_output_schema` remains a small wrapper around a separate
`#[inline(never)]` implementation, and the shared plugin scan/list/retry schema
is a separate 0x2D10-byte (11.3 KiB) helper used by both the direct methods and
`plugins.inventory`. The outer match's frame is now 0x61E80 bytes (391.6 KiB);
the nested list schema no longer allocates another 400 KiB. Generated JSON is
unchanged. Disassembly of the debug fixture on Windows 11 confirms these frame
sizes. The existing dispatcher tests exercise `presets.list` and `describe`,
including schema contents; the focused CI job exercises the real main-thread
reproduction. Temporary WER, method logging and debugger-job instrumentation
must be removed after qualification; they are not product changes.

Validation and rollback: rerun focused hosted reproduction, then the complete
Edge suite, control tests, fmt/Clippy, contracts and docs. Revert only the
wrapper if needed; schemas and plugin behavior are unchanged.
