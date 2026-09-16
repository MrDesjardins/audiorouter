# AudioRouter API reference

This is the readable method and node index for the current development
protocol. The backend discovery response is authoritative for exact JSON
schemas, bounds, defaults, and output shapes:

```powershell
audiorouter schema --json
audiorouter api methods --json
```

All methods use JSON-RPC 2.0 through the local authorized backend. `permission`
is the minimum backend scope; a client name or MCP annotation never grants it.
`side effect` describes the operation boundary exposed to adapters.

## Methods

The current catalog contains 77 methods, including backend-owned quit/finalize,
session portability,
recorder lifecycle, plugin inventory/retry, and startup plan/apply methods added
after the initial 47-method reference.

| Method | Permission | Side effect |
| --- | --- | --- |
| `system.describe` | `read` | read-only |
| `system.handshake` | `read` | read-only |
| `status.get` | `read` | read-only |
| `system.diagnostics` | `read` | read-only |
| `system.quit` | `sessionControl` | external operation; requires an idempotency key; finalizes active recorders before stopping running sessions |
| `system.osTransition` | `sessionControl` | mutating; requires an idempotency key; reports routes requiring explicit resume validation |
| `clients.list` | `read` | read-only |
| `clients.authorize` | `deviceAdministration` | mutating; requires an idempotency key |
| `clients.revoke` | `deviceAdministration` | mutating; requires an idempotency key |
| `operations.get` | `read` | read-only |
| `operations.cancel` | `sessionControl` | mutating; requires an idempotency key |
| `recordings.list` | `record` | read-only |
| `recorders.list` | `record` | read-only |
| `recorders.create` | `record` | mutating; requires an idempotency key |
| `recorders.arm` | `record` | mutating; requires an idempotency key |
| `recorders.start` | `record` | mutating; requires an idempotency key |
| `recorders.pause` | `record` | mutating; requires an idempotency key |
| `recorders.resume` | `record` | mutating; requires an idempotency key |
| `recorders.split` | `record` | mutating; requires an idempotency key |
| `recorders.stop` | `record` | mutating; requires an idempotency key |
| `recordings.get` | `record` | read-only |
| `recordings.recovery` | `record` | read-only |
| `recordings.reveal` | `record` | external operation |
| `recordings.preview` | `record` | read-only |
| `recordings.setMetadata` | `record` | mutating; requires an idempotency key |
| `recordings.rename` | `record` | external operation; requires an idempotency key |
| `recordings.removeEntry` | `record` | mutating; requires an idempotency key |
| `recordings.recycle` | `record` | preview is read-only; confirmed recycling requires an idempotency key |
| `safety.setPrivacyMute` | `capture` | mutating; requires an idempotency key |
| `recovery.clearSafeMode` | `sessionControl` | mutating; requires an idempotency key |

`recordings.recovery` accepts either a `recordingId` for one checkpoint or an
optional `cursor`/`limit` for a bounded recovery listing. Listing returns
checkpoint IDs and `available`, `missing`, or `invalid` status without touching
audio devices or recording files.

Recorder lifecycle methods require `sessionId`; they may also receive
`nodeId` to address an independently attached recorder node. Node-addressed
operations use that node's own lifecycle state and idempotency request hash.
`recorders.create` likewise accepts optional `nodeId`; when supplied, the new
file worker and lifecycle state are attached to that validated recorder node.
`recorders.list` reports `nodeId` for node-targeted entries and keeps it absent
for compatibility session entries; results are bounded and deterministically
ordered.
`recovery.clearSafeMode` clears durable crash markers and the safe-mode latch
atomically with its idempotent journal result; if that journal write cannot be
committed, recovery state remains unchanged.
Finalized node-targeted recording rows from `recordings.list` and
`recordings.get` include the persisted `nodeId`; legacy rows return `null`.
| `startup.get` | `read` | read-only |
| `startup.plan` | `startupWrite` | plan-only |
| `startup.apply` | `startupWrite` | mutating; requires an idempotency key |
| `devices.list` | `read` | read-only |
| `nativeEndpoints.prepare` | `deviceAdministration` | external operation; prepares exact stopped clients |
| `nativeEndpoints.rebind` | `deviceAdministration` | external operation; refreshes and reopens exact stopped clients |
| `nativeEndpoints.detach` | `deviceAdministration` | external operation; detaches an exact stopped native worker |
| `nativeDuplex.detach` | `deviceAdministration` | external operation; detaches an exact stopped project-driver bridge |
| `nativeApplications.prepare` | `deviceAdministration` | external operation; prepares a verified stopped process-loopback capture and exact render client |
| `nativeEndpoints.pump` | `sessionControl` | external operation; drains a bounded packet budget for the exact running native generation |
| `nativeDuplex.pump` | `sessionControl` | external operation; drains independently bounded input and output work for the exact running duplex generation |
| `plugins.scan` | `pluginScan` | read-only |
| `plugins.list` | `pluginScan` | read-only |
| `plugins.retry` | `pluginScan` | mutating; requires an idempotency key |
| `plugins.inspect` | `pluginScan` | read-only |
| `plugins.parameters` | `pluginScan` | external operation; bounded descriptors from the exact revalidated plugin worker |
| `virtualDevices.list` | `read` | read-only |
| `virtualDevices.plan` | `deviceAdministration` | plan-only |
| `virtualDevices.apply` | `deviceAdministration` | mutating; requires an idempotency key |
| `virtualDevices.provision` | `deviceAdministration` | external operation; requires an idempotency key; explicitly creates and owns one managed software device |
| `virtualDevices.remove` | `deviceAdministration` | external operation; requires an idempotency key; explicitly removes one owned managed software device |
| `virtualRoutes.list` | `read` | read-only; returns explicit durable cross-session routes |
| `virtualRoutes.replace` | `deviceAdministration` | mutating; revision-checked and requires an idempotency key |
| `apps.list` | `read` | read-only |
| `applications.list` | `read` | read-only; returns bounded process identity, including nullable executable path and creation timestamp |
| `nodes.types` | `read` | read-only |
| `nodes.describe` | `read` | read-only |
| `presets.list` | `read` | read-only |
| `processors.list` | `read` | read-only |
| `processors.response` | `read` | read-only |
| `routes.inspect` | `read` | read-only |
| `graph.history` | `read` | read-only |
| `graph.undoPlan` | `graphWrite` | plan-only |
| `events.subscribe` | `read` | read-only; replays by cursor with optional session and bounded category filters |
| `sessions.get` | `read` | read-only |
| `sessions.export` | `read` | read-only |
| `sessions.importPlan` | `graphWrite` | plan-only |
| `sessions.importCommit` | `graphWrite` | mutating; requires an idempotency key |
| `sessions.list` | `read` | read-only |
| `sessions.create` | `graphWrite` | mutating; requires an idempotency key |
| `sessions.duplicate` | `graphWrite` | mutating; requires an idempotency key |
| `sessions.delete` | `graphWrite` | mutating; requires an idempotency key |
| `graph.plan` | `graphWrite` | plan-only |
| `graph.commit` | `graphWrite` | mutating; requires an idempotency key |
| `session.start` | `sessionControl` | external operation; requires an idempotency key |
| `sessions.start` | `sessionControl` | external operation; requires an idempotency key |
| `session.stop` | `sessionControl` | external operation; requires an idempotency key |
| `sessions.stop` | `sessionControl` | external operation; requires an idempotency key |

`devices.list` returns active endpoint metadata without opening a stream. Pass
`includeInactive: true` to include disabled, unplugged, not-present, and
forward-compatible unknown endpoint records; those records intentionally omit
format and period data when the endpoint cannot be activated. Each active item
includes a bounded presentation `name` and `defaultRoles`, containing zero or more of `console`,
`multimedia`, and `communications`; these are current Windows default-role
observations, not persistent bindings. A pinned endpoint remains identified by
its opaque ID, and follow-default behavior must be an explicit graph choice.
When endpoint notifications produce a non-empty snapshot diff, the control
plane retains a bounded `devices.changed` state event; clients should refetch
`devices.list` rather than expect endpoint details in the event payload.
If a changed or removed endpoint is owned by a running native endpoint worker,
the worker is stopped and its staged audio is reset before the additional
`devices.bindingInvalidated` event is retained. Rebinding is always explicit
and must revalidate the exact endpoint identity and format.

`nativeEndpoints.prepare` requires exact active capture and render endpoint IDs
and opens both clients stopped; it never changes the system default endpoint,
volume, or mute state. `nativeEndpoints.pump` requires the prepared session's
current positive runtime generation and drains only already-available packets,
up to the documented per-wake bound. It does not wait, rebind, select a
replacement endpoint, or activate a stopped session.
`nativeEndpoints.detach` requires the session to be stopped and removes only
that session's stopped worker, allowing a deliberate exact-endpoint selection
and preparation to follow.
`nativeEndpoints.rebind` is the explicit replacement path for an attached
stopped endpoint worker. It refreshes and validates the supplied exact capture
and render IDs, reopens stopped clients, and returns `configured-stopped`; it
does not choose a replacement or start the session.
`nativeDuplex.detach` has the same stopped-session rule for the project-driver
bridge and returns unavailable on non-Windows hosts.

Pump ticks are intentionally not counted against the user mutation-rate bucket:
they are bounded scheduler traffic rather than user mutations. They remain
authenticated, session-generation-bound, and fail closed when the native
worker or prepared graph is absent. The response also reports the bounded
number of recorder chunks drained on the control thread; file encoding and
flushing never run in the realtime callback.

`nativeDuplex.pump` applies the same authentication, generation, no-rebind,
and fail-closed rules to a paired native bridge worker. Its `maxInputQuanta`
and `maxOutputPackets` budgets are independently bounded, and the response
keeps input and output counters separate so a stalled direction is observable.

The singular and plural session lifecycle names are compatibility aliases with
the same authorization and behavior. Mutating graph and virtual-device calls
require an idempotency key where the discovered input schema says so. For an
authenticated client, operation keys are isolated by client and method; the
original human-readable operation ID remains in responses and events.

## Node types

The current node catalog is available through `nodes.describe` and contains:

| Type | Availability | Notes |
| --- | --- | --- |
| `physical-input@1` | unavailable | Requires M02 Windows audio adapters |
| `application-capture@1` | unavailable | Requires M02 Windows audio adapters |
| `endpoint-loopback@1` | unavailable | Requires M02 Windows audio adapters |
| `physical-output@1` | unavailable | Requires M02 Windows audio adapters |
| `virtual-render-source@1` | unavailable | Requires M03 managed virtual driver |
| `virtual-capture-sink@1` | unavailable | Requires M03 managed virtual driver |
| `recorder@1` | available | Bounded recording sink boundary; the runtime tap is attached by the control plane |
| `mixer@1` | available | Bounded graph mixer |
| `gain@1` | available | `gainDb`, from -60 to +24 dB |
| `mute@1` | available | `muted`, boolean |
| `meter@1` | available | Bounded per-node telemetry boundary |
| `parametric-eq@1` | available | Eight independently enabled bands; peaking, shelf, pass, and notch filters |
| `compressor@1` | available | Stereo-capable dynamics stage; 48 kHz graph baseline |
| `gate@1` | available | Downward gate/expander stage; 48 kHz graph baseline |
| `limiter@1` | available | Sample-peak ceiling stage; -12 to 0 dBFS; bounded 0–10 ms lookahead and 10–1,000 ms release |
| `delay@1` | available | Preallocated delay stage; 0 to 1,000 ms |
| `graphic-eq@1` | available | Fixed ten-band EQ; `band0Db`–`band9Db`, -18 to +18 dB |
| `pitch@1` | available | Fixed 128-frame streaming pitch stage; 1,024 estimated latency samples; semitones -12 to +12 and cents -100 to +100 |

The separately reported `processors` catalog in `system.describe` documents the
implemented DSP primitives and their typed parameter ranges. `parametricEq` is
available as the corresponding eight-band graph node (legacy `frequencyHz`,
`q`, and `gainDb` fields remain as band-0 compatibility aliases), and `compressor` is
available as graph dynamics nodes, `delay` is available as a preallocated graph
time stage, `graphicEq` is available as a fixed ten-band graph EQ, and `pitch`
is available as a fixed-quantum streaming graph stage. Pitch reset/reconnect
semantics and measured realtime quality remain qualification items.
The pitch entry reports its 1,024-sample algorithmic latency.
Route inspection paths also report accumulated `latencySamples`; this includes
the declared 1,024-sample pitch warmup and configured built-in delay at the
48 kHz portable graph baseline. The default limiter lookahead contributes 240
samples at that baseline; changing lookahead or sample rate changes the
effective latency and must be included by callers when budgeting the route.
The UI displays these parameter types and ranges as read-only metadata; it does
not imply that an unavailable processor can be activated.

`processors.response` is a bounded, read-only EQ preview contract. It accepts a
sample rate, up to eight enabled/disabled parametric bands, and up to 256
frequencies, then returns one magnitude value per frequency. The calculation
uses the same Rust biquad coefficients as the audio processor; clients must
not duplicate coefficient math. This endpoint has no audio, graph, plugin, or
machine-configuration side effects.

The built-in preset catalog is exposed by `presets.list`. Each entry includes a
stable numeric `version` alongside its ID, name, and explanation. It currently includes
the voice-chain presets `voiceNeutral` and `voiceGateAndCompression`, plus EQ
starting points `voiceNeutral`, `hum50Hz`, and `hum60Hz`. Preset discovery is
read-only; applying a graph change still requires an explicit plan and commit.

## Adapter commands

The CLI and MCP adapters route through the same control dispatcher. The generic
CLI path is useful when no typed convenience command exists:

```powershell
audiorouter api call status.get --json
audiorouter api call sessions.list --database C:\path\state.sqlite --json
audiorouter diagnostics --output C:\path\diagnostics.json --json
audiorouter diagnostics export --output C:\path\diagnostics.json --json
```

The processor catalog is also available through the typed read-only command
`audiorouter processors list --json`.

Discovery failures are not represented as an empty inventory. The CLI discovery
commands preserve the JSON-RPC error envelope, including `data.code`, unsigned
Windows `data.hresult` when available, `data.retryable`, and `data.remediation`.
The MCP adapter carries the same envelope in both `structuredContent` and its
textual JSON content. Clients should branch on the stable code (for example,
`deviceInUse` or `accessDenied`) rather than parsing the English message.

The diagnostics export is a redacted, read-only JSON snapshot. Its destination
must be absolute and must not already exist; this prevents accidental overwrite
of an earlier support bundle. The explicit `diagnostics export` form and the
option form are equivalent.

Process-loopback packet telemetry is currently exposed by the Windows adapter
that owns a live capture object. When a native endpoint worker is attached,
the control-plane diagnostics response also exposes its bounded scheduler
counter snapshot as `schedulerTelemetry`; it is `null` when no worker is
attached and therefore never presents an unavailable adapter as healthy zeros.
Its bounded snapshot reports wait calls/timeouts, successful packets and
frames, minimum/maximum packet period, silent packets, and rejected packets.
The adapter scheduler probe additionally reports bounded processing-time and
deadline-lateness distributions, including conservative p99.9 bucket upper
bounds, for its processed engine quanta; these are probe evidence, not
control-plane or production-driver callback telemetry.
When no native adapter session is owned by the control plane,
`system.diagnostics.nativeAdapter` is `implemented-not-activated` and
`nativeSessionId` is `null`. An explicitly attached but stopped worker reports
`configured-stopped`; a deliberately started worker reports `running` and its
bound session ID. Attachment requires a caller-supplied exact-binding worker,
so discovery and ordinary session open cannot activate audio implicitly. These
statuses do not claim a production driver, graph activation, or physical
latency qualification.
`nativeAdapterKind` is `endpoint` for the single endpoint worker,
`duplex` for a paired bridge worker, and `null` when no worker is attached.
Adapters use it to select the matching pump method.

`schedulerTelemetry` contains the active graph generation and negotiated sample
rate, bounded input/output overrun and underrun counters, processed quanta,
repaired samples, xruns, processing-time totals/maxima, and deadline-miss and
lateness counters. It is a read-only control-thread snapshot of atomics; the
audio callback does not wait for diagnostics or serialize JSON.

`nodeTelemetry` is a bounded array of observations for the attached session's
prepared meter, gate, compressor, and limiter nodes. Each item includes the
authored node ID and kind, with finite meter dB/clip values or processor gain
reduction/gate state. It is empty without an attached worker or when no
matching prepared stage is available; a busy realtime processor is omitted
rather than waited on. The response is observational and does not alter the
saved graph.

The nested telemetry schema is strict: meter and processor fields are required
when present, channel arrays are capped at two channels, counters are
nonnegative, and unknown properties are rejected.

The UI requests this diagnostic snapshot at most once per second while a
session is running. Diagnostics are not replayed or retained as meter events;
the UI keeps the last successful snapshot if a refresh fails.

The MCP stdio adapter exposes focused read/write tools and `call_api`; it uses
the enrolled client identity and cannot bypass the backend permission checks.
See the [headless runbook](headless-runbook.md) for launch and recovery
examples. Native audio activation, managed endpoint provisioning, and signed
driver actions remain unavailable and are reported as such by discovery.
