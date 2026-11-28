# Live-audio reliability gates C1/C2/G8 - preparation and log audit (2026-11-27)

Owner request: own C1, C2 and G8 in
[human-gated tasks](../human-gated-tasks.md) and
[code-review fixes](../code-review-fixes.md), audit the original failures first,
then prepare instrumentation and harness tests before asking for a quiet window.
No driver change, no pull request, no publication, no eight-hour soak and no
live tone in this work.

This record contains **no live-audio result**. It audits the failed 2026-11-24
runs, explains what the old logs cannot show, and records the test-only
instrumentation and runbooks prepared for a later quiet window.

## 1. Audit of the 2026-11-24 C1 failures

Source logs: [2026-11-24 live continuity](2026-11-24-live-continuity/).
Plan text: [code-review fixes, live re-runs](../code-review-fixes.md#residual-check-and-live-re-runs-2026-11-24).

Harness: `cargo test -p audiorouter-transport --test live_audio_continuity
--release --locked -- live::live_backend_service_keeps_a_routed_tone_continuous
--ignored --exact --nocapture`, 20 s per run, private pipe, in-memory plane, no
plugins, multi-input (`paths`) worker.

### Exact result

| Run | Chain | Reference (CABLE Output) | Routed result (CABLE-A Output) | Audio service |
| --- | --- | --- | --- | --- |
| gain-1 | `gain` | 0 glitches, 0 silent, 0 flags | 0, 0, 0 | lateGaps 0, maxGap 4.158 ms, pump 0.213 ms |
| gain-2 | `gain` | 0, 0, 0 | 0, 0, 0 | lateGaps 3, maxGap 82.689 ms, pump 82.497 ms |
| gain-3 | `gain` | 0, 0, 0 | **2 glitches, 527 silent samples**, 0 flags, at 18.389/18.400 s | lateGaps 0, maxGap 3.062 ms, pump 0.110 ms |
| chain-1 | ten tools | 0, 0, 0 | **2 glitches, 544 silent samples**, 0 flags, at 1.189/1.200 s | lateGaps 0, maxGap 3.095 ms, pump 0.905 ms |
| chain-2 | ten tools | 0, 0, 0 | 0, 0, 0 | lateGaps 1, maxGap 82.640 ms, pump 82.299 ms |
| chain-3 | ten tools | 0, 0, 0 | **2 glitches, 578 silent samples**, 0 flags, at 11.477/11.489 s | lateGaps 0, maxGap 3.204 ms, pump 0.895 ms |

Ten-tool chain: `gain,parametricEq,graphicEq,bassTreble,dehum,compressor,gate,limiter,delay,meter`.
`output underruns: 1` (the startup count) in every run. Silent durations were
10.98, 11.33 and 12.04 ms; every involved endpoint reports a 10 ms shared-mode
period, so each gap is one device packet plus the two boundary samples.
Boundary residuals were 0.27x, 1.00x and 0.61x amplitude.

### Facts, and what they do not establish

- The reference path, which contains no AudioRouter code, was clean in all six
  runs. The routed result lost one contiguous block in three of six runs.
- The three failing runs had a healthy service thread (0 late gaps, 3.06-3.20 ms
  maximum gap, pump under 1 ms). The two passing runs had the only ~82 ms slow
  service pass. Service-thread lateness is not the cause of these glitches.
- `outputUnderruns` did not increase in any failing run. A render output that
  underran would both render digital silence and increment that counter.
- The result capture reported no WASAPI `DATA_DISCONTINUITY` flag.
- The runs were **not provably isolated**: CABLE-A Output is the Windows default
  communications capture, Discord and Zoom were open, and a VirtualBox guest and
  other worktree `node` processes were running. The earlier 2026-11-02 failures
  were traced to the owner's running route looping the cables; that explanation
  does not apply here (no shell ran and the reference was clean), but neither
  does the evidence prove isolation.
- The logs label the result "CABLE-B Output" even though the 2026-11-24 runs used
  CABLE-A through the `AUDIOROUTER_CONTINUITY_*_ID` overrides. That is a harness
  label defect, not a product behavior; the report functions now keep the fixed
  label but the runbook states the actual endpoints.
- The failure is **not reproducible in isolation** and is not attributed to the
  engine, the output pump or the harness. The result stays **failed**, not passed.

### Where the gap can enter, and how the next run distinguishes it

The old logs cannot separate these because all three appear as 480 zero samples:

1. **Capture-side loss.** The harness capture stream missed or was zeroed for one
   device packet. Shared-mode capture reports a contiguous stream otherwise; a
   lost packet moves `device_position` forward by the missing frames.
2. **Render-side silence without an underrun.** The route wrote a silent packet
   while its device buffer never read empty. `device_position` stays contiguous,
   samples are zeros, and `outputUnderruns` stays unchanged.
3. **Output-pump underrun missed by the counter.** The pump drains a partial
   `pending` block before reading `queued_frames`, so an empty device buffer that
   is refilled in the same pass can be heard but not counted. This is a candidate
   only: no isolated run has reproduced it, and no product change is justified
   yet.
4. **Another client or the device engine inserted silence.** A read-only
   session preflight now proves whether any process held an active session on the
   four endpoints at the moment before a run.

The new per-run fields answer this directly: `capture position: N gaps` and
`SILENT-flagged packets` are capture-side evidence; `output underruns during
measurement` is render-side evidence; a contiguous capture with a silent run and
no underrun points at the route or the device engine.

## 2. Instrumentation added (test-only)

All in `crates/transport/tests/live_audio_continuity.rs`.

### Capture provenance

`Recording` now counts, per measured packet:

- `capture_step(previous, current)` classifies `(device_position, frames)` as
  continuous, lost (position jumped forward, with the lost frame count),
  overlapping/rewound, or unknown (position 0). A pure unit test covers a
  contiguous stream, one missing 480-frame packet, a short jump, a rewind, and a
  zero position.
- `SILENT`-flagged packets (`AUDCLNT_BUFFERFLAGS_SILENT`) are counted separately
  from `DATA_DISCONTINUITY`. The recorder already zero-filled a SILENT packet, so
  this explains zeros that carry no discontinuity flag without guessing.
- `report` prints a capture-position line, or "contiguous", for every recording.

### Measurement window and dumps

- The main run takes one final `nativeMultiInputs.pump` read after measurement and
  prints `output underruns during measurement: N (start, end)`; the earlier total
  could not exclude the startup underrun.
- `AUDIOROUTER_CONTINUITY_DUMP_PREFIX` prefixes the raw `.f32` dump, so an active
  and a bypassed G8 run can keep both recordings.

### Read-only endpoint preflight

`live::live_continuity_endpoints_are_unused_preflight`
(`AUDIOROUTER_CONTINUITY_PREFLIGHT=1`) resolves the four endpoints and calls
`enumerate_application_audio()`, which inspects Windows session managers without
opening or starting any audio client. It prints each endpoint's active session
count and fails with the owning PIDs and display names if any is active. It
opens no stream and renders nothing.

### C2 privacy mute

`live::live_privacy_mute_silences_and_resumes_a_running_route`
(`AUDIOROUTER_LIVE_PRIVACY_MUTE=1`) builds a disposable virtual-cable route,
plays the harness tone, samples the destination meter before mute, during mute
and after clearing it, and asserts:

- the tone is present before mute and drops by at least 30 dB while muted;
- `system.diagnostics.audio.state` stays `running` and the same native session
  stays attached;
- a pump with the original generation still succeeds;
- clearing the latch restores the level within 10 dB without Stop/Play;
- `outputUnderruns` does not increase (privacy silence is not an underrun).

It uses only the physical-input tone path, not a microphone.

### G8 licensed plugin

- `AUDIOROUTER_CONTINUITY_FORCE_BYPASS=1` forces saved plugin nodes bypassed;
  `AUDIOROUTER_CONTINUITY_CLEAR_BYPASS=1` (existing) forces them active, so one
  copied session serves both runs.
- `AUDIOROUTER_CONTINUITY_EXPECT_PLUGIN_DELAY_MS` requires the sum of
  plugin-node `timing.delayMs` to reach the owner-supplied value.
- `AUDIOROUTER_CONTINUITY_REQUIRE_PLUGIN_RUNNING=1` requires every plugin worker
  health state to be `running`.
- `AUDIOROUTER_CONTINUITY_EXPECT_DIFFERENT_FROM=<bypass routed.f32>` compares the
  active steady region to the bypassed dump with a relative-RMS difference and
  fails when they are sample-identical. A portable unit test covers identity,
  +6 dB and inversion. This is a lower bound ("the samples changed"); the owner's
  ears still judge whether the change is useful.

## 3. Portable verification (this branch)

Windows MSVC, pinned Rust 1.96.0, `t3/live-audio-reliability-gates`:

- `cargo test -p audiorouter-transport --test live_audio_continuity --locked`:
  3 passed, 0 failed, 12 ignored. The three portable tests are the glitch
  detector, the capture-position classifier and the recording difference.
- `cargo test -p audiorouter-transport --test live_audio_continuity --no-run
  --locked`: compiles with no warnings.
- `cargo clippy -p audiorouter-transport --all-targets --all-features --locked
  -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed; only the harness file changed.
- Read-only preflight, run once at preparation time:
  `AUDIOROUTER_CONTINUITY_PREFLIGHT=1 cargo test ... live::live_continuity_endpoints_are_unused_preflight
  -- --ignored --exact --nocapture` printed 0 active sessions on all four
  endpoints and passed. **This is a snapshot, not a quiet-window proof.** Discord
  was running at the time; no tone was rendered.

## 4. Prepared C1 run (needs a quiet window)

Before **each** run, with the owner having closed Discord, Zoom, games and
AudioRouter, and having confirmed nobody is listening to CABLE-A:

```powershell
# Read-only proof that the four endpoints have no active session now.
$env:AUDIOROUTER_CONTINUITY_PREFLIGHT='1'
cargo test -p audiorouter-transport --test live_audio_continuity --locked `
  live::live_continuity_endpoints_are_unused_preflight -- --ignored --exact --nocapture
```

Then clear the preflight variable and run three `gain` and three ten-tool runs:

```powershell
$env:CARGO_BUILD_JOBS=4
cargo test -p audiorouter-transport --test live_audio_continuity --release --locked --no-run
$env:AUDIOROUTER_LIVE_CONTINUITY='1'; $env:AUDIOROUTER_CONTINUITY_SECONDS='20'
$env:AUDIOROUTER_CONTINUITY_TONE_RENDER_ID='{0.0.0.00000000}.{71f96f14-94c1-4189-b9b9-df8c960db8f2}'
$env:AUDIOROUTER_CONTINUITY_ROUTE_CAPTURE_ID='{0.0.1.00000000}.{06268191-5f8c-42ed-827e-d3c7a19637ed}'
$env:AUDIOROUTER_CONTINUITY_ROUTE_RENDER_ID='{0.0.0.00000000}.{03498a2a-ef6a-4dd4-b9ba-72e22f567d7a}'
$env:AUDIOROUTER_CONTINUITY_RESULT_CAPTURE_ID='{0.0.1.00000000}.{ef18e30a-69c9-4299-bae2-1a2452fb8e4f}'
$env:AUDIOROUTER_CONTINUITY_CHAIN='gain'
cargo test -p audiorouter-transport --test live_audio_continuity --release --locked `
  live::live_backend_service_keeps_a_routed_tone_continuous -- --ignored --exact --nocapture
```

Repeat for `gain,parametricEq,graphicEq,bassTreble,dehum,compressor,gate,limiter,delay,meter`.
Pass is three of three per chain, zero routed glitches and zero silent runs, with
a clean reference. Any glitch is read from the new lines first:

- `capture position: ... gaps` or `... SILENT-flagged packets` -> capture side.
- contiguous capture + silent run + `output underruns during measurement: 0` ->
  route or device engine, to trace at the reported second.
- `output underruns during measurement: N>0` -> output pump path.

Do not start this run until the owner says the cables are free and the preflight
is repeated for each run. The current request does not establish quietness.

## 5. Prepared C2 scenarios

C2 needs the owner's presence for unplugging and sleep. Prepared now:

1. **Privacy mute** (live test above): `AUDIOROUTER_LIVE_PRIVACY_MUTE=1` with the
   tone/route endpoint variables. Pass: silence, same running route, restore on
   clear, no underrun.
2. **UDP network under controlled load** (existing harness): add
   `AUDIOROUTER_CONTINUITY_NETWORK=39001` and
   `AUDIOROUTER_CONTINUITY_STRESS=<start s>,<seconds>,<threads>` to the C1 command.
   Pass: zero routed glitches; the printed `net-receive` telemetry reports no
   lost/replayed packets during the quiet baseline. UDP loss under deliberate CPU
   saturation is recorded, not hidden.
3. **Kill a plugin worker on a protected path.** Use a copied session with the
   plugin (`AUDIOROUTER_CONTINUITY_PLUGIN_DB`, `_PLUGIN_SESSION`, `_PLUGINS`,
   `AUDIOROUTER_PLUGIN_WORKER_PATH`), start the route, then stop the worker
   process by its exact PID (`Get-Process audiorouter-vst3-worker` /
   `audiorouter-vst2-worker`). Owner verifies the protected path is silent until a
   deliberate Stop + Play. No automatic restart is accepted as passing.
4. **Remove and return an endpoint; sleep and resume.** Owner action only. The
   agent must not invoke sleep or stop the owner's app. Expected from the plan:
   protected paths stay silent until deliberate Stop + Play, nothing starts by
   itself, Windows defaults and volumes are unchanged.
5. **Sidechain plugin.** Same kill/stop-check as 3 on a route with a sidechain
   input; requires a saved session with the sidechain wired.

## 6. Prepared G8 run (needs the owner's plugin and sign-in)

The owner names the licensed non-Blue-Cat plugin, signs in, and supplies a COPY
of the session database and the built worker path. Do not use Blue Cat
unattended and do not change licensing or sandbox policy.

```powershell
$env:AUDIOROUTER_LIVE_CONTINUITY='1'; $env:AUDIOROUTER_CONTINUITY_SECONDS='20'
$env:AUDIOROUTER_CONTINUITY_PLUGIN_DB='<copy of the session database>'
$env:AUDIOROUTER_CONTINUITY_PLUGIN_SESSION='<session id>'
$env:AUDIOROUTER_CONTINUITY_PLUGINS='<plugin node id>'
$env:AUDIOROUTER_PLUGIN_WORKER_PATH='<built worker exe>'
$env:AUDIOROUTER_CONTINUITY_DUMP_DIR='<new empty folder>'

# 1) Bypassed baseline.
$env:AUDIOROUTER_CONTINUITY_FORCE_BYPASS='1'
Remove-Item Env:AUDIOROUTER_CONTINUITY_CLEAR_BYPASS -ErrorAction SilentlyContinue
$env:AUDIOROUTER_CONTINUITY_DUMP_PREFIX='bypass-'
Remove-Item Env:AUDIOROUTER_CONTINUITY_EXPECT_PLUGIN_DELAY_MS -ErrorAction SilentlyContinue
# run live::live_backend_service_keeps_a_routed_tone_continuous (C1 command shape)

# 2) Active run, compared with the bypassed dump.
Remove-Item Env:AUDIOROUTER_CONTINUITY_FORCE_BYPASS
$env:AUDIOROUTER_CONTINUITY_CLEAR_BYPASS='1'
$env:AUDIOROUTER_CONTINUITY_DUMP_PREFIX='active-'
$env:AUDIOROUTER_CONTINUITY_EXPECT_PLUGIN_DELAY_MS='1'
$env:AUDIOROUTER_CONTINUITY_REQUIRE_PLUGIN_RUNNING='1'
$env:AUDIOROUTER_CONTINUITY_EXPECT_DIFFERENT_FROM='<folder>\bypass-routed.f32'
# run the same test
```

Pass: the plugin reports latency, every worker is running, the routed tone is
continuous, the active samples differ from the bypassed dump, and the owner hears
a difference. UADx is not a valid substitute without a fresh qualification: the
2026-10-22 run returned input unchanged.

## 7. Remaining human gates

- C1 stays **failed** until three isolated runs per chain pass. Instrumentation
  and a preflight are ready; the quiet window is not.
- C2 stays **open**: privacy mute and UDP load are ready; unplug, return, sleep,
  resume and worker kill need the owner present.
- G8 stays **open**: the plugin choice, sign-in, session copy and worker path are
  the owner's; the harness asserts latency, running state and a changed signal.
- No driver, licensing, sandbox, release or publication work is included.

## 8. Files and rollback

Changed: `crates/transport/tests/live_audio_continuity.rs` (test-only
instrumentation and opt-in tests). Rollback is a revert of that file; no
product, API, schema, storage or audio-policy change is included.
