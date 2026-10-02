# Siege crackling: connected-application liveness

Windows 11 x64, 2026-10-02. Requirements CAP-06/11, GRAPH-14, QUAL-05.
Affected review: token-visibility-20261002, version 0.0.3.

## Reproduction and diagnosis

User hears continuous crackling with Siege output CABLE-B Input and microphone
CABLE-A Output. Direct speaker output is clean. Bypassing Siege EQ and minimizing
AudioRouter do not resolve it. Graph routes CABLE-B Output → EQ → Siege + Discord
Mixer → Scarlett and Outplayed; independent microphone path remains present.
Read-only node telemetry shows zero clipped samples, historical game/EQ peaks
approximately -11.75/-5.37 dBFS. Integration reports action/100% applied, Discord
100%; no continuous volume-update loop was observed.

Three bounded one-packet native pump observations on the already-running route:

| Sample | Output underruns | Service late gaps | Worst service gap |
| --- | --- | --- | --- |
| 1 | 5659 | 1317 | 132966 µs |
| 2, about one second later | 5664 | 1318 | 132966 µs |
| 3, about one second later | 5668 | 1319 | 132966 µs |

Counters confirm ongoing delivery gaps. They aggregate four outputs and do not
establish that every underrun was on the game output. Runtime generation 20 was
read from SQLite in read-only mode; prepared worker generation is 1. No saved
graph, level, device setting or process was changed by the diagnostic probes.

The owning backend thread called full Windows process inventory once per second
to check Discord liveness. A native isolated measurement took **78.8636 ms**
for one inventory, versus **358.7 µs for ten direct identity probes**. This is a
concrete stall source, not proof that all system scheduling problems are resolved.

## Repair

The second defect is a capture-rate mismatch: process loopback requested
44,100 PCM16 frames/s, while the multi-input feeder consumed those frames
directly at the graph's 48,000 frames/s. Expanding PCM16 to float32 did not
resample. This underfed a Mixer by 8.125%, including when the application was
silent. The capture now requests internal graph-rate PCM16 stereo, deriving
its byte rate consistently; Windows shared-mode conversion remains enabled.
No endpoint default or global device format is changed.

Current connected-source checks query only the explicit bound PID and validate
creation time, full path and exit time. Query-only handle is closed on all paths;
unsafe ownership/buffer invariants are documented. PID reuse, stale creation
time, wrong path and exited source cannot match. Both single-input and multi-input
application liveness use this fast path. Failed identity/recovery still uses
existing full inventory and unique verified restart matching; occasional recovery
stalls remain a risk. No VST, global format, user database or stored graph change.

## Checks and experiments

- Direct identity regression passes, with measured timings above; own process,
  wrong path/time, exited bounded helper and restart identity checks included.
- Windows adapter suite: parallel run 104 passed/3 socket-bind failures; serial
  rerun succeeded. Control suite: parallel 219 passed/1 network-tone timing
  failure/8 ignored; serial 220 passed/8 ignored. Network flakes are recorded,
  not repaired or represented as green parallel runs.
- Shell suite: 44 passed, 1 attended HTTP fixture ignored.
- Custom-protocol shell release build passed.
- Initial sandboxed live capture denied 0x80070005. Approved native run opened
  only exact virtual cables and analysed a generated 47 Hz tone; no microphone
  or user-application audio was recorded.
- Detected stale continuity-test binary: initial plain-cable runs reused an
  older binary (one had 0 glitches, another had 2 with a clean reference).
  These do not qualify this repair. Cleaned only transport dev build artifacts,
  rebuilt the fixture and verified the embedded `liveness-mix` string.
- New opt-in `AUDIOROUTER_CONTINUITY_APPLICATION_LIVENESS=1` fixture mixes
  physical tone capture with capture of its own silent, hidden, bounded helper
  at zero gain. The helper is reaped on scope exit; no user process is touched.
  This specifically exercises the connected-application liveness maintenance.
- That new fixture initially failed with 364 glitches/30 seconds and a clean
  reference, but normal control/audio library artifacts were also stale.
  Cleaned those two packages' dev artifacts and rebuilt: 406 routed glitches,
  61 silent runs/118333 samples, clean reference; service late gaps fell from
  30 to 1. Liveness alone did not fix the sustained mixer starvation.
- First graph-rate capture run: 7 routed glitches but 2 reference glitches;
  inconclusive. Prepared sources now also use exact identity verification on
  their first running check rather than an initial full scan. Recovery and
  forced inventory semantics remain unchanged.
- Final mixed fixture, Windows 11, 47 Hz, 30 seconds, Gain + Parametric EQ:
  reference **0 glitches/0 silent runs**, routed **0 glitches/0 silent runs**,
  service **0 late gaps**, max gap 4792 microseconds, 15911 passes. One aggregate
  underrun since Start remains (startup precedes the analysis window).
  Command: `AUDIOROUTER_LIVE_CONTINUITY=1`,
  `AUDIOROUTER_CONTINUITY_SECONDS=30`, `AUDIOROUTER_CONTINUITY_TONE_HZ=47`,
  `AUDIOROUTER_CONTINUITY_CHAIN=gain,parametricEq`,
  `AUDIOROUTER_CONTINUITY_APPLICATION_LIVENESS=1`, then
  `cargo test -p audiorouter-transport --test live_audio_continuity live_backend_service_keeps_a_routed_tone_continuous -- --ignored --nocapture`.

## Rollback and remaining gates

Final serial checks: `cargo test -p audiorouter-windows-audio --lib --
--test-threads=1`: **108 passed**; control equivalent: **220 passed/8 ignored**.
`cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1`:
**44 passed/1 ignored** (the earlier `--lib` attempt was invalid because the
shell has only a binary target). Logs: `target/siege-windows-tests.log`,
`target/siege-control-tests.log`, `target/siege-shell-tests.log` (ignored).
Documentation acceptance: **93 files/486 local links passed**; `git diff
--check` clean. Build: `cargo build --manifest-path src-tauri/Cargo.toml
--release --features custom-protocol --locked` passed in 1m15s, compiling
current Windows audio/control/transport crates. Current ID/token strings
verified in `ui/dist/assets/index-0L4hC8A_.js`; UI index 13:58:37 precedes
new executable 15:00:45.

Unpublished review: `target/reviews/siege-audio-20261002/audiorouter-shell.exe`,
SHA-256 `ED5BB2054839DD43D748D340512A73F9272B0B0146269FEE8C57A95EE6C71943`.
Unchanged v0.0.3 plugin worker accompanies it; no CLI or installer is included.
Previous review retained. No user app was launched or stopped by the agent.
Next attended check: launch this executable, Play the saved session, enable
the HTTP API, run `npm.cmd start` in the Stats.cc example, and compare Siege
on the original Cable-B output. Credential/session configuration is retained.

Repeat mixed fixture: 30 seconds, clean reference and routed result, both
0 glitches/0 silent runs, service 0 late gaps, max 3725 microseconds, 15239
passes, one startup underrun. Plain-route first follow-up was inconclusive:
reference and result each had 2 glitches and the same 492-sample silence.
Repeat plain route passes: both 0 glitches/0 silent runs, 0 output underruns,
0 service late gaps, max 3652 microseconds, 16423 passes. Same command with
`AUDIOROUTER_CONTINUITY_APPLICATION_LIVENESS` unset.

Revert the direct process probe and two maintenance fast paths; old source
identity/restart semantics and graph formats are unchanged. Retain prior review
executables. User closed integration, AudioRouter and Siege normally for the
native cable qualification. Attended gameplay after the repair remains required.
No installer, release publication or M08 completion is claimed.
Also revert `process_loopback_format` to roll back the rate correction, which
restores the known starvation defect. Session schemas and saved bindings do not
change. The format regression locks capture to the internal Mixer sample rate.
