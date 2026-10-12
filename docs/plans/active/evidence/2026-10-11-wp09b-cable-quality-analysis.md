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

## 997 Hz product-route THD+N slice

The native probe now has a `cable-tone` mode that writes a requested sine
through the shared float32 cable endpoints and saves reference/capture WAVs.
The Rust analyzer aligns the capture to the reference and reports per-channel
THD+N with a worst-channel VCAB-22/23 diagnostic threshold. The guarded
`run-cable-quality.ps1 -Case thdn-997-48k` launches `m03_cable_session`, runs
the probe, analyzes the files, and packages a result explicitly marked
`Qualification=false`. Defaults are 2 channels, 997 Hz, and a -60 dB THD+N
diagnostic threshold. No live VM run has been performed.

Checks on Windows host `AR-DriverTest`, pinned Rust 1.96.0:

- `cargo fmt --manifest-path tools/m03-cable-quality/Cargo.toml -- --check` —
  passed.
- Offline analyzer tests — 7 passed; standalone Clippy with `-D warnings` —
  passed.
- `tests/acceptance/m03-cable-quality.ps1` — 24 host-safe checks passed,
  including PowerShell parse and host guard checks for the new runner.
- `tests/acceptance/docs.ps1` — 149 Markdown files and 858 local links passed.
- Native probe static build and existing self-checks — 10 bit-exact and 4
  isolation checks passed.
- Static CRT release analyzer build and PE import check passed; imports are
  `api-ms-win-core-synch-l1-2-0.dll`, `KERNEL32.dll`, and `ntdll.dll`.
- `git diff --check` — passed.

The initial acceptance run found a malformed PowerShell assertion regex caused
by variable expansion; the assertion now uses literal substring checks and
the full acceptance script passes. Static probe compilation does not exercise
WASAPI. Native endpoint capture, route THD+N, and all other A16 cases remain
unmeasured.

## Portable integer PCM WAV codecs

Added strict `read_pcm_wav` / `write_pcm_wav` support for classic PCM16 and
PCM24-in-32 WAVEFORMATEXTENSIBLE. The 24-bit reader checks the PCM subformat
GUID, valid/container bit widths, whole frames, and zero low padding bits;
the writer rounds to nearest, clamps to the representable range, and stores
24-bit signed values left-aligned in the 32-bit sample. Synthetic stereo
round-trips preserve channel distinctions and stay within one LSB. Non-finite
samples, partial frames and malformed padding are rejected. PCM32 remains
deferred because the analyzer currently stores decoded samples as f32; no
integer format has been negotiated with a live endpoint.

Checks on Windows host `AR-DriverTest`, pinned Rust 1.96.0:

- Analyzer unit tests — 9 passed; standalone Clippy with `-D warnings` —
  passed.
- Documentation validation — 149 Markdown files and 858 local links passed.
- `git diff --check` — passed.

The first build found a Rust type inference error in the rejection path; the
result type was made explicit. The first Clippy pass then identified a complex
format tuple and manual divisibility check; these were replaced by a named
format struct and `is_multiple_of`. These were local build/lint issues; no
endpoint, driver or VM state was touched.

## Native PCM shared-format diagnostic wiring

The native probe now accepts Windows `GetMixFormat` results for float32,
PCM16, and PCM24-in-32, emits the actual encoding, quantizes render samples,
and decodes captured samples to normalized float WAV data. The product-route
runner has a `-PcmBits 16|24` mode that checks the actual format and rate before
using the analyzer's VCAB-21 one-LSB metric; any dropped frames, flagged
capture packets, or capture overflow fail the result. The quality entry point exposes
`pcm16-48k` and `pcm24-48k`; bundle preparation runs the new offline integer
conversion self-check. PCM32 and other rates/channel counts remain out of this
path. These are static/host-safe checks only; no endpoint was opened.

Checks on Windows host `AR-DriverTest`, pinned Rust 1.96.0:

- Probe static build — passed; integer conversion self-test — 12 checks;
  existing bit-exact and isolation self-tests — 10 and 4 checks.
- Analyzer tests — 10 passed; Clippy (`-D warnings`) — passed.
- Cable quality acceptance — 27 host-safe checks passed, including script
  parsing, guarded case wiring, and refusal to start audio on the host.
- `git diff --check` — passed.

The first Clippy run found one unnecessary same-type cast in the new PCM
verdict regression; it was removed and all checks passed. No driver, endpoint,
VM or device setting was changed.

## Channel-count alignment regression

Added an analyzer regression for 1-, 2-, and 8-channel float32 captures.
Each case uses independent deterministic channel signals, applies one shared
frame offset, verifies per-channel exactness, and injects corruption into the
last channel to ensure shared alignment does not hide channel-specific
errors. The tool README now documents the available PCM codec and runner
boundary, and states that the VM matrix remains unmeasured.

Checks on Windows host `AR-DriverTest`, pinned Rust 1.96.0:

- Analyzer unit tests — 11 passed; standalone Clippy (`-D warnings`) — passed.
- Cable quality host acceptance — 27 checks passed.
- Documentation validation — 149 Markdown files and 858 local links passed.
- `git diff --check` — passed.

This is portable analyzer evidence only. The A16 1/2/8-channel endpoint
matrix remains unrun in the VM.

## Log-sweep passband response diagnostic

Added `analyze_sweep_pair_wav` and CLI `sweep` mode. It aligns reference and
capture, samples 48 logarithmically spaced local Hann-windowed projections
from 20 Hz to 20 kHz, and reports per-channel relative magnitude deviation
against the VCAB-22 ±0.05 dB passband target. A synthetic 0.03 dB gain passes;
0.10 dB fails, and a request above Nyquist is rejected. The native probe now
generates a matching logarithmic sweep and saves float32 reference/capture
WAVs. The guarded product-route runner exposes 44.1/48/96 kHz cases and checks
the negotiated rate, channel count, float32 format, and clean capture counters.
Each report states that aliasing, imaging and phase are outside this slice.

Checks on Windows host `PATRICK5080`, pinned Rust 1.96.0:

- Analyzer tests — 12 passed; standalone Clippy (`-D warnings`) — passed.
- Probe static build, sample-format self-test (12), and bit-exact self-test
  (10) — passed.
- Cable quality acceptance — 33 host-safe checks passed, including parsing
  the new runner and refusing audio use outside the VM.
- Documentation validation — 149 Markdown files and 858 local links passed.
- `git diff --check` — passed.

No endpoint, driver or VM state was touched. The sweep measurement remains
synthetic/host-checked until its guarded runner is executed in the VM.

## Continuity case entry point

`run-cable-quality.ps1` now exposes `tone-continuity` and forwards the selected
`-Minutes` value to the existing guarded product-route runner for its 997 Hz
and 47 Hz checks. This reuses the established sample-level glitch detector
and Cable A loopback reference. The new host acceptance parses both scripts
and checks the forwarding; it does not start them or open audio.

Checks on Windows host `PATRICK5080`:

- `tests/acceptance/m03-cable-quality.ps1` — 34 host-safe checks passed.
- `tests/acceptance/docs.ps1` — 149 Markdown files and 858 local links passed.
- No continuity or endpoint run was performed.

The same quality entry point now exposes `eight-cable-continuity`, forwarding
`-Minutes` to the existing four-route runner. That runner enables eight cables
and restores the prior cable count in `finally`. The host-safe acceptance
checks the case and runner wiring only; no helper or driver operation ran.

## PCM32 float64 analyzer path

Added strict IEEE float64 WAV read/write and a PCM32 one-LSB comparison path.
The analyzer aligns on a float32 projection only to locate the frame offset,
then computes all errors in float64. A synthetic stereo fixture contains
values below float32 resolution, passes after PCM32 quantization, and fails
when one channel is corrupted by two LSBs. This proves the portable analyzer
can evaluate the target without collapsing samples to float32. The native
probe and VM runner still do not produce PCM32 captures, so there is no
end-to-end PCM32 evidence.

Checks on Windows host `PATRICK5080`, pinned Rust 1.96.0:

- Analyzer tests — 13 passed; standalone Clippy (`-D warnings`) — passed.
- The initial regression used a sample whose float32 rounding happened to be
  within one PCM32 LSB; it was corrected to require at least one sample whose
  float32 projection loses more than one LSB.
- Cable quality acceptance — 35 host-safe checks passed; documentation
  validation — 149 Markdown files and 858 local links passed.
- `cargo fmt --manifest-path tools/m03-cable-quality/Cargo.toml -- --check`
  and `git diff --check` — passed.

The first format check requested one rustfmt line wrap; formatting was applied
and the complete portable checks then passed.

## Native PCM32 preservation and guarded route case

The WASAPI probe now recognizes plain and extensible signed PCM32 formats,
generates deterministic samples on the full 32-bit integer grid, preserves
captured samples in `double`, and writes IEEE float64 reference/capture WAVs.
Its native conversion self-check exercises a sample with nonzero low eight
bits and verifies at most one PCM32 LSB error. The VM runner accepts 32-bit PCM
and the quality entry point exposes `pcm32-48k`; negotiated format and capture
continuity remain required for PASS.

Host checks on `PATRICK5080`:

- Static MSVC build passed; `cable-sample-format-selftest` — 17 checks passed,
  including plain/extensible PCM32 recognition, deterministic full-resolution
  noise and positive/negative full-scale conversion.
- Native float64 WAV self-test analyzed by the Rust CLI — 256 frames aligned
  after an 8-frame offset, zero maximum PCM32 LSB error, PASS.
- Rust quality analyzer — 13 tests passed; standalone Clippy with `-D warnings`
  passed.
- PowerShell parse and cable quality acceptance — 35 checks passed.
- Documentation validation — 149 Markdown files and 858 local links passed.
- No endpoint or driver test was run: this host is not the guarded VM and has
  no `C:\ar` bundle. The PCM32 route case is wired, not measured.

## PCM rate-matrix wiring

Extended the guarded product-route cases to PCM16, PCM24-in-32 and PCM32 at
44.1, 48 and 96 kHz. Each case accepts the selected channel count (1 through 8;
the required A16 points are 1, 2 and 8), passes it to the route runner, and
checks Windows' actual negotiated sample rate and encoding before reporting a
pass. This is wiring coverage only; no VM format/rate/channel measurement ran.
The host-safe cable quality acceptance now passes 36 checks, PowerShell parsing
passes, and documentation validation remains 149 Markdown files / 858 links.

## Sequential channel matrix option

Added `-ChannelMatrix` to the guarded quality entry point for float32, integer
PCM and sweep cases. It runs the selected case at 1, 2 and 8 channels in order,
stops on the first failed child run, and preserves each child runner's separate
evidence package. Host-safe acceptance and parser checks cover the wiring; no
VM route has been started.

Updated the analyzer README with the supported PCM rate and channel matrix,
float64 PCM32 capture and remaining VM gate. Bundle preparation now rejects a
base containing stale cable quality runner files before staging the current
scripts, avoiding a mixed-version evidence bundle.

Review found that the bit-exact route runner wrote `Verdict = FAIL` but still
returned success when setup completed. It now throws after persisting the
failed result, allowing `-ChannelMatrix` to stop at the failing measurement.
Host-safe acceptance covers the failure branch's wiring; no live route ran.
