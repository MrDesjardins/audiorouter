# Audio backend performance analysis — 2026-11-10

Owner request. Environment: the owner's Windows 11 PC (24 logical CPUs),
Rust 1.96.0, `origin/main` at `feaf2108` for the benchmark. The app was
closed (`system.quit` at 22:09 local) while benchmarks ran.

## Sources

1. `%LOCALAPPDATA%\AudioRouter\logs\backend.jsonl` and
   `backend.previous.jsonl`: 6,476 `audioHealth` records (one per minute)
   from 2026-10-31 to 2026-11-10, versions 0.0.24 to 0.0.28, all with a
   running route. Statistics were computed with throwaway Node scripts; the
   logs stay on the owner's PC and are not in the repository.
2. `cargo bench -p audiorouter-engine --bench realtime`.
3. The saved route "Patrick Main Session (Siege Footstep EQ)" read from a
   copy of `state.sqlite` (with its `-wal` file): three capture devices and a
   Soundboard feed connected Mixers that play to a cable and to headphones.

## Results

### Processing cost (not a problem)

| Tool | µs per 128-frame quantum | Share of 2.67 ms |
| --- | --- | --- |
| Pitch | 54.4 | 2.04% |
| Denoise / Speech Denoise / Spectral Gate | 27–29 | 1.0–1.1% |
| Gate | 11.9 | 0.45% |
| Graphic EQ | 10.3 | 0.39% |
| every other tool | 2.2–5.4 | ≤0.2% |
| 32 tools in one route | 330 | 12.4% |

Every "silent tail" time matched its voice time, so no tool slows on
subnormal values.

### Build profile (not a problem)

No `[profile.release]` exists, so release builds use Cargo's defaults.
`CARGO_PROFILE_BENCH_LTO=fat CARGO_PROFILE_BENCH_CODEGEN_UNITS=1` and
`CARGO_PROFILE_BENCH_LTO=thin` each built a separate benchmark binary and
gave the same times within 1% (Pitch 54.3 and 54.1 µs, Denoise 28.0, 32-tool
route unchanged). LTO is not worth its build time here.

### Audio thread scheduling (not a problem)

| Measure | p50 | p90 | p99 | max |
| --- | --- | --- | --- | --- |
| Longest gap between pumps in a minute (ms) | 4.2 | 5.1 | 6.3 | 48.7 |
| Longest pump in a minute (ms) | 1.9 | 3.1 | 4.7 | 14.9 |
| Passes skipped for the control thread's lock, per minute | 313 | 706 | 940 | 1,313 |

MMCSS "Pro Audio" was held in every record. The gap exceeded 8 ms in 15
minutes of 6,476. The thread runs about 660 passes per second.

### Queues and gaps (the problem)

| Measure | Value |
| --- | --- |
| Output queue, first minute of a run (39 runs) | 37 ms median |
| Output queue, all minutes | 25 ms p10, 51 ms median, 62 ms p90 |
| Input pickup age | 11 ms microphone; 17–20 ms the other two captures |
| Underruns | 2,841 in 656 of 6,434 consecutive minutes |
| Blocks dropped at a full output ring | 45,089 |
| Underruns that came with drops in the same minute | 2,817 of 2,841 |
| Dropped blocks per underrun | 13.0 p10, 15.8 median, 17.5 p90 |
| Minutes with an event when an input's age was ≥19.5 ms | 450 of 2,044 (22.0%) |
| Minutes with an event otherwise | 194 of 4,390 (4.4%) |

During events the audio thread showed no long gap, no input reported a
trimmed packet or a discontinuity, and both outputs were hit together.

## Reading

- The output queue is meant to hold only a 10 ms cushion. It instead grows to
  the capacity of the device buffer plus the ring and stays there.
- The repeating "one underrun, about 16 dropped blocks" matches the output
  pump's own rule: after an underrun it queues up to 1,920 frames (15 blocks)
  of silence ahead of the next block, and the backlog behind it overflows
  the 8-block ring.
- An input whose pickup age is pinned near 20 ms has a full capture buffer.
  That is what a device clock faster than the path's pace produces, since
  nothing corrects clock differences. The link from a full capture buffer to
  the stall is an inference from the correlation above, not a measurement.
- One event in five happens without a saturated input; the log cannot say
  which input stalled. The repair adds a per-input longest-gap field for
  that.

## Limits

No audio was measured at the devices. Latency figures are the backend's own
queue estimates, not an end-to-end loopback measurement.
