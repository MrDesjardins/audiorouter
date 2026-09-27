# Audio continuity (crackling) qualification — 2026-09-26

Scope: the audible crackling the user reported on native routes (NFR-02,
GRAPH-05/14/15, PLUG-03, UI-04). Measured on the development machine
(Windows 11 Home 10.0.26200) with VB-Audio virtual cables only; nothing was
routed to speakers and no private audio was recorded.

## Method

`crates/transport/tests/live_audio_continuity.rs`, test
`live_backend_service_keeps_a_routed_tone_continuous` (ignored; also needs
`AUDIOROUTER_LIVE_CONTINUITY=1`).

```
harness render  -> CABLE Input          (VB-Audio Virtual Cable)
AudioRouter     :  CABLE Output -> [processors / plugins] -> CABLE-B Input
harness capture <- CABLE-B Output        (result)
harness capture <- CABLE Output          (reference)
```

- A -26 dBFS sine is recorded again after the route. A sampled sine obeys
  `x[n] = 2cos(w)x[n-1] - x[n-2]`, so any residual above 5 % of the amplitude
  is a dropped, repeated or zeroed block. Runs of exact zeros count as
  underruns. The reference recording separates harness faults from
  AudioRouter faults; the test refuses to pass if the reference glitches.
- The backend runs through the production loop
  (`serve_control_connections_forever_with_grant`) on a private pipe. The
  client sends no audio-rate pumps, only UI-like load: 20 Hz
  `system.diagnostics` and a 10 Hz counter pump.
- Only the steady window from 1 s after Start to Stop is analysed.
- `find_glitches` has its own unit test (clean tone → 0; dropped and zeroed
  blocks found at exact positions).

Useful variables: `AUDIOROUTER_CONTINUITY_SECONDS`,
`AUDIOROUTER_CONTINUITY_CHAIN` (built-in kinds, e.g.
`gain,parametricEq,compressor,gate,limiter`), `AUDIOROUTER_CONTINUITY_MODE=endpoint`
(single-endpoint worker instead of the multi-input worker),
`AUDIOROUTER_CONTINUITY_PLUGIN_DB` (a COPY of a user database; its
`patrick-main-session` plugin nodes are inserted with saved state) with
`AUDIOROUTER_PLUGIN_WORKER_PATH`, `AUDIOROUTER_CONTINUITY_TONE_HZ` (47 Hz makes
block-sized jumps unambiguous), `AUDIOROUTER_CONTINUITY_DUMP_DIR` (raw f32 dumps).

## Findings and fixes

1. **Every processed quantum was written twice into each output ring.**
   `WasapiOutputFanout::new` put an `AudioBlockRingTap` on each output's own
   ring into that branch's tap set. `process_to_rings_with_tap_sets` writes
   the ring directly *and* notifies the tap set. So each quantum was
   enqueued twice, the 4-slot ring overflowed, and the output was a stream
   of repeated and skipped 128-frame blocks.
   - Before: no processors, 7.4 s → 2,046 discontinuities (~275/s), all on
     128-frame boundaries. A 47 Hz tone showed the jumps are −128 (repeat)
     and +256/+384 (skip). The reference was clean.
   - Fix (`crates/windows-audio`): branch tap sets hold only observers. The
     combined `taps` keep the ring tap for the endpoint-scheduler path,
     where taps are the only feeder. Tap-only branches (virtual sinks,
     recorders on multi-input routes) had the same double-enqueue. Their
     rings are now recycled after each quantum. Before, they filled after
     one quantum and then silently skipped their observers.
   - After: no processors, 30 s → 0 glitches. Output queue fell from 24 ms
     to 13 ms.
2. **Audio was only pumped when the UI sent RPCs.** The WebView timer
   (5 ms target, throttled when minimized or occluded) drove every native
   worker over the named pipe, serialized with all other requests.
   - Fix (`crates/transport`, `crates/control`): the production backend
     loop keeps the (thread-affine) control plane on its thread. A separate
     I/O thread accepts pipe clients. Between requests, the plane thread
     calls `ControlPlane::service_running_native_audio` every 1 ms, under
     MMCSS "Pro Audio" with 1 ms timer resolution. UI pumps are now
     optional counter reads (100 ms when `audioService.active`).
     `audioService` reports passes, late gaps (>8 ms) and max gap on pump
     results.
   - Measured: max service gap 2.6–7.7 ms, zero late gaps, while under
     diagnostics load.
3. **No jitter margin at the render device.** With the minimal 22 ms
   (1,056-frame) device buffer, a late capture packet emptied the device.
   - Fix: multi-input outputs open with 50 ms headroom
     (`SharedRender::open_with_headroom`) and an 8-quantum ring. When a
     block finds the device buffer empty (startup or underrun),
     `RingOutputPump` queues a 10 ms silence cushion ahead of it. Shared-mode
     capacity is not latency. Underruns are counted (`outputUnderruns` on
     `nativeMultiInputs.pump`, and shown in the status line).
   - After: 5-processor chain, 30 s → 0 glitches. Output queue 17–27 ms.
4. **A bypassed plugin made Play fail.** The user's saved session had ReaEQ
   bypassed; `nativePaths.prepare` returned "a path needs one source or one
   Mixer, then a single chain". The session-graph validation listed
   bypassable kinds without `Plugin`, and a disabled or bypassed *first*
   processor was rejected outright. Both conflict with GRAPH-05 (dry
   bypass).
   - Fix (`crates/engine`) plus regression
     `native_paths_pass_a_bypassed_plugin_dry_anywhere_in_the_chain`
     (first/second/third position; the bypassed stage passes audio unchanged).
5. **Plugin threads ran at normal priority.** The host bridge thread and
   the worker process loop now join MMCSS "Pro Audio" (`ProAudioThread`).
   The bridge's first misses are its intended startup fill.

6. **Generated sources ran ahead of the output device.** Test Signal and
   Audio File are paced by a `SilentCapture`, which offered a packet on
   every call. Each 1 ms service pass generated up to 64 quanta, and the
   full output ring discarded almost all of them. The generator's timeline
   advanced anyway.
   - Before (`AUDIOROUTER_CONTINUITY_SOURCE=testSignal`): 20 s → 1,965
     discontinuities. Output queue pinned at 67 ms, and 1,784 of 1,785
     service passes were late (max 20 ms). That would also starve every
     other path in the same session.
   - A trial fix (engine backpressure: skip a path whose destination is
     full) cured the generator, but it produced a ~10 ms gap in 4 of 5
     live-capture runs, against 0 of 6 with drop-on-full. It was reverted.
   - Fix: `SilentCapture` yields packets at the 48 kHz wall-clock rate, one
     packet primed, catch-up bounded to 100 ms after a stall.
   - After: Test Signal, 30 s → 0 glitches, 11 ms output queue, 0 late
     passes.

7. **Recording on multi-path sessions was broken.** Nothing drained a
   Recorder branch's queue on the multi-input worker, so the UI's 8-chunk
   queue (~21 ms) filled and the tap dropped the rest of the take. Also,
   `recorders.list` never reported the live frame, and `start`/`resume`
   demanded that the next chunk begin at exactly the client's frame.
   Stopping therefore failed with `FrameWentBackwards` or `NotRecording`.
   - Fix: the multi-input pump drains recorder workers. `recorders.list`
     reports the end of the newest committed audio. Start/resume skip audio
     queued before the requested frame, then adopt the stream's timeline;
     contiguity is still enforced afterwards (regression tests in
     `audiorouter-recording`). The UI also sends routes with a connected
     Recorder to the multi-path worker, because the single-endpoint worker
     rejected that fan-out with `UnsupportedTopology`.
   - After (`AUDIOROUTER_CONTINUITY_RECORD=1`): a 15 s float32 WAV on the
     multi-path worker with 0 glitches.
8. **Render cushion made adaptive.** The 10 ms default grows 5 ms per real
   underrun, up to 40 ms, and resets on Stop (unit-tested).

## Environment observation (23:00 onwards)

From about 23:00 the machine produced occasional ~10 ms output gaps on the
virtual-cable harness: typically one per 10–20 s run, clustered 2–4 s after
the streams open. They also appeared, at the same rate, with a build of the
earlier commit `47410f12` that had measured clean at 22:40. The reference
recording sometimes gapped too. The backend service never ran late
(max gap under 8 ms). The conclusion is system or virtual-cable scheduling
noise, not a code regression; the adaptive cushion is the mitigation. The
single-processor sweep below ran under these conditions, so a one-gap
result for a processor is not attributed to that processor. Processors
with a clean 10 s run: Bass & Treble, Dehum, Declick, Denoise, FIR Filter,
Meter, Advanced EQ, Delay. One environment-pattern gap: Volume (the
reference gapped too), Speech Denoise (the reference gapped too), Time
Shift, Graphic EQ, Gate. Two to six gaps: Compressor, Limiter, Pitch
(13 glitches in its first 10 s run). A Pitch recheck of two 20 s runs
found 1 gap and then 0, with the plain direct route gapping in between,
so Pitch is not systematically discontinuous.
None showed the systematic per-block pattern of finding 1.

## Clock-drift survey of the user's saved session

`live_saved_session_output_queue_drift`, 180 s, run on a copy of the
user's database with privacy mute latched (every output rendered silence;
nothing recorded). The session's two paths: PD200X mic → ReaPlugs →
CABLE-A Input and Scarlett, plus CABLE-B Output → EQ → Scarlett. Each
output's queue depth was sampled every 2 s:

| Output | Queue (ms) | Slope |
| --- | --- | --- |
| voice-to-cable-a (CABLE-A Input) | 17.4–17.5 | −0.006 ms/min (≈ −0.1 ppm) |
| voice-monitor (Scarlett) | 16.7–16.8 | +0.024 ms/min (≈ +0.4 ppm) |
| siege-out (Scarlett) | 16.7–16.8 | +0.053 ms/min (≈ +0.9 ppm) |

On this machine, drift is far too small to matter within a session: the
10 ms cushion would take hours to drain. Other hardware can drift more,
so drift correction stays a low-priority gap rather than a closed one.

## Results after all fixes (commands run from the repository root)

| Route (CABLE Output → … → CABLE-B Input) | Worker | Duration | Result glitches | Reference |
| --- | --- | --- | --- | --- |
| direct | multi-input | 30 s | 0 | 0 |
| Gain, PEQ, Compressor, Gate, Limiter | multi-input | 30 s | 0 | 0 |
| Gain, PEQ, Compressor | single endpoint | 30 s | 0 | 0 |
| ReaFIR → ReaEQ (bypassed) → ReaComp → ReaGate (user's saved nodes and state) | multi-input + plugin workers | 60 s | 0 | 0 |
| Test Signal (997 Hz, paced) | multi-input | 30 s | 0 | n/a |

Example (plugins): `AUDIOROUTER_CONTINUITY_PLUGIN_DB=<copy.sqlite>
AUDIOROUTER_PLUGIN_WORKER_PATH=C:\code\audiorouter\target\release\audiorouter-plugin-worker.exe
AUDIOROUTER_LIVE_CONTINUITY=1 AUDIOROUTER_CONTINUITY_SECONDS=60
AUDIOROUTER_CONTINUITY_CHAIN= cargo test -p audiorouter-transport --test
live_audio_continuity -- --ignored --nocapture`.

One 60 s plugin run also contained a system-wide stall at ~10 s. It
silenced the harness's own MMCSS generator (a 50 ms buffer) as well, and
the test correctly reported that run as inconclusive. Two isolated
AudioRouter-only events appeared in that run and none in the next 60 s run.

## Limits (not claimed)

- The glitch harness uses virtual cables only. Physical devices were covered
  only by the drift survey (queue depth, not waveform). Clock drift is
  still uncorrected in code (`DriftController` is unused); it measured
  under 1 ppm here but can be larger on other hardware.
- Latency does not shrink by itself after a stall. The queue can stay up
  to about 70 ms (50 ms device headroom plus 21 ms ring) until Stop/Play.
- The user's by-ear confirmation on the real microphone → Scarlett /
  CABLE-A route is still pending.
