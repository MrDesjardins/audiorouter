# M03 cable quality analyzer

The standalone analyzer consumes float32 WAVs written by the native WASAPI
probe and emits `metrics.json`:

```powershell
m03-cable-quality.exe analyze reference.wav capture.wav metrics.json [max-offset-frames] [frequency-hz]
```

It checks the WAV encoding and channel/rate match, aligns on channel zero,
compares every channel at the same frame offset, and reports per-channel
maximum/RMS error and optional THD+N. The first integrated VM case is
`tools/vm/run-cable-bitexact.ps1`, which runs the product cable route and
saves the probe's real endpoint samples. Its result remains diagnostic, not
VCAB-20 qualification.

Portable analyzer tests run with:

```powershell
cargo test --locked --manifest-path tools/m03-cable-quality/Cargo.toml
```

WAV decoding intentionally accepts only canonical interleaved IEEE float32
WAV. It rejects integer and extensible WAVs until explicit decoders and
fixtures exist; the 44.1/96 kHz, channel-count and integer-format cases remain
unimplemented.
