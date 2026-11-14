# Issue #136 - VST2 chain matrix with Blue Cat PatchWork

Status: test written, first run recorded below. Parent plan:
[issue #136 VST experience](issue-136-vst-experience.md). Branch
`feature/vst-plugin-experience`. Requirements: PLUG-01..07, GRAPH-14, UI-08;
milestones M06/M07.

## Objective

Answer, with evidence, the reporter's three questions for the plugin that
reportedly fails: can AudioRouter **open** it, **run** it, and **configure**
it, alone and in a chain of one to three VST2 effects, at mono and stereo
widths, in different orders?

The installed Blue Cat plugins are `BC PatchWork VST.dll` (an effect that
hosts other plugins) and `BC PatchWork Synth VST.dll` (an instrument, which
AudioRouter refuses by design: no MIDI input). Both are the free demo build
installed under `C:\Program Files\VSTPlugins`. Companion effects with known
behavior: Pitchproof x64 (VST2, the plugin that crashed on its editor before),
ReaComp and ReaEQ (ReaPlugs, VST2).

## What the test does

[vst2_chain_matrix.rs](../../../crates/plugin-host/tests/vst2_chain_matrix.rs)
is opt-in (`#[ignore]`, Windows only). For each combination and each width
(mono, stereo) it uses the real worker process and the real runtime bridge,
the path a route takes at Play, without an audio device:

| Step | What is checked |
| --- | --- |
| start | The chain launches (one plugin: a single worker; two or three: one shared chain worker). |
| audio | 400 blocks of a 440 Hz tone pass through every member in order: output finite, not failed; records peak, silence, and whether the tone differs from dry. |
| configure | Each member's first parameter is moved away from its default and accepted; audio keeps flowing. |
| saveState | Each member saves its state; records size and whether the parameter change reached it. |
| editors | Each member's native editor opens in a message-pumping parent window while audio runs; the editor child window exists and has a real size; it closes; audio still runs, with no worker failure. Host and editor client sizes are recorded. |
| restore | The saved states restore into a fresh chain, which runs again and saves again. |

Combinations (13 orders, plus one with the same plugin twice):
`BC`, `PP`, `RC`, `BC>RC`, `RC>BC`, `BC>PP`, `PP>BC`, `BC>EQ`,
`BC>RC>EQ`, `RC>BC>EQ`, `RC>EQ>BC`, `BC>PP>RC`, `PP>RC>BC`, `BC>RC>BC`
(BC = Blue Cat PatchWork, PP = Pitchproof, RC = ReaComp, EQ = ReaEQ). The
instrument is checked separately and must be reported as unsupported.

The test writes `vst2-chain-matrix.json` and `.md` beside the test binary
(or to `AUDIOROUTER_CHAIN_REPORT`) and fails when a step that should work did
not. A plugin with no editor, or one that passes audio through unchanged, is
recorded, not failed.

## How to run

```
cargo test -p audiorouter-plugin-host --all-features --test vst2_chain_matrix -- --ignored --nocapture
```

- `AUDIOROUTER_CHAIN_FILTER=BC>RC` runs only matching combinations.
- `AUDIOROUTER_CHAIN_SHOW_EDITORS=1` shows the editor host window and keeps each editor open for `AUDIOROUTER_CHAIN_HOLD_MS` (default 1500), for a look by eye.
- `AUDIOROUTER_CHAIN_PLUGINS="BC=path;PP=path"` replaces default plugin paths.

No audio device, Windows default or database is touched. Close other
AudioRouter shells is not needed; the test starts only its own workers.

## Limits

- It proves worker, chain, state and editor behavior with generated audio. It
  does not prove the desktop shell's window (that needs the shell run by eye
  with the same plugins), a live route, or hardware latency.
- The demo build can add periodic silence or noise by design; record it, do not
  treat it as an AudioRouter defect.
- PatchWork with nothing loaded inside it passes audio through unchanged. In a
  real route fed by a microphone, AudioRouter silences that on purpose
  (unchanged-audio guard). This is expected and is recorded as "does not
  differ from dry".

## Results (2026-11-13, Windows 11, debug workers, no audio device)

**Other plugins** (`AUDIOROUTER_CHAIN_SKIP=BC`; Pitchproof x64, ReaComp, ReaEQ,
ReaGate, ReaDelay, ReaXcomp, ReaFIR): 24 combinations of one to three plugins
(including the same plugin two and three times), each at mono and stereo, 48
runs. All started, processed a tone without a worker failure, accepted a
moved parameter, saved state, showed a new editor window for every member,
closed it with audio still running, left no window behind, and restored into a
fresh chain. Problems: none. 319 s.

**Blue Cat PatchWork** (demo, 218 parameters, 40 KB state): chains of PatchWork
with ReaComp, ReaEQ and Pitchproof in every tried order passed all steps, and
its editor opened and closed with audio running. First run of those chains
reported two problems: PatchWork **alone** showed no editor window (mono and
stereo). The same plugin's editor appeared when it sat first in a chain, so
this single-plugin case needs a look. The test check was then tightened to
count only a window created by that open (the first run counted a window left
behind by the previous member); the corrected BlueCat rerun was stopped
before it finished because the demo's "limiting demo" dialog pops up in its own
window, which interrupted the owner's game. BlueCat chains therefore need an
attended rerun.

Also found: `BC PatchWork Synth VST.dll`, an instrument, is classified
`SupportedVst2X64Gated` (a supported effect) instead of being refused as an
instrument. AudioRouter has no MIDI input, so it cannot play. The plugin
metadata check does not recognise a VST2 instrument by itself; to be fixed
with a regression (VST2 `effFlagsIsSynth`).

Not proven: the desktop shell's own window (resize, close order) with these
plugins, a live route, or hardware latency.

## Rollback

Delete the test file and this page; nothing else changed.

## Next

1. Run the full matrix; fix every problem it finds, each with a regression.
2. Run the shell build by eye with the same plugins and `AUDIOROUTER_CHAIN_SHOW_EDITORS=1` evidence.
3. Then the stopped-session editor and the sidechain design from the parent plan.
