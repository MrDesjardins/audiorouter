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

| Method | Permission | Side effect |
| --- | --- | --- |
| `system.describe` | `read` | read-only |
| `system.handshake` | `read` | read-only |
| `status.get` | `read` | read-only |
| `system.diagnostics` | `read` | read-only |
| `clients.list` | `read` | read-only |
| `clients.authorize` | `deviceAdministration` | mutating |
| `clients.revoke` | `deviceAdministration` | mutating |
| `operations.get` | `read` | read-only |
| `operations.cancel` | `sessionControl` | mutating |
| `recordings.list` | `record` | read-only |
| `recordings.get` | `record` | read-only |
| `recordings.recovery` | `record` | read-only |
| `recordings.reveal` | `record` | external operation |
| `recordings.preview` | `record` | read-only |
| `recordings.setMetadata` | `record` | mutating |
| `recordings.rename` | `record` | external operation |
| `recordings.removeEntry` | `record` | mutating |
| `recordings.recycle` | `record` | external operation |
| `safety.setPrivacyMute` | `capture` | mutating |
| `recovery.clearSafeMode` | `sessionControl` | mutating |
| `startup.get` | `read` | read-only |
| `devices.list` | `read` | read-only |
| `plugins.scan` | `pluginScan` | read-only |
| `plugins.inspect` | `pluginScan` | read-only |
| `virtualDevices.list` | `read` | read-only |
| `virtualDevices.plan` | `deviceAdministration` | plan-only |
| `virtualDevices.apply` | `deviceAdministration` | mutating |
| `apps.list` | `read` | read-only |
| `applications.list` | `read` | read-only |
| `nodes.types` | `read` | read-only |
| `nodes.describe` | `read` | read-only |
| `presets.list` | `read` | read-only |
| `routes.inspect` | `read` | read-only |
| `graph.history` | `read` | read-only |
| `graph.undoPlan` | `graphWrite` | plan-only |
| `events.subscribe` | `read` | read-only |
| `sessions.get` | `read` | read-only |
| `sessions.list` | `read` | read-only |
| `sessions.create` | `graphWrite` | mutating |
| `sessions.duplicate` | `graphWrite` | mutating |
| `sessions.delete` | `graphWrite` | mutating |
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

The built-in preset catalog is exposed by `presets.list`. It currently includes
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

The diagnostics export is a redacted, read-only JSON snapshot. Its destination
must be absolute and must not already exist; this prevents accidental overwrite
of an earlier support bundle. The explicit `diagnostics export` form and the
option form are equivalent.

The MCP stdio adapter exposes focused read/write tools and `call_api`; it uses
the enrolled client identity and cannot bypass the backend permission checks.
See the [headless runbook](headless-runbook.md) for launch and recovery
examples. Native audio activation, managed endpoint provisioning, and signed
driver actions remain unavailable and are reported as such by discovery.
