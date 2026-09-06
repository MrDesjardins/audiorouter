# M02 audio adapter groundwork

## 2026-09-05 — Read-only endpoint adapter

Added `crates/windows-audio` as the first reusable Windows adapter boundary. It explicitly owns COM initialization/uninitialization, enumerates active capture and render endpoints, copies the COM-owned `WAVEFORMATEX` metadata before freeing it, and returns endpoint ID, direction, shared-mode periods, sample rate, channels, bits, and format tag. It also provides a shared-capture lifecycle wrapper with exact endpoint selection, bounded initialization, start/stop/reset, and packet metadata reads that release the underlying device buffer immediately.

Verification on the Windows 11 host:

```powershell
cargo test -p audiorouter-windows-audio
```

Passed 4 tests, including live active-endpoint enumeration and unknown capture/render endpoint rejection tests. The wrapper's real capture start/packet/stop path is covered by the separately run native diagnostic; the ordinary workspace suite does not open the user's microphone. The adapter does not change defaults, volume, mute, driver state, or other persistent configuration.

This does not yet satisfy M02. Endpoint notifications, event-driven stream ownership, preallocated realtime buffers, channel conversion/resampling, graph activation, routing, latency, drift, process-tree capture data, and failure recovery remain open. The native diagnostic separately provides the current capture and process-loopback activation/data evidence.
