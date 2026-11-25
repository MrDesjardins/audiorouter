# Q5.4 synthetic whole-app soak and Q5.5 local UI baselines

Date: 2026-11-24. Requirements: NFR-04/05/06/08/11/14, ENG-02/03/04, UI-17.
Offline Windows work only: no audio endpoint, route start, installer, driver or
device change. The local runs used a debug shell and a disposable database,
profile and default control pipe. No other AudioRouter instance was running
(checked by process name and `\\.\pipe\audiorouter-control`).

## Q5.4 tooling

- `tools/quality/run-app-soak.ps1 -Synthetic` launches the packaged shell on a
  fresh disposable database, samples shell/WebView private bytes, working set,
  handles, threads, CPU and log bytes every 10 s, and stops only the processes
  it started. It requires the default control pipe to be free (setting
  `AUDIOROUTER_CONTROL_PIPE` would suppress the shell's owned backend) and
  clears that variable for the child. It does not press Play and does not grant
  device administration.
- `tools/quality/packaged-shell-synthetic.mjs` connects over loopback CDP and
  performs device-free graph edits (`nodes.add` after the input, `nodes.set`
  parameter/enabled edits, one `nodes.remove` with bridge), idle status polling,
  and a UI reload every 2 minutes. It builds the graph to 64 nodes, times the
  last edit to the painted 64-node frame, and records a post-GC JS heap sample at
  start and end. It never starts audio.
- `tools/quality/app-soak-analysis.mjs` holds the verdict logic (least-squares
  slope per hour; the live soak's 5/20 MiB/h, 20/60 MiB/h, 50/200 handles/h,
  8-thread-spread and 24-hour log-history limits). Log history is measured after
  warm-up, so first-run startup writes do not look like a flood. The live and
  synthetic modes share it. In `--mode short`, the table still shows the per-hour
  verdicts but the overall result is INFO, not an endurance FAIL; a dead shell
  still fails. Ten tests cover passing and leaking data, a log flood, too few
  samples, CSV parsing and the short/endurance exit codes.
- `run-app-soak.ps1` now calls the Node analyzer for both modes instead of its
  own copy of the formulas. `-SyntheticMinutes` defaults to 3; a longer value is
  the opt-in. The non-synthetic live soak is still `-Hours 8` with its audio
  play step.

## 10-minute synthetic run (recorded short run, not endurance)

Command (debug shell built with `cargo build --manifest-path
src-tauri/Cargo.toml --features custom-protocol`, `ui/dist` current; shell
SHA-256 starts `DFCA6AF3582EA259`):

```
powershell -NoProfile -ExecutionPolicy Bypass -File tools\quality\run-app-soak.ps1 ^
  -Synthetic -SyntheticMinutes 10 ^
  -Shell src-tauri\target\debug\audiorouter-shell.exe -Output <dir>
```

Result: exit 0, 10.0 min, 59 samples (53 after the 60 s warm-up), shell alive,
no real user data changed (isolation check), 0 error lines in either log.
Activity: 1803 graph edits, 4 UI reload restarts, 0 backend RPC errors, 0
uncaught page errors, last node count 64, one remove test passed, 64-node
input-to-paint **623 ms**.

| Check | Value | Verdict in this short run |
| --- | ---: | --- |
| Shell private bytes slope after warm-up | 86.78 MiB/h (84.3 MiB at end) | FAIL vs the 5/20 endurance limits |
| WebView private slope | -1219.46 MiB/h (232.3 MiB at end, 5 processes) | PASS (the drop followed the reload/GC) |
| Shell handles slope | -244.1/h (438 at end) | PASS |
| Threads spread after warm-up | 3 (48 at end) | PASS |
| Backend log history at this rate | 1.8 h | FAIL vs the 24 h target |
| Shell log history at this rate | 2.3 h | FAIL vs the 24 h target |
| Post-GC JS heap | 10.8 MiB at start; end unavailable in this run (old `Runtime.getHeapUsage` call returned no value) | INFO |

The per-hour verdicts are **not** endurance results: the synthetic driver made
1803 commits and wrote the matching audit log lines, and the shell was a debug
build with DevTools open. The log-history numbers measure that workload, not an
idle product. The only growth signal is shell private +13 MiB over 10 minutes
under that edit rate; handles fell and the WebView private fell after reload, so
it is not a simple leak, but a longer run should confirm it. No gate is set.

A corrected 3-minute run (same tools, `Runtime`/`Performance` heap fixed) recorded
the missing heap pair: 601 edits, 1 reload, 0 RPC/page errors, 64 nodes,
64-node input-to-paint 432 ms, **post-GC JS heap 10.7 -> 20.0 MiB**, shell
private 88.0 MiB, WebView private 739.8 MiB, handles 503, threads 49, 0 error
lines. Its slopes were 291.80 MiB/h shell, 3907.63 MiB/h WebView and 94.5
handles/h; they are dominated by the 64-node build in a 3-minute window and are
reported as short-run observations only.

The heap-end bug was fixed by collecting `JSHeapUsedSize` through
`Performance.getMetrics` after `HeapProfiler.collectGarbage`; both the driver and
the Q5.5 spec now use it.

## Q5.5 local UI baselines (reported, not gated)

Command from `ui/` with the dev server and Edge:

```
$env:AUDIOROUTER_PERF_BASELINE='1'
node node_modules\@playwright\test\cli.js test --config=playwright.config.ts ^
  e2e/performance-baseline.pw.ts --workers=1
```

`ui/e2e/performance-baseline.pw.ts` injects a 64-node device-free fixture with
synthetic 20 Hz telemetry, loads `/route-harness.html`, and records:

| Measure | Local baseline |
| --- | ---: |
| 64-node input-to-paint (route-harness load; dev server) | 1617 ms |
| Telemetry display changes in 3 s | 56 (about 18.7 Hz) |
| Telemetry gap, min / median / p95 / max | 33.6 / 47.9 / 80.1 / 100.3 ms |
| Post-GC JS heap (CDP `JSHeapUsedSize`) | 31.9 MiB |
| Browser process private | 54.6 MiB |
| Renderer private, all renderers (2) | 558.2 MiB (16.2 + 542.0) |
| GPU process private | 334.3 MiB |
| Network / storage private | 32.6 / 7.7 MiB |

The renderer private figure is the sum of all renderers from the browser-target
`SystemInfo.getProcessInfo`; this record does not attribute which renderer is the
page, and the 542 MiB one is the likely page/HMR renderer. That is a large gap
from the 31.9 MiB post-GC JS heap and is expected to include DOM, canvas and
Edge/React Flow overhead; it is not a leak finding. These are a development
Edge/dev-server baseline, not the packaged WebView: the packaged shell's memory
is in the soak above and the 8-hour live soak remains the endurance harness.
No threshold is asserted; only 64 nodes and at least one telemetry change are
required so a broken harness fails instead of recording zero.

## Q4.4 verification against M02/M04 evidence

| Q4.4 sub-item | Covered by | Where |
| --- | --- | --- |
| Deterministic continuity, silence, sample fidelity | `crates/engine/tests/deterministic_audio.rs`: exact gain/mute levels, delay markers, effect bypass nulls, bit-for-bit fresh preparation, silent-channel isolation, mixer/input-switch signals; [deterministic audio](2026-09-30-deterministic-audio.md), [continuity](2026-09-26-audio-continuity.md) | M02/M04 |
| NaN/Inf rejection | `non_finite_samples_are_silenced_and_counted`, `non_finite_gain_is_safe_silence`, `linear_resampler_repairs_non_finite_source_samples`, `compiled_branch_paths_sanitize_non_finite_sink_samples` (engine/src/lib.rs) | M02 |
| Clock mismatch | `drift_controller_keeps_both_clock_mismatches_bounded_for_eight_hours`, `a_path_tap_sees_contiguous_frames_while_another_path_runs_on_its_own_clock`; `drift_sim_tests.rs` matched clocks, stalls, four devices for ten minutes, late passes | M02, [latency repair](2026-11-11-audio-latency-repair.md) |
| Queue bounds | `bounded_audio_queue_is_nonblocking_and_explicit_when_full`, `audio_queue_constructors_reject_unbounded_capacity_before_allocation`, `pcm16_quantum_adapter_accumulates_split_packets_with_backpressure`, recorder queue bounds, `sidechain_handoff_never_allocates_for_ready_late_or_full_queues` | M02/M04 |
| Bypass | `effect_bypass_nulls_against_input_including_high_latency_tools`; `tool_chains_compile_process_and_apply_flags_live_in_every_layout` | M02 |
| Graph replacement | `recompiling_the_same_combination_gives_identical_audio`, `direct_mono_bass_treble_path_shapes_tones_after_live_replacement`, `scheduler_publish_recycles_queued_generation_before_replacement`, `scheduler_lifecycle_discards_queued_input_across_generation_boundaries` | M02 |
| Every worker kind and mixer topology | `crates/engine/tests/source_combinations.rs` (every source alone, fan-out, all pairs, all triples in connected mixers), `tool_combinations.rs` (every tool in many chains/positions/flags in single and connected Mixers), M02 live worker evidence | M02, owner rule |
| Burst producers, delayed consumers, stalls, restarts | Bounded multi-input packet slices (2026-09-17 lesson), render burst checks, drift-simulation stalls/late passes, scheduler generation restarts, [render publication review](2026-10-09-m03-render-publication-review.md), live continuity | M02 |
| Live endpoint evidence | The same suites run on declared endpoints in M02/M03 evidence; the current live continuity gate is Q4.6 (hardware, open) | M02/M03 |

Verdict: Q4.4's offline/deterministic scope is covered by named tests and M02/M04
evidence. The only open sub-item is live continuity on the owner's declared
endpoints (Q4.6), which this session must not run.

## Q4.5 verification against M04 evidence

| Q4.5 sub-item | Covered by | Where |
| --- | --- | --- |
| Parse every recording: header, frame count | `test_support::assert_playable_wav` (parses RIFF/data and frame count) used by one-click and network tests; `wav_inspector_reports_file_shape_and_rejects_truncation`, `flac_inspection_reads_streaminfo_and_library_status`, `read_wav_metadata`/`read_flac_metadata`, `recovery_patches_header_and_drops_partial_frame` | M04 |
| Failure intervals | `one_click_recording_survives_dropped_and_repeated_blocks_and_stays_playable` (stall, dropped and repeated blocks, then parsed), `one_click_flac_and_mp3_keep_a_finished_file_after_lost_audio`, threaded-recorder staged finalize tests, the 2026-10-01 recording-integrity lesson in [AGENTS.md](../../../../AGENTS.md) | M04 |
| Multi-path fan-out | `independent_recorder_nodes_bind_distinct_workers_and_taps`, `recorder_branch_accepted_by_multi_input_fanout` (M02 evidence 2026-09-16), `recorder_branch_plays_without_a_file_and_records_once_one_is_attached`, Q4.2 recording fault cases | M04/M02 |
| Disk/write failure while unrelated routes continue | `failed_node_recorder_does_not_stop_healthy_sibling` (failing tap recorder + healthy writer), `writer_flush_failure_fails_wav_worker_before_checkpoint`, `checkpoint_persistence_failure_fails_wav_worker`, `segmented_wav_recorder_fails_closed_on_segment_flush_error` | M04 |
| File existence/size is not acceptance | Every one-click test calls `assert_playable_wav`; the 2026-10-01 lesson added it because "file > 44 bytes" passed a broken multi-path take | M04 |
| Bounded parsers | `crates/recording/tests/bounded_fuzz.rs` mutates WAV/FLAC and recovery inputs; no panic/hang, repaired files inspect | Q4.3 |

Verdict: Q4.5's offline scope is covered. A real disk-full volume and a live
multi-device recording remain hardware cases; the current tests inject the
failure below the filesystem and verify the sibling route continues.

## Full UI unit suite (config change verification)

`npm --prefix ui test` after the Playwright config change: **136 files, 1191
tests passed**, 35.47 s. `typecheck:all`, `lint` (0 warnings) and `format:check`
also pass.

## Not run

- No live audio, device, driver, installer or VM action; no Play in any run.
- The packaged release shell was not built; the synthetic runs used a debug shell
  with DevTools. A release-mode synthetic run would change the memory baseline.
- The synthetic soak is not scheduled in CI; it is a local/opt-in tool. The
  Q5.5 spec is skipped unless `AUDIOROUTER_PERF_BASELINE=1`.
- The 10-minute run's post-GC end value is unavailable (old heap call). The
  corrected 3-minute run is the heap pair above; the 10-minute run's other
  metrics are valid.

## Rollback

Delete `app-soak-analysis.mjs` and its test, `packaged-shell-synthetic.mjs`, the
`-Synthetic` parameters/branches in `run-app-soak.ps1`, and
`ui/e2e/performance-baseline.pw.ts`; restore the PowerShell analyzer block if the
live soak's own formulas must return. No product or user state is touched by the
tools.