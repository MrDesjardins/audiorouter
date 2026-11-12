# Audio latency and clock-drift repair — 2026-11-11

Plan: [audio latency](../audio-latency.md). Analysis that led to it:
[2026-11-10 performance analysis](2026-11-10-audio-backend-performance.md).
Branch `perf/audio-queue-latency`. Environment: the owner's Windows 11 PC,
Rust 1.96.0, debug test builds. Requirement IDs: ARCH-08, CAP-04, QUAL-04,
ENG-03, NFR-03.

## Method

`crates/windows-audio/src/drift_sim_tests.rs` runs the real
`WasapiMultiInputFanout`, `RealtimeMixerFanout` and `RingOutputPump` against
simulated capture and render devices on one virtual clock. Each device has
its own rate, delivers or takes 10 ms at a time, and can stall. Each input
carries a continuous 440 Hz tone, and each device records the largest step
between consecutive samples it receives, so a lost or repeated stretch is
detected. One audio thread pass runs every 1.5 ms, or irregularly in one
test. Minutes of drift take seconds to run and give exact counts.

The simulation was written first and run against the unchanged code.

## Before and after

| Scenario (simulated time) | Before | After |
| --- | --- | --- |
| Matched clocks, 60 s | pass | pass; no block altered once settled |
| Path 500 ppm faster than the outputs, 300 s | 37 blocks dropped per output; queue at the 74 ms ceiling | 0 dropped, 0 underruns; queue ≤ 28 ms; tone unbroken |
| Path 500 ppm slower than the outputs, 300 s | 6 underruns per output | 0 underruns, 0 starved frames; tone unbroken |
| One input stalls 80 ms, then delivers its backlog | queue stays above its settled depth; nothing removes the 69 ms the other input accumulated meanwhile (seen in a trace of an intermediate version, not measured on the old code) | 1 underrun (the stall itself), 0 ring drops, cushion not deepened, queue settled, the other input's backlog gone |
| One input 500 ppm faster than the pacing input, 300 s | 4 capture packets lost after about 100 ms of internal backlog | 0 lost; never more than 30 ms behind; pacing input untouched; tone unbroken |

Scenarios added with the repair (all pass):

- four devices on four clocks (+80, −60, +120, −150 ppm) for 10 minutes: no
  underrun, drop, overflow or step in the tone;
- a 30 ms hiccup on both inputs deepens the cushion one step, and five quiet
  minutes give it back;
- an output that takes 20 ms at a time settles on a deeper target instead of
  underrunning repeatedly;
- an input that waits for a pacing input with 30 ms packets is not treated
  as backlogged;
- irregular passes (one in 50 is 6 ms late, one in 500 is 12 ms late) for 10
  minutes on four clocks: 2 and 1 underruns on the two outputs (minute 1 and
  minute 7), cushions settle at 20 ms and 15 ms, nothing discarded.

## Findings made while building it

1. **A depth read at one moment hides drift.** Devices move audio 10 ms at
   a time, so the queue depth at any sampling point is a staircase. In the
   first version the pump watched the lowest depth per window: it read 384
   frames for 16 seconds while the mean fell steadily, then the device ran
   empty. The controller now uses the time-weighted mean depth, which moves
   smoothly with drift (`drift::MeanDepth`).
2. **An input's accumulator was only emptied when a packet arrived.** A pass
   that read no packet, or skipped a stale one, left the path waiting for
   that input although up to 85 ms of its audio sat in the accumulator. The
   existing stale-packet trim had the same effect. The feeder now moves
   accumulated audio to the ring on every pass (`submit_accumulated`).
3. **An input could be far behind without the log showing it.** Each input
   has a 4,096-frame accumulator, a 4-block ring and one packet in hand,
   about 100 ms, before its capture buffer starts to fill and `waitMs`
   rises. The owner's log showed `waitMs` pinned near 20 ms on two inputs,
   which means those inputs were about 100 ms late at the time.
4. **A fixed 10 ms period is the wrong assumption for some devices.** The
   pump measures how far the queue dips below its mean and plans for that
   (`drift::QueueSwing`), and the trim depth scales with it so two packets
   in one late pass are never discarded.

## Slip quality

`drift::tests::a_slipped_block_follows_the_signal_closely`: a 128-frame block
resampled to 127 or 129 frames differs from the ideal signal by less than
0.001 of full scale at 1 kHz (−60 dB) and less than 0.02 at 5 kHz (−34 dB),
and only within the slipped block. At 100 ppm of drift one block in about 78
is slipped.

## Checks

| Check | Command | Result |
| --- | --- | --- |
| Drift unit and simulation tests | `cargo test -p audiorouter-windows-audio` | 161 passed, 4 ignored (device tests) |
| Workspace tests, with `source_combinations` (5) and `tool_combinations` (4) | `cargo test --workspace --no-fail-fast` | 1,343 passed, 0 failed, 32 ignored |
| Clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| Shell crate Clippy | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | clean |
| Format | `cargo fmt --all -- --check` and the same for `src-tauri` | clean |
| Documentation links | `tests/acceptance/docs.ps1` | passed (256 files, 1,460 links) |
| Engine benchmark | `cargo bench -p audiorouter-engine --bench realtime` | unchanged (Pitch 54.1 µs, Denoise 28.4 µs); the engine crate is not modified |

An earlier fail-fast run of the workspace stopped at one failure in
`audiorouter-driver-helper`, a crate this change does not touch. It passed
in the six reruns and in the full run above; the plan records it.

The pump now reads the device padding and the clock once per pass per
output even when no block moves, and the feeder keeps a few counters per
input. That cost was not measured on real devices.

## Not verified

- No real device was used. The continuity test
  `live_backend_service_keeps_a_routed_tone_continuous` needs the owner's
  cables and was not run.
- The owner has not listened to the change, and no health log from a real
  session with it exists yet.
- The simulated devices are a model: 10 ms steps, exact rates, no resampling
  inside Windows. Real WASAPI padding and packet timing differ in detail.
- The cause of stalls that happened without a saturated input (one in five
  events in the owner's log) is still unknown. `maxPacketGapMs` was added to
  find it.
