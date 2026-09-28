# Combined audio-tool scenario coverage — 2026-09-28

Scope: review of supported source, processor, mixer, branch, destination,
recording, plugin and network combinations. Requirements: GRAPH-01–16,
DSP-01–18, REC-01–12, PLUG-01–07, QUAL-01–06; workflow scenarios UC-01,
UC-02, UC-04–08 and UC-10. This maps existing tests; it does not claim every
possible node ordering or parameter cross-product.

## Existing deterministic combination coverage

| Scenario shape | Existing evidence |
| --- | --- |
| Every built-in processor compiled and processed finite stereo audio; disabled/bypassed cases; disabled sources/sinks | `crates/engine/tests/tool_routes.rs`: `every_builtin_processor_compiles_and_processes_stereo_with_finite_meters`, `every_builtin_processor_off_and_bypass_preserve_the_dry_stereo_signal`, `disabled_source_and_sink_are_silent_for_every_builtin_processor_route` |
| Test Signal plus live physical input into a Mixer; two-source summation; Mixer to many outputs | `crates/engine/src/lib.rs` tests: `mixer_generates_a_test_signal_input_alongside_a_live_source`, `compiler_and_runtime_execute_a_two_source_mixer_graph`, `compiler_and_runtime_execute_mixer_to_many_outputs` |
| Processor chain before a split; plugin before every branch; per-node plugin health | `crates/engine/src/lib.rs`: `mixer_fanout_runs_a_volume_chain_before_one_input_and_allows_one_output`, `mixer_fanout_runs_bound_plugin_before_every_branch`, `mixer_fanout_plugin_health_reaches_the_realtime_wrapper` |
| Independent voice and game source paths with no cross-feed; bypassed plugin remains dry | `crates/engine/src/lib.rs`: `independent_paths_run_voice_and_game_in_one_session_without_crossfeed`, `native_paths_pass_a_bypassed_plugin_dry_anywhere_in_the_chain` |
| Processor output plus recorder sink without losing the live route | `crates/engine/src/lib.rs`: `compiler_preserves_output_when_recorder_is_on_the_validated_route`, `compiler_accepts_recorder_node_as_a_runtime_sink_boundary`; live WAV evidence in [audio continuity](2026-09-26-audio-continuity.md) |
| Built-in processor behavior on speech; file source, mixer, input switch, recording and localhost network delivery | [voice/tool qualification](2026-09-27-voice-tools.md), with private-sample/localhost opt-in commands and limitations |
| Full native source → built-in processing → Network Send → localhost UDP → Network Receive → output | [audio continuity](2026-09-26-audio-continuity.md); two 30-second runs, as recorded there |
| Saved ReaPlugs chain and shared adjacent VST2 processing worker | [shared VST2 chain](2026-09-26-shared-vst2-chain.md) and the M06 entries in [active plan](../current.md) |

These tests target distinct failure surfaces: compiler topology, DSP sample
behavior, fan-out isolation, sink preservation, and production transport.
Tests for all node permutations would be a large, misleading proxy for those
invariants. The current review adds no duplicate test because the named
regressions already exercise the supported combination classes above.

## Remaining scenario gaps

- UC-01's complete Discord/game/headphone isolation acceptance with distinct
  signals at every destination remains an M08 hardware/app gate. Existing
  engine isolation tests are digital topology evidence only.
- UC-03/06/09 require actual app identity, device removal/rebind, sleep/resume,
  reboot and user-session transitions on Windows; portable tests cannot close
  these gates.
- UC-08's disk-full/revoked-access behavior and long-duration recording remain
  open real-device qualification.
- UC-10's full CLI create/start/change/record/export/import scenario needs its
  acceptance workflow rerun on the current tree; individual API/CLI parity
  tests do not replace the end-to-end scenario.
- Network evidence is localhost-only. Two-computer delivery, firewall
  behavior and real-clock drift remain open. Network audio is unencrypted and
  the receiver filters by sender address.
- Installed-plugin voice behavior covers nine local ReaPlugs, while vendor
  compatibility, plugin rights/sandbox review and additional plugin chains
  remain open.

## Verification on 2026-09-28

- `cargo test -p audiorouter-engine -p audiorouter-transport --locked` on
  Windows x64: engine unit 141 passed, `tool_routes` 5 passed; transport unit
  22 passed and continuity detector 1 passed. Eight private-audio/live-device
  opt-ins were ignored as designed. Doc tests passed (zero cases).
- `cargo test -p audiorouter-control -p audiorouter-domain -p audiorouter-dsp
  -p audiorouter-plugin-host --locked`: passed, including plugin-chain group,
  branch-health, shared-worker, DSP, and worker-process regressions. The
  private installed-plugin/voice opt-in was ignored as designed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File
  .\tests\acceptance\docs.ps1`: passed, 67 Markdown files and 356 local links.
- `git diff --check`: passed.

These checks establish deterministic code and documentation consistency only.
The private voice and native route results cited above are the dated reports
linked in the table, not rerun during this review. No two-PC, reboot, installer,
or user-session test was performed. None of these checks establishes the M08
release gate.
