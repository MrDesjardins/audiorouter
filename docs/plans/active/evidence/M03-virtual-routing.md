# M03 virtual-routing contract evidence

## 2026-09-06 — capability-contract foundation

The authoritative Rust and shared TypeScript node registries now include
`virtual-render-source@1` and `virtual-capture-sink@1`. Both advertise an
explicit unavailable capability reason: `requires M03 managed virtual driver`.
The UI library exposes both entries for discovery but keeps them disabled, so
the editor cannot imply that a virtual endpoint exists or install a driver.

The portable engine matches both kinds as device-bound placeholders. Disabled
nodes therefore retain the existing silence-boundary behavior; enabled nodes
do not create a fake bus or bypass resource validation. M03 driver installation,
bridge ownership, endpoint identity, and Windows routing remain open.

Validation: all locked Rust workspace tests and strict Clippy passed; the
contracts TypeScript check passed; UI Vitest passed 56 tests using Vite's
runner config loader. The UI production build could transform and render but
Windows returned `EPERM` while creating the configured or alternate output
directory, so no production-build pass is claimed. No driver, audio endpoint,
or machine audio configuration was changed.

The control-plane foundation now includes `VirtualBusLease`. A lease accepts
one non-empty owner, rejects competing acquisition, requires both owner and
generation for release, and prevents delayed releases from clearing a newer
owner. `force_release` clears ownership for crash/reconnect cleanup while the
generation remains monotonic. Two domain regressions cover ownership and stale
release behavior. This primitive carries no audio and is not a driver or
endpoint implementation.

`VirtualBusRegistry` now provides the portable desired-state inventory for up
to eight stereo buses. It trims and bounds names, rejects duplicate IDs and
case-insensitive names, sorts listing by stable ID, requires disablement before
delete, and refuses deletion while a lease is held. Lease acquisition/release
is routed through the same registry. Two additional regressions cover naming,
capacity, disable/delete ordering, and lease cleanup. Native driver endpoint
creation, persistence, bridge ownership, and external-client routing remain
open.

The shared control API now exposes `virtualDevices.list` as a read-only,
cursor-compatible inventory query. It reports each managed bus's desired state,
availability reason, endpoint identity placeholders, and current lease owner;
the empty initial registry is returned without activating a stream or creating
an endpoint. The method is included in domain discovery and the generated
TypeScript client contract.

CLI and MCP adapter parity now expose the same read-only query as
`virtual-devices list` and `list_virtual_devices`, respectively. The CLI
regression confirms an empty initial managed inventory rather than inventing
third-party cable endpoints. Lifecycle mutation, persistence, and native driver
integration remain open.

Desired bus inventory is now durable in SQLite. The storage layer persists
stable ID, validated name, and enabled state transactionally; leases are
runtime-only and are intentionally clear after restart. Control-plane methods
persist successful create, rename, enable/disable, and delete operations and
roll back the in-memory registry if storage fails. Storage and control restart
regressions verify the behavior without creating endpoints.

The UI backend now exposes the same inventory through `listVirtualDevices`,
normalizing the bounded page shape for React consumers. Its regression confirms
the client requests `virtualDevices.list` and preserves the unavailable/empty
state; the UI does not offer a provisioning side effect. UI tests pass 57 tests
and typecheck passes.

The lifecycle API now provides `virtualDevices.plan` and
`virtualDevices.apply`. Plans validate the candidate registry before issuing a
five-minute plan ID; apply rechecks expiry, commits only the desired registry
state, persists it transactionally, and replays a completed result for the same
idempotency key. Create/rename/enable-disable/delete are represented explicitly,
while the result continues to report `requires M03 managed virtual driver`, so
no API call claims that a Windows endpoint was provisioned. Control tests verify
the create/apply/list flow and idempotent replay.

The UI backend now exposes typed `planVirtualDevice` and `applyVirtualDevice`
methods. The live adapter forwards the shared API contracts, while the demo
adapter returns an explicit unavailable plan or rejects apply. UI tests cover
both request shapes; 58 UI tests and typecheck pass.

The registry now exposes force-release cleanup for a crashed or disconnected
owner. Cleanup clears the active owner but preserves the monotonic generation;
a delayed release from the old owner cannot release a replacement lease. This
is portable ownership evidence only: bridge heartbeat detection, buffer reset,
and native endpoint silence/recovery remain unimplemented.
