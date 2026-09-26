# Active plan — VB-Cable-first completion

Status: active, rebaselined 2026-09-17.

## Objective

Complete the non-driver AudioRouter scope using existing VB-Cable,
Voicemeeter, physical WASAPI, and other installed virtual endpoints. The
AudioRouter-owned kernel driver, PortCls activation, production signing, and
clean-machine driver qualification are deferred; they remain documented
requirements but are not active completion gates for this plan.

## Current implementation task — one session with independent paths (2026-09-26)

- **Objective:** Run the user's whole setup as ONE saved session, "Patrick
  Main Session" (user decision 2026-09-26: "it has to be ONE session",
  latency accepted). Voice path: PD200X mic → ReaFIR → ReaEQ → ReaComp →
  ReaGate → CABLE-A Input + Scarlett (monitor). Game path: CABLE-B Output →
  Advanced EQ → Scarlett. Then (second task, same request): show per-tool
  timing along the signal path so the user can see which tool slows the
  sound.
- **Requirements:** GRAPH-15 (independent source paths), GRAPH-14
  (protected voice path), CAP-02 (per-node exact endpoint binding), CAP-13
  (concurrent path ownership), API-04/08, UI-04/05.
- **Prerequisites:** The native multi-input worker already owns N exact
  captures → one compiled graph → M output rings with one start/stop and
  privacy latch. Only the graph shape was limited to one Mixer.
- **Decisions:**
  - **P1 — Paths are weakly connected components** of the enabled graph.
    Each path is a direct source or exactly one Mixer/Input Switch of
    sources, then a linear chain, then 1..8 outputs. Anything else is
    rejected with the path named. A lone edgeless node is ignored.
  - **P2 — One worker, per-path clocks.** The multi-input worker runs every
    path. `RealtimeMixerFanout` holds several paths. Each path processes
    when its own inputs are ready, so one device's clock never paces
    another path. Start, stop, generation, privacy mute and failure stay
    session-wide.
  - **P3 — Exact device per node.** PhysicalInput/PhysicalOutput nodes
    persist `endpointId`. The UI's per-session hint remains the fallback
    for single-route sessions.
  - **P4 — Backend-derived bindings.** New `nativePaths.prepare
    {sessionId, generation?}` (DeviceAdministration) resolves each source
    and output from its node, prepares the worker, and attaches outputs.
    Start, pump, stop and detach reuse the multi-input methods.
  - **P5 — Mono capture into stereo nodes** is adapted in the compiled copy
    (column-summed edge matrices = `[1,1]` duplication), without changing
    the saved graph or the realtime path.
  - **P6 — The same render endpoint may appear on several paths** (voice
    monitor and game both to the Scarlett) as separate shared-mode clients.
- **Ordered tasks:** (1) engine path partition + direct-source path compile
  + multi-path realtime adapter; (2) domain `endpointId`; (3) control
  `nativePaths.prepare`, republish, contracts, API docs; (4) UI per-node
  device selectors and Play via `nativePaths.prepare`; (5) build the
  session in the user's running shell and qualify it attended; (6) per-tool
  timing.
- **Validation matrix:** engine tests (partition, direct path, two paths
  with independent readiness, no crossfeed, the existing Mixer unchanged);
  domain parameter test; control dispatch/schema tests; contract drift; UI
  tests; Windows attended run with PD200X, CABLE-A/B and Scarlett.
- **Risks:** Path clocks drift independently (bounded rings drop like the
  existing Mixer). Recorder tap timeline is per worker, not per path.
- **Rollback:** Revert the engine/control/UI changes. Saved `endpointId`
  parameters would then fail validation; remove them from the session
  before downgrading.

**Outcome (2026-09-26), tasks 1–4 and 6 implemented; task 5 partly done.**
- Engine: `independent_path_sessions` and
  `compile_native_paths_with_plugins_and_audio` build a `CompiledPathSet`. The
  Mixer compiler is split into `compile_mixer_entry` / `compile_direct_entry`
  plus a shared chain/output walk. A direct path whose single connection enters
  a processor uses that connection as its input matrix (mono mic → mono
  plugins works without adaptation). `RealtimeMixerFanout` now holds
  `RealtimePath`s with per-path input ranges and output positions. Each path
  processes when its own inputs are ready. The caller-block methods remain
  single-path only. New `GraphCompileError::UnsupportedPath` names the path.
- Domain: `endpointId` is valid on PhysicalInput/PhysicalOutput (CAP-02).
  `API_METHODS` has 98 entries.
- Control: `nativePaths.prepare` (DeviceAdministration) reads node bindings,
  prepares the worker through `prepare_native_path_worker`, then outputs with
  shared endpoints allowed. It detaches the worker if an output fails.
  `native_paths_session` applies mono adaptation (P5). Live republish uses
  `replace_path_set`.
- UI: device pickers save `endpointId` on the node (plus the old per-session
  hint). Play uses `nativePaths.prepare` when `needsNativePaths` (≥2 paths,
  or ≥2 device inputs/outputs on one path). It names unbound device nodes.
- Signal timing (task 6): per-stage processing counters in `RuntimeGraph`
  (timed start-to-start, so `continue` stages count). `StageTiming` exposes
  them. `RealtimePluginProcessor::latency_samples` reports the plugin bridge's
  in-flight quanta × 128 frames. The multi-input feeder smooths capture
  packet age from WASAPI QPC timestamps (`Win32_System_Performance` feature
  of the existing `windows` crate; no new crate, `Cargo.lock` unchanged).
  The output fan-out smooths queued frames (ring + partial block +
  `GetCurrentPadding`). Control adds an optional `timing` object to
  multi-input `nodeTelemetry`. The new **Timing** tab (`SignalTiming.tsx`)
  lists each output's steps in travel order and marks the slowest. Limits:
  timing is only for the multi-input worker (Mixer or multi-path routes),
  not single endpoint routes. A plugin's own reported latency (VST
  initialDelay) is not yet included; only its pipeline queue is.
- Also fixed: a stage insertion in `compile_capture_test_signal_mixer`
  after preparation left the timing counters one short. They are now
  resized there, and reads are bounds-checked.
- Checks (Windows 11): engine 137, domain 69, control 190 (4 ignored),
  windows-audio 93, plugin-host 71 + 13, UI 343 (26 files), UI typecheck,
  contract drift (98 methods), `cargo check --workspace --tests`, and shell
  `cargo check`. `audiorouter-cli` 38/39: its
  `list_commands_use_discovery_and_do_not_fake_devices` expects 7 processors,
  but the catalog has 16 since the earlier advanced-tools commit. It fails
  without changes in `crates/cli`, so it is not fixed here.
- Visual: `evidence/signal-timing-themes.png` (Edge, dark/light/high
  contrast, sample data). The light-theme accents were darkened after review.
- Session (task 5): the user's normal database was backed up to
  `%LOCALAPPDATA%\AudioRouter\state-backup-20260926-before-patrick-main.sqlite`.
  ReaPlugs were scanned into it (`C:\Program Files\VSTPlugins`, 9 entries
  remembered). `Patrick Main Session` (`patrick-main-session`) was imported
  with 10 nodes and 8 connections, all devices bound by exact ID and the EQ
  flat. A CLI dry run of `nativePaths.prepare` against it returned 2 paths:
  sources `mic` and `siege-in`, outputs CABLE-A, Scarlett and Scarlett, with
  all four ReaPlugs loaded. No audio was started.
- Build: the release shell and plugin worker were built to
  `target/patrick-main-release/release/` (UI dist 11:46:04 < shell 11:46:26).
  The default `src-tauri/target/release` shell is locked by running PID 60776
  (agent-launched on the test DB). Stopping it was refused by the agent's
  permission classifier, so the user must close it.
- **Attended defect 1 (2026-09-26, user-launched PID 59604):** Play prepared
  all paths (four plugin workers alive) but `session.start` refused with
  "enabled plugin nodes require an attached native endpoint session". Cause:
  the start guard counted endpoint, duplex and render-source workers as able
  to run plugins, but not the multi-input worker, which binds its plugin
  stages at preparation. Fix: `native_graph_attached` includes the
  multi-input worker.
- **Attended defect 2 (found by the new live test):** after starting, all four
  ReaPlugs were `failed` with "VST2 output channels 2 do not match graph
  channels 1". UI-created plugin nodes are mono and ReaPlugs are stereo. A
  failed plugin outputs silence, so the voice path would have been silent.
  Fix: the plugin worker maps mono↔stereo. A mono graph feeds every plugin
  input and averages a stereo output. A stereo graph feeds a mono plugin
  (L+R)/2 and duplicates its output (`channel_layouts_are_mappable`,
  `spread_graph_to_plugin_inputs`, `fold_plugin_outputs_to_graph`, 2 unit
  tests).
- **Attended defect 3 (UI):** the refusal showed as neutral text because tone
  was guessed from wording. `formatUiError` now marks caught-error text as an
  error (`markErrorMessage`). Messages have four tones: blue info, green
  success, orange warning, red error, in both the global bar and panel
  messages, for all three themes. Evidence:
  `evidence/message-tones-themes.png`.
- **Live verification (agent, Windows, user's devices, privacy mute on):** new
  ignored test `live_native_paths_start_pump_and_report_signal_timing` on a
  copy of the user's database, with the fixed worker from
  `target/release`. Both paths prepared and started (`runtime: native`),
  delivering 4135 branch blocks in 4 s. All four plugins were `running`
  with 0 failures. Measured timing: mic wait 15.2 ms; ReaFIR 13.3, ReaEQ
  18.7, ReaComp 13.3, ReaGate 10.7 ms (plugin worker queues); CABLE-A queue
  26.7, monitor 26.4; CABLE-B wait 15.7, Advanced EQ 0, Scarlett 27.4 ms.
  So voice ≈ 97 ms and game ≈ 43 ms. The audible result is not yet
  confirmed by the user.
- Checks after the fixes: control 190 (+1 live ignored), plugin-host
  71 + 2 + 13, engine 137, windows-audio 93, UI 344.
- Build: `target/patrick-main-release-2/release/` (UI dist 12:00:24 < shell
  12:00:46; worker rebuilt). `patrick-main-release` is locked by the user's
  running PID 59604 and its workers.
- **Next action:** after the user closes PID 59604, launch
  `target/patrick-main-release-2/release/audiorouter-shell.exe` with
  `AUDIOROUTER_DATABASE=%LOCALAPPDATA%\AudioRouter\state.sqlite` and
  `AUDIOROUTER_ALLOW_DEVICE_ADMIN=1`. Select Patrick Main Session, press Play,
  and verify both paths are heard at once with no game audio in CABLE-A. Load
  the ReaPlugs presets, save the plugin states, and read the Timing tab.
  Record the result here.

## Previous implementation task — per-source volume, Mixer reconnect, advanced tools (2026-09-25)

- **Objective:** (1) Let a Mixer route that includes application sources run
  from Play and reconnect a restarted application without stopping the other
  sources. (2) Add native per-source volume: a one-in/one-out **Volume** tool
  (0–200 %), and per-input Mixer volume sliders (0–100 %) on the node face and
  in the inspector. (3) Add native equivalents of the Audio Hijack advanced
  blocks: Declick, Dehum, Denoise (learned noise profile), FIR Filter (impulse
  response), Input Switch (A/B with 0.5 s / 2 s crossfade), Speech Denoise,
  Sync, and Time Shift (pause / jump back / jump forward / live).
- **Requirements:** GRAPH-02/04/05/08/13/15; DSP-01/04/05/07/08; CAP-06/11/13;
  UI-01/02/04/05/09; API parameter validation (10-api). New DSP IDs are added to
  `docs/spec/07-processing.md` as each tool lands (DSP-10 onward).
- **Reference:** Rogue Amoeba Audio Hijack manual, "Advanced blocks" and the
  Mixer block (fetched 2026-09-25). Its behavior is a UX reference only; no code
  or assets are used.
- **Prerequisites and constraints:** No new crates. `microfft` 0.6 (already
  locked through `pitch_shift`, `size-1024`) provides the FFT for STFT tools and
  uniformly partitioned convolution. All realtime state is preallocated at
  compile time, with no allocation, locks or I/O in the callback. Parameter
  changes ramp (default 10 ms). Cloud inference is not permitted (AGENTS.md).
  Live audio acceptance needs attended Windows evidence; portable tests are not
  Windows evidence.
- **Decisions:**
  - **D1 — Sync is the existing Delay (DSP-05, 0–1000 ms).** It is presented in
    the tool library as "Sync (delay)" instead of a duplicate node kind.
  - **D2 — Volume is a new `volume` node kind.** `percent` runs 0–200 on a
    linear scale, click-free ramp. It is kept separate from `gain` (dB) because
    the user asked for percentage-based per-source volume.
  - **D3 — Mixer per-input volume.** Stored as Mixer parameters
    `inputVolume:<upstreamNodeId>` (0–100 %, default 100) and applied as a
    ramped per-input gain inside `MixerStage`. The default is unity when
    absent; stale keys are ignored by compile and removed by the UI when an
    input is disconnected.
  - **D4 — Per-input chains.** The Mixer compilers accept a linear processor
    chain between each source and the Mixer (Volume, Gain, EQ, Sync, …),
    compiled per input. This makes Volume usable in multi-source routes.
  - **D5 — Speech Denoise is local.** It is a speech-band-weighted spectral
    Wiener filter with minimum-statistics noise tracking, not a machine
    learning model. The UI and spec say so. Adopting a local ML model (for
    example RNNoise) would need a separate dependency and licence decision.
  - **D6 — Stateful tools.** Denoise "learn noise", Time Shift transport and
    Input Switch selection are control RPCs, like the Audio File transport.
    They reach the callback through lock-free atomics.
  - **D7 — Bounds.** FIR impulse response ≤ 2 s (stored as audio media and
    decoded off the callback). Time Shift buffer ≤ 120 s, preallocated. Dehum
    fundamental is 50 or 60 Hz ±5 Hz, with up to 8 harmonics.
- **Ordered tasks:**
  1. Volume node end to end (domain, engine and DSP, discovery, contracts, UI
     library, inspector slider, node-face slider).
  2. Mixer per-input volume: `MixerStage` ramped per-input gains, domain
     validation, node-face and inspector sliders.
  3. Per-input chains in the Mixer compilers (D4).
  4. Play prepares `nativeMultiInputs.prepare` for mixed routes. Add a
     multi-input application-capture runtime that replaces only the exited
     source's capture and keeps other inputs live; the source is silent while
     its application is closed.
  5. Sync presentation (D1); Dehum; Declick.
  6. Input Switch (two inputs, crossfade).
  7. Denoise with learned profile; Speech Denoise (D5).
  8. FIR Filter with impulse-response media.
  9. Time Shift with transport RPCs.
  10. Spec and docs updates, then attended acceptance.
- **Validation matrix:** DSP unit tests per tool: unity and bounds, ramp
  continuity, NaN/denormal safety, and a known-signal response (hum notch
  depth, click repair, noise reduction on synthetic noise, FIR impulse
  identity, delay alignment, Time Shift jump positions). Engine compile tests
  for per-input chains and pruning. Domain validation tests for every new
  parameter. Control tests for the multi-input reconnect state machine using
  portable resolver fixtures. UI tests for the library entries, sliders and
  Play routing. Attended: Discord + microphone + Test Signal through Volume
  into the Mixer with a Discord restart.
- **Evidence:** this plan; `docs/plans/active/evidence/M05-visual-editor.md`
  for UI; `M02-audio-engine.md` for live routes.
- **Risks:** Per-input chains increase callback work; a per-input processing
  budget is enforced through the existing realtime cost classes. STFT tools add
  latency (≈21 ms at 1024/48 kHz), which must be disclosed as
  `latency_samples`. A long FIR costs CPU, which the 2 s bound limits. Replacing
  a single multi-input capture must not glitch the other inputs.
- **Rollback:** Each task is independently revertible. New node kinds are
  additive; removing a kind requires migrating saved sessions that use it, so
  the kinds ship only after their tests pass.
- **Next action:** Task 1 (Volume node).

## Previous implementation task — media source and Test Signal usability (2026-09-22)

- **Objective:** Add a graph-native WAV/MP3 playback source with play/pause/stop,
  looping, and an explicit temporary voice-recording workflow for calibration;
  keep the canvas node compact and put descriptive controls in the inspector.
  Also make Test Signal fit the canvas and make endpoint readiness actionable
  without hiding the existing authorization boundary.
- **Requirements:** UI-01/02/04/05/09/11/12/13/15; GRAPH-01/05/08/09/13;
  REC-04/07/08/10/12; SEC-03/04/06; NFR-08.
- **Prerequisites:** The graph now includes a bounded Audio File source and
  WAV/MP3 decoder. Test Signal and file playback require an exact prepared
  endpoint; the current shell's preparation was denied for missing
  `DeviceAdministration`. Existing recording APIs do not expose a temporary
  recorded take as graph source media.
- **Decisions:** Keep decoding, capture, and playback in backend/native audio
  paths; do not use Web Audio or renderer microphone capture. Bound media size,
  duration, channels, decoder memory, and realtime callback work. Decode WAV and
  MP3 off the callback. Temporary voice capture must be deliberate, visible,
  bounded, permission-gated, and not silently persisted in the library. The
  user explicitly authorized local-shell `Record` on 2026-09-22; do not add
  `Capture` or `DeviceAdministration`. Keep only playback/stop status on the source
  card; put file selection, pause/resume, loop, and temporary record/retake in
  its inspector.
- **Ordered tasks:** (1) Map registry, graph, decoder, media API, and
  permissions. (2) Implement opaque persisted media upload, bounded WAV/MP3
  decode off callback, and an allocation-free per-node source stage. (3) Add a
  compact source card with Play/Stop and put Pause/Resume, import, and loop
  details in the inspector. (4) Fix Test Signal width and keep endpoint readiness
  actionable. (5) Add graph/media/UI regressions; run acceptance and record
  live native limitations. (6) Implement temporary voice capture only through
  the existing permissioned backend recorder path using that authorized local
  shell `Record` scope, without adding `Capture` or `DeviceAdministration`.
- **Validation matrix:** Domain/engine tests for the node and bounded playback;
  WAV and MP3 fixture decoding; media persistence/deletion and upload bounds;
  temporary capture lifecycle, expiry/deletion, permission and no-persistence
  tests; UI component tests; M05, domain, engine, control, docs and traceability
  acceptance. Attended playback and animation require an authorized prepared
  endpoint and remain separate evidence.
- **Evidence:** `docs/plans/active/evidence/M05-visual-editor.md`.
- **Risks:** Malformed or large media can exhaust memory or block audio if
  decoded in the callback. Temporary voice capture is sensitive and needs
  visible consent and bounded retention. Source transport must not stop other
  session routes. Test Signal cannot start a route while endpoint preparation
  is denied.
- **Rollback:** Remove only the new source, temporary capture/playback
  operations, UI controls, and this task's contract/evidence additions. Preserve
  the existing graph engine, session lifecycle, and signal-flow visualization.
- **Outcome (2026-09-22):** Added the `audioFile` graph node, bounded opaque
  media storage and ordered upload API, WAV/MP3 Symphonia decoding/resampling
  off callback, immutable allocation-free source playback, per-node transport,
  and compact React Flow Play/Stop controls. Pause/Resume, file selection, and
  looping are in the inspector. Test Signal remains 184px wide. Runtime source control
  starts the graph session when necessary and still requires exact endpoint
  preparation under existing authorization. Temporary voice recording is not
  complete. The user explicitly authorized local-shell `Record` for temporary
  takes; endpoint capture and device administration remain separate. The UI
  currently supports import and playback; it does not yet claim an in-app
  microphone record action.
- **Validation (2026-09-22):** `cargo test --workspace --locked --quiet`
  passed on Windows; M05 UI acceptance passed typecheck, 20 files/292 tests,
  and temporary production build; control passed 181 plus 4 guarded-live
  ignored; domain 66; engine 123; storage 93; Windows-audio 88; all remaining
  workspace suites and doc-tests passed. The audio upload, engine graph source,
  and storage persistence regressions passed. Contract drift passed (91
  methods, 21 node kinds, 7 processors, 20 event categories); docs validation
  passed (58 Markdown files/267 links); traceability passed (160 IDs).
  Package Clippy passed for domain/engine/storage. Workspace Clippy remains
  blocked by the existing `too_many_arguments` lint at
  `crates/control/src/lib.rs:284` (`finalized_mp3_recording`). `cargo fmt --all
  -- --check` reports formatting diffs across the current working tree and was
  not applied to avoid reformatting unrelated user changes. `git diff --check`
  and final source review remain the immediate next checks. `rtk` is
  unavailable. After the workspace run, the added synthetic MP3 fixture
  regression passed independently (`cargo test -p audiorouter-engine --locked
  decodes_synthetic_mp3_fixture_to_finite_samples`).
- **Next action:** implement and verify the permissioned temporary-take path
  using an already prepared input, then test signal-flow animation in the
  attended shell. The user authorized `Record`, not `Capture` or
  `DeviceAdministration`; if endpoint preparation is unavailable, keep live
  playback blocked and record the exact authorization result.

### 2026-09-22 follow-up: local Record grant and animation UI check

The user explicitly authorized `Record` for the local desktop shell. Added
`PermissionScope::Record` to `ClientGrant::for_desktop_shell()` and the
explicit device-administration opt-in grant. The desktop shell grant remains
without `Capture`, `PluginScan`, and `DeviceAdministration`; this only permits
explicit recorder methods under their already approved roots. Updated the
security/privacy guidance and the validated lesson accordingly.

Verification: the focused control grant test passed; UI typecheck passed; M05
UI acceptance passed (20 files, 292 tests, temporary production build); the
focused session canvas test passed (23 tests); contract drift passed (91
methods, 21 node kinds, 7 processors, 20 event categories); docs validation
passed (58 Markdown files/267 links); M08 traceability passed (160 IDs); and
`git diff --check` passed. Shell `cargo check --locked` could not reach
crates.io (`serde` index download denied by the network); `--offline --locked`
also could not proceed because this separate shell lockfile requires a lock
update. The already-tested control crate compiled during its focused test.

Animation verification is UI-only. All 23 canvas tests passed with fresh meter
telemetry mapping to the edge state and width. `cua.getState()` reported no
apps or browsers, so no attended animation was observed and no sound was run.
The earlier exact endpoint preparation denial remains
`permissionDenied: DeviceAdministration`; the `Record` authorization does not
change it. At that point the temporary-take-to-audio-source backend flow was
still missing; it is completed in the subsequent execution note below.

Next at that point: implement bounded temporary WAV recording from an already
prepared graph input and verify cleanup; this was completed in the subsequent
execution note below.

### 2026-09-22 completed temporary take workflow and signal-flow preview

Implemented `audioMedia.importTemporaryRecording` under the desktop shell's
authorized `Record` grant. It accepts only a completed WAV recording with the
`audio-file-take-` recorder identity, verifies file bounds and WAV metadata,
decodes off the callback, stores expiring media for 24 hours, and removes the
temporary file plus recording-library entry. Expired media is hidden and
pruned. The Audio File inspector can record for at most 120 seconds through an
already-routed, enabled Recorder node on the running session, stop/import the
take, then use the resulting media after graph plan/commit. It does not open a
microphone or grant `Capture`/`DeviceAdministration`.

Validation on Windows 11: `cargo test --workspace --locked --quiet` passed,
including the temporary-import permission/cleanup regression and expired
media test; `npm.cmd run typecheck` passed; the full UI suite passed 20 files /
294 tests; `tests/acceptance/m05-ui.ps1` passed the same UI suite and a temporary
production build; contract drift passed (92 methods, 21 node kinds, 7
processors, 20 event categories); docs validation passed (58 Markdown files /
267 links); M08 traceability passed (160 normative IDs). A local browser
preview now presents a compact microphone â†’ gain â†’ headphones graph without
edge controls obscuring the signal lines. Its telemetry is explicitly
simulated; it is visual-only evidence and does not qualify live audio.

After formatting the touched Rust files, the complete locked workspace suite
passed again; `cargo fmt --all -- --check` and `git diff --check` also passed.

Live 10-second audio remains unverified: the computer-use surface exposes no
attached app, and the recorded endpoint-preparation attempt returned
`permissionDenied: DeviceAdministration`. No endpoint was prepared and no
sound was started. Keep this as the next attended task; do not report the
simulated preview as a live animation test. `rtk` remains unavailable.

## Current implementation task — Test Signal playback controls (2026-09-22)

### Follow-up: hover jump and stationary active edges (2026-09-22)

Reported symptoms: Test Signal appears to bounce on hover and active route
edges do not move visibly. Plan: remove hover elevation and limit hover
feedback to a static border cue; keep the volume-responsive thick stroke as a
solid base and render a separate narrow, moving dash highlight only for edges
whose fresh backend meter evidence says active. The prior dash gaps could be
lost as stroke width grew. Exercise this state in the simulated harness and
focused regression. Preserve reduced-motion behavior and keep
silent/stale/muted/faulted paths static. Requirements: UI-04/11/15,
NFR-08. Validate with focused canvas tests, M05 UI acceptance, docs/traceability,
formatting, and diff checks. The harness remains simulated evidence; actual
meter flow still requires an authorized live session.

Outcome: removed the large hover shadow lift that made the node look as if it
jumped. Active audio edges now render the responsive wide amber stroke as a
solid body with a separate narrow, pale moving dash overlay, so motion remains
visible at maximum stroke width. Only fresh active meter evidence creates the
moving overlay; stopped, silent, stale, unavailable, muted, disabled, and
faulted paths remain static. The browser harness is marked running and uses
fake meters to make this active presentation visible for inspection.

Verification: focused canvas and Test Signal suites passed (29 tests),
typecheck passed, and M05 acceptance passed all 20 UI files / 294 tests plus a
temporary production build. The Chromium visual preview was inspected and
shows the thicker amber body with the moving highlight; readings are simulated.
No live sound was started or independently observed; the computer-use surface
still reports no apps/browsers. Reduced-motion users retain the static-flow
presentation.

### Follow-up issues reported 2026-09-22

Objective: make Test Signal Play actionable on a committed/routed/enabled node
even when endpoint diagnostics are incomplete (surface backend readiness errors
after click), and ensure Audio File placeholder/status text remains readable on
the dark node surface in every theme. Requirements: UI-01/04/05/09/11/13 and
UI-15. Steps: (1) retain disabled state for uncommitted, unrouted, disabled, or
busy playback; remove only the endpoint-prepared disabled gate; (2) show the
exact endpoint readiness state and route start failure in the workspace status;
(3) use explicit high-contrast node text colors and regressions for light theme;
(4) rerun focused UI and M05 acceptance, then attempt live playback only if an
exact endpoint is already prepared under the existing grant. This does not
grant endpoint administration or create simulated live meter events.

Outcome: Play is enabled for a committed, routed, enabled Test Signal even
when endpoint diagnostics do not show preparation. It uses the ordinary
`session.start` action so a missing endpoint produces the backend error in the
workspace status. Uncommitted, unrouted, disabled, busy, and already-running
states remain guarded. Audio File source/status text now has explicit light
foreground colors on its fixed dark node surface, including under the light
application theme.

Verification: focused Test Signal/canvas checks passed (29); typecheck passed;
M05 acceptance passed all 20 UI test files / 294 tests and temporary production
build; contract drift passed (92 methods, 21 node kinds, 7 processors, 20 event
categories); docs passed (58 Markdown files / 267 links); M08 traceability
passed (160 IDs); Rust formatting and `git diff --check` passed. An
`audiorouter-shell` process is running against the Vite dev URL, but the
computer-use surface exposes no apps/browsers and Windows denied reading the
process command line. No click or live audio could be observed through the
tooling; backend endpoint authorization remains enforced.

- **Objective:** Put accessible Play and Stop controls on Test Signal canvas
  nodes so the user can start and stop the configured signal route in place.
- **Requirements:** UI-01, UI-04, UI-09, UI-11, UI-13; API-03/API-09 lifecycle
  reporting; SEC-03/04/06 scope enforcement and capture privacy.
- **Prerequisites:** M05 Test Signal and session lifecycle APIs exist. Real
  endpoint playback still requires the exact native endpoint worker to be
  prepared under the existing `deviceAdministration` scope.
- **Decision:** Reuse `session.start`/`session.stop` and their idempotency,
  status, and permission behavior; add no alternate audio path or permission
  bypass. The node controls start/stop the whole session, so label that clearly
  and keep the start action available only for an enabled Test Signal with a
  connected enabled physical output and a prepared exact native endpoint for
  that session. A graph with other sources will start those sources too;
  document this on the control.
- **Ordered tasks:** (1) Add the controls and lifecycle/busy state to the
  Test Signal canvas card and wire them to the existing App actions. (2) Add
  focused interaction tests for routed/unrouted, running/stopped, and busy
  states; the existing App lifecycle action owns backend error messaging. (3)
  Update UI/API interaction contracts and M05 evidence. (4) Run focused and
  M05 validation, inspect the diff, and record attended playback as open unless
  a real authorized endpoint run is available.
- **Validation matrix:** UI component tests cover accessible labels, callback
  selection, and disabled states; M05 acceptance covers typecheck, complete UI
  suite, and production build; docs and requirement traceability cover the
  synchronized contract. Windows live playback requires an authorized prepared
  endpoint and is reported separately.
- **Evidence:** `docs/plans/active/evidence/M05-visual-editor.md`.
- **Risks:** Session lifecycle affects every connected source and stopping it
  stops the entire route. A button must never claim sound is playing if endpoint
  preparation or session start fails. Device authorization must remain enforced
  by the backend.
- **Rollback:** Remove only the Test Signal card controls, their App wiring,
  focused regressions, and this task's contract wording; keep the existing
  session lifecycle and signal-flow visualization intact.
- **Outcome (2026-09-22):** The Test Signal card now has accessible Play and
  Stop buttons wired to the existing App lifecycle actions. Play is disabled
  for uncommitted, unrouted, disabled, busy, or already-running states. Once
  committed/routed/enabled, Play invokes the backend lifecycle even when
  endpoint diagnostics do not report a prepared worker; the backend's exact
  start error is shown in workspace status. Actual sound still requires the
  exact prepared endpoint.
- **Validation:** `npm.cmd run typecheck` passed; focused canvas/control tests
  passed (28 tests); `tests/acceptance/m05-ui.ps1` passed typecheck, 20 UI test
  files/291 tests, and a temporary production build; docs validation passed
  (58 Markdown files/267 links); traceability passed (160 IDs); and
  `git diff --check` passed. `rtk` is unavailable in this environment.
- **Unresolved blocker:** The current attended shell's exact endpoint
  preparation was rejected with `permissionDenied: DeviceAdministration`, so
  no live audio was played. The controls preserve that backend requirement;
  live button playback and meter animation remain to be confirmed after the
  current-user shell has the required authorization and the endpoint is
  prepared.
- **Next action:** The user can inspect the controls in the running dev shell;
  perform the attended Play/Stop and animation check once exact endpoint
  preparation is authorized.

### Follow-up: graph vanished after Play report (2026-09-22)

The user reports that clicking Play made all canvas nodes disappear. Read-only
inspection found the shell process still responding, the Vite page returning
HTTP 200, and no matching Windows Application Error/Windows Error Reporting
event in the recent window. The attended-app surface exposes no windows. The
running shell has no persisted diagnostic log, and its process command line is
not readable here, so the session database and historical RPC response cannot
be identified from available evidence. Do not claim that the prior graph was
deleted or recovered.

Add bounded shell JSONL diagnostics for `session.start`, `session.stop`,
`sessions.get`, and `sessions.list`, under
`%LOCALAPPDATA%\\AudioRouter\\logs\\shell.jsonl`; record method, session
identity, outcome, revision, and node/edge counts without graph contents,
audio data, or request payloads. Keep one 5 MiB current file and one rotated
previous file. Add a focused test that graph counts are logged but node
content is not. The current attended process cannot use the new logger until
a rebuilt shell is launched. Then correlate Play, session reads, and transport
failures before changing session or route state.

Requirements: UI-04/09/11, API-08, STATE-01/02, ENG-04. The focused logger
regression passed, followed by all 30 shell tests. The shell-specific lockfile
was updated with the WAV/MP3 dependency set; crate access succeeded. A regular
in-place build was refused by Windows because the running shell owns the exe.
An isolated `cargo build --manifest-path src-tauri/Cargo.toml --locked
--target-dir %TEMP%\\audiorouter-diagnostic-build` succeeded and produced
`%TEMP%\\audiorouter-diagnostic-build\\debug\\audiorouter-shell.exe`. The
current live shell cannot load these changes until it is relaunched. Preserve
the live app and current session; no restart or graph mutation was performed.

## Requirement coverage

The stable specifications and milestone contracts remain authoritative. This
plan covers the non-driver portions of PROD, ARCH, GRAPH, CAP, DSP, REC, PLUG,
UI, API, AUTO, STATE, SEC, NFR, QUAL, and ENG requirements mapped in
[M08 delivery traceability](../../spec/15-delivery.md#requirement-traceability).
Managed virtual-device requirements and SEC-08 are deferred in
[the driver/signing plan](../future/M03-driver-signing.md).

## Current implementation task â€” workspace, MCP observability, and end-to-end qualification (2026-09-23)

- **Objective:** Replace the crowded session-left-bar/workspace layout with a
  wider canvas and a clear right-side tabbed workbench for Add tools, selected
  node properties, session actions, and MCP setup/activity. Redesign compact
  status/full workspace navigation with task-focused tabs and plain-language
  guidance. Add privacy-conscious client/backend/MCP diagnostic streams, use
  diagnostics to identify/fix regressions, establish browser-driven E2E tests
  for source, modifier, and destination tools across simple and complex graphs,
  and inspect rendered dark/light/high-contrast UI screenshots for consistency.
- **Requirements:** UI-01â€“15; AUTO-06â€“12; API-08/10; STATE-01/02; SEC-03â€“06/10;
  NFR-06/08/14/15 UI portions; ENG-04/05. User direction supersedes the
  existing session-sidebar layout description in UI-09 for the new workspace.
- **Prerequisites:** Preserve all current working-tree edits, including the
  media source, Test Signal, flow visualization, and diagnostics logger. The
  active attended shell is still the old process. Its current session database
  could not be identified; do not restart it or alter audio/session state
  without a verified safe path. The newly built diagnostics shell is at
  `%TEMP%\\audiorouter-diagnostic-build\\debug\\audiorouter-shell.exe`.
- **Decisions:** Keep the graph canvas central and move tool discovery to the
  right workbench; use accessible tabs with a strong active indicator and a
  selected-node properties default when relevant. Session name/revision and
  plan/commit/duplicate/delete/discard actions live under Session. MCP has a
  dedicated setup tab with copyable client instructions and a bounded live
  activity stream; do not add an unauthenticated loopback bridge or grant MCP
  broader scopes. Logs are bounded, user-local, redact credentials and audio,
  and distinguish inbound request, backend operation, result, and errors.
  E2E tests use a controlled fake backend for deterministic UI coverage and
  only claim physical loopback when guarded Windows tests actually run.
- **Ordered tasks:** (1) Inspect current app modes, MCP transport/server,
  event bus, graph source/modifier/sink catalog, and available browser-test
  tooling; map UI/AUTO/SEC requirements. (2) Implement bounded structured
  frontend diagnostics and backend/MCP request audit events with a UI-visible
  MCP activity stream. (3) Restructure App workspace into a right-side tabbed
  workbench, remove the left session sidebar, migrate session/revision/edit
  actions to Session, and improve responsive/compact navigation. (4) Refine
  global styles for scrollbars, controls, text fields, forms, contrast,
  spacing, and consistent panel hierarchy; render and inspect screenshots at
  supported sizes and themes. (5) Add Playwright (or repository-compatible
  browser automation) E2E tests for each input/modifier/output family and
  representative complex routes, including plan/commit/errors. (6) Run UI,
  E2E, contract, security, M05/M07/docs/traceability checks; use available
  logs/screenshots to self-diagnose and fix failures. (7) Update stable UI/MCP
  specs and evidence, recording inaccessible live loopback or client setup as
  explicit user inputs rather than inferring success.
- **Validation matrix:** UI tests assert tab semantics, selected state,
  keyboard traversal, responsive layout, no left session bar, session actions,
  and MCP stream/redaction. E2E route matrix builds microphone/application/
  file/test sources through gain/EQ/mixer/recorder/physical or virtual outputs
  using fixtures and plan/commit. MCP acceptance uses a real stdio client and
  explicit local grant where available; denied scopes stay denied. Screenshot
  review covers dark/light/high-contrast at 1280Ã—720 and narrow responsive
  dimensions. Live loopback remains a separately guarded physical acceptance.
- **Evidence:** `docs/plans/active/evidence/M05-visual-editor.md` and a new
  checked-in UI/MCP E2E evidence artifact if the acceptance harness needs one.
- **Risks:** Large App.tsx and inherited CSS make layout regressions likely;
  MCP calls may contain user-authored names/configuration, so logs must be
  visibly bounded and sensitive-value aware. Rebuilding/relaunching the active
  shell could disrupt the user's session and is deferred until its database
  and route state are identified or the user is available. UI simulation is not
  proof of actual endpoint playback.
- **Rollback:** Keep changes partitioned into shell diagnostics, MCP events/UI,
  workspace composition, global style tokens, and E2E harness. Revert only the
  affected slice if it regresses existing graph/session behavior; never reset
  user sessions or audio configuration as rollback.
- **Outcome (2026-09-22):** Implemented and visually reviewed the right-side
  tabbed workbench (Tools, Properties, Session, Setup, Devices, Recording,
  Advanced, MCP, Logs); removed the old left session rail from the workspace
  and moved session/revision/draft lifecycle actions under Session. Setup and
  operational controls now have focused pages. Added exact desktop Codex TOML
  and Claude PowerShell snippets with browser-preview placeholders, client
  authorization guidance, redacted MCP call activity, bounded shell/backend
  RPC logs, frontend error/graph checkpoints, and the browser E2E harness.
  Logs now serialize cross-thread writes and rotations, cap caller-controlled
  labels/fields, retain safe error categories, and omit raw error/runtime text
  and session IDs. Read-only inspection found two malformed records in the
  pre-fix LocalAppData backend log after separate Cargo processes were run
  concurrently. The Logs reader already skips invalid lines. A Windows
  same-user named mutex now serializes writers across processes and recovers
  abandoned ownership; concurrent authenticated transport/shell test binaries
  passed after the change. A post-run log audit found no new malformed lines:
  33 of 35 backend records parse, with two historical malformed rows, all 25
  MCP activity rows parse, and no shell log exists for the legacy shell.
  Screenshot review surfaced and fixed clipped MCP configuration text and
  an initial setup flow that hid snippets outside desktop mode. Reviewing
  the narrow Devices tab then exposed browser-default endpoint controls and
  overlapping labels; shared styling now stacks full-width rounded selectors
  and actions. Rapid tool
  additions now reserve distinct grid positions. `AUTO-13/14` were added to
  the delivery traceability map. A Windows PowerShell parser defect in the
  existing traceability script was fixed so missing IDs are reported.
- **Validation (2026-09-22, Windows 11):** `npm.cmd run typecheck` passed;
  `npm.cmd test` passed 20 files / 294 tests; Playwright E2E passed 5/5
  (all 16 built-in node kinds can be added, multi-source/modifier/output set
  renders and mute edits, one source-to-output connection works, 1280x720
  remains usable, all nine workspace tabs plus MCP config/copy control are
  reachable, status toggling preserves the selected tab, and selecting a node
  switches to Properties. Targeted transport diagnostics test passed 1/1; CLI MCP
  activity redaction test passed 1/1; the MCP stdio integration suite passed
  3/3; Tauri shell tests passed 30/30. Contract drift passed (92 methods,
  21 node kinds, 7 processors, 20 event categories); docs validation passed
  (58 Markdown files / 272 links); M08 traceability passed (162 normative
  IDs); `git diff --check` passed. The full locked workspace suite also
  passed; Tauri shell tests passed 31/31. MCP activity redaction/bounds
  tests passed 3/3 and backend log tests 2/2. The CLI/transport package
  formatting checks and isolated Tauri `main.rs` rustfmt check passed. The
  whole Tauri crate format check still reports pre-existing formatting in
  unchanged startup, backend-supervisor, and OS-transition source; these
  unrelated files were restored untouched. On 2026-09-23, Playwright E2E
  passed 6/6 after the Devices style fix; UI typecheck and all 294 UI
  tests passed, `git diff --check` passed, docs passed (58 Markdown files /
  272 links), and requirement traceability passed (162 IDs). Visual screenshots
  were captured under `%TEMP%\audiorouter-designer-review\` for Tools,
  Session, Properties, Setup, Devices, Recording, Advanced, MCP, Logs, dark,
  light, and high-contrast views; narrow Devices selectors were corrected and
  reviewed at 1280x720. A later designer pass also restyled Session name and
  session selection controls as stacked, rounded fields; the refreshed Session
  screenshot was reviewed. E2E passed 6/6 again after this CSS change. An
  exploratory ten-edge pointer-drag E2E failed
  because rapid auto-placement and viewport transforms overlapped nodes and
  intercepted pointer events. That brittle sequence was removed; retained
  complex-graph coverage verifies composition and mute interaction, not a
  connected/committed chain. The E2E graph harness uses fake telemetry
  and does not start a backend session or prove physical audio routing. The
  browser shell preview has no Tauri bridge; exact desktop paths are therefore
  not screenshot-verified. External Codex/Claude enrollment, attended shell
  interaction, Narrator, and live loopback remain unverified. `rtk` is absent.
- **Next action:** use an attended desktop surface to confirm the live shell's
  database override/session, inspect its current draft and route, enroll a real
  Codex or Claude client, observe an inbound MCP call in the UI, then run the
  guarded playback/loopback route and confirm live edge motion. PID 53576 is
  older than the shell logger; no `shell.jsonl` exists. The default database
  path is known from code but the process environment cannot yet prove whether
  that shell uses it. Do not mutate or restart the session based only on the
  persisted default DB snapshot. CUA currently reports no app or browser.

## Validation update (2026-09-23, Windows 11)

Added a controlled App-level Playwright route harness. It builds all source
families (Test Signal, physical input, Audio File, application capture,
endpoint loopback), then a mixer → gain → EQ → compressor → limiter → mute
chain with meter, recorder, and physical output branches. It verifies an
unconnected graph is rejected, adds 13 reviewed edges through the Setup UI,
then plans and commits revision 8. This is deterministic UI/draft-adapter
evidence only: its planner and commit are fake and it does not start native
audio. The test exposed that the old main-content status message was hidden by
the workbench layout. A sidebar status slot fixed visibility; the duplicate
old status renderer then caused 22 existing text-query failures and was
removed. Final results: typecheck, 294 UI tests, and 7 Playwright E2E tests
passed; contract drift passed (92 methods / 21 node kinds / 7 processors / 20
event categories), documentation passed (58 Markdown files / 273 links), M08
traceability passed (162 IDs), and `git diff --check` passed. The production UI
build passed with output under `%TEMP%\audiorouter-ui-build-review`; the
default `ui/dist` build first hit `EPERM` while clearing a bundle held by the
running shell, which was left untouched. The complex route screenshot was
reviewed at `%TEMP%\audiorouter-designer-review\workspace-complex-route.png`.
The setup panel was scrolled during the 13-edge workflow; graph nodes and links
remain visible, but this is not attended desktop evidence. `rtk` is
unavailable. No app/browser surface exists for safe attended interaction,
real MCP client enrollment, or guarded loopback playback; the legacy shell and
its unidentified session/database remain untouched.

**Next action:** resume at the attended Windows-shell gate: identify the
shell's database/session and route safely, confirm the new diagnostics log is
created, configure Codex or Claude MCP through the visible setup, observe a
real inbound call, then run guarded playback/loopback and confirm animated edge
levels. Do not claim those gates from the simulated Playwright route.

## 2026-09-23 - status workspace and route harness hardening

Read-only recheck found no CUA app/browser surfaces. `Get-Process` still sees
PID 53576 (`audiorouter-shell`, started 2026-09-21). Unprivileged CIM access was
denied; an explicit read-only elevated query found
`src-tauri/target/debug/audiorouter-shell.exe` with no command-line arguments.
Its environment override could not be inspected, so the active process database
path remains unproven. The code's default database at
`%LOCALAPPDATA%\AudioRouter\state.sqlite` was opened read-only; it contains one
saved session at revision 4 with two nodes (physical input and physical
output), one edge, and five history rows. The DB was last modified 2026-09-18,
so its snapshot may not represent the old shell's current in-memory draft.
Local log metadata: `backend.jsonl` has 33 parseable rows and
`mcp-activity.jsonl` 25, both last modified 2026-09-22; `shell.jsonl` is absent.
No DB, route, or audio state was mutated.

The route harness planner now checks connection endpoint existence/direction,
channel matrix dimensions and finite [-2,2] coefficients, duplicate edges,
the one-incoming-edge rule outside mixers, and source-to-destination reach.
This is a stricter fake planner, not a substitute for the Rust/domain backend.
The formerly broad `Show status` layout exposed two dozen ungrouped legacy
panels. It now keeps the canvas and right workbench layout, adds the compact
route summary, and asserts those unrelated main-content panels remain hidden.
Virtual-device lifecycle/routes are grouped under Devices. The designer
screenshot at `%TEMP%\audiorouter-designer-review\workspace-status.png` was
reviewed; status, canvas, selected Session tab, and controls remain aligned.
Typecheck, all 294 UI tests, and all 7 E2E tests passed. The temporary
production build passed at `%TEMP%\audiorouter-ui-build-review-0923b`; the
normal output remains locked by the running shell. Docs validation passed (58
Markdown files / 273 links), contract drift passed (92 methods / 21 node kinds
/ 7 processors / 20 event categories), M08 traceability passed (162 IDs), and
`git diff --check` passed. Scoped MCP stdio integration passed 3/3,
transport diagnostic privacy/concurrency passed 3/3, and MCP activity
redaction/bounds passed 3/3. These use fixtures and do not enroll a real
Codex/Claude client. No current shell log exists; live playback and MCP remain
unverified.

### Follow-up verification (2026-09-23)

Added an Audio File WAV upload UI E2E using canned fake-backend metadata. It
checks filename/status, loop editing, and that play/stop remain disabled until
the graph is committed; it does not decode or play the fixture. The MCP E2E
also scrolls the right sidebar and confirms its final plain-language setup
guide is reachable. A separate simple Test Signal-to-Physical Output draft
connects, plans, and commits; the existing complex route covers five source
families, mixer/modifier processing, and three output branches. Focused cases
passed, then the complete Playwright suite passed 11/11. The diagnostics E2E
feeds a redacted MCP tool event and backend graph-commit error through the
bridge and checks both streams plus the client graph checkpoint. UI typecheck and
unit/component tests passed (20 files / 294 tests); production bundle passed
at `%TEMP%\audiorouter-ui-build-review-0923c`; contract drift passed (92
methods / 21 node kinds / 7 processors / 20 event categories), docs passed
(58 Markdown files / 273 links), M08 traceability passed (162 IDs), and
`git diff --check` passed.

MCP stdio/backend interop passed 3/3; backend diagnostic privacy/concurrency
passed 3/3; MCP activity redaction/bounds passed 3/3. These exercise real
process/pipe protocol fixtures and log writers, not enrollment/use by an
external Codex or Claude process.

The MCP stdio integration now inspects activity emitted by the actual CLI child
process after `tools/call`. It overrides only that child's `LOCALAPPDATA` with
a unique temporary root, checks method/tool identity, safe field names,
outcome/error category, and proves a nested sentinel value is absent from the
JSONL file. This both exercises the real writer through the protocol path and
prevents MCP integration tests from appending to the user's real app log.
`cargo test -p audiorouter-cli --test mcp_stdio -- --test-threads=1` passed
3/3; formatting was corrected after the first `rustfmt --check` identified one
line-wrap difference. The existing shell remains inaccessible; this is local
process integration evidence, not external Codex/Claude enrollment.

Client diagnostics were previously only in memory. They now persist the last
80 short rows to the local WebView app profile, capped at 320 characters per
row. Persisted entries contain only bounded error type/script basename/line,
safe MCP read-failure type, and graph node/edge counts plus revision; no raw
exception text, path, audio, parameter, or node name is stored. Storage errors
leave the in-memory view available. An App regression dispatches a synthetic
error containing a private audio path, verifies that the path is absent from
storage, remounts the app, and confirms both error category and graph checkpoint
remain visible. The focused regression passed; all UI tests passed (21 files /
297 tests), all 11 E2E tests passed, typecheck passed, and production build
passed into `%TEMP%\audiorouter-ui-build-review-0923d`. This makes client
diagnostics survive renderer reloads but does not create evidence from the
already-running pre-logger shell or explain its historical node disappearance.

The dark, light, high-contrast, MCP, and compact-status screenshots were
reviewed. MCP guidance extends below the initial sidebar viewport; the panel
scrolls, and the new E2E confirms the final instructions can be brought into
view. The desktop preview reports backend unavailable by design and cannot
prove live shell appearance, MCP calls, or audio behavior. `cua.getState()`
still provides no app/browser surfaces. PID 53576's active database override
and in-memory session remain unknown, so its process and route were not
restarted or mutated. Existing default-path logs are stale for that process;
new live shell, MCP-client, and playback diagnostics remain attended gates.
Read-only log audit found 35 backend rows (33 parseable, 2 malformed legacy),
with five `graph.commit` errors (three classified `permissionDenied`); MCP
activity has 25 parseable rows with ten errors (six classified `toolError`),
and no shell log exists. Raw runtime messages/arguments are intentionally
omitted from those logs, so they cannot establish a live playback or graph
deletion cause. Historical test/auth denials are not evidence of the current
session's failure mode.

An isolated second shell was evaluated but not launched: the production shell
hard-codes the shared `\\.\pipe\audiorouter-control` default unless an
environment override is added, while repository guidance says attended shell
testing must launch plainly with only `AUDIOROUTER_DATABASE`. The legacy shell
may already own that pipe. Starting a second runtime could therefore collide
with its backend or expose the wrong database, and stopping/restarting the old
process is not safe without its session identity. The existing
`target/debug/audiorouter-cli.exe help` output lists `mcp serve`; the earlier
`mcp --help` probe was simply the wrong CLI shape (`mcp serve` is nested), not
proof of a stale/missing feature.

## Current evidence

- Backend graph ownership, validation, persistence, authorization, routing, and
  bridge lease/generation contracts are covered by the locked Rust suites and
  guarded VB-Cable/physical endpoint runs. Privacy coverage is split: the
  authorized durable control contract, allocation-free realtime mute primitive,
  and live native-worker latch propagation are tested. The control latch now
  propagates into attached native endpoint, process-loopback, multi-input,
  render-source, and duplex workers. The latest guarded user-mode
  dispatch-to-processed-block sample measured 16.734 ms p95; calibrated
  effective-output timing and attended microphone-path measurement remain open.
- Existing VB-Cable routing has live multi-input/many-output evidence,
  application capture, physical output, recorder taps, Voicemeeter tool
  integration, and cleanup without persistent audio configuration changes.
  The 2026-09-17 refresh fixed bounded multi-input packet backpressure and
  requalified the exact CABLE plus Focusrite capture / Voicemeeter In 2 plus
  In 5 render route with 47,520 captured frames, 256 delivered quanta, and
  47,232 rendered frames; the feeder now retains unread packet slices across
  pumps.
  Corrected current guarded application capture passes both include and
  exclude modes for Voicemeeter and Zoom against exact process identities and
  the existing VB-Cable render endpoint, including same-process worker
  restart. An earlier retry supplied the full executable path in the adapter's
  basename field and correctly failed closed as an identity mismatch; the
  corrected basename/path split now qualifies both applications without
  fallback or persistent audio configuration changes.
  The exact Zoom application-capture acceptance was rerun elevated in both
  include and exclude modes after the capture polling compatibility retry;
  both modes passed with unchanged media-device identity/state.
- M04 DSP/recording and built-in pitch, M05 drag-and-drop editor, M06 local
  VST3/VST2 worker boundaries, and M07 CLI/MCP/headless/recovery foundations
  are implemented and tested. M04 pitch/recording, M06 fixture, M07 shell RPC,
  and M07 headless requalifications were refreshed on 2026-09-17.
- The 2026-09-17 VB-Cable digital impulse run detected 97/100 groups with
  zero p95 spacing error and a 76.56 ms estimated digital onset. This is not
  calibrated physical or acoustic latency evidence.
- Documentation validation covers 56 Markdown files and 224 local links;
  traceability covers 159 normative requirement IDs.

The post-privacy-propagation acceptance refresh on 2026-09-17 passed M07
headless (CLI 36, MCP 3, control 177 plus 3 guarded-live ignored,
plugin-host 70, worker-process 13, M01 CLI, and strict Rust checks), M05 UI
(typecheck, 257 tests, temporary production build), and contract drift (86
methods, 20 node kinds, 7 processors, 20 event categories). No device or
persistent machine configuration was changed by these checks.
The full workspace `cargo clippy --workspace --locked --all-targets
--all-features -- -D warnings` also passed after the privacy propagation
changes.
A subsequent `cargo test --workspace --locked` refresh passed across CLI/MCP,
control (177 passed, 3 guarded live ignored), domain (65), DSP (34), engine
(116), plugin-host (70), worker-process (13), protocol (8), recording (40),
storage (92), transport (19), Windows audio (88), and all doc-tests.
The release preparation command was also invoked against a new workspace-local
output path and stopped before any build or artifact creation with its
authoritative clean-tree error: `release inputs must come from a clean Git
working tree`. No release output directory was created.

The post-rebaseline full locked workspace run passed on 2026-09-17: CLI 36,
MCP stdio 3, control 177 with 3 guarded live tests ignored, domain 65, DSP
34, engine 116, plugin-host 70, storage 92, transport 19, Windows audio 88,
all doc-tests, and the remaining workspace suites. This is portable regression
evidence; it does not close attended UI, physical hardware, sandbox, clean
release, or deferred driver gates.

The latest current-tree refresh also passed `cargo test --workspace --locked`
and `cargo clippy --workspace --locked --all-targets --all-features -- -D
warnings`. The workspace counts were CLI 36, MCP stdio 3, control 177 plus 3
guarded-live ignored, domain 65, DSP 34, engine 116, plugin-host 70,
worker-process 13, protocol 8, recording 40, storage 92, transport 19, and
Windows-audio 88, with all doc-tests passing. This remains portable regression
evidence; it does not close attended, hardware, sandbox, clean-release, or
deferred driver gates.

The disposable elevated M08 NSIS smoke also passed on the current tree,
producing and removing a 4,888,262-byte x64 unsigned bundle while preserving
the source manifest. It did not execute or install the bundle; clean-tree
artifact preparation and production release gates remain blocked as documented.

The current M03 virtual-bus CLI acceptance also passed bounded desired-state
and persistence lifecycle, eight-bus capacity, cross-session route
validation, and managed-driver-unavailable reporting. Its temporary database
was removed; no native device or machine audio configuration was touched.

An earlier elevated M00 VB-Cable loopback also passed with 213,044 nonzero
captured payload bytes from a 1,000 ms capture while a 1,500 ms tone rendered
through the exact CABLE pair. Temporary probe resources were removed and no
machine audio state changed. This remains digital signal-path evidence, not
physical or acoustic latency evidence.

The current elevated event-driven M00 acceptance also passed on the exact
CABLE pair with 24,480 captured frames and 28,320 submitted render frames over
500 ms. Event registration and full stream lifecycle cleanup passed without
changing machine audio state.

The 2026-09-17 Rust adapter-bridge retry initially returned
`adapter_bridge_error=audio frame size was invalid` before telemetry because
the probe's current active inventory did not contain the supplied DELL/P32p-30
physical render IDs. The exact current CABLE Output/CABLE Input pair then
passed two bounded cycles with 24,480/24,960 captured frames, 191/195
processed quanta, 24,448/24,960 rendered frames, zero drops/xruns/deadline
misses, and finalized temporary recordings. Physical-monitor bridge selection
remains an endpoint-inventory limitation, not a new routing pass claim.

A later 2026-09-17 direct recheck found the same CABLE endpoint IDs still
enumerated, but `adapter-bridge` now fails during capture polling
initialization with Windows HRESULT `0x80070057` before any frames or
processing telemetry. That probe was non-elevated; it is not evidence of an
endpoint-format regression. The same endpoint pair succeeds when the bounded
probe is run from the documented elevated `deviceAdministration` context.
A same-day elevated rerun of the raw Rust initialization and full adapter
bridge succeeded: 4,800 captured frames, 37 processed quanta, 4,736 rendered
frames, zero drops/xruns/deadline misses, and a 25,072-byte finalized
temporary recording. The earlier failure was the non-elevated execution
context; guarded native adapter qualification requires the documented
`deviceAdministration` elevation boundary. The capture adapter's bounded
`E_INVALIDARG` duration retry remains in place, but is not treated as the
reason for this successful elevated run.
The authoritative elevated two-cycle acceptance then passed on the same exact
CABLE pair: each 500 ms cycle produced 51 packets, 24,480 captured frames,
191 processed quanta, 24,448 rendered frames, zero drops/xruns/deadline
misses, and a 25,072-byte finalized recording, with temporary resources and
media-device state cleaned unchanged.
An earlier bounded five-cycle endurance rerun passed with identical per-cycle
counts and zero drops/xruns/deadline misses or non-finite samples; the latest
five-cycle rerun has bounded packet/frame variation but the same zero-failure
result. All temporary streams/recordings were removed and media-device state
remained unchanged.

The same current tree passed `cargo clippy --workspace --locked
--all-targets --all-features -- -D warnings` on 2026-09-17. An implementation
scan found no in-scope TODO, `unimplemented!`, or unfinished feature path; the
remaining future/unsupported markers are documented capability boundaries or
deferred driver work.

After the OS-transition mapping changes, the locked workspace regression was
rerun on 2026-09-17 and passed again: CLI 36, MCP stdio 3, control 177 plus 3
guarded-live ignored, domain 65, DSP 34, engine 116, plugin-host 70,
worker-process 13, recording 40, storage 92, transport 19, Windows-audio 88,
and all doc-tests.

The current-tree refresh also passed the locked workspace regression and
`tests/acceptance/m05-ui.ps1` on 2026-09-17. M05 reported TypeScript
typecheck, 19 UI files/257 tests, and a temporary four-file production build.
The automated gate does not replace attended accessibility, first-run, or
live drag-and-drop evidence.
The 2026-09-18 resume checkpoint reran `tests/acceptance/docs.ps1` and
`tests/acceptance/m08-traceability.ps1`, which passed with 56 Markdown files,
224 local links, and 159 normative requirement IDs. It also reran
`cargo test --workspace --locked`: all current workspace suites and doc-tests
passed, with 3 guarded-live control tests correctly ignored. This refresh does
not close the unavailable interactive-surface, hardware, sandbox,
clean-checkout release, or deferred driver/signing gates.
After the polling-capture compatibility retry, the full locked workspace
regression was rerun: all workspace tests and doc-tests passed, strict
workspace Clippy passed, contract drift passed with 86 methods/20 node kinds/
7 processors/20 event categories, and documentation/traceability passed with
56 Markdown files/224 links/159 normative IDs.
The current editor slice also routes existing virtual-output shelf drops
through the supported physical-output branch, while managed virtual-bus drops
remain explicitly deferred; physical endpoints, processors, and recorder
destinations remain covered by the same draft shelf and connection tests.

`tests/acceptance/m07-headless.ps1` also passed on 2026-09-17 with 36 CLI,
3 MCP stdio, 177 control plus 3 guarded-live ignored, 70 plugin-host, and 13
worker-process tests, plus doc-tests. This refresh does not promote the
attended UI, physical-latency, managed-driver, signing, or clean-machine gates.
The contract drift checker was rerun with the same recovery change and passed
with 86 methods, 20 node kinds, 7 processors, and 20 event categories.
The shell-owned per-user startup helper is implemented and its exact
enable/disable round trip passed in the authorized elevated context on
2026-09-17; a read-only follow-up confirmed the `AudioRouter` value was absent
after cleanup. Attended startup usability and native audio restart remain
separate open gates.

Recovery implementation was tightened so a resume resnapshot now retains exact
endpoint changes for the deliberate rebind boundary and emits the bounded
`devices.changed` event immediately. Resume still never selects a replacement
or reopens a native stream automatically; the control regression suite passed
177 tests after this change, with three guarded live tests ignored.
The subsequent full locked workspace regression also passed after the live
privacy acceptance change: CLI 36, MCP 3, control 177 plus 3 guarded-live
ignored, domain 65, DSP 34, engine 116, plugin-host 70, worker-process 13,
protocol 8, recording 40, storage 92, transport 19, Windows audio 88, and all
doc-tests.

Specification audit correction: `PROD-03`, the product release slices,
`02-workflows.md`, `06-virtual-devices.md`, and the delivery decision register
now distinguish the supported current VB-Cable-first profile from the deferred
AudioRouter-owned managed-driver profile. Existing-device routing is therefore
not described as a driver or as managed-bus provisioning, while the original
managed requirements remain traceable for future signed delivery.

Security audit note: the plugin worker boundary currently revalidates binary
identity, runs outside the backend, uses a Windows kill-on-close job with one
active process and a 512 MiB process-memory cap, and fails closed on worker
failure. This is process/resource containment evidence. It is not full
filesystem/network sandboxing or a broad commercial-plugin compatibility claim;
those remain explicit limitations under SEC-07 and the M06 operations notes.
The explicitly selected installed ReaJS x64 VST2 candidate was also rejected
at the worker boundary because its state asset was empty; its fingerprint and
the temporary acceptance environment were restored. It remains an unsupported
candidate rather than an expanded compatibility claim.

Attended M05 review was attempted on 2026-09-17 through the Windows
computer-use surface, but its authoritative inventory returned no targetable
Windows applications or browsers. Therefore keyboard, Narrator, scaling,
first-run, and live drag/drop observations remain unverified; automated UI
tests must not be promoted to attended acceptance evidence. Reattempt when a
targetable interactive shell is available.

## Explicit non-driver audit boundary

The implementation/evidence audit leaves these requirement groups open or
externally blocked, rather than treating broad suite success as completion:

- `UI-01`–`UI-14` automated editor behavior is covered, but attended keyboard,
  Narrator, scaling, first-run, and live drag/drop acceptance remains open
  (`NFR-08`, `NFR-14`).
- `STATE-08` shell startup ownership and cleanup are qualified by the elevated
  exact-value round trip; attended sign-in/tray behavior and automatic native
  audio restart remain open.
- `STATE-11`, `CAP-11`, `CAP-12`, and `NFR-10`/`NFR-11` have bounded policy,
  authenticated delivery, exact notification mapping, listener
  startup/teardown, and guarded exact stopped rebind evidence. Actual
  lock/sign-out/sleep/resume delivery, endpoint re-enumeration caused by an
  OS transition, native reopen after that transition, and endurance cycles
  remain unverified.
- `NFR-01` is closed on one reference device under the revised ≤250 ms target
  (DEC-14). `NFR-02` is closed on the same device under the revised ≤160 ms
  target (DEC-15) — four runs measured p95 of 97.5/102.9/115.5/110.9 ms
  through the real engine route; see
  [the wasapi-probe evidence file](evidence/M00-wasapi-probe.md).
  `NFR-03`–`NFR-06`, `NFR-13`, `QUAL-01`, `QUAL-04`, and `QUAL-05`
  still need the declared physical reference hardware, calibrated
  latency/clock data, or endurance workload; digital impulse timing alone is
  not physical latency evidence.
- `PLUG-07`/`SEC-07` have local VST2/VST3 worker and resource-containment
  evidence, but redistribution rights, broad third-party compatibility, and a
  full filesystem/network sandbox remain open by documented boundary.
- `SEC-06`/`NFR-09` have focused evidence for durable authorization/state,
  process-local block silencing, repeated next-block fan-out silence, and
  native-worker propagation. The latest guarded user-mode
  processing-boundary sample measured 16.734 ms p95, but calibrated
  effective-output timing and an attended microphone-path measurement remain
  open; do not promote the sample to the full physical NFR-09 claim.
- `ENG-05`/M08 artifact preparation remains blocked by the dirty user worktree;
  no reset, stash, commit, or clean-tree bypass is authorized.

## Remaining tasks, in order

1. Keep the repository-wide non-driver requirement/evidence audit current;
   the present audit has recorded the supported VB-Cable-first routing,
   application, DSP/recording, plugin, API, recovery-policy, and packaging
   evidence in their owning milestone files. Privacy propagation into active
   native workers is implemented and regression tested; the next
   implementation slice is guarded endpoint revalidation and recovery after
   an allowed OS transition.
2. Preserve the completed local plugin qualification and keep redistribution
   rights, OS sandboxing, native editors, and unsupported formats explicitly
   bounded; qualify additional vendor binaries only when available and
   authorized.
3. Complete portable and Windows user-mode recovery evidence, including
   guarded OS-transition delivery and endpoint revalidation where the host
   permits it. Exact stopped rebind is now qualified; do not claim power-state
   delivery or transition-triggered native reopen from it.
4. Perform attended M05 accessibility, scaling, first-run, and drag/drop
   review when an interactive target is available; retain automated UI tests
   as separate evidence.
5. Prepare and verify release artifacts from a clean checkout. Do not call an
   unsigned or dirty-tree preparation a release qualification.
6. At completion, archive this plan with a dated milestone/task name and an
   index entry. If a task remains blocked, record the exact evidence and
   replace this file with the next actionable plan rather than claiming done.

M08 audit result: `tools/release/prepare-artifacts.ps1` has no override for
the clean-tree check and stops before building whenever `git status --porcelain
--untracked-files=all` is non-empty. The current checkout contains unrelated
user changes and this plan's documentation changes, so release preparation is
an evidence-backed clean-checkout prerequisite, not a code failure. Do not
stash, reset, commit, or discard the current worktree to bypass it.

An earlier elevated multi-input/many-output VB-Cable qualification on
2026-09-17 used `CABLE Output` plus Focusrite capture and `CABLE Input` plus
DELL physical render. It captured 47,040 frames, delivered 244 quanta,
rendered 47,232 frames, and cleaned up the same-process streams. This refreshes
the supported existing-device route evidence without changing defaults,
volume, mute, privacy, drivers, or persistent audio settings.

A subsequent earlier rerun used exact CABLE Output plus Focusrite capture and
Voicemeeter In 2 plus CABLE Input render endpoints, directly covering an
installed virtual output and an existing VB-Cable output. It delivered 48,480
captured frames through 259 graph quanta and rendered 47,712 frames; temporary
streams were cleaned and no persistent audio configuration changed.

A subsequent mixed-topology rerun used the same two captures with Voicemeeter
In 5 plus CABLE Input renders and passed with 48,000 captured frames, 273 graph
quanta, and 47,712 rendered frames. Current attempts using CABLE Input,
Voicemeeter In 2, or Focusrite render endpoints also preserved the expected
`deviceInUse` diagnostic when those endpoints were externally occupied; no
external clients were closed and no ownership conflict was hidden.

The elevated control-owned rebind lifecycle was refreshed again against exact
Focusrite capture, DELL render, and P32p-30 fan-out endpoints: 23,520 capture
frames, 183 graph quanta, 23,424 primary rendered frames, 169 fan-out packets,
and 21,632 fan-out rendered frames passed through stop, exact stopped rebind,
restart, and cleanup. The separate
multi-input/many-output run passed with exact Focusrite plus Voicemeeter
captures and DELL plus P32p-30 renders: 47,040 capture frames, 302 quanta,
and 46,752 rendered frames. The initial CABLE Input render attempt returned
the preserved `AUDCLNT_E_DEVICE_IN_USE` diagnostic; no ownership conflict was
masked as a successful route.

A further guarded control-owned lifecycle retry on 2026-09-17 selected exact
CABLE Output capture, P32p-30 render, and DELL fan-out render after the default
CABLE render reported `AUDCLNT_E_DEVICE_IN_USE`. The alternate run passed
stop, exact rebind, restart, and cleanup with 24,480 captured frames, 191
graph quanta, 24,448 primary rendered frames, 155 fan-out packets, and 19,840
fan-out rendered frames. No persistent audio state changed.

The elevated, explicitly authorized `m02-control-application-live.ps1`
acceptance was also refreshed against the currently running Zoom process using
its exact PID, `Zoom.exe` basename, verified full executable path, and creation
timestamp. Include mode passed two bounded start/pump/stop cycles,
same-process worker restart, and unchanged media-device state. The harness
restored its temporary environment; no endpoint default, volume, mute, privacy,
or persistent audio configuration changed.

An earlier 2026-09-17 guarded native privacy sample measured the explicit
control-dispatch-to-next-processed-block boundary on exact `CABLE Output`
capture, `P32p-30` render, and `DELL U3223QE` fan-out endpoints. Eight bounded
mute transitions produced a p95 of 22.132 ms and the route completed cleanup.
This superseded sample remains historical user-mode processing-boundary
evidence; the latest 16.734 ms sample is authoritative. Calibrated
effective-output timing and attended microphone-path measurement remain open.

The preceding elevated control-owned lifecycle used exact CABLE Output
capture, Voicemeeter In 5 primary render, and CABLE In 16ch fan-out. It delivered
24,000 captured frames, 187 processed quanta, 23,648 primary rendered frames,
and 166 fan-out packets covering 21,248 rendered frames. Privacy dispatch to
the next processed block measured 14.076 ms p95; exact rebind, restart, and
cleanup passed without persistent audio configuration changes. The default
CABLE Input retry remains recorded as the preserved `deviceInUse` diagnostic;
the latest physical PD200X fan-out run is authoritative for the current live
sample.

An earlier same-day elevated multi-input/many-output acceptance used exact `CABLE
Output` and Focusrite capture endpoints with DELL U3223QE and P32p-30 render
endpoints. It delivered 47,520 captured frames through 277 graph quanta and
rendered 48,512 frames; same-process cleanup passed without changing defaults,
volume, mute, privacy, or persistent audio configuration.

The earlier guarded application-capture acceptance also passed both include
and exclude modes against a prior exact Zoom identity (PID 21768, basename
`Zoom.exe`, verified executable path and creation time) and the existing CABLE
Input render endpoint. Each mode completed two bounded lifecycle cycles and a
same-process worker restart; media-device identity/state remained unchanged.

An additional elevated include-mode run used the same verified Zoom identity
with exact Voicemeeter In 5 as the render destination. Two bounded lifecycle
cycles and same-process restart passed with unchanged media-device state,
extending application/tool routing evidence to an installed virtual endpoint.
The matching exclude-mode run against Voicemeeter In 5 also passed the same
two-cycle and restart boundary with unchanged media-device state.

On 2026-09-17 the focused control recovery regression
`recovery_endpoint_resnapshot_publishes_a_bounded_device_change_event` passed,
and the elevated `m07-shell-rpc.ps1` acceptance passed through WebView2 startup,
the native command, and authenticated `system.describe`. These refresh
portable recovery and shell-transport evidence only; actual OS power/session
delivery, endpoint re-enumeration, and automatic native reopen remain open.

## Validation matrix

| Area | Evidence | Current result |
| --- | --- | --- |
| Docs/traceability | `docs.ps1`, `m08-traceability.ps1` | 56 files/224 links; 159 IDs |
| DSP/recording | `m04-dsp-recording.ps1` | 34 DSP + 40 recording passed |
| UI | `m05-ui.ps1` | typecheck, 257 tests, production build passed |
| Headless | `m07-headless.ps1` | CLI/MCP/control/plugin/worker passed; guarded live tests ignored |
| M06 | VST3/VST2 worker and editor wrappers | local fixture boundaries passed |
| VB-Cable route | guarded M02/M03 wrappers | multi-input/fan-out/tool routes passed |
| OS transitions | 29 shell tests, `m07-shell-rpc.ps1`, guarded exact rebind | policy/shell transport and stopped rebind passed; attended OS delivery/reopen open |
| Privacy mute | focused engine/control Rust tests; guarded native timing sample | realtime block gate, fan-out silence, durable authorized state, worker propagation, and 16.734 ms user-mode processing-boundary p95 passed; calibrated output-boundary timing open |
| NFR-01 physical latency | `m00-native-impulse-loopback.ps1`, wired Focusrite loopback | p95 ≈ 155–186 ms across four runs, passes the revised ≤250 ms target (DEC-14); validated on one reference device only |
| NFR-02 virtual-capture latency | `m02-nfr02-mic-virtual-capture.ps1`, live engine route + wired Focusrite loopback | p95 ≈ 97–115 ms across five runs, passes the revised ≤160 ms target (DEC-15); validated on one reference device only |
| Release | M08 preparation | blocked until clean checkout is supplied |

## 2026-09-18 completion audit

The attended-surface recheck on 2026-09-18 returned `apps: [], browsers: []`
again from the computer-use inventory. No Windows application or browser was
targetable, so no keyboard, Narrator, scaling, first-run, live drag/drop, or
native Test Signal plan/commit/start/meter/stop observation was attempted or
claimed. Reattempt when an interactive shell is exposed; portable acceptance
remains authoritative for the implementation slice.

The M05 Test Signal/destination-meter slice was implemented and refreshed on
2026-09-18. The locked workspace regression passed: CLI 36, MCP stdio 3,
control 177 plus 3 guarded-live ignored, domain 66, DSP 34, engine 118,
plugin-host 70, worker-process 13, protocol 8, recording 40, storage 92,
transport 19, Windows audio 88, and all doc-tests. M05 UI acceptance passed
typecheck, 19 UI files/260 tests, and a temporary four-file production build.
Contract drift passed with 86 methods, 20 node kinds, 7 processors, and 20
event categories; documentation validation passed with 56 Markdown files and
224 local links. These are portable/editor checks only; native plan/commit,
endpoint playback, attended UI, physical-latency, driver, signing, and clean
release gates remain separate.

The disposable `tests/acceptance/m08-installer-smoke.ps1` also passed on
2026-09-18, producing a 4,887,249-byte unsigned x64 NSIS installer after
verifying the source manifest hash and cleaning its bundle output. This is
installer-generation evidence only; the clean-checkout, signing, install,
driver, hardware, and release-publication gates remain open.

The elevated `tests/acceptance/m07-shell-rpc.ps1` refresh also passed on
2026-09-18, covering WebView initialization, the Tauri command boundary, and
authenticated `system.describe` with disposable state. It changed no audio
endpoint or persistent machine configuration and does not close attended UI,
OS-transition delivery, native-reopen, or release gates.

The elevated M02 native lifecycle requalification on 2026-09-18 preserved an
endpoint-specific `AUDCLNT_E_DEVICE_IN_USE` (`0x8889000A`) result for the
occupied CABLE render endpoint, then passed against the exact CABLE capture and
already qualified PD200X render endpoint: 24,000 captured frames, 187
processed quanta, 23,936 rendered frames, 187 fan-out packets, 23,936
fan-out frames, and 11.215 ms privacy dispatch-to-processed p95. This is
existing-device lifecycle evidence only; physical latency, OS-transition
reopen, managed-driver, and release gates remain open.

The current `tests/acceptance/m07-headless.ps1` refresh passed on 2026-09-18:
CLI 36, MCP stdio 3, control 177 with 4 guarded-live tests ignored,
plugin-host 70, worker-process 13, associated doc-tests, the M01 CLI
acceptance, and `git diff --check`. This refresh strengthens the headless
control-plane evidence but does not close attended UI, actual OS-transition,
physical, driver, signing, or clean-checkout release gates.

The active-plan audit found one active execution plan (`current.md`) and no
additional active execution-plan files. The milestone documents are stable
contracts, not separate plans to archive. The current VB-Cable-first scope has
portable and guarded existing-device evidence through M07, but the plan is not
complete because the following gates remain authoritative and open:

- M00/M03 managed virtual-device provisioning, production signing, and
  clean-machine driver qualification remain deferred to the future driver
  plan.
- M02/M04 physical or acoustic latency, long-duration/endurance, and attended
  listening evidence remain open even where digital or portable substitutes
  pass.
- M05 attended keyboard/Narrator, scaling, and first-run remain open. A
  first attended interaction/drag-drop review happened 2026-09-21 (see
  [the M05 evidence file](evidence/M05-visual-editor.md)). An initial test
  harness flaw (see the M07 methodology-defect entry) produced one false
  finding (a side-panel parameter editor that appeared broken but was not,
  retracted); the canvas mini-fader/EQ drag bug it also surfaced was real,
  found (a 3-argument callback silently misread as 2 arguments in
  `ui/src/App.tsx`), fixed, and confirmed by the user, with `npm run
  typecheck` and the full 282-test UI suite passing. The light-theme
  contrast/consistency bug was also found and fixed (hardcoded-dark CSS
  from a later redesign pass had silently overridden the pre-existing but
  correctly-wired theme system; converted to CSS custom properties so
  theme switching is robust to selector specificity/order) and confirmed
  across several rounds of user retesting, alongside a duplicated
  "Session inventory unavailable" label fix found along the way. The
  connection-anchor and input/output-distinction findings were also fixed
  together (same root cause): the canvas nodes had no registered
  `nodeTypes`, so React Flow silently fell back to its built-in "default"
  node type, which adds its own implicit Top/Bottom handles on top of the
  app's own fully custom ones — producing 3 overlapping handles on the
  top/bottom edges of a 2-port node (2 on left/right) and very plausibly
  explaining the original "top anchor doesn't work" report. Registered a
  proper pass-through custom node type to fix it, confirmed by the user
  after three rounds of color-scheme iteration (final: blue input / orange
  output, plus a permanent on-canvas legend and clear in-app feedback for
  an invalid-direction drag attempt, which previously failed silently).
  Two further findings were also added and fixed the same day: the canvas
  library had no way to add an application-capture source (a working
  "Applications" panel already existed but was hidden by default and
  absent from the library palette; added an "Application" library entry
  opening a focused picker, iterated to a filtered dropdown after user
  feedback that an unfiltered per-row list with disabled entries was
  confusing), and a clarification that Windows has no "application as
  output" concept symmetric with application capture (verified against
  `NodeKind` in the domain crate; corrected the library tooltip to point at
  the already-working "Existing virtual output" mechanism instead of the
  permanently-disabled, deferred-driver "Virtual capture sink"). Still open
  and not yet fixed: an M08 release qualification gap. The
  graph-native Test Signal and destination-meter slice is implemented and
  its exact stopped-to-plan/commit-to-start-to-meter-to-stop workflow passed
  against the authorized exact CABLE/PD200X endpoints, but automated passes
  do not substitute for the remaining attended defects.
- M07 attended tray review happened 2026-09-21, initially under the same
  flawed test harness noted above; corrected and reran once the harness
  issue was found (see
  [the M07 automation evidence file](evidence/M07-automation-recovery.md#attended-testing-methodology-defect-found-and-corrected-2026-09-21)):
  Close window/reopen from tray passed both times; Quit and stop audio
  initially appeared to fail but passed cleanly on the corrected retest
  (confirmed via `tasklist`) — the original "failure" was a harness
  artifact, not a real defect. The current authenticated transport is a
  same-user Windows named pipe; Firefox, Claude/ChatGPT Work, and remote MCP
  clients do not have browser transport access.
- M08 clean-tree artifact preparation, signed installer, clean install/upgrade
  and uninstall, hardware/app matrix, and release publication evidence remain
  open. The worktree is intentionally dirty and was not reset, stashed, or
  bypassed.

Current portable refresh on 2026-09-18 passed:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\docs.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m08-traceability.ps1
npm.cmd --prefix ui run typecheck
npm.cmd --prefix ui test -- --run
```

Results were 56 Markdown files/224 local links, 159 normative IDs, and 19 UI
test files/257 tests. These checks are portable documentation/UI evidence only;
they do not close the open Windows-attended, physical, driver, signing, or
clean-release gates. The plan remains active with the exact next slice in the
actionable handoff below. It must not be archived as a completed milestone
until those gates are either satisfied with matching evidence or explicitly
superseded by a new authorized scope decision.

## Risks and rollback

Do not alter default devices, volume, mute, privacy, driver state, startup
registration, or persistent audio settings during guarded qualification.
Temporary endpoints, processes, databases, recordings, and build outputs must
be exact-run-owned and removed. Revert documentation-only status changes by
restoring the archived plan; revert implementation changes with focused
patches and rerun the affected acceptance suite. Never reset or discard
unrelated user changes.

## Next action

### 2026-09-18 M05 Test Signal and destination-meter execution slice

Objective: implement the graph-native Test Signal source and expose bounded
destination meter observations through the existing backend/editor contracts.

Requirement IDs: UI-01, UI-05, UI-12, GRAPH-08, DSP-07, NFR-08, ENG-01.

Prerequisites and decisions: preserve the stopped/unarmed default, use the
existing validated node-parameter path, keep generation in the realtime graph
allocation-free, and do not add Web Audio, browser microphone access, or
endpoint-default changes. The source is an ordinary versioned node with
bounded frequency, level, and duration parameters. Existing Meter nodes remain
the destination observation boundary.

Ordered tasks:

1. Extend the domain, shared TypeScript contract, discovery metadata, graph
   compiler, and UI library with Test Signal while preserving unknown-node and
   parameter rejection behavior.
2. Add a prepared realtime source stage whose phase/duration state is bounded
   and reset at activation/stop; add focused domain/engine regression tests.
3. Add the Test Signal acceptance shape to the editor and expose existing
   bounded meter observations with explicit stopped/no-signal/not-prepared
   wording; add focused UI tests.
4. Run contract drift, domain/engine/control tests, UI typecheck/tests, and
documentation validation. Record exact results and remaining Windows gates
here before changing plan status.

The administrator-authorized `tests/acceptance/safe-all.ps1` chain was rerun
on 2026-09-18. Its available M00 through M07 steps passed, and its M08 release
step stopped at `tools/release/prepare-artifacts.ps1:39` with `release inputs
must come from a clean Git working tree`; the runner removed 15 run-owned
temporary children. The existing dirty worktree was preserved, so clean-tree
artifact preparation remains an external prerequisite and no release artifact
claim is made.

Validation matrix: portable contract/domain/engine/control/UI tests first;
guarded Windows route validation only if the existing authorized endpoint
context is available. The latter must prove plan/commit, start, meter signal,
stop, unchanged endpoint identity/default state, and cleanup. A portable pass
does not close attended UI, physical-latency, driver, signing, or release gates.

Rollback: revert only the Test Signal/meter patches and their evidence entry;
do not reset, stash, commit, or discard unrelated worktree changes. If a
native gate is unavailable, retain the implementation as unqualified and
document the exact missing evidence.

The 2026-09-18 UI simplification pass applies the M05 visual-editor direction
from the supplied Audio Hijack reference: the default editor foregrounds the
session list, signal-flow canvas, drag/drop node library, and transport/status
actions. Dense setup, endpoint inventory, plugin, recovery, recording-detail,
and diagnostic panels are visually removed from the default path while their
existing backend-backed controls remain in the source for the advanced/editor
workflow. The pass is presentation-only and does not change graph semantics,
audio lifecycle, or authorization. UI typecheck and the full 257-test suite
passed after the change. Attended visual, scaling, Narrator, and live drag/drop
acceptance remain open because the interactive Windows surface is unavailable.

The same review identified a separate transport boundary for browser clients:
the current shell/backend uses an owner-authenticated Windows named pipe, not a
TCP listener, so Firefox, Claude/ChatGPT Work, and MCP cannot yet be treated as
equivalent browser clients. The existing tray already exposes Open, close,
quit, privacy, status, and recorder actions, but it does not have a truthful
IP/port to display. `docs/spec/10-api.md` explicitly classifies a standalone
browser adapter as future scope; therefore loopback HTTP/WebSocket JSON-RPC,
origin/authentication policy, and a browser-facing tray endpoint are not active
M07/M05 completion tasks. Do not claim browser or remote control support.

The follow-up M05 polish on 2026-09-18 added a generated waveform tray icon,
an expanding canvas layout, an initialization-time ReactFlow fit pass, and
clearer draft/VoiceMeeter guidance in the editor. UI typecheck and all 257 UI tests
passed; the Tauri shell also passed `cargo check`. Live tray rendering and
interactive signal playback remain Windows-attended evidence rather than
portable test evidence.

The documentation reconciliation on 2026-09-18 updated the interface,
M05/M07 milestone contracts, and quickstart to define the graph-native
Test Signal plus meter slice, explain VoiceMeeter endpoint ownership, give the
current raw VB-Cable signal smoke, and preserve the named-pipe/browser
transport boundary. The Test Signal/meter implementation is now portable-
tested; no browser listener is being represented as implemented.

The 2026-09-17 headless, documentation/traceability, plugin-worker,
application-capture, and existing-device rebind gates are current. Continue
with the next actionable non-driver slice: advance guarded Windows recovery
evidence where the host permits it, and prepare a clean-checkout release run
when an authorized clean checkout is supplied. Do not promote automated UI
tests or policy-only transition tests to attended acceptance, and retain the
documented driver, signing, physical-latency, sandbox, accessibility, and
clean-release limits.

### Actionable handoff

- The supported exact CABLE bridge is now requalified through five elevated
  cycles with zero drops, xruns, deadline misses, or non-finite samples; keep
  the documented elevated `deviceAdministration` boundary and do not use
  Windows-default or driver changes as a workaround for non-elevated
  `0x80070057`.
- Existing physical output is qualified through the PD200X fan-out route;
  Focusrite primary-render attempts remain endpoint-specific
  `deviceInUse` evidence and must not be represented as a general hardware
  pass.
- Reattempt attended M05 and actual OS transition delivery only when the
  computer-use inventory exposes an interactive Windows surface; the latest
  authoritative inventory was `apps: [], browsers: []`.
- A fresh computer-use inventory recheck on 2026-09-18 again returned
  `apps: []` and `browsers: []`; no attended UI or real lock/sign-out/
  sleep/resume transition was attempted, and the corresponding gates remain
  open rather than being inferred from portable policy tests.
- Run `m08-release.ps1` only from an authorized clean checkout; the current
  worktree is intentionally dirty and no reset, stash, commit, or bypass is
  authorized.
- Next M05/M02 qualification slice: qualify the Test Signal workflow against
  bounded per-node/destination meters, then qualify the stopped → plan/commit →
  start → meter → stop workflow with exact existing endpoints. Keep VoiceMeeter
  coexistence and `deviceInUse` ownership diagnostics explicit.

This handoff is superseded by the 2026-09-18 native Test Signal acceptance:
the exact CABLE/PD200X plan/commit/start/meter/stop workflow passed. The next
actionable non-driver slice is guarded OS-transition delivery and endpoint
revalidation where the host permits it.

The elevated M02 multi-input/many-output acceptance also passed on 2026-09-18
with exact CABLE Output plus Focusrite capture and DELL plus PD200X render
endpoints: 47,520 captured frames, 270 delivered quanta, and 48,000 rendered
frames. This refreshes bounded existing-device fan-out evidence without
changing persistent audio configuration; physical latency, endurance,
OS-transition reopen, and managed-driver gates remain open.

The elevated M00 event-driven WASAPI refresh also passed on 2026-09-18 using
exact CABLE Output capture and silent PD200X render endpoints: 24,480 capture
frames and 28,320 submitted render frames. Event initialization/start/stop/
reset and cleanup passed with no media-device identity/state change. This is
not calibrated physical-latency or managed-driver evidence.

The elevated M02 application-capture refresh also passed on 2026-09-18 against
the observed `voicemeeterpro.exe` identity (PID 71412, creation timestamp
`134342414024784043`, verified path) and exact PD200X render endpoint. Include
mode completed two bounded start/pump/stop cycles plus same-process worker
restart with unchanged media-device state. No process or persistent audio
configuration was changed; this is not universal application compatibility
evidence.

The current local M06 VST2 acceptance also passed for six x64 fixtures at
44.1, 48, and 96 kHz with worker, parameter-offset, and fixture-integrity
checks. No registration, audio stream, or machine audio configuration action
occurred. Redistribution rights, full sandboxing, broad vendor compatibility,
native editors, and release gates remain explicitly open.

The current `cargo test --workspace --locked` regression passed on 2026-09-18:
CLI 36, MCP stdio 3, control 177 plus 4 explicitly guarded live tests
ignored, domain 66, DSP 34, engine 118, plugin-host 70, worker-process 13,
recording 40, storage 92, transport 19, Windows audio 88, and all workspace
doc-tests. This confirms the current shared tree after the native Test Signal
acceptance; it does not close attended, physical, OS-transition, driver,
signing, sandbox, or clean-release gates.

The elevated pinned VST3 SDK acceptance also passed on 2026-09-18 after the
initial non-elevated MSBuild FileTracker access-denied failure. SDK/fixture
build, validator self-tests, offline loader, parameter descriptors,
auxiliary-bus handling, explicit single-bus rejection, and the five-class mda
matrix passed. No system registration, audio stream, or machine audio
configuration changed; third-party rights, sandbox, native-editor, and
release gates remain open.

The current M04 DSP/recording acceptance also passed on 2026-09-18: 34 DSP
tests, 40 recording tests, associated doc-tests, formatting, strict package
Clippy, and diff checks. This refreshes portable processing and file-boundary
evidence only; physical pitch/latency and long-duration live-performance
gates remain open.

The current `tests/acceptance/m01-cli.ps1` parity acceptance also passed on
2026-09-18, refreshing offline discovery/help/schema, shared dispatch,
persistence, authorization, virtual-route, recording, recovery, startup,
and MCP/CLI parity without audio-endpoint or machine-configuration changes.

The current `tests/acceptance/m03-virtual-buses.ps1` acceptance also passed,
refreshing eight-bus desired-state capacity, lifecycle persistence,
cross-session route validation, cycle/conflicting-writer rejection, and stale
revision safety. No native device provisioning, driver installation/loading,
or audio configuration action occurred.

The current x64 M03 prototype build acceptance also passed, covering WDK
source-contract compilation, INF/package qualification, signability/catalog
checks, and lifecycle guards without installation, loading, signing-mode,
boot-policy, service, or audio configuration changes. The read-only signing
prerequisite check remains unresolved because `Win32_DeviceGuard` is
unavailable on this host; no VBS or signing conclusion is claimed.

The non-installing ARM64 M03 driver-build acceptance also passed on
2026-09-18, covering the ARM64 WDK compile, INF/package, signability/catalog,
and lifecycle guard path. No installation, loading, signing-mode, boot-policy,
service, or audio configuration action occurred.

The no-side-effect M03 software-device API probe also passed on 2026-09-18,
compiling and reporting the bounded `SWD\\AudioRouterVirtual` dry-run plan
before removing its temporary artifacts. No software device, driver,
endpoint, or machine audio configuration was created.

The elevated read-only M00 native-format inventory also passed on 2026-09-18
for 34 active endpoints. Mix-format metadata was successful for every endpoint
with observed 48 kHz mono/stereo and 96 kHz mono/eight-channel float layouts;
media state and temporary-artifact cleanup were unchanged. No stream, driver,
or machine configuration action occurred.

The read-only M00 toolchain compatibility acceptance also passed on
2026-09-18 with Visual Studio 18 Community, MSVC 14.51.36231, Windows
SDK/WDK `10.0.28000.0`, and matching `signtool.exe`. No SDK installation,
driver action, signing-mode change, or audio configuration action occurred.

The focused recovery refresh on 2026-09-18 passed 7 locked control
OS-transition tests and all 29 locked `src-tauri` shell tests. The fresh
computer-use inventory remained empty (`apps: []`, `browsers: []`), so no
attended lock/sign-out/sleep/resume or UI acceptance was attempted; real
transition delivery, endpoint re-enumeration, and native reopen remain open.

The M08 traceability acceptance was rerun on 2026-09-18 and passed with all
159 normative requirement IDs covered by the delivery map. This confirms
documentation coverage only and does not promote the remaining hardware,
driver, signing, attended, sandbox, or clean-release gates.

The current-tree `cargo test --workspace --locked` rerun on 2026-09-18 also
passed: CLI 36, MCP stdio 3, control 177 plus 4 explicitly guarded live tests
ignored, domain 66, DSP 34, engine 118, plugin-host 70, worker-process 13,
protocol 8, recording 40, storage 92, transport 19, Windows audio 88, and all
workspace doc-tests. No live-test credentials or endpoint state were changed.

The same current-tree strict static gates also passed on 2026-09-18:
`cargo fmt --all -- --check` and
`cargo clippy --workspace --locked --all-targets --all-features -- -D warnings`.
Milestone review found no additional portable implementation task authorized
by the current VB-Cable-first/non-driver boundary; the remaining tasks are
explicitly attended, hardware, signed-driver, rights/sandbox, or clean-release
gates.

The external prerequisite recheck on 2026-09-18 again found no interactive
Windows surface (`apps: []`, `browsers: []`), and the read-only M03 signing
check again failed because `Win32_DeviceGuard` is unavailable. No OS security,
signing mode, driver, or audio configuration was changed.

The direct rerun of `tests/acceptance/m03-signing-prerequisites.ps1` on
2026-09-18 produced the same explicit `Win32_DeviceGuard is unavailable;
cannot record the VBS prerequisite` result. This is a host-capability blocker,
not evidence of signing or HVCI readiness.

The current UI verification also passed on 2026-09-18 through `npm.cmd`:
TypeScript typecheck succeeded and Vitest reported 19 test files and 260 tests
passed. The initial `npm` wrapper invocation was blocked by PowerShell
execution policy; no policy change was made. This remains automated UI evidence
only, separate from attended accessibility, scaling, first-run, and drag/drop
acceptance.

The dedicated `tests/acceptance/m05-ui.ps1` wrapper then passed on 2026-09-18,
covering the same typecheck and 19-file/260-test suite plus a temporary
production Vite build (4 output files). The wrapper removed its temporary
output; no audio, driver, or machine configuration changed. This strengthens
automated M05 evidence only and does not close attended UI gates.

The dedicated `tests/acceptance/m07-headless.ps1` wrapper also passed on
2026-09-18: 36 CLI tests, 3 MCP stdio tests, 177 control tests plus 4
explicitly guarded live tests ignored, 70 plugin-host tests, 13 worker-process
tests, and associated doc-tests. The wrapper also passed the M01 CLI
acceptance; no audio-device, driver, or machine configuration changed.

The current `tests/acceptance/m03-virtual-buses.ps1` acceptance also passed,
refreshing bounded desired-state capacity, route persistence, cycle and
conflicting-writer rejection, and revision safety. It performed no native
device provisioning, driver installation/loading, or machine audio
configuration action.

The current `tests/acceptance/m04-dsp-recording.ps1` acceptance also passed:
34 DSP tests, 40 recording tests, both crate doc-test sets, and its formatting,
strict package Clippy, and diff checks. This refreshes portable transfer,
pitch, recording, checkpoint, recovery, segmentation, and library evidence;
physical pitch/latency and long-duration live-performance gates remain open.

The current `tests/acceptance/m06-vst2-state-fixture.ps1` acceptance passed at
44.1, 48, and 96 kHz. Repository-owned ignored DLL fixtures covered chunk
state restore, legacy-main handling, non-finite output rejection, and native
fault containment through disposable workers. No plugin registration, audio
stream, or machine audio configuration changed; rights, full sandboxing,
native editors, and broad compatibility remain open.

The current `tests/acceptance/m00-native-build.ps1` acceptance also passed,
revalidating the compile-only native WASAPI probe boundary. No audio stream,
driver, signing-mode, or machine-configuration action occurred.

The current ARM64 M03 driver-build acceptance also passed, covering the
project-owned WDK compile, signability, and catalog qualification path. No
installation, loading, signing-mode, boot-policy, service, or audio
configuration action occurred; production signing and clean-machine
qualification remain open.

The matching x64 M03 driver-build acceptance also passed the project-owned
WDK compile, signability, and catalog qualification path. No installation,
loading, signing-mode, boot-policy, service, or audio configuration action
occurred; production signing and clean-machine qualification remain open.

The current M03 Software Device API probe also passed its native compile and
no-side-effect `SWD\\AudioRouterVirtual` dry-run; the temporary executable was
removed. No software device, driver, endpoint, or machine audio configuration
was created.

The current `tests/acceptance/m06-vst3-worker.ps1` acceptance also passed for
the pinned local `again.vst3` fixture. It covered isolated single-stream and
auxiliary-bus processing, asynchronous graph staging, bounded
restart/quarantine recovery, validated state restoration where supported,
repeated-quantum timing, finite output, and bounded shutdown. No plugin
registration, audio stream, or machine audio configuration changed; rights,
full sandboxing, native editors, and release qualification remain open.

The current M06 SDK installer-provenance acceptance also passed using
disposable Git metadata checks. No SDK, plugin, driver, or audio configuration
was changed.

The unsigned M08 NSIS smoke was requalified on 2026-09-18: its restricted
attempt hit npm registry/cache `EACCES`, while the authorized elevated retry
passed and produced a disposable x64 `AudioRouter_0.1.0_x64-setup.exe` bundle
of 4,892,281 bytes with manifest-hash verification. Temporary bundle output
was removed. No installer execution, installation, signing, driver, or audio
configuration action occurred; clean-checkout, signing, install/upgrade/
rollback, and managed-driver gates remain open.

The post-acceptance cleanup inspection found pre-existing `audiorouter-*`
temporary databases/cache entries under the user temp directory. Ownership
could not be established safely from the current state, so none were deleted;
the acceptance-owned installer output itself was removed. This is a cleanup
limitation, not release evidence.

The M08 traceability and documentation gates were rerun on 2026-09-18:
159 normative IDs remained covered, and documentation validation passed for
56 Markdown files and 224 local links. `git diff --check` passed.

The read-only M00 format inventory was rerun on 2026-09-18. Its non-elevated
attempt received `Get-PnpDevice` access denied (`0x80041003`); the authorized
elevated retry passed for 34 active endpoints with bounded 48 kHz mono/stereo
and 96 kHz mono/eight-channel float formats. No stream, driver, or machine
configuration action occurred.

The current M06 VST2 acceptance also passed for six x64 local fixtures at
44.1, 48, and 96 kHz, including intra-block parameter-offset and before/after
binary-integrity checks through disposable workers. No plugin registration or
audio configuration changed; rights, full sandboxing, native editors, and
broad compatibility remain open.

The read-only M00 toolchain compatibility check also passed on 2026-09-18:
Visual Studio 18 Community, MSVC 14.51.36231, matching SDK/WDK 10.0.28000.0,
and matching x64 `signtool.exe` were discovered. No SDK installation, driver
action, signing-mode change, or audio configuration action occurred.

The M07 shell-RPC acceptance was rerun on 2026-09-18. The non-elevated wrapper
correctly refused the administrator-only WebView setup; the authorized
elevated retry passed WebView initialization through Tauri to authenticated
backend `system.describe` using disposable state. No audio endpoint or
persistent machine configuration changed.

The dedicated `tests/acceptance/m01-cli.ps1` acceptance was rerun on
2026-09-18 and passed offline discovery/help/schema, shared dispatch,
persistence, authorization, virtual-route, recording, recovery, startup, and
MCP/CLI parity checks. No audio endpoint, driver, or machine audio
configuration was accessed.

The current contract-drift check also passed: 86 methods, 20 node kinds, 7
processors, and 20 event categories match the UI/CLI/Rust catalogs.

The guarded Rust adapter route also passed on the exact CABLE Output/PD200X
pair: 48 kHz capture/render, 128-frame graph quantum, 2.666667 ms deadline,
24,000 captured frames, 187 graph blocks, 23,936 scheduler/routed frames,
zero deadline misses, and maximum processing time 57,700 ns (p999 upper bound
65,536 ns). Temporary probe artifacts were removed and defaults, volume, mute,
privacy, drivers, signing, and startup configuration were unchanged.

The guarded M02 native lifecycle requalification preserved the default CABLE
render `deviceInUse` diagnostic (`0x8889000A`), then passed with exact CABLE
Output capture and PD200X render: 24,000 captured frames, 187 processed
quanta, 23,936 rendered frames, 187 fan-out packets, and 23,936 fan-out
frames. Privacy mute dispatch-to-processed-block p95 was 11.007 ms; stop,
reset, exact rebind/restart, and cleanup passed without persistent audio
configuration changes.

The guarded application-capture retry preserved a rounded-DMTF identity
failure, then passed after a direct `Process.StartTime.ToFileTimeUtc()` query
supplied exact identity `134342414024784043` for `voicemeeterpro.exe` PID
71412. Include mode completed two bounded start/pump/stop cycles and a
same-process worker restart against the exact PD200X render; media-device
state and persistent audio configuration remained unchanged.

The guarded M02 multi-input/many-output acceptance also passed with exact
CABLE Output plus Focusrite captures and CABLE Input plus DELL renders:
48,000 captured frames, 283 delivered quanta, and 47,808 rendered frames.
Same-process cleanup passed without persistent audio configuration changes.

The complementary guarded application-capture run also passed in `exclude`
mode for the same verified Voicemeeter identity and PD200X render, including
two bounded start/pump/stop cycles and same-process worker restart with
unchanged media-device state and no persistent audio configuration.

The guarded M02 control-owned route adapter also passed for exact CABLE Output
capture and PD200X render: generation 1, 50 packets, 24,000 captured frames,
187 processed quanta, 23,936 rendered frames, 95,788 recording bytes, one
successful start/stop, one reset, one rejected pump, and 48 kHz on both
directions. The worker stopped/detached without changing endpoint defaults,
volume, mute, privacy, drivers, signing, startup, registration, or persistent
audio configuration.

The guarded Rust adapter bridge also passed two 500 ms cycles on exact CABLE
Output capture and PD200X render: cycle 1 captured/rendered 24,960 frames in
195 quanta; cycle 2 captured 24,480 and rendered 24,448 in 191 quanta. Both
were 48 kHz stereo with zero non-finite tap samples, dropped frames, xruns,
and deadline misses; temporary resources were removed and media-device state
remained unchanged.

The guarded native digital impulse correlation refresh also passed against the
existing VB-Cable pair: 97 of 100 expected groups detected, zero-frame p95
spacing error, and estimated 81.02 ms digital onset. Temporary probe artifacts
were removed. This refresh is bounded software signal-correlation evidence,
not calibrated acoustic/physical latency or managed-driver timing evidence.

The current `tests/acceptance/m07-headless.ps1` refresh also passed: 36 CLI
tests, 3 MCP stdio tests, 177 control tests with 4 guarded live tests ignored,
70 plugin-host tests, 13 worker-process tests, and the M01 CLI acceptance. It
changes no endpoint or machine configuration and does not close the real
OS-transition, native-reopen, attended-shell, or endurance gates.

## Current handoff refresh (2026-09-18)

The previously listed M05 Test Signal/destination-meter execution slice is
implemented and its portable, UI, contract-drift, and guarded exact-endpoint
evidence is recorded above. The next actionable slice is guarded user-mode
recovery after an allowed OS transition: first qualify notification delivery
and endpoint revalidation/reopen where the host permits it, then update the
M07 recovery evidence without claiming unattended lock/sign-out/sleep,
endpoint re-enumeration, or endurance gates that were not observed. If no
interactive or authorized transition surface is available, retain the exact
blocked evidence and proceed with documentation-only audit maintenance; do
not substitute portable tests for those Windows gates.

The current committed clean tree (`5d2123cd`, `Complete VB-Cable-first
milestone work`) passed `tests/acceptance/m08-release.ps1`. Optimized Rust
release binaries and the 214-module production UI were built, unsigned
artifacts were prepared and verified, and disposable outputs were removed.
This closes unsigned artifact preparation only; signing, installer execution,
clean-machine, managed-driver, and publication gates remain open.

The active `tests/acceptance/safe-all.ps1` harness was narrowed to this
VB-Cable-first track: deferred M03 driver-build, ARM64-driver, signing,
Software Device API, and SysVAD steps remain available as separate future
track scripts but are no longer run as active completion gates.

The corrected active safe chain then passed end-to-end on 2026-09-18. It
covered M00, M01, existing-device M03 routing, M04, M05, M06, M07, unsigned
M08 artifact preparation and NSIS smoke, frontend-owned shell RPC, M08
traceability, and documentation. No driver was installed or loaded and no
persistent audio configuration changed.

The clean-commit guarded M05 Test Signal acceptance then passed against exact
CABLE Output capture and CABLE Input render endpoints: 187 processed quanta,
destination peak `-18.000 dB`, and successful plan/commit, start, meter,
stop, and cleanup. This is existing-device graph evidence, not attended UI or
physical-latency qualification.

The exact physical PD200X speaker/microphone impulse attempt was also run with
100 impulses and detected 0 groups, so the analyzer rejected it. Temporary
artifacts were cleaned. The result preserves the current evidence boundary:
digital CABLE correlation is repeatable, but calibrated acoustic/physical
latency remains unverified.

The user-authorized higher-volume Focusrite-headset retry detected 733 of
1,000 physical impulse groups and correctly failed the ≥900 acceptance
threshold. It remains insufficient for NFR-01; a wired Focusrite output-to-
input loopback is still required for a stable calibrated measurement.

The subsequent retry detected 2,006 groups for 1,000 impulses with 471-frame
p95 spacing error and an estimated 188.98 ms onset. It is explicitly rejected
as a physical-latency result despite the wrapper exit code being zero because
the cadence is not valid.

## Physical-latency gate parked (2026-09-18)

The calibrated physical-latency gate is intentionally deferred until the user
has the required loopback cable available. The current USB PD200X microphone
and Focusrite headset arrangement can produce audible output, but it cannot
provide a stable wired reference; repeated acoustic runs were therefore
rejected for insufficient detection or invalid cadence. This is a recorded
prerequisite, not a waiver of NFR-01/NFR-02/QUAL-04. Continue with the next
available non-physical acceptance work.

## Physical-latency loopback cable now available (2026-09-21)

The user connected a wired 1/4" TRS instrument cable between the Focusrite
interface's instrument input and its headphone/speaker output, providing the
NFR-01 required wired mic-to-headphones physical loopback. A read-only
inventory probe confirmed the exact endpoints: render `Speakers (Focusrite
USB Audio)` and capture `Analogue 1 + 2 (Focusrite USB Audio)`. No stream was
opened and no device state changed during inventory.

The guarded `tests/acceptance/m00-native-impulse.ps1 -AllowLiveAudio` run
against that exact wired pair (1,000 impulses, default 10 ms interval) failed
at `render_initialize` with the preserved `AUDCLNT_E_DEVICE_IN_USE`
(`0x8889000A`) diagnostic before any capture/render lifecycle or media-device
state change. Per the existing ownership-diagnostic lesson, this is recorded
as an endpoint-in-use conflict, not a routing, format, or cable failure, and
is not masked as one. The Focusrite `Speakers` render endpoint is currently
held by another process (an existing Voicemeeter hardware-out binding is the
likely candidate given this host's configuration) and must be released before
this endpoint is free for the probe to open.

Blocked on: freeing the Focusrite `Speakers (Focusrite USB Audio)` render
endpoint (check Voicemeeter's hardware-out assignment and any exclusive-mode
application holding that device) before the NFR-01 wired-loopback impulse
acceptance can be retried. Once free, rerun the same command and, if it
passes group detection, extend the analyzer to publish per-impulse
round-trip min/p50/p95/max (the current script only reports spacing-error
and a single onset estimate) before claiming the NFR-01 gate closed.

A follow-up 0-groups run confirmed the WASAPI lifecycle was healthy but no
signal crossed the detector; a physical LED check on the Scarlett Solo's
Instrument gain halo confirmed the wired loop is electrically live once the
dedicated `Instr` input mode is enabled. With `Instr` engaged, the retried
run returned exit code 0 (`detected_groups=8928`, `p95_spacing_error_frames=
471`, `estimated_onset_ms=-156.00`) but this is explicitly rejected as
physical-latency evidence: 8,928 groups for 1,000 impulses, a spacing error
nearly equal to the expected interval, and a physically impossible negative
onset together indicate input-stage clipping producing multiple spurious
threshold crossings per impulse rather than one clean detection per impulse.
This repeats an already-documented rejected-cadence pattern from an earlier
acoustic attempt and must not be promoted to a pass on exit-code alone. After
the user lowered the instrument gain substantially, the same command returned
a clean `detected_groups=1000` with zero spacing-error frames, confirming the
wired path itself is now electrically sound; the reported single onset
estimate is process-launch-tick based and explicitly out of scope for a
calibrated NFR-01 claim (see below).

## NFR-01 calibrated wired loopback measurement implemented (2026-09-21)

Added a new single-process `impulse-loopback` mode to
`tools/m00-native-wasapi-probe/main.cpp` (`impulse_loopback_probe`) plus
`tests/acceptance/m00-native-impulse-loopback.ps1`, replacing the two-process
onset estimate with a calibrated per-impulse round-trip distribution: render
is measured via `IAudioClock` (start/end anchors extrapolated to a common
QPC timeline), capture is measured via `IAudioCaptureClient::GetBuffer`'s own
per-packet device position and QPC timestamp, and each of 1,000 impulses is
paired and converted to a latency sample; the tool publishes min/p50/p95/max
and the negotiated render/capture buffer sizes as NFR-01 requires.

Getting to a trustworthy result required fixing several real bugs found via
live hardware iteration, kept here for a future agent's benefit:
- `IAudioClock::GetFrequency` on this Focusrite driver reports the **byte**
  rate (`nAvgBytesPerSec`), not the frame rate; frame indices must be
  multiplied by `nBlockAlign` before dividing by that frequency.
- The initial `hnsBufferDuration=1000000` (100 ms) copied from this file's
  other diagnostic-only probes adds its own size directly to the measured
  round trip and must not be used for a latency measurement; switched both
  streams to a `0` (minimal engine-period) shared-mode buffer.
- A single-threaded cooperative poll loop alternating between render and
  capture cannot reliably service a ~22 ms buffer; it produced tens of
  thousands of dropped capture frames and corrupted timing. Replaced with
  two event-driven threads (`AUDCLNT_STREAMFLAGS_EVENTCALLBACK` + dedicated
  `WaitForSingleObject` loops), which produced zero dropped frames.
- A `std::cout` precision/format leak in a diagnostic print line caused a
  later value to render in truncated scientific notation, which the
  acceptance wrapper's regex silently mis-parsed as a passing `p95=2` — a
  false pass caught only by cross-checking the raw anchor numbers, not by
  the wrapper's own exit code. Fixed the stream-state leak.

With all of the above fixed, four independent elevated runs against the
user's wired Focusrite `Speakers`→`Analogue 1 + 2` Instrument-mode loopback
all passed group detection (1,000/1,000 pairs, zero dropped capture frames)
and were tightly reproducible (sub-2 ms spread within each run): p95 results
were 206.748 ms, 213.97 ms, and two further runs in the same ~205–214 ms
band. `GetStreamLatency` reported 0 for both streams (queried pre-`Start`,
inconclusive either way).

**NFR-01 result: FAILED on this configuration.** Measured wired physical
loopback p95 ≈ 207–214 ms initially, using WASAPI shared mode (matching
AudioRouter's production sharing model; not exclusive/ASIO mode) on this
Focusrite Scarlett Solo and its current driver. This is a genuine calibrated
measurement, not a parked/blocked gate and not a probe defect: the physical
loopback cable, gain staging, zero-drop capture, and cross-stream QPC
calibration are all independently confirmed sound. The finding itself — that
this shared-mode configuration does not currently meet NFR-01 — is the
actual evidence the gate has been asking for, and must be recorded as a
failing result rather than left parked. No default device, volume, mute,
privacy, or persistent audio configuration was changed by any of these runs.

A same-day follow-up found and fixed a real ~41-45 ms render-clock
calibration bias (a `render-clock-ramp` diagnostic proved this driver's
render position stays at exactly 0 for a genuine engine warm-up window
before advancing at the expected rate) and corrected the result to
**p95 ≈ 185.5-185.6 ms**, reproduced across two full runs. Buffer/engine-
period tuning (including Windows 10+ `IAudioClient3` low-latency shared
mode) was also tested and ruled out as a fix on this hardware — the
device's own low-latency engine-period floor is a fixed 10 ms, and neither
buffer size nor low-latency-mode activation changed the measured round trip.
Full detail, including one unresolved low-latency-path capture-pairing
anomaly, is in
[the wasapi-probe evidence file](evidence/M00-wasapi-probe.md).

## DEC-14: NFR-01 target revised to ≤250 ms p95 (2026-09-21)

User-approved decision, recorded here and in
[15-delivery.md DEC-14](../../spec/15-delivery.md#initial-decision-register)
and [14-quality.md NFR-01](../../spec/14-quality.md#quantitative-requirements):
the original ≤30 ms p95 target was an unvalidated aspirational figure, not
evidence-backed. With buffer size and `IAudioClient3` low-latency shared-mode
tuning both tested and ruled out as a fix on the one reference device
measured (see the evidence-file entries above), meeting ≤30 ms in WASAPI
shared mode is not achievable on this hardware without a different mechanism
entirely (WASAPI exclusive mode, different hardware/driver) that has not
been explored or authorized. Rather than leave the gate silently failing
against an unreachable number, the target is revised to ≤250 ms p95 — about
35% headroom over the worst of three reproducible ~185.5–185.9 ms
measurements on this device.

This is validated on exactly one reference device (Focusrite Scarlett Solo).
It is not yet known whether other supported interfaces perform better or
worse; qualifying additional reference hardware against this target remains
open future work. Exploring WASAPI exclusive mode as a lower-latency option
is explicitly not authorized by this decision and would need its own scope
approval, since exclusive mode would conflict with AudioRouter's multi-app
shared-routing architecture for that endpoint while held.

**NFR-01 gate status: now needs re-verification against the revised ≤250 ms
target.** The 185.5–185.9 ms measurements already on record pass it; a
formal rerun of `tests/acceptance/m00-native-impulse-loopback.ps1` with an
updated `-P95ThresholdMs 250` default is the next concrete step before this
gate can be marked closed in the validation matrix.

QUAL-04 remains unmeasured and should reuse this same calibrated
`impulse-loopback` tool once a target render/capture pair is chosen for
that measurement. NFR-02 has since been measured separately — see DEC-15
below.

## DEC-15: NFR-02 target revised to ≤160 ms p95 (2026-09-21)

User-approved decision, recorded here and in
[15-delivery.md DEC-15](../../spec/15-delivery.md#initial-decision-register)
and [14-quality.md NFR-02](../../spec/14-quality.md#quantitative-requirements),
following the same methodology as DEC-14: the original ≤40 ms p95 target was
an unvalidated aspirational figure. A new `capture-loopback` measurement
(see [the wasapi-probe evidence file](evidence/M00-wasapi-probe.md)) ran the
mic-to-virtual-capture path through the real AudioRouter engine route
(`PhysicalInput → Gain → Recorder → PhysicalOutput`, capturing Focusrite
`Analogue 1 + 2` and rendering to `CABLE Input`) on the same reference
device, with an independent capture client timestamping arrivals at both the
physical mic input and the virtual `CABLE Output` capture endpoint via each
stream's own per-packet QPC timestamp — no render-side anchor or warm-up
bias involved, unlike NFR-01. Four runs measured p95 of 97.521 ms, 102.874
ms, 115.461 ms, and 110.881 ms; each run's own internal spread stayed under
~13 ms, with more variance appearing across separate engine-route launches
than within a single run. The revised target of ≤160 ms p95 gives roughly
35% headroom over the worst observed run (115.461 ms), matching DEC-14's
methodology.

This is validated on exactly one reference device and one microphone/driver
combination. The cross-run variance (97–115 ms) is larger than NFR-01's
(sub-2 ms); whether this narrows with more runs, a different mic, or a
warmed-up (already-running) engine rather than a fresh route launch each
time is open future work, not resolved here.

**NFR-02 gate status: closed against the revised ≤160 ms target.** The
acceptance wrapper `tests/acceptance/m02-nfr02-mic-virtual-capture.ps1` was
updated to default `-P95ThresholdMs 160.0` and reran successfully; see its
result recorded in the evidence file and the validation matrix above.

After the user closed Voicemeeter Banana, the `AUDCLNT_E_DEVICE_IN_USE`
conflict on the Focusrite `Speakers` render endpoint cleared: the retry
opened both endpoints and completed the full capture/render lifecycle
(start/stop/reset all `0x0`, `render_impulse_written=1`). The analyzer then
rejected the run for a different reason: `only 0 impulse groups detected;
expected at least 900`, i.e. no sample on the `Analogue 1 + 2` capture
crossed the 0.05 peak-detection threshold at any point during the 1,000
impulses. This is evidence that the WASAPI stream lifecycle is healthy but no
audible signal is reaching the capture input over the physical wired path;
it is not an ownership, routing, or software conflict. Candidate physical
causes (not yet checked, require the user at the hardware): input gain trim
for the connected instrument-input channel turned down or unset to
instrument mode, output/headphone volume at zero, cable not fully seated in
one of the two jacks, or the `Speakers` WASAPI endpoint being routed to a
different physical output than the jack the cable is plugged into on this
interface model. No media-device state changed and no persistent
configuration was altered by this run.

## Authoritative current-state reconciliation (2026-09-18)

This section supersedes older point-in-time notes above where they describe a
different checkout state. The current Git worktree is clean at commit
`81bd72b0` (`Park physical latency pending loopback cable`). The clean-tree
`tests/acceptance/m08-release.ps1` run passed unsigned optimized Rust/UI
artifact preparation and verification; its temporary outputs were removed.
Documentation validation currently reports 56 Markdown files and 225 local
links, and traceability covers 159 normative requirement IDs.

The remaining blockers are evidence or authority gaps, not uncommitted source:

- The AudioRouter-owned driver, PortCls, production-signing, and clean-machine
  driver track is future work under `docs/plans/future/` and is excluded from
  the active VB-Cable-first completion chain.
- NFR-01/NFR-02/QUAL-04 physical latency remains parked until a wired
  Focusrite output-to-input loopback cable is available. The USB PD200X
  acoustic retries were rejected and are not latency evidence.
- Attended M05 accessibility/scaling/first-run/drag-and-drop and M07 actual
  OS-transition delivery/native reopen remain unverified because the current
  computer-use inventory is `apps: []`, `browsers: []`.
- Plugin redistribution rights/full OS sandboxing and signed installer,
  clean install/upgrade/uninstall, and publication evidence remain open.

Until one of those external prerequisites changes, the next actionable work
is documentation/audit maintenance; the plan must not be archived or marked
complete on the basis of portable or unsigned evidence.

The 2026-09-18 capability recheck also found no `chrome`, `msedge`, or
`firefox` executable and no repository Playwright installation; the
computer-use inventory remained `apps: []`, `browsers: []`. A browser-level
test therefore cannot be started on this host, and even an available browser
would cover only the WebView/page surface—not Narrator, the native Tauri
window/tray, or actual Windows lock/sign-out/sleep transitions.

The subsequent full locked workspace regression passed with 36 CLI, 181
control tests plus four guarded-live tests ignored, 66 domain, 34 DSP, 118
engine, 70 plugin-host, 13 worker-process, 8 protocol, 40 recording, 92
storage, 19 transport, 88 Windows-audio tests, and all doc-tests. This
refreshes portable regression evidence only; the guarded-live tests remain
explicitly opt-in and do not close the physical, attended, or OS-transition
gates.

Strict static verification was rerun afterward and passed: `cargo clippy
--workspace --locked --all-targets --all-features -- -D warnings` and
`cargo fmt --all -- --check`. No source or generated artifact changes were
left by either check.

The native AudioRouter shell was subsequently exposed to the Windows
computer-use surface and an attended M05 check was performed without starting
audio. The initial shell showed backend ready, a stopped revision-0 session,
safe startup, monitoring muted/unarmed, labeled session/canvas/inspector
controls, and deferred virtual-device controls visibly disabled. The user
clicked `+ Test Signal`; the node appeared on the canvas and in the inspector
with bounded frequency, level, and duration controls. No plan was committed
and the session remained stopped. The draft was then discarded and the
original graph was confirmed restored. A maximized-window screenshot also
showed the session list, canvas, shelf, inspector, and draft controls in one
responsive layout. This is attended native-shell evidence for the click path
and safe first-run state; Narrator speech, Windows display-scaling settings,
and live drag-and-drop remain unverified.

The elevated `tests/acceptance/safe-all.ps1` chain was rerun after this
reconciliation and passed end to end on the clean tree. It covered M00
toolchain/native compile and 34-endpoint format inventory, M01 CLI, existing-
device M03 routing, M04 DSP/recording, M05 typecheck/19 UI files/260 tests and
temporary production build, M06 SDK/validator/VST3 worker/VST2 fixture gates,
M07 headless and frontend-owned shell RPC, unsigned M08 artifact preparation
and NSIS smoke, traceability, and documentation. The run reported 1,598
plugin-validator tests passed, 181 control tests with four guarded-live tests
ignored, a 4,895,384-byte disposable unsigned installer, 159 normative IDs,
and 56 Markdown files/225 local links. Run-owned temporary children and
installer output were removed. This refresh does not close the explicitly
listed attended, physical-latency, OS-transition, redistribution/sandbox,
signing, installation, or publication gates.

## Native drag-and-drop defect follow-up (2026-09-18)

The user reproduced that dragging a library control does not add it to the
canvas in the attended shell. The UI drop path was hardened for WebView/native
drag behavior: the shelf now supplies both the AudioRouter MIME type and a
plain-text fallback, while the editable canvas accepts drag-over events even
when the host exposes no drag MIME types and validates the payload at drop
time. A focused regression covers the no-types/plain-text case. UI typecheck,
261 Vitest tests, the production UI build, and `tests/acceptance/m05-ui.ps1`
all pass. Native post-fix drag verification remains open because the
computer-use RPC surface became unavailable after the rebuilt shell was
launched; do not count this change as attended drag evidence until the user or
an available native surface confirms a node appears after dragging.

The 2026-09-18 UI redesign slice, later archived as
[the completed UI redesign plan](../archived/2026-09-19-ui-redesign.md), was
built in commit `d3b2dbdf`. It adds an
Ambient-inspired shared lighting treatment, Audio Hijack-inspired compact
source/processor/output cards, inline bounded meters, an interactive
eight-band EQ preview, and telemetry-driven active edge animation. The UI
suite now passes 263 tests, typecheck, production build, and M05 acceptance.
Native-shell visual inspection at 1280×720/maximized remains open because the
attended computer-use surface is unavailable; do not treat the automated build
as visual acceptance.

The subsequent 2026-09-18 usability pass addressed four user-reported M05
defects: initial canvas fit after draft hydration, physical-input endpoint
selection in the node inspector, recorder format/approved-path visibility, and
inspector width/control layout. The UI typecheck, 263 tests, production build,
and M05 acceptance passed again. Native visual confirmation remains open.

## 2026-09-19 connection-handle fixes and UI redesign plan archival

The UI redesign sub-plan reached the end of its own implementation scope and
is archived at
[docs/plans/archived/2026-09-19-ui-redesign.md](../archived/2026-09-19-ui-redesign.md);
its unique remaining item (attended native shell inspection) duplicated what
this file already tracks below, so it is not repeated in two places.

Three defects were found and fixed in the connection-handle/canvas area while
reviewing whether any four-sided connector combination (e.g. input bottom to
output top) actually works end to end:

1. The connection target side was decoded twice — once correctly in
   `onConnect`, then again in `onConnectEnd` from an already-stripped handle
   id that had lost the side suffix — so every connection rendered as if
   dropped on the input's left side regardless of where it was actually
   released. Fixed by preserving the raw target handle id through to
   `onConnectEnd`.
2. A node with exactly one input and one output port placed both ports' four
   side handles at the same offset per side, so the later-rendered output
   handle always won hit testing and that node's input handle was
   ungrabbable on any side. Fixed by slotting every port's offset against the
   node's full port list instead of per direction.
3. The shared `actionMessage` status banner was silently clobbered: a
   `virtualBridge.expired` or `devices.bindingInvalidated` safety notice
   (explaining that audio had stopped) was immediately overwritten by the
   same automatic resync path's unconditional device-list refresh message.
   This was caught by two tests in `App.accessibility.test.tsx` that had been
   failing on `main` before this session (unrelated to the handle work).
   Fixed by making automatic/background device refreshes silent while
   keeping the manual "Refresh available inputs/outputs" button announced.

All three fixes, plus a handle-decluttering pass (dimmed idle handles,
brighten on hover, fade non-matching handles during an active drag), are in
`ui/src/SessionFlowCanvas.tsx`, `ui/src/App.tsx`, and `ui/src/styles.css`.
`npm run typecheck` is clean and the full UI suite now passes 262/262 (the 2
pre-existing failures above are fixed; no new failures introduced).

A new no-backend harness (`ui/harness.html` + `ui/src/devHarness.tsx`) was
added and is retained as a standing dev tool. It mounts `SessionFlowCanvas`
directly with a local demo session and `canEdit` forced on, which let the
target-side and handle-overlap fixes be verified with real mouse drags in
Chrome instead of only by code inspection. This does not substitute for
attended native-shell evidence: the harness proves the React Flow canvas
logic works in a browser, not that the packaged Tauri shell, Narrator,
Windows scaling, or the native WebView drag path behave the same way. Those
gates remain exactly as open as recorded in the audit boundary below; this
session closes a portable UI-logic and pre-existing-test gap, not an attended
one.

A follow-up pass gave each draggable library node kind its own purposeful
inline visual instead of a generic Ready/Bypassed/Disabled chip, closing a gap
against the UI redesign plan's own design decision that "every graph node has
... one useful visual." Gain, Delay, and Pitch shift now render a compact
draggable/keyboard-adjustable fader routed through the existing draft
parameter path. Mute renders a one-tap Live/Muted toggle. Compressor and
Limiter render a gain-reduction meter driven by the backend's existing
`nodeTelemetry[].processor.gainReductionDb` field, which was already defined
in the contract but unused anywhere in the UI. Gate renders an Open/Closed
indicator from the equally unused `processor.gateOpen` field. Mixer gets a
distinct sum-glyph visual, and Test Signal now shows its configured
frequency/level alongside its existing meter. `npm run typecheck` is clean
and the full suite still passes 262/262. All eight new visuals were verified
rendering and interactive (fader drag, mute toggle click) with real mouse
input against the no-backend harness above, seeded with synthetic telemetry
for the compressor/gate/limiter cases since no native backend is available
here to produce real gain-reduction/gate readings.

A follow-up audit asking "do they all have a useful, appealing node" found
two library nodes that still fell short: Mixer showed a static "Sums
connected inputs" label with no relationship to the actual graph, and
Recorder showed only a generic pass-through meter with no indication of
armed/recording/paused state — arguably the single most important fact about
a recorder node. Mixer now counts real incoming edges
(`session.edges.filter((edge) => edge.destinationNode === node.id).length`)
and shows "Summing N inputs" (or "No inputs connected"), plus its own meter
when the backend reports one. Recorder now looks up its `RecorderStatus` by
`nodeId` (plumbed as a new optional `recorderStatuses` prop from `App.tsx`'s
existing `recorderStatuses` state, not previously passed to the canvas) and
shows a labeled, colored state chip (Idle/Armed/Recording/Paused/Stopping/
Completed/Failed) with a pulsing dot while recording, alongside its meter.
Verified with the harness: seeded two edges into a Mixer node ("Summing 2
inputs" rendered correctly) and one `recording`-state `RecorderStatus`
("Recording" chip with red dot rendered correctly). Typecheck clean, full
suite still 262/262.

The remaining node kinds that are not part of the drag library itself
(`applicationCapture`, `endpointLoopback`, `plugin`, and the deferred
`virtualRenderSource`/`virtualCaptureSink`) were initially left at the
generic Ready/Bypassed/Disabled visual since they are not draggable library
entries — application capture and endpoint loopback nodes are created from
dedicated panels elsewhere in the UI, plugin nodes from the plugin-scan flow,
and the two virtual-bus kinds are disabled in the library pending the
deferred managed driver. The user asked for full coverage regardless, so all
five now have dedicated visuals too: Application Capture shows the captured
executable and instance-selection policy; Endpoint Loopback shows a
truncated bound endpoint ID; the two virtual-bus kinds show their bound bus
identity with a direction glyph; and Plugin shows a VST2/VST3 format badge,
the bound binary's filename, and an active/stopped/bypass state chip reusing
the existing `.node-state` styling. Every one falls back to its meter when
the backend reports telemetry for it. Every node kind in the domain now has
a purpose-built visual; only the fully generic fallback for any future
unknown kind remains. Verified in the harness with one of each kind
(including a seeded `voice-bus` virtual capture sink and a `ReaComp.vst3`
VST3 plugin); typecheck clean, full suite still 262/262.

The user then asked for the canvas to look as good as Audio Hijack and for
the app to at least match VoiceMeeter's core "redirect input, tool, and
output" workflow. The drag shelf (`.canvas-library`) was a flat,
alphabetized button list with no visual distinction between a source, a
processor, and a destination, which worked against both asks at once. Every
`LibraryEntry` in `ui/src/library.ts` now carries an explicit `flow: "input"
| "tool" | "output"` field, and the shelf renders three labeled, colored
groups (Inputs/cyan, Tools/amber, Outputs/green) with a colored monogram
icon per entry. The same three-way classification (`nodeFlowGroup()` in
`SessionFlowCanvas.tsx`, covering the full `NodeKind` enum) now sets a
matching colored left-edge accent on every node card via a new `className`
on the React Flow node object, so the canvas itself reads left-to-right as a
signal path the way Audio Hijack's board does, not just the palette next to
it. Verified in the harness: the shelf renders correctly grouped with
correct colors; clicking "Test Signal" (Inputs) and "Physical output"
(Outputs) both correctly added nodes (confirmed via
`onAddLibraryNode testSignal testSignal-1` in console output, and the new
node appearing after Tidy layout); node cards show the correct cyan/amber/
green left accent per group in zoomed screenshots. Connection-dragging logic
itself was not touched and was not re-verified in this pass since it was
already extensively proven working in prior sessions. Typecheck clean, full
suite still 262/262.

This closes the shelf/board half of "compete with VoiceMeeter Banana and
Audio Hijack" — it does not close the larger, unscoped ask. VoiceMeeter's
actual differentiator is flexible virtual-bus strip routing, which this app
already supports end-to-end through existing VB-Cable/Voicemeeter endpoints
per the VB-Cable-first evidence above; AudioRouter-owned virtual bus
provisioning remains the deferred driver-signing gate, unchanged by this UI
work. Audio Hijack has an entire feature surface (saved snapshots, a
hardware-style routing matrix, global hotkeys) this pass did not attempt.
Treat "make it look amazing" as addressed for the node/shelf visual layer;
a broader feature-parity gap analysis remains a separate, larger initiative
if wanted.

The user then asked, reasonably, whether they could avoid the signed-driver
requirement entirely by routing through their own existing VoiceMeeter
Banana install. The answer is yes, and this was already the supported
VB-Cable-first path documented and qualified throughout this file — the
signed-driver gate only applies to AudioRouter provisioning its *own new*
virtual endpoint (the deferred `virtualRenderSource`/`virtualCaptureSink`
kinds), never to binding a plain `physicalInput`/`physicalOutput` node to an
endpoint VoiceMeeter or VB-Cable already registered with Windows. That
distinction was true but not obvious in the product itself: the shelf had an
"Existing virtual output" entry for exactly this free path, but no input-side
equivalent, and neither the available entries nor the deferred ones said in
plain language that the free path existed. Library search for "voicemeeter"
also returned nothing.

Fixed: added a symmetric "Existing virtual input" library entry
(`kind: physicalInput`, same free binding, no driver) next to the existing
output one; both now carry a `note` field ("Binds to an already-installed
virtual endpoint such as VoiceMeeter or VB-Cable ... no AudioRouter driver
required") shown as a tooltip and matched by library search in both the
canvas shelf and the advanced Library panel; and the two deferred entries'
`unavailableReason` now explicitly says "For VoiceMeeter or VB-Cable today,
use Existing virtual input/output instead" at the exact point of confusion.
Searching "voicemeeter" in the library now surfaces all four related entries.
A new `library.test.ts` case pins this behavior. Typecheck clean, full suite
263/263 (was 262; +1 new test, and one existing assertion updated to match
the improved message text via `toContain` instead of exact match).

## 2026-09-19 permanent scope decision: no AudioRouter-owned driver

The user made an explicit, final scope decision: AudioRouter-owned virtual
endpoints (the managed driver, PortCls integration, and production signing)
are set aside indefinitely for cost reasons — a driver-signing credential is
not funded and none is planned. This is not a re-statement of the existing
"deferred pending signing" language; it changes that language from temporary
to permanent. Updated to match:
[`docs/plans/future/M03-driver-signing.md`](../future/M03-driver-signing.md),
[`docs/spec/06-virtual-devices.md`](../../spec/06-virtual-devices.md), and
[`docs/spec/15-delivery.md`](../../spec/15-delivery.md). VDEV-01/03/09 and
SEC-08 remain textually normative only for a possible future funded signed
track and are explicitly excluded from this project's v1 completion target.

The user's instruction was: complete "everything else" using the existing
VoiceMeeter/VB-Cable endpoint strategy (already the supported path, and the
subject of the prior session's library fix). Given the sheer size of this
codebase and the extensive completion claims already recorded throughout
this file, the next step before writing any more code is an evidence-based
audit — not blind trust in this log's own prior "implemented" claims — to
find any feature described in `docs/spec/*.md` or `docs/milestones/*.md`
that is not actually present in `crates/` or `ui/src/`, excluding: attended
Windows UI verification, physical/acoustic hardware measurement, the
now-permanently-set-aside driver/signing work, and legal/licensing gates
(plugin redistribution rights, OS sandboxing certification) that no amount
of code can resolve. That audit and its resulting task list are recorded in
the next entry below.

## 2026-09-19 audit result and two implemented backend-complete-but-UI-missing gaps

The audit found the Rust workspace genuinely matches this file's completion
claims: zero `todo!()`/`unimplemented!()`/TODO/FIXME hits anywhere in
`crates/` or `ui/src/` outside test fixtures, and every one of the 86
dispatched API methods in `crates/domain/src/lib.rs`'s catalog has real
dispatch logic in `crates/control/src/lib.rs`, not a stub. M04/M06/M07
completion claims were spot-checked directly against `crates/dsp`,
`crates/recording`, `crates/plugin-host`, and `crates/control` source and
hold up.

It found exactly two genuine gaps, both backend-complete with zero UI
surface: **committed graph history/undo** (`graph.history`, `graph.undoPlan`
— the React app only had local pre-commit draft undo, no way to view or
revert an already-committed revision) and **client authorization management**
(`clients.list`, `clients.authorize`, `clients.revoke` — no way to see or
revoke a connected CLI/MCP client from the app, which matters directly for
SEC-01–06 multi-client scoping). Both are now implemented: `backend.ts`
gained `listGraphHistory`/`undoGraphPlan`/`listClients`/`authorizeClient`/
`revokeClient`, wired to the existing live-client `request()` pattern and to
safe disconnected-backend defaults. `App.tsx` gained a `GraphHistoryPanel`
(lists committed revisions; "Undo last committed change" reuses the existing
`graph.plan`→`graph.commit` flow, since `graph.undoPlan` always targets the
single revision immediately before the current one, confirmed from
`domain::Store::undo_plan`'s `.rev().nth(1)`, not an arbitrary past
revision) and a `ClientsPanel` (list/authorize/revoke). A genuine, separate
contract-drift bug was found and fixed in the same pass: `MethodParams`
for `clients.authorize`/`clients.revoke` in `contracts/src/index.ts` were
missing the `idempotencyKey` field that `crates/control/src/lib.rs`'s
dispatch functions actually require — the drift checker only validates
method-name parity, not per-field param shape, so this had never been
caught. Five new focused tests added to `App.accessibility.test.tsx`.
Typecheck clean, full suite 268/268 (was 263), contract drift still passes
(86 methods, 20 node kinds, 7 processors, 20 event categories).

**A far more significant finding surfaced while visually verifying the two
new panels in a real browser (not jsdom, which never applies real CSS):
they rendered but were completely invisible, `display: none`.** Tracing why
found that an entire class of already-implemented, already-tested panels
was silently unreachable in the shipped UI, not just the two new ones:
`ProcessorCatalog`, `PresetCatalog`, `SessionTransferPanel`,
`PluginScanPanel`, `StartupPanel`, `OsTransitionPanel` were hidden by an
unconditional `.main-content > .panel:not(.inspector) { display: none; }`
rule in `styles.css` with no escape hatch at all — and separately,
`RecorderActions` (the entire M04 recorder create/arm/start/pause/split/stop
UI), `RecordingActions`, `VirtualDeviceLifecyclePanel` (the M03 managed-bus
UI), and `VirtualRoutePanel` were gated behind an `.app-shell.compact-status`
class that could only be toggled by a "Compact status" button — which was
*itself* hidden by a separate rule (`.topbar .status-cluster > .secondary
{ display: none; }`). The theme picker (`.theme-picker`, the only way to
satisfy UI-11's dark/light/high-contrast requirement) was hidden by the same
rule. `grep`-ing `App.tsx` confirmed `setCompactStatus` and `setTheme` had
no other call site — no keyboard shortcut, no alternate control. **The
result: theme switching and this entire set of already-built, already-tested
features were completely unreachable by any real user, through any path, in
the shipped UI**, despite this file's own extensive log claiming them
complete — because none of the automated tests check real CSS visibility, so
nothing caught it. This is almost certainly an unfinished half of the
2026-09-18 UI simplification pass, which intended (per its own `styles.css`
comment) that these controls "remain available through... list view... and
selected-node inspector" — a claim that was already inaccurate for most of
them — and left an unused `.advanced-tools`/`<summary>` disclosure pattern in
the CSS that was never wired into `App.tsx` to reveal them again.

## 2026-09-20 full stack code review

The user asked for a full code review, backend then frontend. The `code-review`
skill is diff/PR-oriented and found nothing to review (the working tree was
clean and the last commit didn't touch `crates/`), so this was done manually:
parallel review forks for the backend crates (`domain`+`protocol` and
`control` succeeded cleanly; the `engine`+`dsp` fork got derailed by its own
inherited context — it spent its entire budget reasoning about the earlier
rate-limit failures instead of reviewing anything, a real risk worth knowing
about when forking after a notification-heavy stretch of conversation), then
direct grep-first review of the remaining crates and the full `ui/src` tree
by the top-level session once fork reliability became suspect.

**Backend (~77k lines across 11 crates): no confirmed correctness or security
bugs.** Verified, not just inspected: the named-pipe transport's same-user
claim (real owner-only ACL plus an explicit SID comparison before any request
is read, no TOCTOU, non-Windows stub fails closed); plugin-host's identity
revalidation (every real `control` call site uses the `*_verified_*` spawn
path, never the unverified one); storage's ZIP bundle import (correct
component-based `..` detection, not string matching, plus symlink/reparse
rejection); and engine's `RealtimeDsp` lock-free gate (correct Acquire/Release
ordering with a drop-guard, and fail-closed contention handling applied
consistently everywhere traced). One minor, non-bug duplication note in
`protocol/lib.rs`'s two near-identical frame-decode functions.

**Frontend (~8,200 lines): one confirmed bug, now fixed.**
`ui/src/draft.ts`'s `appendDraftConnection` built its default channel matrix
with `Math.min(destinationChannel, sourcePort.channels - 1)`, which correctly
fans a mono source out to a stereo destination but, for the reverse case,
only ever gave the source's first (left) channel any gain — connecting a
stereo output into a mono input silently dropped the right channel entirely,
with no downmix and no warning. Every other frontend file was read in full
(all small utility/panel files, `host.ts`, `graphView.ts`, `layout.ts`,
`processorCatalog.ts`) and found clean; `App.tsx`/`backend.ts`/
`SessionFlowCanvas.tsx` were not re-read fresh line-by-line since they were
extensively edited and live-browser-verified earlier this same session.

Fixed by extracting a `defaultChannelMatrix(sourceChannels, destinationChannels)`
helper in `draft.ts`: when the source has more channels than the destination,
every destination channel now averages all source channels (gain
`1/sourceChannels` each) instead of keeping only the first. The mono→stereo
and equal-channel-count cases are unchanged (still identity/duplication).
One pre-existing test (`draft-connection.test.ts`) had pinned the buggy
`[1, 0]` default for a stereo→mono case inside an `insertDraftMixer` round
trip; corrected it to the now-correct `[0.5, 0.5]`, and added a direct
`appendDraftConnection` integration test plus four `defaultChannelMatrix`
unit tests covering all four 1/2-channel combinations. Typecheck clean, full
suite 273/273 (was 268; +5 new, 1 corrected).

## 2026-09-20 VST plugin hosting completion, archived

The VST plugin hosting sub-plan (shelf-reachable add/remove/manage, canvas
VST-vs-native visual identity, mixed physical/application multi-input mixing,
isolated-worker health visibility, and edge quick-insert) reached the end of
its own implementation scope and is archived at
[docs/plans/archived/2026-09-20-vst-plugin-hosting.md](../archived/2026-09-20-vst-plugin-hosting.md),
including a dispatch-blocking `validate_method_params` allow-list bug found
and fixed during a full post-implementation review. No attended Windows
evidence was produced or claimed; every check was portable (`cargo test
--workspace`, clippy, the UI suite, typecheck, the contract drift checker,
and the docs validator), so this closes a portable-completeness gap only, not
any attended gate tracked elsewhere in this file. Final counts: control 179
tests (was 177), engine 119 (was 118), plugin-host 71 (was 70), full UI suite
282/282 (was 273).

## 2026-09-20 closed one self-identified gap: multi-input worker plugin/processor telemetry

Follow-up to the archived VST plugin hosting plan above. Asked to scope what
was independently achievable without the deferred driver or attended UI
verification, an audit of the active plan's own tracked non-driver gaps
found most genuinely require hardware, physical measurement, legal review,
or a clean release worktree — none of that is something this session can
do. One gap was a real, portable, code-only completeness item the archived
plan's own "known limits" section had just flagged: the separate multi-input
mixer worker never reported any node telemetry at all (unlike the default
single endpoint/duplex worker, which already reports meter/processor/plugin
telemetry), because `CompiledMixerFanoutGraph` had no telemetry accessors —
only the endpoint-worker's `RuntimeGraph` did.

Fixed by adding `processor_telemetry_for_node`/`plugin_health_for_node` to
`CompiledMixerFanoutGraph` (delegating to its existing `processing_graph:
Option<RuntimeGraph>`, the shared post-mixer chain) and mirrored wrapper
methods on `RealtimeMixerFanout` and `NativeMultiInputWorker`
(`crates/engine`, `crates/windows-audio`). `crates/control` gained a
`native_multi_input_node_telemetry()` mirroring the existing
`native_node_telemetry()`, merged into the same `nodeTelemetry` diagnostics
array (the two native worker types are mutually exclusive per session, so
concatenation is safe and one side is always empty). No contract or UI
change was needed: the UI already looks up telemetry generically by node ID
into one shared array, so a plugin running inside a multi-input session's
post-mixer chain now shows worker health in the canvas, inspector, and
"loaded plugins" list exactly like the default single-chain path, with zero
UI code changes.

Scope explicitly not attempted: adding meter instrumentation for the mixer's
input sources, its own combined signal, or its output branches. None of that
existed before either, and building it is a separate, larger
realtime-instrumentation task (new lock-free meter state threaded through
`RealtimeMixerFanout::pump_inputs`/`process`), not a small extension of what
already existed for the endpoint-worker path the way processor/plugin
telemetry was.

Everything else on the active plan's non-driver list — attended
accessibility/scaling/drag-drop verification, real OS-transition delivery
(lock/sign-out/sleep/resume), physical latency/hardware calibration, VST
redistribution rights and a broader third-party compatibility matrix, and
release artifact preparation (blocked by the current dirty worktree) —
was assessed and is *not* something this session can close alone: each
needs either the user's interactive machine, real hardware, or a legal/rights
decision. The existing Job-Object-based worker sandbox (kill-on-close,
single-process limit, 512 MB memory cap) was also reviewed; it already
provides real resource/lifecycle containment, but true filesystem/network
sandboxing (e.g. AppContainer or a restricted token) is a separate,
security-sensitive design task, not something to bolt on quickly, and was
correctly left out of this pass rather than attempted partially.

Verification: `cargo test --workspace --lib` all green (engine 120, was 119;
control 179 unchanged; every other crate unchanged), `cargo clippy
--workspace --all-targets` zero warnings, `tools/contracts/check-drift.mjs`
clean, UI typecheck clean, full UI suite 282/282 unchanged. Nothing
committed; awaiting the user's go-ahead to push.

## 2026-09-21 goal kickoff: MP3, installed-plugin scan, and M08

The user authorized completion of all three remaining workstreams, beginning
with the easiest bounded investigation. The installed-plugin gap was checked
before any mutation: `C:\Program Files\VSTPlugins\Reaication` is absent, while
`C:\Program Files\VSTPlugins\ReaPlugs` exists with nine DLLs. The existing
read-only installed-VST2 acceptance passed for
`reaeq-standalone.dll` at 44.1/48/96 kHz and both editor-containment checks,
with its SHA-256 unchanged. The scanner/UI root and classification path still
need tracing; no conclusion that the directory scan is correct or defective
has been made. Next: reproduce the scan through the shared API, then implement
the smallest evidence-backed fix or document the precise unsupported boundary.

The scan reproduction is now complete: the actual ReaPlugs root returns all
nine DLLs as x64 VST2 `supportedVst2X64Gated` entries with no errors. The
reported Reaication gap is a path/name mismatch, not a scanner defect; no
code change is warranted.

MP3 implementation progress: added the bundled `mp3lame-encoder` Windows-safe
worker path, bounded queue drain/finalization, non-overwriting `.mp3` path
creation, finalized library metadata, preview structural validation, CLI and
JSON schema format values, and UI format selectors. MP3 is fixed 192 kbps,
supports mono/stereo 44.1/48 kHz, disables dither, and intentionally rejects
split until a gapless part policy is specified. Recording and control tests
cover encoder output, queue frame boundaries, structural inspection, factory
selection, and metadata. The bundled `mp3lame-encoder` dependency reports
LGPL-3.0; release artifacts must include the corresponding notices/source-
offer compliance before publication.

M08 verification on 2026-09-21: documentation validation passed for 58
Markdown files and 264 local links; traceability covered 159 normative IDs;
contract drift passed; CLI tests passed (36), control tests passed (180),
recording tests passed (43), UI typecheck passed, and the UI suite passed
(282). Unsigned installer smoke is blocked by npm registry access/permission
(`@tauri-apps/cli` fetch returned `EACCES`), so no installer or upgrade/
uninstall evidence is claimed. Release artifact preparation is also blocked by
its intentional clean-working-tree guard; the user explicitly requested that
these uncommitted changes remain in place. Signing, clean install/upgrade/
uninstall, hardware/application matrix, and publication remain M08 gates.

## 2026-09-20 ran the portable safe acceptance chain; found and fixed a real `cargo fmt` gap

Asked to keep doing whatever was autonomously possible, ran the project's own
documented "safe" acceptance scripts (native compile, VST3 SDK build +
validator + offline loader, native VST3 worker, VST2 chunk-state fixture,
CLI, headless, M08 traceability, docs) — the exact set the repository
defines as running with no live audio, no driver/installer changes, and no
machine configuration changes, so it was safe to run unattended. All of it
had never been run this session; `cargo test`/`vitest`/clippy/drift-check
are not the same invocation path as these wrapper scripts (they also cover
the native C++ VST3 SDK/CMake build, which cargo never touches).

Result: everything passed except `m04-dsp-recording.ps1`, which failed on
its embedded `cargo fmt --check` step — five files from this session's Rust
edits (`crates/cli`, `crates/control`, `crates/engine`, `crates/plugin-host`)
had never been run through `cargo fmt`. This was a real, if low-severity,
gap: nothing was behaviorally wrong, but the tree did not actually pass the
project's own formatting gate. Fixed by running `cargo fmt` (mechanical,
zero behavior change) and confirming `cargo fmt --check` passes; re-ran the
full workspace test suite, clippy, the drift checker, and the specific
`m04-dsp-recording.ps1` script afterward, all clean.

Everything else confirmed passing and unaffected by this session's work:
M00 native probe compile, M00 native format inventory (34 endpoints), M01
CLI, M06 VST3 SDK (pinned checkout, build, validator — 1,598 + 94 tests,
offline loader, AGain main/auxiliary-bus classes, five-class mda matrix),
M06 native VST3 worker (isolated single-stream/auxiliary-bus processing,
restart/quarantine recovery, state restoration, shutdown), M06 VST2
chunk-state/legacy-main fixture at 44.1/48/96 kHz, M07 headless (CLI 36,
control 179, plugin-host 71, worker-process 13), M08 traceability (159
normative requirement IDs covered), and documentation validation.

This closes out the "do what you can" pass: every other open item on the
active plan's non-driver list genuinely requires the user's interactive
machine, real hardware, or a rights/legal decision, and was not attempted.
Nothing committed; awaiting the user's go-ahead to push.

## 2026-09-22 pushed MP3 changes; mic route requalified after reconnect

Committed and pushed the current MP3 recording and acceptance work as
`08c6b982` (`Add MP3 recording and guarded microphone acceptance`). The
installed-plugin read-only check had found the expected ReaPlugs directory;
the live scenario was then attempted with the guarded NFR-02 wrapper.

The first attempt ran from a non-elevated shell and stopped before opening
audio because Windows denied the required before-run PnP media snapshot. A
retry with elevated execution access passed the snapshot and ran the bounded
AudioRouter `adapter-control-route` for 8 seconds: 801 packets, 384,480
captured frames, 3,003 processed quanta, 384,384 rendered frames, and a
finalized 1,537,580-byte temporary recording. The native probe submitted
314,016 render frames and emitted 654 impulses, but detected zero impulse
groups on either capture stream (`pairs=0/500`), so the wrapper correctly
failed without reporting latency. The user reported that the loopback was
disconnected during this run; treat this as a known setup-related,
non-qualifying attempt, not an NFR-02 regression. Temporary route/probe
artifacts were cleaned by the wrapper. Reconnect the loopback before repeating
this live scenario.

The explicit installed ReaEQ worker acceptance passed at 44.1, 48, and 96
kHz, plus both editor-containment checks. The selected DLL's SHA-256 remained
`c200e540c26ac793b43611aaceb4aa42cdd2829cdfbf0d60494716d9bdde8a7d`.
This confirms the isolated plugin worker separately; it does not prove a
single live microphone graph with ReaEQ inserted.

After the user reconnected the loopback, the same guarded wrapper passed:
`pairs=500/500`, p95 117.403 ms (min 116.139, p50 116.848, max 128.688 ms),
within the 160 ms target. It detected 627 physical-mic and 614 virtual-capture
impulse groups, with zero dropped frames. The internal route reported 800
packets, 384,000 captured and rendered frames, 3,000 processed quanta, and
1,536,044 finalized recording bytes. Its one `rejected_pumps` count is the
acceptance's deliberate stale-generation rejection check before normal
generation pumping, not an audio-path failure. This requalifies the real
microphone-to-VB-Cable route on the reference setup.

The combined live graph check is completed below. Attended UI interaction and
receiving-application listening remain unverified because the computer-use
surface exposed no apps; the earlier idle shell was closed and its temporary
database cleaned without starting audio. The unsigned installer smoke remains
an M08 packaging gate and is not a prerequisite for MP3 recording behavior.

## 2026-09-22 M02/M06 end-to-end installed-plugin workflow

Objective: make installed supported plugins usable end to end: explicitly
scan/select an exact plugin, add/insert it as a graph node, inspect/edit its
parameters in the AudioRouter UI, bind it to the isolated worker, and prove
that a non-default plugin setting changes signal in a real microphone route.
The native vendor editor window is a separate capability; generic parameter
editing is the required configuration path for this task unless existing
contracts reveal a safe already-supported editor route.

Requirements: CAP-02/03, GRAPH-01/06/14, DSP-01, PLUG-01/03/05/07,
SEC-07, NFR-02, QUAL-01/02/03, ENG-04. This is a single reference-machine
integration experiment; it does not qualify all plugins or close the broader
M06 rights, sandbox, W2 timing, or compatibility gates.

Prerequisites: Windows 11 x64, elevated PowerShell for the existing before/
after media inventory, the connected reference microphone loopback, active
Focusrite capture and VB-Cable render/capture endpoints, the existing isolated
plugin worker, and the exact local `reaeq-standalone.dll` whose recorded hash
is `c200e540c26ac793b43611aaceb4aa42cdd2829cdfbf0d60494716d9bdde8a7d`.
The user explicitly authorized live microphone routing. Do not change Windows
defaults, endpoint volume/mute, privacy, driver, or startup state.

Decision: keep the existing non-plugin acceptance behavior unchanged. Use the
existing explicit local scan, draft-node insertion, generic parameter panel,
verified worker binding, and graph processing boundaries; repair only
evidence-backed gaps. VST2 metadata does not publish VST3-style class IDs, so
the graph's existing `default` placeholder token is not plugin identity:
binary path plus the freshly rescanned SHA-256 remain authoritative for VST2
worker authorization. Do not synthesize or present a plugin-reported class ID.
The plugin remains user-installed and is never copied or modified. The guarded
live proof must set a real non-default parameter; a controlled same-input
real-DLL worker regression must demonstrate a processed-signal difference.
Both require worker health and exact binary identity evidence.

Ordered tasks and outcome:

1. Trace and test scanner → UI picker → draft insertion/splice → parameter
   descriptors/editor → commit → runtime worker binding and parameter events.
   Done for the exercised path: VST2 placeholder insertion accepts an empty
   static class-ID list via the same neutral `default` authoring token already
   used by the UI; the scanned canonical binary path and SHA-256 remain the
   actual worker identity. The focused UI test now selects this entry, inserts
   it, loads worker-described generic controls, and changes `1-Gain` in draft.
2. Complete the bounded `adapter-control-vst2-route` live route. Its chain is
   microphone capture → gain → enabled ReaEQ worker → recorder/output branch;
   support a selected non-default plugin parameter. Done; the exact ReaEQ
   route binds parameter ID 1 (`1-Gain`) at normalized 0.75. A real-DLL worker
   regression confirms that changing an exposed `*-Gain` value changes
   processed samples for the same input.
3. Extend the guarded NFR-02 PowerShell wrapper with explicit `-PluginPath`;
   retain exact endpoints, media snapshots, cleanup, latency checks, binary
   fingerprint checks, worker health, and optional described parameter input.
   Done; the route uses the exact scanned identity and checks worker health,
   fingerprint, parameter acknowledgement, paired output, and the NFR-02 gate.
4. Verification completed for this bounded slice: locked probe build, focused
   UI test/typecheck, control and plugin-host suites, opt-in real ReaEQ worker
   parameter-difference test, and guarded live runs with default/non-default
   ReaEQ and ReaComp. Exact results are in [M00 probe evidence](evidence/M00-wasapi-probe.md)
   and the [plugin compatibility snapshot](../../operations/plugin-compatibility.md).

Validation matrix: scanner and UI identify exact supported VST2/VST3 format,
draft insertion keeps identity/path/fingerprint, parameter changes traverse
the backend to the isolated worker, runtime telemetry stays healthy, and the
graph remains fail-closed. Probe builds with `--locked`; no-plugin M02
acceptance stays unchanged; plugin mode rejects unsupported/non-x64/changed
binaries. Live run requires all 500 signal pairs, p95 ≤160 ms,
`plugin_worker_state=running`, zero worker failures, and unchanged DLL/media
identity; the opt-in worker regression must show finite output that differs
for a non-default parameter.
Any failed guard yields no pass claim.

Result: the initial plugin route at a two-slot queue depth was healthy but lost
enough correlated output for only 405/500 pairs. Increasing the already-bounded
plugin handoff to its existing eight-slot maximum recovered the signal path.
The final non-default `1-Gain=0.75` live route passed 500/500 pairs at
139.115 ms p95 (160 ms threshold), with the verified ReaEQ worker running at
zero failures and its SHA-256 unchanged; an earlier repeat was 136.783 ms.
The default-parameter plugin route also passed at 123.773 ms p95, and the
unchanged no-plugin route passed 500/500 at 115.155 ms p95 after the change.
The production route uses an eight-slot
preallocated queue to absorb native capture packet bursts; callback behavior
remains nonblocking and fail-closed.

Risks: VST2 remains a gated local extension; no change here establishes rights,
vendor-editor window support, W2 callback timing/soak, or broad plugin
compatibility. A signal metric plus a real-DLL worker parameter regression
proves processing alteration, but does not replace attended UI interaction or
listening in the receiving application.

The full locked workspace tests and full UI suite passed. Strict all-features
workspace Clippy currently reports an existing `too_many_arguments` lint in
`finalized_mp3_recording` (`crates/control/src/lib.rs`, unrelated to plugin
hosting); the plugin-host package and M00 probe Clippy checks passed cleanly.

Rollback: the new invocation is opt-in and separate; revert only the plugin
workflow/probe changes if they fail or perturb existing M02 behavior. The
original no-plugin route and its evidence remain intact. Next action: retain
this ReaEQ workflow as the reference acceptance, wire a tested native editor
window only if that richer UX is needed, and qualify attended selection/commit
plus listening when the shell is targetable. Do not infer release availability
from this reference-machine result.

## 2026-09-22 M05 signal-flow visualization specification slice

Objective: make live audio movement between graph nodes plainly visible, with
edge stroke width responding to recent volume, so users can spot silence or a
blocked/faulted path while following microphone and application-capture audio
through the graph.

Requirements: UI-02/03/04/11/12/15 and NFR-08. The user has now authorized
implementation and asked for the visualization to be fully functional.

Prerequisites: existing backend meter/event telemetry and the M05 canvas. Audit
finding: engine diagnostics currently provide authored Meter-stage readings
and automatic sink readings, but do not consistently meter microphone or
application source nodes and intermediate processor boundaries. The UI must
never infer direct per-edge signal levels from connectivity alone. Keep the
work presentation-only and within the existing 20 Hz diagnostics refresh /
NFR-08 telemetry bounds; do not add browser audio processing.

Decision: add a thick, directional edge stroke with a restrained moving
highlight. Derive bounded width from fresh peak/RMS telemetry, smooth and
decay it to avoid flicker, and clamp it to the canvas design range. Preserve
distinct configured-silent, stopped, muted, bypassed, stale, disconnected,
and faulted presentations. Reduced-motion, high-contrast, zoom, and accessible
text/meter equivalents are required; the feature is presentation-only.

Ordered tasks complete: (1) mapped runtime meter observations to edge signal
state using fresh evidence and active-generation status; (2) added a custom
React Flow edge with bounded RMS-responsive stroke width, arrow/dash direction
cue, reduced-motion fallback, and accessible text; (3) covered meter mapping,
width clamping, stale, disabled, privacy-muted, stopped, and ambiguous branch
behavior with UI regressions; (4) passed focused and full M05 checks, docs and
traceability checks, inspected the change, and recorded evidence. No audio or
machine configuration was changed.

Validation results: typecheck passed; `SessionFlowCanvas.test.ts` passed 21/21;
`tests/acceptance/m05-ui.ps1` passed (19 files / 284 tests and temporary
production build); documentation validation passed (58 Markdown files / 266
links); M08 traceability passed (160 normative IDs); `git diff --check` passed.
Attended Windows checks for live microphone/application-capture flow, Narrator,
reduced motion, high contrast, and 100–200% scaling remain open.

Risk/limitation: the backend meters explicit Meter stages and sinks rather than
every source/processor boundary. The canvas uses a downstream meter to show
end-to-end flow only on unambiguous single-output chains and stops at mixers or
fan-out. This confirms flow reaches a measured downstream point, but cannot
pinpoint a block inside an unmetered processor span. See [M05 evidence](evidence/M05-visual-editor.md).

Rollback: revert only the signal-flow visualization implementation if its
freshness/state checks fail or existing canvas behavior regresses; retain
UI-15 as the requested product contract. Next action: qualify in the attended
Windows shell with a live microphone route and an application-capture route,
then review Narrator, reduced motion, contrast, and scaling. Exact processor
fault localization remains a separate backend metering extension if required.

## 2026-09-23 browser design review follow-up

The user reports that the canvas/right workbench is not fully visible and that
Recording controls look inconsistent. Browser reproduction and fix are recorded
in [M05 evidence](evidence/M05-visual-editor.md).
The workspace now accounts for the 58px header, and the recording form uses
workbench field/control styles with a readable default ID. Browser viewport
checks cover 1280x720, 1440x900, and 1920x1080. This is simulated-browser
evidence only; effective native shell DPI and live recording remain open.

**Next task:** continue the user-requested M05 completion goal. First obtain an
attended shell surface for live source-to-output audio/edge animation and
accessibility review; the user has not provided a shell-session identifier, and
the computer-use inventory previously exposed no apps. Continue independent
work on the input/modifier/output E2E matrix and compact-status redesign while
that environment is unavailable. Do not mark live audio or attended visual
acceptance complete based on route-harness tests.

### 2026-09-23 resumed M05 verification

The browser review uncovered and fixed compact-status overflow: at 1280x720 the
document had grown to 2403px and the graph nodes were clipped. The compact mode
now keeps the viewport at 720px, refits the graph after large canvas resizes,
and keeps the Properties inspector usable with its own scroll region. The route
E2E matrix now also adds, connects, plans, and commits an existing virtual
input -> Gain -> existing virtual output path, and checks that managed-driver
tools state why they are unavailable. See [M05 evidence](evidence/M05-visual-editor.md).

Verification: `npm.cmd run typecheck`, UI tests (21 files / 297 tests), and
Playwright (14 tests) passed on Windows 11; `tests/acceptance/m05-ui.ps1`
passed typecheck, the 297 UI tests, and a temporary production build.
Documentation validation passed (58 Markdown files / 275 local links), M08
traceability passed (162 requirement IDs), and `git diff --check` passed. The
compact-status regression confirmed a 1280x720 document with every graph node
inside the graph viewport. The browser harness uses a simulated backend.

Native shell availability check: Windows Computer Use returned no apps or
browsers. The existing `audiorouter-shell` process (PID 53576) is responsive
but has no targetable main window; its backend and MCP logs were last updated
2026-09-22 and no shell log exists. Do not infer that process owns the session
the user saw. No shell restart, process termination, endpoint change, or audio
action was taken.

**Prior next task (superseded by the defect review below):** perform attended M05 live audio and accessibility qualification
when a targetable shell window with the intended route is available. Verify the
live Test Signal -> physical output animation first, then microphone and
application capture with reduced motion/high contrast/Narrator. The independent
tool catalog and route E2E coverage is now in place; do not claim hardware or
MCP-client acceptance from it.

### 2026-09-23 session refresh and playback defect review

Objective: fix reported canvas resets/flicker on Start and make unavailable
audio actionable. Requirements: UI-01/02/04/05/07/08/09/12/15 and the shared
graph/session API. The dirty working tree contains prior authorized work and
must be preserved. Windows and installed UI dependencies are available; browser
fixtures cannot establish audible playback or native shell acceptance.

Code inspection found: session object identity resets the draft on every fresh
snapshot; local creation copies override newer revisions; the live adapter plans
against its startup session instead of the candidate; commit does not reconcile
the saved revision; Start runs the saved graph even with pending edits and
reports simulated runtime as audio success. The browser route fixture did not
persist commits or exercise lifecycle refreshes, leaving these defects uncovered.

Ordered tasks:
1. Preserve drafts/selection across refreshes, prefer newer revisions, and scope
   graph requests to the selected session; reconcile successful commits.
2. Block starting an unsaved graph, distinguish real audio from simulation, and
   provide specific endpoint preparation guidance without claiming a production
   driver is necessary for ordinary physical endpoints.
3. Review native preparation/transport and correct owning-layer defects found.
4. Add fresh-object/event/commit regressions and a stateful browser fixture; run
   targeted UI, adapter, and control tests, typecheck, and browser regressions.
5. Record evidence and remaining native acceptance limitations honestly.

Rollback: reverse only this defect-review patch, preserving earlier UI/backend
work and user sessions. No database migration or endpoint/default change is
planned. Risk: concurrent external edits must remain visible as a conflict,
never silently overwrite the draft.

Implemented: stable draft reconciliation, monotonic inventory merge, selected
session adapter targeting, immediate commit reconciliation, awaited refresh,
pending-plan invalidation on undo/redo, native playback preflight with explicit
setup guidance, and no success claim for simulated audio. Audio import now
retains both media identity and filename across consecutive updates.

The longer browser regression reproduced a second disappearing-canvas cause:
20 Hz renders recreated controlled React Flow nodes without their measured
dimensions, hiding nodes and removing edges until remeasurement. The canvas now
retains dimensions, selection and drag positions. The repeated lifecycle test
failed 2/3 times before this correction and passed 3/3 afterward. Capture privacy
mute also incorrectly suppressed Test Signal/file edge animation; it now marks
capture-only contributions muted while allowing measured synthetic playback.

Verification on Windows 11: UI typecheck; 22 test files / 309 tests; all 15
Playwright tests; production UI build; two focused Rust control status tests;
documentation validation (58 files / 277 links); and diff whitespace validation
passed. The browser fixture now persists commits and returns fresh snapshots,
simulates lifecycle and changing meter values, and checks node visibility,
moving dashes, changing stroke width and stopped animation. It never opens audio.

Real native evidence: the guarded Test Signal check passed on the exact existing
CABLE Output/Input pair, 180 processed quanta, destination peak -18.000 dB.
Before/after media identities/state, endpoint formats and default roles matched.
The first PnP snapshot was denied before any stream opened; the authorized
elevated rerun passed. The UI build initially hit EPERM replacing a prior dist
asset; rebuilding elevated in the verified project dist directory passed.

The shell linked, but Cargo could not replace the existing desktop executable
(Windows access denied, including elevated retry). Its freshly linked executable
was copied without overwriting to
`src-tauri/target/debug/audiorouter-shell-review-20260923.exe`. The existing shell
was not stopped. Do not claim that it is running this new backend.
`cargo check --manifest-path src-tauri/Cargo.toml --locked` also passed.

Evidence and limitations: [M05 defect-review evidence](evidence/M05-visual-editor.md#2026-09-23-refresh-playback-and-canvas-measurement-defects).
Next action: qualify the new shell build against the user's selected saved
session after closing the old shell normally. Attended native UI,
microphone/application animation and accessibility remain separate gates;
browser simulation and the bounded native meter check do not establish them.

### 2026-09-23 attended shell and interaction defects

Objective: make Play/Start failures stable and actionable; simplify the route
workbench, node selection, edit history, connection actions, and tool discovery
reported by the user. Covers UI-01/02/04/05/07/08/09/12/15 and SEC-03.
Preserve the current dirty tree and saved session. Prerequisite: exact Windows
endpoint choice and device-administration opt-in for native preparation; browser
fixtures alone cannot establish audible playback.

Live diagnosis: the old shell (PID 53576) owned the default control pipe while
the rebuilt review UI (PID 64384) connected to it. The old backend reported
zero active sessions, `audio=unavailable`, and its obsolete production-driver
reason. After stopping both idle shells and relaunching the review executable,
its PID 83592 owns the pipe and reports the newer device guidance. The saved
`desktop-session` remains present at revision 4 with two nodes and one edge;
`recorders.list` now succeeds, confirming the current built-in grant. No audio
was started or endpoint changed during this handover.

Ordered work: (1) remove tab switches and layout shifts from failed Play/Start,
put a persistent recovery action near the canvas, and simplify stopped
telemetry; (2) compact status and right tabs, clarify Save route and session
selection/rename; (3) make node deletion immediate/undoable with Delete key,
tone down unrelated-node selection, and improve connection controls and tool
icons/help; (4) run focused browser/component checks and inspect the built UI;
(5) relink/restart the shell with a verified saved route, opt-in and exact
endpoint binding, then record what is and is not audibly/live qualified.

Rollback: reverse only this slice while retaining the previous saved graph;
node deletion before Save is restored with Undo. Do not infer driver support
or choose a replacement microphone automatically. Risks: the active shell
binary is locked during relink, and the exact physical output must be chosen
deliberately for attended playback.

Implemented and observed: the previously running UI was connected to the
old shell's pipe (PID 53576). The current review shell now owns the pipe,
and its endpoint guidance and recording grant are current. Failed Play/Start
keeps the current tab and canvas bounds and offers an explicit Devices action.
Tools stay open while adding several nodes; selection no longer dims other
paths. The status disclosure and tab rail are shorter. Save route performs
backend plan/commit in one click when warnings are empty and requires a second
confirmation for warnings. Session selection is one control with on-demand
rename. Node Delete works without a confirmation and is reversible with Undo.
Connection actions have distinct pause/remove/add icons and tooltips; tool
cards have unique icons and short explanations.

The guarded live route found a real native incompatibility: the UI used mono
ports on Gain and other processors between stereo Test Signal/output ports.
The native compiler accepts only equal channel counts on its single-chain
path, so the old draft planned but failed at Start with UnsupportedTopology.
New built-in processor nodes now default to stereo. A matching-channel
Test Signal -> Gain -> VB-Cable route ran for 10.01 seconds with 597 pump calls,
3,553 processed quanta, zero XRuns, and unchanged endpoint states. The
Focusrite speakers returned 0x8889000A (device in use) during preparation;
no stream started on that endpoint. The UI now explains this ownership
failure and the older unsupported-channel error in plain language.

The successful isolated route m05-signal-1790226441 remains saved for manual
inspection. Three agent-created sessions from failed probes were deleted
after exact ID/name checks. The newest review shell PID 82224 was relinked
with the current UI assets (SHA256
D3409349C075F078D0FDDBB5557963C2EAA585B1DE54A65374B483560FADE11B)
and launched with process-scoped device-administration opt-in. That isolated
route is prepared on the exact existing CABLE Output/Input pair and stopped.
The user's desktop session was not modified. Select the 10-second test route
in Session to press Play and inspect the animation; VB-Cable output may need
separate monitoring to be audible. Do not claim an attended visual check or
physical speaker playback from the headless pump result.

Checks so far: UI typecheck; full Vitest 22 files / 310 tests; full
Playwright 16/16 across tool catalog, editing, route lifecycle, layout and
MCP fixture; production UI build; relinked shell; guarded native pump;
documentation validation (58 Markdown files / 279 local links); and
`git diff --check`. The first full UI run caught a stale expected HRESULT
message; it was updated to assert the new actionable text, then all 310
tests passed. Evidence: [M05 interaction review](evidence/M05-visual-editor.md#2026-09-23---attended-shell-handover-route-compatibility-and-interaction-review).
The Windows native UI surface was unavailable to Computer Use, so browser
screenshots at 1280x720/1440x900/1920x1080 were inspected instead. Remaining:
attended Play/Stop and accessibility inspection on the actual window. The native compiler still
rejects older saved channel-changing or branching routes at Start rather
than at Save; record this as a graph-compatibility defect for the next slice.

### 2026-09-23 occupied output connection defect

Reproduction: on `desktop-session` revision 4, the saved route has
`desktop-input:main -> desktop-output:main`. The user added Test Signal and
tried to connect it to that Physical output. `appendDraftConnection` rejects
the second edge because an ordinary input accepts one connection (GRAPH-02).
The shell/backend logs contain only polling around the report; this is a
renderer draft rejection before any RPC. The UI presents only a transient
text error and no direct repair action. A current desktop browser surface is
unavailable; the saved topology was read from the live shell's pipe without
mutation. Requirements: UI-03/08/13 and GRAPH-02/09.

Plan: expose the occupied-input reason next to the canvas and offer an
explicit, undoable replacement of the prior edge with the attempted source.
Keep mixing a separate visible operation; do not silently mix two sources or
change the saved route until Save route. Verify the occupied-output browser
flow, draft regression, UI typecheck, and no unintended saved-session change.
Rollback: revert this focused interaction and retain the saved revision;
Undo restores the replaced draft edge before Save.

Outcome: both canvas and form/keyboard connection attempts now explain the
occupied input and offer **Replace input connection**. The replacement is one
undoable local draft edit; the old saved `desktop-session` revision 4 remains
untouched. A privacy-safe client diagnostic records the rejection category.
The running PID 82224 remains open to preserve the user's unsaved Test Signal.
A newly built side-by-side shell executable is ready but has not replaced the
running window. Evidence: [M05 occupied-output diagnosis](evidence/M05-visual-editor.md#2026-09-23---occupied-physical-output-connection-diagnosis).

Validation: Windows UI typecheck, Vitest 310/310, Playwright 17/17, UI
production build, shell `cargo build --locked`, docs validation (58 files /
279 links), and `git diff --check` passed. Next: use the currently running
window's remove-edge/Undo workaround for the unsaved draft, or launch the new
side-by-side executable after preserving that draft. The new window's attended
connection and audio acceptance remain open.

### 2026-09-24 output inspector, draft audition, and source summing

Objective: fix Physical Output property sizing, make unsaved route edits safely
auditionable without saving, and support multiple audio inputs converging on
one Physical Output. Covers UI-05/08/09, GRAPH-02/04/09 and API lifecycle
contracts. Preserve the user's live shell and draft; browser checks do not
establish audible native output.

Prerequisites: read M05 visual-editor contract and graph/API specifications;
existing source branch is dirty and must be preserved. No database schema
migration or endpoint change is intended.

Ordered tasks: (1) move the selected-node inspector into the Properties tab and
remove the tiny split-row layout; (2) add an ephemeral backend preview lifecycle
for a validated candidate graph, leaving the saved revision unchanged and
stopping/restoring cleanly; (3) make a second source connected to a Physical
Output create a visible explicit Mixer, preserving GRAPH-02 and compiling
supported source branches to bounded native mixing; (4) add regressions, run
UI/Rust checks, and capture evidence.

Validation matrix: selected Physical Output has usable full-height property
controls; draft Test Signal routes preview without changing persisted revision;
two compatible source edges reach one output and sum without clipping or
callback blocking; incompatible formats/topologies fail before activation.
Rollback: revert this focused feature slice; persisted routes remain untouched.
Risk: native source acquisition differs by source kind, so report any route
types that cannot yet be auditioned instead of silently falling back.

Progress: the Properties inspector now uses the full sidebar height. A second
source connected to an occupied Physical Output inserts a visible, undoable
Mixer while retaining GRAPH-02. Test Signal Play is actionable for an enabled,
routed source. The shared `session.start` API accepts an optional candidate
graph; it checks the current saved revision and GraphWrite permission, plans
the candidate in the UI, publishes it only to an already prepared single-
endpoint native scheduler, and leaves the saved revision unchanged. Stop ends
that preview. Multi-capture preview remains unsupported: its worker is prepared
from the committed Mixer graph and accepts Physical Input/Application Capture
bindings only. Test Signal or Audio File mixed with capture still requires
native source scheduling. The shell logs for the reported earlier click had no
`session.start` request, consistent with the disabled renderer control.

Windows verification: typecheck; Vitest 22 files / 311 tests; Playwright 19/19,
including unsaved Test Signal preview with saved revision unchanged, output
Mixer insertion and Properties sizing; control tests 184 passed / 4 ignored;
Vite production build; shell `cargo build --locked`; docs validation (58 files /
281 links); and `git diff --check`. Browser audio is simulated. No native route
was started in this verification, and the current shell PID 82224 remains open
with its draft untouched. Side-by-side review shell:
`src-tauri/target/debug/audiorouter-shell-review-20260924-preview.exe`, SHA256
`FE488467E3688B4ECAC3D5C0CBE...` (full hash in the evidence note).

Next action: after preserving the current draft and handing off the old shell,
attend-test the new shell with a prepared exact endpoint: preview Test Signal
-> Gain -> Physical Output without Save, verify 10 seconds of native pumping
and unchanged saved revision, then qualify a two-source explicit Mixer route
with exact prepared physical/application capture and output bindings. Do not
claim those native routes verified from browser or portable control tests.

### 2026-09-24 workspace height and simple Save/Play follow-up

Reproduction: opening Properties in compact status view applies a three-row
`main-content` override while the canvas still spans only rows 1–2, shrinking
it and the sidebar. Start currently requires a separately prepared capture
and render endpoint even for Test Signal; the canvas output binding is only a
local hint. The Session panel exposes planning and revision language that is
backend detail. Requirements: UI-01/04/05/08/09/13, API-04/05, SEC-03.

Plan: (1) make the canvas and sidebar use one stable viewport row in every
tab; (2) keep one Save action in the main header and put session management in
Session; (3) surface clear stopped/starting/running/failed status and explain
the selected physical device and preparation prerequisite beside Play; (4)
prepare exact selected endpoints on Play when permitted, without choosing an
unselected capture device; (5) verify layout at desktop sizes, Save and Start
interactions, then record native limitations. Rollback: restore the prior UI
controls and explicit preparation path; do not modify saved sessions or OS
endpoint defaults. Current native adapter may still require capture selection
for a Test Signal route, which must be called out until its source-only path
is implemented.

Outcome: the app now uses viewport rows that follow the actual header and
status heights. Properties keeps both primary columns at their prior height;
the inspector begins directly below the tabs. The header has one Save button,
Play/Stop, and an explicit audio state. Session holds setup management and
Undo/Revert without presenting plan or revision terminology. Starting audio
uses the exact endpoint hints already selected and attempts backend preparation
when necessary. If either selection or device administration is unavailable,
the UI gives a persistent reason. The Devices tab explains its advanced
binding role. The current native adapter still requires a capture selection
for Test Signal; the UI calls this out, and a render-only worker is future
implementation work. No endpoint or saved session was changed by this slice.

Verification: Windows UI typecheck and production build passed; Vitest 22
files / 311 tests passed; Playwright 19/19 passed, including equal canvas and
sidebar heights before/after opening Properties in compact and normal views,
and automatic preparation in the simulated route harness. Screenshots were
inspected at 1280x720 and the page has no outer vertical overflow. The shell
was force-relinked after the UI asset build and copied to
`src-tauri/target/debug/audiorouter-shell-review-20260924-workspace.exe`,
SHA256 `AB2FF89D5B1C274F65EEE7A4DEB243064D9ACBAC65FD47C994724ACF03438FF2`.
The running user shell PID 82224 was left untouched; Computer Use returned no
accessible apps or browsers, so its unsaved canvas could not be handed over.
No native audio was started here.
Next: attended Play with the new shell and exact existing bindings, then
implement and qualify render-only Test Signal playback so selecting a Physical
Output alone is sufficient.

### 2026-09-24 attended noise playback diagnosis

Reproduction: in the current `desktop-session` revision 11, Test Signal and
Physical Input both feed Mixer, which feeds Physical Output. The updated shell
PID 51844 received repeated Play requests. Exact VB-Cable capture and
Focusrite render preparation eventually succeeded, but `session.start`
returned `native graph rejected: UnsupportedTopology`; the header returned to
stopped and no sound played. The shell JSONL logged only the error code, hiding
the reason. A temporary linear Test Signal -> Physical Output candidate started
as native generation 10 for 10 seconds, reported audio available, and stopped;
the saved revision stayed 11. Audible confirmation is pending from the user.

Requirements: GRAPH-02/03/04, UI-05/09/15, API-04/05. Prerequisite: exact
selected endpoints are active; live Windows audio and user listening are
required to establish actual sound. Decision: support this bounded two-source
case in the existing endpoint graph by mixing one deterministic Test Signal
with one physical capture stream before render, preserving both explicit
matrices and the saved Mixer. Other branch combinations remain rejected with
clear guidance rather than silently dropping a source.

Ordered tasks: (1) add a prepared allocation-free additive Test Signal stage
and exact-shape compiler path; (2) expose the full backend error in redacted
shell diagnostics and plain-language UI status; (3) add focused compiler and
UI regressions; (4) run portable checks, build a review shell, and perform an
attended live start/stop on the exact route. Rollback: revert this compiler and
diagnostic slice and relaunch the prior shell; the database and endpoint
defaults remain unchanged. Risk: this narrow graph path does not establish
general multi-source mixer support or release-ready native qualification.

Implementation: the engine now compiles exactly one enabled stereo Physical
Input and one enabled stereo Test Signal converging at one enabled Mixer and
one Physical Output for the single-endpoint worker. The prepared callback
adds the bounded tone to the captured block with the authored matrix products
before the output meter; it allocates and waits on nothing. Source meters for
capture and Test Signal now report their own fresh levels, so both incoming
edges can animate from actual signal while a silent branch stays static. Other
topologies still reject. The shell log records bounded error kind/HRESULT/retryability
and a safe topology reason, without recording arbitrary error text or node
names. UI unsupported-topology guidance no longer incorrectly blames missing
Mixers or channel counts. The saved desktop session remains revision 11.

Verification: `cargo test -p audiorouter-engine --locked` 125 passed; control
184 passed, 4 ignored; shell focused logging test passed; UI Vitest 311 passed
on rerun (one lifecycle test failed only in the first full run and passed both
in isolation and full rerun); Playwright 19 passed; UI TypeScript/Vite build
passed into `ui/dist-review-20260924`; shell debug build with that asset
directory passed; `git diff --check` passed. Review binary:
`src-tauri/target/debug/audiorouter-shell-review-20260924-mixer.exe`, SHA256
`463F79AC1E85B0356203A60FCEBA6336220991031877CED4CF6E817685B44456`.
Evidence: [M05 attended Play diagnosis](evidence/M05-visual-editor.md#2026-09-24-attended-play-failure-and-exact-mixer-repair).
Attended native confirmation: after the user closed the old shell, review PID
60912 loaded saved revision 11 and prepared the exact VB-Cable capture and
Focusrite render pair. `session.start` succeeded as native generations 1, 2,
and 3; matching stops succeeded, and the user confirmed hearing the Test
Signal. The earlier direct 10-second preview was silent because that RPC
probe did not call `nativeEndpoints.pump`; the UI pump is what feeds the output.
Animation confirmation remains pending from a longer Test Signal run with
fresh source/output meter observations. No saved graph change was made.

### 2026-09-24 existing endpoint tool clarity

User reproduction: `Existing virtual input` and `Physical input` create the
same `physicalInput` graph kind, while `Voicemeeter Input` is the Windows
playback endpoint receiving default system sound and is absent from the
capture picker. Requirements: UI-01/05/09, CAP-08, GRAPH-02. Decision: keep
one input-device and one output-device tool, each accepting compatible
physical or already-installed virtual endpoints; do not imply endpoint
loopback activation where the native route cannot prepare it. Guide the
Voicemeeter system-sound path through its B1 capture bus. Tasks: remove the
duplicate shelf entries, explain capture/render direction in tooltips and
device pickers, update regressions and quickstart, then run UI checks and
inspect the diff. Rollback: restore the two duplicated shelf entries and
their notes; saved graph kinds and bindings do not change. Live device
selection and audible confirmation still require the attended user shell.

Outcome: the duplicate existing-virtual cards were removed. The shelf now
offers one Input device and one Output device; both keep their existing graph
kinds, so saved routes remain compatible. Tooltips and the input picker
distinguish playback `Voicemeeter Input` from capture `Voicemeeter Out B1`.
The quickstart records the B1-to-Focusrite setup. The currently open shell is
the earlier build; a separate review executable was prepared at
`src-tauri/target/debug/audiorouter-shell-review-20260924-device.exe`
(SHA256 `3E37F9ACAE5D0712D104A536672FA09F692B5F6266BFBF996267B511124E3248`).
Windows checks: UI typecheck passed; Vitest 22 files / 310 tests passed;
Playwright 19/19 passed; production UI and shell build passed; docs
validation passed (58 Markdown files / 282 local links); `git diff --check`
passed. The first UI production build was denied filesystem write access;
the same command succeeded with the required elevation. No Voicemeeter
setting, endpoint binding, saved route, or live shell was changed. Next:
after the user closes the current shell, launch the new review executable
and attend-test the B1 capture route with system playback.

### 2026-09-24 invalidated endpoint after Voicemeeter switch

Reproduction: shell PID 60912 returned `deviceInvalidated` / HRESULT
`0x88890004` on two `session.start` attempts after the user opened
Voicemeeter; `devices.list` succeeded but no `nativeEndpoints.rebind` occurred.
The UI currently skips preparation whenever a stopped native worker is
attached, even if the selected endpoint IDs changed or the old clients were
invalidated. Requirements: CAP-01/02/03/11/12, UI-04/08/09. Decision: on Play,
rebind an existing stopped endpoint worker to the exact selected capture and
render IDs before starting. Preserve explicit selection and report a missing
endpoint instead of falling back to another device. Give an invalidated-device
error a concrete refresh/rebind action. Ordered tasks: update the start path,
add a lifecycle regression and error-format regression, run UI/build checks,
and prepare a new side-by-side shell. Rollback: restore prior start behavior;
leave persisted route and Windows defaults untouched. Attended B1 audio
requires the user's currently open shell to hand over safely.

Implementation: `session.start` now rebinds a stopped endpoint worker to the
exact input/output IDs selected in the UI before graph activation. This
reopens stale WASAPI clients after a device transition and applies a changed
selection rather than silently starting the previously prepared pair.
`deviceInvalidated` has a clear refresh/select/retry message. The endpoint
inventory still reports default system playback as `Voicemeeter Input`
(render), default capture as `Voicemeeter Out B1` (capture), and the Focusrite
render and VB-Cable pair as active. That inventory does not prove which stale
client failed to start. Voicemeeter's B1 bus needs its mixer app running;
for an app-independent cable route the user must deliberately change Windows
playback to `CABLE Input` and select `CABLE Output` as capture.

Verification: Windows UI typecheck passed; Vitest 22 files / 312 tests
passed, including rebind-before-start and invalidation-message regressions;
Playwright 19/19 passed; production UI build and shell build passed; docs
validation passed (58 files / 282 links); `git diff --check` passed.
Review binary:
`src-tauri/target/debug/audiorouter-shell-review-20260924-rebind.exe`,
SHA256 `E21E65C8F2D12419ACABF81027BBB8892607FC29FB0F80BBF585EA01B5B99CD7`.
The active shell PID 60912 was not closed or replaced and no live rebind/audio
test was performed on it. Next: hand over that shell after its unsaved edits
are preserved, run the new build, and inspect logs for a B1 or VB-Cable
capture-to-Focusrite start while the user checks audio.

### 2026-09-24 route Play versus source Play

User observation: Route Status repeats the top Play/Stop action, and top Play
currently starts the Test Signal tone because that source auto-emits whenever
the session starts. Requirements: UI-01/04/09/15, GRAPH-05, API lifecycle.
Decision: top Play/Stop controls the visible canvas route; Test Signal
Play/Stop controls that source's tone within the running route. A node Play
may start a stopped route first, then start its own tone. Remove the duplicate
Route Status transport buttons but retain state, privacy mute, and the guide.
Extend the existing per-source transport contract to Test Signal with
allocation-free atomic state in the engine; default to silence on route
activation. Ordered tasks: add source transport and control dispatch, wire
node UI, adjust docs/contracts/tests, then run relevant checks and build a
review shell. Rollback: restore whole-session Test Signal controls and prior
source auto-emission; no persisted graph schema or OS endpoint changes.
Live audible qualification remains separate and requires handoff of the open
shell.

Implementation: Test Signal is silent when a route starts. Its prepared source
now accepts `audioSources.transport` play/stop/status, with the audio callback
reading only atomic transport state. Node Play starts a stopped route if needed
and then plays that one source; node Stop leaves the route running. The top
Play/Stop controls the route, and Route Status no longer repeats those buttons.
The guarded live Test Signal script now explicitly starts the source after
starting its route. The method catalog, API/interface specs, quickstart, and
operational API reference describe the split. No saved graph migration or
Windows default-device change is required.

Verification on Windows: engine 125 tests passed; control 185 tests passed
(4 ignored); UI Vitest 312 tests passed; Playwright 20/20 passed, including
silent route start and source-only Play; contract drift passed (92 methods,
21 node kinds, 7 processors, 20 event categories); production UI and Tauri
shell builds passed; docs validation passed (58 files, 282 links); `git diff
--check` passed. RTK was unavailable, so raw commands were used. Review shell:
`src-tauri/target/debug/audiorouter-shell-review-20260924-routeplay.exe`,
SHA256 `3FEB9648FF67727E54D0C0ED790B7D6B298B0455ACA5BCBF53EFFE7BDF415BE4`.
The existing shell PID 60912 was not closed or replaced. Next: preserve its
unsaved route, hand over to this review shell, then attend-test route Play,
Test Signal Play/Stop, and VB-Cable capture with the user's listening feedback.

### 2026-09-24 advanced visual EQ

Objective: match the user-supplied `EqualizerSteelseriesGG.png` reference with
an addable advanced EQ whose points can be placed on a logarithmic frequency
axis, moved for frequency/gain, and assigned peaking, low/high shelf,
low/high pass, or notch behavior. Requirements: DSP-01/02/08, UI-01/05/07/11,
GRAPH-05. Prerequisites: existing parametric EQ biquads, `processors.response`
coefficient-based preview, node parameter validation, and stopped graph edit
path. Decision: retain the `parametricEq@1` graph kind for saved-route
compatibility, present the existing tool as Advanced EQ, and expand its bounded
capacity from 8 to 16 bands. Existing eight-band sessions keep their parameter
names and sound. The per-node parameter bound rises from 64 to 96 because the
new node defaults contain 83 values; a full EQ must pass graph validation.
A visual editor edits those same backend parameters; the
backend response remains authoritative. Spatial audio is separate: the image
shows mode/tuning controls but does not define a signal-processing contract,
so no spatial processor is added in this slice.

Ordered tasks: (1) expand DSP/domain/control capacity and discovery together;
(2) add a property-panel EQ graph with point add/select/drag, filter type,
frequency, gain, Q, enable and remove controls, while keeping the canvas node
compact; (3) update contracts/spec/user guidance and focused regressions;
(4) run relevant portable/Windows checks, inspect diff and browser layout.
Validation: DSP/domain/engine/control and UI checks, contract drift, visual
browser check, docs, diff. Hardware listening remains a separate attended
check. Risk: excessive or unstable cascaded boosts; bound count and values,
use the existing coefficient validation, and keep backend rejection visible.
Rollback: restore eight-band bounds and remove the new visual editor/tool
presentation; no storage migration or endpoint change is required.

Outcome: the existing `parametricEq@1` processor is now shown as Advanced EQ
in the tool shelf and new node names; saved node kinds and user names are not
renamed. Its bounded DSP, validation, catalog, and response API support sixteen
bands. The per-node parameter limit is now 96 so the full 83-value EQ default
passes backend graph validation; a dedicated regression covers it. The
Properties panel has a logarithmic graph with add/double-click,
selection, pointer drag, filter choice, precise frequency/gain/Q controls, and
remove (disable). The curve comes from the backend's coefficient-based
`processors.response`; the browser-only harness supplies a labelled simulated
shape for visual tests. The compact canvas node remains unchanged apart from
reading all sixteen bands. Spatial audio is recorded under
`docs/plans/future/spatial-audio.md` because mode/tuning labels do not define
a reproducible processor.

Windows verification: DSP 34, domain 67 (including the full-EQ regression),
engine 125, and control 185 tests
passed (4 control tests ignored); UI Vitest 23 files / 314 tests passed;
Playwright 21/21 passed, including point add/filter/frequency/drag/remove and
desktop geometry; contract drift passed (92 methods, 21 node kinds, 7
processors, 20 event categories); production UI and Tauri builds passed;
docs validation passed (59 files, 284 links); `git diff --check` passed.
`cargo fmt --all -- --check` still fails on formatting differences in the
preexisting dirty control/engine changes; no workspace-wide formatting was
applied. Browser visual evidence is the simulated
`docs/plans/active/evidence/advanced-eq-browser-preview.png`. Review shell:
`src-tauri/target/debug/audiorouter-shell-review-20260924-advanced-eq.exe`,
SHA256 `4FAA8877A6B28DE9355D2CFD064F4FFE0B684565CBC9B4E2BA1346555D7DB56E`.
No shell process was present before launch. The review shell is now open as
PID 63692 and responding; no audio endpoint or saved session was changed.
Next: audition and inspect the real sixteen-band response with an attended
audio route.

### 2026-09-24 mono microphone endpoint qualification

Objective: make a mono PD200X capture endpoint usable in the stereo graph and
bind it to the user's Cable A voice route. Requirements: CAP-01/02/03/11/12,
GRAPH-03, API-04. Reproduction: the live device inventory reports PD200X as
48 kHz mono float32, while VB-Cable A render is 48 kHz stereo. The current
duplex preparation hardcodes a stereo scheduler and rejects endpoint channel
mismatch before capture starts; graph edges also require a stereo physical
input node. The first attempt returned `InvalidFrameSize` and saved a
mono-to-stereo graph that the engine cannot compile; it must be corrected to
stereo graph ports with an explicit endpoint-boundary mono duplicate.

Decision: retain stereo graph ports for the existing processor chain and
perform bounded mono-to-stereo duplication at the native endpoint boundary,
equivalent to the specified `[1,1]` map. Do not alter Windows defaults or start
live microphone audio during preparation. Keep the game route bound to Cable B
capture and Focusrite render. The existing control plane had a single endpoint
worker slot, which prevents the independently routed mic and game sessions from
running together. Expand that bounded capacity to two exact endpoint workers,
preserving per-session graph, pump, stop, detach, privacy, and invalidation
ownership. The UI must keep servicing every native endpoint session it started
even when another session is selected. Preparation also initially returned
`deviceInUse` for Focusrite; after shell recovery, the exact same binding
prepared successfully, so the earlier ownership conflict was transient and
does not justify changing endpoints. Avoid direct pipe probes that close before
the server disconnect handshake; that reproduced a backend pipe-ended error
and durable safe mode. The current task uses a cleanly restarted shell and
waits for the server's disconnect before releasing each probe connection.

Ordered tasks: (1) extend the shared WASAPI scheduler bridge to accept a mono
capture / stereo render pair, duplicate each mono frame into left and right in
preallocated storage, preserve graph-rate resampling and stereo output, and
reject unsupported channel shapes with a specific diagnostic; (2) add focused
regressions for mono duplication, supported endpoint metadata, and invalid
formats; (3) correct the saved voice graph to stereo ports and exact PD200X ->
Cable A bindings; (4) add a second bounded control-plane endpoint slot with
per-session lifecycle and endpoint-change handling; (5) update UI start and
pump handling so both exact endpoint sessions remain serviced; (6) prepare both
routes stopped, inspect backend logs, and verify session state; (7) start only
after both native preparations succeed, then inspect flow telemetry and keep
the routes running for the user.

Validation: focused Windows-audio tests, control tests for session compilation,
UI tests/build, formatting, `git diff --check`, and a live preparation/start
attempt on the identified Windows devices. The Focusrite `deviceInUse` was observed
once and later cleared; a repeat is a distinct remaining hardware owner and
must not trigger fallback. Risks: channel duplication is a deliberate audible
mono-to-stereo map; invalid formats must not enter the callback. Rollback:
detach only the two newly prepared route workers, stop only the two new
sessions, revert only the bridge conversion and its regressions, and remove
the two newly created route sessions if the user asks; preserve existing test
sessions and OS endpoint defaults.

### 2026-09-24 correction: Siege and microphone must share one session

The user clarified that the Siege/game path and processed microphone path must
run as separate branches of one AudioRouter session, because Siege, Discord,
and Outplayed are used at the same time. The two independent sessions prepared
above are not an acceptable final configuration. Their native endpoint workers
were detached and both are stopped. The user explicitly authorized deletion
on 2026-09-24; both assistant-created sessions and their histories were deleted
through the running backend. Do not start either split route as the user's
final setup.

Code inspection found the current native multi-input path compiles a specific
topology: multiple direct capture sources feed one mixer, one shared processing
chain follows the mixer, and that same signal fans out to its outputs. It cannot
apply independent processing to the game and microphone branches or send them
to different destinations. The standard realtime scheduler also currently
accepts one input block and produces one output block, while the Windows
endpoint adapter binds one capture/render pair. Merely placing two graphs in
one session would therefore either fail activation or incorrectly mix game
audio into the microphone feed. Do not claim this task is complete by saving
such a graph or by running the two independent sessions.

Required work before the requested setup can be called functional: specify and
implement one session with independent source paths, e.g. Cable B capture ->
game EQ -> Scarlett render, and PD200X capture -> microphone EQ/VST -> Cable A
render. Bind exact endpoints per path; compile and service every path under one
session generation; publish telemetry and lifecycle state per path; fail closed
on endpoint loss; preserve microphone privacy; and ensure Outplayed can select
the processed Cable A mic and the processed game/system mix. If Outplayed needs
a combined recording feed, explicitly route the required mix to its chosen
capture endpoint without feeding game audio into the voice-chat microphone.
Requirements include GRAPH-01/02/03/09/10/12/14/15, CAP-01/02/03/11/12/13,
API-04/08, and the M02 latency/lifecycle gates. Stable requirements were
added to [04 Graph](../../spec/04-graph.md) and [05 Windows capture](../../spec/05-windows-capture.md),
with the acceptance scenario added to [M02](../../milestones/M02-audio-engine.md).

Next actions: (1) extend API contracts, limits, rollback, and failure behavior
for parallel per-source processing and distinct destinations; (2) update the
active plan's validation matrix before substantial implementation; (3) implement bounded
per-path realtime scheduling and exact endpoint binding in one session; (4)
add portable and Windows regressions plus an attended test with the installed
Cable A/B and Focusrite; (5) configure one saved session, select Cable B as
Siege's output, Cable A Output as Siege/Discord mic input, and Outplayed inputs
for processed game plus processed mic; (6) verify the user can hear/record both
without cross-feeding game audio into the voice mic. Keep the two assistant-
created split sessions stopped until the one-session route is ready. Current
live evidence: both exact split bindings prepared successfully after the
two-worker backend trial, then detached successfully. Neither route was
started; that evidence is not a same-session qualification. Command outcomes,
limitations, and the next acceptance scenario are recorded in
[2026-09-24 route review evidence](evidence/2026-09-24-siege-mic-route-review.md).

### 2026-09-24 application-capture picker discoverability and theme

Objective: make the application-source picker easy to find in the right Tools
tab, style its native application selector for the active theme, and explain
that Application Capture is a source into the graph rather than a way to inject
audio into another application. Requirements: UI-01/09/11, CAP-05/06/09.
Prerequisites: existing application inventory, process-loopback worker, and
virtual endpoint routing remain unchanged. The user screenshot shows the
picker action above the tool search and a dark-theme selector whose option list
is unreadable.

Decisions: move the application picker action into the Inputs group, give it a
distinct app-to-graph icon and short supporting label, and scope native
select/option colors and `color-scheme` to the picker dialog for dark, light,
and high-contrast themes. Explain in the picker that application capture is
input-only; sending processed audio into an app requires an Output device node
targeting an installed virtual playback endpoint and selecting its paired
capture endpoint in that app. Do not imply that AudioRouter can directly
assign a process output or that this workflow needs AudioRouter's deferred
managed driver when using an existing VB-Cable/Voicemeeter endpoint.

Ordered tasks: (1) place the application source action under Inputs in the
Tools tab and keep its icon/helper copy; (2) theme the picker select and native options with readable contrast;
(3) clarify one-way application capture and virtual-endpoint output in the
picker/source guidance; (4) inspect the diff and run available non-test static
checks.
Validation: review the rendered UI if a browser surface is available, inspect
the final source diff, and run `git diff --check`. Do not claim attended shell
or live audio evidence from a source-only review.

Risk: Windows native option-list rendering can differ from the select control;
set both `color-scheme` and explicit option foreground/background colors.
Rollback: revert only the picker markup, scoped styles, and this plan entry; no
graph schema, backend, device, or session changes are involved. Next: complete
the UI adjustment and report that Application Capture only contributes audio
to an AudioRouter input path.

Outcome: the Tools sidebar search is separate from the application-source
action, which sits under the Inputs group with a window/audio icon and a concise
description. Code inspection confirmed the chooser directly creates the
`applicationCapture` graph node; the separate disabled “Application capture”
catalog card had no independent behavior and duplicated the chooser. It has
been removed from the tool catalog. The chooser and source guidance distinguish
input capture from sending route audio through an installed virtual output.
The application-picker select and options use theme-aware foreground,
background, and native color-scheme settings for dark, light, and high-contrast
themes. Per the user's follow-up, the chooser was moved from the Add tools intro
into the Inputs group. Removal of the redundant catalog entry is the current
change; verification is recorded below.

### 2026-09-24 application process exit and restart reconnection plan

Objective: keep an application-capture graph source configured when its target
application exits, show its stopped/reconnecting/connected state, and resume
capture automatically when exactly one safely verified matching application
instance starts again. Requirements: CAP-05/06/11/13, GRAPH-09/14/15,
API-04/08, UI-04/08/15, NFR-08.

Evidence and gap: `appendApplicationCaptureNode` currently persists executable,
optional executable path, and (when available) the selected PID plus process
creation time. The native worker preparation path binds to that exact instance.
`resolve_application_restart_with_path` already provides a unique-match
resolver, but code search found no product-runtime caller. Canvas rendering has
no process-liveness state, and application inventory refreshes on general
snapshot events rather than reliably reporting each app source's lifecycle.
The picker currently tells users that restart requires deliberate
re-selection, which conflicts with CAP-06's unique verified restart binding.

Decisions: persist a restart-safe selector based on verified executable and
full path identity; treat PID and creation time as the current instance lease,
never as the saved selector. On exit, preserve the node and route, report that
source as stopped/silent, and stop or release only that process-capture worker.
Reconnect with bounded backoff and process/session notifications where
available. Rebind automatically only when one candidate matches the verified
selector. Keep the source silent and report an actionable ambiguous state when
there are zero or multiple matches, path/identity cannot be verified, or the
process is unsupported. Never broaden a selected-instance source to all
instances, bind by basename alone, or fall back to a microphone/endpoint.
Expose per-node lifecycle through the versioned backend snapshot/event contract
so the canvas can show Stopped, Reconnecting, Connected, Ambiguous, and
Unsupported distinctly. Signal-flow animation still requires fresh audio
telemetry; process presence alone must not animate a route.

Ordered tasks: (1) trace current Windows process observation, worker ownership,
session generation, and API event/snapshot pathways; map the current node
selector and policy semantics; (2) add a bounded backend lifecycle/rebind state
machine using verified executable plus full-path identity and unique-match
resolution, with exact creation-time checks against PID reuse; release only the
exited source worker and recover it without restarting unrelated sources where
the existing session activation policy permits; (3) publish privacy-safe
per-source lifecycle transitions and actionable errors through the shared API;
(4) show status on the canvas and in Properties, retaining the node and keeping
meters/edge animation inactive until fresh signal arrives; correct picker help
text to describe automatic unique-match reconnection and manual ambiguity
resolution; (5) add regressions for exit, unique restart, multiple matching
instances, PID reuse, executable/path mismatch, unsupported/protected process,
backoff bounds, repeated crash/restart, and unaffected parallel route paths;
(6) qualify Discord and Siege close/reopen behavior on Windows with exact
process identity and audio evidence, plus API/UI acceptance and documentation.

Prerequisites: Windows process notification or a measured bounded inventory
polling strategy; exact application identity from the existing inventory;
backend worker lifecycle control; Windows machine with Discord/Siege available
for attended acceptance. Hardware and a driver are not required for process
loopback, but live source audio still needs an authorized output route.

Validation matrix: backend transition tests cover exit without node deletion,
unique verified restart, ambiguous duplicate instances, stale PID reuse,
identity/path mismatch, process crash loops, bounded reconnect cadence, and
per-path failure isolation. Contract tests verify stable status/event fields
and redaction. UI checks verify clear text/icon status, no false signal animation
while the app is merely open, and fresh meter-driven animation after audio
resumes. Windows acceptance records OS/app versions, selector path, observed
PID/creation-time changes, lifecycle events, capture/render meters before exit
and after restart, and confirms unrelated routes remain healthy. A portable
simulation is not Windows reconnection evidence.

Risks: process-loopback support and inventory visibility vary by app and Windows
state; matching by executable path can still yield multiple legitimate
instances. Rebinding must not widen capture scope or restart the whole graph
unexpectedly. A source-only failure policy must align with CAP-13's declared
all-or-nothing activation policy. Process notifications may be unavailable or
missed, so any polling fallback needs bounded rate and stale-state behavior.

Rollback: disable automatic worker rebind while retaining the graph node and
explicit stopped/ambiguous status; preserve the last verified selector and
existing exact-instance preparation path. Do not delete or rewrite user graph
nodes during recovery.

Next action: after implementation and portable verification, run the attended
Windows close/reopen acceptance with Discord and Siege only when those apps can
be safely restarted without interrupting the user's active work. Do not claim
live restart recovery until that acceptance passes.

Implementation started on 2026-09-24. Trace confirmed the current application
route uses one `ProcessLoopbackWorker` attached to a session and an explicitly
selected render endpoint. It can be restarted with a newly verified capture
client while retaining the graph scheduler/render binding. The runtime helper
for unique process matching exists but has no caller. The initial implementation
will use a bounded one-second process-inventory poll while that worker is
running (no process-start notification integration exists yet), then apply a
capped retry delay if verified activation fails. Per-source lifecycle will be
carried in the existing versioned diagnostics snapshot and refreshed by the
existing one-second UI diagnostics poll; no audio payload or executable path
will be added to that status. This implementation preserves the existing
single application-worker/topology limitation and does not claim independent
multi-path processing.

Implementation update on 2026-09-24: added a one-second lifecycle poll on the
control-plane pump, exact PID plus creation-time exit detection, and automatic
reattachment through the existing process-loopback worker while retaining the
compiled graph and render endpoint. Rebind requires the persisted executable
and full path; the worker stream reset clears queued audio before resumed
playback. The diagnostics snapshot and `application.captureStateChanged` event
report privacy-safe per-node states, and canvas nodes/edges remain inactive
until the source is connected and new audio telemetry arrives. Errors are
bounded/retried with a one-second to five-second backoff. Windows inventory now
includes parent PID internally so same-image child processes (as in
Electron/Chromium apps) fold under a unique matching root; independent roots
remain ambiguous. The Discord process inventory showed several same-path
processes, so this root grouping was added before considering the matcher safe
for common desktop apps. We did not terminate the live Discord process.

Verification on 2026-09-24: `cargo test -p audiorouter-windows-audio --locked`
passed (91 tests), `cargo test -p audiorouter-control --locked` passed (185
passed, 4 ignored), UI `npm.cmd run typecheck` passed, UI `npm.cmd test` passed
(315 tests), and `git diff --check` passed. Windows machine was available, but the live Discord
close/reopen and audio-resumption acceptance was not run to avoid interrupting
the user's active process. `cargo fmt --check` reports formatting differences
across the workspace, including existing and touched code; applying workspace
formatting would produce unrelated churn, so it was not applied. `rtk` is not
installed in this environment.

Remaining risk/gate: process tree identity grouping and lifecycle are covered
by portable regression tests, but real Discord/Siege restart, process-loopback
audio after rebind, and unrelated live-route continuity still require attended
Windows evidence. Per-source unsupported states are actionable and fail closed;
CAP-13 all-or-nothing route compatibility remains to be confirmed during that
acceptance.

Attended test setup on 2026-09-24: the saved `desktop-session` was inspected
read-only and contains a physical-input/test-signal/mixer/physical-output graph,
not an application-capture source. To preserve it, a byte-identical database
copy was made under the ignored `target/discord-test-profile/` workspace path,
and a separate temporary `Discord reconnect check (temporary)` session was
added there with a selected Discord root identity and physical output. The
main LocalAppData database was not modified. The updated debug shell was built
and launched against this isolated profile. No session-start or reconnect
evidence has been recorded yet; next action is to select that temporary session
in the visible UI, choose the Focusrite render endpoint if prompted, start it,
and observe the connected/closed/reconnected states while Discord is restarted.

Attended Discord restart attempt on 2026-09-25: the rebuilt debug shell (PID
4248) was launched with `AUDIOROUTER_DATABASE` pointed at
`target/discord-test-profile/state.sqlite`, and the user closed and reopened
Discord several times. No UI change was observed. The shell log
(`%LOCALAPPDATA%\AudioRouter\logs\shell.jsonl`) shows that the UI stayed on
`desktop-session`, and no `sessions.start` was issued for
`discord-reconnect-check`, so the lifecycle poll was never exercised. That
restart attempt therefore produced no reconnect evidence. Separately, Discord
self-updated during the restarts, moving from `app-1.0.9258\Discord.exe` to
`app-1.0.9259\Discord.exe`. `resolve_application_restart_with_path` requires
an exact persisted full-path match, so even a started session would report
`ApplicationRestartNotFound` and would not reattach after an update of a
Squirrel-style versioned install. Whether restart identity may accept a
version-directory change needs a decision before any change is made, because
the exact-path rule is deliberately fail-closed.

UI follow-up on 2026-09-25 (attended feedback, UI-04): (1) the
application-capture inspector now has an application selector
(`ApplicationCaptureBinding` in `ui/src/App.tsx`, backed by
`rebindApplicationCaptureNode` in `ui/src/draft.ts`). It swaps executable,
path, policy, PID and creation time as one undoable draft change and keeps the
node id, connections and other parameters. The selector is disabled while the
session runs. (2) The "Add an application capture source" picker previously
listed only processes with `captureCapability === "observed"`, which means a
*microphone* session, so Discord disappeared once it was restarted outside a
call. Process loopback captures what an app plays, so the picker and inspector
now list every running application via `applicationCaptureChoices`. Processes
with audio sessions come first, and other same-path helper processes fold into
their earliest-created instance. (3) New application-capture nodes are created
enabled; they were disabled before, which drew the connection as an
unexplained orange dash. Capture still begins only when the session is
explicitly started. The canvas legend is now a per-state list with line
samples. Verification: UI `npm.cmd run typecheck` passed and `npm.cmd test`
passed (319 tests, including new rebind, choice-grouping and legend-coverage
regressions). Attended visual check in the running debug shell is still
pending.

Attended follow-up on 2026-09-25: Play returned `permissionDenied` for
`nativeEndpoints.prepare` (`DeviceAdministration`) because the shell had been
launched without the documented `AUDIOROUTER_ALLOW_DEVICE_ADMIN=1` opt-in. It
was relaunched with that opt-in against the same isolated database (PID 69308,
stderr confirms the grant). The node header chip previously showed a generic
"ready" for every enabled node, which contradicted a disconnected application
source. `nodeHeaderState` in `ui/src/SessionFlowCanvas.tsx` now shows
application-source states: live, reconnecting, stopped, app closed, pick
instance, pick app, retrying, not prepared. Non-live states use an attention
style. UI typecheck passed, and `npm.cmd test` passed (321 tests).

Open decision: the user proposed reconnecting by application name when a
restarted process has a new PID. While a session runs, the backend already
re-resolves by executable plus exact path and a unique root. The remaining
gaps are (a) prepare with a saved PID that has exited, which fails instead of
using the same resolver, and (b) Squirrel-style version-folder path changes.
(b) relaxes the fail-closed identity rule, so it needs an explicit decision
record before implementation.

Decision on 2026-09-25 (user-approved, CAP-06): application restart identity
now accepts a self-updated version directory. `resolve_application_restart_with_path`
(`crates/windows-audio/src/lib.rs`) prefers an exact persisted full-path
match. Only when no exact path is running does it accept a path that differs in
exactly one `app-<digits[.digits]>` directory, with an identical file name,
depth and all other components. The unique-root/ambiguity rules still apply,
and pure name matching was declined as unsafe. Preparing a source whose saved
PID has exited now falls back to the same resolver through
`bind_application_or_restarted`. This covers `nativeApplications.prepare` and
the application sources of `nativeMultiInputs.prepare`. A reused PID for a
different executable still fails closed because the resolver re-verifies
name, path and creation time. The application-capture runtime tracks
`current_executable_path` separately from the persisted selector, so a
reconnected process in a new version folder is not re-detected as exited on
every liveness poll. Spec CAP-06 was updated. AGENTS.md now makes the attended
launch recipe `AUDIOROUTER_DATABASE` plus `AUDIOROUTER_ALLOW_DEVICE_ADMIN=1`
(user preference).
Verification: `cargo test -p audiorouter-windows-audio --locked` passed (92,
including the new `restart_binding_follows_one_self_updated_version_directory`),
and `cargo test -p audiorouter-control --locked` passed (185 passed, 4 ignored).
Live Discord close/reopen/update acceptance is still pending attended evidence.

Attended follow-up on 2026-09-25 (Play with application capture): Play always
prepared the physical capture/render pair (`nativeEndpoints.prepare`).
`session.start` then rejected a graph that contained an application source
(`UnsupportedTopology` in the shell log). Mixing mic + Test Signal + Discord
into one Mixer is also beyond the current engine's compile topologies. The UI
also never pumped a `process-loopback` adapter, so the backend's
liveness/reconnect maintenance (run inside `nativeEndpoints.pump`) could not
run from the canvas. Changes: `applicationOnlyRouteSource` (`ui/src/draft.ts`)
detects a saved route whose only enabled source is one bound application.
`startSession` then prepares `nativeApplications.prepare` (include tree,
exact output from Physical Output Properties) instead of the endpoint pair,
detaching any stale worker for that session first. `selectNativePump` maps
`process-loopback` to the endpoint pump. The picker now shows one entry per
executable path (the earliest-created instance, normally the tree root)
instead of one per audio-session helper process. UI typecheck passed, and
`npm.cmd test` passed (323 tests). Mixed application + physical/test-signal
routes still require the Many-input panel and do not reconnect automatically.
Attended Discord evidence is still pending.

Attended follow-up on 2026-09-25 (message visibility): the user retried Play
on `desktop-session`, whose mic and Test Signal sources are still enabled
beside Discord. Play therefore took the endpoint path and hit the generic
`UnsupportedTopology` message again. The top message bar also showed action
messages only on the Properties tab; other tabs placed them in a small
sidebar line and showed the default "Connect a source…" hint on top.
Changes: the top bar (`global-action-message`) is now the single location for
action messages on every tab. It carries the sidebar's Replace input
connection / Open Devices / Open Session actions and a dismiss button, and the
duplicate sidebar copy was removed. `actionMessageTone` (`ui/src/actionMessage.ts`)
classifies failure phrasing so errors render red with an icon and
`role="alert"`, in the dark, light and high-contrast themes. Play now blocks a
route that mixes an application source with other enabled sources, and says
which sources to turn off (`mixedApplicationRouteOtherSources`), instead of
reaching the engine's generic topology error. Lifecycle tests that located
messages inside the sidebar now read the top bar. UI typecheck passed, and
`npm.cmd test` passed (325 tests).

Attended follow-up on 2026-09-25 (disabled sources on a Mixer): the saved
`desktop-session` (revision 16) had its microphone and Test Signal disabled
but still wired into `mixer-1` by enabled edges, beside the enabled Discord
source. The shell log showed the old endpoint path (`nativeEndpoints.rebind`
then `session.start` returning `UnsupportedTopology`), so the WebView was
still running pre-change JS. Hot reload had not applied, and the shell was
restarted. Separately, the general compiler counts every enabled edge, so a
Mixer fed by disabled sources is rejected as a multi-input topology even when
only one source is live. Added `audiorouter_engine::prune_inactive_upstream`,
which iteratively removes disabled nodes with no live enabled input, together
with their edges. The control plane applies it in
`compile_session_graph_with_audio` only when this session's attached adapter
is the process-loopback application worker. Endpoint routes deliberately keep
disabled physical inputs, because their mute stage silences the microphone
that worker really captures (privacy). Verification:
`cargo test -p audiorouter-engine --locked` passed (126, including the new
`pruning_drops_only_disabled_sources_nothing_feeds`), and
`cargo test -p audiorouter-control --locked` passed (185 passed, 4 ignored).
The debug shell was rebuilt and relaunched with the standard recipe.

Attended acceptance on 2026-09-25, Discord restart while the route runs
(user-observed success). Environment: Windows 11 Home 10.0.26200, debug shell
PID 35308 launched with `AUDIOROUTER_DATABASE=target/discord-test-profile/state.sqlite`
and `AUDIOROUTER_ALLOW_DEVICE_ADMIN=1`, and Discord
`...\Discord\app-1.0.9259\Discord.exe`. Route: saved `desktop-session`
revision 16, with Discord application capture into `mixer-1` into the
physical output, and the microphone and Test Signal disabled but still wired.
The shell log (`%LOCALAPPDATA%\AudioRouter\logs\shell.jsonl`, processId 35308)
records `nativeApplications.prepare` ok, `session.start` generation 1 ok, a
user `session.stop`, and `session.start` generation 2 ok. The user then quit
and reopened Discord. The new Discord root started at 17:22:08 as PID 83496
(the node was saved with PID 24264), and no further start or prepare RPC was
issued. The user reported that the route reconnected and audio resumed
("it worked"). Per-pump RPCs are not written to the shell log, so the node
state transitions are user-observed rather than logged.
Still unverified live: reconnect across a Discord self-update into a new
`app-<version>` folder (covered only by the portable resolver test); mixed
application + physical/Test Signal routes, which remain Many-input-only
without automatic reconnect; and Siege close/reopen. Next action: exercise a
Discord version-folder update when one is available, then decide whether
mixed application routes should run from Play (a multi-input worker with the
reconnect runtime).

Progress on 2026-09-25, tasks 1–2 (Volume tool, Mixer per-input volume):
- Added a `volume` node kind: domain registry and `percent` validation 0–200;
  the engine compiles it to a linear `Gain` stage and allows it wherever Gain
  is a pass-through/bypass processor; discovery advertises `percent`.
- Added Mixer `inputVolume:<upstreamNodeId>` (0–100) through
  `audiorouter_domain::mixer_input_volume`. It is applied by scaling the input
  matrix in every mixer compile path: the linear single-live-input path,
  `compile_mixer_session`, `compile_mixer_fanout_session_with_plugins`, and
  the capture + Test Signal mixer.
- UI: Volume library entry, defaults and node-face fader. Mixer node-face
  per-input faders and a "Mixer inputs" inspector section (`mixerInputs`,
  `mixerInputVolumeKey` in `ui/src/draft.ts`). The generic parameter editor
  skips parameter families. Delay is presented as "Sync (delay)" (D1), and its
  canvas fader range is corrected from 0–2000 to the backend's 0–1000 ms.
- Spec: DSP-10 added.
- Verification: domain tests 68, engine tests 127 (new Volume compile test and
  per-input mixer assertion), control tests 185 (4 ignored), UI typecheck,
  and UI tests 327. Updated test expectations for the Volume library entry and
  the renamed Sync insert label.
- Limitation: parameter edits republish the compiled graph, so Volume and
  Mixer input changes step exactly as Gain does today (no cross-graph ramp).

Progress on 2026-09-25, task 3 (per-input chains, D4):
`compile_mixer_fanout_session_with_plugins` now walks upstream from each Mixer
input through linear chain processors (`is_chain_processor`: Gain, Volume,
Mute, Meter, EQs, dynamics, Delay/Sync, Pitch, bound Plugin) to the real
source. `input_node_ids` still names the real sources for native binding. Each
chain compiles through the extracted `compile_processor_chain` helper (shared
with the post-Mixer chain) and runs in `CompiledMixerFanoutGraph::mix_inputs`
on a block preallocated at `PROCESSING_QUANTUM_FRAMES`. That block is gated by
`RealtimeDsp` (fail-closed: contention silences only that input for one
quantum). The Mixer mixes input by input through `MixerStage::mix_input`.
Per-input volume keys on the node directly upstream of the Mixer, matching
the UI. A processor fed by more than one connection is rejected. Chains are
capped at `MAX_MIXER_INPUT_CHAIN_NODES` = 16. The fan-out output minimum was
relaxed from 2 to 1 so a plain "Mixer → speakers" route compiles on the
multi-input path. Telemetry and plugin-health lookups also search input
chains. Verification: engine 128 (new
`mixer_fanout_runs_a_volume_chain_before_one_input_and_allows_one_output`),
control 185 (4 ignored), and windows-audio 92.

Checkpoint on 2026-09-25 before task 4: the native multi-input path
(`nativeMultiInputs.prepare`) has no output-device argument. Its branches are
delivered through a separate branch-ring and output pipeline, which (per the
2026-09-24 Siege review) has never been exercised on live endpoints. Task 4
therefore needs Play to orchestrate output and input preparation in compiled
input order (`input_node_ids`), plus a per-source application-capture runtime
inside the multi-input worker, and then attended qualification. Until then,
tasks 1–3 are usable live on the qualified application-worker path
(one live source → Volume → Mixer → output, with disabled sources pruned).
The debug shell was rebuilt and relaunched with the standard recipe.

Plan update on 2026-09-25 (user request): add task 5b, a **Bass & Treble**
tool with two sliders; and fix the missing Volume icon in the tool library.

Progress on 2026-09-25, task 5 (Sync, Dehum, Declick) and 5b (Bass & Treble):
- Domain: new kinds `bassTreble` (bassDb/trebleDb ±12), `dehum`
  (frequencyHz 45–65, amountPercent 0–100, harmonics 1–8), and `declick`
  (thresholdPercent 0–100; `latency_samples` = `DECLICK_LATENCY_SAMPLES` 64).
- DSP: `audiorouter_dsp::restoration::Declicker`, a mono second-difference
  detector with a 40 ms warm-up and linear-interpolation repair inside a
  64-sample lookahead ring. Allocation-free.
- Engine: Bass & Treble and Dehum compile to the existing `ParametricEq`
  stage (shelves; Q-20 peaking cuts per harmonic). Declick adds a
  `ProcessingStage::Declick` with per-channel `RealtimeDsp` state and reset.
  All three are accepted as chain/bypass processors, including Mixer input
  chains.
- Control discovery parameters; UI library entries (new "Restoration"
  category), icons for Volume and all planned tools, node-face faders
  (Bass/Treble pair, Dehum amount, Declick threshold), Dehum 50/60 Hz preset
  buttons, and the insert-on-connection list.
- Spec: DSP-11, DSP-12, DSP-13.
- Verification: dsp restoration tests 3 (click repair, bit-exact
  pass-through, non-finite safety); engine test
  `restoration_and_tone_tools_compile_and_shape_known_signals` (60 Hz hum
  < 10 % with 1 kHz within 5 %, +12 dB bass > 3×, −12 dB treble < 0.4×,
  Declick transparent); domain 68, engine 129, control 185 (4 ignored), and
  UI 327.

Progress on 2026-09-25, task 4 (Mixer routes with applications from Play;
per-source reconnect):
- Control: `nativeMultiInputs.prepare` and `nativeOutputs.prepare` accept an
  omitted `generation` and default to the session's next start generation
  (`requested_or_next_generation`). A running session is refused. The
  schemas and contract were updated. `nativeEndpoints.detach` also releases a
  stopped session's multi-input worker. Multi-input prepare prunes disabled,
  unfed sources (safe here: only bound sources are opened).
- Per-source application runtime (`MultiInputApplicationSource`, one per
  application input). `maintain_multi_input_applications` runs from the
  multi-input pump with a one-second probe. An exited application's input is
  swapped to `MultiInputCaptureSource::Silence` so the other inputs keep
  playing. The replacement is found with the CAP-06 resolver (exact path,
  else one version directory), re-opened, and swapped in with
  `NativeMultiInputWorker::replace_capture`, which resets only that input's
  partial packet. A pump error while an application source exists forces an
  immediate probe, and a successful swap turns the error into a zero-progress
  pump. An application closed at prepare time starts silent (`app-closed`)
  instead of failing the route. States are reported per node through
  `applicationCaptureStates` (maxItems raised to 8).
- UI: Play detects a saved single-Mixer route mixing applications with at
  most one input device (`mixerRouteSources` and `pruneInactiveUpstream`
  mirror the engine). It detaches a stale worker, prepares sources in engine
  input order and the selected output, then starts; the pump uses the
  multi-input path. Unsupported mixes (Test Signal or Audio File with an
  application, or more than one input device) get specific messages.
- Verification: windows-audio 93 (new `silent_capture_offers_bounded_zeroed_packets`),
  control 185 (4 ignored), UI typecheck, and UI 330 (route-source and pruning
  tests).
- Not yet exercised live: multi-input output delivery, the silence pacing
  assumption, loopback behavior at process exit, and reconnect. This is the
  first attended check in the manual test pass.

Progress on 2026-09-25, live parameter changes (found while wiring task 4):
a `graph.commit` on a running session only restarted the fake runtime; no
native graph was republished, so saved slider changes (Gain included) were
not heard until Stop/Play. Now `republish_running_native_graph` recompiles
the saved graph into the attached adapter: endpoint and process-loopback
routes use `activate_native_graph`, and the multi-input Mixer uses
`RealtimeMixerFanout::replace_graph`, which swaps between pumps, requires the
same generation, sources and branches, and keeps the privacy latch. The commit
result reports `activation.native` = `applied` or `restartRequired` (reason);
the contract type was updated. UI: `finishGraphSave` reports whether a save
reached the playing audio, and while a session runs, parameter-only edits
(`isParameterOnlyChange`) auto-save after a 400 ms pause, so sliders act
live. Topology edits still need Save, then Stop/Play. Verification: control
186 (new `native_preparation_defaults_to_the_next_start_generation`), UI 331.

Progress on 2026-09-25, task 6 (Input Switch, DSP-14): new `inputSwitch`
kind (`selected` a/b, `fade` normal/slow; ports a, b, out). The multi-input
compiler accepts it as the convergence node (exactly two inputs, one per
side, unity input volume). `InputSwitchState` holds an equal-power position
advanced per block, `MixerStage::mix_input_ramped` applies per-frame gains,
and `replace_graph` carries the position into the recompiled graph so a live
switch crossfades. The linear path passes the single live input only when its
side is selected. UI: library entry, A/B node buttons (Shift = slow fade), and
route detection for Play. Verification: engine 130 (new
`input_switch_passes_one_side_and_crossfades_after_a_live_switch`: equal-power
midpoint and full B after 0.5 s); UI draft tests 36.

Progress on 2026-09-25, task 7 (Denoise, Speech Denoise; DSP-15/16):
`audiorouter_dsp::spectral` adds a self-contained radix-2 FFT (so no
`Cargo.lock` change), an STFT framework (1024/256, sqrt-Hann WOLA, 1024-sample
latency), `Denoiser` (learned profile, over-subtraction, smoothed gains,
remaining floor) and `SpeechDenoiser` (minimum statistics with ×2.5 bias
compensation and a 20-frame warm-up, decision-directed Wiener gain, speech-band
floors). Profiles serialize as 64 log bands × 1 dB (128 hex characters).
Engine: `ProcessingStage::Denoise` and `SpeechDenoise` (per channel,
`RealtimeDsp`) and `noise_profile_for_node` from the runtime graph through
the published processor, fan-out graph and multi-input worker. Control:
discovery parameters and an optional `noiseProfile` in node telemetry while
learning (schema and contract). Domain: `denoise` (reductionPercent,
floorPercent, learning, noiseProfile) and `speechDenoise` (strengthPercent),
both with `latency_samples` 1024. UI: library entries, node faders and
status, and a "Learn noise / Stop learning and keep profile" inspector flow
(learning and profile saves apply live). Measured in unit tests: learned
profile −18.6 dB on white noise with a 1 kHz tone kept; Speech Denoise
−15.5 dB on steady noise with a syllable-gated tone kept at 99 %.
Verification: dsp 42, engine 131, control 186, UI 332.

Progress on 2026-09-25, task 8 (FIR Filter, DSP-17):
`audiorouter_dsp::spectral::Convolver` performs uniformly partitioned
overlap-save convolution (512/1024, frequency-domain delay line, IR at most
96 000 samples, unit-energy normalization, wet/dry mix with an aligned dry
delay). New `firFilter` kind (mediaId, fileName, wetPercent, gainDb;
latency 512). The engine builds a `ProcessingStage::Fir` from decoded media
(left/right from IR channels 0/1) and passes audio through when no IR is set.
Control: media loading is extracted into `session_audio_media` (Audio File
and FIR); a new `compile_mixer_fanout_session_with_plugins_and_audio`
carries media into Mixer input and post-Mixer chains, used by multi-input
prepare and live republish. UI: library entry, node face (IR name, Mix
fader), and an impulse-response picker in the inspector. Chunked upload is
extracted into `ui/src/audioUpload.ts`, shared with Audio File. Verification:
dsp convolver tests (match direct convolution within 1e-3; impulse delay;
50 % mix; non-finite safety); engine 131, control 186, UI 333.

Progress on 2026-09-25, task 9 (Time Shift, DSP-18): `audiorouter_dsp::timeshift`
(`TimeShift` ring up to 120 s, a lock-free `TimeShiftTransport` command
mailbox and status, bounded rewind, 256-sample fade-in after jumps). Engine:
a `TimeShiftState` shared through a compile-time registry keyed by session
and node (weak references; never locked on the audio thread), so a live
recompile reuses the recording. Mixer chain sessions are named
`<session>::<chain>` to share the key. `ProcessingStage::TimeShift` handles
it, and `time_shift_state` gives control access. Control/API: new
`timeShift.transport` (pause, resume, back, forward, live, status) with
SessionControl permission, schemas, description, contract, API-reference
row, and API_METHODS count 93. UI: library entry, node transport buttons with
live/delayed/paused status, and a once-per-second status poll while running.
Verification: dsp timeshift 2, engine 132 (new
`time_shift_buffer_survives_recompilation_and_obeys_transport`), domain 68,
control 186, UI 333.

Follow-up on 2026-09-25 (UI-01/05, right-sidebar controls): the Advanced and
MCP tabs (and the other sidebar tabs) used the global bevelled-gradient
input/select style, native white dropdown lists, an unstyled file picker, and
inline label/field/button flow. `ui/src/styles.css` now gives text-like
inputs, selects and textareas inside `.right-workbench` the flat rounded
Properties style, with a dark `color-scheme` and option colors, in the dark,
light and high-contrast themes. Checkboxes, sliders and file inputs are
excluded; the file picker gets its own themed button. Labels that wrap a
field stack the caption above a full-width field. Plugin-scan placeholders
rendered doubled backslashes (`C:\Plugins`) because JSX attribute strings
do not unescape; fixed. Verified with Edge/Playwright screenshots of the
expanded Advanced and MCP tabs against the Vite server (all text fields and
selects computed flat, 9 px radius, dark scheme). UI 333 passed.

Contract drift found and fixed: the nine new tools were node kinds but absent
from the processor catalog (`processors.list`), so `check:drift` failed.
`processor_catalog` now adds them through `catalog_entry`, which takes
parameters and latency from the node registry (catalog limit 7 → 32; control
test counts updated). `npm run check:drift` passed (93 methods, 30 node kinds,
16 processors, 21 event categories); control 186.

VST workflow review: plugins are added from the canvas shelf's
"Plugin (VST2/VST3)" picker (scan a folder → add or insert the verified result
→ edit worker-described parameters in Properties), not from the Tools tab.
Every `plugins.*` method requires `PluginScan`, which both desktop-shell
grants deliberately withhold (2026-09-22 decision), so scanning from the app
is refused. Enabling it needs an explicit user authorization decision.

## VST workflow (2026-09-25, user-approved scope: all four items)

User decision (2026-09-25): allow plugin scanning in the desktop app by default
and implement (1) the scanning permission, (2) Tools-tab plugins with a
standard-folder scan and remembered results, (3) the vendor editor window, and
(4) saving plugin state with the route.

- **(1) Permission — pending, blocked.** The edit adding
  `PermissionScope::PluginScan` to `ClientGrant::for_desktop_shell()`
  (`crates/control/src/lib.rs`) and to the explicit device-admin opt-in grant
  (`src-tauri/src/main.rs`) was refused by the agent's auto-mode safety
  classifier as a permission grant. Per policy it was not worked around. The
  user must apply it (or approve the edit). Until then, every `plugins.*`
  method, including those below, is denied in the desktop shell. The CLI with
  an enrolled role is unaffected.
- **(2) Tools tab — done.** Storage persists scan inventories
  (`control_settings` key `pluginInventories`, ≤ 4 MiB, malformed state ignored),
  and control restores them at startup. A side effect fixes a latent defect:
  plugin preparation requires a matching remembered scan, so saved plugin
  routes previously failed after every app restart until a rescan. New
  read-only `plugins.inventory` (PluginScan). UI: a "Plugins (VST2/VST3)"
  group in the Tools tab (`PluginToolsGroup`; `pluginCatalog` lists supported
  x64 plugins, deduplicated, sorted), **Scan standard folders**
  (`STANDARD_PLUGIN_FOLDERS`), **Scan another folder…** (existing picker), and
  a refresh when the picker closes.
- **(4) State — done.** `PluginRuntimeBridge` has a request mailbox served by
  the plugin runtime thread between frames (never the audio callback):
  `save_state`, `open_editor`, `close_editor`. New `plugins.saveState` writes
  the asset with `write_state_asset` under `<database dir>/plugin-states/` and a
  `plugin_states` record keyed by the plugin SHA-256; the UI sets node
  parameter `stateId` (domain-validated). `prepare_plugin_stages` restores it
  (`read_state_asset`, hash-verified) before the bridge starts and fails
  visibly if it is missing or belongs to another binary. Live bridges are
  tracked weakly by (session, node).
- **(3) Editor — implemented, VST2 only.** The VST2 editor runs a second plugin
  instance inside the worker. It now starts from the processing instance's
  state (`Open` carries state) and hands its state back on `Close`, which the
  worker applies to the processing instance. New `plugins.openEditor` and
  `plugins.closeEditor`; the control plane issues the parent-window
  authorization from a per-run key. The shell's `open_plugin_editor` command
  creates a resizable top-level host window on its own message-loop thread
  (`src-tauri/src/plugin_editor_windows.rs`); closing it calls `closeEditor`
  first. UI: Properties → **Open plugin editor** / **Save plugin settings**.
  VST3 `IPlugView` editors remain unsupported, with a clear message.
- **Evidence:** new opt-in fixture test
  `runtime_bridge_saves_state_and_opens_the_editor_in_a_pumping_parent`
  passed with ReaEQ (`reaeq-standalone.dll`). This is the first successful
  VST2 editor open/close in this repository. The earlier "non-returning
  editor" results came from containment tests that deliberately block the
  parent's thread; a cross-process child window needs its parent thread to
  pump messages. The existing ReaEQ fixture tests still pass (load/process,
  editor-thread bound, supervised timeout kill). Three other opt-in tests fail
  when pointed at ReaEQ because they require dedicated fault, non-finite and
  chunk fixtures by file name; this is not a regression. Also passing:
  plugin-host 71 + 13, control 187, storage 94, domain 68, UI 334, drift
  (97 methods), and shell `cargo check`.
- **Limits and risks:** editor edits reach the audio when the editor window
  closes (no live sync while open). `plugins.openEditor` trusts a caller that
  names a window owned by an existing process; the worker re-checks
  ownership. It is local-only and PluginScan-gated. The editor window has not
  been exercised attended in the shell.
- **Next action:** the user applies the PluginScan grant, then an attended
  test: scan standard folders → add ReaEQ → play → open editor → change →
  close → save settings → restart route.

Follow-up on 2026-09-25 (user report: VST picker field styling; scan denial
shown as plain text):
- Global form-control style: `styles.css` now separates the base rule. Buttons
  keep the bevel; `input, select, textarea` get the flat app field style at
  element specificity (dark `color-scheme`, themed options, focus, disabled,
  placeholder, range reset, file picker and `::file-selector-button`), with
  light and high-contrast variants. The redundant `.right-workbench` field
  overrides were removed; only the sidebar caption-above-field layout remains.
  Verified with Edge/Playwright: the picker markup (text, select, textarea,
  file) computes flat, 9 px, dark; canvas, top bar and Tools tab render
  correctly.
- AGENTS.md gains a "UI conventions" section (app field style, stacked
  captions, three-theme visual check) and two validated lessons (this user
  preference; Bash backslash collapse).
- Panel status lines: new `PanelMessage` renders error-phrased messages
  (`actionMessageTone`) as a red `role="alert"` block. Used by 9 panel
  messages including the plugin scan panel, 4 other status lines, and the
  plugin editor hint. `formatUiError` turns `permission denied: <Scope> …`
  into a readable sentence naming the missing permission. The disconnected
  top-bar message no longer shows a doubled period.
- The user's scan returned `permission denied: PluginScan`, confirming that
  the pending grant (VST item 1) is the blocker.
- Verification: UI typecheck; UI tests 335 (new permission-message test);
  backend tests 64.

VST item (1) resolved on 2026-09-25: the user explicitly instructed the agent
to apply the PluginScan grant ("You do it"). `PermissionScope::PluginScan` is
now in `ClientGrant::for_desktop_shell()` (with a control test assertion) and
in the explicit device-admin opt-in grant in `src-tauri/src/main.rs`. The
desktop shell still withholds `Capture`, and `DeviceAdministration` remains
opt-in. Scans read metadata only; plugin code runs only in isolated workers.
The AGENTS.md lesson on shell grants was updated. Verification: control 187
passed; the shell was rebuilt and relaunched with the standard recipe
(PID 56608). Attended check pending: Scan standard folders from the Tools tab.

Defect on 2026-09-26 (first attended plugin save): after the PluginScan grant,
saving a route with a scanned plugin failed with "plugin placeholder requires
a current explicit scan result", and `plugins.parameters` also failed.
Cause: `appendPluginPlaceholderNode` stored `identity.binaryPath`, which is
canonical with a verbatim prefix (`\?\C:\...`), while the save validation,
plugin preparation and parameter lookup matched remembered scans by the
scanned `entry.path` (`C:\...`). No plugin could ever match. This path was
never exercised in the shell because scanning was denied until 2026-09-25.
Fix: the UI stores the scanned `entry.path`. Control matches with
`scan_entry_matches_path`, which accepts the scanned or canonical binary path,
ignores the verbatim prefix and case, and still checks the fingerprint, so
existing nodes carrying the canonical path also work. Revalidation now
inspects the entry's own scanned path under its root. New plugin nodes are
also named after the plugin file (was the vendor, for example "Cockos 1") and
added enabled (they were disabled, which silently bypassed the plugin).
Verification: control 188 (new
`scan_entries_match_the_scanned_or_canonical_binary_path`) and UI 335 with
updated expectations; shell rebuilt and relaunched.

Pipe collision on 2026-09-26: while relaunching the debug shell after the
plugin-path fix, a user-started release shell was found running (PID 76704,
`src-tauri/target/release/audiorouter-shell.exe`, built 2026-09-25 23:04,
started 23:21). The new debug instance (PID 85268) could not create the
default control pipe ("Access is denied. (0x80070005)"). Its backend retried,
entered durable safe mode in the test database and exited, and its UI would
have forwarded to the release backend. The debug instance was stopped
immediately. The release instance was not touched: a first Stop-Process
attempt against it failed without effect before its identity was known. The
test database safe mode was cleared with `audiorouter-cli recovery
clear-safe-mode` (`safeMode: false`). A read-only check of the user's real
database found `recoverySafeMode = false`. AGENTS.md gained a lesson: check
for another shell before launching, and ask rather than stopping a
user-started instance.

Release build for attended testing on 2026-09-26: after the user closed their
release instance (no other `audiorouter-shell` running, checked first),
`cargo tauri build --no-bundle` rebuilt `src-tauri/target/release/audiorouter-shell.exe`
with the embedded production UI, and the release `audiorouter-plugin-worker.exe`
was rebuilt beside it. The build was launched against the disposable test
database with `AUDIOROUTER_ALLOW_DEVICE_ADMIN=1` (PID 60068); its stderr shows
only the device-admin grant (no pipe errors). This is a local build for
testing, not an M08 release: no installer, checksums or release notes.

Attended feedback on 2026-09-26 (release build):
1. **Test Signal with Discord and mic in one Mixer was refused by Play.** The
   multi-input Mixer now supports generator sources. A Test Signal or Audio
   File becomes the first stage of its input chain (`compile_processor_chain`
   skips the synthetic source for a chain that starts with a generator), and
   its native input is a new `generated` binding (`NativeMultiInputSourceBinding::Generated`
   → `MultiInputCaptureSource::Silence` pacing, zero entry matrix). Control
   registers Test Signal and Audio File transport handles for the multi-input
   worker (`register_multi_input_generators`, after prepare and live
   republish). The prepare request/result schemas and contracts accept
   `generated`. UI Play sends generated bindings; only endpoint-loopback and
   virtual sources remain unsupported alongside applications. Engine test:
   `mixer_generates_a_test_signal_input_alongside_a_live_source`.
2. **The ReaComp editor could not be opened.** This was a consequence of (1)
   (Play never started, and the editor needs a playing route) and of (3)
   (parameter loading failed).
3. **Console windows showing `0x800700E8` on each plugin worker launch.** The
   windowless release shell started the console-subsystem worker, Windows
   allocated a new console hosted by the default terminal, and the launch
   failed. Workers now start with `CREATE_NO_WINDOW`
   (`crates/plugin-host/src/lib.rs`) and communicate only over piped stdio.
   Debug builds never showed it because they own a console the worker
   inherited.
Verification: engine 133, control 188, windows-audio 93, plugin-host 71 + 13,
UI 335, drift passed. The release app and worker were rebuilt and relaunched
(only the agent's own instance was running and was stopped first). A live
worker launch from the windowless shell is not yet confirmed.

Stale release UI on 2026-09-26: the user still saw the old "combine
applications and one input device only" message. `ui/dist` was dated
2026-09-25 23:03 and contained the old text: neither `cargo tauri build
--no-bundle` run today refreshed it, although both reported success. The
tauri output never showed Vite output, and the cause inside the tauri run was
not isolated. Running the identical `beforeBuildCommand` directly succeeded.
The UI was rebuilt, then the release (dist 10:45:48 contains the new text and
not the old; binary 10:46:21), and relaunched as PID 60776
(stderr: grant line only). AGENTS.md gained a lesson: verify the embedded UI
after each release build.
