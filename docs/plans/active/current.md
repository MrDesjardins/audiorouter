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
| [M02](../../milestones/M02-audio-engine.md) audio engine | Realtime graph, capture/render adapters, multi-input/many-output paths, backend audio service | Guarded routes ([M02](evidence/M02-audio-engine.md)); **continuity 0 glitches for 30–60 s on four route shapes** ([audio continuity](evidence/2026-09-26-audio-continuity.md)); NFR-02 p95 ≈97–115 ms (≤160 ms, DEC-15) | Clock-drift correction between independent devices; endurance/soak |
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
- **Bypassed plugins no longer block Play.** The user's saved session has
  ReaEQ bypassed, which made `nativePaths.prepare` fail. Regression test
  added.
- Plugin bridge and worker threads now use MMCSS "Pro Audio".
- Status line now shows output underruns and late backend audio-service
  gaps.
- Checks: workspace `cargo test --workspace --locked` passed (847 tests,
  0 failed). UI vitest 346 passed. UI typecheck and production build
  passed.
- Artifact for the attended test (built 22:50 local, UI bundle
  `index-CT3skZtC.js` confirmed embedded, shell newer than `ui/dist`):
  `C:\code\audiorouter\target\patrick-main-release-2\release\audiorouter-shell.exe`
  SHA256 `E47017C3C87785424FDFD60A505C986DEE6CC5CBB34314D41D3A104E8B283C10`.
  Worker beside it: SHA256
  `6CBB240B07041F05481316BEF9CA51416C80D0DCAD14A068141EA8FAFD276CF0`.

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
7. Attended gates (see the table): the user's by-ear confirmation of the
   real microphone and game routes, accessibility, scaling, first-run,
   OS transitions.

## Next actions, in order

1. **User (attended):** close any running AudioRouter, then launch only the
   artifact above:
   ```powershell
   $env:AUDIOROUTER_DATABASE = "$env:LOCALAPPDATA\AudioRouter\state.sqlite"
   $env:AUDIOROUTER_ALLOW_DEVICE_ADMIN = "1"
   & "C:\code\audiorouter\target\patrick-main-release-2\release\audiorouter-shell.exe"
   ```
   Select Patrick Main Session and press Play. Also try a Test Signal
   route. Listen to the voice and
   game with the window minimized and restored. The status line should
   show no "output underrun" or "late audio service gap". If clicks
   remain, note roughly how often they occur (drift gives rare, regular
   clicks).
2. Network send/receive tools (user-requested, 2026-09-26). See the log.
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
