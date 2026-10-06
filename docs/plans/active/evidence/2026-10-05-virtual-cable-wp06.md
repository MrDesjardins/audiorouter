# Virtual cable WP-06 host evidence — 2026-10-05

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

- `powershell -NoProfile -ExecutionPolicy Bypass -File drivers/audiorouter-virtual/tests/build-tests.ps1` — passed; 158 host checks including all cable IDs and unique directional slots.
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-driver-build.ps1 -Platform x64` — passed; WDK compile/package acceptance.
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-driver-build.ps1 -Platform ARM64` — passed; WDK compile/package acceptance.
- `cargo test -p audiorouter-windows-audio --target x86_64-pc-windows-msvc native_bridge_control_request_matches_bounded_driver_layout` — passed; ABI version, cable ID, and direction assertion.
- `git diff --check` — passed.

## Limits and next work

The Rust mapped session still uses the existing float32 internal region. The
driver expects float64 payloads, so this control encoder is not yet an
operational end-to-end client. Query/capability negotiation, the optional OPEN
extension, the 128-byte counters header, float64 session APIs and WP-09 engine
precision are still required before integration or any VM load. Driver
installation/loading, endpoint enumeration, multi-cable audio, mismatch
counters, and sound-quality measurements were not run.
