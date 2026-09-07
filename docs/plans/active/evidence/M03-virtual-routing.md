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

The MCP adapter now exposes `plan_virtual_device` and `apply_virtual_device`
alongside the inventory tool. They use the same authorized control-plane
methods, preserving device-administration scope checks and apply idempotency.
The MCP catalog and process interoperability tests pass with 26 tools; no
native endpoint is created while the driver capability is unavailable.

Apply cleanup is atomic at the storage boundary: replacement of the desired
bus rows and deletion of the durable lifecycle plan occur in one transaction.
This prevents a crash between those steps from replaying an already-applied
plan. Focused storage/control tests and strict Clippy pass.

Inventory results now include explicit capability flags, the required
deviceAdministration privilege, restart impact, and client-impact metadata.
All render/capture capabilities and endpoint IDs remain unavailable until the
managed driver exists, preventing callers from mistaking desired state for
provisioned Windows devices.

Lifecycle authorization is explicitly scoped to `deviceAdministration`; the
read-only, editor, and operator convenience grants cannot plan or apply bus
changes. The regression target compiles and strict Clippy passes. Execution of
that rebuilt control test was blocked before launch by Windows Application
Control OS error 4551, so this checkpoint records compile/static evidence rather
than a runtime test pass.

Virtual-device apply results now use the durable operation journal. Bus state,
plan deletion, and the replay result commit together; after restart, a retry
with the same idempotency key returns the completed result even when the plan
row is gone. The control restart regression passes in available runs, and
storage/control strict Clippy checks remain green.

The journal entry is now bound to a deterministic hash of the apply method and
plan ID. A reused idempotency key with a different plan is rejected with the
stable `idempotencyConflict` code both in memory and after restart; same-request
replay remains allowed. This closes the request-confusion gap without opening
audio or changing Windows endpoint configuration.

In-memory operation outcomes now use one bounded retention path for graph and
virtual-device operations. Evicting a virtual result also evicts its request
hash, keeping the replay/conflict metadata bounded with the cached outcome.

Virtual-device plans are now durable SQLite records containing the validated
operation and bounded expiry. Valid plans reload after a control restart and are
removed after successful apply; expired plans are hidden. Storage and control
regressions verify this without opening audio or creating endpoints.

The registry now exposes force-release cleanup for a crashed or disconnected
owner. Cleanup clears the active owner but preserves the monotonic generation;
a delayed release from the old owner cannot release a replacement lease. This
is portable ownership evidence only: bridge heartbeat detection, buffer reset,
and native endpoint silence/recovery remain unimplemented.

## UI inventory and desired-state controls (2026-09-06)

The M05 editor now renders a managed virtual-device panel backed by
`virtualDevices.list`, `virtualDevices.plan`, and `virtualDevices.apply`. It
shows the authoritative inventory and requires an explicit plan followed by an
apply action for desired-state creation, rename, enable/disable, and deletion.
The panel preserves the backend's
`deviceAdministration` requirement and unavailable-driver explanation; it does
not synthesize endpoint IDs or activate a native bus. UI typecheck, all 61
tests, and the disposable production build pass.

## Portable global feedback boundary (2026-09-07)

The domain now exposes `VirtualBusRoute` and `validate_global_graph` for the
known cross-session topology. It validates each session, requires referenced
sessions to exist, rejects duplicate routes and conflicting writers for one
managed bus, and rejects cycles formed by virtual-bus boundaries. One writer
fan-out to multiple consumers is accepted. `GraphStore` exposes the same check
over its current sessions. The 38-test domain suite and strict Clippy pass.

This is a portable control-plane proof only. It does not identify native
endpoint identities, inspect unknown external application selections, install a
driver, or open a live audio stream; those M03 gates remain open.

## CLI lifecycle parity (2026-09-07)

The CLI now exposes `virtual-devices plan --operation <json-file> --database
<path>` and `virtual-devices apply <plan-id> --idempotency-key <key>
--database <path>`. Both commands use the explicit `deviceAdministration`
grant and the durable control-plane plan/journal. The focused CLI/MCP process
suite passes with 25 CLI tests and two interoperability tests. Apply persists
the desired bus state and reports `state: applied` with endpoint availability
still `unavailable` until the managed driver exists. No endpoint, driver, or
machine audio configuration is changed.

## Capacity discovery parity (2026-09-07)

`system.describe.limits` now includes the authoritative `maxVirtualBuses: 8`
value alongside the graph/session limits. The JSON schema marks it as required,
and the control discovery regression verifies the value comes from the domain
constant. Control tests (81), strict Clippy, and documentation validation pass.
This reports desired-state capacity only; it does not imply that native driver
endpoints exist.

## Global validation hardening (2026-09-07)

The reusable global validator now rejects aggregate node/edge counts above the
declared global budgets and empty virtual-bus IDs before evaluating known
cross-session routes. A regression covers 129 nodes across three sessions and
an empty bus identifier. The domain suite passes 39 tests with strict Clippy.
This remains portable validation and does not create endpoints or open audio.

## Control-plane capacity acceptance (2026-09-07)

The control dispatcher now has an acceptance regression that creates all eight
declared virtual buses through the real plan/apply path, verifies the resulting
inventory, and confirms that a ninth create plan is rejected at the lifecycle
boundary. The control suite passes 82 tests with strict Clippy. Desired state is
persisted and clearly remains unavailable until native driver provisioning.
