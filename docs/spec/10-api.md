# 10 — Public local API and transaction contract

Milestone ownership: M01 protocol/discovery/transactions; M02–M06 extend node/resource capabilities; M07 event/retry/parity hardening. No feature may wait until M07 to acquire its basic API.

## Transport and schemas

The baseline application protocol is JSON-RPC 2.0 over a local Windows named pipe. The pipe is scoped to the authenticated Windows user/logon context, rejects remote clients, and uses explicit ACLs. Desktop shell, CLI, and MCP adapter use the same endpoint and generated client contracts. No public HTTP listener is required for v1. An eventual standalone-browser adapter must forward the same methods and pass the security gate; it is future scope.

Use UTF-8 messages framed by a 4-byte unsigned little-endian length, maximum 4 MiB, then exactly that many bytes. Reject malformed/oversized frames before allocation. JSON-RPC batches are supported with at most 32 requests, processed independently in supplied order; a batch is not a graph transaction. Mutating notifications without request IDs are rejected at the application boundary because callers need a result. Document this restriction in protocol discovery. [JSON-RPC 2.0](https://www.jsonrpc.org/specification) provides request/result/error semantics; framing and methods here are AudioRouter decisions.

M01 shall produce machine-readable method and node schemas, golden request/response fixtures, a versioned generated TypeScript client, Rust types, and readable reference documentation. This file is the design contract, not a substitute for those generated schemas.

## Requirements

- **API-01 — Discovery.** `system.describe` returns protocol/schema versions, build, supported methods with input/output JSON Schemas, permission scopes, node registry versions, limits, event types, and unsupported capability reasons. Include descriptions, units, ranges, defaults, examples, and side-effect classification. Discovery is usable offline before any session exists.
- **API-02 — Identity/versioning.** Resources use UUID-like opaque IDs; clients never resolve an ambiguous label silently. A handshake negotiates major/minor API versions. Unknown major versions fail without mutation; additive minor fields are tolerated. Persisted session schema version is independent of transport version.
- **API-03 — Reads.** Queries provide authoritative snapshots with config revision, active runtime revision, generation, timestamp, and stale/availability indicators. List methods accept cursor pagination with default 100/max 500 records. Route inspection reports actual desired and running paths where activation is pending.
- **API-04 — Graph planning.** `graph.plan` takes a session ID, base revision, ordered typed operations, and proposed client-created IDs. It validates the complete candidate, resolves capabilities, and returns a plan ID, canonical diff, dependencies/assumptions, permission requirements, affected routes, warnings, estimated latency, and expiry. Planning may inspect resources but shall not capture audio, create files, install drivers, or activate the graph.
- **API-05 — Commit.** `graph.commit` supplies plan ID, base revision, idempotency key, and acknowledgments for specific warning IDs if required. Recheck revision, device generation, capabilities, and permissions. A plan expires after five minutes or relevant resource changes. One accepted commit produces exactly one session revision; conflicting clients receive `revisionConflict` and latest revision, never last-writer-wins overwrites.
- **API-06 — Atomicity boundaries.** Graph topology/parameter edits commit together. Driver lifecycle, app selections, recording start/stop, file operations, and startup registration are explicit operation resources outside graph atomicity. A plan that needs them returns ordered prerequisite operations; it cannot pretend an OS install or file write is reversible by graph rollback.
- **API-07 — Durable outcomes.** Every mutation has an idempotency key scoped to authenticated client and method. Store request hash and outcome for at least 24 hours; same key/different payload returns `idempotencyConflict`. After a transport timeout, `operations.get` or a same-key retry reveals the prior outcome. Long operations return an operation ID with stage/progress/cancellation limits.
- **API-08 — Events.** Support `events.subscribe` with session/resource filters, event categories, and after-sequence cursor. State events have increasing sequence, backend epoch, resource revision, and operation ID. Retain 10,000 state events or 15 minutes, whichever bound is reached first. A lost cursor returns `resyncRequired`; clients fetch a snapshot. Meter events are lossy/coalesced and not replayed.
- **API-09 — Errors.** Errors have a stable application code, readable message, JSON field path, relevant resource IDs, retryability, and suggested remediation. No raw filesystem secrets or stack traces in normal responses. Distinguish unsupported, unavailable, access denied, conflict, invalid, budget exceeded, failed, and canceled.
- **API-10 — Undo.** Store enough bounded history for at least 100 graph commits per session. Undo/redo produces a new revision through plan/commit and validates the current base. Show the inverse diff, account for intervening changes, and refuse ambiguous inversion. Graph undo cannot restore deleted files or undo an external application setting.
- **API-11 — Control/telemetry load.** Rate-limit each client to 20 mutation requests/second with bounded bursts of 40 and `rateLimited` retry hints. Coalesce slider updates client-side. Limit meter subscriptions to 30 updates/second per client. Slow clients lose metering first; state-event overflow triggers resync rather than unbounded queues.
- **API-12 — No private feature paths.** Device provisioning, recorder actions, plugin scans/state, startup, emergency mute, imports, and all graph edits have documented methods. The shell may pick a file or display OS dialogs; resulting authorized paths/choices go through the backend. Automated parity tests compare all adapters against the same fixtures.

## Minimum method families

| Methods | Contract and important result |
| --- | --- |
| `system.describe`, `system.status`, `system.diagnostics` | Discovery; backend/audio health; redacted diagnostic report |
| `clients.list`, `clients.authorize`, `clients.revoke` | Local enrollment/grants; authorization only by an already authorized owner context |
| `devices.list`, `applications.list` | Paged endpoint/application descriptors and binding candidates |
| `sessions.list/get/create/duplicate/delete` | Versioned session resources; create stopped; delete refuses active resources unless a separate stop completed |
| `sessions.start/stop` | Idempotent lifecycle operations with per-node/recorder outcomes |
| `sessions.export`, `sessions.importPlan`, `sessions.importCommit` | Portable bundle and stopped import with rebind report |
| `graph.plan/commit`, `graph.history`, `graph.undoPlan` | Atomic graph edit/inspection/revision history |
| `routes.inspect`, `nodes.describe` | Source provenance and complete type/parameter schemas |
| `virtualDevices.list/plan/apply` | Bus lifecycle as explicit privileged operations, separate from graph commits |
| `plugins.scan/list/inspect`, `plugins.retry` | Isolated discovery, compatibility/quarantine, deliberate retry |
| `presets.list/save/import/export` | Versioned parameter/subgraph presets; apply through graph.plan |
| `recorders.arm/start/pause/resume/split/stop` | Recording state changes with exact frame/file identities |
| `recordings.list/get/rename/setMetadata/reveal/preview/removeEntry/recycle` | Independent library/file actions and permission scopes |
| `startup.get/plan/apply` | Sign-in behavior preview and registration result |
| `safety.setPrivacyMute` | Immediate source suppression, then durable latch update |
| `operations.get/cancel` | Persistent operation status; cancellation cannot undo completed side effects |
| `events.subscribe/unsubscribe` | State and meter event stream |

`graph.plan` operation types minimally include `node.add`, `node.remove`, `node.rename`, `node.setEnabled`, `node.setBypass`, `node.setParameter`, `node.setBinding`, `edge.add`, `edge.remove`, `edge.setEnabled`, `edge.setMatrix`, and `session.setPolicy`. UI layout uses a separate presentation resource/revision so dragging does not cause audio revision conflicts.

## Worked request

The following uses abbreviated IDs only for readability. Discovery returns the actual parameter/type identifiers. Gain changes also use plan/commit; no adapter writes node state directly.

```json
{
  "jsonrpc": "2.0",
  "id": "request-41",
  "method": "graph.plan",
  "params": {
    "sessionId": "session-gaming",
    "baseRevision": 12,
    "operations": [
      {
        "op": "node.setParameter",
        "nodeId": "mic-monitor-gain",
        "parameter": "gainDb",
        "value": -12
      }
    ]
  }
}
```

```json
{
  "jsonrpc": "2.0",
  "id": "request-41",
  "result": {
    "planId": "plan-42",
    "baseRevision": 12,
    "expiresAt": "2026-09-05T20:05:00Z",
    "diff": [{"path": "/nodes/mic-monitor-gain/parameters/gainDb", "before": 0, "after": -12}],
    "affectedDestinations": ["headphones"],
    "warnings": [],
    "requiredScopes": ["graph.write"]
  }
}
```

Commit adds `idempotencyKey`, references `plan-42`, and returns `operationId`, `revision:13`, and activation status. The UI must not call the route changed until `graph.activated` or a completed operation confirms it.

## Configuration and runtime crash consistency

For a running session, prepare candidate resources, journal the candidate durably, publish the runtime generation at a safe block boundary, and durably mark the revision committed before reporting success. The journal distinguishes `prepared`, `activated`, and `committed`. On crash during this boundary, resolve the last durable committed revision on restart and expose the interrupted operation as rolled back/failed; never replay a recording start merely because its graph revision exists. A client must be able to determine whether an unacknowledged request committed.

Retain the previous runtime generation until the durable commit succeeds. If final persistence fails while the process remains alive, restore that generation at a block boundary, publish an activation-failed event, and return failure. A short-lived candidate may already have affected live sound; transaction atomicity does not undo audio already emitted. Configuration readers distinguish pending activation from durable commitment throughout this interval.

If activation cannot finish within two seconds, retain the old graph and return a timed operation/failure, cleaning prepared resources off-thread. Multi-device output transitions are not guaranteed to change at the same wall-clock nanosecond; atomicity refers to coherent graph generation and bounded transition, not hardware-clock simultaneity.

## Application error vocabulary

Use JSON-RPC standard numeric errors for parse/invalid request/method/params failures. Application errors use server-error numeric codes plus `data.code`, including `revisionConflict`, `planExpired`, `capabilityChanged`, `permissionDenied`, `deviceUnavailable`, `ambiguousBinding`, `feedbackCycle`, `unsupportedFormat`, `pluginUnavailable`, `resourceConflict`, `budgetExceeded`, `diskFull`, `resyncRequired`, `restartRequired`, `idempotencyConflict`, and `rateLimited`. M01 freezes numeric mappings in schemas and golden fixtures; clients branch on stable codes rather than English messages.
