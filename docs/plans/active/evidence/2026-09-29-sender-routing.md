# Siege EQ, Scarlett and Discord Mixer routing — 2026-09-29

Requirements: UI-03/08/12, GRAPH-01/02/03, ARCH-04/09. Affected build:
`banner-timing-20260929`. User confirms they start EQ → Mixer at the EQ's blue
dot and want Siege → EQ → Scarlett plus EQ → Mixer ← Discord → another output.
The intended graph is valid. No user session/database edits were made.

## Findings and decisions

The receiver-first idle affordance treated the EQ's blue input as a request
to change what feeds the EQ. User-confirmed sender-first intent supersedes
that gesture convention: all real output ports now offer blue **Send (start)**
handles; real input ports reveal orange **Receive (end)** handles during drag.
Tools, Mixers and input devices follow one rule. Backend port roles and saved
edges do not change. Keyboard connection form remains available.

Native playback had a second defect: a weakly connected component containing
one Mixer could compile its input chains and mixed outputs, but rejected a
direct destination of the final pre-Mixer tool as a nonparticipating node.
Compiled input branches now reuse the already-processed quantum before Mixer
input gain, other sources or shared downstream processing. Scratch storage is
allocated during preparation, bounded by the existing eight-output limit.
Realtime access is nonblocking; busy/unavailable scratch is silent for the
current quantum rather than replaying a prior one. Privacy mute applies before
every delivery. Stateful tools run once; branches retain their channel matrices,
meters, bounded ring and tap delivery. No new permissions or schema required.

Supported extension: direct sinks attached to the final source/tool feeding a
Mixer or Input Switch. This does not add arbitrary internal DAG branching,
nested Mixers, or independent processor chains after each branch.

## Verification

Windows MSVC, Edge, React/React Flow route harness and virtual WASAPI cables.

- `npm.cmd test`: 379 passed in 31 files.
- Production UI build/typecheck passed, isolated UI assets
  `index-BfyAmG7V.js` and `index-C83LaAIt.css`.
- Direct Playwright against separately started Vite: all 31 tests in
  `audio-tools.pw.ts`, `banner-layout.pw.ts`, `playing-canvas-lines.pw.ts`
  passed (1.2 min). Temporary manual config removed; owned Vite stopped.
  Exact Siege fixture checks EQ → Mixer → output from sender dots, retained
  Scarlett and Discord edges, and both empty/previously direct-fed outputs
  without a second Mixer. Generic Gain fan-out, a complex processing chain,
  connector visibility, banner controls, Timing and playing lines also pass.
- Initial broad run found stale Undo selector/text and an off-canvas test node
  after hover scrolled it into view. Scoped Undo to topbar, updated expected
  wording and positioned the fixture within the canvas. Final suite passes.
- `cargo test -p audiorouter-engine -p audiorouter-control -p audiorouter-transport -p audiorouter-windows-audio --lib --locked --target-dir target/sender-routing-20260929 --quiet`:
  engine 144 passed; control 198 passed/6 opt-in tests ignored; transport 22
  passed; Windows adapter 99 passed. Existing unrelated unused-mut warning in
  Windows adapter test remains.
- New engine regressions verify direct-game isolation from Discord and Mixer
  gain, mono/stereo mapping, privacy mute, exactly one block per output,
  stateful plugin single execution, busy-buffer silence, and actual +6 dB EQ
  response at 997 Hz in both direct and mixed outputs across 200 quanta.
  Fixture compile initially caught missing Debug and invalid `bands` parameter;
  corrected to the existing `band0Enabled` contract.

Native continuity command (environment variables set only in test process):

```text
AUDIOROUTER_LIVE_CONTINUITY=1
AUDIOROUTER_CONTINUITY_PRE_MIXER=1
AUDIOROUTER_CONTINUITY_CHAIN=parametricEq
AUDIOROUTER_CONTINUITY_TONE_HZ=47
AUDIOROUTER_CONTINUITY_SECONDS=15
cargo test -p audiorouter-transport --test live_audio_continuity --locked --target-dir target/sender-routing-20260929 live_backend_service_keeps_a_routed_tone_continuous -- --ignored --nocapture
```

Passed: one native multi-input path, processed cable input and stopped generated
second input → Mixer → CABLE-B; the same processed input → Recorder directly.
Reference: 15.0 s, 0 glitches/silent runs/discontinuity flags. Mixed output:
15.0 s, 0 glitches/silent runs/discontinuity flags. Direct recording: 15.1 s,
0 glitches/silent runs/discontinuity flags. Peak 0.0500 on all three; lateGaps
0, max service gap 7147 µs, output underrun counter 2 (no discontinuity in the
analysed steady signal). No microphone or Scarlett opened. No shell was running
when the harness started; private pipe and disposable test session used.

Inspected screenshots:
[dark](2026-09-29-siege-mixer-false-dark.png),
[light](2026-09-29-siege-mixer-false-light.png),
[high contrast](2026-09-29-siege-mixer-false-high-contrast.png).

## Handoff and rollback

New complete pair lives in `target/sender-routing-20260929/release`.
Frontend path selected temporarily for embedding, restored afterwards.
Shell and worker release builds passed with `--locked`; shell used
`--features custom-protocol`. Final JS/CSS asset names were verified inside
the shell. UI index: 2026-09-30 02:13:35 UTC; shell: 02:31:37 UTC;
worker: 02:32:16 UTC. SHA256:

- Shell: `7C903D588BF8BBB9963C23D1EF8E28A583F732E173F2B3E5E8FAE45FC17B2CC6`
- Worker: `12712258AA265BD31BE3CD63E4F3D43DC451068D8ABB96270DDE04A0C6E61FC3`

Documentation acceptance passed: 71 Markdown files, 384 local links.
`git diff --check` passed.

Next: attended Scarlett/Discord manual check on the user's real endpoints and
saved route; virtual-cable evidence does not establish that exact setup.
Close the previous app before launching the new shell; keep the worker beside
it. No graph migration is needed. Revert this slice/use the previous pair to
roll back; the old native topology limitation then returns.
