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
averaging for compatible 32-bit endpoints, plus fixed-quantum linear
sample-rate conversion into preallocated graph blocks. Three pure mapping
regressions and strict probe Clippy pass; the existing stereo VB-Audio route
also passed again with 24,480 captured, 24,448 scheduled, and 23,904 routed
frames. Cross-block clock-drift correction and hardware synchronization remain
separate open gates.

The route uses the existing preallocated linear resampler when the selected
capture and render rates differ, preserving the fixed 128-frame graph quantum.
It does not yet maintain a cross-block fractional phase or feed FIFO occupancy
back into `DriftController`; those remain native scheduler work.

The endpoint selector has two additional pure regressions: an opaque ID must
match the requested direction, and an omitted ID may select only within the
requested direction. The standalone probe tests, strict Clippy, and locked
compile check pass.

## Fail-closed endpoint binding resolution (2026-09-07)

The Windows-audio adapter now exposes `resolve_endpoint_binding` for a fresh
read-only endpoint snapshot. It returns an endpoint only when the persisted
opaque ID and expected direction both match; a missing ID or direction change
is returned explicitly, with no friendly-name or enumeration-order fallback.
The regression passes in the 18-test Windows-audio suite with strict Clippy
and doc-tests. This is a recovery decision boundary only; native stream
re-opening, hardware removal/reconnect, and graph rebind remain open.

## Route output ownership cleanup (2026-09-07)

The adapter-route probe now separates render serialization/submission errors
from scheduler-output recycling, so every received processed block is returned
to the bounded output pool even when routing fails. The compile check and a
guarded 500 ms VB-Audio route acceptance passed again with 24,480 captured,
24,448 scheduled, and 24,448 routed frames; the media snapshot and temporary
artifact cleanup remained unchanged.

## Current-head one-second adapter-route requalification (2026-09-07)

The authorized Rust adapter-route wrapper was rerun for 1,000 ms using the
explicitly selected VB-Audio render and capture endpoint IDs. Capture delivered
48,960 frames; the fixed-quantum scheduler processed 48,896 frames; and
48,384 frames were routed through the generation-1 graph. The endpoint/media
snapshot remained unchanged and temporary native outputs were removed. This
extends the adapter data-path evidence under a longer bounded window; it does
not qualify cross-device clock synchronization, physical latency, driver
integration, or production graph lifecycle.

## Format-aware endpoint binding resolution (2026-09-07)

The fail-closed endpoint resolver now also compares the persisted mix format
(sample rate, channels, bits per sample, and format tag). A changed format
returns `FormatChanged` with the expected and observed metadata instead of
silently accepting a stale graph binding. The regression passes in the
19-test Windows-audio suite with strict Clippy and doc-tests. This remains a
read-only decision boundary; deliberate native renegotiation and stream
recovery are still open.

The resolver is now exposed on `EndpointMonitor` as a snapshot-only method,
making the fail-closed decision available immediately after notification-driven
refresh without coupling recovery code to enumeration or stream activation.

Persisted endpoint format identity also includes the extensible channel mask and
subformat GUID. A changed mask or subformat now returns `FormatChanged`, just
like a changed rate, channel count, bit depth, or format tag; recovery must
explicitly renegotiate rather than reopening a materially different stream.

Recovery can call `EndpointMonitor::refresh_changes` to force a fresh endpoint
enumeration even when no dirty notification is visible. The method clears the
coalesced notification flag, returns the same deterministic diff, and remains
metadata-only; stream reopen, renegotiation, and replacement selection remain
deliberate caller operations.

The stream clients now also expose `open_refreshed_bound`, which refreshes the
monitor snapshot immediately before applying the exact ID/direction/format
guard. The adapter route uses this entry point for both streams, reducing the
window in which a pending coalesced notification could leave validation stale;
an endpoint change after validation is still reported by WASAPI.

The stream clients now expose `open_bound` entry points that enforce this
decision immediately before activation. Missing, direction-changed, or format-
changed bindings return a structured `AudioError::EndpointBinding` without
activating WASAPI; a topology race after validation remains surfaced by the
underlying WASAPI HRESULT. The new error payload is boxed so ordinary audio
errors remain small, and native stream recovery still requires the caller to
refresh and deliberately retry or renegotiate.

The adapter-route probe now creates an `EndpointMonitor` from the selected
inventory and opens both streams through `SharedCapture::open_bound` and
`SharedRender::open_bound`. Packet strides come from the validated endpoint
metadata helper, so the live route cannot bypass the binding or frame-shape
checks. The monitor validation remains a control-plane precondition; topology
races after validation are still surfaced by WASAPI errors.

The one-second authorized route also asserted scheduler telemetry before
stopping: processed quanta matched graph blocks, the active generation remained
current, and input/output overruns plus XRuns were zero. This is bounded route
health evidence, not proof of long-run native callback timing.

The adapter route now requires `EndpointInfo::is_ieee_float32` for both selected
streams. This accepts legacy IEEE-float tags and extensible float subformats,
but rejects 32-bit integer PCM before any packet bytes are decoded as floats,
closing a format-confusion data-path risk.

`AudioError::binding_resolution` exposes the structured fail-closed decision to
recovery callers without requiring string parsing. This preserves explicit
handling for endpoint disappearance, direction changes, and deliberate format
renegotiation while retaining the stable HRESULT/error-kind classification.

Each stream client now exposes `replace_with_refreshed_bound`. This explicit
recovery operation stops/resets and drops the old client before forcing a fresh
metadata snapshot and reopening the exact binding. If refresh or validation
fails, no replacement stream is opened; callers must deliberately renegotiate
or choose another endpoint. This closes the portable recovery orchestration
boundary while native device-invalidation fault injection remains open.

The adapter now exposes a ratio-controlled resampler boundary and feeds the
render-side bounded pending-frame occupancy into `DriftController` whenever
capture and render rates differ. This makes the correction loop explicit and
bounded at the route boundary, while cross-block fractional phase, sufficient
FIFO depth for arbitrary rate ratios, and hardware clock qualification remain
native scheduler work.

The route now uses the preallocated `StreamingResampler` FIFO for those
different-rate streams. Fractional phase and source samples survive packet and
quantum boundaries, while a short production result remains an explicit
bounded underflow rather than repeating the last sample. Capacity exhaustion
is surfaced as an error; arbitrary-rate stress, native callback timing, and
hardware clock qualification remain open.

The streaming boundary now treats an incomplete destination quantum
transactionally: it silences the caller-owned output and leaves source FIFO
ownership plus fractional phase untouched. This prevents an adapter route from
dropping the consumed prefix of a partial block; the focused engine and probe
checks pass, while arbitrary-rate stress and native timing remain open.

The authorized one-second adapter-route acceptance was rerun after this fix.
The selected VB-Audio pair delivered 48,960 capture frames, the scheduler
processed 48,896 frames, and 47,968 frames were routed through the generation-
1 graph. Endpoint/media identity and state remained unchanged and temporary
outputs were removed. This requalifies the existing-rate live path; differing
hardware-rate stress and native callback timing remain open.

The route now waits on the production render client's event before draining
the bounded carry queue. This connects the native event-driven render boundary
to route submission and avoids treating an unavailable render period as a
successful write. Compile/tests and strict Clippy pass; the follow-up live run
was blocked before execution by the host's Application Control policy (OS
error 4551), so no new runtime route claim is made.

The differing-rate route now targets 512 queued source frames, the midpoint of
its fixed 1,024-frame resampler FIFO. This avoids the previous unreachable
8,192-frame target, which forced the integral/proportional controller to remain
at a correction bound. A centered-target regression and focused engine/probe
checks pass; hardware clock behavior remains unqualified.

Adapter-route output now reports the resampler queue depth and applied drift
correction, and rejects a run if queue occupancy exceeds 1,024 frames or the
correction exceeds the configured ±100 ppm bound. This makes the bounded
feedback state observable to acceptance tooling; probe tests and strict Clippy
pass, while live differing-rate execution remains blocked by host policy.

## Differing-rate candidate and host launch boundary (2026-09-07)

The read-only native format inventory found an available 48 kHz CABLE capture
and 96 kHz SteelSeries Sonar Aux render pair, both using float32 mix formats.
The guarded differing-rate route was then attempted with those exact friendly
names. Its temporary native build completed, but Windows Application Control
blocked launching the freshly generated executable before endpoint inventory;
therefore no stream opened and no differing-rate runtime result is claimed.
The route wrapper now uses a per-run temporary object path and cleans it with
the executable, so this host-policy failure cannot strand `main.obj` in the
repository.

## Repeatable differing-rate route (2026-09-07)

The route acceptance now accepts both exact endpoint IDs as an alternative to
friendly-name resolution. This preserves fail-closed opaque binding while
allowing a run to bypass the host's blocked temporary native inventory launch.
Using the 96 kHz mono Digital Audio Interface capture ID and 48 kHz stereo
CABLE render ID, the authorized 500 ms wrapper run passed with 48,000 capture
frames, 23,936 scheduler frames, and 23,936 routed frames. The output reported
zero XRuns and input/output overruns, 133 queued resampler frames, and
correction within the configured ±100 ppm bound. Media identity/state and
temporary-artifact cleanup checks passed; no persistent audio configuration
changed.

## 2026-09-07 — Current-head Rust adapter route

The authorized guarded route was replayed after the event-gated render and
bounded streaming-resampler changes using the existing VB-Audio endpoints.
The route reported 24,000 capture frames, 23,936 scheduler frames, and 23,456
routed frames through the generation-1 graph. Endpoint/media identity and state
were unchanged, and the temporary executable was removed. This is real
existing-rate route evidence; it does not qualify differing-rate hardware
clock behavior, native invalidation recovery, or physical acoustic latency.

The adapter now also exposes bounded `open_refreshed_bound_with_retry` helpers
for capture and render. They run only on the recovery/control thread, force a
fresh snapshot before every exact-ID/direction/format validation, retry only
`DeviceInUse`, `DeviceInvalidated`, or `ServiceUnavailable`, and cap attempts
at five with a one-second maximum delay. They never select a substitute or
sleep on the realtime path. The focused Windows-audio suite passes 22 tests
with formatting and strict Clippy; native fault-injection evidence remains a
separate host/device gate.

The recovery boundary now also provides
`replace_with_refreshed_bound_with_retry` for both stream directions. It stops
and releases the failed client before invoking the bounded exact-binding retry
policy, so no old client overlaps a replacement and no substitute endpoint can
be selected. The focused Windows-audio suite remains green at 22 tests with
doc-tests, formatting, and strict Clippy; actual device-invalidation fault
injection is still unqualified.

The shared bounded retry policy is now directly regression-tested: transient
device invalidation retries until success, invalid argument fails immediately
without retry, and an always-transient failure stops at five attempts. This
tests policy behavior without pretending to be native fault injection; the
focused Windows-audio suite passes 25 tests with doc-tests, formatting, and
strict Clippy.

The full locked workspace was then requalified at this checkpoint: 393
unit/integration tests, all doc-tests, and strict all-target/all-feature
Clippy with `-D warnings` passed. This validates dependent control and
transport compilation against the recovery API; it does not replace native
device-invalidation fault injection or production supervisor evidence.

## Current-head guarded live requalification (2026-09-07)

The authorized guarded wrappers passed at the current head. The native bounded
capture/silent-render lifecycle discovered 13 capture and 21 render endpoints
and completed its 200 ms run with one occupied render endpoint classified. The
VB-Audio digital loopback passed with nonzero capture payload, and the explicit
Rust adapter route passed with 48,000 capture frames and 23,936 scheduler and
routed frames. Temporary binaries were removed and media identity/state checks
were unchanged. This is digital/adapter evidence only; calibrated physical
latency, device-invalidation fault injection, managed driver, signing, and
production shell gates remain open.

## Current-tip adapter route requalification (2026-09-07)

The guarded Rust adapter-route acceptance was rerun against the existing
friendly-name VB-Audio endpoints. The bounded run captured 24,000 frames and
processed 23,936 scheduler frames into 23,936 routed frames. It passed exact
endpoint binding, cleanup, and before/after media-state checks. A separate
read-only format inventory passed for all 34 active endpoints. This is
adapter-path evidence only; native scheduler ownership, hardware clock drift,
and physical acoustic latency remain open. No defaults, volume, mute, privacy,
driver, signing, startup, or other machine audio configuration changed.
## Rust adapter route requalification (2026-09-07)

The guarded `m02-rust-adapter-route-live.ps1 -AllowLiveAudio -DurationMilliseconds
500` wrapper passed over the existing VB-Audio Virtual Cable endpoints. The
route processed 24,480 capture frames and 24,448 scheduler/routed frames, then
completed lifecycle cleanup and verified unchanged media-device identity/state.
Defaults, volume, mute, privacy, drivers, signing, startup, and other machine
audio configuration were not changed. This is digital adapter/engine evidence;
physical acoustic latency, production-driver integration, and hardware clock
qualification remain open.

An authorized differing-rate attempt used the read-only inventory's 96 kHz
render endpoint and 48 kHz capture endpoint. The render endpoint is 8-channel,
so the adapter correctly failed closed with `InvalidFrameSize` before stream
activation; exact cleanup and media-state checks passed. No valid differing-rate
mono/stereo pair is currently available, leaving hardware clock/resampler
qualification open.

The legacy array response of `devices.list` now advertises the 500-endpoint
bound and returns a pagination-required error if more endpoints are present,
instead of silently truncating a read-only native inventory. This check does
not open an audio stream or change endpoint state.
## Production Rust adapter live run (2026-09-07)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m02-rust-adapter-live.ps1 -AllowLiveAudio
-DurationMilliseconds 250`.

The selected existing endpoints completed a bounded production Rust adapter run
with 26 capture packets, 12,480 capture frames, 97 generation-1 graph blocks,
13,536 render frames, zero scheduler XRuns, and clean stream stop/reset. The
run used zero-valued caller-owned render buffers, so `routed_frames=0` and
`route=false` are expected; this proves adapter lifecycle and processing only.
It does not close endpoint-specific initialization failures, routed signal, or
calibrated physical-latency gates. Media-device state and persistent audio
configuration were unchanged.
## Production Rust adapter route run (2026-09-07)

Command: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m02-rust-adapter-route-live.ps1 -AllowLiveAudio
-DurationMilliseconds 100`.

The selected existing VB-Audio virtual cable completed the bounded route run
with 4,800 capture frames, 4,736 scheduler frames, and 4,736 routed frames.
The adapter stopped and reset its streams, and the acceptance harness reported
defaults, volume, mute, privacy, drivers, signing, and startup configuration
unchanged. This proves selected-endpoint route activation only; calibrated
physical latency and production-driver gates remain open.

## Rust process-loopback adapter (2026-09-07)

The Windows adapter now provides bounded asynchronous process-loopback capture
with explicit include and exclude target-tree selection. Focused Windows tests
(27), strict workspace Clippy, and the guarded live acceptance passed. Both
modes completed 250 ms runs with 24 packets and 10,584 frames, and the harness
verified stream stop/reset plus unchanged media-device identity/state. The
implementation uses caller-owned packet storage and the supported 44.1 kHz
stereo PCM/event-callback initialization shape; physical latency and full
cross-process isolation thresholds remain separate gates.

## PCM16 planar boundary (2026-09-07)

`AudioBlock` now exposes allocation-free bridges for one complete interleaved
PCM16 block. Decode maps signed 16-bit samples to planar float32 using
`-32768 -> -1.0` and `32767 -> 32767/32768`; encode silences non-finite values,
clamps finite values to `[-1, 1]`, and emits the bounded PCM16 range. Both
methods reject any source or destination whose shape is not exactly the
preallocated block shape.

The focused engine suite passed 66 tests, including round-trip boundary,
non-finite, clamp, and shape regressions. Strict engine Clippy, formatting, and
diff checks passed. This is a portable adapter-boundary result; packet
accumulation/splitting, native realtime scheduling, physical latency, and
production-driver integration remain open.

## Live PCM16 packet-boundary adapter (2026-09-07)

The guarded `m00-rust-process-live.ps1 -AllowLiveAudio` acceptance fed actual
Rust process-loopback packets through the fixed PCM16 quantum adapter. Include
and exclude modes each delivered 10,584 PCM16 frames in 24 packets and emitted
82 complete 128-frame engine quanta. Both modes stopped/reset successfully and
the media-device identity/state snapshot was unchanged. No persistent audio
configuration changed.

This is packet-boundary and format-conversion evidence only. The adapter is not
yet connected to the realtime scheduler rings; physical latency,
generation-aware live routing, and production-driver integration remain open.

## Live scheduler-ring integration (2026-09-07)

The guarded `m00-rust-process-live.ps1 -AllowLiveAudio` acceptance now feeds
the fixed PCM16 quantum adapter into a bounded `RealtimeScheduler`. Include and
exclude modes each converted 10,584 frames across 24 packets into 82 exact
128-frame blocks, processed every block as generation 1, and recycled the
matching output. Both modes stopped/reset successfully and the media-device
identity/state snapshot was unchanged. No persistent audio configuration
changed.

This proves process-loopback packet conversion and scheduler-ring ownership for
the tested path. Deterministic overflow/underrun stress, native output routing,
physical latency, and production-driver integration remain open.

## Scheduler pressure regression (2026-09-07)

The engine regression suite now exercises a one-block `RealtimeScheduler` under
both input and output pressure. A second prepared input is rejected immediately
and increments `input_overruns`; when the only output block is deliberately
held, the next processing step fails closed, recycles the input, and increments
the scheduler xrun counter without waiting.

The focused engine suite passed 68 tests, with strict engine Clippy, formatting,
and diff checks passing. This proves portable bounded ownership behavior;
native scheduler timing, physical latency, and production-driver integration
remain open.

## Fixed PCM16 quantum adapter (2026-09-07)

`Pcm16QuantumAdapter` stages split interleaved PCM16 packets in a fixed
one-quantum buffer. It accepts complete frames only, returns the number of
frames consumed, stops accepting input once 128 frames are ready, and exposes
`pop_into` to decode the complete block into caller-owned planar engine
storage. It rejects malformed channel shapes and destination shapes without
discarding pending audio. No allocation occurs after construction and the
buffer cannot grow beyond one mono/stereo quantum.

The focused engine suite passed 67 tests, including split-packet accumulation,
backpressure, exact 128-frame output, and shape regressions. Strict engine
Clippy, formatting, and diff checks passed. Native packet-boundary integration,
realtime scheduling, physical latency, and production-driver integration remain
open.

## Cross-crate verification after scheduler changes (2026-09-07)

Command: `cargo test --workspace --locked` at pushed head `2ec7701`.

All workspace unit tests and doc-tests passed, including engine (68),
Windows-audio (27), control (86), CLI (25), plugin-host and worker, recording,
storage, transport, domain, DSP, protocol, and their documented test targets.
This is portable and Windows host test evidence; it does not close native
realtime timing, physical latency, driver, signing, installer, or hardware
acceptance gates.

## Generation replacement and live recheck (2026-09-07)

`RealtimeScheduler::publish` now recycles queued output blocks at the control
publication boundary before exposing the replacement generation. A concurrent
old callback may still submit an old block, so generation-filtered receive
remains the final protection; the new regression verifies queued ownership is
restored and the next block is processed under the replacement generation.

The engine suite passed 69 tests, strict Clippy, formatting, and diff checks
passed, and the guarded process-loopback acceptance re-ran successfully in both
include and exclude modes (10,584 frames, 82 generation-1 quanta each). No
persistent audio configuration changed. Native callback timing and physical
latency remain unqualified.

## Event-driven process-loopback wakeup (2026-09-07)

`ProcessLoopbackCapture::wait_for_data` now waits on the WASAPI event handle
with a caller-bounded timeout and structured failure result. The guarded live
probe uses this wait before draining packets rather than a polling sleep.
Include and exclude modes each delivered 11,025 frames across 25 packets and
processed 86 generation-1 quanta; stop/reset and media-device snapshot checks
passed, with no persistent audio configuration changed.

This qualifies event delivery and bounded wakeup behavior on the tested host,
not callback deadline, clock-drift, physical-latency, or production-driver
compliance.

## Process-loopback delivery telemetry (2026-09-07)

The Windows adapter now exposes a nonblocking `ProcessLoopbackTelemetry`
snapshot backed by saturating atomics. It records wait calls, wait timeouts,
successful packets and frames, minimum/maximum packet-frame counts, and silent
packets. Counters update only after successful buffer release or bounded event
wait outcomes; no logging or allocation is introduced into packet delivery.

The guarded live acceptance measured, for both include and exclude modes, 25
packets and 11,025 frames, with 441 minimum and maximum packet frames. Include
recorded 39 waits/14 timeouts; exclude recorded 40 waits/15 timeouts; both had
zero silent packets. Media state and persistent audio configuration were
unchanged. These are host observations, not callback deadline or physical
latency evidence.

## Windows-audio identity and adapter regression suite (2026-09-08)

The complete `audiorouter-windows-audio` test target passed 29 tests plus
doc-tests. Coverage includes exact endpoint binding and format validation,
snapshot diffs, bounded transient recovery, process-loopback input bounds and
telemetry, read-only application/session inventory, creation-time identity
matching, and the restart helper regression. This is read-only adapter and
portable identity evidence; managed-driver callback timing, physical latency,
and production endpoint ownership remain open.

## Bounded process-loopback packet-period policy (2026-09-07)

The process-loopback adapter now enforces the explicit
`MAX_PROCESS_LOOPBACK_PACKET_FRAMES = 4,096` bound after `GetBuffer` and before
copying. A zero or oversized packet is released and rejected, so an endpoint
period cannot expand staging beyond the fixed quantum contract. The policy
regression covers the lower and upper accepted bounds and both rejection cases.

Windows-audio tests (28), strict Clippy, formatting, and tool compilation
passed. Guarded live include/exclude runs observed 25/24 packets, 11,025/10,584
frames, and 441-frame minimum/maximum packets; both completed 86/82 scheduler
quanta with unchanged media state and no persistent audio configuration
changes. These observations do not establish realtime deadline or physical
latency compliance.

## Rejected-period telemetry (2026-09-07)

`ProcessLoopbackTelemetry` now includes a saturating `rejected_packets` counter.
The process-loopback adapter increments it only after releasing a packet that
violates the zero/4,096-frame admission policy, so rejected ownership cannot
remain held while diagnostics are recorded.

The guarded live acceptance reported zero rejected packets in both modes:
include delivered 10,584 frames in 24 packets and 82 quanta; exclude delivered
11,025 frames in 25 packets and 86 quanta. Both observed 441-frame packets and
unchanged media state/configuration. This remains host telemetry, not deadline
or physical-latency evidence.

## Telemetry invariants (2026-09-07)

Windows-audio regression coverage verifies that `ProcessLoopbackTelemetry` has
an all-zero initial snapshot and that its saturating atomic increment helper
does not wrap a counter at `u64::MAX`. The Windows-audio suite passed 29 tests,
with strict Clippy, formatting, and diff checks passing. No audio stream or
machine configuration was accessed by these tests.

## Synthetic packet-period boundary matrix (2026-09-07)

The engine regression suite now covers a 127-frame packet followed by one
frame, an exact 128-frame packet, and a maximum 4,096-frame packet drained in
128-frame steps. The adapter emits complete quanta while retaining at most one
quantum of staging; no source growth or blocking is possible. The engine suite
passed 70 tests with strict Clippy, formatting, and diff checks.

This is deterministic portable period-policy evidence only. No hardware,
native stream, or machine audio configuration was accessed or changed.

## Shared packet-admission policy (2026-09-07)

The engine-side `Pcm16QuantumAdapter::push_packet` now enforces the same
non-empty, 4,096-frame maximum period bound as the Windows process-loopback
adapter before copying into fixed staging. The existing chunk method remains
available only for already validated packet pieces. Regression coverage checks
empty, malformed, and over-bound inputs while preserving pending ownership.

Engine tests (69), Windows-audio tests (28), strict workspace Clippy, formatting,
and tool compilation passed. Guarded live include/exclude runs each observed
441-frame packets and emitted 82 generation-1 scheduler quanta; stop/reset and
media-state checks passed with no persistent audio configuration change.

## Process-loopback rate-domain bridge (2026-09-08)

The live process-loopback probe now makes the source/engine clock conversion
explicit: caller-owned 44,100 Hz PCM16 stereo packets are accumulated into
128-frame source blocks, passed through the bounded phase-preserving
`StreamingResampler` at `44,100/48,000`, and drained into zero-or-more fixed
128-frame 48 kHz scheduler quanta. The FIFO is allowed to emit an occasional
second engine quantum after enough source data arrives; this prevents a false
one-quantum-per-source-block assumption from filling the bounded queue.

Focused engine tests (70), probe compilation, and formatting passed. The
guarded 300 ms include/exclude acceptance passed: each mode captured 12,789
source frames and emitted 13,696 engine frames across 107 generation-1 quanta;
packets were 441 frames, rejected packets were zero, and the final resampler
queue was 89 frames. Media-device identity/state snapshots were unchanged.
This proves the adapter's explicit rate-domain bridge and lifecycle only; it
does not claim drift correction against an independent render clock or
physical/native production-driver latency.

## Differing-rate native route qualification (2026-09-08)

The guarded, endpoint-ID-selected route wrapper passed against the current
96 kHz mono capture endpoint and 48 kHz stereo render endpoint. The Rust
adapter processed 28,800 capture frames into 14,336 scheduler/routed frames,
with generation 1 active and zero scheduler xruns or overruns. The bounded
streaming resampler and drift controller were exercised; correction reached
the declared -100 ppm bound during this short run. The wrapper verified
media-device identity/state equality and the documented unchanged defaults,
volume, mute, privacy, driver/signing, and startup state.

This is endpoint-specific shared-mode adapter evidence only. The correction
bound and short duration do not establish long-term independent-clock lock,
physical latency, managed virtual-driver lifecycle, or Discord/OBS behavior.

## Maximum-duration differing-rate stability (2026-09-08)

The guarded endpoint-ID-selected route wrapper passed its maximum 2,000 ms
duration using the same 96 kHz mono capture and 48 kHz stereo render pair:
191,040 capture frames, 95,488 scheduler frames, and 95,488 routed frames.
The detailed run observed 192,000 capture frames and 96,000 scheduler quanta,
20 queued resampler frames, -100 ppm bounded correction, and zero scheduler
xruns/overruns. The wrapper confirmed stream cleanup and unchanged endpoint
identity/default/volume/mute/privacy/driver/signing/startup state.

This strengthens bounded stability evidence for the adapter's differing-rate
path, but remains short-duration shared-mode testing and does not establish
independent-clock lock, physical latency, or production-driver behavior.

## Scheduler telemetry drain accounting (2026-09-08)

The scheduler output-ring control-boundary drain now uses a non-counting raw
pop. Intentional draining to an empty queue no longer increments the consumer
underrun metric; actual empty consumer reads retain the underrun counter. A
regression test covers generation replacement and asserts zero false output
underruns.

Engine tests (70), probe compilation, formatting, and the guarded 300 ms
include/exclude live acceptance passed. Include emitted 107 and exclude 112
generation-1 quanta; both modes reported zero scheduler xruns, input/output
overruns, and input/output underruns. Media-device snapshots were unchanged.

## Workspace regression after scheduler accounting (2026-09-08)

The locked full workspace suite passed after the queue-drain correction:
control 86, domain 53, DSP 27, engine 70, plugin-host 39 plus 8 worker-process
tests, protocol 6, recording 30, storage 40, transport 17, and Windows audio
29, with all doc tests passing. This is portable/native API regression
coverage; it does not close managed-driver, signing, installer, physical
latency, or manual UI gates.

## Native scheduler adapter and scoped route qualification (2026-09-08)

The existing native `adapter_smoke` acceptance passed for 300 ms using the
selected shared capture/render endpoints: 32 capture packets, 15,360 capture
and scheduler frames, 120 generation-1 graph quanta, zero scheduler xruns or
overruns, and zero queued resampler frames. Streams stopped/reset and the
media-device identity/state snapshot was unchanged.

The explicitly selected existing VB-Audio virtual-cable route acceptance also
passed for 300 ms: 13,920 capture frames, 13,824 scheduler frames, and 13,824
routed frames. Defaults, volume, mute, privacy, drivers, signing, startup
configuration, and media-device identity were unchanged. This is bounded
shared-mode adapter/route evidence only; it does not prove managed AudioRouter
driver lifecycle, Discord/OBS compatibility, physical latency, or callback
deadline compliance.

## Histogram-content validation (2026-09-08)

Both guarded adapter acceptance wrappers now validate the raw processing-time
histogram structurally: exactly 32 entries, sequential labels from bucket 0
through 31, numeric nonnegative counts, and a count sum equal to the reported
sample total. The adapter and routed 300 ms checks passed; the routed run
reported 108 complete samples and unchanged media state. This remains
adapter/event-loop evidence, not native callback deadline compliance.

## Runtime processing-time instrumentation (2026-09-08)

`CallbackMetrics` now records saturating total and maximum monotonic processing
duration in nanoseconds at the `RuntimeProcessor` boundary. `SchedulerTelemetry`
exposes both values for off-thread diagnostics. The update uses atomics only:
it allocates no memory, takes no locks, logs nothing, and performs no I/O.
Processing without an active graph is timed as well, so a future native callback
can account for the complete runtime boundary rather than only successful graph
blocks.

The engine suite (70 tests), locked workspace tests/doc-tests, strict Clippy,
formatting, and diff checks passed. This is instrumentation readiness and
portable processing evidence; it is not native callback p99.9/deadline evidence
until a production-style native scheduler owns the endpoint callback.

## Bounded processing-time histogram (2026-09-08)

The callback metrics now retain a fixed 32-bucket logarithmic nanosecond
histogram alongside saturating total and maximum durations. Each processing
observation updates one atomic bucket; no per-callback allocation or sample
list is retained. This is sufficient for a later control-boundary percentile
calculation while preserving the realtime boundary constraints.

The engine suite (70 tests), workspace compilation, strict Clippy, formatting,
diff checks, and documentation validation passed. The histogram is portable
instrumentation readiness only; it does not establish native callback p99.9 or
deadline compliance until a production-style native scheduler owns the endpoint
callback.

## Adapter timing telemetry propagation (2026-09-08)

The Rust `adapter_smoke` probe now reports and validates scheduler processing
time total, maximum, and histogram sample count. Its validation requires the
histogram sample count to equal processed quanta, preventing a silently stale
or partial timing report. The guarded route acceptance requires the same timing
fields and bounded total/max relationship.

The probe check and guarded 300 ms live adapter/route checks passed. The
adapter run reported 116 histogram samples for 116 processed quanta,
1,090,800 ns total, 21,600 ns maximum, zero scheduler xruns/overruns, and
unchanged media state. The routed run passed with 14,400 capture frames,
14,336 scheduler frames, and 13,856 routed frames. This remains adapter/event
loop evidence and does not establish production native callback deadline
compliance.

## Inactive-runtime timing regression (2026-09-08)

The engine regression suite now covers the pre-activation silence path: it
verifies that an inactive `RuntimeProcessor` records exactly one nonzero timing
observation while leaving processed-quanta telemetry at zero and clearing the
block. Engine tests (71), strict Clippy, formatting, diff checks, and workspace
compilation passed. This remains portable instrumentation evidence and does not
establish native callback deadline compliance.

## Routed telemetry completeness regression (2026-09-08)

The guarded routed-adapter acceptance now parses `graph_blocks` and requires
the processing-time histogram sample count to match it exactly. This prevents
a route from passing with only a partial timing report. The 300 ms acceptance
passed with 13,920 capture frames, 13,824 scheduler frames, and 13,824 routed
frames; endpoint/media state remained unchanged.

## Raw timing-distribution propagation (2026-09-08)

The adapter probe now emits all 32 fixed processing-time histogram entries as
`bucket:count` pairs. Both guarded adapter acceptance wrappers require exactly
32 entries, and the routed wrapper preserves the complete distribution in its
final summary. The guarded 300 ms adapter run reported 116 samples, with 22 in
bucket 13 and 94 in bucket 14; the routed run reported 116 samples, with 9 in
bucket 13, 102 in bucket 14, and 5 in bucket 15. The routed aggregate was
1,134,000 ns total and 28,400 ns maximum. Both runs completed with unchanged
media state. This remains adapter/event-loop evidence, not native callback
deadline compliance.

## Portable scheduler deadline telemetry (2026-09-08)

`RealtimeScheduler::process_once_with_deadline` now provides an explicit
caller-owned deadline boundary. Completed processing that finishes after the
deadline records a saturating miss count and total/maximum lateness using
atomics only; the scheduler does not wait, allocate, log, or touch an
endpoint. A regression verifies a late deadline is recorded while the active
generation and processed output remain correct.

Engine tests (72), doc-tests, strict Clippy, formatting, and diff checks pass.
This closes portable callback-integration readiness only. Native endpoint-owned
callback period/deadline measurements, managed-driver lifecycle, signing,
physical latency, and manual application acceptance remain open.

## Shared-mode scheduler deadline qualification (2026-09-08)

The authorized guarded adapter and explicitly selected VB-Audio route now call
`RealtimeScheduler::process_once_with_deadline` for each processed 128-frame
engine quantum. The acceptance wrappers require deadline-miss and lateness
fields and validate their bounds alongside the existing processing histogram.

The 300 ms adapter run processed 120 quanta with zero xruns, zero deadline
misses, and zero deadline lateness. The routed run processed 112 quanta with
zero xruns, zero deadline misses, and zero deadline lateness. Both runs stopped
and reset their streams and verified unchanged media-device state. This is
shared-mode endpoint-adapter evidence only; it does not establish managed
virtual-driver ownership, production callback compliance, physical latency, or
long-term soak behavior.

## Deadline-lateness distribution (2026-09-08)

Scheduler telemetry now retains a fixed 32-bucket logarithmic histogram for
positive deadline lateness. The adapter output preserves all buckets, and both
guarded acceptance wrappers require sequential bucket labels, numeric counts,
and a histogram sum equal to the reported deadline misses.

The follow-up 300 ms adapter run processed 120 quanta with zero deadline
misses/lateness; the routed run processed 116 quanta with zero deadline
misses/lateness. Both stopped/reset their streams and verified unchanged media
state. This remains shared-mode adapter evidence, not managed-driver callback,
physical-latency, or long-term soak evidence.

## Strict deadline-boundary correction (2026-09-08)

Deadline accounting now treats completion exactly at the supplied deadline as
on time. Only strictly positive `Instant` lateness increments the miss count,
lateness totals/maximum, and positive-only histogram. Engine tests (72), strict
Clippy, formatting, diff checks, and documentation validation pass.

## Deterministic exact-deadline regression (2026-09-08)

A direct zero-duration lateness regression now proves that an exactly on-time
completion does not increment deadline misses or the positive lateness
histogram. Engine tests (73), strict Clippy, formatting, diff checks, and
documentation validation pass. Native callback and hardware timing gates are
unchanged.

## Histogram counter-overflow hardening (2026-09-08)

Percentile extraction now saturates the diagnostic histogram total before rank
calculation, preventing malformed or adversarial counter snapshots from
panicking in a diagnostics path. A regression covers a saturated zero bucket
plus an additional sample. Engine tests (75), strict Clippy, formatting, and
diff checks pass. This remains bounded portable telemetry and does not change
native callback or hardware timing evidence.

## Bounded percentile extraction (2026-09-08)

The engine now exposes conservative upper-bound extraction from fixed
logarithmic histograms. The 99.9th-percentile rank uses integer ceiling
arithmetic, rejects invalid percentile inputs, returns no value for an empty
histogram, and never reports below the bucket containing the selected sample.
The adapter emits processing and deadline-lateness p99.9 upper-bound fields;
both guarded live acceptance paths require those bounds to be at least the
reported maxima. Engine tests (74), strict Clippy, formatting, tool checking,
PowerShell parsing, live adapter/route acceptance, diff checks, and
documentation validation pass. The guarded 300 ms runs reported processing
p99.9 upper bounds of 32,768 ns and zero deadline-lateness p99.9 bounds, with
unchanged media-device state. Native callback and hardware timing gates remain
open.

The guarded route acceptance summary now includes both validated p99.9 upper
bounds instead of leaving them visible only in the underlying adapter line.
The 300 ms route requalification reported 14,400 captured frames, 13,856
routed frames, processing maximum 26,200 ns, processing p99.9 upper bound
32,768 ns, and zero deadline misses/lateness. The endpoint snapshot and
explicit configuration-safety checks passed; this remains shared-mode adapter
evidence rather than production callback evidence.

## Rust adapter live requalification (2026-09-08)

The guarded 300 ms production Rust adapter smoke completed with 32 capture
packets, 15,360 captured/scheduled frames, 15,456 silent render frames, 120
graph blocks, zero xruns, zero deadline misses, and a 32,768 ns processing
p99.9 upper bound. Streams stopped/reset cleanly and the media-device snapshot
was unchanged. This is shared-mode endpoint evidence; managed-driver callback,
physical-latency, signing, and installer gates remain open.

The guarded 300 ms route acceptance also passed on the explicitly selected
VB-Audio render/capture pair: 14,400 captured frames, 13,824 routed frames,
112 graph blocks, 32,768 ns processing p99.9 upper bound, and zero xruns or
deadline misses/lateness. Streams stopped/reset and defaults, volume, mute,
privacy, drivers, signing, and startup configuration were unchanged.
## Live shared-mode adapter requalification (`acf1454`, 2026-09-08)

The authorized bounded adapter smoke passed on the selected native endpoints:
24,480 capture frames produced 191 generation-1 graph blocks and 24,448
scheduler frames, with zero scheduler xruns, input/output overruns, or
deadline misses. Processing p99.9 was 65,536 ns. The wrapper used zero-valued
caller-owned render buffers, stopped/reset both streams, and verified an
unchanged media-device snapshot. This is shared-mode user-space adapter
evidence only; it does not satisfy managed-driver callback-deadline or
physical-latency gates.

## Routed adapter requalification (`a4b72ec`, 2026-09-08)

The authorized bounded route acceptance passed on the explicitly selected
existing VB-Audio endpoints: 24,000 capture frames became 23,936 routed
frames across 187 graph blocks. Processing p99.9 was 32,768 ns, with zero
xruns, deadline misses, or deadline lateness. Streams stopped/reset and
endpoint/default/volume/mute/privacy/driver/signing/startup snapshots were
unchanged. This remains shared-mode diagnostic evidence, not managed-driver
callback or physical-latency qualification.

## Rust process-loopback requalification (2026-09-08)

`tests/acceptance/m00-rust-process-live.ps1 -AllowLiveAudio
-DurationMilliseconds 500` passed for both include and exclude modes. Include
processed 21,609 source frames into 23,296 engine frames and 182 scheduler
quanta; exclude processed 22,050 source frames into 23,936 engine frames and
187 quanta. Both used the explicit 44.1 kHz-to-48 kHz bridge, reported zero
rejected packets, scheduler xruns, and buffer overruns/underruns, then stopped
cleanly with persistent media configuration unchanged. This is asynchronous
user-mode process-loopback evidence, not managed-driver callback or physical
latency evidence.
