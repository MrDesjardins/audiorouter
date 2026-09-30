# 2026-09-30 Dehum preservation correction

Status: DSP-12 failing offline cases fixed. This does not qualify all M04/M08
listening, hardware, latency or callback-deadline requirements.

## Reproduction and owning-layer correction

Affected baseline: commit `4d896636`. On Windows 10.0.26200.0 x64, Rust/Cargo
1.96.0, a 48 kHz synthetic 1003 Hz tone through 60 Hz Dehum, eight harmonics,
100% Amount retained 0.923110973461047 amplitude. The original 5% preservation
test failed with exit 101; local log: `target/dehum-before.log`.

Deep Q20 peaking cuts widen with depth and overlap. The DSP now prepares
finite-depth notches with harmonic Q=20*h; Amount changes numerator depth,
retaining the specified 0 to -36 dB center attenuation without moving poles.
The graph compiler uses this preparation for mono and stereo Dehum. Existing
fixed-capacity biquad processing adds no latency or callback allocation.
The DSP-12 contract records the algorithm and wanted passband domain outside
center +/- fundamental/8 transition neighborhoods. The original 1003 Hz test
and its 5% tolerance are unchanged; it is no longer ignored.

The independent double-precision transfer oracle evaluates an algebraically
equivalent peaking response with Q_peak=20*h/sqrt(center_gain), rather than
calling production coefficient construction. Additional tests project individual
hum and wanted components in stereo mixtures, sweep passband boundaries and
inter-harmonic tones, and cover Amount zero and reset repeatability.

## Verification

From `C:\code\audiorouter`, using locked dependencies and release builds:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/deterministic-audio.ps1 -TargetDirectory target/groups-http-20260929 -ArtifactDirectory target/dehum-fix-evidence -NativeFixtures -Vst2Fixture third_party/local-test-fixtures/Vst2State/audiorouter-vst2-state-fixture.dll -Vst3Fixture third_party/vst3sdk-build/VST3/Release/again.vst3/Contents/x86_64-win/again.vst3 -Vst3Worker target/deterministic-plugin/m06-vst3-worker.exe
```

Exit 0. Logs and `qualification.json` under `target/dehum-fix-evidence`:

- Built-in signals: 30 pass, zero failed/ignored. Added 132 passband cases
  across 8/48/192 kHz and 45/50/60/65 Hz fundamentals, six stereo program/hum
  mixtures and the original wanted-tone regression.
- DSP/engine units: 52 + 145 pass.
- Separately executed unchanged preservation gate: one pass. Amplitude ratio
  0.9999558431167525, approximately 0.0044% loss versus the prior 7.69% loss.
- Actual isolated native VST2 and VST3 gain fixtures: two tests pass.

Review experiment: assigning harmonic Q directly to the generic EQ constructor
failed its Q<=20 validation (`target/dehum-final-focused.log`, exit 101).
Correction keeps validated generic state initialization at Q20 and installs the
dedicated Dehum coefficients privately, without widening public EQ limits.
The full command above was rerun afterward and all four checks returned zero.
Documentation acceptance passes: 78 Markdown files, 419 local links. The
deterministic test file passes rustfmt checking; `git diff --check` passes.

Generated float-WAV inputs/outputs and JSON metrics remain ignored local
artifacts; no microphone, endpoint, desktop or saved session was changed.
The runner's `qualified: true` applies only to these requested offline checks.
Native WASAPI continuity, physical latency, callback timing and real-speech
listening were not run. Preserve active user routes when scheduling those gates.

## Compatibility and rollback

Existing saved parameters remain valid and require no migration. Full Amount
removes less wanted audio between hum harmonics. No desktop executable was
rebuilt for this change. Revert the DSP preparation, graph compiler and DSP-12
contract together to roll back; retain the regression and reopen its gate.
Next task: guarded native Dehum continuity/performance and listening evidence.
