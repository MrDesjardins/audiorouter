# M03 cable quality analyzer

The standalone analyzer consumes float32 WAVs written by the native WASAPI
probe and emits `metrics.json`:

```powershell
m03-cable-quality.exe analyze reference.wav capture.wav metrics.json [max-offset-frames] [frequency-hz]
```

It checks the WAV encoding and channel/rate match, aligns on channel zero,
compares every channel at the same frame offset, and reports per-channel
maximum/RMS error and optional THD+N. The integrated VM runner accepts
float32 shared mix rates of 44.1, 48 and 96 kHz when both cable endpoints are
configured to that rate. It runs the product cable route and saves the probe's
real endpoint samples. Its result remains diagnostic, not VCAB-20 qualification.

The `isolation` probe case writes the captured Cable B Output to float32 WAV;
`m03-cable-quality.exe silence capture.wav metrics.json -140` reports peak,
RMS, DC and a VCAB-26 threshold verdict. The VM wrapper's Cable A-to-B case is
one pair only and does not close the all-pairs isolation requirement.

PCM16 and PCM24-in-32 WAV fixture codecs are available through the library.
The `pcm` analyzer mode accepts float32 reference/capture WAVs and reports the
VCAB-21 one-LSB measurement for valid bits 16 or 24. For 32 bits it requires
float64 reference/capture WAVs. Guarded VM cases cover PCM16, PCM24-in-32 and
PCM32 at 44.1, 48 and 96 kHz. Set both endpoints to the requested rate and
format first; the runner checks the native probe's negotiated shared mix
format and fails on dropped, flagged, or overflowed capture packets. Add
`-ChannelMatrix` to run a supported format/rate case at 1, 2 and 8 channels in
sequence. PCM32 uses deterministic full-resolution integer-grid noise and
float64 captures. These cases are diagnostic and have not yet been measured in
the VM.

The `sweep` analyzer mode compares 48 logarithmically spaced local magnitude
projections from a reference/capture pair. Guarded VM cases
`sweep-44k1`, `sweep-48k`, and `sweep-96k` generate a 20 Hz–20 kHz sweep through
the product route and report the worst channel deviation against ±0.05 dB.
This slice measures passband magnitude only; it does not measure aliasing,
imaging, phase, or group delay. The diagnostic cases have not been run in the
VM.

The `tone-continuity` VM case reuses `run-cable-tone.ps1` for the product-route
997 Hz and 47 Hz continuity method. Set `-Minutes` on the quality runner to
choose the duration per tone. It is a diagnostic run; A16's one-hour and
eight-cable gates still require their full execution and evidence.

The `eight-cable-continuity` case reuses the guarded four-route runner for
Cable A→B, C→D, E→F and G→H with identity-coded impulses. Its `-Minutes`
argument controls the run length and the runner restores the VM's prior cable
count in `finally`. Short runs are diagnostic only.

Portable analyzer tests run with:

```powershell
cargo test --locked --manifest-path tools/m03-cable-quality/Cargo.toml
```

The sample comparison and THD+N modes consume canonical interleaved IEEE
float32 WAVs. The dedicated PCM fixture decoder accepts classic PCM16 and
PCM24-in-32 WAVEFORMATEXTENSIBLE and rejects other encodings. PCM32 analysis
accepts float64 reference/capture WAVs so the one-LSB metric retains precision
below float32 resolution. The native probe recognizes plain and extensible
PCM32 and writes full-precision captures. The VM runner accepts channel counts
1 through 8; use `-ChannelMatrix` for the required 1/2/8 points. The matrix has
not been measured in the VM; each endpoint format must be set before a run.
