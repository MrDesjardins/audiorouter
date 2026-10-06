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

## Protocol 1.1 negotiation, QUERY and stream counters (2026-10-05)

Host-only slice. No driver was installed or loaded; nothing here is VM,
endpoint, audio or sound-quality evidence.

Implemented:

- Driver ABI (`Source/Inc/bridgeio.h`): 128-byte shared header
  (`AR_BRIDGE_SHARED_HEADER`: counters at 32, `SampleBytes` at 88,
  `ReaderSequence` at 96, payload at 128); 64-byte `AR_BRIDGE_OPEN_EXTENSION`
  with a required `FLOAT64` flag (unknown flags, reserved words and a
  prefix-only OPEN return `STATUS_NOT_SUPPORTED`); exact-length validators;
  `IOCTL_AUDIOROUTER_BRIDGE_QUERY` returning the 104-byte
  `AR_BRIDGE_DRIVER_INFO` with only implemented capabilities (multichannel,
  44.1/48/96 kHz, stream counters, float64). Low-latency periods and registry
  configuration are not reported because they are not implemented.
- Driver callbacks: underrun, overrun (via the consumer's `ReaderSequence`),
  sequence-gap, non-finite and format-mismatch counters plus the last
  position/QPC pair, updated through one rundown-protected, lock-free helper
  per callback. OPEN resets bytes 32–127 and writes `SampleBytes = 8` after the
  ownership checks and before the view becomes visible to callbacks.
- `build.ps1 -Version` passes the four-part version to MSBuild so QUERY
  reports the same version StampInf writes into the INF.
- Rust client: exact `METHOD_BUFFERED` IOCTL codes, the 240-byte extended
  request for every control IOCTL, `NativeBridgeControlClient::query()`,
  `NativeBridgeDriverInfo::check_compatible()` (refuses a driver without
  float64; no silent float32 fallback), `NativeBridgeRegion::counters()`,
  `SampleSizeMismatch` before payload access, consumer acknowledgement after
  each read, and `classify_native_bridge_error` with distinct variants and user
  messages for version mismatch, lease held, other-session access, cable not
  enabled, lease lost and driver unavailable.

Defects found and fixed in this slice (both would have failed in the VM):

1. The Rust IOCTL constants used transfer method 3 (`METHOD_NEITHER`,
   `0x0022E003`…) while the driver dispatches on `METHOD_BUFFERED`
   (`0x0022E000`…), so every OPEN/HEARTBEAT/CLOSE from the app would have
   returned `STATUS_INVALID_DEVICE_REQUEST`. Both sides now pin the literal
   codes in tests (`C_ASSERT` in bridgeio.h,
   `native_bridge_protocol_1_1_abi_matches_driver_header`).
2. The capture endpoint (`WriteBytes`) reset its read sequence when the
   capture-sink lease rate differed from the stream rate, but still copied
   the lease's blocks, which would have played audio at the wrong speed. The
   copy now requires a usable lease (same rate and channel count); otherwise
   it outputs silence and counts `FormatMismatches`. A static acceptance
   guard enforces this.

The NTSTATUS→Win32 table used by the Rust classifier was confirmed on this
host with ntdll's `RtlNtStatusToDosError` (REVISION_MISMATCH→1306,
NOT_SUPPORTED→50, SHARING_VIOLATION→32, ACCESS_DENIED→5,
DEVICE_NOT_CONNECTED→1167, INVALID_DEVICE_STATE→22,
OBJECT_NAME_NOT_FOUND→2); the regression test calls that routine directly.

Checks run (Windows 11 host, 10.0.26300; WDK tools 10.0.28000.0):

- `powershell -NoProfile -ExecutionPolicy Bypass -File drivers/audiorouter-virtual/tests/build-tests.ps1` — passed, 206 checks (was 158); host copy 0.124 µs per 128×2 float64 block (not kernel DPC evidence).
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-driver-build.ps1 -Platform x64` — passed (log `target/driver-wp06-x64.log`).
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/acceptance/m03-driver-build.ps1 -Platform ARM64` — passed (log `target/driver-wp06-arm64.log`).
- `cargo test -p audiorouter-windows-audio` — 116 passed, 0 failed, 1 ignored (live test) (log `target/wp06-rust-tests.log`).
- `cargo check --workspace --all-targets` — passed; the two `unused_mut` warnings it shows were there before this change.

Remaining WP-06 work: 8-channel leases in the Rust client (the region and
`AudioBridgeHello` still use the internal protocol's 2-channel bound),
the `tools/m03-bridge-tone` VM tool, and every VM acceptance item (both
directions on two cables, bit-exact float32→float64→float32, crosstalk,
counters zero in a clean 10-minute run and rising under a deliberate stall,
8-channel and 96 kHz leases, version-mismatch and unknown-flag paths against
the loaded driver).
