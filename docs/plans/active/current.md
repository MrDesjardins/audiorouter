# Active plan — VB-Cable-first completion

Status: active. Rewritten 2026-09-27 as a current-state plan. The
append-only log it replaces (2026-09-17 → 2026-09-26: every decision,
defect, experiment and superseded handoff) is archived verbatim as the
[execution log](../archived/2026-09-26-vb-cable-first-execution-log.md).
Add new entries under "Log" below, and keep the sections above it current.

## Objective and scope

Finish AudioRouter's non-driver scope. Audio routes use existing endpoints
(VB-Cable, Voicemeeter, physical WASAPI devices) and are driven by one
backend-owned graph. The desktop UI, CLI and MCP are adapters over the same
versioned API. **Permanent scope decision (user, 2026-09-19):** no
AudioRouter-owned driver, PortCls endpoint or production signing.
VDEV-01/03/09 and SEC-08 stay normative only for a possible future funded
track ([future plan](../future/M03-driver-signing.md)).

Requirement families in scope: PROD, ARCH, GRAPH, CAP, DSP, REC, PLUG, UI,
API, AUTO, STATE, SEC (non-driver), NFR, QUAL, ENG, as mapped in
[delivery traceability](../../spec/15-delivery.md#requirement-traceability).

## Where things stand

Legend: **Implemented** = code exists with automated tests. **Qualified** =
measured on real Windows devices (evidence linked). **Open** = required
evidence is missing. Nothing here is a release claim: M08 is not done.

| Milestone | Implemented | Qualified on Windows | Open |
| --- | --- | --- | --- |
| [M00](../../milestones/M00-feasibility.md) feasibility | WASAPI probes, calibrated loopback tooling | [WASAPI probe](evidence/M00-wasapi-probe.md); NFR-01 p95 ≈155–186 ms (target revised to ≤250 ms, DEC-14) | Owned-driver feasibility (permanently out of scope) |
| [M01](../../milestones/M01-contracts.md) contracts | Domain, storage, authorization, CLI/MCP parity | [M01 evidence](evidence/M01-contracts.md) | Final release acceptance only |
| [M02](../../milestones/M02-audio-engine.md) audio engine | Realtime graph, capture/render adapters, multi-input/many-output paths, backend audio service, Network Send/Receive (GRAPH-16) | Guarded routes ([M02](evidence/M02-audio-engine.md)); **continuity 0 glitches for 30–60 s on four route shapes** ([audio continuity](evidence/2026-09-26-audio-continuity.md)); NFR-02 p95 ≈97–115 ms (≤160 ms, DEC-15) | Clock-drift correction between independent devices; endurance/soak |
| [M03](../../milestones/M03-virtual-routing.md) virtual routing | Exact endpoint identity, VB-Cable/Voicemeeter routes, rebind | [M03](evidence/M03-virtual-routing.md) | Owned virtual devices (out of scope) |
| [M04](../../milestones/M04-effects-recording.md) effects/recording | 17 built-in processors incl. pitch; recorder (WAV/FLAC/MP3), library | Synthetic DSP vectors; [feature confidence](evidence/2026-09-26-feature-confidence.md) | Attended/long-duration recording on real devices |
| [M05](../../milestones/M05-visual-editor.md) visual editor | Canvas, Properties/Tools, live flags, timing, three themes | 65 browser E2E + 345+ UI unit tests; Edge visual review | Attended Narrator, 200 % scaling, first-run, live drag/drop; UI-15 attended edge activity |
| [M06](../../milestones/M06-plugins-pitch.md) plugins/pitch | VST3 worker, x64 VST2 worker, shared adjacent-VST2 chain, editors, saved state | ReaPlugs chain live ([shared chain](evidence/2026-09-26-shared-vst2-chain.md)); 60 s glitch-free with the user's saved ReaPlugs nodes | Rights/sandbox review, multi-vendor matrix |
| [M07](../../milestones/M07-automation-recovery.md) automation/recovery | MCP, persistence, crash journal, safe mode, sign-in helper | [M07](evidence/M07-automation-recovery.md), [OS transitions](evidence/M07-os-transitions.md) | OS power/session delivery and native reopen (attended) |
| [M08](../../milestones/M08-release.md) release | Unsigned artifact preparation | [M08](evidence/M08-release.md) | Clean-checkout release run, installer, signing, clean machine, traceability gaps (CAP-13, GRAPH-15) |

## 2026-09-26/27 overnight session — outcome

User request: commit everything, then verify features, above all sound
quality (crackling), clean up the Markdown, and add network send/receive
tools once everything else is in order. Full permission; the user tests
by hand in the morning.

- Committed the pending work (`cf50d8ff`). Build outputs `ui/dist-review-*`
  are now ignored rather than committed.
- **Crackling root cause found and fixed**
  ([evidence](evidence/2026-09-26-audio-continuity.md)). Each processed
  quantum was written twice into the output ring: about 275 repeated or
  skipped 128-frame blocks per second. Audio also depended on UI-timed
  RPC pumps, and the render device had no jitter margin. Measured after
  the fixes: 0 glitches over 30–60 s for a direct route, a five-processor
  chain, the single-endpoint worker, and the user's saved ReaPlugs chain.
- **Test Signal and Audio File were chopped** (~100 discontinuities/s)
  because generated sources were not paced in real time. They now are:
  30 s clean.
- **Network Send / Network Receive tools added** (user request, see the log
  and [quickstart how-to](../../operations/quickstart.md#stream-audio-to-another-computer-network-send-network-receive)).
  Verified on one machine over UDP 127.0.0.1: 2 × 30 s, 0 packets lost,
  0 glitches.
- **Bypassed plugins no longer block Play.** The user's saved session has
  ReaEQ bypassed, which made `nativePaths.prepare` fail. Regression test
  added.
- Plugin bridge and worker threads now use MMCSS "Pro Audio".
- Status line now shows output underruns and late backend audio-service
  gaps.
- **Recording on multi-path sessions fixed.** Before, stop failed and only
  about 21 ms was kept. Routes with a Recorder branch now use the
  multi-path worker. Live: a 15 s WAV with 0 glitches
  ([evidence](evidence/2026-09-26-audio-continuity.md), finding 7).
- The render jitter cushion is adaptive: 10 ms, growing 5 ms per real
  underrun, up to 40 ms.
- Checks (final, 2026-09-27): `cargo test --workspace` 0 failures; UI vitest
  353 passed; browser E2E 67/67; contract drift, documentation and M08
  traceability (175 IDs) all pass.
- Artifact for the attended test (built 03:10 local on 2026-09-27; UI
  bundle `index-DAgNSICR.js` confirmed embedded; shell newer than
  `ui/dist`):
  `C:\code\audiorouter\target\patrick-main-release-2\release\audiorouter-shell.exe`
  SHA256 `67D6E97696984209360CF7599895267AA051773BF9CE6DCAC75517727CFA9FEF`.
  Worker beside it: SHA256
  `275190960CDF8E6ADEF5BEEFF596A474B084CE72494D6FF7399D9DE683DDBCA8`.
- Since about 23:00 the machine has shown occasional ~10 ms gaps on the
  virtual-cable harness. An earlier commit that measured clean at 22:40
  shows the same rate, so this is environmental; see the
  [evidence](evidence/2026-09-26-audio-continuity.md#environment-observation-2300-onwards).

## Open defects and known gaps

1. Clock drift is not corrected in code (`DriftController` is unused). On
   the user's devices it measured under 1 ppm (flat 17 ms queues over
   3 minutes, [drift survey](evidence/2026-09-26-audio-continuity.md#clock-drift-survey-of-the-users-saved-session)),
   so it is low priority here. Other hardware can drift more.
2. Latency does not shrink by itself after a stall. The output queue can
   stay up to about 70 ms until Stop/Play.
3. Signal timing covers multi-input routes only. A plugin's own reported
   VST latency is not added to its queue time.
4. Plugin failure reasons are not shown in the UI (only the `failed`
   state).
5. `list_commands_use_discovery_and_do_not_fake_devices` in `audiorouter-cli`
   is recorded as expecting 7 processors while the catalog has 16. The
   workspace run passed tonight, so recheck before editing.
6. Workspace-wide `cargo fmt`/clippy drift, and missing CAP-13/GRAPH-15 rows
   in delivery traceability (M08 blockers).
. Network tools: never run across two physical computers or over Wi-Fi.
   The stream is unencrypted and authenticated only by source address
   (SEC-13, LAN only). The receiving PC's firewall prompt has not been
   observed. The receiver's drift correction is not measured on real clocks.
   OS transitions.

## Next actions, in order

1. **User (attended):** close any running AudioRouter, then launch only the
   artifact above:
   ```powershell
   $env:AUDIOROUTER_DATABASE = "$env:LOCALAPPDATA\AudioRouter\state.sqlite"
   $env:AUDIOROUTER_ALLOW_DEVICE_ADMIN = "1"
   & "C:\code\audiorouter\target\patrick-main-release-6\release\audiorouter-shell.exe"
   ```
   Select Patrick Main Session and press Play. Also try a Test Signal
   route. Listen to the voice and
   game with the window minimized and restored. The status line should
   show no "output underrun" or "late audio service gap". If clicks
   remain, note roughly how often they occur (drift gives rare, regular
   clicks).
2. **User (attended), when a second PC is available:** Network Send on the
   gaming PC to Network Receive on the streaming PC, per the quickstart.
   Report the receive status line (packets, lost, gaps).
3. Low priority: live clock-drift correction for cross-device paths.
4. M05 attended accessibility/scaling review; M08 clean-checkout release
   preparation.

## Validation commands

| Area | Command |
| --- | --- |
| Rust | `cargo test --workspace --locked` |
| UI | `cd ui; npx vitest run --configLoader runner --exclude "e2e/**"`; `npm run build` |
| Browser E2E | `cd ui; npm run e2e` |
| Contracts | `cd contracts; npm run typecheck; npm run check:drift` |
| Docs | `powershell -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\docs.ps1` |
| Audio continuity (live, VB-Cable) | `AUDIOROUTER_LIVE_CONTINUITY=1 cargo test -p audiorouter-transport --test live_audio_continuity -- --ignored --nocapture` ([options](evidence/2026-09-26-audio-continuity.md#method)) |
| Saved-session drift (live, privacy-muted, DB copy) | `AUDIOROUTER_DRIFT_DATABASE=<copy> cargo test -p audiorouter-transport --test live_audio_continuity live_saved_session_output_queue_drift -- --ignored --nocapture` |
| Release shell | `cargo build --manifest-path src-tauri/Cargo.toml --release --features custom-protocol`, then confirm the embedded bundle (AGENTS.md lessons) |

## Decisions in force

- No AudioRouter-owned driver (2026-09-19, permanent).
- DEC-14 NFR-01 ≤250 ms p95; DEC-15 NFR-02 ≤160 ms p95 (2026-09-21).
- Adjacent compatible VST2 plugins share one isolated worker. A fault
  silences the whole group (2026-09-26).
- Live flag changes apply without stopping Play when topology is prepared
  (2026-09-26).
- Network audio (user request, 2026-09-26): UDP on the LAN, uncompressed
  48 kHz float32, one quantum per datagram. Addresses are IP literals only;
  the receiver accepts one sender address. No encryption. Streaming exists
  only while a session with a network node is prepared (GRAPH-16, SEC-13).
- The desktop shell grant includes Record (2026-09-22) and PluginScan
  (2026-09-25). DeviceAdministration requires the explicit
  `AUDIOROUTER_ALLOW_DEVICE_ADMIN=1` opt-in.

## Risks and rollback

- Backend audio service: reverting `serve_control_connections_forever_with_grant`
  to the single-threaded loop brings back UI-dependent audio. Keep the 5 ms
  UI pump fallback for backends that do not report `audioService.active`.
- The render cushion and headroom add up to 10 ms of steady latency, bounded
  by about 70 ms. Revert with `RENDER_JITTER_CUSHION_FRAMES = 0` and
  `SharedRender::open` in the fan-out.
- Saved sessions and database formats are unchanged tonight.

## Log

New dated entries go here (objective, requirement IDs, work, verification,
result, next action). Keep them short, and move settled facts into the
sections above.

### 2026-09-26 — Network Send / Network Receive (GRAPH-16, SEC-13)

- Objective (user): a gaming PC sends audio to a streaming PC over IP. One
  send tool whose property is the destination IP. One receive tool (a
  source) whose property is the sender's address.
- Implemented: domain kinds and validation; `network_audio` (wire format,
  pooled sender tap plus I/O thread, receiver thread with sender filter,
  concealment, and a paced jitter buffer with ±1-frame drift correction);
  native-path source and branch wiring; node telemetry; UI library, editor,
  canvas card, and docs (spec, quickstart, privacy, API reference).
- Verification: 4 network unit tests, including loopback tone continuity
  (0 discontinuities in 3 runs), sender filtering, and gap concealment.
  Live CABLE → Network Send → UDP 127.0.0.1 → Network Receive → CABLE-B:
  2 × 30 s, 0 glitches, 11,640 of 11,640 packets. UI: 6 unit tests,
  2 E2E real-backend cases, three-theme bounds and screenshots.
  Workspace Rust, UI unit, E2E (67) and contract drift all pass.
- Also fixed on the way: generated sources (Test Signal, Audio File) were
  not paced (chopped audio). Card status text was unreadable in the light
  theme.
- Rollback: remove the two node kinds and `network_audio`. Saved sessions
  without network nodes are unaffected.
- Next: the attended two-PC test (see Next actions).

### 2026-09-27 — Live Bypass toggles stalled a playing route (GRAPH-05/08, UI-04)

- Report (user): toggling Bypass while playing did nothing; Stop then Play
  was needed.
- Reproduced live on a privacy-muted copy of the saved session, and with the
  continuity harness (`AUDIOROUTER_CONTINUITY_TOGGLE`). The commit was
  applied, but it started a new runtime generation. The multi-path worker
  kept its prepared generation, so the backend audio service and the pump
  rejected it as stale and stopped pumping.
- Fix: the worker serves both the prepared and the live-applied generation,
  and the UI adopts the committed generation. Bypassed plugins are prepared
  in the graph with the bridge passing audio dry. `graph.commit` lets audio
  advance between the durable save and the graph rebuild, so the maximum
  service gap during toggles is 6–8 ms (it was 18.8 ms).
- Verification: all five effects toggled both ways while playing, all
  applied, with pumping continuing on both generations
  (`live_saved_session_bypass_toggles_apply_while_playing`). A Gain at
  −12 dB follows every toggle with zero time jump on both workers. A
  ReaEQ toggle adds no plugin misses. Workspace Rust tests (0 failures),
  UI tests 353, E2E 67/67, and drift checks pass.
- Known: a bypass switch is instantaneous, not crossfaded, so it can click
  slightly on loud material.
- Artifact: `target/patrick-main-release-3/release/audiorouter-shell.exe`,
  built 11:05, bundle `index-Cbk6z3dN.js` embedded. It is a new folder
  because the user's `release-2` shell was running.

### 2026-09-27 — VST2 editor showed a copy that received no audio (PLUG-03/05)

- Report (user): ReaFIR's editor did not pick up audio. After closing it,
  the node reported 18 missing output blocks and 10 dropped input blocks.
- Cause: the worker opened the editor on a *second* plugin instance loaded
  on its UI thread, and copied its state to the processing instance on
  close. The editor therefore never saw audio (ReaFIR's noise profile and
  analyser need it), and edits were not heard until close. Open and close
  also waited for the plugin's window on the worker's audio loop, so blocks
  were missed and dropped meanwhile.
- Fix: `Vst2EditorAccess` gives the UI thread the editor opcodes of the
  processing instance (the VST2 threading model: GUI thread plus audio
  thread). Open and close are requested without waiting for the window, and
  the editor thread closes the editor and is joined (bounded to 2 s) before
  the plugin can unload. The editor acceptance tests were rewritten: the
  editor opens the processing instance without blocking its caller, and the
  worker keeps processing while an editor hangs.
- Verification: `m06-vst2-editor.ps1` passes for all 6 ReaPlugs fixtures;
  the shared-chain acceptance, the pumping-parent editor/state test, and
  workspace Rust tests (0 failures) all pass.
- Artifact: `target/patrick-main-release-4/release/audiorouter-shell.exe`,
  built 12:08. It is a new folder because `release-3` is running.

### 2026-09-27 — UI polish list from attended review (UI-01/04, persistence)

- Report (user): node colors ignored the theme; long Input Device note;
  network tools lacked icons; tools unsorted; "Observed runtime" unclear and
  unpolished; Setup and Devices tabs confusing; no session file to back up or
  move a setup; Advanced EQ status line moved the sidebar; sidebar too narrow;
  number fields could not be retyped or take negative values.
- Done (committed earlier today): themed node cards, shorter notes, unique
  network icons, alphabetical tools, "Live readings" card, fixed-height
  inspector status lines, resizable sidebar, `NumberField` for all precise
  values. E2E: `inspector-stability.pw.ts`, `sidebar-resize.pw.ts`.
- Session files: `sessions.exportFile` / `sessions.importFile` write/read the
  versioned `.audiorouter` bundle, now carrying the imported audio and plugin
  states the session references. Import never replaces a session (used IDs
  become `-imported-N`, name "(imported)"). An existing file is replaced only
  with `replace: true` after the Windows Save dialog confirmed it, staged
  beside it first. The shell adds native Save/Open dialogs
  (`src-tauri/src/session_file_dialog.rs`). Tests: storage
  `session_file_carries_imported_audio_and_plugin_state_to_another_database`,
  control `session_file_export_imports_on_another_database_without_replacing_sessions`,
  E2E `session-file.pw.ts`.
- Setup/Devices decision: Setup is "Set up this PC" (app-wide status, device
  list, other-app guidance, start at sign-in). The Devices tab is removed;
  its controls live in Advanced → Troubleshooting. The connection form stays
  (keyboard access) in Advanced → Connect nodes without dragging. Play reads
  a single route's devices from its nodes first. A generated-only route
  (Test Signal/Audio File → Output) with no chosen input, or saved with its
  output chosen, runs on the multi-path worker, so no input device is needed.
- Verification: vitest 358/358 in `src` (the Playwright file
  `e2e/audio-tools.spec.ts` that vitest also collects fails as before);
  Playwright 88+ pass; control/storage/domain/CLI Rust tests pass; contract
  drift and docs validation pass; three-theme screenshots reviewed.
- Not yet verified live: a generated-only route through the multi-path worker
  on the release shell (covered by the continuity harness `testSignal` mode
  on that worker, but not re-run for this UI change).

### 2026-09-27 — Plugin settings were lost on Stop/Play (PLUG-04)

- Report (user): ReaFIR settings made in its editor survived reopening the
  editor but were gone after Stop and Play; after relaunching, every
  plugin's settings were at defaults.
- Cause: editor edits lived only in the running plugin. They were stored
  only by an explicit Save plugin settings plus Save, which the user never
  had reason to do (no `plugin-states` folder existed). ReaComp and ReaGate
  could not be saved at all: they lack VST2 chunk support
  (`vst2StateSave:StateUnsupported`).
- Fix: the backend records each plugin node's latest captured state
  (`plugin_node_states` table). It captures on editor close, Save plugin
  settings, and before Stop (including tray Quit). Play restores that
  capture first, then the node's `stateId`. Automatic captures
  (`plugin-autostate-*`) replace the node's previous one, so they do not
  accumulate. Duplicated sessions and session files keep them. VST2 plugins
  without chunks save/restore a parameter bank.
- Verification: new live test `live_plugin_settings_survive_stop_and_play`
  (copy of the user database, privacy-muted) passes for ReaFIR, ReaEQ,
  ReaComp and ReaGate. ReaEQ reloads one stored frequency with a last-bit
  rounding difference; the test tolerates two differing bytes. About 15 runs
  left exactly 4 state files. Storage 96, control 196, plugin host, CLI,
  transport and UI 358 tests pass.
- Artifact: `target/patrick-main-release-5/release/` (shell, plugin worker,
  CLI), built 16:28 with `custom-protocol`, embedding UI bundle
  `index-CYp4WBYo.js`. A new folder because `release-4` was running. Rebuilt
  17:09 with FIR Filter Hz, readable labels and name fields (bundle
  `index-Zg_vPOaz.js`).

### 2026-09-27 — Names with spaces, readable settings, FIR Filter Hz (UI-04, GRAPH, DSP)

- Report (user): node names rejected spaces; FIR Filter showed `wetPercent`
  with no explanation; asked for a ReaFIR-like learned per-frequency noise
  gate with a live spectrum, named "FIR Filter Hz".
- Names: every keystroke was trimmed, so a typed space vanished. New
  `TextField` keeps the typed text and stores the trimmed name (node name and
  session rename). Tests: `TextField.test.tsx`.
- Labels: `parameterText.ts` gives every built-in setting a label, a
  one-line explanation and readable choices; Properties shows the tool's
  description and name instead of the raw kind. Accessible names use the
  labels (`Wet mix precise value`).
- FIR Filter Hz: new node kind `spectralGate` (`spectral-gate@1`), 33 kinds,
  17 processors. DSP `SpectralGate` (STFT, 64 log-spaced bands): peak-hold
  learning, per-band gate with `thresholdDb` (−20…20, default 3) and
  `reductionDb` (0…80, default 40), open at once and close over ~20 ms.
  Live levels are published lock-free (`SpectrumTap` atomics), so reading the
  spectrum can never make the audio thread skip a block. Telemetry adds
  `spectrum { levelsDb, bandFrequenciesHz }`; the learned profile reuses
  `noiseProfile`. The control crate's `json!` recursion limit was raised to
  256 for the larger discovery schema.
- Verification: DSP tests (learn, gate noise, pass louder tone, band
  round-trip), engine tool-route test with the new kind, UI editor tests,
  vitest 364, Playwright suite, contract drift (33 kinds, 17 processors),
  three-theme screenshots of the editor with a synthetic spectrum.
- Live continuity (30 s, VB-Cable → CABLE-B): inconclusive this evening.
  FIR Filter Hz 2 glitches per run (4 runs); plain Gain 3–4 per run on the
  current build; this morning's commit `65b7c704` built in a worktree gave 0
  and then 5. The reference tone stayed clean. This is the recorded
  environment-dependent gap pattern, not a regression of this change; rerun
  on a quiet machine before release.

### 2026-09-27 — Stereo tool in a mono plugin chain refused the path (GRAPH)

- Report (user): Play said `path 1 (starting at "Microphone (PD200X)") is not
  supported: a path needs one source or one Mixer, then a single chain, then
  its outputs` after FIR Filter Hz replaced ReaFIR.
- Cause: the voice path is mono (PD200X and the ReaPlugs nodes are 1 ch) but
  library tools are created stereo. The path compiler only lets the width
  change from the source into the first tool, so mic (1) → FIR Filter Hz (2)
  → ReaEQ (1) was refused. Any built-in tool added to that chain would fail.
- Fix (`harmonize_chain_widths`, engine): built-in tools are width-agnostic,
  so in each linear chain they take the chain's width: its plugins' input
  width, otherwise the first tool's. Only edges whose matrix no longer fits
  are rebuilt (mono copied to every channel, averaged into mono, else
  identity). Applied before both compilers; a no-op when nothing differs.
- Verification: engine regression
  `a_stereo_built_in_tool_in_a_mono_plugin_chain_runs_at_the_chain_width`
  reproduces the exact error without the fix. Live: the user's saved session
  (copy) prepares, starts and delivers audio with FIR Filter Hz running and
  ReaEQ → ReaComp → ReaGate sharing one worker
  (`live_native_paths_start_pump_and_report_signal_timing`, now reading the
  session's plugin nodes instead of a fixed list).
- Known, unchanged: the single-output (endpoint worker) compiler refuses a
  mono plugin chain feeding a stereo output even when every node is mono.

### 2026-09-27 — Save flickered "Saving…" every second while playing (UI-04)

- Report (user): after Play, Save showed "Saving…" about once a second, and
  FIR Filter Hz showed a blue line after Stop but no live movement.
- Cause: while playing, a parameter-only difference between the draft and
  the saved route is applied live after 0.4 s. `isParameterOnlyChange`
  compared parameters with `JSON.stringify`, which depends on key order.
  The backend returns parameters with sorted keys, and FIR Filter Hz's
  defaults are not alphabetical, so each 1 s snapshot looked like a change.
  Each re-apply started a new runtime generation, rebuilding the tool and
  resetting its spectrum smoothing. After Stop the editor still drew the
  last telemetry.
- Fix: compare with the existing order-insensitive `sameJsonValue`; draw
  the live line only while the route plays.
- Verification: regression tests in `draft.test.ts` and
  `SpectralGateEditor.test.tsx`; vitest 366, Playwright suite. Live: the
  user's session copy reports `spectrum.levelsDb` for FIR Filter Hz while
  playing (live test now prints it).
- Artifact: `target/patrick-main-release-6/release/` (shell, plugin worker,
  CLI), built 17:30 with `custom-protocol`, embedding UI bundle
  `index-BUK9DptH.js`. A new folder because `release-5` was running.
