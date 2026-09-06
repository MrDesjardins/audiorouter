# M02 audio adapter groundwork

## 2026-09-05 — Read-only endpoint adapter

Added `crates/windows-audio` as the first reusable Windows adapter boundary. It explicitly owns COM initialization/uninitialization, enumerates active capture and render endpoints, copies the COM-owned `WAVEFORMATEX` metadata before freeing it, and returns endpoint ID, direction, shared-mode periods, sample rate, channels, bits, and format tag.

Verification on the Windows 11 host:

```powershell
cargo test -p audiorouter-windows-audio
```

Passed 2 tests, including live active-endpoint enumeration. The adapter does not initialize, start, or read streams and does not change defaults, volume, mute, driver state, or other persistent configuration.

This does not yet satisfy M02. Endpoint notifications, process-tree capture, event-driven stream ownership, preallocated realtime buffers, channel conversion/resampling, graph activation, routing, latency, drift, and failure recovery remain open. The native diagnostic separately provides the current capture and process-loopback activation/data evidence.
