# WP-09b portable signal-analysis layer — 2026-10-11

Scope: first implementation slice for WP-09b / VCAB-20–26 measurement
support. `tools/m03-cable-quality` is an independent Rust crate with seeded
noise, tones, logarithmic sweeps, impulse trains, bounded sample alignment,
error metrics and a least-squares single-bin THD+N fit. Synthetic tests cover
reproducibility, alignment, corruption detection and sine distortion.

Checks on Windows host `AR-DriverTest`, pinned Rust 1.96.0:

- `cargo fmt --manifest-path tools/m03-cable-quality/Cargo.toml -- --check` —
  passed.
- `cargo test --offline --manifest-path tools/m03-cable-quality/Cargo.toml` —
  3 passed.
- `cargo clippy --offline --manifest-path tools/m03-cable-quality/Cargo.toml
  --all-targets -- -D warnings` — passed.
- `tests/acceptance/docs.ps1` — 148 Markdown files and 857 local links passed.
- `git diff --check` — passed.

The first online Cargo attempt could not resolve the crates.io index in the
restricted network environment. The identical build and test then succeeded
offline with cached dependencies. Cargo emitted a non-fatal Windows hard-link
warning for the incremental compilation cache.

Limitations: this layer does not open WASAPI, play or capture samples, write
`metrics.json`/WAV evidence, or satisfy A16. The VM runner, endpoint format
matrix, actual-route cases and all device measurements remain open. No audio,
driver or VM state was touched. A16's 8-channel acceptance remains unchanged.
