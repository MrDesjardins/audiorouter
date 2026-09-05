# M01 — Domain, local API, storage, and CLI foundation

Status: not started. Prerequisite: M00 decisions and gate evidence. Outcome: a headless, inspectable control system operating a deterministic fake engine.

## Read first

[Architecture](../spec/03-architecture.md), [graph](../spec/04-graph.md), [API](../spec/10-api.md), [automation](../spec/11-automation.md), [persistence](../spec/12-persistence.md), [security](../spec/13-security.md), and [quality](../spec/14-quality.md).

## Ordered implementation

1. Create only the domain/contracts/control/CLI and fixture components needed here. Pin toolchains and lockfiles; establish Windows CI plus portable checks where useful.
2. Define session/node/edge/device-binding schemas, node registry metadata, revisions, bounded graph compiler, and explicit channel matrices. Implement cycle/dangling/duplicate/arity/limit validation and readable field-path errors.
3. Implement per-user singleton backend, restricted named pipe, framed JSON-RPC, version negotiation, client enrollment/grants, and method discovery. Add a test owner client without weakening shipping authorization.
4. Implement SQLite storage, journaled revisions, operations/idempotency, bounded history, stopped session lifecycle, validated bundles, backups, and initial schema migrations. Add fake device/clock/storage adapters for deterministic failures.
5. Implement graph plan/commit, plan expiry, conflicts, undo planning, route inspection, and baseline event subscriptions/resync. Use fake runtime activation to test crash stages and generation identity.
6. Implement CLI help/discovery/status/session/graph/watch/export/import and generic API invocation. Generate TypeScript contracts for the later UI. Feature methods not yet implemented report explicit unavailable capability; no success-shaped stubs.
7. Add contract fixtures, invariants/property tests, permission tests, parser/import fuzz seeds, and exact PowerShell examples. Document every method that actually exists.

## Acceptance gate

ARCH-01–03/10/12; GRAPH-01–09/12/13 model behavior; API-01–12 foundation; AUTO-01–05 foundation; STATE-01–07/12 foundations; SEC-01–04/09/10/12 applicable control/import boundaries; ENG-01/02/04 are implemented with evidence. This gate concerns fake runtime behavior and does not satisfy M02 hardware activation.

Two clients read revision N; one commits and the other receives a conflict without data loss. Retrying a committed idempotency key returns the original result. A crash at every journal stage restores a coherent committed snapshot. Imported traversal/oversized assets fail without writes outside staging. A graph explanation lists all sources for a destination. A denied client fails identically through typed and generic calls. An API batch is demonstrably not atomic while graph.plan/commit is.

## Verification and scope

Run affected Rust checks, schema/client drift checks, unit/property/integration tests, and Windows pipe/security tests. Attach request/response/error fixtures. No UI audio logic, Windows DSP engine, plugin hosting, or real virtual endpoint provisioning belongs here. Fake devices must be visibly labeled in discovery and cannot be shipped as a production audio path.

## Handoff

Document crate boundaries, API/schema versions, fixture conventions, database/journal behavior, security bootstrap, and commands for starting the backend/CLI. Preserve migration fixtures before M02 changes schemas. Archive execution evidence and set the next active task to real audio adapters.

Suggested request: “Implement M01 against the accepted M00 decisions. Deliver the backend domain/API/storage and CLI with fake-engine evidence, keeping all not-yet-implemented audio capabilities explicit.”
