# M03 virtual-routing contract evidence

## Virtual-device operation schema (2026-09-07)

The `virtualDevices.plan` input and `virtualDevices.plan`/`virtualDevices.apply`
outputs now reuse one fixed operation schema. It constrains the action enum,
128-byte bus identity, 120-character bus name, and optional enabled flag,
matching the domain lifecycle validator. Control discovery regression coverage,
strict Clippy, formatting, and documentation validation pass. Driver
installation and endpoint activation remain open.

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

## Portable bridge boundary (2026-09-07)

The engine now provides `VirtualBusBridge`, a fixed-capacity render-to-capture
boundary for the future managed driver. It copies only same-shaped blocks,
starts inactive and silent, rejects stale ownership generations, clears queued
blocks on deactivation or replacement, and drops input when the capture ring is
full rather than growing memory. The 45-test engine suite and strict Clippy
pass. This is not native endpoint or driver evidence and does not open audio.

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

## Portable bounded fan-out (2026-09-07)

`VirtualBusBridge::fanout_once` now copies each accepted render block into
multiple caller-owned destination rings. Delivery is independently bounded per
destination, and a full/slow destination cannot block or grow the others. The
engine suite passes 46 tests with strict Clippy. This supports the portable
one-to-many bridge contract only; native virtual endpoints and live routing
remain unimplemented.

## Allocation-free capture silence (2026-09-07)

The bridge consumer API now fills a caller-owned block and clears it to
initialized silence on inactive or underrun reads, returning a delivery flag.
Shape mismatches are reported without retaining queued data. Existing bridge,
fan-out, and backpressure regressions remain green at 46 engine tests with
strict Clippy. This is portable bridge behavior only; no live endpoint is
opened.

## Bridge sample-safety hardening (2026-09-07)

Both bridge processing paths now sanitize NaN and infinite input samples to
silence before publishing capture blocks or fan-out copies. The regression
uses non-finite input and verifies finite zero output for every destination.
The engine suite passes 46 tests with strict Clippy. This is portable safety
evidence only and does not open a live endpoint.

## Publication generation guard (2026-09-07)

Bridge processing now rechecks active ownership and generation after copying
and sanitizing, before publishing to capture or fan-out destinations. A block
that observes deactivation or replacement is recycled and counted as dropped.
The 46-test engine suite and strict Clippy pass. This closes a portable
publication-safety boundary; native driver synchronization remains open.

## Atomic ownership generation advancement (2026-09-07)

Bridge activation now advances the ownership generation with an atomic
compare-exchange. A stale activation request is rejected without clearing the
current active generation, while accepted replacements still clear queued
blocks before becoming active. The 46-test engine suite and strict Clippy
pass. Native driver ownership synchronization remains open.

## Concurrent activation ownership (2026-09-07)

Four concurrent activation attempts for the same bridge generation are now
covered by a barrier-synchronized regression; exactly one succeeds, the others
are rejected as stale, and the bridge remains active at the winning generation.
The engine suite passes 47 tests with strict Clippy. This validates portable
ownership contention only, not native driver synchronization.

## Serialized replacement activation (2026-09-07)

Bridge activation and deactivation now serialize the control-plane generation
transition and queue drain. This prevents concurrent replacement or shutdown
requests from interleaving their drain/reactivate sequences. The realtime
bridge methods never acquire the guard; the engine suite passes 48 tests with
strict Clippy. Native driver synchronization remains open.

## Capture shape-mismatch safety (2026-09-07)

The bridge regression now submits a valid block, reads it into an incompatible
consumer shape, and verifies `ShapeMismatch` while the queued block is
recycled. This prevents malformed consumers from retaining stale bridge data.
The 48-test engine suite and strict Clippy pass; no live endpoint is opened.

## Post-submit generation guard (2026-09-07)

The bridge now checks ownership after capture publication as well as before
copying. If activation drains immediately before an old-generation submit,
the stale block is counted and the capture queue is drained before it can be
read by the replacement owner. The 48-test engine suite and strict Clippy
pass; native driver synchronization remains open.

## Generation-tagged bridge buffers (2026-09-07)

Bridge blocks now carry their runtime ownership generation through pooled
copies. Capture consumers reject and recycle stale tagged blocks, and the
bounded ring exposes the same filter for fan-out consumers. Regression coverage
verifies tag propagation and stale-block recycling; the engine suite passes 49
tests with strict Clippy. Native driver synchronization remains open.

Inactive capture now drains and rejects queued blocks before applying the
generation filter, preserving the bridge's fail-silent shutdown contract even
if a consumer races deactivation. The 49-test engine suite and strict Clippy
remain green; no live endpoint is opened.
## Virtual-device inventory page bound (2026-09-07)

The read-only `virtualDevices.list` contract now shares a 500-item page bound
across input validation, runtime truncation, and discovery schema metadata.
Control tests (86), strict Clippy, and documentation validation pass. This
does not create endpoints or claim managed-driver lifecycle functionality.

The legacy unpaged `virtualDevices.list` array response now advertises the
authoritative eight-bus domain ceiling. The in-memory registry already rejects
the ninth bus, so complete unpaged results remain safe without arbitrary
truncation or managed-driver activation.
## Virtual-bus name limit (2026-09-07)

The managed virtual-bus name ceiling is now the public domain constant
`MAX_VIRTUAL_BUS_NAME_CHARS` (120 Unicode characters). Domain validation,
virtual-device plan/item schemas, and `system.describe` discovery all reuse
the same value; the domain boundary regression covers an over-limit name.
Domain/control tests, strict Clippy, formatting, and documentation validation
pass. Driver installation and endpoint activation remain open.
## Virtual-device identity contract (2026-09-07)

Managed virtual-device plan operations and list items now advertise the shared
128-byte entity-ID limit enforced by the domain registry. Control discovery
regression coverage verifies both input and output schemas. Workspace tests,
strict Clippy, formatting, and documentation validation pass; driver
installation and endpoint activation remain open.
