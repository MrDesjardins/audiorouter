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
VCAB-21 one-LSB measurement for valid bits 16 or 24. Guarded VM cases
`pcm16-48k` and `pcm24-48k` use the native probe's actual shared mix format;
the endpoints must already be configured to the requested format and 48 kHz.
The runner rejects a mismatch and fails on dropped, flagged, or overflowed
capture packets. These cases are diagnostic and have not yet been measured in
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

Portable analyzer tests run with:

```powershell
cargo test --locked --manifest-path tools/m03-cable-quality/Cargo.toml
```

The sample comparison and THD+N modes consume canonical interleaved IEEE
float32 WAVs. The dedicated PCM fixture decoder accepts classic PCM16 and
PCM24-in-32 WAVEFORMATEXTENSIBLE and rejects other encodings. PCM32 remains
deferred because the current analyzer sample representation cannot establish
its one-LSB target. The VM runner can be invoked for 1, 2, or 8 channels, but
no channel matrix has been measured yet; each endpoint format must be set
before a run.
