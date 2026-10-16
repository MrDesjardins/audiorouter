# Tool teaching and sidebar spacing — 2026-10-15

## Scope and requirements

Owner requested another review of every tool's teaching quality and audible
relevance, compact question-mark guide targets inside tool cards, and enough
space between wrapped workbench tabs and the Selected node inspector.

Requirements: UI-11/12/17/20; DSP-01–08/10–19 and recording/routing contracts
inform the documentation. Read M05, interface and relevant tool contracts,
guide metadata/source references, actual editor controls and prior guide
decisions. Existing unrelated working-tree changes were preserved.

## Changes and decisions

- Added `site/guide-content/tutorials.json`: all 35 kinds have their own route,
  setup steps, expected result, first fault check and explanation of what the
  example teaches. Fixed copied Gate/Duck parameter descriptions, capture
  endpoint advice, shelf-frequency examples and the Test Signal level key.
- Added deterministic PCM input fixtures, relevant processing models and actual
  sample waveform/level views. One transport drives audio, seek and animation.
  Reference listening chooses output or an individual input without another
  Play button. Browser monitoring gain is deliberately lower than sample levels.
- Mixer/Input Switch use distinct drums/bass and bell tracks. Duck uses a
  separate voice-like control key without mixing it into the target output.
  Delay shifts the whole signal without a dry echo. Pitch changes the musical
  interval without changing phrase timing. EQ uses browser biquads and FIR a
  short built-in, normalized-energy convolution response with aligned dry/wet.
- Cleanup models target known synthetic hum, impulses or background layers.
  Each guide explicitly describes its limits: no claim that oracle-separated
  noise layers reproduce the engine's detectors or spectral algorithms.
- Meter reads actual example PCM for RMS/current peak/held peak and clipped
  sample count, with the app's stereo scale and markers. Recorder stores only
  the stopped synthetic interval, retimes file playback and keeps its cursor
  inside the appropriate Recorder/Audio File stage.
- A card's compact question mark is a separate sibling link visually inside
  the add button, including application capture, Group and installed plugins.
  It has a tool-specific accessible label and keyboard focus. Native opening
  retains the known-kind allowlist. Clicking help does not add a node.
- Properties measures wrapped tab height with ResizeObserver and reserves it
  plus 12 px above the inspector. No telemetry enters App state. Mobile retains
  the existing stacked panel layout. Help failure notices float within the
  card rather than moving the tool list.

## Per-tool review

Every kind was checked against the field/source references in processing.json
and routing.json; the generated reference still covers all 101 properties.

| Tool | Teaching and example outcome |
| --- | --- |
| Gain | Headroom and matched-level comparison; sample amplitude changes without timing change. |
| Volume | Branch balancing, 100% unity and 50% ≈ −6 dB; scalar percentage processing. |
| Bass & Treble | Shelf transition versus amount, focused bass/air examples; broadband fixture and two shelves. |
| Graphic EQ | Purpose, listening use and tradeoff for each of ten bands; bass, rumble, overtones and noise support the spectrum. |
| Advanced EQ | Band selection, shape/frequency/Q/gain/enable workflow, combined response, all eight shapes and All pass phase. |
| Compressor | Quiet/loud level sections, threshold/ratio, attack/release and fair makeup comparison. |
| Gate / Expander | Noise versus quiet speech, opening/closing, hysteresis/hold/range and word tails; pause-noise attenuation. |
| Limiter | Final-stage sample ceiling, headroom and constant limiting diagnosis; oversized third section. |
| Sync (delay) | Earlier-path alignment and positive delay; exact shift with initial silence, no dry mix. |
| Pitch | Semitones/cents and artifacts; shifted bell melody with unchanged note timing. |
| Dehum | Match 50/60 Hz before adding harmonics; actual notches against known 60 Hz harmonic hum. |
| Declick | Threshold versus wanted transients; varied known impulses and bounded repair illustration. |
| Denoise | Learn noise-only, keep profile, reduction/floor and relearning; prepared known hiss layer. |
| Speech Denoise | No manual profile, quiet syllables and strength tradeoff; adaptive illustrative layer attenuation. |
| Spectral Gate | Learned region-relative threshold versus whole-signal Gate; prepared-noise model with explicit FFT limitation. |
| FIR Filter | File approval, preparation, partitions/latency, wet/dry and gain; real browser convolution, not generic EQ. |
| Mute | Branch-specific silence and separate reference listening; no global-mic implication. |
| Input Switch | Alternatives versus sum, selected source and fade; genuinely distinct A/B tracks. |
| Duck | Target/key wiring, control-only trigger and recovery timing; separate audible trigger reference. |
| Time Shift | Accumulated history, pause/replay/live edge; documented fixed live/pause/behind/live story. |
| Mixer | Identify each connected input, independent levels and summed headroom; two distinct audible tracks and raw waveforms. |
| Meter | Stereo RMS/current/held peaks and clipping diagnosis; actual fixture-derived readings, unchanged audio. |
| Physical Input | Exact capture binding, channel mode, Windows access and missing-device recovery; visual-only. |
| Physical Output | Exact render endpoint, Windows volume and downstream diagnosis; visual-only. |
| Application Capture | Running process selection/refresh, activity and restart identity; visual-only. |
| Endpoint Loopback | Whole-output capture versus process capture, feedback prevention; visual-only. |
| Virtual Render Source | Sending app playback into the chosen cable and route; direction diagram only. |
| Virtual Capture Sink | Route to receiving app microphone selection; direction diagram only. |
| Test Signal | Quiet level, frequency, duration and routing diagnosis; correct levelDb sine generator. |
| Audio File | File approval, transport/seek semantics and load errors; synthetic stand-in, no personal file access. |
| Recorder | Folder prerequisite, session Play versus Record, finalize then file playback; early-stop take duration. |
| Network Send | Two AudioRouter PCs, destination IP, matching port/key, pairing before flow; delayed received copy after pairing. |
| Network Receive | Sender restriction, pairing versus buffer problems and jitter/latency tradeoff; matching two-PC scene and adjustable delay. |
| VST Plugin | Scan/inspect/add/save/play, exact identity, runtime health and deliberate recovery; no misleading generic plugin sound. |
| Visual Group | Name/resize/color/opacity/lock and unchanged wiring; no irrelevant audio. |

## Verification and evidence

Windows host, Node 22.22.3 and installed Microsoft Edge. No driver, hardware,
credential or external service is needed for these UI/documentation checks.

- `node site/generate-guides.mjs` — 35 pages, 101 configuration entries.
- `node site/check-guides.mjs` — pass: all 35 unique tutorials, property anchors,
  1,011 local links/assets, catalog links and UI/native kind parity.
- `node node_modules/typescript/bin/tsc --noEmit` in ui — pass.
- Focused ESLint for Workbench, ToolGuideLink, PluginPanels and both changed
  browser test files — pass. Targeted Prettier checks and `git diff --check`
  pass. No unrelated mass formatting or commit was performed.
- `npm.cmd run build` in ui — pass. The sandbox first denied removal of an old
  generated CSS asset; automatic escalation approved the same build, which
  completed. Existing large-bundle advisory remains.
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/docs.ps1`
  — pass. Counts vary when the final execution record is included.
- Edge: `node node_modules/@playwright/test/cli.js test
  --config=.guide-review.config.ts e2e/site-guides.pw.ts
  e2e/tool-help-spacing.pw.ts` in ui — **10 passed in 1.5 minutes**.
  Temporary config: one worker, Chromium browser with explicit executable
  `C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe`, viewport
  1440×1000, base URL `http://127.0.0.1:4186`, Vite route harness, traces and
  screenshots outside the repository. The temporary config was removed afterward.
- Screenshots/trace output:
  `C:/Users/miste/AppData/Local/Temp/audiorouter-guide-review-final-20261015/`.
  Reviewed dark/light/high-contrast Properties at 1440/1120/980 px, compact help,
  mobile/reduced-motion guides, and full Mixer example including sample inputs.

The numerical check verifies distinct-source correlation and exact weighted
mixing, solo source, pure delay, trigger-dependent Duck, quiet-bed Gate,
denoise residual, harmonic Dehum, click-threshold effect, pitch phrase timing,
Test Signal level, limiter ceiling, selected switch source and network pairing
silence. Browser checks cover all pages, controls, playback/pause/seek, EQ/FIR,
Network Send/Receive, early Recorder take length, reference audition, and
guide search. Tab tests hit-test every tab and verify at least 10 px clearance;
help is keyboard-opened and asserted to call the native guide opener without
adding a node, followed by successful card addition.

Failed experiments were resolved: a switch comparison needed floating-point
tolerance; new sample views placed sliders below the diagram, revealing that
diagram-only offscreen observation paused active editing. References now follow
the sliders and the entire example owns visibility. A finite-loop screenshot
raced completion, so the screenshot follows an explicit Pause. Final suite
passes with those fixes; earlier failures are not reported as passes.

## Limitations, rollback and next task

Examples qualify teaching behavior only, not production DSP, personal files,
network interoperability, hardware audio or release gates. Device/plugin/group
pages deliberately remain visual-only. No website publication occurred.

The existing desktop shell was running (PID 73136 at inventory). It was not
stopped or replaced. The UI assets are rebuilt; the running embedded executable
still needs a desktop rebuild after it is closed. No native source changed in
this follow-up.

Rollback only this follow-up's tutorial/content/runtime/generated pages,
ToolGuideLink, Workbench, PluginPanels, relevant styles, two browser tests,
site README, interface contract and plan record. Preserve earlier uncommitted
UI/backend work. No database migration or saved-session change is involved.

Next task: owner reviews local guides and the UI changes; close the current
desktop app before rebuilding/staging its executable. Website publication
remains separately authorized.
