# 2026-09-30 deterministic audio transformation evidence

Status: substantial offline signal qualification; **not a complete release
gate**. DSP-12 wanted-band preservation fails and remains explicitly open.

## Environment and scope

Windows NT 10.0.26200.0, x64; Rust/Cargo 1.96.0, locked dependencies,
release builds. Synthetic inputs only, fixed seeds 7 and 11. No audio endpoint,
microphone, real session, database, running desktop or machine configuration was
changed. Tests use production graph compilers and isolated native workers.

Requirements addressed in part: DSP-01–05/07/08/10–18, GRAPH-02/04/10/14,
QUAL-01–03, PLUG-03/04/07. These IDs contain broader obligations than this
matrix. QUAL-04/05 timing/glitch/hardware gates, presets, metering, subjective
quality and arbitrary vendor compatibility are not established.

## Commands and results

Final command, from `C:\code\audiorouter`:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/deterministic-audio.ps1 -TargetDirectory target/groups-http-20260929 -NativeFixtures -Vst2Fixture third_party/local-test-fixtures/Vst2State/audiorouter-vst2-state-fixture.dll -Vst3Fixture third_party/vst3sdk-build/VST3/Release/again.vst3/Contents/x86_64-win/again.vst3 -Vst3Worker target/deterministic-plugin/m06-vst3-worker.exe
```

Exact Cargo arguments and exit codes are retained in
`target/deterministic-audio-evidence/qualification.json`:

- `built-in-signals`: exit 0; **21 passed, one explicitly ignored known gate**.
  899 measurements, 881 unique named cases with input/output float WAVs and
  measured/expected JSON. Dehum fundamental/highest-harmonic entries coincide
  for one harmonic; unique counts exclude these duplicates.
- `dsp-engine-regressions`: exit 0; **52 DSP + 145 engine tests passed**.
- `native-plugin-signals`: exit 0; **two tests passed**, covering 48 worker
  configurations: VST2 five gains plus a frame-offset change; VST3 five gains
  with bypass off/on, each at 44.1/48/96 kHz. Input/output arrays are retained.
- `dehum-preservation-gate`: exit 101; **one failed**. Wanted amplitude ratio
  **0.923110973461047**, approximately 7.69% attenuation rather than ≤5% deviation.
- Overall runner: exit 1, `qualified: false`, solely because of that gate.

Additional run: `cargo test -p audiorouter-plugin-host --features test-fixtures
--release --locked --target-dir target/deterministic-plugin` passed 73 library,
two binary and 35 worker integration tests. Native opt-ins were reported ignored
by that command, then explicitly executed separately above. Log:
`target/deterministic-audio-plugin-suite.log`.

A complete runner replay, before the final two floor cases and removal of
irrelevant repeated Mixer/Switch configurations, yielded identical SHA-256
hashes for **2,667 WAV and metric files**. Local
`target/deterministic-audio-repeat-result.json` records zero changed files.
Fresh/reset bit-exact tests also run on every normal invocation.

Generated audio, JSON, logs and native binaries are ignored local evidence;
regenerate them using the committed runner. The
[qualification guide](../../../operations/deterministic-audio-tests.md) records
tool inventory, frequencies, settings, windows, tolerances and native setup.

## Proven defects repaired

1. **Limiter:** impulse appeared at N+1 while lookahead reported N. The
   read-before-write ring now has N slots (one inert slot for zero lookahead).
   Exact impulse timing/ceiling sweeps pass at three rates.
2. **Speech Denoise reset:** smoothed power/noise estimates survived STFT/frame
   reset. Clearing them reproduces fresh output bit-for-bit.
3. **Biquad precision:** a 48 kHz, 60 Hz Q20/−36 dB cut measured about −34.92 dB
   even after eight seconds of warmup. Double-precision coefficients and
   recursive state now match the independent transfer law; audio buffers remain
   f32. No runtime allocation was added.
4. **Gate:** an expired positive hold reloaded forever below threshold, leaving
   the gate open after seconds of quiet. Reload only while the signal sustains
   the open state, then expire once. Both interleaved and linked stereo paths
   changed. Independent envelope tests cover holds 0/50/1000 ms and releases
   10/150/2000 ms, plus hysteresis.

Initial logs remain locally under `target/deterministic-audio-*`: first/second
runs and gate-first reproduce the failures. The first Dehum oracle wrongly
assumed harmonic cuts were isolated; the final oracle sums an independent f64
cascade. AGain's initial bypass fixture used read-only meter ID 1; checking the
pinned SDK corrected bypass to ID 2. Acceptance was not weakened. The runner
uses native exit codes because Windows PowerShell wraps Cargo's stderr progress
as errors; environment overrides are restored in `finally`.

## Compatibility, risks and next task

No migration is needed. Gate settings now close after hold; limiter delay is
one sample shorter and matches telemetry; low-frequency filters gain precision;
Speech Denoise reset deliberately discards adaptive estimates. These changes are
in source; the user's running desktop executable was not replaced or rebuilt.

Resolve DSP-12's wanted-band preservation versus the specified Q20/−36 dB
cascade without silently weakening either requirement. Ordinary Cargo tests
explicitly ignore its known-failure test; the qualification runner always runs
it and fails while the gate is open.

Remaining qualification: slow live Input Switch fade/position carry with exact
transition waveforms; more parameter/rate endpoints; FIR media decode/resampling;
transport bounds/drift; callback CPU and native continuity after numeric DSP
changes; real speech/music listening. Static selection is tested for normal
and slow settings; existing tests cover normal equal-power live fade. No finite
matrix proves every signal, configuration, topology or vendor effect.

Rollback: revert owning-layer corrections and their regressions together if a
verified incompatibility requires it; retain the harness and record the new
failure. No device/storage rollback is needed. The active plan remains open
until remaining required gates have evidence.

## Continuation: transition and boundary qualification

Starting from commit `a626cdf0`, six further test groups passed without DSP
implementation changes. New signal evidence covers:

- Eight Input Switch transitions: normal/slow, A→B/B→A, uninterrupted/mid-fade
  reversal. Public `RealtimeMixerFanout.replace_graph` and bounded input/output
  rings exercise production position carry. A double-precision equal-power
  endpoint/interpolated reference checks every sample with 1e-5 tolerance.
- Sixteen peaking-EQ cases: 8/192 kHz, 20 Hz/highest eligible frequency,
  Q 0.1/20 and gain ±24 dB, ten-second warmup and ±0.5 dB center response.
- Two sixty-second Pitch cases: ±12 semitones, initial/midstream silence,
  signal recovery, zero frame-count deviation and ≤10 cents at three windows.
- Three live stereo Delay tap changes: 0→240, 240→0, 240→480 samples,
  compared with the 64-frame reference crossfade at ≤1e-6 sample error.
- Nine linked stereo Gate cases: attack 0.1/5/100 ms, hold 0/50/1000 ms,
  release 150 ms; independent exponential envelope within 0.1 dB, retained
  4:1 channel ratio. Complements the mono release sweeps.
- Six imported FIR cases: generated mono/stereo float WAVs at 24 kHz decoded
  to 8/48/96 kHz; independent linear-resampling error ≤1e-6, normalized direct
  convolution error ≤1e-3. Uploaded/decoded IR WAVs accompany route outputs.

Full qualification command is unchanged. Final results: **27 normal signal
tests pass**, 943 measurements / **925 unique cases**; **197 DSP/engine unit
tests** and **two native plugin signal tests** pass. The Dehum gate again fails
with amplitude ratio 0.923110973461047 and `qualified: false`; runner exit 1
does not hide the other command results. Local log:
`target/deterministic-audio-continuation-runner.log`, individual logs and JSON
under `target/deterministic-audio-evidence`. Rustfmt checks on changed test files,
authored diff checks and documentation/link validation pass.

Next: resolve DSP-12's algorithm/preservation domain, then remaining frequency,
signal and hardware/callback/load boundaries. Slow fade, live position carry,
60-second pitch and FIR WAV decode/resampling gaps above are now covered.
MP3 IR import, every possible rate/setting and subjective listening remain
unqualified. No executable was launched, rebuilt or replaced for this round.
