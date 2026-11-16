# Live parameter saves must not stop audio

> **Archived 2026-11-16 — merged in PR #150. The attended Mute/Fun Voice Changer recording remains open in the active plan.**

- **Status:** implemented locally, review-repaired three times and passed the
  copied-owner-database live parameter and bypass runs (2026-11-14). The
  built-shell run with the owner's Clear VST remains unrun.
- **Scope:** live autosave and live parameter application on a playing route
- **Owner report:** toggling a Mute node or an option on the last Fun Voice
  Changer node stops all output for up to two seconds before autosave finishes.

## Objective

A parameter-only change (Mute on/off, a Fun Voice Changer preset, an EQ band,
a slider, and so on) made while audio plays must be saved and applied without
reloading the route or interrupting output. The user must also be able to turn
live parameter autosave off, and rapid changes from a script or macro must
coalesce into one save instead of one full save per change.

The implementation must preserve graph validation, the authoritative backend,
the all-or-nothing commit, the privacy rules, and the current explicit-Save
behavior for topology changes.

## Requirement IDs

- **GRAPH-08** — parameter changes validate in the backend and advertise when
  they need a rebuild or a stop.
- **GRAPH-09** — transactions carry a base revision, idempotency key and
  canonical diff; a failed compile creates no revision.
- **UI-07** — live parameter edits use bounded, coalesced requests and one
  final committed target; local optimism never overrides a backend conflict.
- **UI-05** — expensive rebuilds show progress and keep the last committed
  values on failure.
- **GRAPH-15** — an independent-path session keeps every path's identity while
  a change is applied atomically.
- **UX continuity** — saving while playing must not starve the audio pump
  (existing rule behind the dedicated audio thread).

## Reported behavior and evidence

The owner's route is approximately:

`Input → … → Clear VST → … → Fun Voice Changer → { virtual cable → Discord/games; Mute → headphones }`

A key script toggles Fun Voice Changer options; the owner also toggles the
Mute node. Both edits save automatically while the route plays. During the
save, all output stops for up to two seconds.

To reproduce, copy (do not modify) the owner's database and session, start the
route with the Clear VST present, then change only the Mute node's `muted`
parameter while recording the output. The owner's
`%LOCALAPPDATA%\AudioRouter\logs\backend.jsonl` and session copy will confirm
which plugin is reloaded and the commit duration.

## Root cause

This is not slow change detection in the UI. The backend currently treats
almost every live change as a full route preparation, and then stops feeding
the audio worker until that preparation finishes.

1. **Autosave path** — `ui/src/App.tsx:1654` schedules a parameter-only draft
   400 ms after the last edit; `ui/src/App.tsx:1613` sends `graph.plan` then
   `graph.commit`.
2. **The backend's `flags_only` flag is not "parameter only."** In
   `crates/control/src/graph.rs:498`, `flags_only` is true only when the sole
   differences are `enabled`/`bypass`. A changed parameter makes it false.
3. **False selects full plugin preparation.** `crates/control/src/graph.rs:526`
   passes that flag into `republish_running_native_graph`
   (`crates/control/src/status.rs:138`). With false, both the multi-path and
   single-path paths call `prepare_plugin_stages`
   (`crates/control/src/status.rs:196`,
   `crates/control/src/native_workers.rs:615`).
4. **Every plugin is started again.** `prepare_plugin_stages`
   (`crates/control/src/plugins.rs:964`) always builds requests for every
   enabled plugin node and calls `start_plugin_bridges`
   (`crates/control/src/plugins.rs:1096`). That re-hashes each binary, spawns
   a new worker process, restores saved state and reapplies parameters
   (`crates/control/src/plugins.rs:142`). The Clear VST is reloaded even
   though it was not edited, which is the two-second part.
5. **Audio is switched off before that work completes.** In
   `crates/control/src/graph.rs:485`, `runtime.start()` advances the runtime
   generation. The service pass at `crates/control/src/graph.rs:512` then sees
   that the attached worker does not serve the new generation
   (`multi_input_worker_serves`, `crates/control/src/audio_service.rs:411`) and
   calls `set_pumping(false)`. Pumping is not re-enabled until
   `republish_running_native_graph` finishes and records
   `native_multi_input_applied_generation`
   (`crates/control/src/status.rs:263`). The plugin reload therefore happens
   inside the silence window.
6. **The audio slot is also held during the swap.** `replace_path_set`
   (`crates/control/src/audio_thread.rs:720`) takes the worker slot lock
   (`with_mut`), so the dedicated audio thread skips passes while the graph is
   replaced. This should be a short, atomic swap, never the place where a
   plugin is loaded.

A related single-endpoint route has the same shape: the control thread pumps
those workers, so slow work outside `while_servicing_audio` is directly a
dropout.

## Decisions

- **D1 — Classify edits in the backend.** Replace the overloaded `flags_only`
  boolean with an internal `LiveEdit` value:
  `FlagsOnly`, `ParametersOnly` or `Structural`. Classification uses the
  previous committed session and the candidate, not a client hint. No new API
  parameter is needed, so REST/MCP parity is unchanged.
- **D2 — Reuse live plugins for parameter-only saves.** `ParametersOnly`
  reuses every plugin bridge whose plugin binary, class, channel count, saved
  state and VST2 chain grouping are unchanged. Built-in parameter edits then
  compile a new graph around the same loaded plugin instances. Slider changes
  on a reused plugin are delivered with `set_parameters`; a plugin identity or
  saved-state change restarts only that plugin (or VST2 group), never the whole
  chain.
- **D3 — Never stop the old graph while preparing the new one.** Keep the
  worker pumping its current prepared graph until the new path set is ready,
  then swap atomically. Update the applied generation as part of the swap, not
  before preparation. A failed preparation leaves the old graph playing and
  reports the existing error/`restartRequired` semantics.
- **D4 — Reuse unchanged plugins for structural saves too.** Adding a Gain or
  rewiring a branch must not reload an unrelated Clear VST. `prepare` becomes
  "reuse or start" rather than "start every plugin."
- **D5 — Bounded, coalesced autosave.** The UI keeps at most one autosave in
  flight; edits made during it collapse to the latest draft. An automatic save
  that races an edit must not show "Your edits changed during review"
  (bug-audit UI-2). A failed automatic save still is not retried for the same
  unchanged draft.
- **D6 — A user switch for live parameter autosave.** Add a local UI
  preference (`Live autosave`, default on). Turning it off stops automatic
  parameter commits only. Explicit Save and the existing immediate flag
  changes (`enabled`/`bypass`) keep working as specified. This is a UI
  preference, not a new API setting, so no backend migration is needed.
- **D7 — Measure before and after.** Record `graph.plan`, `graph.commit`,
  plugin preparation and path-set swap durations, and count plugin starts per
  commit, without names or paths, so the owner's report is proved fixed rather
  than assumed.

## Ordered tasks

1. **Add a reproduction harness and measurement.**
   - Copy a disposable owner database/session; start the route with privacy
     mute so the measurement is silent; record the output loopback.
   - Add bounded timing fields to the existing diagnostics or backend log for
     plan, commit, plugin preparation and path swap, plus a plugin-start
     count.
   - Reproduce the current two-second gap with the owner's Clear VST before
     changing behavior.

2. **Classify live edits (`crates/control/src/graph.rs`).**
   - Add `LiveEdit` and a pure classification function comparing the previous
     session with the candidate: whole-session fields, node identity, names,
     ports, flags, topology, parameter map.
   - Thread it through `commit_graph_scoped`, `republish_running_native_graph`
     and `activate_native_graph_candidate_with_flags`; keep the existing
     behavior for `Structural`.

3. **Reuse plugin stages (`crates/control/src/plugins.rs`).**
   - Make `prepare_plugin_stages` accept the previous session and the live
     bridge map, and return either reused bridges or `PluginStageRequest`s.
   - Reuse when path, fingerprint, class ID, channels, saved state and VST2
     chain members are unchanged. Do not re-hash a reused binary.
   - For reused bridges, apply only changed `pluginParameter:` values through
     `set_parameters`, then update `set_silence_if_unchanged` and
     `set_processing_active`.
   - Restart only the changed plugin or VST2 group when identity/state/chain
     changes. Keep the current all-or-nothing error behavior.

4. **Keep audio moving during commit (`crates/control/src/graph.rs`,
   `crates/control/src/status.rs`, `crates/control/src/audio_service.rs`).**
   - Prepare and compile before advancing the served generation, or teach the
     service about a transition so the old prepared graph keeps pumping until
     `replace_path_set` succeeds.
   - Remove the `service_running_native_audio` call that disables pumping
     between `runtime.start()` and graph publication, or make it accept the
     previous generation during the transition.
   - Hold the worker slot only for the atomic swap. Service single-endpoint
     workers during any remaining preparation work.
   - On failure, keep the old graph running and restore the previous applied
     generation.

5. **Avoid repeated media work (`crates/control/src/audio_media.rs`).**
   - Cache decoded Audio File/FIR media by media ID and sample rate across
     parameter-only commits, so a file route does not decode on every save.
     Invalidate on media change/removal and keep one bounded cache per session.

6. **Coalesce and control autosave (`ui/src/App.tsx`,
   `ui/src/preferences.ts`, `ui/src/UiOptionsWorkbench.tsx`,
   `ui/src/SettingsSections.tsx`).**
   - Add `liveAutosave` (default true) with read/write helpers and a Settings
     control that follows the app field style and Settings search keywords.
   - Skip the parameter autosave effect when off. Keep flag changes and
     explicit Save unchanged.
   - Replace the single timer/last-key check with one in-flight save and a
     pending-latest draft. Pass an `automatic` flag to `planChanges`; on a
     race, stay silent and let the queued latest draft save.
   - Keep the existing "one attempt per unchanged draft" behavior after a
     failure.

7. **Update the specifications and user documentation.**
   - Clarify in `docs/spec/04-graph.md` and `docs/spec/09-interface.md` that a
     parameter-only live save reuses loaded plugins and does not stop audio,
     and that Live autosave is optional.
   - Update `docs/operations/quickstart.md` (autosave section), and mention
     the setting in the relevant guide if controls are user-visible.

## Validation matrix

| Check | Environment | Pass condition |
| --- | --- | --- |
| `cargo test -p audiorouter-control --lib` (new classification and plugin-stage-reuse units) | Windows or GNU-target Linux | Unchanged plugins are reused; state/identity/chain changes restart only the affected plugin |
| `cargo test -p audiorouter-engine --test voice_changer` | Windows or GNU target | Existing live preset change remains continuous |
| New ignored live test: parameter-only commits keep the worker playing | Windows, a copied owner database/session with an enabled plugin and a Gain/Volume/Mute/plugin-slider control, privacy muted, `AUDIOROUTER_PLUGIN_WORKER_PATH` (or `AUDIOROUTER_VST3_WORKER_PATH`) set | Four commits stay `applied`; no sample gap over 750 ms; each commit within `AUDIOROUTER_LIVE_COMMIT_MAX_MS`; `activation.timing.pluginStarts` is 0; sampled `submittedQuanta` keeps advancing; `outputUnderruns` does not increase; every unchanged plugin's `workerId` stays the same |
| Existing `live_saved_session_bypass_toggles_apply_while_playing` | Windows, copied owner database | Every flag change still reports `applied` and keeps pumping |
| `npm --prefix ui test` focused autosave/settings suites | Windows | Off disables parameter autosave; rapid edits coalesce; no mid-flight "edits changed during review"; preference persists |
| `npm --prefix ui run typecheck`, `lint`, `format:check` | Windows | Clean, zero warnings |
| Edge `compact-workspace.pw.ts` plus a new autosave-off scenario | Windows Edge, production bundled assets | Default still autosaves; off leaves a Save marker and never commits parameter drafts |
| Built shell with disposable database/profile, real plugin | Windows | Toggle Mute and Fun Voice Changer from a script; backend log shows reuse (no plugin start); output has no gap; exe newer than `ui/dist` |
| Full Rust workspace tests and strict Clippy, both formatting checks | Windows | Clean before reporting ready |

Live hardware evidence is required for the continuity claim. Portable tests
and a CI-only run may prove classification, but must not be reported as proof
that the owner's audio no longer stops.

## Risks

- **Bridge reuse across generations** could process audio twice if the old
  graph is not dropped promptly. Cover with the live continuity test and
  assert no doubled level or duplicate plugin processing.
- **VST2 exclusive chains** share one worker; reuse is only valid when every
  member is unchanged. Otherwise restart the whole group.
- **Plugin state changes** must never be applied to a stale bridge. Treat
  `stateId`, path, fingerprint, class, channels or chain membership changes as
  a restart of that plugin/group.
- **Transition window:** serving the old generation briefly could let a caller
  observe old audio after `graph.commit` returns. Keep the window bounded and
  only until the atomic swap; return the new generation only after the swap.
- **A failed preparation** must leave audio playing and report the failure,
  not fall into silence or an unrecoverable `Preparing` state.
- **Turning autosave off** leaves unsaved parameter drafts; the Save marker
  and explicit Save must remain clear, and flag changes must still apply live
  per the interface spec.

## Rollback

Revert the implementation branch. The UI preference defaults to on, so omitting
it preserves today's behavior. No API method, parameter, database schema or
session migration is added; a reverted build reads the same sessions. If the
pumping change proves risky, ship the plugin-reuse and coalescing work first
and keep the old transition behavior behind the same tests.

## Implementation record — 2026-11-14

Implemented on the working tree; no API method, parameter or schema changed.

- **Classification:** `LiveEdit` and `classify_live_edit` in
  `crates/control/src/graph.rs`. A commit is `FlagsOnly`, `ParametersOnly` or
  `Structural`; parameter changes win over simultaneous flag changes.
- **Plugin reuse:** `prepare_plugin_stages` in
  `crates/control/src/plugins.rs` now takes the previous session and reuses a
  live bridge whose binary identity, class, channels, saved state and VST2
  chain membership are unchanged. Changed plugin sliders are pushed with
  `set_parameters`; only changed identities/states or changed VST2 groups
  start a worker. `plugin_identity_matches`, `plugin_sliders_changed`,
  `vst2_eligible` and `plugin_group_key` implement the decision.
- **Pumping through preparation:** `graph.rs` marks the running multi-input
  worker as serving the new generation before compilation, and no longer runs
  a service pass that disables pumping between `runtime.start()` and the graph
  swap. The old prepared graph plays until `replace_path_set` swaps it
  atomically.
- **Sources and media:** `LiveEdit::retains_sources` keeps Test Signal, Audio
  File and Soundboard transports for live edits.
  `ControlPlane::audio_media_cache` keeps decoded Audio File/FIR media by
  session, media ID and sample rate; it is pruned to the session's referenced
  media and invalidated when media is deleted.
- **UI:** `readLiveAutosave`/`writeLiveAutosave` in `ui/src/preferences.ts`, a
  `LiveAutosaveSetting` panel in Appearance, and optional autosave in
  `ui/src/App.tsx`. Automatic saves pass an `automatic` flag so a mid-save
  edit stays quiet and the latest draft saves once, rather than showing the
  manual "Your edits changed during review" message. Flag changes remain
  immediate; explicit Save is unchanged.

Local validation (Windows, Rust 1.96.0, Node 22 / Vitest 5.1):

- `cargo test -p audiorouter-control --lib`: 402 passed, 0 failed, 13 ignored.
- `cargo test -p audiorouter-transport`: all non-ignored tests pass; the new
  ignored live parameter test compiles and is listed as
  `live_saved_session_parameter_change_keeps_the_worker_playing`.
- `cargo test -p audiorouter-engine --test voice_changer`: 6 passed.
- `cargo test --workspace --locked`: every non-ignored target passed with no
  failures or panics.
- `npm --prefix ui test`: 120 files, 1,073 tests passed, including new
  autosave-on, autosave-off and in-flight coalescing regressions.
- `cargo fmt --all -- --check` and the shell manifest check pass; strict
  workspace and shell Clippy pass; UI `format:check`, zero-warning `lint`,
  `typecheck:all` and docs validation (268 files / 1,492 links) pass.
- Production bundled Edge, `compact-workspace.pw.ts`: 12/12 passed across
  dark, light and high contrast, including the existing debounced autosave
  test and the new autosave-off scenario.

Retained limitations: the live continuity test was not run (it needs the
owner's audio devices and a copied session), and no built shell or real Clear
VST was exercised. Those are the remaining release-evidence gates, not claims
of a passing live result.

## Review repair — 2026-11-14

An independent review of the implementation found nine issues. All were
repaired on the same working tree; no API method or request parameter changed.

1. **Changed source settings no longer keep their old transport.**
   `ControlPlane::reusable_source_transports` keeps a live Test Signal, Audio
   File or Soundboard transport only when the node's kind and parameters are
   unchanged. Changing `mediaId`, `loop`, frequency, level or a Soundboard
   configuration rebuilds that source; an unchanged source keeps its playback
   position. The engine's retention helper is now documented as a live-rebuild
   operation, not flag-only.
2. **Mixed reuse and restart preparation no longer fails.** `start_plugin_bridges`
   filters the session's worker groups through `requested_plugin_groups`: a
   group with no requested member is already playing and is skipped, a fully
   requested group starts, and an impossible partial shared VST2 chain is
   rejected instead of leaving a bridge without a worker. Every requested
   node is verified as started.
3. **VST2 reuse now follows processing order.** `plugin_group_key` is an
   ordered chain identity, so reversing `A → B` into `B → A` restarts the
   group; the old order-insensitive test was replaced.
4. **Reused bridges get their active/bypass state.** Every reused bridge
   receives `set_processing_active(node.enabled && !node.bypass)` at
   publication, including parameter commits that also change a flag. A stale
   inactive bridge can no longer pass dry audio after an un-bypass.
5. **Preparation no longer mutates playing plugins.** `prepare_plugin_stages`
   now returns a `PluginStageSet`. Reused bridges are only read; parameter
   templates, silence policy and active flags are held in deferred updates,
   and new worker handles are published only after the compiled graph has
   replaced the playing one. A failed compile or swap therefore leaves the old
   graph and its control state untouched.
6. **Only changed sliders reach a reused plugin.** Reuse computes the slider
   delta between the previously saved node and the candidate
   (`reused_parameter_events`), instead of sending every authored slider, so
   values changed in the plugin's own editor survive. A member becoming active
   gets the full authored set, because its template may hold changes saved
   while it was bypassed; a slider removed from the node keeps its last
   applied value.
7. **A failed preparation keeps the old route playing.** Live publication now
   distinguishes `ControlError::RestartRequired` (the playing layout cannot
   absorb the change) from preparation failures. Only the former may stop and
   restart the worker; any other failure leaves the old graph pumping and
   reports `restartRequired` with the reason. The pending saved graph is
   marked stale so the next commit attempts a full preparation instead of
   reusing a bridge that no longer matches the saved session.
8. **The decoded-media cache is bounded.** Entries are retained per session,
   media ID and sample rate only while referenced; other sample-rate variants
   are dropped, deletion of a media object or a session clears its entries, and
   a 256 MiB budget releases other sessions' decoded audio. A
   `trim_audio_media_cache_to` seam allows the bound to be tested without large
   allocations.
9. **The live continuity test can now fail.** It requires an enabled plugin and
   a Gain/Volume target, samples the running worker every 20 ms across four
   parameter commits, requires processed quanta to keep advancing with no new
   output underruns, and requires every unchanged plugin's `workerId` (new
   read-only diagnostics field) to stay the same. Missing fixtures panic
   instead of passing silently.

Regression coverage added: source-retention filtering, requested worker
groups (including the partial-chain refusal), ordered chain keys, slider-delta
events and identity changes, restart-only-for-layout errors, media-cache
bounding and deletion. `cargo test --workspace` passed (including 407 control
tests), strict Clippy and both formatting checks passed, and contract drift is
clean. The live hardware gate remains unrun here.

## Follow-up review repair — 2026-11-14

A second review of the live-reuse path found ten issues. All are repaired on
the same working tree; no API method or request parameter changed.

1. **A plugin enabled while playing restarts or prepares instead of failing.**
   A flag-only republish now checks that every enabled plugin node has a live
   bridge (`live_plugin_bridges_are_complete`). If one is missing, it falls
   through to `prepare_plugin_stages`, which reuses the playing bridges and
   starts only the missing plugin. Compiled from code inspection; the trigger
   is unit-tested (`a_plugin_enabled_without_a_live_bridge_requires_preparation`).
2. **Flag-only publications keep the stale-graph marker.**
   `record_live_publication` clears `live_graph_stale` only for
   `ParametersOnly`/`Structural` publications prepared from the saved session.
   Hold-to-compare and flag toggles reuse by node ID, so a marker left by a
   failed save survives them and the next save prepares fully.
   Regression: `only_a_prepared_publication_clears_the_stale_graph_marker`.
3. **A failed or quarantined plugin is no longer reused.**
   The reuse selection skips a bridge whose published health is `Failed` or
   `Quarantined`; that plugin is started fresh, so a parameter save revives it
   as the pre-change behavior did. `Stopped` is treated as healthy (no quantum
   yet). Regression: `failed_or_quarantined_plugin_bridges_are_not_reused`.
4. **A control-update rejection after the swap is not a failed save.**
   `PluginStageSet::publish` errors are reported as a `warning` on the applied
   activation and keep the stale marker, so the route keeps playing and the
   next commit prepares the plugin again instead of trusting a stale template.
5. **The live continuity test can use the owner's route and can fail.**
   It targets a Gain, Volume, Mute or plugin slider (the owner's route has
   Mute), follows the committed generation while sampling, records sample
   timestamps, requires no control-plane gap over 750 ms, requires commit
   durations within `AUDIOROUTER_LIVE_COMMIT_MAX_MS` (default 1000), and
   requires `activation.timing.pluginStarts == 0` on every round. The
   required `AUDIOROUTER_PLUGIN_WORKER_PATH`/`VST3` setup is documented on the
   test and in the validation matrix.
6. **Reuse is observable through the dispatcher.** `graph.commit` reports
   `activation.timing.pluginStarts`, `prepareMicros`, `swapMicros` and
   `commitMicros`; `graph.plan` reports `timing.planMicros`. The live
   dispatcher test asserts zero starts; non-live tests cover the health,
   missing-bridge and stale-marker decisions.
7. **D7 measurement is implemented.** The timing fields above are returned
   without names, paths or parameters, and the backend log already records
   per-request `durationMs` in the verbose window.
8. **No stale "Checking route before saving…" message.**
   Only a manual save sets that message; an automatic save stays quiet. An
   automatic save holds one in-flight request (`autoSaveRunning`) and the
   autosave effect picks up edits made during it, so they coalesce into the
   latest draft. The coalescing regression also asserts the message is absent.
   The plan now describes this effect-driven queue rather than an unimplemented
   queue object.
9. **Quickstart names the autosave setting once.** The duplicated instruction
   was removed from the Spectral Gate paragraph; the EQ paragraph keeps the
   single pointer to **Settings → Appearance → Live autosave**.
10. **Stopping a route releases its decoded media.** `session_stop` drops the
    session's decoded Audio File/FIR PCM; deletion already did. Regression:
    `stopping_a_session_releases_its_decoded_media`.

Local validation after this round: `cargo check -p audiorouter-control
--all-targets` and `cargo check -p audiorouter-transport --tests` pass; the
five new regression tests pass; `npm --prefix contracts run check:drift`,
contracts/UI `typecheck:all` and the focused autosave UI tests pass. The full
control lib suite reported 410 passed, 1 unrelated OBS test failed once and
passed when rerun alone; after the final edits the full control suite passed
411/411 and `cargo test --workspace --locked` passed every target.

## Live evidence — 2026-11-14

Ran on a copy of the owner's database in
`%TEMP%\opencode\autosave-live-db` with the real Clear/ReaPlugs route
(`patrick-main-session`), privacy mute latched and
`AUDIOROUTER_PLUGIN_WORKER_PATH` pointing at the built VST2 worker:

- `live_saved_session_parameter_change_keeps_the_worker_playing`: four
  `siege-eq.band0GainDb` commits all reported `applied` in 39.7–48.7 ms each;
  `activation.timing.pluginStarts` was 0; no sampler gap over 750 ms; the same
  plugin worker IDs remained; no new output underruns. The test now selects
  the owner's EQ because this route has no Gain or Volume node.
- `live_saved_session_bypass_toggles_apply_while_playing`: every bypass
  toggle for ReaComp, ReaGate, the EQ, Spectral Gate and Parametric EQ reported
  `applied`.

The private backend's fixed 300 ms startup was replaced by a 10 s retry loop in
both tests; a large copied database took longer than the old wait. The built
shell with the owner's Clear VST and a real script-driven Mute/Fun Voice
Changer toggle remains unrun.

## Verify pass — 2026-11-14

An independent verification of the two repair rounds found three leftovers. All
are resolved on the same working tree; no API method or request parameter
changed.

1. **Reuse is now proven by ordinary tests, not only the ignored live run.**
   `LivePluginBridge` (in `crates/control/src/plugins.rs`) names the live-bridge
   operations a live rebind needs, and `PluginRuntimeBridge` is its production
   implementation. `prepare_plugin_stages` keeps its signature and delegates to
   `prepare_plugin_stages_with`, which takes the candidate's live bridges and a
   start function, so a test can supply a bridge double.
   - `a_parameter_commit_reuses_a_loaded_plugin_bridge` prepares an
     `effect → out` route with a loaded double, changes `pluginParameter:0`,
     and requires zero worker starts, the same bridge in the compiled stage map,
     and one applied slider at publication.
   - `a_comparison_republish_reuses_loaded_plugins` covers hold-to-compare (see
     item 3).
   The full `graph.commit` dispatcher path with a real plugin worker remains
   covered by the ignored live test; the worker cannot be attached in a unit
   test without audio devices.
2. **The live test's plugin-slider fallback now keeps the parameter name.**
   It selected the numeric parameter, then stripped the `pluginParameter:`
   prefix before writing the candidate (`live_audio_continuity.rs`), which would
   have written a bogus parameter such as `"3"`. It now keeps the full name and
   only uses the prefix to choose the slider step.
3. **Hold-to-compare with a missing plugin worker reuses what is loaded.**
   `ControlPlane::saved_reuse_base` supplies the saved session as the reuse base
   for a `FlagsOnly` publication that has no committed previous session; both
   live publication paths (multi-input and single-endpoint) now use it. A
   comparison only overlays `bypass`, so plugin identity and saved state still
   match. Regression:
   `a_flag_only_publication_reuses_the_saved_session_as_its_base`, plus the
   comparison reuse test above showing that a missing base falls back to a full
   preparation.

Local validation of this pass: `cargo test -p audiorouter-control --lib` 414
passed (3 new), 0 failed, 13 ignored; `cargo clippy -p audiorouter-control
--all-targets -- -D warnings` clean; `cargo check -p audiorouter-transport
--tests` clean; both formatting checks clean. A full
`cargo test --workspace --locked` run passed every target except one
`audiorouter-windows-audio` timing test
(`network_audio::tests::a_paired_tone_plays_back_continuously`), which asserts
packet counts and discontinuities under load and passed when rerun alone; that
crate is untouched by this change.

## Built-shell/live evidence — 2026-11-14

The remaining copied-database gate was rerun after rebuilding the desktop
artifacts from this branch:

- `powershell -NoProfile -ExecutionPolicy Bypass -File tools/build-desktop.ps1
  -Profile release` passed. The release shell and both plugin workers were
  rebuilt, the UI was rebuilt first, and `ui/dist/index.html` is newer than the
  prior shell build. Shell SHA-256:
  `0AE9D2DF12B617D8C84A2EA08F0E3BB1316A582D8AC1719A2474D88BACEF28DB`.
- `live_saved_session_parameter_change_keeps_the_worker_playing` passed against
  `%TEMP%\opencode\autosave-live-db` with the real Clear/ReaPlugs route,
  privacy mute latched, and the release VST2 worker. Four EQ parameter commits
  reported `applied` in 18.9–20.6 ms; no plugin starts, worker identity
  changes, sampler gap or output-underrun increase was reported.
- `live_saved_session_bypass_toggles_apply_while_playing` passed on the same
  copied route. ReaComp, ReaGate, Siege EQ, Spectral Gate and Parametric EQ
  each toggled on and off with `state: applied`.

This is strong copied-owner-database evidence for the backend reuse and
continuity path. The exact attended desktop-shell interaction with the
owner's script-driven Mute/Fun Voice Changer toggle and an output recording is
still a separate user-route gate; no unattended test can substitute for that
evidence. The implementation is ready for human review and PR approval, but
must not be described as fully hardware-qualified until that attended check is
recorded.

## CI repair — 2026-11-15

PR #150's first hosted run failed one Edge scenario,
`session-workflows.pw.ts:99` (shard 3/4). The test compares a replayed
`graph.commit` to the original response; the new `activation.timing` block
made those responses differ because a per-request duration is not
reproducible. The repair keeps the per-request number only where work was
actually published: a simulated/pending commit and an idempotent storage
replay now return the original `{ "state": "pending", "runtime": "fake" }`
shape, while a running commit still reports `timing.commitMicros` and a live
native publication reports `commitMicros`/`prepareMicros`/`swapMicros`/
`pluginStarts`. Regressions: `dispatch_plan_and_commit_use_json_contracts`
now also replays the same idempotency key and requires identical activation,
and `a_running_commit_reports_its_own_timing` keeps the timing assertion on
the path that publishes. The failing Edge spec was rerun locally with the
rebuilt `e2e_backend` (`session-workflows.pw.ts`, 7/7 passed).

## Next action

PR [#150](https://github.com/MrDesjardins/audiorouter/pull/150) is open with
the CI repair pushed. Inspect the new hosted checks and annotations, then
finish the attended Mute/Fun Voice Changer recording as the final
hardware/user-route gate; do not merge or release on the copied database
evidence alone.
