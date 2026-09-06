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

## Next action

Implement backup restore from a validated staging area over the now-tested local transport. Keep a portable fake transport for deterministic tests and do not add an HTTP listener. Bundle staging now has bounded v1 ZIP validation and optional asset hash/size verification; remaining bundle work is required-node-type compatibility and API integration.
