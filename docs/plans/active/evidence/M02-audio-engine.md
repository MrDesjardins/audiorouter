# M02 audio adapter groundwork

## 2026-09-05 — Read-only endpoint adapter

Added `crates/windows-audio` as the first reusable Windows adapter boundary. It explicitly owns COM initialization/uninitialization, enumerates active capture and render endpoints, copies the COM-owned `WAVEFORMATEX` metadata before freeing it, and returns endpoint ID, direction, shared-mode periods, sample rate, channels, bits, and format tag. It also provides shared capture/render lifecycle wrappers with exact endpoint selection, bounded event-driven initialization, owned event handles, start/stop/reset, timeout waits, and packet/buffer operations that release device buffers immediately.

The control plane now uses this adapter for `devices.list`, returning active endpoint IDs, direction, state, format, and period metadata. The adapter also provides an identity-preserving metadata snapshot diff for added, removed, and changed endpoints; it is a polling helper and does not silently rebind a missing device. `status.get` reports device discovery as available while full audio remains unavailable because the realtime graph and routing are not implemented. `apps.list` now returns a bounded process snapshot with PID and executable name only.

Endpoint topology notifications are now registered through an RAII `IMMNotificationClient` subscription. Every callback only sets an atomic dirty flag; the control plane must consume that flag and resnapshot, so callbacks never enumerate, allocate, lock, or rebind streams.

`SharedCapture::next_packet_into` now provides a bounded data-copy boundary into caller-owned storage, handles silent packets as zeroes, validates the requested bytes-per-frame and destination capacity, and releases every WASAPI packet before returning. This is an adapter primitive, not yet an end-to-end realtime graph.

`SharedRender::submit_bytes` provides the matching bounded output boundary for caller-owned interleaved bytes. It rejects partial frames, limits writes to available device capacity, and releases the WASAPI render buffer. The normal adapter tests remain non-invasive; no render stream was started.

The adapter now classifies retained Windows HRESULTs into stable `AudioFailureKind` values, including device contention versus invalid argument and exclusive-only behavior. Six adapter tests pass. This allows control diagnostics to distinguish the earlier `E_INVALIDARG` condition from `AUDCLNT_E_DEVICE_IN_USE` without changing the underlying error.

Application discovery now attempts `PROCESS_QUERY_LIMITED_INFORMATION` and records an optional process creation timestamp alongside PID and executable name. This is sufficient identity material for a future PID-reuse check without exposing command lines or paths. The adapter and contracts checks pass; process-loopback binding has not yet been wired to enforce this identity.

`bind_application` now enforces that identity material before a future process-loopback activation: PID, executable name, and creation timestamp must all match, otherwise binding is rejected. The Windows identity test passes without opening an audio stream; the native loopback harness remains a separate data-path implementation.

Buffer-capacity failures are now classified separately as `BufferConstraint`, while invalid frame sizes remain `InvalidArgument`. Seven adapter tests and strict adapter Clippy pass.

Verification on the Windows 11 host:

```powershell
cargo test -p audiorouter-windows-audio
```

Passed 5 tests, including live active-endpoint enumeration, endpoint snapshot diffing, and unknown capture/render endpoint rejection tests. The wrapper's real capture start/packet/stop path is covered by the separately run native diagnostic; the ordinary workspace suite does not open the user's microphone. The adapter does not change defaults, volume, mute, driver state, or other persistent configuration.

This does not yet satisfy M02. Graph activation/routing, end-to-end realtime buffer transfer, latency measurement, dual-device drift validation, process-tree capture data, and failure recovery remain open. The native diagnostic separately provides the current capture and process-loopback activation/data evidence.
