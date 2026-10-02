# Visual tool inspectors — 2026-09-30

Scope: user request to tune Compressor and Gate by talking, slow the Meter's
peak line, then review every tool's Properties UI. Requirements UI-04, DSP-01,
GRAPH-15. Windows 11 Home 10.0.26200, development machine.

## Changes

- **DSP/engine telemetry.** Compressor, Gate and Limiter keep a per-channel
  `LevelFollower` for input and output (instant rise to each block's sample
  peak, 40 dB/s fall). It is updated once per processed block inside the
  existing loops, with no allocation, lock, logging or reset-on-read. These
  levels are exposed as optional `inputLevelDb`/`outputLevelDb` in
  `nodeTelemetry[].processor` (contract, schema and API reference updated).
- **Defect fixed (pre-existing).** `RuntimeGraph::processor_telemetry_for_node`
  used the first stage tagged with the node, which on any routed chain is the
  incoming edge's `ChannelMatrix`. It therefore returned `None`, so gain
  reduction and gate state were never reported on native routes. It now
  searches every stage with that identity, as the other per-node lookups do.
  Regression: `native_route_reports_dynamics_telemetry_past_the_incoming_edge_matrix`
  fails with the old lookup and passes with the fix.
- **Meter.** The recent-peak line holds 1.2 s, then falls 20 dB/s. The RMS bar
  falls at 30 dB/s. Canvas mini meters use the same hold. Readouts stay exact.
- **Compressor/Gate/Limiter live editor** (`ui/src/DynamicsEditor.tsx`). It
  shows a transfer curve drawn from the DSP formulas, with keyboard-operable
  drag handles (threshold, ratio, hysteresis, ceiling) and a live operating dot
  plus trail. It also has In/GR/Out meters, an 8 s history with a draggable
  threshold line, a gain-reduction or gate open/closed strip, and room-noise/
  voice estimates with a one-click threshold suggestion. A response sketch
  reruns the detector equations on a synthetic word.
- **Tool editors** (`ui/src/ToolVisuals.tsx`):
  - Graphic EQ: ten faders under the backend's exact response, with an exact
    field for each band and presets.
  - Bass & Treble: two draggable shelves on the exact response.
  - Dehum: the notch comb computed from the DSP's harmonic coefficients.
  - Pitch: a keyboard with semitone choices.
  - Delay: a timeline with samples and video-frame conversions.
  - Volume/Gain: % ⇄ dB with a before/after level bar and clipping warning.
  - Input Switch: source buttons named after their upstream nodes.
  - Declick and Speech Denoise: named strengths.
- **Generic settings.** The caption shows a readable value with its unit. The
  slider sits beside the exact field, and double-clicking it restores the
  default. Enabled and Bypass share a row. Empty "no readings" notes were
  removed.

## Verification

| Check | Result |
| --- | --- |
| `cargo test -p audiorouter-dsp -p audiorouter-engine -p audiorouter-control --lib` | 54 / 148 / 203 passed (live tests ignored) |
| `node tools/contracts/check-drift.mjs` | passed (103 methods, 33 node kinds, 17 processors) |
| `npx vitest run src` (ui) | 415 passed, 39 files |
| `playwright.production.config.ts` (canvas, inspector, meter, dynamics editor) | 24 passed, then dynamics 6/6 rerun |
| `npm run e2e` (dev + real e2e backend) | 8 failures also fail on the unchanged UI baseline (Bypass/Undo locator ambiguity, live-controls message, EQ-points undo). `session-file` is intermittent (2/3 alone; `route.fulfill` already handled). The tests changed here (tool-workflows, themes-and-bounds, advanced-eq-filters) pass. |
| Live continuity, VB-Cable, 47 Hz, 20 s, chain `compressor,gate,limiter` + branch Meter + Recorder | reference, routed result and recorder: 0 glitches, 0 silent runs; 1 late service gap (21 ms) while the user's shell was also running |

The continuity run printed `processor=null`. That led to the telemetry lookup
fix above, which was verified by the portable regression only. The live
processor levels on hardware have not been observed yet: the user's release
shell was running, so no further hardware tests were started.

Screenshots reviewed (dark, light, high contrast):
[compressor dark](2026-09-30-compressor-live-dark.png),
[compressor light](2026-09-30-compressor-live-light.png),
[compressor high contrast](2026-09-30-compressor-live-high-contrast.png),
[gate dark](2026-09-30-gate-live-dark.png),
[gate light](2026-09-30-gate-live-light.png),
[gate high contrast](2026-09-30-gate-live-high-contrast.png),
[word sketch](2026-09-30-gate-word-sketch-dark.png),
[Graphic EQ light](2026-09-30-tool-graphicEq-light.png),
[Pitch high contrast](2026-09-30-tool-pitch-high-contrast.png),
[Input Switch light](2026-09-30-tool-inputSwitch-light.png),
[Bass & Treble high contrast](2026-09-30-tool-bassTreble-high-contrast.png).
The other `2026-09-30-tool-*` files cover the remaining tools in each theme.

## Artifact

`src-tauri/target/visual-tools/release/audiorouter-shell.exe`, 2026-09-30
21:34, SHA-256 `c1e2ad7435f89a374afcc9978845475b423362ae9b3e1b7ff233360264439c2b`.
Built with `npm.cmd run build` (ui) then `cargo build --manifest-path
src-tauri/Cargo.toml --release --features custom-protocol --target-dir
src-tauri/target/visual-tools`, in a separate directory because the user's
release shell was running from `src-tauri/target/release`. Its `ui/dist`
(21:32) is newer than every UI source edit and contains the new editors.
This is an unsigned manual build, not M08 qualification.

## Remaining

- Attended: talk through a Gate and a Compressor on the PD200X route and
  confirm the live levels, dot and suggestion.
- Rollback: the previous release exe in `src-tauri/target/release`. The
  telemetry fields are optional, so older UIs ignore them.

## Follow-up: no live response in the attended shell (2026-09-30)

User report: Compressor and Gate showed "Not playing" and still meters while
playing, and the Mixer showed no reading. Mute mic could not be undone while
playing.

- **Two shells were running.** PID 4280 (`target/release`, 20:51) and PID 71612
  (`target/visual-tools`, 21:37). This is the known pipe-collision case
  (AGENTS.md lesson): the second window's UI uses the first shell's backend,
  which predates the level telemetry and the telemetry lookup fix.
- **The Mixer had no meter.** A probe that compiled the exported
  `patrick-main-native` session and processed 40 blocks reported a level for
  every node except `mixer-1`. `CompiledMixerFanoutGraph` now observes the
  mixed block once per quantum (lock-free `BlockMeter`) and reports it for the
  Mixer node. Regression: assertion added to
  `pre_mixer_branch_survives_a_disabled_input_and_effect_bypass`.
- **The Level tile did not move.** It showed `peakDb`, the stage meter's
  cumulative hold. It now shows `currentPeakDb`, and Live readings has a
  moving level bar (1.2 s peak hold, falling RMS) for every playing node.
- **The dynamics pill said "Not playing" whenever readings were missing.** It
  now names the cause: Off, Bypassed, No readings (save the route; run one
  window) or Old backend.
- **Mute mic while playing.** The 50 ms diagnostics refresh replaced the
  snapshot object, and an effect keyed on that object reset the button from
  the stale full-snapshot `status`. Muting worked, the button reverted, and the
  next click muted again. The UI now follows the live
  `diagnostics.privacyMute` value and reacts only when it changes. Regression
  test: `toggles privacy mute on and off while playing despite a stale
  full-snapshot status`, which fails with the old effect.

Checks: `cargo test -p audiorouter-engine -p audiorouter-control -p
audiorouter-windows-audio --lib` 148/203/100 passed; UI unit tests 416 passed;
production canvas suite 24 passed. The live level bar was reviewed in dark,
light and high contrast. No hardware test was run, because both of the
user's shells were running.

Artifact: `src-tauri/target/live-readings/release/audiorouter-shell.exe`,
21:50, SHA-256 `62e1e35b6bd56ce2255af334de5886b3d65b49777234295a6c630335c845af4e`,
with UI bundle 21:48. Next: close every AudioRouter window, start only this
exe and confirm that the Compressor, Gate, Mixer and other tools move.

## Follow-up: group Lock and audio connection redesign (2026-09-30)

User request: lock a canvas group against accidental drag/resize; make
playing connections a refined, animated picture of audio and its intensity
in all three themes (first iteration).

- **Group Lock** (`CanvasGroups.tsx`). New optional `locked` presentation
  flag, validated as a boolean. A locked group is not draggable, has no
  resize handles, cannot be deleted from the canvas, and shows a dashed
  border and a lock icon. Properties has a "Lock position and size" switch.
  Tests: `CanvasGroups.test.tsx` (persist/validate/inspector) and
  `e2e/canvas-group-lock.pw.ts` (drag, Delete and resize have no effect;
  unlock restores handles). The existing group e2e still passes.
- **Connections** (`flowLine.tsx`). Every connection has a soft cable track
  and a small fixed-size notched arrowhead at its input; markers no longer
  scale with stroke width. When playing, the core uses a source→target
  gradient whose colour follows level from the theme palette (`--flow-cold`
  teal, `--flow-warm` gold, `--flow-hot` orange, `--flow-clip` red). A
  blurred glow grows with level (no blur in high contrast). One to six light
  "comets" travel at a constant 150 px/s (SMIL `animateMotion` on the edge
  path), more and larger as the level rises. Reduced motion keeps colour,
  glow and arrow but no comets. Edge action bars float above the line and
  stay dim until hover, focus or selection. The legend matches.
  `signalStrokeWidth` was removed; `flowLine.test.ts` covers the level
  mapping, palette and path length. The `audio-tools` e2e now proves a comet
  moves along the playing line.

Checks: UI unit tests 420 passed (40 files); production canvas suite 24
passed; `audio-tools` + group e2e 26 passed, with 1 failure that is the
known baseline Bypass-locator ambiguity. Screenshots:
[playing dark](2026-09-30-flow-playing-dark.png),
[playing light](2026-09-30-flow-playing-light.png),
[playing high contrast](2026-09-30-flow-playing-high-contrast.png),
[stopped dark](2026-09-30-flow-stopped-dark.png).

Artifact: `src-tauri/target/release/audiorouter-shell.exe`, 22:04, SHA-256
`1de45c22d8b83d4e37e04a4eafaa77e2689a8f6110e76c2f5aa4d27e09870535`, with UI
bundle 22:03. The user's running `live-readings` build was not touched.
