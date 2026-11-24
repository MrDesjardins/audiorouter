# Remaining low-severity findings of the 2026-11-11 bug audit

Date: 2026-11-24 (overnight, unattended). Branch `fix/audit-low-remaining`, based on
origin/main `0fb41911b`. Source write-ups:
[bug audit](../../archived/2026-11-11-bug-audit.md). Environment: Windows 11, Rust 1.96.0
(pinned), `CARGO_BUILD_JOBS=6`, no backend or shell launched, no release build.

| Finding | Result |
| --- | --- |
| DSP-4 | Fixed |
| REC-2 | Not reproduced (latent; no change) |
| SES-4 | Fixed |
| HTTP-1 | Already fixed on origin/main |
| UI-1 | Fixed |
| UI-2 | Already fixed on origin/main |

## DSP-4 — running-sum RMS meters report NaN after silence

- Still present: yes. `crates/dsp/src/lib.rs` `WindowedSignalMeter` and
  `crates/engine/src/lib.rs` `RmsWindow` subtract the oldest square from a running `f64`.
- Tests (written first):
  `audiorouter_dsp::tests::windowed_meter_rms_stays_finite_and_zero_after_signal_then_silence`
  and `audiorouter_engine::tests::rolling_rms_window_stays_finite_and_zero_after_signal_then_silence`.
  Each runs 50 seeds of 48 000 frames of sine plus noise, then one full window of zeros, and
  requires a finite `rms` of exactly 0.
- Failing before the fix: DSP `seed 0: rms [NaN, 0.0]`; engine `seed 0: left: 2.7104598e-8,
  right: 0.0` (a positive residue, so a bare clamp would not have made it zero).
- Fix: a clamp (`.max(0.0)` before `sqrt`) as asked, plus an exact count of non-zero squares in
  the window per channel; when it reaches zero the running sum is set to exactly 0. The
  clamp alone cures NaN but leaves a non-zero RMS for a silent window, which the audit's own
  regression test rejects.
- Observation, not acted on: the audit says both types are reachable only through
  `VoiceChainBlockProcessor`, which no production code constructs. Nothing was deleted; removing
  `VoiceChain`, `VoiceChainBlockProcessor`, `WindowedSignalMeter` and `RmsWindow` stays an owner
  decision.
- Files: `crates/dsp/src/lib.rs`, `crates/engine/src/lib.rs`. Commit `2e77b51c7`.

## REC-2 — one-click recording hard-codes 48 000 Hz

- Not reproduced. `start_node_recording` does use the literal `48_000`
  (`crates/control/src/recording.rs`), but every worker that can reach it runs the graph at
  `INTERNAL_SAMPLE_RATE_HZ`: `prepare_native_endpoint_worker` builds its bridge with
  `WasapiSchedulerBridge::new_for_endpoints_at_graph_rate(..., INTERNAL_SAMPLE_RATE_HZ)`, the
  multi-input and render-source workers use the same constant, and Windows resamples other device
  rates (`multi_path_rate_supported`). The reproduction the audit asks for (a 44.1 kHz
  endpoint-rate worker) needs real WASAPI capture/render clients, which a control test cannot
  build, and no production path creates one.
- No change. If the owner wants a guard, replace both literals with
  `audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ` (naming only, no behavior change, no test can fail
  without it).

## SES-4 — deleting a session leaves recorder state for its nodes

- Still present: yes.
- Test (through `ControlPlane::dispatch`): `recording::tests::deleting_a_session_finalizes_its_one_click_recordings_and_clears_recorder_state`
  starts `recorders.startRecording` on a stopped session, feeds blocks, calls `sessions.delete`,
  requires all five maps cleared, the WAV playable (`assert_playable_wav`) and `system.quit`
  without error.
- Failing before the fix: `recorder state left behind: Some(Recording)`.
- Fix: `delete_session` now calls `finalize_session_recorders` after every refusal and before the
  durable delete (stopping a recorder reads the session, so it cannot run after removal), and
  `forget_session_recorder_nodes` clears `recorder_node_workers`, `recorder_node_states`,
  `recorder_node_sessions`, `recorder_node_failures` and `recording_splits` together, for the
  session's Recorder nodes and any node still mapped to it.
- Decision: finalize before the durable delete. If the storage delete then fails, the session
  stays and its recording is already saved and stopped; this is the audit's "stop each node
  recording before the delete (preferred)".
- Files: `crates/control/src/sessions.rs`, `crates/control/src/recording.rs`,
  `crates/control/src/recording_tests.rs`. Commit `570dae81d`.

## HTTP-1 — desktop-only refusals all name the recording folder

- Already fixed on origin/main (PR #170). `src-tauri/src/http_api.rs` `http_access` carries a
  specific message per desktop-only method, and
  `desktop_methods_explain_their_own_action` requires each listed method's 403 body to contain its
  own message and every message to be unique. Only `recordings.setRoot` says "Choose the recording
  folder". Not run here (src-tauri untouched).

## UI-1 — hand-kept generator and route-source kind lists

- Still present: partly. `draft.ts` already had the exhaustive `routeSourceType`, but four
  hand-kept copies remained: `App.tsx` (application Mixer source allow-list and the
  `generated` binding test), `SessionFlowCanvas.tsx` (`hasOnlyCaptureSources`), and
  `smartLayout.ts` (`INPUT_KINDS`).
- Test (written first): `ui/src/sourceKinds.test.ts`. It lists every `NodeKind` as a
  `Record<NodeKind, true>` (typecheck fails when a kind is added), reads the engine's
  `renders_own_audio` arms from `crates/engine/src/lib.rs` and requires `rendersOwnAudio` to agree
  kind by kind, and pins `isRouteSource` and `supportsApplicationMixerRoute`. The audit suggested
  comparing with `nodes.types`, but that discovery data carries no source role, so the engine
  source is the comparison point (adding a field would be an API change).
- Failing before the fix: all 4 tests failed (`undefined is not a function`; the helpers did not
  exist).
- Fix: `draft.ts` exports `rendersOwnAudio` (exhaustive switch, `never` default),
  `isRouteSource` and `supportsApplicationMixerRoute` (both derived from the exhaustive
  `routeSourceType`), used at all four sites.
- Behavior change to note: `hasOnlyCaptureSources` and the generated mixer binding now treat an
  Instrument as a generator, as the engine does (before, only Test Signal, Audio File and
  Soundboard counted). Instruments are refused earlier in the application-Mixer path, so only the
  privacy-mute indicator for a branch containing an Instrument differs (it is no longer "capture
  only").
- Hand-kept lists reviewed and left, because they classify something else:
  `appUsage.ts` `SOURCES_AND_SILENT_BYPASSES` (sources plus sinks, Mixer and switches),
  `obsUsage.ts` `SOURCE_KINDS` (omits Instrument; unclear whether intended, left for the owner),
  `draft.ts` `mixesFileOrBoardWithAnotherSource` (file/board/instrument rule), `routeHarness.tsx`
  (test fixture), `useAudioFileStatus.ts` (audio file status), per-kind CSS classes in
  `SessionFlowCanvas.tsx`. Converting the first two to `Record<NodeKind, boolean>` maps would make
  them typecheck-enforced; not done to keep this change behavior-neutral.
- Files: `ui/src/draft.ts`, `App.tsx`, `SessionFlowCanvas.tsx`, `smartLayout.ts`,
  `sourceKinds.test.ts`. Commit `6cdf57373`.

## UI-2 — autosave while dragging reports "Your edits changed during review"

- Already fixed on origin/main by `bab8f196c` ("Fix live parameter autosave continuity").
  `planChanges(automatic)` returns silently when the draft changed during `planGraph`, and the
  effect saves the latest draft. `ui/src/App.lifecycle.test.tsx` (the parameterized autosave test
  with `planning` and `committing` phases) changes the slider while the first plan or commit is in
  flight and requires the latest value, no "Your edits changed during review" and no
  "Checking route before saving". PLAN-1 (abandoned plans count against the cap) is a separate
  finding and was not in this task.

## Checks

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `cargo test -p audiorouter-dsp` | 84 passed, 1 ignored (+2 passed) |
| `cargo test -p audiorouter-engine` | lib 189 passed; `source_combinations` 6, `tool_combinations` 6, `deterministic_audio` 33, `tool_routes` 8, all passed |
| `cargo test -p audiorouter-control` | 454 passed, 24 ignored (run after the SES-4 fix) |
| UI `typecheck:all`, `lint` (zero warnings), `format:check` | clean |
| UI `npm test` | 134 files, 1170 tests passed |
| `playwright e2e/smart-layout.pw.ts` (Edge; temporary config on port 4197 because 4186 was in use) | 3 passed (dark, light, high-contrast) |
| `node tools/docs/validate.mjs` | passed (296 Markdown files, 1642 local links) |

Not run: src-tauri Clippy and tests (untouched, so `ui/dist` was not built); `audiorouter-cli`,
`audiorouter-transport` and `audiorouter-storage` tests; the remaining Playwright suites; any run of
the built app, since the shell and default control pipe were off limits. No API method, schema or
MCP tool changed, so the AUTO-18 steps (`API_METHODS`, `check:drift`, reference regeneration) did
not apply.
