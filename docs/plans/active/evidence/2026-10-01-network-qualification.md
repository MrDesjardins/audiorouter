# Network Send / Network Receive qualification (2026-10-01)

Requirement: GRAPH-16 and SEC-13. Requested by the user before release 0.0.1
("ensure the feature works"). Environment: Windows 11, this workstation,
VB-Cable pair; the user's AudioRouter window was closed for the live runs.

## Defects found and fixed

1. **A session whose only outputs are Network Send, Recorder or virtual
   sinks played nothing.** `pump_native_multi_input_worker_once` processed
   outputs only when `output_count() > 0`, which counts *physical* outputs.
   So "microphone → Network Send", a sending PC without local monitoring,
   captured audio and discarded it. The fix gates on `has_output_owner()`.
   Found by the new device-free backend test (0 packets sent before the fix).
2. **Live network edits reported "applied" but kept the old socket.** A
   saved address/port/buffer change while playing recompiled the graph, but
   the UDP sender and receiver live outside it.
   `reconfigure_running_network_nodes` now:
   - retargets the sender (`NetworkSender::retarget`, which reconnects or
     opens a new socket for another IP family);
   - reconfigures the receiver's sender address and buffer in place;
   - opens a new receiver only for a port change.

   A failure reports `restartRequired`, never "applied". The test fails with
   the change disabled.
3. **Recorder Stop raced the audio** (`FrameWentBackwards`, found in the
   first live run). A stop frame read a moment earlier is already behind the
   audio written since. Worker stops now use `request_stop_at_or_after`,
   which stops at the end of the written audio (the counterpart of start
   alignment).

## Improvement for two computers

The receiver remembers the last address that sent valid AudioRouter audio but
is not the configured sender (`rejectedFrom`). The receive node says
"audio from X was ignored" and offers **Use X**. The network fields are
editable while playing, because edits now apply live. Arbitrary traffic
never becomes the hint. Screenshots:
`2026-10-01-network-receive-hint-{dark,light,high-contrast}.png`.

## Automated evidence (all passing)

- `audiorouter-windows-audio` network_audio (9, real UDP sockets; stable in
  3 consecutive runs):
  - packet validation;
  - loopback tone continuity;
  - wrong-address rejection and its hint;
  - gap concealment and late drop;
  - stereo separation and mono to both channels;
  - **sender clock ±0.3 %** over 6 s: 0 steps, 0 loss, no skip, delay held
    between 20 and 60 ms around the 40 ms target;
  - restarted sender followed immediately;
  - burst after a stall trimmed back to the target;
  - **this PC's LAN address (10.0.0.73) and IPv6 ::1**.
- `audiorouter-control`:
  - `network_send_and_receive_carry_a_continuous_tone_through_the_backend`.
    Test Signal → Network Send → UDP → Network Receive → Recorder, run
    through `nativePaths.prepare`, `session.start`, the backend service loop
    and telemetry, in one session and as **two separate backends**. Both
    layouts: about 1,316 packets, 0 lost, 0 rejected, 40 ms buffered,
    −12 dBFS preserved, 0 steps in the recorded WAV.
  - `network_settings_corrected_while_playing_take_effect_without_restart`:
    - a wrong sender port, then a wrong receiver address, are fixed live
      via `nodes.set`;
    - `rejectedFrom` names 127.0.0.1;
    - a receiver port move is followed live.
- `audiorouter-recording`: `a_stop_frame_already_behind_written_audio_stops_at_the_end_of_it`.
- UI:
  - `NetworkNodeEditor.test.tsx` (7);
  - e2e `network-receive-hint.pw.ts` (3 themes).

## Live evidence (VB-Cable, production backend loop on a private pipe)

`AUDIOROUTER_LIVE_CONTINUITY=1 AUDIOROUTER_CONTINUITY_CHAIN=gain,compressor`,
plus `AUDIOROUTER_CONTINUITY_NETWORK=47911` and/or
`AUDIOROUTER_CONTINUITY_RECORD=1`:
`cargo test -p audiorouter-transport --test live_audio_continuity live_backend_service_keeps_a_routed_tone_continuous -- --ignored --nocapture`.

| Run | Packets sent/received | Lost/late/rejected | Receiver underruns | AudioRouter-only output dropouts |
| --- | --- | --- | --- | --- |
| Network + recorder, 60 s (reference glitched: inconclusive) | 22,884 / 22,884 | 0 / 0 / 0 | 1 | 1 |
| Network + recorder, 60 s (reference glitched) | 22,860 / 22,860 | 0 / 0 / 0 | 1 | 2 |
| Network + recorder, 60 s (clean reference) | 22,855 / 22,855 | 0 / 0 / 0 | 1 | 1 (at 1.9 s) |
| Network + recorder, 20 s (clean reference) | 7,899 / 7,899 | 0 / 0 / 0 | 0 | 1 at the output; the Recorder on the receive path: **0 glitches over 20 s** |
| Network only, 30 s (clean reference) | 11,605 / 11,605 | 0 / 0 / 0 | 2 (before measuring) | **0 (test passed)** |
| Network only, 30 s (reference glitched) | 11,597 / 11,597 | 0 / 0 / 0 | 1 | 1 |

No packet was lost in any run. The remaining ~10–25 ms output dropouts are
**not network-related**:
- plain routes without network or recorder showed them too (two 30 s runs:
  1 each);
- the last commit before this work (`1d4a2335`, built in a separate
  worktree) showed the same or more (two 30 s runs: 1–2 each, 2–4 output
  underruns);
- the recorder on the receive path was glitch-free in the same run;
- instrumentation found no service pass or recorder drain over 5 ms.

This matches the 2026-09-26 environment observation (virtual-cable render
scheduling). It stays an open, pre-existing output-render issue for release
notes, not a network defect. The receiver also counts one underrun at startup
before playout settles.

## Not covered

- Two physical computers, Wi-Fi jitter, and real Windows Firewall prompts.
  The first install on the user's second computer is the first such test.
  The LAN-address test only proves delivery through this PC's own network
  interface.
- Clock drift between two real devices was tested synthetically (±0.3 %),
  not between two PCs.
