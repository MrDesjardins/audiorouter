# Reading the audio health log

While a route plays, the backend writes one `"event": "audioHealth"` line a
minute to `%LOCALAPPDATA%\AudioRouter\logs\backend.jsonl`. It holds counts
and times only, never audio. This page says what each field means and what a
healthy route looks like. Requirement IDs: ENG-03 (observability), ARCH-08
(clock domains), QUAL-04 (timing and drift).

Counters are totals since Play. To see what happened in one minute, subtract
the previous record's value.

## How a route keeps time

Every audio device has its own clock. Two devices set to 48 kHz still differ
by tens to hundreds of parts per million, so a queue between them slowly
fills or drains. AudioRouter corrects this in two places:

- **Each input** is read at the pace of the slowest input in its path. An
  input whose device runs faster would fall behind, so one frame is removed
  from a packet now and then until it is current (`slippedFrames`).
- **Each physical output** keeps a small queue ahead of the device (its
  cushion). When the queue drifts above or below its target, single
  128-frame blocks are played one frame shorter or longer.

A slip is one block resampled by 0.78% with cubic interpolation. It is not a
gap, and blocks that are not slipped pass through unchanged.

## Fields

### `plane.inputs[]`

| Field | Meaning | Healthy |
| --- | --- | --- |
| `nodeId` | The input node. | |
| `waitMs` | Smoothed age of a packet when picked up. | About one packet (10–12 ms). |
| `discontinuities` | Packets Windows flagged as following lost capture data. The first packet of a stream usually has it. | Not rising. |
| `trimmedPackets` | Whole packets skipped after a stall: packets older than 40 ms, or packets of an input that was held back 30 ms or more while another input stalled. Each is a heard skip. | Not rising. |
| `slippedFrames` | Frames removed to follow a faster device clock. Inaudible. | Rises slowly on a faster device; 0 on the input that paces the path. |
| `maxPacketGapMs` | Longest time between two packets in this minute. | 10–20 ms. A value near or above the output queue names the input that stalled. |

### `plane.outputs[]`

| Field | Meaning | Healthy |
| --- | --- | --- |
| `nodeId` | The output node. | |
| `targetMs` | The cushion: the lowest queue depth the output aims for. 10 ms by default, 5 ms deeper after each underrun (to 40 ms), and one step back after five minutes that did not need it. | 10–20 ms. |
| `queueMs` | Smoothed audio queued ahead of the device. It sits about one device period (10 ms) above `targetMs`. | `targetMs` + 5 to 15 ms. |
| `underruns` | Times the device buffer ran empty. Each is a heard gap. | Not rising. |
| `fullDrops` | Blocks dropped because the output ring was full. Each is a heard skip. | 0. |
| `latencyTrims` | Blocks discarded because a backlog arrived behind an already deep queue (after a stall). Each is a heard skip, in place of lasting delay. | Rises only with a stall. |
| `slippedFrames` | Frames added or removed to hold the queue at its target against clock drift. Inaudible. | Rises steadily; 100 ppm of drift is about 290 frames a minute. |

### `plane.audioThread` and top level

| Field | Meaning | Healthy |
| --- | --- | --- |
| `intervalMaxGapMs` | Longest time between two audio thread passes this minute. | Under 8 ms. |
| `intervalMaxPassMs` | Longest single pass this minute. | Under 3 ms. |
| `intervalSkippedPasses` | Passes skipped because a request held the route. | A few hundred. |
| `mmcssProAudio` | The thread runs in the Windows "Pro Audio" class. | `true`. |
| `pumpErrors`, `lastPumpError` | Failed passes and the category of the last failure. | Not rising. |
| `slowestRequestMethod`, `slowestRequestMs`, `slowRequests` | The slowest control request this minute and how many took over 8 ms. | |

## Reading a problem

- **`underruns` rises and one input's `maxPacketGapMs` is large:** that input
  stopped delivering for that long, and every output of its path went silent
  with it. Look at that device or application.
- **`underruns` rises with `intervalMaxGapMs` above 10 ms:** the PC did not
  run the audio thread in time (heavy load, a driver holding the CPU).
- **`underruns` rises with neither:** the cushion was too small for this
  device's timing. `targetMs` grows by 5 ms each time and should settle.
- **`queueMs` far above `targetMs` for minutes:** not expected since the
  2026-11 repair; report it with the log.
- **`trimmedPackets` or `latencyTrims` rise together with an underrun:** the
  route recovered from a stall by discarding the backlog once.

## History

Before the 2026-11 repair an output's queue grew to the capacity of the
device buffer and ring (about 74 ms) and stayed there, an input could sit up
to about 100 ms behind unseen, and a stall was followed by about 16 dropped
blocks. See the [analysis](../plans/active/evidence/2026-11-10-audio-backend-performance.md)
and the [repair record](../plans/active/evidence/2026-11-11-audio-latency-repair.md).
