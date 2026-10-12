# WP-09b portable signal-analysis layer — 2026-10-11

Scope: implementation progress for WP-09b / VCAB-20–26 measurement support.
`tools/m03-cable-quality` is an independent Rust crate/CLI with seeded noise,
tones, logarithmic sweeps, impulse trains, float32 WAV decoding, bounded
frame alignment, per-channel error metrics and a least-squares single-bin
THD+N fit. The native WASAPI bit-exact probe writes its real reference and
captured samples as float32 WAVs on request; the product-route VM script calls
the Rust analyzer and preserves `metrics.json` in its evidence bundle. A
guarded VM entry point exposes float32 cases at 44.1/48/96 kHz (guest Cable
formats must be set to match), a single unrouted Cable A-to-B isolation case,
and a 56 ordered-pair isolation runner that temporarily enables eight cables
and restores the initial count in `finally`. Every case is marked diagnostic;
the full A16 gate remains incomplete.
Synthetic tests cover reproducibility, alignment, channel-specific corruption
detection, sine distortion and rejection of non-finite WAV samples.

Checks on Windows host `AR-DriverTest`, pinned Rust 1.96.0:

- `cargo fmt --manifest-path tools/m03-cable-quality/Cargo.toml -- --check` —
  passed.
- `cargo test --offline --manifest-path tools/m03-cable-quality/Cargo.toml` —
  6 passed.
- `cargo clippy --offline --manifest-path tools/m03-cable-quality/Cargo.toml
  --all-targets -- -D warnings` — passed.
- `tests/acceptance/docs.ps1` — 149 Markdown files and 858 local links passed.
- `tests/acceptance/m03-cable-quality.ps1` — 19 host-safe checks passed.
- `tests/acceptance/m03-cable-bitexact.ps1` — 7 host-safe checks passed.
- Native probe `cable-bitexact-selftest` — 10 passed. Cross-tool WAV writer →
  Rust analyzer self-test — bit exact, 2 channels, four-frame alignment.
- Native probe `cable-isolation-selftest` — 4 passed. Cross-tool silence WAV →
  Rust analyzer reports −1000 dBFS and passes the −140 dBFS threshold.
- The VM entry point's PowerShell parse and static scope guard passed through
  the host-safe acceptance test.
- Release analyzer built with `RUSTFLAGS=-C target-feature=+crt-static`; PE
  import inspection passed (`api-ms-win-core-synch-l1-2-0.dll`, `KERNEL32.dll`,
  `ntdll.dll`; no Visual C++ runtime dependency).
- `git diff --check` — passed.

The first online Cargo attempt could not resolve the crates.io index in the
restricted network environment. The identical build and test then succeeded
offline with cached dependencies. Cargo emitted a non-fatal Windows hard-link
warning for the incremental compilation cache.

Limitations: the analyzer itself does not open WASAPI. Live endpoint samples
are produced by the existing native Windows probe, and its current direct
measurement supports only the matching float32 shared mix format. The wired
product-route case has not been run in the VM. It covers neither all A16
rates/formats/channel counts nor the full VCAB-20–26 matrix. No audio,
driver or VM state was touched. A16's 8-channel acceptance remains unchanged.
