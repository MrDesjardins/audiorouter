# M01 contracts and control-plane evidence

## Scope and boundary

This report covers the portable M01 foundation implemented while M00 Windows capture, process-loopback, physical-latency, and managed-driver gates remain blocked. It does not claim a Windows named pipe, real audio activation, driver provisioning, or realtime behavior.

## Implemented components

- `crates/domain`: opaque IDs; session/node/port/edge contracts; graph limits; direction/channel/matrix/dangling/duplicate/cycle validation; node registry; fake runtime generation lifecycle; revision-checked plans; idempotent commit replay; plan expiry.
- `crates/control`: discovery metadata; session reads; graph plan/commit; storage-backed construction; fake session start/stop; JSON-RPC dispatch; batch handling; scoped grants; explicit role enrollment/revocation lookup; notification semantics.
- `crates/protocol`: 4-byte little-endian framing, 4 MiB maximum frame, JSON-RPC request/response types, 32-request batch limit, malformed/version/method validation.
- `crates/storage`: SQLite schema migration, transactional session JSON/history writes, validated bounded import/export, safe staged ZIP bundle import with optional asset size/SHA-256 verification, online SQLite backups, client enrollment/revocation records, and idempotent operation journal.
- `crates/cli`: offline discovery commands plus validated persistent JSON and `.audiorouter` bundle import/export commands with human and `--json` output.
- `contracts`: strict TypeScript JSON-RPC/domain contract package with pinned compiler and shared session/node/edge/discovery/response types.
- `tests/fixtures/valid-session.json`: checked-in camelCase contract fixture.
- `tests/acceptance/m01-cli.ps1`: offline CLI acceptance script.

## Reproducible checks

From the repository root:

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo check --manifest-path tools/m00-wasapi-probe/Cargo.toml
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\tests\acceptance\m01-cli.ps1
npm --prefix contracts run typecheck
```

On 2026-09-05, the workspace suite passed 4 CLI, 16 control, 12 domain, 5 protocol, 14 storage, and 13 transport tests. The standalone WASAPI probe checked successfully. The CLI acceptance script now exercises a bundle round trip and reported `M01 CLI acceptance passed`; the strict TypeScript contract package passed `npm run typecheck`. The script-policy bypass was process-scoped; Windows execution policy was not changed.

## Requirement evidence

Portable evidence supports the domain/control portions of ARCH-01/03/06/10/12, GRAPH-01/02/03/05/06/07/08/09/12/13, API-01/02/03/04/05/06/09/10/11, AUTO-02/03/04/09/10/11/12, and ENG-01/02/04. Storage and fake lifecycle are foundations for STATE and persistence requirements, not final crash-recovery proof.

Still not evidenced: required-node-type/type-version compatibility beyond the frozen v1 manifest boundary; real endpoint discovery/activation in the control plane; process-tree audio data capture; driver lifecycle; realtime callback safety; physical latency; and M02 hardware acceptance. M00 remains open and M01 is not a releasable product gate.

The read-only `routes.inspect` method now validates a session and returns all enabled desired upstream paths to a requested destination as ordered node and edge IDs. Disabled edges produce an unreachable destination path, and unknown destinations return a path-specific validation error. This is desired-topology provenance only; it does not claim a running graph or physical audio reachability.

Each inspected path now also includes the validated `channelMaps` in edge order. This keeps mono/stereo conversion visible to clients instead of inferring or hiding it; the current fixture verifies the identity map, while live adapter conversion remains an M02 concern.

The CLI exposes the same read-only inspection through `routes inspect <session-id> <destination-node> --database <absolute-path>`. The M01 acceptance script imports the checked-in fixture into a temporary database, inspects the output route, verifies reachability and channel-map presence, and then removes all temporary files.

`graph.history` is now advertised as a read-only method with a bounded `limit` (default 100, maximum 500). The domain store retains at least the newest 100 in-memory snapshots, and the control method uses durable SQLite history when the session is not loaded in memory. Tests verify newest-first ordering and bounded retrieval; undo planning and event replay remain separate plan items.

`graph.undoPlan` now creates a normal expiring graph plan from the immediately prior retained snapshot, requiring the caller's current base revision and GraphWrite permission. It never commits automatically; a caller must review/acknowledge and invoke `graph.commit`. No prior snapshot returns `NoUndoAvailable`, while intervening revision conflicts are checked by the same plan/commit authority.

The portable `EventLog` now establishes bounded state-event replay semantics: every event carries backend epoch, sequence, resource revision, optional operation ID, category, and optional session ID. Replay is limited to 500 records and returns `ResyncRequired` when retention has passed the requested cursor. It excludes meter data and is not yet connected to transport subscriptions.

Control now owns an event log and exposes `events.subscribe` with optional `afterSequence`, `limit`, and `sessionId` filters. Session creation and graph commits append state events; the control test verifies epoch, ordering, operation ID, and filtering. Transport-level subscriber lifetime and snapshot resync responses remain open.

`nodes.describe` is now a documented read-only method and `nodes describe` is available in the CLI. Both expose the same registry entries as `nodes.types`, including availability and realtime cost class; the M01 acceptance script verifies equal entry counts while real device/plugin schemas remain future milestone work.

The CLI now exposes persisted revision history through `history <session-id> --database <absolute-path> [--limit N]`. It reads the bounded SQLite history newest-first, rejects limits outside 1–500, and is covered by the M01 acceptance round trip using temporary files.

The in-memory graph store now drops the oldest snapshot once its 100-entry retention bound is exceeded. A regression test inserts 101 snapshots and verifies that revisions 1–100 are retained newest-first (revision 0 is discarded), matching the documented undo/history budget.

The CLI now supports `session start|stop <session-id> --database <absolute-path>`. The control plane lazily loads the persisted session and runs the deterministic fake lifecycle, returning `runtime: "fake"`; the acceptance script verifies running then stopped state and cleans up its temporary database. This is not evidence of live audio activation.

Read parity now includes `sessions.get` and `session get <session-id> --database <absolute-path>`. The CLI acceptance round trip reads the imported fixture and verifies its ID/revision before starting the fake runtime, keeping persisted configuration inspection separate from external audio activation.

Session resource listing now includes `sessions.list` and `session list --database <absolute-path> [--limit N]`. Storage returns sessions in stable ID order with a 1–500 bound; the acceptance script verifies the imported fixture is listed. Cursor pagination and create/duplicate/delete operations remain future API work.

`graph.plan` now returns a deterministic preview containing changed `/name`, `/nodes`, or `/edges` entries, physical-output destinations, warnings, required scope, and expiry duration in addition to its plan ID. The control test verifies the diff and scope fields; capability resolution, external-resource activation, and full typed-operation semantics remain open.

`graph.commit` now re-prepares and advances the fake runtime when its session is already running, returning an `activation` object with the new fake generation; commits for stopped sessions report pending fake activation. The control regression verifies generation 1→2 after an online edit. This proves only deterministic lifecycle wiring, not live endpoint resource activation or atomic native graph publication.

`graph.undoPlan` now hydrates the latest bounded SQLite history when a fresh control plane lacks prior in-memory snapshots. A restart regression creates and commits revision 1, opens a new control plane, and successfully creates the undo plan against base revision 1. Runtime state and external audio remain untouched.

In-memory graph idempotency now binds each committed key to its original plan ID. A repeated commit of the same plan replays the original result, but reusing that key for another plan is rejected as an idempotency conflict. The durable journal still needs request-hash validation and bounded expiry across process restarts.

Bundle manifests now optionally carry `requiredNodeTypes` entries with a type name and version. Export derives the list from the session, and import rejects unknown or mismatched versions before writing the session; the existing v1 fixture remains compatible. The storage suite passed 15 tests with strict Clippy. The rebuilt CLI test executable was blocked from launch by the host Application Control policy (OS error 4551), so no CLI result is claimed for this slice.

The SQLite operation journal now persists a request hash and exposes checked replay lookup. Matching hashes replay the stored result; a reused key with a different hash returns `IdempotencyConflict`. The migration preserves existing databases with an empty legacy hash, so control integration must treat legacy rows conservatively. Storage tests passed 16 cases with strict Clippy.

The domain store now enforces the specification's global caps of 128 nodes and 256 edges across sessions, subtracting the replaced session before checking a new version. `system.describe` and the TypeScript discovery contract expose both global limits. Regression coverage passes 20 domain and 24 control tests with strict Clippy, and contract typecheck is green.

Control mutations now checkpoint the in-memory `GraphStore` before changing it. A failed durable session save or graph journal transaction restores the checkpoint, so a storage error cannot leave memory ahead of SQLite. The affected suites pass 24 control and 20 domain tests with strict Clippy.

The request-hash migration is now tested against a pre-hash SQLite schema. Existing journal rows receive an empty legacy hash and replay only for an explicitly empty hash; a new hash returns `IdempotencyConflict`. The temporary migration database is cleaned up, and storage coverage is 18 passing tests with strict Clippy.

History restoration now applies the global 128-node/256-edge budgets after hydration and restores the prior `GraphStore` checkpoint if the aggregate exceeds a cap. A regression uses two valid full sessions plus a third restored session and verifies no partial state remains; 21 domain and 24 control tests pass with strict Clippy.

The fake/control lifecycle now enforces the two-active-session specification limit. Repeated start of an existing running session remains idempotent, while a third distinct session is rejected; discovery and the TypeScript contract expose `maxActiveSessions`. Domain/control tests pass (21/25), strict Clippy is green, and the contracts typecheck passes. This is a deterministic control-plane bound and does not claim native device-wide enforcement.

Runtime stop events now publish the stopped session's current resource revision rather than zero. The lifecycle regression verifies the event metadata, and the control suite passes 25 tests with strict Clippy.

CLI list dispatch now validates `nodes types|describe` explicitly. Invalid subcommands return `InvalidArguments` rather than triggering the former unreachable panic path; the CLI suite passes 4 tests with strict Clippy.

Authenticated control dispatch now enforces the API mutation budget with a per-client token bucket: 40 initial tokens, refilling at 20 requests per second. Rejected requests return JSON-RPC server error `-32000` with `{ "code": "rateLimited", "retryAfterMs": ... }`; Windows transport wiring keys the bucket by the authenticated client SID. Deterministic bucket and end-to-end response coverage pass, with control/transport verification at 27/14 tests and strict Clippy green. Meter-subscription throttling and production daemon lifecycle remain open.

`EventLog` now enforces both retention dimensions from API-08: at most 10,000 state events and no more than 15 minutes of monotonic age. Expired entries are removed on append and before replay, so an old cursor returns the existing explicit resync response without idle logs retaining stale entries. A deterministic expiry regression passes; domain/control verification is 22/27 tests with strict Clippy.

`sessions.list` now accepts an opaque stable-ID cursor and returns bounded `{ items, nextCursor }` pages. The in-memory and SQLite paths share the same ascending-ID semantics, and the existing internal array helper remains available for resync snapshots and CLI compatibility. Cursor regressions pass in control and storage; contract typecheck remains green.

`graph.history` now accepts a revision cursor and returns bounded `{ items, nextCursor }` pages in newest-first order. Both in-memory and SQLite history use one-record look-ahead to avoid advertising a cursor after the final page; control/storage/domain verification passes with strict Clippy.

The session resource lifecycle now includes `sessions.create` and `sessions.delete`. Create accepts only a validated revision-0 stopped session; delete transactionally removes current and history rows, clears in-memory plans/runtime state, refuses active sessions until stopped, emits `session.deleted`, and is available through the CLI. Contract, domain, storage, control, and CLI coverage passes (29/20/23/4 tests) with strict Clippy and TypeScript typecheck green.

`sessions.duplicate` now clones a source session through the backend into a new revision-0 stopped resource, permits an explicit replacement name, and rejects an existing destination ID. Control and CLI regressions cover cloning and collision handling; the affected suites pass (29 domain, 20 storage, 23 control, 4 CLI) with strict Clippy and contract typecheck green.

The CLI now exposes `session create <absolute-document> --database <path>` for the same backend-owned create operation. A temporary revision-0 fixture is created and validated against SQLite, then removed with its database; the CLI lifecycle coverage remains green with strict Clippy.

Application JSON-RPC failures now include stable `error.data.code` categories for domain, storage, permission, and internal failures, including `revisionConflict`, `planExpired`, `notFound`, and `idempotencyConflict`. Mutating notification rejection is applied consistently to session create/duplicate/delete as well as graph/session operations; a revision-conflict regression passes in the 30-test control suite with strict Clippy.

`graph.commit` now hashes the planned session payload with SHA-256, checks the durable journal before domain mutation, and stores the hash in the same SQLite transaction as the session revision. Matching requests replay across a fresh control plane; mismatched reuse is rejected. The combined domain/control/storage verification passed 19, 22, and 16 tests respectively, with strict Clippy green. Expiry/retention policy for journal rows remains future work.

Durable idempotency records now have a 24-hour retention bound. Journal reads and transactional writes prune older rows using an indexed `created_at`; a regression verifies an expired key can be reused while a current mismatched hash still conflicts. Storage coverage is 17 passing tests with strict Clippy.

The durable commit hash is derived from the operation, plan ID, and base revision rather than requiring the candidate to remain in memory. This permits replay from a fresh control process before plan lookup; a regression verifies the persisted result is returned without a second mutation. The affected suites pass 23 control, 19 domain, and 17 storage tests with strict Clippy.

Status correction for the earlier coverage summary: subsequent native evidence now establishes process-loopback include/exclude activation and data reads. Controlled per-process tone attribution, physical latency, driver lifecycle/signing, and full M02 hardware acceptance remain open; the portable M01 tests do not claim those results.

Discovery method descriptions now include JSON-Schema-style `inputSchema` and `outputSchema` fields. Required identifiers, graph commit/plan fields, and pagination bounds are described for clients, while read-only array results are distinguished from object results. Rust discovery regression and TypeScript contract typecheck pass; runtime schema enforcement and generated client validation remain future work.

Each discovered method now also carries a human-readable `description`, keeping purpose, permission, side-effect class, and machine-readable schemas together in the offline contract. The control discovery regression and TypeScript typecheck pass.

The control dispatcher now enforces the published request-object boundaries: array/scalar params and unknown properties are rejected as `-32602` invalid parameters. This prevents clients from receiving success after sending fields that the discovery schema does not define; the control suite passes 32 tests with strict Clippy.

The read-only `system.handshake` method now negotiates protocol major/minor versions independently of persisted session schema. Minor versions are additive-compatible with the current implementation, while an unknown major is rejected before method dispatch; control/domain tests and strict Clippy pass.

Application failures now publish stable structured metadata alongside their numeric JSON-RPC code: `fieldPath`, affected `resourceIds`, `retryable`, and a remediation hint. Rate-limit failures additionally publish `retryAfterMs`; the TypeScript contract models these fields and control/protocol tests pass.

The contracts package now includes a versioned transport-agnostic `createAudioRouterClient` surface covering all currently implemented methods. It maps method names to typed parameters/results, allocates request IDs, omits absent parameters, and raises `AudioRouterRpcError` with server metadata. `npm run typecheck` passes; native transport wiring remains separate.

`system.describe` now advertises state-event categories, the fact that meter events are not replayed, and the event log’s 10,000-event/15-minute retention bounds. A discovery regression confirms these values against the event-log implementation, with control/domain tests, strict Clippy, and contract typecheck green.

The CLI now exposes the generic `api call` path, reading bounded JSON parameters from an absolute file or stdin and routing through the same control dispatcher, with optional SQLite-backed state and JSON-RPC response envelopes. The source compiles and passes strict Clippy; the rebuilt CLI test executable was blocked by Windows Application Control (OS error 4551), so no runtime CLI result is claimed for this slice.

The API now exposes the specification’s canonical `applications.list` method while preserving `apps.list` for compatibility. Both dispatch to the same authoritative application discovery implementation, and a regression confirms identical results.

Node discovery now includes parameter descriptors for implemented built-in processors: Gain exposes `gainDb` in the bounded -60 to 12 dB range with a zero default, and Mute exposes the boolean `muted` parameter. The TypeScript contract models these descriptors. Source compilation and strict Clippy pass; runtime control regression execution is blocked by Windows Application Control (OS error 4551), so this slice does not claim a fresh executable test pass.

Checked-in golden fixtures now cover a protocol handshake request and exact response. The control regression deserializes both fixtures and compares the actual dispatcher response, providing a parity anchor for future CLI/UI/MCP adapters. Compilation and strict Clippy pass; execution remains blocked by the host Application Control policy.

Optional nullable `cursor` and duplicate `name` fields now honor the published schema by treating explicit `null` as omitted rather than returning an invalid-params error. The regression is compile-validated with strict Clippy; executable control tests remain blocked by host Application Control.

The specified read-only `system.diagnostics` method is now available and included in discovery/client mappings. Its response reports only redacted control-plane, storage mode, audio capability, and event-log counters; a regression verifies that no path field is exposed. Compilation, strict Clippy, and contract typecheck pass.

The client enrollment API is now public: `clients.list` returns stable sorted `{clientId, role, revoked}` records, while `clients.authorize` and `clients.revoke` use the explicit `deviceAdministration` scope and preserve durable revocation state in SQLite. Storage runtime coverage passes 20 tests; control execution is subject to the host Application Control limitation when rebuilt.

Canonical plural `sessions.start` and `sessions.stop` methods are now advertised and dispatch through the same lifecycle implementation as the legacy singular aliases. Their control authorization, mutation limits, and notification rules are shared; compilation, strict Clippy, and contract typecheck pass.

`operations.get` now exposes durable outcomes from the SQLite journal by idempotency/operation ID, including completed status, operation name, committed revision, creation time, and parsed result. A control regression covers a committed graph outcome; storage runtime tests pass 20 tests, while rebuilt control execution remains subject to Application Control.

The same method now returns in-process graph-commit outcomes when the backend is memory-only, with a bounded 100-entry cache and an explicit `durable:false` marker. SQLite outcomes remain durable and are preferred when available; compile and lint validation pass.

Added the first repository CI workflow with separate portable and Windows
jobs. Ubuntu runs locked workspace tests, strict Clippy, formatting, contract
typechecking, and UI tests/build; Windows runs the non-hardware workspace test
set, compiles the Windows audio adapter, and runs strict Clippy. Hardware
endpoint tests remain deliberately separate so CI does not claim audio
availability or mutate machine configuration.

`status.get` now returns a dynamic control-plane snapshot rather than only static capability text: storage mode, loaded session count, active runtime IDs/count, and event cursor are reported while unavailable audio remains explicit. The TypeScript contract models the fields and compile/lint checks pass.

Status session counts now come from the durable SQLite table when a storage-backed control plane is used, avoiding false zero counts before lazy hydration; active IDs are sorted for deterministic output. Storage runtime tests pass 20 tests and compile/lint checks remain green.

When an event cursor falls outside retained history, `events.subscribe` now returns an explicit resync result containing `resyncRequired`, the backend epoch/current sequence, and a bounded current session snapshot. A control regression covers the expired-cursor path; 24 control tests pass with strict Clippy. Transport subscriber lifetime and reconnect ownership remain open.

The native transport now exposes a bounded persistent session API and a control-plane adapter. One authenticated named-pipe connection can carry a fixed number of framed requests before deterministic disconnect; the Windows transport suite passed 14 tests, including same-connection multi-frame exchange, with compile and strict Clippy green. The API remains bounded and does not claim an unbounded production daemon.

On 2026-09-06, the checked-in `tests/acceptance/m01-cli.ps1` was rerun with
the current branch and reported `M01 CLI acceptance passed`. It exercised the
offline discovery, fixture import/export, persisted session inspection and
fake lifecycle paths using temporary data; no audio endpoint or machine
configuration was changed.

## Next action

Implement application-loopback data-path validation and controlled process attribution in the native Windows probe. Preserve the existing no-default-change policy and keep driver installation/signing out of scope until isolated target and signing evidence exists.
## 2026-09-06 — Current acceptance rerun

The checked-in `tests/acceptance/m01-cli.ps1` completed successfully against
the current branch. It verified discovery, persisted session lifecycle, route
inspection, history, and bundle round-trip behavior using temporary files.
The M08 release artifact verifier and qualification-command documentation
regressions also passed. Temporary state was removed; no audio endpoint,
driver, or machine configuration was changed.
The discovery output schema for `nodes.types` and `nodes.describe` now explicitly
models node type identifiers, availability status/reasons, realtime cost class,
and parameter descriptors. This matches the serialized control response rather
than advertising only an unstructured array. The control suite passes 55 tests
with strict Clippy; the change is transport/domain metadata only.

The `clients.list` output schema is also explicit: each enrollment record has a
non-empty client ID, one of the built-in roles, and a boolean revocation state.
The schema correction is covered by control discovery tests and does not alter
enrollment records.

The `status.get` output schema now models the stable snapshot returned by the
control plane, including capability strings, session counters/IDs, privacy and
recovery state, and event cursors. The schema is covered by control discovery
tests and remains a read-only contract.

The `system.diagnostics` output schema now explicitly models its redacted
backend, unavailable-audio, privacy/recovery, and event-log fields. Discovery
tests verify the redaction marker and bounded counters; the operation remains
read-only.

The fixed `startup.get` capability response and `recovery.clearSafeMode` result
now have explicit output schemas and shared TypeScript types. Their
unavailable/cleared-state invariants are discovery-tested without registering
startup or mutating recovery state.

The paged `sessions.list` and `graph.history` responses now advertise explicit
page envelopes and serialized session snapshots, including graph nodes, ports,
edges, and channel matrices. Discovery assertions cover the cursor and revision
fields; the change is contract-only and does not mutate sessions.
