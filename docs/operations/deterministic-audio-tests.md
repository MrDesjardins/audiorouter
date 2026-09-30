# Deterministic audio transformation qualification

Run from the repository root with the pinned Rust toolchain:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/deterministic-audio.ps1
```

This generates synthetic signals, compiles real backend graphs, processes them
in 128-frame quanta, captures the outputs, and checks independent expectations.
It never starts the desktop, opens an audio device, records a microphone, or
changes sessions. No external audio download is needed. Noise uses fixed LCG
seeds 7 and 11; other inputs are tones, DC, impulses and stereo markers.

The runner writes float32 `input.wav`, `output.wav` and `metrics.json` under
`target/deterministic-audio-evidence/<case>/`. Mixer/Switch cases also retain
`input-b.wav`. Native plugin cases retain exact interleaved input/output arrays
in `samples.json`. Logs and `qualification.json` report each command and exit
code. Generated files are ignored by Git and can be regenerated; allow roughly
1.3 GB for the present sweeps. Supply `-ArtifactDirectory` and
`-TargetDirectory` to choose other output/build locations. Environment overrides
are restored when the script exits. A reused output directory may retain cases
from earlier versions; the current logs identify executed cases. Choose a fresh
artifact directory when you need a standalone run without older artifacts.

## What the checks prove

- Gain, Volume and Mute: exact linear scaling/silence, including both channels.
- Delay: exact requested sample offsets, initial silence, channel isolation and
  the 64-frame live read-tap crossfade against a reference waveform.
- Advanced/Parametric EQ: all eight shapes, Q and gain combinations, independent
  magnitude laws, notch rejection, all-pass phase change, and sixteen-band
  composition with enabled/disabled bands. Graphic EQ exercises every band.
- Bass & Treble: shelf asymptotes. Dehum: an independent double-precision cascade
  response at fundamental/harmonic/wanted frequencies, amount and harmonic counts.
- Compressor: independent hard/soft-knee transfer, ratio/threshold/makeup, attack
  and release time constants. Gate: expansion/range, hysteresis, finite hold and
  release, including linked stereo timing/channel-ratio preservation. Limiter:
  ceiling and exact declared lookahead at multiple rates.
- Declick: injected impulses repaired against a clean reference; clean samples
  pass exactly after disclosed lookahead.
- Denoise and Spectral Gate: learning transparency, stored-profile reuse, settled
  noise attenuation, neutral settings and wanted-tone retention. Speech Denoise
  tests noise suppression and an intermittent speech-like tone separately.
- FIR Filter: independent normalized direct convolution against a sparse stereo
  impulse response crossing the partition boundary, wet/dry and output gain.
  Generated float-WAV uploads additionally exercise actual decoding and mono/
  stereo resampling before convolution; uploaded and decoded IRs are retained.
- Pitch: measured frequency within ten cents for semitone/cent combinations,
  retained energy and unchanged frame count, plus sixty-second ±12-semitone
  runs with initial/midstream silence and subsequent signal recovery.
- Mixer and Input Switch: weighted sum or exact source selection from distinct
  deterministic inputs. Public running graph replacement tests verify normal/
  slow equal-power crossfades in both directions and reversal during a fade,
  comparing every output sample with a reference transition waveform.
- Time Shift: exact recorded history through live, pause, resume, back/forward
  ten seconds and live again, excluding only the documented jump fade.
- Cross-cutting checks: finite output, bypass/null comparison, stereo isolation,
  and fresh/reset repeatability for stateful processors.

Selected gain/filter/delay/pitch/limiter sweeps run at 44.1, 48 and 96 kHz;
spectral, FIR, dynamics timing and transport sweeps run at the internal 48 kHz.
Additional EQ frequency/Q/gain endpoint cases exercise 8 and 192 kHz; imported
FIR conversion cases exercise 8, 48 and 96 kHz from a 24 kHz source file.
These are meaningful representative and boundary configurations, not an
exhaustive proof over every parameter, signal or graph topology.

Meter and Recorder observe/copy audio; Audio File and Test Signal generate it;
capture/render and Network Send/Receive transport it. They are not transformations
in this one-effect matrix. Their source, recording, routing and transport tests
remain separate. VST hosting cannot prove arbitrary vendors' transfer functions.
The native fixtures below prove known gain laws through actual isolated workers.

## Native plugin fixtures

Use the approved repository VST2 state fixture and pinned SDK AGain VST3 module,
not arbitrary plugins renamed to those filenames. Build missing fixtures using
their existing M06 scripts. The native worker can be built locally with:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/m06-vst3-worker/build.ps1 -Output C:\code\audiorouter\target\m06-vst3-worker.exe
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/deterministic-audio.ps1 -NativeFixtures -Vst2Fixture third_party/local-test-fixtures/Vst2State/audiorouter-vst2-state-fixture.dll -Vst3Fixture third_party/vst3sdk-build/VST3/Release/again.vst3/Contents/x86_64-win/again.vst3 -Vst3Worker target/m06-vst3-worker.exe
```

The VST2 fixture checks five gains and sample-offset automation; AGain checks
five gains with bypass on/off, both at three sample rates. AGain's parameter
IDs come from the pinned SDK: gain 0, read-only meter 1, bypass 2. No native
editor is opened. Native fixtures are explicit opt-ins and are not redistributed.

## Known failing acceptance gate

DSP-12 is **not fully qualified**. At 48 kHz, 60 Hz fundamental, eight harmonics
and 100% amount, a 1003 Hz wanted tone retains approximately 92.31% amplitude;
the requirement allows at most 5% deviation. This follows the specified Q20,
−36 dB peaking cascade, so correct transfer-law execution does not satisfy the
separate preservation requirement. Fewer harmonics/lower amount can reduce this
effect, but do not resolve the general gate.

The named preservation test is explicitly ignored in ordinary `cargo test`
because it records this known defect. The qualification runner **always runs
it separately**, reports `qualified: false`, and exits unsuccessfully while the
defect exists. Do not treat the normal suite's green result as a release pass.
See the [dated evidence](../plans/active/evidence/2026-09-30-deterministic-audio.md).

Offline checks do not prove intelligibility, musical quality, callback deadlines,
WASAPI continuity, physical latency or every installed plugin's compatibility.
Those Windows/hardware and listening gates remain required separately.
