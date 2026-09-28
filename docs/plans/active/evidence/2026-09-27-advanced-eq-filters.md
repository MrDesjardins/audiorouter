# Advanced EQ Band pass and All pass — 2026-09-27

Scope: DSP-02/08 and UI-05/11, M04/M05. Windows host, local Rust/Node
toolchains, Edge. Implemented constant-0-dB-peak Band pass and phase-changing
unity-magnitude All pass; existing enum values and defaults are preserved.

## Automated verification

- `cargo test -p audiorouter-dsp -p audiorouter-domain -p audiorouter-engine
  -p audiorouter-control --locked`: passed. DSP 52, domain 70, engine 139,
  control 202 unit tests, plus integration/doc tests. New DSP vectors cover
  8/44.1/48/192 kHz, Q 0.1/1/20, magnitude, impulse energy/phase effect,
  silence, finite boundary output. Engine fixture compiles all eight types.
- After adding domain round-trip and response API assertions:
  `cargo test -p audiorouter-domain --locked` (70 passed), and
  `cargo test -p audiorouter-control
  processors_response_uses_bounded_shared_eq_coefficients --locked` (1 passed).
- `npx.cmd vitest run --configLoader runner src` in ui: 368 passed.
- Contracts `npm.cmd run typecheck` and `npm.cmd run check:drift`: passed.
- UI `npm.cmd run build`: TypeScript passed; Vite failed deleting a locked
  dist asset. `npx.cmd vite build --configLoader runner --outDir
  dist-review-eq` passed with elevated execution after a sandbox EPERM.
  This review bundle is not a rebuilt desktop release.
- `cargo build -p audiorouter-control --example e2e_backend --locked`, then
  `npx.cmd playwright test --config=playwright.config.ts
  advanced-eq-filters.pw.ts` in ui: 3 passed. Both choices persist across
  Save/reload; gain is disabled, Q enabled, and the actual response arrives.
  Six screenshots visually inspected in dark/light/high contrast:
  `%TEMP%/audiorouter-designer-review/eq-<theme>-<bandPass|allPass>.png`.
  Initial screenshot checks raced response arrival. A subsequent assertion
  incorrectly treated a flat SVG polyline's zero-height box as hidden;
  corrected to require attachment and completed response before screenshots.
- Documentation acceptance and `git diff --check`: passed. RTK unavailable;
  direct commands used. No release shell was launched or replaced.

## Live continuity — qualification remains partially open

Command: `cargo test -p audiorouter-transport --test live_audio_continuity
live_backend_service_keeps_a_routed_tone_continuous --locked -- --ignored
--nocapture`, with `AUDIOROUTER_LIVE_CONTINUITY=1`,
`AUDIOROUTER_CONTINUITY_CHAIN=parametricEq`,
`AUDIOROUTER_CONTINUITY_EQ_TYPE=<type>`,
`AUDIOROUTER_CONTINUITY_TONE_HZ=47`, and
`AUDIOROUTER_CONTINUITY_SECONDS=30`. Uses the harness's exact VB-Cable
source/reference and CABLE-B render/result endpoint IDs, native multi-path
worker, quiet test tone, no microphone or device-default changes.

- Band pass first run: reference 2 glitches, result 2; inconclusive.
- All pass: reference 0 glitches, result 0, output underruns 0,
  late service gaps 0, max service gap 4452 microseconds; passed.
- Band pass repeat: reference 0, result 3 glitches, output underruns 5,
  late service gaps 0; failed continuity gate.
- Comparison with unchanged Gain (`AUDIOROUTER_CONTINUITY_CHAIN=gain`,
  no EQ override): reference 0, result 2 glitches, output underruns 4,
  late service gaps 0; also failed. This supports a shared native-route or
  environmental issue rather than isolating a Band pass defect, but does
  not waive the Band pass gate. Offline behavior is verified; Band pass
  needs a zero-glitch rerun on a clean reference before release qualification.

Next: rerun Band pass continuity, then package the filters for attended use.
