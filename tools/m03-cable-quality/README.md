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

Portable analyzer tests run with:

```powershell
cargo test --locked --manifest-path tools/m03-cable-quality/Cargo.toml
```

WAV decoding intentionally accepts only canonical interleaved IEEE float32
WAV. It rejects integer and extensible WAVs until explicit decoders and
fixtures exist; channel-count and integer-format cases remain unimplemented.
