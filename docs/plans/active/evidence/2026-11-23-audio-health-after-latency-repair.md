# Audio health on the owner's route after the latency repair — 2026-11-23

Plan: [audio latency (archived)](../../archived/2026-11-11-audio-latency.md),
task 9, "a new health log". Repair record:
[2026-11-11 audio latency repair](2026-11-11-audio-latency-repair.md).
Requirement IDs: ARCH-08, CAP-04, NFR-03, ENG-03, QUAL-04.

## Method

Read-only analysis of the owner's own backend log, the files
`%LOCALAPPDATA%\AudioRouter\logs\backend.previous.jsonl` and `backend.jsonl`
on the owner's PC, read on 2026-11-23 while the route was playing. No audio,
device or setting was touched. Command:

```text
node tools/quality/audio-health-summary.mjs backend.previous.jsonl backend.jsonl
```

The script uses only the one-minute `audioHealth` records, which hold counts
and times, never audio. It splits the log into runs (new process, counters
going back, or a gap of more than five minutes), discounts the underrun
every stream reports in its first minute, and calls a minute a stall when an
input's longest packet gap exceeds 30 ms or the audio thread's exceeds 10 ms.
Field meanings: [reading the audio health log](../../../operations/audio-health-log.md).

The log covers 2026-11-07 00:44 to 2026-11-23 07:23 UTC: 4,994 records from
versions 0.0.26 to 0.0.31. Records without `targetMs` were written by builds
before the repair. The route is the owner's usual one: mic, Siege and a
physical input plus a Soundboard into the `voice-to-cable-a` (CABLE-A) and
`voice-monitor` outputs.

## Before and after, same route

| Output | Before the repair (27.2 h) | After the repair (55.8 h) |
| --- | --- | --- |
| `voice-to-cable-a` queue, median / p90 | 46.9 / 59.6 ms | 20.0 / 26.2 ms |
| `voice-monitor` queue, median / p90 | 51.7 / 61.1 ms | 20.1 / 26.0 ms |
| `voice-to-cable-a` underruns outside stalls | 490 (18.0 an hour) | 21 (0.38 an hour) |
| `voice-monitor` underruns outside stalls | 386 (14.2 an hour) | 6 (0.11 an hour) |
| Blocks dropped from a full ring (`fullDrops`) | 7,899 and 6,216 | 0 and 0 |
| Underruns during stalls | 5 and 3 | 11 and 15 |
| `latencyTrims` | (field did not exist) | 0 and 4 |

The earlier 107-hour analysis (2026-11-10) found the same pattern: a 51 ms
median queue and 26 underruns an hour. After the repair the queue sits where
the design puts it (cushion plus one device period), no block has been
dropped for a full ring, and gaps outside stalls are about 40 to 130 times
rarer.

Clock drift is now corrected by slips instead of drops. `voice-to-cable-a`
slipped 949,755 frames in 55.8 h, about 99 ppm against the path's clock;
`voice-monitor` slipped 79,509, about 8 ppm. Inputs slipped a few thousand
frames each, so the pacing input and the others agree closely.

## The remaining gaps

1. **Stream start.** 33 underruns in the first minute of a run, one per output
   per Play. Expected and not counted above.
2. **Stalls (26 underruns in 11 minutes).** Three kinds:
   - The microphone alone stopped delivering for 35–42 ms: 10 underruns in
     5 minutes (2026-11-15 02:37, 11-16 02:39, 11-20 00:37, 11-22 00:41 and
     11-23 00:42 UTC). The other inputs and the audio thread were on time,
     so this is the mic device or its driver. Nothing in the queue can cover
     a 40 ms hole without adding that much latency all the time.
   - The audio thread alone ran late by 11–21 ms: 5 underruns in 3 minutes
     (11-13 03:47, 11-14 22:10, 11-22 22:08 UTC). The PC was busy at that
     moment.
   - Every input and the audio thread stopped together for 56–124 ms: 11
     underruns in 3 minutes on 2026-11-22 between 00:45 and 01:09 UTC
     (2026-11-21 16:45–17:09 local). That is the whole PC not running the
     thread. No commit, checkout or build output in the main checkout falls
     in that window (reflog and `target/` timestamps); worktrees under
     `.worktrees/` were not checked, so the cause is not established.
3. **The 10 ms cushion on `voice-to-cable-a` (20 underruns).** Underruns
   outside stalls, by the cushion (`targetMs`) in force when they happened:

   | Output | 10 ms cushion | 15 ms cushion |
   | --- | --- | --- |
   | `voice-to-cable-a` | 20 in 41.3 h (0.48 an hour) | 1 in 12.8 h (0.08 an hour) |
   | `voice-monitor` | 5 in 52.1 h (0.10 an hour) | 1 in 3.0 h |

   After each underrun the cushion grows to 15 ms and, after five quiet
   minutes, shrinks back to 10 ms (decision D4). On CABLE-A that 10 ms is
   slightly too thin, so the output saw a gap about every two hours. A
   15 ms floor for that output would make them about six times rarer at the
   cost of 5 ms more delay on that path. **Decision (owner, 2026-11-24): keep
   the 10 ms floor** for `voice-to-cable-a`; 5 ms more delay is not worth the
   rarer gaps. No pump constant changed, so no drift-simulation or live
   continuity rerun is required by this decision; the measured ~0.48 gaps/hour
   on CABLE-A at 10 ms is the expected behavior.

## What this does not show

- It does not replace the owner's listening test: a slip is designed to be
  inaudible and the counters cannot prove it.
- It does not replace the live cable continuity test
  (`live_backend_service_keeps_a_routed_tone_continuous`), which checks the
  waveform rather than counters.
- The 2026-11-22 system stall's cause is unknown.

## Outcome

Task 9's health-log half is answered: the repair works on the owner's real
route over 55.8 hours. The CABLE-A cushion floor is decided (keep 10 ms,
2026-11-24). The owner reports the listening test was done across recent
evening play sessions and chose to keep 10 ms; the automated multi-input
continuity runs passed on CABLE/CABLE-A. A long live session through the
owner's own running app is not separately claimed.
