# Lasting static and stall backlog — investigation and fix (2026-10-19)

Owner report (0.0.19 known issue): after the PC was heavily loaded for a
while, incoming Discord voices had lasting "scratchy" static that Stop/Play
did not seem to clear. Owner route `patrick-main-siege-footstep`: Discord
(process loopback `application-capture-1`), the mic chain (`mic` → … →
`voice-to-cable-a`, CABLE-A) and Siege (`siege-in`, CABLE-B capture,
headphones spatial) feed `mixer-2` → `voice-monitor` (Focusrite USB Audio,
also the Windows default output) and `physicalOutput-1` (CABLE Input).
Affected versions: 0.0.19 and earlier multi-input routes (the feeder rule
below is older than 0.0.19).

Outcome in one line: the static the owner heard on 2026-10-19 was in the
Focusrite's own Windows stream, not in AudioRouter; separately, a real
AudioRouter defect (inputs keep the backlog a stall leaves) was reproduced
and fixed for inputs.

## 1. Code and log analysis (no audio run)

- Stop/Play rebuilds every piece of audio state (graph, DSP, rings, WASAPI
  clients, capture and render workers); only the control thread, control
  plane and UI outlive it.
- The multi-input feeder is lockstep: `stage_path_inputs`
  (`crates/engine/src/lib.rs`) mixes a path only when every input ring has a
  block, and the 4-block input rings are filled by
  `WasapiMultiInputFanout::pump_inputs`
  (`crates/windows-audio/src/lib.rs`), which stops reading a capture while
  its ring is full. Every input is therefore consumed at the output clock
  rate. After a stall, unread packets stay in the input's WASAPI capture
  buffer (100 ms for polling captures) and never drain: lasting delay until
  Stop/Play. `DriftController` exists but is unused.
- `RingOutputPump` queues a 10 ms silence cushion whenever the device buffer
  is found empty, deepens it 5 ms per underrun up to 40 ms, and never
  shrinks it.
- The 2026-10-18 incident log showed exactly these plateaus after each
  service gap: Discord pickup 10.7 → 22.4 ms, `siege-in` pinned near 100 ms,
  output queues 26–50 ms.
- Discord's process-loopback stream never set `DATA_DISCONTINUITY`, even
  across 170 ms stalls, so losses there would be invisible to the counters.

## 2. Stall injection harness

`live_saved_session_tone_survives_starvation`
(`crates/transport/tests/live_audio_continuity.rs`) loads a copy of the
owner's database (`AUDIOROUTER_STATIC_DATABASE`, taken with its `-wal`),
plays a tone helper captured by process loopback in place of Discord, and
analyses the routed result sample by sample. New switches:

| Variable | Effect |
|---|---|
| `AUDIOROUTER_STATIC_STALLS=<at s>:<ms>,...` | suspends the audio service thread (`SuspendThread`) for `<ms>` at `<at s>` |
| `AUDIOROUTER_STATIC_SPARE_RENDER_ID` | moves every other physical output to a spare endpoint |
| `AUDIOROUTER_STATIC_SET=<node>/<parameter>=<n>;...` | sets numeric parameters before planning |

Existing switches used: `AUDIOROUTER_CONTINUITY_TONE_RENDER_ID`,
`AUDIOROUTER_STATIC_RESULT_ID`, `AUDIOROUTER_CONTINUITY_RESTART_AT`
(Stop/Play), `AUDIOROUTER_CONTINUITY_SECONDS`. Every run below used the
release test build beside the owner's live app, the tone on CABLE Input
(Discord's output), other outputs on the unused Realtek digital output, the
result from the PD200X loopback, `mixer-2/inputVolume:mixer-1=0` (only the
Discord-like path measured), stalls of 60, 90, 170, 30, 120 and 200 ms at
20–120 s, Stop/Play at 145 s, 175 s total. Logs: `stall-run*.log` in the
session scratchpad (not kept in the repository).

Runs 1–2, before any fix: the stalls reproduce the incident's lasting
plateaus (`siege-in` 11.6 → 17 → 79 → 89 ms, output queues 15 → 40–48 ms,
both held until Stop/Play), but the tone stayed clean between stalls
(2–3 glitches per stall, nothing lasting; one burst of tiny 0.06× residuals
at 79 s in run 2). The lasting static itself did not reproduce.

## 3. Recurrence: the static was the Focusrite

At about 19:03 the owner heard the crackle again (app PID 25792). YouTube,
which plays straight to the Focusrite and not through AudioRouter, crackled
too, and kept crackling after Stop in AudioRouter. At 19:08: audiodg 0.6 %
of a core, AudioRouter 7 %, total CPU 22 %, DPC 0.9 %, interrupt 0.4 %; no
System-log USB, audio, PnP or WHEA events in 90 minutes; all Focusrite
devices OK; AudioRouter's counters flat since 18:52. Changing the format in
Sound settings → Focusrite USB Audio → Properties → Advanced (and back)
cleared it at once; the endpoint stored 48 kHz 24-bit stereo afterwards. A
format change restarts the endpoint's audio engine and device stream, so
the fault lived there (driver or device clock state). The earlier "clean
`mixer-3` recording" could never have shown it: `mixer-3` does not pass
through the Focusrite. The cause of the Focusrite state is unknown; next
time, before resetting, note Focusrite Control's sample rate and buffer
size and run LatencyMon for a minute.

The same log showed the AudioRouter defect live: `siege-in` at 97–105 ms
since a series of 80–85 ms service gaps at 15:42–15:52 (it was 19 ms at
start). With Siege closed the cable carries silence, so the delay was not
audible, but game sound would have lagged about 80 ms.

## 4. Fix and rejected alternatives

| Run | Change | `siege-in` after stalls | Glitches |
|---|---|---|---|
| 6 | none (A/B, same session) | 20 → 32 → 85 → 95 → 105 ms | 23 |
| 3 | input trim + output trim + timed cushion decay | 20 ms | 43 |
| 4 | input trim + output trim | 20 ms | 47 (bursts at 47, 87, 112 s) |
| 5 | input trim only | 20 ms | 133 (83 after Stop/Play, no stall) |
| 7 | input trim, not for app loopback | 20 ms | 23; app pickup 11 → 18 → 28 ms, lasting |
| 8 | input trim incl. app loopback (shipped) | 20 ms | 32, at or beside stalls |

Shipped (run 8): `AudioCaptureSource::has_queued_packet` (physical,
binaural and process-loopback captures, via `GetNextPacketSize`). In
`pump_inputs`, a packet older than 40 ms at pickup
(`INPUT_BACKLOG_TRIM_AGE_100NS`) while a newer one is already queued is
skipped, and skipping continues until no newer packet waits; the newest is
always kept. Each skip is counted per input as `trimmedPackets` in the
once-a-minute `audioHealth` log record. Healthy inputs are picked up at
11–20 ms, so the rule fires only after a stall. Regression:
`multi_input_feeder_submits_coherent_sources_to_both_outputs` (two stale
packets skipped and the newest kept; a lone old packet kept).

Rejected:

- Output trimming (drop a block when more than cushion + 20 ms is queued):
  CABLE-A's ordinary queue (about 32 ms) already exceeds it, so healthy
  audio was cut. Each device buffers differently; a fixed limit is wrong.
- Timed cushion decay (one 5 ms step per clean minute): the dropped audio
  caused a fresh underrun about 60 s after the last one, then the cushion
  grew back.
- Excluding application loopback from trimming: the backlog moved onto the
  app input (run 7). With the 40 ms rule, the app input was never trimmed
  in run 8 (it peaked at 27 ms and settled back to 11 ms).

Run 5's burst did not recur in run 8 with the same input rule; it is
attributed to load from the owner's session (Firefox, Discord), not proven.

## 5. Checks

`cargo fmt --all -- --check` and the shell's `--check`: clean. Strict
all-target, all-feature Clippy (workspace): clean. Full workspace tests:
1,138 passed, 0 failed, 29 ignored (before the final narrowing edits);
afterwards windows-audio + control: 395 passed, 0 failed. Release shell
built with `--features custom-protocol` (embedded bundle
`index-2RY2YMXQ.js`, unchanged UI) for the owner to run.

## 6. Remaining

- Output queues keep their deepened cushion (up to 40 ms after repeated
  underruns) until Stop/Play. Removing it without a glitch needs inaudible
  catch-up (playing very slightly fast for a moment), not block drops.
- Owner listening test of the new build over a long session, and an
  `audioHealth` check that `trimmedPackets` rises only after real stalls.
- If the Focusrite crackle recurs: the checks in section 3.
