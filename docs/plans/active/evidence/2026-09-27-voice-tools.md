# Local voice tool qualification — 2026-09-27

Requirements: DSP-01–18, GRAPH-04/14/15, REC behavior and installed plugin
parameter delivery. Windows x64, local approved five-second 48 kHz mono
sample. No voice data or transcript is included in the repository or logs.
Private WAVs and aggregate metrics remain in the user's temp directory.

## Bass & Treble defect and decision

The original fixed 120 Hz/6 kHz shelves functioned correctly on tones but
poorly suited the user's speech expectations. Treble +12 produced only
0.03 dB overall RMS change and +3.25 dB in upper speech probes. These are
different quantities: unchanged overall RMS does not mean unchanged tone.
The 300/3000 Hz candidate increased the upper probes +9.13 dB, but the
user still found the loudness-matched audition too subtle.

Final defaults: bass 500 Hz, treble 1500 Hz. Their separate +12 dB
settings produce +11.75 dB low and +11.50 dB upper probe changes.
Frequency controls permit bass 80–1000 and treble 800–12000 Hz. Gain
range remains ±12 dB. User confirmed the final flat → warm/dark →
thin/bright playback was strong enough. Same sample, RMS-matched versions,
common quiet playback scaling through the exact Scarlett output. No
comparison against Voicemeeter or Audio Hijack binaries was available.

Compatibility: missing frequency parameters use new defaults; saved
nonzero gains consequently sound stronger. Explicit 120/6000 frequencies
restore the old response. No database migration or user graph edits.

## Reproducible checks

Set `AUDIOROUTER_VOICE_SAMPLE` to the consented local PCM16 mono WAV.
Run `cargo test -p audiorouter-engine --test voice_tools -- --ignored`.
The ignored harness tests graph processing with actual speech:

- Bass/Treble independent gains and opposite tonal settings; all eight
  Advanced EQ filter types (All Pass changes phase), Graphic EQ and Pitch.
- Gain −12 dB and Volume 25% yield expected uniform attenuation; Mute
  is silent; Meter preserves samples; Delay aligns exactly at 100 ms.
- Compressor reduces RMS over 13 dB; closed Gate attenuates about 60 dB;
  Limiter caps amplified speech at the configured −12 dBFS ceiling.
- Dehum removes injected 60 Hz hum; Declick reduces injected impulse
  error by more than half; learned Denoise/FIR Filter Hz and adaptive
  Speech Denoise suppress steady hiss while preserving speech level.
  Clean speech preservation is checked separately.
- Unconfigured FIR is transparent; configured two-tap convolution changes
  speech. Audio File reproduces decoded samples exactly. Mixer and Input
  Switch produce the selected/summed sample values. Time Shift is neutral
  live, silent paused, buffers/rewinds and returns to live.

Run `cargo test -p audiorouter-transport --test voice_delivery -- --ignored`:
passed. PCM16/24/float32 WAV round trips have bounded quantization error;
MP3 decode retains speech energy. Production Network Send delivers every
sample exactly to a localhost reference receiver. Production Network
Receive preserves speech energy/waveform after its jitter buffer, with
zero lost/rejected/overflow packets. Only localhost is qualified here.

Run `cargo test -p audiorouter-plugin-host --test private_voice -- --ignored`
with approved plugin-root and isolated-worker environment variables:
passed for nine installed x64 ReaPlugs. Every worker processes the full
sample finitely; Wet/Gain/Thresh edits alter samples where exposed.
ReaControlMIDI, ReaJS without a selected script, and ReaStream without a
configured remote destination are processing checks, not tonal-effect or
remote-delivery claims. Initial ReaGate Wet test correctly failed because
both versions were silent; using the declared Thresh control passes.

Unavailable owned virtual-device tools and the placeholder Endpoint
loopback library item remain unavailable. Physical Input is the original
consented sample's capture path; exact Scarlett output was auditioned.
These checks do not establish every possible configuration, third-party
plugin sound quality, second-PC networking, or M08 release readiness.
Test Signal creates a tone rather than modifying speech; its generated-source
continuity is covered by the existing [continuity qualification](2026-09-26-audio-continuity.md).

## Regression, UI and continuity

- Engine: 141 unit + five integration tests passed, including synthetic
  speech-frequency shelf regressions and live graph replacement tests.
- Domain 70 tests and control suite passed (hardware opt-ins remain ignored).
- UI 370 tests passed with the final frequency controls. UI build passed
  with the existing bundle-size warning.
- Edge Bass/Treble E2E passed in dark/light/high-contrast. Initial browser
  failures were missing/malformed catalog fixture and incorrect accessible
  labels; corrected fixture uses the versioned processor shape.
- Native continuity: `AUDIOROUTER_LIVE_CONTINUITY=1`, chain `bassTreble`,
  47 Hz, 30 seconds, existing CABLE → CABLE-B route. Reference and result:
  zero glitches, zero silent runs, zero WASAPI discontinuity flags.
  Backend late gaps zero; output underrun counter four includes startup,
  while the measured steady waveform remains continuous.

Rollback: retain release-9 executable; revert only the new shelf/schema/UI
changes or set old frequencies explicitly. Private audio is not needed
for standard CI; the synthetic regression remains reproducible without it.

## Build

`cargo build --manifest-path src-tauri/Cargo.toml --release --features custom-protocol`
with `CARGO_TARGET_DIR=C:\code\audiorouter\target\patrick-main-release-10`.
UI `index-3aXwyTtB.js` embedded and timestamps checked. Matching source
CLI/plugin worker built adjacent with the root workspace release build.
Keep the worker beside the shell when running installed plugins.
