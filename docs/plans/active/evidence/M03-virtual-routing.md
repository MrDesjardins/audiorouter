# M03 virtual-routing contract evidence

## 2026-09-17 - current adapter-bridge recheck boundary

The exact CABLE Output capture and CABLE Input render endpoint IDs remained in
the active inventory. A non-elevated direct `adapter-bridge 100` probe returned
`IAudioClient::Initialize(capture,polling)` HRESULT `0x80070057`, while the
same raw Rust initialization and full bridge succeeded from an elevated
process. The elevated bridge produced 4,800 captured frames, 37 processed
quanta, 4,736 rendered frames, zero drops/xruns/deadline misses, and a
25,072-byte finalized temporary recording. The capture adapter's exact
`E_INVALIDARG` duration retry remains bounded and tested; the operative live
qualification prerequisite is now recorded as the required elevated
`deviceAdministration` execution context, not an endpoint-format failure.

The authoritative elevated `m02-rust-adapter-bridge-live.ps1` acceptance then
passed two 500 ms cycles on the same exact pair. Each cycle produced 51
packets, 24,480 captured frames, 191 processed quanta, 24,448 rendered frames,
zero non-finite tap samples, drops, xruns, or deadline misses, and a 25,072-byte
finalized recording. Temporary streams and recordings were removed and media
device state remained unchanged.

## 2026-09-17 - current VB-Cable-first virtual-routing acceptance

`tests/acceptance/m03-virtual-buses.ps1` passed the bounded desired-state
planning and persistence lifecycle, including create/rename/enable/disable/
delete operations, cross-session route persistence, cycle and conflicting-
writer rejection, revision checks, and cleanup across separate CLI processes.
The acceptance explicitly reported managed-device availability as
`unavailable`; no native device provisioning, driver installation/loading, or
machine audio configuration occurred. This is the current supported
VB-Cable-first contract boundary, not managed-driver qualification.

## 2026-09-16 - managed-device presence boundary

An elevated read-only PnP query for `SWD\\AudioRouterVirtual*` returned no
matching present device. The project-owned virtual endpoint is therefore not
provisioned on this host. No device state changed; temporary creation remains
an explicitly authorized isolated-environment operation.

## 2026-09-16 - locked workspace requalification after lifecycle hardening

The locked workspace unit/integration suites and doc-tests passed after the
managed-device rollback and native schema updates. Relevant totals included
control 175 passed with 3 guarded live tests ignored, storage 92, engine 116,
Windows-audio 87, CLI 36, and transport 19, with the remaining crate suites
also green. No driver or endpoint was opened; loaded PortCls and release
hardware gates remain separate.

## 2026-09-16 - managed virtual-bus lifecycle requalification

`m03-virtual-buses.ps1` passed bounded create/rename/enable/disable/delete,
cross-session route persistence, cycle/conflicting-writer rejection, and
revision checks across separate CLI processes. This qualifies the durable
desired-state lifecycle only; no native device provisioning, driver
installation/loading, or audio configuration action occurred.

## 2026-09-16 - managed software-device dry-run

`m03-swdevice-probe.ps1` passed its native compile and default no-side-effect
dry-run. The probe did not create a software device, install/load a driver,
open an endpoint, or change machine audio configuration. Explicit
administrator-authorized provisioning and loaded-driver endpoint qualification
remain separate gates.

## 2026-09-16 - locked workspace requalification

The locked workspace test run passed across all crate unit/integration suites
and doc-tests after the public bridge authorization and bounds additions. The
control suite included 175 passing tests and three guarded live tests ignored;
no driver or endpoint was opened. This is cross-layer portable evidence, not
loaded PortCls transport qualification.

## 2026-09-16 - full control requalification

The locked control suite passed 175 tests with three explicitly guarded live
tests ignored. This includes shared bridge authorization and bounds,
generation/lease cleanup, virtual route isolation, and fail-closed recovery.
No driver or endpoint was opened; loaded PortCls transport remains a separate
gate.

## 2026-09-16 - public bridge input bounds

The Windows-gated control regression
`native_bridge_preparation_rejects_unbounded_or_relative_inputs_before_driver_open`
passes: zero lease, zero generation, and relative mapping paths are rejected
at `prepare_native_bridge` before any driver open. Formatting passed. This is
bounded API validation evidence and does not qualify loaded-driver transport.

## 2026-09-16 - native bridge authorization boundary

The control regression `native_bridge_preparation_requires_device_administration_before_parameters`
passes: an operator without the dedicated `deviceAdministration` scope is
rejected by `nativeBridges.prepare` before bridge parameters are processed or
native bridge state is mutated. This is shared API authorization evidence and
does not qualify loaded-driver transport.

## 2026-09-16 - virtual shelf-drop placement regression

The M05 canvas regression now covers a virtual capture-sink shelf drop through
the App adapter: the existing bus ID prompt remains explicit, the returned
node identity is retained, and the exact computed canvas position is persisted
in presentation layout. The focused `SessionFlowCanvas` suite passed 14 tests
with TypeScript typecheck green. This is stopped draft/UI evidence and does
not claim managed-driver endpoint publication.

## 2026-09-16 - explicit multi-input branch binding

The shared API now exposes nativeMultiInputs.bindBranches. Control validates
the running generation and exact ordered destination node IDs from the
prepared fan-out graph, then attaches only the matching virtual-bus or exact
recorder-node observer sets while the worker is stopped. Reordered, unknown, unsupported, or
generation-stale branches fail before tap membership changes. This provides
the control contract for drag/drop-authored virtual output branches; it is
portable/control evidence and does not qualify a loaded driver or live
multi-capture route.

When a prepared multi-input graph contains only virtual capture-sink or
recorder branches, binding creates a bounded tap-only output owner. Physical
branches are rejected without a prepared physical render owner, preserving
truthful endpoint state and branch isolation. The worker still starts and
stops under the exact session generation; this remains portable evidence until
the live project-driver bridge consumes the tap.

Virtual-only multi-input routes now receive a tap-only output owner during
branch binding, so virtual capture sinks and recorders are not discarded when
no physical render endpoint is selected. Physical branches still require an
exact prepared render owner; the live project-driver bridge and external
application capture remain unqualified.

The shared preparation API now also exposes `nativeMultiInputs.prepare`,
returning the exact committed source and destination node order used for
virtual branch binding. It is generation-bound and stopped until explicit
session start; no endpoint substitution or implicit default selection occurs.

## 2026-09-16 - bounded mixer/fanout ring handoff

The engine now exposes `CompiledMixerFanoutGraph::process_to_rings` for the
next native adapter boundary. It performs one validated multi-input mix and
submits each physical-or-virtual branch to its own preallocated ring; a full
branch is dropped independently, and destination shape errors are rejected
before mixer mutation. The focused compiler/runtime regression and full engine
suite passed 1/1 and 115/115. This does not qualify loaded PortCls transport or
physical endpoint timing.

## 2026-09-18 - virtual-bus CLI acceptance refresh

`tests/acceptance/m03-virtual-buses.ps1` passed on the current tree, covering
the bounded desired-state and persistence lifecycle, including capacity,
route persistence, cycle/conflicting-writer rejection, and revision safety.
The run performed no native device provisioning, driver installation/loading,
or machine audio configuration.

## 2026-09-18 - virtual-bus desired-state acceptance refresh

`tests/acceptance/m03-virtual-buses.ps1` passed on the current tree. The run
revalidated eight-bus capacity, create/rename/enable/disable/delete lifecycle,
cross-session route persistence, cycle and conflicting-writer rejection, and
revision safety across separate CLI processes. It performed no native device
provisioning, driver installation/loading, or audio configuration action.

## 2026-09-17 - current desired-state lifecycle refresh

`tests/acceptance/m03-virtual-buses.ps1` passed on the current tree. The
acceptance covered bounded desired-state planning, persistence, lifecycle,
revision/conflict handling, and explicit unavailable managed-device
provisioning. No native device was provisioned, installed, loaded, or changed;
existing VB-Cable/Voicemeeter/physical endpoints remain the supported current
profile boundary.

## 2026-09-16 - direct fan-out supports virtual capture sinks

The engine’s bounded direct fan-out compiler now accepts both physical output
and virtual capture-sink destinations, each with its own validated channel
matrix. A regression covers one source feeding two branches with one virtual
sink; the engine suite passed all 115 tests and formatting passed. This is
portable graph/runtime evidence and does not qualify loaded driver transport,
virtual endpoint identity, or external application compatibility.

The full locked Rust workspace was requalified after this change; all crate
tests and doc-tests passed, including engine (115), control (173 plus 2
guarded-live ignores), Windows-audio (85), CLI/MCP (36 plus 3 stdio), and the
remaining suites. No managed driver or virtual endpoint was activated.

## 2026-09-16 - latest guarded fan-out after bridge contract correction

The elevated control-owned lifecycle was re-run with the exact existing
VB-Cable capture endpoint and the two explicit render branches. The 500 ms
run captured 24,000 frames, processed 187 quanta, rendered 23,936 primary
frames, and rendered 23,424 fan-out frames (`fanout_packets=183`), then
stopped cleanly. No persistent audio configuration changed. This is
user-mode VB-Cable fan-out evidence; managed AudioRouter PortCls transport,
virtual endpoint identity, production signing, and physical latency remain
open.

## 2026-09-16 - latest guarded multi-output native fan-out lifecycle

Command:
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m02-control-native-live.ps1 -AllowLiveAudio -CaptureEndpointId
'{0.0.1.00000000}.{06268191-5f8c-42ed-827e-d3c7a19637ed}' -RenderEndpointId
'{0.0.0.00000000}.{81a91c6d-531c-4b80-853a-af1f4ebf50de}'
-OutputFanoutEndpointIds
'{0.0.0.00000000}.{d31b2d50-0969-4fdf-8961-ad642e573743}'`

With elevated read-only Windows access, the 500 ms control-owned lifecycle
used the exact active VB-Cable capture and two virtual render branches. It
captured 23,520 frames, processed 183 quanta, rendered 23,424 primary frames,
and rendered 23,424 fan-out frames, then stopped cleanly. Process environment
was restored and no persistent audio configuration changed. This qualifies
user-mode VB-Cable fan-out only; managed PortCls transport, production
signing, physical latency, and clean-machine qualification remain open.

## 2026-09-16 - earlier guarded multi-output native fan-out lifecycle

Command:
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m02-control-native-live.ps1 -AllowLiveAudio
-DurationMilliseconds 500 -CaptureEndpointId
'{0.0.1.00000000}.{06268191-5f8c-42ed-827e-d3c7a19637ed}'
-RenderEndpointId
'{0.0.0.00000000}.{81a91c6d-531c-4b80-853a-af1f4ebf50de}'
-OutputFanoutEndpointIds
'{0.0.0.00000000}.{d31b2d50-0969-4fdf-8961-ad642e573743}'`

With elevated read-only Windows access, the control-owned lifecycle prepared
the exact VB-Cable capture endpoint and two explicitly selected virtual render
branches (`CABLE In 16ch` plus `Voicemeeter In 1`). The 500 ms run delivered
23,520 captured frames, 183 processed quanta, 23,040 primary rendered frames,
137 fan-out packets, and 17,536 fan-out rendered frames. Session stop and
explicit fan-out detach completed successfully; no endpoint default, volume,
mute, privacy, driver, signing, or persistent audio configuration changed.

An initial attempt using the canonical `CABLE Input` render endpoint returned
`AUDCLNT_E_DEVICE_IN_USE` during exact render initialization. The diagnostic
remained distinct and the qualification retried with other exact active virtual
render IDs; no endpoint was substituted automatically. This qualifies the
current user-mode multi-output adapter path, not the managed AudioRouter driver,
PortCls ownership, production signing, or physical latency.

## 2026-09-16 - endpoint invalidation survives read-only inventory

The control seam now retains endpoint changes observed by `devices.list` while
an endpoint worker is attached. The next mutating native pump consumes those
pending changes and fails closed on an exact capture/render binding; explicit
prepare/rebind clears observations resolved against the latest snapshot. This
closes the notification-loss case where inventory was queried before the
audio pump. The retention path filters to changes affecting the exact bound
worker, so unrelated endpoint churn cannot accumulate or cause invalidation.
Control (173) and Windows-audio (82) tests plus strict Clippy passed. No
endpoint or persistent machine audio configuration changed.

## 2026-09-13 - guarded VB-Cable bridge publication

Command:
`powershell.exe -NoProfile -ExecutionPolicy Bypass -File
tests/acceptance/m02-rust-adapter-bridge-live.ps1 -AllowLiveAudio
-DurationMilliseconds 500 -Cycles 1`

The runner discovered the existing VB-Cable render/capture pair, opened the
explicitly selected endpoints for one bounded cycle, and removed its temporary
native executable/object and recording output afterward. Telemetry was:
48 kHz, stereo, 50 packets, 24,000 captured frames, 187 processed quanta,
187 tap calls, zero non-finite tap samples, 23,936 rendered frames, zero
dropped render frames, zero scheduler XRuns, zero deadline misses, and a
25,072-byte temporary recording. Before/after `Get-PnpDevice -Class Media
-PresentOnly` snapshots were identical. The test changed no defaults, volume,
mute, privacy, driver installation, signing, or persistent audio state.

This qualifies the existing third-party VB-Cable user-mode bridge path for a
human-testable M02/M03 demo. It does not qualify AudioRouter's own driver,
PortCls callbacks, persistent endpoint lifecycle, or production signing.

The companion control-owned route check initially exposed a probe defect: the
probe explicitly started the endpoint after `session_start`, although the
attached-native session lifecycle already starts it. Removing that redundant
idempotent call restored the intended single-start assertion. The corrected
run passed with generation 1, 51 packets, 24,480 captured frames, 191
processed quanta, 24,448 rendered frames, 4,140 temporary recording bytes,
one start/one successful start, one stop/one successful stop, one reset, one
rejected stale-generation pump, and 48 kHz capture/render rates. Media-device
identity/state remained unchanged.

## 2026-09-08 - Far-future plan expiry cap

Restart hydration caps positive persisted virtual-device and startup-plan
lifetimes at the five-minute plan TTL. This prevents a corrupt or manually
altered far-future SQLite timestamp from producing an `Instant` overflow or an
effectively unbounded pending plan. Control tests (93), strict Clippy, and
formatting pass; no endpoint or driver is activated.

## 2026-09-08 - Durable plan expiry reload hardening

Control restart now uses a checked, strictly-positive remaining duration when
reconstructing persisted virtual-device and startup plans. An expired or
overflowing timestamp is ignored instead of being cast to `u64` and revived as
an effectively unbounded in-memory plan. The focused control suite passes 93
tests with strict Clippy and formatting; no endpoint or driver is activated.

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
At this historical managed-device checkpoint, the AudioRouter-owned
render/capture capabilities and endpoint IDs remained unavailable until the
managed driver existed, preventing callers from mistaking desired state for
provisioned Windows devices. This does not describe the separately qualified
existing VB-Cable/physical endpoint boundary used by the current profile.

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
## Virtual-bus persistence read bound (2026-09-08)

`Storage::load_virtual_buses` now uses a SQL limit of
`MAX_VIRTUAL_BUSES + 1` and fails with an explicit oversized-inventory error
before constructing a larger snapshot vector. A nine-row SQLite regression
verifies the boundary; storage passes 70 tests with strict Clippy and
formatting. This is persistence-boundary evidence only; no driver or endpoint
was activated.

## Virtual-device inventory page bound (2026-09-07)

The read-only `virtualDevices.list` contract now shares a 500-item page bound
across input validation, runtime truncation, and discovery schema metadata.
Control tests (86), strict Clippy, and documentation validation pass. This
does not create endpoints or claim managed-driver lifecycle functionality.

The legacy unpaged `virtualDevices.list` array response now advertises the
authoritative eight-bus domain ceiling. The in-memory registry already rejects
the ninth bus, so complete unpaged results remain safe without arbitrary
truncation or managed-driver activation.

## Leased-bus disable safety (2026-09-08)

`VirtualBusRegistry::set_enabled(false)` now rejects a bus with an active
writer lease using `VirtualBusError::Owned`. The caller must release the
current generation or perform an explicit recovery force-release before
disabling the bus; deletion retains its separate disabled and ownership guards.
The domain suite passes 53 tests with strict Clippy, formatting, and diff
checks. This is portable ownership evidence only; no driver or endpoint was
activated.
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
# M03 virtual routing evidence

## 2026-09-17 - current virtual-bus lifecycle refresh

`tests/acceptance/m03-virtual-buses.ps1` passed the current CLI desired-state
and persistence gate: bounded create/rename/enable/disable/delete lifecycle,
eight-bus capacity enforcement, persisted three-route cross-session topology,
cycle/conflicting-writer/stale-revision rejection, and explicit
managed-driver-unavailable reporting. The temporary database was removed.
This qualifies the portable desired-state and existing-device routing boundary;
it performs no native device provisioning, installation, loading, or audio
configuration action.

## 2026-09-16 - three-route reference CLI acceptance

`tests/acceptance/m03-virtual-buses.ps1` now applies a three-route
cross-session fixture for Desktop In, Voice Chat, and Monitor buses, then
lists the routes from a separate CLI process and verifies all three stable bus
identities. The same run retains cycle, conflicting-writer, stale-revision,
eight-bus capacity, and disabled-before-delete checks. Acceptance passed.
This proves desired-state and persistence routing only; no native device,
driver, installation, loading, or persistent machine audio configuration was
accessed.

## 2026-09-16 - exact endpoint invalidation policy

The Windows endpoint worker now retains the exact capture/render endpoint IDs
used to open its clients and exposes a control-thread policy that marks the
worker affected when either endpoint is removed or its metadata changes.
Unrelated additions and process-loopback changes do not invalidate the
worker. This prevents an unchanged opaque ID from being treated as a valid
binding after a format or state transition; deliberate stop, refreshed exact
binding validation, and rebind remain the caller's responsibility. The
focused Windows-audio suite passed 82 tests and strict Clippy passed. No
endpoint was opened by this change and no driver or machine audio
configuration changed.

The control plane applies that policy at the mutating native pump boundary,
not in read-only `devices.list`: a running affected worker is stopped and its
staged bridge audio is reset before `devices.bindingInvalidated` is published.
Replacement selection and reopening remain explicit operations. Control (171,
with two guarded-live ignores), Windows-audio (82), strict Clippy, and diff
checks passed. This is portable/control lifecycle evidence; loaded-driver
callback ownership and production rebind timing remain open.

## 2026-09-16 - guarded VB-Cable bridge requalification

Command: `tests/acceptance/m02-rust-adapter-bridge-live.ps1 -AllowLiveAudio
-DurationMilliseconds 750`. The explicitly selected existing VB-Cable pair
completed one 750 ms cycle at 48 kHz stereo with 36,480 captured frames,
36,480 rendered frames, 285 processed quanta, zero non-finite samples, drops,
xruns, or deadline misses, and a 25,072-byte temporary recording. The harness
stopped and removed temporary streams/files and verified media-device state was
unchanged. This is existing user-mode VB-Cable evidence only; the managed
AudioRouter driver, loaded PortCls transport, signing, and physical-latency
gates remain open.
# M03 virtual-routing evidence

## 2026-09-17 - adapter-bridge endpoint-selection resolution

The initial explicitly selected DELL/P32p-30 physical-render retries returned
`adapter_bridge_error=audio frame size was invalid` before telemetry because
those IDs were absent from the probe's current active inventory. Re-running
`m02-rust-adapter-bridge-live.ps1 -AllowLiveAudio` with the exact current CABLE
Output/CABLE Input pair passed two 500 ms cycles: 24,480 and 24,960 captured
frames, 191 and 195 processed quanta, 24,448 and 24,960 rendered frames, zero
drops/xruns/deadline misses, and 25,072-byte finalized temporary recordings.
All temporary resources were removed and media-device state was unchanged.
The physical-monitor pair remains unqualified by this probe because it is not
present in its current active inventory. The probe now emits an explicit
`adapter_bridge_diagnostic=endpoint_not_present` record before retaining the
stable legacy HRESULT text, so future endpoint-availability failures are
distinguishable from frame-shape failures.

## 2026-09-16 - bounded mixer/fanout ring handoff

The engine now exposes `CompiledMixerFanoutGraph::process_to_rings` for the
next native adapter boundary. It performs one validated multi-input mix and
submits each physical-or-virtual branch to its own preallocated ring; a full
branch is dropped independently, and destination shape errors are rejected
before mixer mutation. The focused compiler/runtime regression and full engine
suite passed 1/1 and 115/115. This does not qualify loaded PortCls transport or
physical endpoint timing.
