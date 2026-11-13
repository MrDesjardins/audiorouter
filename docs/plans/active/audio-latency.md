# Audio latency and clock-drift repair

Started 2026-11-10, owner request ("do a performance analysis of the audio
backend", then "create a plan and execute it. I want you to optimize the
audio. Make sure to document"). Branch `perf/audio-queue-latency`, worktree
`.worktrees/audio-latency`, based on `origin/main` at `4078a9ad`.

## Objective

Remove the extra output latency and the recurring audible gap that the
owner's own health log shows on a multi-device route, without changing how a
healthy single-clock route sounds.

Requirement IDs: ARCH-08 (clock domains: correct asynchronous sources and
sinks, bound FIFO occupancy, report drift corrections), CAP-04 (output clock
correction), QUAL-04 (queues bounded under simulated clock mismatch, no
monotonic latency growth), ENG-03 (drift and xrun counters), NFR-03 (added
buffering delay). No API method, node kind or setting changes, so the
REST/MCP parity rule (AUTO-18) needs no new method; the new counters are
backend-log fields only.

## Evidence

- Before: [2026-11-10 audio backend performance analysis](evidence/2026-11-10-audio-backend-performance.md).
  Over 107 hours of the owner's `audioHealth` records the output queue sat at
  51 ms median (62 ms p90), with 2,841 underruns and 45,089 dropped blocks,
  99% of the underruns arriving with 13–17 dropped blocks. DSP cost, build
  profile and thread scheduling were not the cause.
- After: [2026-11-11 audio latency repair](evidence/2026-11-11-audio-latency-repair.md).

## Cause (from the code)

1. A path runs at the pace of its slowest input and nothing corrected the
   clock difference between devices. A faster input backed up (about 100 ms
   inside the feeder, then in its capture buffer); a slower output filled the
   queue to its 74 ms ceiling and dropped blocks; a faster output drained it
   and underran.
2. After an underrun the output pump queued up to 40 ms of silence ahead of
   the next block. A backlog arriving behind it overflowed the 8-block ring,
   so about 16 blocks were dropped, and the queue then stayed at its ceiling.
3. The cushion only ever grew (10 ms to 40 ms) until Stop/Play.
4. An input's accumulator was moved to its ring only when a packet arrived,
   so a pass without one could starve the path although the audio was there.

## Decisions

- **D1. Each physical output holds a target queue depth.** Its low points
  sit at the cushion; the pump measures the time-weighted mean depth in
  250 ms windows and corrects toward the cushion plus the queue's swing (one
  10 ms device period, or the measured dip for a device with larger steps).
- **D2. Small corrections are slips, not dropped blocks.** One block is
  resampled to one frame fewer or more (4-point cubic interpolation, aligned
  to the block, so untouched blocks stay bit-exact). A correction starts
  when the mean is 5 ms off target, stops within 1 ms, has a two-second time
  constant and never exceeds one frame per 128-frame block (0.78%).
- **D3. A backlog far above the ordinary peak is discarded at once** (20 ms
  above the target mean plus the swing), counted per output as
  `latencyTrims`. A stall was already heard; keeping its backlog as lasting
  delay is worse.
- **D4. The cushion shrinks again** by one 5 ms step after five minutes in
  which the queue stayed at least a step and 5 ms above empty. It still
  grows 5 ms on an underrun, except after 50 ms or more without a block: that
  is a stalled source, which no cushion covers.
- **D5. A faster input slips too.** When its mean backlog in the feeder
  stays above one period plus 5 ms and it never emptied during the window,
  one frame per packet is removed until it is within 1 ms of one period.
  Inputs are never slowed down, so the slowest input still paces the path
  and the input and output controllers cannot fight each other.
- **D6. An input held back 30 ms or more for a whole window loses whole
  packets** until it is below that, counted in `trimmedPackets`. This is the
  backlog a healthy input collects while another input of its path stalls.
- **D7. Accumulated audio moves to the ring on every pass.**
- **D8. Not done here:** substituting silence for a late input (a path
  would follow its fastest input and chop the slowest); event-driven waits
  in the audio thread (saves CPU, does not fix gaps); LTO (measured: no
  gain).

Constants are in `crates/windows-audio/src/lib.rs` beside
`RENDER_JITTER_CUSHION_FRAMES` and in `crates/windows-audio/src/drift.rs`.

## Tasks

1. [x] Worktree, branch, this plan, analysis evidence.
2. [x] Simulation harness (`drift_sim_tests.rs`) on a virtual clock with the
   real feeder, mixer and output pump; four failures reproduced on the
   unchanged code.
3. [x] `drift` module: `MeanDepth`, `QueueSwing`, `SlipController`,
   `slip_frames`.
4. [x] Output pump: D1–D4.
5. [x] Input feeder: D5–D7 and per-input longest packet gap.
6. [x] Health log fields: inputs `slippedFrames`, `maxPacketGapMs`; outputs
   `targetMs`, `slippedFrames`, `latencyTrims`.
7. [x] Documentation: [audio health log guide](../../operations/audio-health-log.md),
   release notes, privacy guide, documentation map, a validated lesson in
   `AGENTS.md`.
8. [x] Local checks (see the repair record).
9. [ ] Owner: listen on the real route; send the next `backend.jsonl`.
10. [ ] `live_backend_service_keeps_a_routed_tone_continuous` on the owner's
    cables, for the multi-input worker.

## Validation matrix

| Check | Environment | Result |
| --- | --- | --- |
| Simulation tests fail before, pass after | Windows host, unit tests | done; see the repair record |
| Existing `windows-audio`, `engine`, `control` tests | Windows host | see the repair record |
| `source_combinations`, `tool_combinations` | Windows host | see the repair record |
| fmt and Clippy (workspace and `src-tauri`) | Windows host | see the repair record |
| Live continuity test | owner's devices | not run |
| Listening test and a new health log | owner | not run |

## Risks and rollback

- A slip changes audio content: a 2.7 ms block played 0.78% fast or slow,
  within −60 dB of ideal at 1 kHz and −34 dB at 5 kHz. It replaces dropped
  blocks and underruns, which are far more audible, but no listening test
  has been done yet.
- The target depth (about 20 ms) is lower than what the queue drifted to
  before (50–60 ms). A system whose timing needs more will underrun, deepen
  the cushion 5 ms at a time and settle, as before.
- The simulation is a model of WASAPI, not WASAPI. Real padding and packet
  timing may expose a case it does not cover.
- Rollback: revert the branch. No stored data, schema or API changes.

## Observed, not part of this change

- `audiorouter-driver-helper`
  `install_rolls_back_the_package_when_the_device_cannot_be_created` failed
  once in seven runs of its suite on 2026-11-11 (assertion at
  `crates/driver-helper/src/tests.rs:734`) and passed alone. The crate is
  not touched here. It needs its own look; a shared state file between
  parallel tests is the first thing to check.

## Later

- Event-driven audio thread (wait on the WASAPI capture events): about 660
  wake-ups a second today.
- A deadline for a stalled input, once `maxPacketGapMs` names which input
  stalls on the owner's route.
- Measured mic-to-cable latency on the owner's devices (NFR-02).

## Next action

Task 9: the owner runs the build on the usual route for an evening and sends
`backend.jsonl`. Compare `queueMs`, `underruns`, `fullDrops` and
`maxPacketGapMs` with the 2026-11-10 analysis.
