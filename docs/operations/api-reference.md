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

The current catalog contains 61 methods, including the session portability,
recorder lifecycle, plugin inventory/retry, and startup plan/apply methods added
after the initial 47-method reference.

| Method | Permission | Side effect |
| --- | --- | --- |
| `system.describe` | `read` | read-only |
| `system.handshake` | `read` | read-only |
| `status.get` | `read` | read-only |
| `system.diagnostics` | `read` | read-only |
| `clients.list` | `read` | read-only |
| `clients.authorize` | `deviceAdministration` | mutating; requires an idempotency key |
| `clients.revoke` | `deviceAdministration` | mutating; requires an idempotency key |
| `operations.get` | `read` | read-only |
| `operations.cancel` | `sessionControl` | mutating; requires an idempotency key |
| `recordings.list` | `record` | read-only |
| `recorders.arm` | `record` | mutating |
| `recorders.start` | `record` | mutating |
| `recorders.pause` | `record` | mutating |
| `recorders.resume` | `record` | mutating |
| `recorders.split` | `record` | mutating |
| `recorders.stop` | `record` | mutating |
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
| `startup.get` | `read` | read-only |
| `startup.plan` | `sessionControl` | plan-only |
| `startup.apply` | `sessionControl` | mutating |
| `devices.list` | `read` | read-only |
| `plugins.scan` | `pluginScan` | read-only |
| `plugins.list` | `pluginScan` | read-only |
| `plugins.retry` | `pluginScan` | mutating |
| `plugins.inspect` | `pluginScan` | read-only |
| `virtualDevices.list` | `read` | read-only |
| `virtualDevices.plan` | `deviceAdministration` | plan-only |
| `virtualDevices.apply` | `deviceAdministration` | mutating |
| `apps.list` | `read` | read-only |
| `applications.list` | `read` | read-only; returns bounded process identity, including nullable executable path and creation timestamp |
| `nodes.types` | `read` | read-only |
| `nodes.describe` | `read` | read-only |
| `presets.list` | `read` | read-only |
| `processors.list` | `read` | read-only |
| `routes.inspect` | `read` | read-only |
| `graph.history` | `read` | read-only |
| `graph.undoPlan` | `graphWrite` | plan-only |
| `events.subscribe` | `read` | read-only; replays by cursor with optional session and bounded category filters |
| `sessions.get` | `read` | read-only |
| `sessions.export` | `read` | read-only |
| `sessions.importPlan` | `graphWrite` | plan-only |
| `sessions.importCommit` | `graphWrite` | mutating |
| `sessions.list` | `read` | read-only |
| `sessions.create` | `graphWrite` | mutating; requires an idempotency key |
| `sessions.duplicate` | `graphWrite` | mutating; requires an idempotency key |
| `sessions.delete` | `graphWrite` | mutating; requires an idempotency key |
| `graph.plan` | `graphWrite` | plan-only |
| `graph.commit` | `graphWrite` | mutating |
| `session.start` | `sessionControl` | external operation |
| `sessions.start` | `sessionControl` | external operation |
| `session.stop` | `sessionControl` | external operation |
| `sessions.stop` | `sessionControl` | external operation |

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
| `mixer@1` | available | Bounded graph mixer |
| `gain@1` | available | `gainDb`, from -60 to +24 dB |
| `mute@1` | available | `muted`, boolean |
| `meter@1` | available | Bounded per-node telemetry boundary |
| `parametric-eq@1` | available | One-band peaking EQ stage; 48 kHz graph baseline |
| `compressor@1` | available | Stereo-capable dynamics stage; 48 kHz graph baseline |
| `gate@1` | available | Downward gate/expander stage; 48 kHz graph baseline |
| `limiter@1` | available | Sample-peak ceiling stage; -12 to 0 dBFS |
| `delay@1` | available | Preallocated delay stage; 0 to 1,000 ms |
| `graphic-eq@1` | available | Fixed ten-band EQ; `band0Db`–`band9Db`, -18 to +18 dB |
| `pitch@1` | available | Fixed 128-frame streaming pitch stage; 1,024 estimated latency samples; semitones -12 to +12 and cents -100 to +100 |

The separately reported `processors` catalog in `system.describe` documents the
implemented DSP primitives and their typed parameter ranges. `parametricEq` is
available as the corresponding one-band graph node, and `compressor` is
available as graph dynamics nodes, `delay` is available as a preallocated graph
time stage, `graphicEq` is available as a fixed ten-band graph EQ, and `pitch`
is available as a fixed-quantum streaming graph stage. Pitch reset/reconnect
semantics and measured realtime quality remain qualification items.
The pitch entry reports its 1,024-sample algorithmic latency.
Route inspection paths also report accumulated `latencySamples`; this includes
the declared 1,024-sample pitch warmup and configured built-in delay at the
48 kHz portable graph baseline.
The UI displays these parameter types and ranges as read-only metadata; it does
not imply that an unavailable processor can be activated.

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
that owns a live capture object, not by the control-plane diagnostics response.
Its bounded snapshot reports wait calls/timeouts, successful packets and
frames, minimum/maximum packet period, silent packets, and rejected packets.
The adapter scheduler probe additionally reports bounded processing-time and
deadline-lateness distributions, including conservative p99.9 bucket upper
bounds, for its processed engine quanta; these are probe evidence, not
control-plane or production-driver callback telemetry.
When no native adapter session is owned by the control plane,
`system.diagnostics.nativeAdapter` remains `not activated`; it must not be
interpreted as a zeroed live stream.

The MCP stdio adapter exposes focused read/write tools and `call_api`; it uses
the enrolled client identity and cannot bypass the backend permission checks.
See the [headless runbook](headless-runbook.md) for launch and recovery
examples. Native audio activation, managed endpoint provisioning, and signed
driver actions remain unavailable and are reported as such by discovery.
