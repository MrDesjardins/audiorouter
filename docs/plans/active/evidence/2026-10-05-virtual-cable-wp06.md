# Virtual cable WP-06 host evidence â€” 2026-10-05

This is an incremental host-only record. It does not complete WP-06 or qualify
the driver for loading. WP-05 registry-controlled names/periods and all WP-03/
04 VM gates remain pending.

## Stable bus routing slice

Added exact `cable-a` through `cable-h` parsing and a pure lease-slot index
helper mapping `(bus index, direction)` onto 16 unique slots. OPEN now resolves
the request's bus ID, rejects unknown values and returns
`STATUS_DEVICE_NOT_CONNECTED` for a cable above the configured enabled count.
WaveRT stream access uses the bus index from its stable endpoint type. Lease
shape now includes sample rate; capture emits silence and render publication
is disabled when the stream and lease rates differ.

The Windows Rust driver-control encoder now writes driver ABI version 1.1,
independently of the existing internal AudioBridge protocol 1.0, and requires a
canonical cable ID. This avoids forwarding the unrelated internal protocol
minor into the kernel request.

## Checks run

- `powershell -NoProfile -ExecutionPolicy Bypass -File drivers/audiorouter-virtual/tests/build-tests.ps1` â€” passed; 158 host checks including all cable IDs and unique directional slots.
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-driver-build.ps1 -Platform x64` â€” passed; WDK compile/package acceptance.
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-driver-build.ps1 -Platform ARM64` â€” passed; WDK compile/package acceptance.
- `cargo test -p audiorouter-windows-audio --target x86_64-pc-windows-msvc native_bridge_control_request_matches_bounded_driver_layout` â€” passed; ABI version, cable ID, and direction assertion.
- `git diff --check` â€” passed.

## Limits and next work

The Rust mapped region now uses float64 wire samples and exposes `write_f64`
and `read_into_f64`; legacy f32 session APIs widen on write and narrow on read.
The regression maps PCM32 value 1,073,741,889 to float64 and back exactly.
Query/capability negotiation, the optional OPEN extension, the 128-byte
counters header, and WP-09 float64 engine precision remain required before
integration or any VM load. Driver installation/loading, endpoint enumeration,
multi-cable audio, mismatch counters, and sound-quality measurements were not
run.

Additional checks: `cargo test -p audiorouter-windows-audio` passed (111
passed, 1 ignored at this point in the run); targeted Windows-target tests
`native_bridge_control_request_matches_bounded_driver_layout` and
`native_bridge_float64_mapping_preserves_pcm32_precision` passed. Existing
non-finite input regression was updated to require silence-filled output.
