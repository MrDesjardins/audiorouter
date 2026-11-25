# Code-review-fixes residual items — 2026-11-24

Closing check for the one portable item left in
[code-review fixes](../code-review-fixes.md#synthetic-service-timing-2026-11-24)
and for the “remaining silent catches” and “remaining mechanical splits”
sections, plus the live continuity re-runs the owner authorized on
2026-11-24. Raw run logs are in
[2026-11-24-live-continuity](2026-11-24-live-continuity/).

## 1. Portable timing item: comparison with the #187 evidence

The plan’s item is “large-library concurrent Play/export/import timing on
synthetic data”. The #187 harness and record are
[2026-11-24-synthetic-service-timing.md](2026-11-24-synthetic-service-timing.md);
the code is `crates/control/src/service_timing_tests.rs` and
`crates/transport/tests/service_timing_pipe.rs`. Item by item:

| What the item asks | Evidence in #187 | Covered? |
| --- | --- | --- |
| A large library, not a two-session toy | ~400 MiB of media (25 × 16 MiB near the 512 MiB cap), 100 fillers + two export sessions toward 128 sessions/128 nodes, 2,000 recording rows | yes |
| Play timing | `play_path_media_load_and_compile`: eight 100 s stereo Audio Files, cold and warm decode under `while_servicing_audio`, plus the inline compile; 48 kHz and 44.1 kHz | yes, device-free steps only |
| Export timing through real dispatch | `res6_slow_method_timings` calls `ControlPlane::dispatch`; `pipe_quick_requests_wait_behind_bundle_work` goes through the production serve loop on a private pipe. Maximum-size compressible and noise bundles | yes |
| Import timing through real dispatch | Same two harnesses; stage, restore and the inline session insert are also timed separately | yes |
| Play/export/import overlapping | The pipe harness starts `exportFile` and a re-`importFile` together and measures the queued second request; a separate scenario runs the noise export with six concurrent quick-request clients. Export/import work runs under the production `run_while_servicing` with the real `service_running_native_audio` closure | yes for the queueing and service cadence |
| Raw numbers recorded | Three runs, medians and maxima, CSVs linked from the record | yes |

**Does RES-6 cover all three overlapping?** No. It covers Play's two
device-free steps *separately*, and export + import overlapping (queued on one
serve loop, plus concurrent quick clients). It does not run decode, export and
import at the same time. A real starting Play still needs endpoints
(`nativePaths.prepare`), so that half is the live gate. The portable
concurrency was added:

`concurrent_play_export_import_service_cadence`
([service_timing_tests.rs:1021](../../../../crates/control/src/service_timing_tests.rs)),
opt-in and `#[ignore]`, on the same ~400 MiB library. It runs Play's decode of
eight 100 s stereo files, a bundle export (prepare + ZIP) and a bundle import
(stage + restore) on three threads while the real
`service_running_native_audio` closure ticks. Two runs at scale 1:

| Run | Play decode | Export | Import | Service passes | Gap median / p99 / max | Gaps > 8 ms |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | 748 ms | 447 ms | 1,688 ms | 1,094 | 1.54 / 1.75 / 2.04 ms | 0 |
| 1 | 750 ms | 462 ms | 3,204 ms | 2,072 | 1.54 / 1.66 / 12.65 ms | **1** |

Raw rows: [concurrent timing](2026-11-24-concurrent-service-timing.csv).
Command: `$env:AUDIOROUTER_TIMING_RUNS=2;
$env:AUDIOROUTER_TIMING_CSV=<absolute path>; cargo test -p audiorouter-control
--release --lib --locked
service_timing_tests::concurrent_play_export_import_service_cadence -- --ignored
--exact --nocapture`. The median stayed at ~1.54 ms in both runs. Run 1 had
**one** pass of 12.65 ms (one gap over the 8 ms `AUDIO_SERVICE_LATE_GAP`),
which run 0 did not reproduce. That is a host stall under concurrent I/O on a
PC already running other work, not a device run; it does not open a device. An
earlier pair of runs (before a clippy-only borrow edit) measured play 753/853,
export 447/507 and import 2,148/2,520 ms with max gaps 1.88/3.20 ms and no gap
over 8 ms, so the late pass is intermittent.

No measured method approached its deadline (slowest 2.0 s of 60 s; slowest
quick call 28 ms of 5 s).

The one candidate defect the evidence found (the 2 s pipe-busy wait) is
already fixed on main with regressions: `connect_client_waiting`,
`probe_pipe` and `crates/transport/tests/busy_pipe_wait.rs`.

## 2. Remaining silent catches: confirmed in code

| Category | Where | One message per failure run? | Keeps last good answer? | Regression |
| --- | --- | --- | --- | --- |
| MCP setup, tunnel, MCP activity, backend activity, client permissions | `ui/src/useWorkbenchStatus.ts:21-88` | yes, `failed` set; each `.then` deletes its category to rearm | yes, state is only replaced on a successful read | `ui/src/useWorkbenchStatus.test.ts` (one safe category per failure run; recovery re-arms; late rejection ignored) |
| Virtual cable status | `ui/src/App.tsx:909-926` | yes, `statusFailed` latch cleared on success | yes, comment says “Keep the last answer through a failed poll (UI-17)” | `ui/src/App.accessibility.test.tsx` virtual-cable cases |
| Session inventory after a deletion | `ui/src/App.tsx:2070` | category recorded once for the refresh | last inventory retained | `ui/src/sessionInventory.test.ts` |
| OBS status | `ui/src/useObsStatus.ts:16`; `ui/src/useObsStatus.test.tsx:74` | read reports stale per read | explicitly “status still holds the last good one” | same test |

The intentional pre-App packaged-shell probe catch and the pointer-capture and
diagnostic-sink fallbacks stay as documented at their call sites; no swallow
was added by this check.

## 3. Remaining mechanical splits: confirmed in code

| Concern | Module | Caller proof |
| --- | --- | --- |
| Recording-library actions | `ui/src/useRecordingActions.ts` | imported and called at `ui/src/App.tsx:46,420` |
| Canvas connector measurement/repair | `ui/src/CanvasConnectorMeasurements.tsx` | rendered at `ui/src/SessionFlowCanvas.tsx:3899` |
| Engine branch delay compensation | `crates/engine/src/latency_compensation.rs` | `mod`/`pub use` at `crates/engine/src/lib.rs:46-47` |
| Native planar/interleaved codecs | `crates/windows-audio/src/packet_codec.rs` | `mod`/`pub use` at `crates/windows-audio/src/lib.rs:27-29` |

This is the bounded-concern pass the plan claims, not a claim that the large
compiler and endpoint files are small.

## 4. Live continuity re-runs (owner authorized 2026-11-24)

### Deviation: the runs were not proven isolated

The brief for this slot said to skip if any app was busy or the owner might be
in a call. Six 20 s tones were rendered into CABLE-A Input and captured from
CABLE-A Output before that was proven. CABLE-A Output is the Windows default
communications capture, and Discord and Zoom were running. That is a
deviation from the brief; it is recorded rather than excused. No further audio
was run after the failures, and none will run until the owner gives a quiet
window.

Exact wall-clock windows (UTC; local was PST, 2026-11-24 evening), from the
PowerShell log files’ creation and last-write times:

| Run | UTC start | UTC end | Result |
| --- | --- | --- | --- |
| gain-1 | 2026-11-25 02:20:49 | 02:21:11 | pass |
| gain-2 | 02:21:28 | 02:21:51 | pass |
| gain-3 | 02:21:51 | 02:22:13 | fail |
| chain-1 | 02:22:13 | 02:22:35 | fail |
| chain-2 | 02:22:35 | 02:22:57 | pass |
| chain-3 | 02:22:57 | 02:23:20 | fail |

Other load on the machine: `VirtualBox`, `VBoxSVC` and `VBoxSDS` were running
(a guest, as the manager reported), together with several `node` processes from
other worktree sessions. The two passing runs that showed an ~82 ms service
pass (gain-2, chain-2) are consistent with host-wide stalls; the failing runs
did not have one.

### Preconditions and limits

- No `audiorouter-shell`, `audiorouter-cli` or other AudioRouter process was
  running (`Get-Process` returned none) before and between runs.
- Endpoints used, all 48 kHz float32 stereo: tone render CABLE Input
  `{0.0.0.00000000}.{71f96f14-…}`; route capture CABLE Output
  `{0.0.1.00000000}.{06268191-…}`; route render CABLE-A Input
  `{0.0.0.00000000}.{03498a2a-…}`; result capture CABLE-A Output
  `{0.0.1.00000000}.{ef18e30a-…}`.
- CABLE-B Input remains 8-channel, so the stereo worker refuses it. No
  CABLE-B result run was attempted; that is the open owner decision, not a
  skipped run.
- CABLE-A Output is Windows’ default communications capture. Discord (a text
  channel was shown, no voice window) and Zoom were running. Whether a call or
  another consumer held CABLE-A Output could not be established, so the runs
  were **not provably isolated**; they were stopped after the failures below.
- Harness: `cargo test -p audiorouter-transport --test live_audio_continuity
  --release --locked -- live::live_backend_service_keeps_a_routed_tone_continuous
  --ignored --exact --nocapture`, 20 s per run, in-process disposable plane on a
  private pipe, no recording root, no plugins.

### Results

| Run | Chain | Reference (CABLE Output) | Routed result (CABLE-A Output) | Service thread |
| --- | --- | --- | --- | --- |
| gain-1 | `gain` | 0 glitches, 0 silent, 0 flags | 0 glitches, 0 silent, 0 flags | lateGaps 0, maxGap 4.158 ms, pump max 0.213 ms |
| gain-2 | `gain` | 0, 0, 0 | 0, 0, 0 | lateGaps 3, maxGap 82.689 ms, pump max 82.497 ms (one slow pass) |
| gain-3 | `gain` | 0, 0, 0 | **2 glitches, 1 silent run (527 samples ≈ 11 ms)**, 0 flags, at 18.389/18.400 s | lateGaps 0, maxGap 3.062 ms, pump max 0.110 ms |
| chain-1 | 10-tool chain | 0, 0, 0 | **2 glitches, 1 silent run (544 samples ≈ 11 ms)** | lateGaps 0, maxGap 3.095 ms, pump max 0.905 ms |
| chain-2 | 10-tool chain | 0, 0, 0 | 0, 0, 0 | lateGaps 1, maxGap 82.640 ms, pump max 82.299 ms (one slow pass) |
| chain-3 | 10-tool chain | 0, 0, 0 | **2 glitches, 1 silent run (578 samples ≈ 12 ms)** | lateGaps 0, maxGap 3.204 ms, pump max 0.895 ms |

Latency samples: source 12.1–19.9 ms, destination 22.5–28.1 ms; `output
underruns: 1` (startup/teardown) in every run.

### Reading the failures

- The reference path, which contains no AudioRouter code, was clean in every
  run. The routed result lost one ~11–12 ms contiguous block in three of six
  runs, always with **zero WASAPI discontinuity flags**.
- The three failing runs had a healthy service thread (0 late gaps, max gap
  ≈ 3 ms, pump < 1 ms). Two passing runs had an ~82 ms slow service pass and
  one had three late gaps — yet no routed silence. Service-thread lateness is
  therefore not the cause of these glitches.
- The failures fit the earlier 2026-11-02 pattern recorded in the plan when the
  owner’s route or another consumer shared these cables: the result capture
  reads a short silence while the reference stays clean. It is **not**
  attributed to the audio engine here and no product change was made.
- This is a **failed** automated continuity assertion under conditions that
  are not provably isolated, not a passed gate. It needs a re-run with the
  owner confirming that no call/game holds the cables, and with CABLE-A Output
  free. The CABLE-B 8-channel decision is separate.

### Where the ~11 ms gap entered

The three failures lost one contiguous silent block, framed by exactly two
sample-level discontinuities:

| Run | Glitch times (s) | Silent samples | Silent duration | One 10 ms device period? |
| --- | --- | --- | --- | --- |
| gain-3 | 18.389 and 18.400 | 527 | 10.98 ms | 10 ms + ~1 ms |
| chain-1 | 1.189 and 1.200 | 544 | 11.33 ms | 10 ms + ~1.3 ms |
| chain-3 | 11.477 and 11.489 | 578 | 12.04 ms | 10 ms + ~2 ms |

Every endpoint’s `default100ns` is 100,000, i.e. a 10 ms shared-mode period
(`devices list` in the same session). The gap is one period plus the two
boundary samples, so it is the shape of one capture packet lost or zeroed —
not a drift slip and not several periods.

Which side:

- `output underruns: 1` in every run, the startup/teardown one. A route output
  underrun renders exact digital silence and increments that counter (the
  harness’s own `report` comment says so). The failing runs did not increment
  it.
- The failing runs had 0 audio-service `lateGaps` and a 3.06–3.20 ms maximum
  gap; the pump maxima were 0.11–0.91 ms. The two *passing* runs had the only
  ~82 ms slow service pass. So the service thread and the output pump were
  on time in the failures.
- The result capture reported 0 WASAPI discontinuity flags in the failures.
  A lost capture *packet flag* would normally set `DATA_DISCONTINUITY`; a
  silence produced inside the shared-mode engine (or a capture stream that
  another client disturbed) need not.
- The reference capture, which contains no AudioRouter code, was clean.

Hypothesis, not a conclusion (manager review): these facts point to the
**result-capture side of the harness** (a shared capture of CABLE-A Output)
rather than the route’s output pump or the audio-service thread, but no run
isolates it, so the product is not cleared and the result stays failed. Why
the captured stream had one silent period is not established; another client on the default communications capture is
the leading possibility, consistent with the unproven isolation above.

Comparison with the 2026-11-23 health log: on the owner’s real CABLE-A route at
a 10 ms cushion, `voice-to-cable-a` recorded 20 underruns outside stalls in
41.3 h (0.48 an hour), and every one was counted by that output’s counter. The
failing runs here did not count one. Earlier passing runs on this CABLE-A pair
(2026-11-02, 20 s, gain and ten tools) were clean; the 2026-11-02 repeat
failures were traced to the owner’s running route forming a loop, and their
reference path also glitched in 2 of 3 runs. Here no shell ran and the
reference was clean, so the loop explanation does not apply.

### Harness counters added

`live_audio_continuity` now prints one line per second during measurement:

```text
t=<s>s utc_ms=<epoch ms> outputQueueMs=<destination timing.delayMs>
outputUnderruns=<total> (+<delta>) submittedQuanta=<total> (+<delta>)
cpuBusyPercent=<host busy %> audioThread=<passes/lateGaps/maxGapMicros/...>
```

The queue is the destination node’s `timing.delayMs` from `system.diagnostics`;
the counters come from `nativeMultiInputs.pump`; host CPU is between
`GetSystemTimes` samples; the timestamp is UTC epoch ms. It is test-only
logging, no product behaviour change, and it compiles in the release test
binary. It was not run against audio (the quiet-window rule); the next isolated
run will show the queue and underrun delta at the failing second.

## 5. Remaining prerequisites

- **CABLE-A isolated repeat:** owner confirms no call, stream or other app on
  CABLE-A Output (currently the default communications capture), then the same
  six commands; three clean runs per chain are the pass criterion. A
  reproducible failure with an isolated endpoint becomes a product defect to
  investigate (sample the result capture and the worker counters at the glitch
  offset).
- **CABLE-B result runs:** owner decision on the 8-channel output. Until the
  worker supports it, no valid CABLE-B result endpoint exists on this PC.
- **Large-library live load with a playing route:** not created here; the
  synthetic half is in the #187 record.
- **Latency-changing commercial plugin:** not run. Installed non-Blue-Cat
  candidates exist (UADx 610/Pure Plate/Polymax/Century, TDR Nova), but
  licensing and the exact plugin are the owner’s choice, and UADx needs a
  signed-in iLok session. UADx effects returned input unchanged in the
  2026-10-22 qualification, so a substitute would not be valid evidence.