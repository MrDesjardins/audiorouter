# M02 audio adapter groundwork

## 2026-09-07 — Rust capture compatibility mode

The Windows adapter now exposes an explicit `SharedCapture::open_polling`
path. It uses the native-qualified shared-mode request (exact endpoint mix
format, `AUDCLNT_STREAMFLAGS_NOPERSIST`, and a bounded buffer duration) and
polls packet availability, while the event-driven request remains the primary
`open` behavior. This addresses the observed distinction between successful native
non-event initialization and the Rust event-callback `E_INVALIDARG` path
without silently weakening the event-driven contract. The adapter package's
15 tests and strict Clippy pass; no live stream was opened for this change.

The normal `SharedCapture::open` path now retries that polling request only
when event-callback initialization returns exactly `E_INVALIDARG`. Busy-device,
permission, and endpoint-loss errors are not retried or relabeled. The retry
uses a fresh COM client, so a failed initialization cannot leave a partially
configured client in use. This is a compatibility implementation, not live
Rust stream qualification.

## 2026-09-05 — Read-only endpoint adapter

Added `crates/windows-audio` as the first reusable Windows adapter boundary. It explicitly owns COM initialization/uninitialization, enumerates active capture and render endpoints, copies the COM-owned `WAVEFORMATEX` metadata before freeing it, and returns endpoint ID, direction, shared-mode periods, sample rate, channels, bits, and format tag. It also provides shared capture/render lifecycle wrappers with exact endpoint selection, bounded event-driven initialization, owned event handles, start/stop/reset, timeout waits, and packet/buffer operations that release device buffers immediately.

The control plane now uses this adapter for `devices.list`, returning active endpoint IDs, direction, state, format, and period metadata. The adapter also provides an identity-preserving metadata snapshot diff for added, removed, and changed endpoints; it is a polling helper and does not silently rebind a missing device. `status.get` reports device discovery as available while full audio remains unavailable because the realtime graph and routing are not implemented. `apps.list` returns bounded process identities and, on Windows, the read-only audio-session observations described below.

Endpoint topology notifications are now registered through an RAII `IMMNotificationClient` subscription. Every callback only sets an atomic dirty flag; the control plane must consume that flag and resnapshot, so callbacks never enumerate, allocate, lock, or rebind streams.

`SharedCapture::next_packet_into` now provides a bounded data-copy boundary into caller-owned storage, handles silent packets as zeroes, validates the requested bytes-per-frame and destination capacity, and releases every WASAPI packet before returning. This is an adapter primitive, not yet an end-to-end realtime graph.

`SharedRender::submit_bytes` provides the matching bounded output boundary for caller-owned interleaved bytes. It rejects partial frames, limits writes to available device capacity, and releases the WASAPI render buffer. The normal adapter tests remain non-invasive; no render stream was started.

The adapter now classifies retained Windows HRESULTs into stable `AudioFailureKind` values, including device contention versus invalid argument and exclusive-only behavior. Six adapter tests pass. This allows control diagnostics to distinguish the earlier `E_INVALIDARG` condition from `AUDCLNT_E_DEVICE_IN_USE` without changing the underlying error.

Application discovery now attempts `PROCESS_QUERY_LIMITED_INFORMATION` and records an optional process creation timestamp alongside PID and executable name. This is sufficient identity material for a PID-reuse check without exposing command lines or paths. The adapter's `bind_application` path enforces the identity tuple; controlled process-loopback tone attribution remains open.

`bind_application` now enforces that identity material before a future process-loopback activation: PID, executable name, and creation timestamp must all match, otherwise binding is rejected. The Windows identity test passes without opening an audio stream; the native loopback harness remains a separate data-path implementation.

The validator rejects missing creation-time metadata explicitly, preventing PID/name-only matches from being accepted. Seven adapter tests and strict adapter Clippy pass.

The adapter now also exposes a restart resolver for persisted executable
selectors. It refuses to rebind when no executable matches, when multiple
case-insensitive matches exist, or when the sole match lacks a creation
timestamp; only one verified candidate is returned. Portable regression
coverage exercises all four outcomes. This is identity-only evidence: no
audio stream is opened, and controlled process-tree attribution and native
restart/PID-reuse runtime evidence remain open.

PID-bound verification now compares executable names case-insensitively, while
still requiring the observed creation timestamp and PID. This keeps live
rebinding consistent with the restart resolver and avoids false identity
changes caused only by Windows name casing; no audio stream is opened.

Control serialization now exposes creation timestamps as decimal strings rather than JSON numbers, preserving the full `u64` identity across TypeScript clients. Control tests and contract typechecking pass; no audio operation is involved.

Buffer-capacity failures are now classified separately as `BufferConstraint`, while invalid frame sizes remain `InvalidArgument`. Seven adapter tests and strict adapter Clippy pass.

`EndpointMonitor` now owns an initial active-endpoint snapshot and refreshes it only after an `IMMNotificationClient` notification, returning identity-preserving diffs. Its read-only startup/poll test passes; stream rebinding and graph recovery remain separate work. Eight adapter tests and strict adapter Clippy pass.

Verification on the Windows 11 host:

```powershell
cargo test -p audiorouter-windows-audio
```

Passed 5 tests, including live active-endpoint enumeration, endpoint snapshot diffing, and unknown capture/render endpoint rejection tests. The wrapper's real capture start/packet/stop path is covered by the separately run native diagnostic; the ordinary workspace suite does not open the user's microphone. The adapter does not change defaults, volume, mute, driver state, or other persistent configuration.

This does not yet satisfy M02. Graph activation/routing, end-to-end realtime buffer transfer, latency measurement, dual-device drift validation, process-tree capture data, and failure recovery remain open. The native diagnostic separately provides the current capture and process-loopback activation/data evidence.

The process-tree capture clause above is superseded by the later native probe evidence: include and exclude modes have successful data-path reads. Remaining M02 gaps are controlled attribution, graph-to-device activation, end-to-end scheduling, latency, dual-device drift validation, failure recovery, and driver lifecycle.

The native probe now provides an opt-in controlled-attribution harness: a bounded child process renders a deterministic tone while the parent captures only that process tree and verifies child cleanup. The implementation builds with the installed Windows toolchain, but runtime execution is intentionally pending because it emits an audible test tone; this does not yet constitute attribution evidence.

The Windows application inventory now supplements process identities with a read-only `IAudioSessionManager2` snapshot across active render and capture endpoints. `applications.list` reports active/inactive session counts, separate render/capture session counts, display names, and capture sessions observed for each PID. It does not open or start an audio stream, alter endpoint state, or claim that an unobserved protected/background capture is impossible. Process-loopback attribution and capture capability beyond the observed snapshot remain open.

Session enumeration is fail-soft at the individual session and endpoint
manager boundaries: an aggregate session, inaccessible endpoint, or transient
session disappearance is skipped while the rest of the process inventory is
retained. The adapter regression still verifies nonzero process identities and
consistent session-count bounds.

The process inventory also excludes Windows' PID-0 system pseudo-process so
every returned application satisfies the API's positive-PID schema constraint.

Results are sorted case-insensitively by executable and then PID, making
read-only UI/CLI refreshes deterministic even though Toolhelp enumeration
order is not guaranteed.
The policy has both a synthetic regression and a live Windows snapshot check.

## Current adapter status (2026-09-07)

The earlier introductory read-only wording in this report is superseded by
the later production-adapter smoke evidence below. `SharedCapture` and
`SharedRender` now provide explicit bounded stream ownership, event/polling
delivery, caller-owned packet copies, and lifecycle cleanup; metadata and
application/session discovery remain read-only. The guarded adapter-to-engine
and virtual-cable runs are the current runtime evidence. Native graph-to-device
scheduling, dual-device drift, failure recovery, and measured physical latency
remain open.

## 2026-09-06 — Prepared session activation boundary

`RuntimeProcessor::activate_session` now compiles a complete session candidate
before publishing its immutable runtime graph. If validation or supported-topology
preparation fails, no publication occurs and the previously active generation
continues processing. A regression uses a valid fixture followed by an invalid
matrix and verifies that generation and output remain on the original graph.
The engine suite passes 40 tests with strict Clippy. This is a portable
publication/rollback boundary; WASAPI scheduling, device-resource activation,
and live routing remain open.

## 2026-09-06 — Cross-milestone regression

The full locked Rust workspace passed, including the 40-test engine suite and
all adapter, control, storage, recording, plugin-worker, transport, CLI/MCP,
and doc-test targets. Contracts/UI typechecks, 23 UI tests, and the production
Vite build also passed. This confirms regression health for the portable
boundaries; live graph-to-device scheduling, physical latency, and driver
lifecycle remain open.

The Windows adapter now includes the numeric HRESULT in displayed audio errors
while retaining stable failure classification. A regression confirms
`0x80070057` is reported as `InvalidArgument` and remains visible in the
diagnostic text. This improves investigation of the unresolved Rust
`IAudioClient::Initialize` discrepancy; it does not claim that discrepancy is
fixed and does not open or start an audio stream.

Shared capture and render initialization now wrap failures with the operation
name (`IAudioClient::Initialize(capture)` or `(render)`) while preserving the
underlying HRESULT and `AudioFailureKind`. The Windows-audio regression covers
the formatted invalid-argument path; this remains diagnostic hardening only,
not a fix or evidence of a started stream.

## Production Rust adapter smoke (2026-09-07)

The standalone M00 probe now has an explicit `adapter-smoke` mode that uses the
production `SharedCapture` and `SharedRender` types. On the current host it
opened the first active capture and render endpoints, started both streams,
read capture packets into a preallocated caller-owned buffer, submitted only
silent render buffers, and stopped/reset both clients. A 500 ms run collected
51 capture packets/24,480 frames/195,840 bytes and submitted 25,536 silent
render frames. This qualifies the Rust adapter's bounded stream data path and
cleanup, but not graph-to-device scheduling or audible end-to-end routing.
The final media snapshot remained ten present devices, all `OK`.

The live run is now reproducible through
`tests/acceptance/m02-rust-adapter-live.ps1 -AllowLiveAudio`. The wrapper
requires an explicit bounded duration, snapshots media-device identity/state
before and after, requires positive capture and silent-render counts, and is
not invoked by ordinary CI or non-live acceptance.

The guarded wrapper was executed with `-AllowLiveAudio -DurationMilliseconds
200` and passed: 21 capture packets/10,080 frames/80,640 bytes and 11,136
silent render frames were observed, and the before/after media-device snapshot
was identical.

The smoke path was then tightened to exercise `SharedRender::submit_bytes`
with a preallocated all-zero caller buffer. The guarded 200 ms acceptance
passed again with 21 capture packets/10,080 frames/80,640 bytes and 10,656
zero-valued render frames; the media-device identity/state snapshot remained
identical. This validates the caller-owned render-copy path without generating
an audible signal.

## Adapter-to-engine block smoke (2026-09-07)

The opt-in smoke was extended to feed the captured 32-bit samples into the
portable `RealtimeScheduler` in fixed 128-frame blocks. A guarded 200 ms live
run processed 8,064 capture frames through the scheduler while submitting
11,136 zero-valued render frames through `SharedRender::submit_bytes`; capture
and render clients stopped/reset successfully and the media-device snapshot
remained identical. This qualifies bounded adapter-to-engine ownership and
block processing, not complete graph activation, audible routing, or physical
latency.

The packet adaptation was then tightened to carry partial WASAPI packets across
boundaries in a fixed staging buffer. The latest guarded 300 ms run consumed 30
packets (14,400 capture frames), processed 14,336 complete 128-frame scheduler
frames, retained the final 64-frame remainder at bounded shutdown, and
submitted 15,936 zero-valued render frames. Counts vary with scheduling during
the bounded window; this validates packet-to-quantum carry rather than assuming
packet/quantum alignment.

The smoke now publishes a prepared generation-1 graph containing the portable
0.5x gain stage. It verifies that every returned scheduler block belongs to
that generation and contains only finite samples. A guarded 300 ms run passed
with 32 packets/15,360 capture frames, 120 graph blocks/15,360 processed
frames, no pending remainder, and 16,032 zero-valued render frames. The media
device identity/state snapshot remained identical; this qualifies graph
activation and finite-output handling at the adapter boundary, not audible
routing or physical latency.

## Rust adapter-to-render route smoke (2026-09-07)

The standalone probe now has an explicit `adapter-route` mode that selects a
capture and render endpoint by their discovered opaque endpoint IDs, requires
matching 32-bit channel/rate metadata, feeds capture packets through the
generation-1 0.5x gain graph, and submits the processed caller-owned frames to
the render client. The guarded VB-Audio cable run passed for 500 ms with
24,480 capture frames, 24,448 scheduler frames, and 24,448 routed render
frames. The media-device identity/state snapshot was unchanged.

The reproducible wrapper is
`tests/acceptance/m02-rust-adapter-route-live.ps1 -AllowLiveAudio`; it resolves
friendly names through the native inventory probe, removes its temporary probe
and generated object, and never changes defaults, volume, mute, privacy,
drivers, signing, or startup configuration. This qualifies the digital
adapter-to-render data path, not physical acoustic latency, arbitrary format
conversion, or production graph lifecycle.

The route carry queue is bounded to 64 fixed 128-frame blocks (approximately
170 ms for stereo float32); if render capacity cannot catch up, the route
fails closed rather than dropping a processed block or allocating without a
limit.

The route now supports explicit mono-to-stereo duplication and stereo-to-mono
averaging for compatible 32-bit endpoints, while retaining a fail-closed
sample-rate match requirement until resampling is connected at this boundary.
Three pure mapping regressions and strict probe Clippy pass; the existing
stereo VB-Audio route also passed again with 24,480 captured, 24,448 scheduled,
and 23,968 routed frames.

The endpoint selector has two additional pure regressions: an opaque ID must
match the requested direction, and an omitted ID may select only within the
requested direction. The standalone probe tests, strict Clippy, and locked
compile check pass.

## Route output ownership cleanup (2026-09-07)

The adapter-route probe now separates render serialization/submission errors
from scheduler-output recycling, so every received processed block is returned
to the bounded output pool even when routing fails. The compile check and a
guarded 500 ms VB-Audio route acceptance passed again with 24,480 captured,
24,448 scheduled, and 24,448 routed frames; the media snapshot and temporary
artifact cleanup remained unchanged.
