# M07 automation and recovery evidence

## 2026-09-06 — CLI graph plan/apply files

Implemented the portable AUTO-04 CLI slice in `crates/cli`:

- `graph plan <session-id> --base-revision <n> --file <candidate.json> --output <plan.json> --database <path>` validates the candidate through the shared `graph.plan` dispatcher and writes a `audiorouter.graph-plan` schema-versioned JSON envelope.
- `graph inspect <plan.json>` validates the envelope and returns it without opening storage or mutating state.
- `graph apply <plan.json> --idempotency-key <key> --database <path>` reads the current session revision, rejects stale plans before planning, then replans and commits through the same control-plane methods.
- Plan files use exclusive creation to avoid accidental overwrite. Candidate/session identity and absolute file/database paths are checked.

Regression coverage verifies an independent CLI-process-equivalent plan, inspect, and apply sequence. `cargo test -p audiorouter-cli` passes 6 tests. The implementation is configuration-only: it does not open audio devices, install drivers, or change machine audio settings.

Remaining AUTO-04 work includes durable server-side plan retention/expiry across backend restarts, warning acknowledgments, and parity for all convenience commands.

## 2026-09-06 — Event epoch reconnect guard

`events.subscribe` now accepts an optional `backendEpoch`. A mismatched epoch immediately returns `resyncRequired: true`, the current epoch, a bounded session snapshot, and the next event sequence. This prevents a restarted backend from replaying a cursor from a previous process epoch. The request schema and strict unknown-parameter validation include the new field. `cargo test -p audiorouter-control` passes 42 tests and strict Clippy passes.

## 2026-09-06 — Recovery backup overwrite guard

`Storage::backup_to` now rejects every existing destination, not only symbolic links and the live database. The regression confirms an existing recovery copy remains byte-for-byte unchanged after a rejected backup. `cargo test -p audiorouter-storage` passes 24 tests and strict Clippy passes.

## 2026-09-06 — MCP stdio adapter foundation

Added `audiorouter mcp serve --client-id <id> --database <absolute-path>`. The adapter uses newline-delimited UTF-8 JSON-RPC over stdin/stdout, keeps diagnostics off stdout, negotiates the pinned MCP `2025-06-18` version, and exposes seven tools: `describe_capabilities`, `list_devices`, `list_applications`, `get_session`, `inspect_routes`, `get_operation`, and `call_api`. Tool calls are translated into `ControlPlane::dispatch_authorized_for_client`, so the MCP layer owns no graph or audio state and cannot bypass enrolled scopes. The implementation follows the official MCP stdio framing/lifecycle shape documented at https://modelcontextprotocol.io/specification/2025-06-18/basic/transports and https://modelcontextprotocol.io/specification/2025-06-18/schema.

The CLI/control tests pass and strict Clippy passes. This is a foundation slice: live backend named-pipe connection, resources, cancellation, progress, and external-client interoperability remain unverified.

## 2026-09-06 â€” MCP resource discovery

Added `resources/list` and `resources/read` for `audiorouter://capabilities`, `audiorouter://diagnostics`, and `audiorouter://workflow/headless`. Capabilities and diagnostics are fetched through the enrolled client's authorized control dispatcher; workflow guidance is static and explicitly says imports do not arm recorders or install drivers. The adapter still emits only newline-delimited JSON-RPC on stdout. CLI tests pass 7 cases with strict Clippy.

## 2026-09-06 — MCP backend pipe proxy

Added optional `--pipe <\\.\\pipe\\name>` mode to MCP serve. API tool and capability/diagnostic resource requests are encoded with the existing 4-byte framed protocol and sent through `audiorouter-transport::round_trip` to the local backend; the backend remains responsible for authenticated authorization and state. No network listener, audio endpoint, or machine configuration is introduced. The changed CLI compiles and strict Clippy passes. Runtime test launch was blocked by Windows Application Control OS error 4551, so native pipe interoperability remains pending.

## 2026-09-06 — Recording metadata API parity

Added `recordings.list` to `API_METHODS`, discovery schemas, stable unknown-parameter validation, and control dispatch. Storage-backed results expose recording identity, session/recorder association, format/shape, frame/file counts, lifecycle state, missing flag, and user metadata; the method performs no file or audio action. The MCP adapter exposes the same operation as `list_recordings`. Control tests pass 43 cases, CLI tests pass 7 cases, and strict Clippy passes.

## 2026-09-06 — Single recording metadata API

Added `recordings.get` with validated `recordingId` input and storage lookup, plus MCP `get_recording`. Missing IDs return an actionable not-found response; successful results contain the same metadata shape as list entries and never open or modify the recording path. Targeted storage/control/CLI tests pass (24/43/7) with strict Clippy.

## 2026-09-06 — Authorized recording metadata mutation

Added `recordings.setMetadata` with explicit `Record` permission scope and bounded title/artist/comment fields. The control method delegates to the existing transactional storage update, reports not-found cleanly, and never changes recording path or audio content. A read-only grant is denied while an explicit record-scope grant succeeds. Targeted storage/control/CLI tests pass (24/44/7) with strict Clippy.

## 2026-09-06 — Non-destructive recording entry removal

Added `recordings.removeEntry` with explicit `Record` permission scope and MCP `remove_recording_entry`. It deletes only the library metadata row, reports `fileAction: none`, and leaves the recorded file path outside the operation. Targeted storage/control/CLI tests pass (24/44/7) with strict Clippy.

## 2026-09-06 — Focused MCP graph/session tools

Added `plan_graph_change`, `apply_graph_change`, and `control_session` MCP tools. Each is a thin translation to the existing authorized API dispatcher, preserving complete candidate planning, base-revision checks, idempotency keys, and role-specific graph/session permissions. No MCP-owned state is introduced. CLI tests pass 7 cases and strict Clippy passes.

## 2026-09-06 — Application snapshot alias parity

Fixed a race in adjacent `apps.list` and `applications.list` requests by retaining one live process enumeration for 100 ms in the control plane. Both aliases now return the same coherent snapshot while still refreshing promptly. Control tests pass 44 cases with strict Clippy.

## 2026-09-06 — Typed CLI read parity

Added `diagnostics [--database <path>]` and `operation get <operation-id> --database <path>` convenience commands. Both use the shared dispatcher and preserve read-only semantics; help now advertises their exact forms. CLI tests pass 8 cases with strict Clippy.

## 2026-09-06 — Recording path privacy boundary

Changed `recordings.list` and `recordings.get` from generic `Read` to explicit `Record` permission because their metadata includes absolute file paths. The MCP focused tool descriptions now disclose the requirement, and a read-only grant regression confirms denial before storage access. Existing control/CLI suites remain green with strict Clippy.

## 2026-09-06 — Graph plan expiry contract

Aligned the default in-memory graph-plan lifetime and control responses with
API-05: plans now expire after five minutes (`expiresInMs: 300000`). The domain
TTL override remains available for expiry tests, so deterministic short-lived
plan coverage is preserved.

## 2026-09-06 — Durable uncommitted graph plans

SQLite-backed control planes now retain graph plan IDs, candidates, base
revisions, and expiry timestamps. A new control instance hydrates the current
session, revalidates the retained candidate and revision, then commits through
the normal graph store and journal path. Expired rows are pruned and successful
commits remove the retained plan. The restart regression passes, including the
fresh-commit response shape (`idempotentReplay: false`); 45 control tests, 24
storage tests, and strict Clippy pass.

## 2026-09-06 — Operation cancellation contract

Added `operations.cancel` to discovery, authorization, strict parameter
validation, the CLI's `operation cancel` command, and MCP's
`cancel_operation` tool. The backend currently
retains only completed graph-commit outcomes; cancellation therefore reports
`status: completed`, `cancelled: false`, and `reason: alreadyCompleted` rather
than claiming to undo a committed side effect. Unknown IDs return an explicit
not-found error. Domain/control/CLI tests pass (23/46/9) with strict Clippy.

## 2026-09-06 — Recording CLI parity

Added `recordings list`, `recordings get`, and `recordings remove-entry` to the
headless CLI. These commands open only the caller-selected absolute SQLite
database and dispatch through the same control-plane methods as MCP/API calls.
The end-to-end CLI regression verifies metadata listing and retrieval, then
removes only the library row and confirms `fileAction: none`; the underlying
recording path is not touched. Nine CLI tests and strict Clippy pass.
