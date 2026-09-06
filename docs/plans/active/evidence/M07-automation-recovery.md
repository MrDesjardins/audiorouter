# M07 automation and recovery evidence

## 2026-09-06 — CLI graph plan/apply files

Implemented the portable AUTO-04 CLI slice in `crates/cli`:

- `graph plan <session-id> --base-revision <n> --file <candidate.json> --output <plan.json> --database <path>` validates the candidate through the shared `graph.plan` dispatcher and writes a `audiorouter.graph-plan` schema-versioned JSON envelope.
- `graph inspect <plan.json>` validates the envelope and returns it without opening storage or mutating state.
- `graph apply <plan.json> --idempotency-key <key> --database <path>` reads the current session revision, rejects stale plans before planning, then replans and commits through the same control-plane methods.
- Plan files use exclusive creation to avoid accidental overwrite. Candidate/session identity and absolute file/database paths are checked.

Regression coverage verifies an independent CLI-process-equivalent plan, inspect, and apply sequence. `cargo test -p audiorouter-cli` passes 6 tests. The implementation is configuration-only: it does not open audio devices, install drivers, or change machine audio settings.

Remaining AUTO-04 work includes parity for the remaining convenience commands and full end-to-end warning-producing operations.

## 2026-09-06 — Node parameter convenience parity

Added `node set <session-id> <node-id> <parameter> --value <json-scalar>
--idempotency-key <key> --database <path>`. The command loads the current
session, validates a scalar parameter value, hydrates the control plane's
authoritative session copy, and commits through the same graph plan/commit
boundary as generic graph edits. Missing sessions/nodes, malformed values, and
invalid processor parameters fail before a commit. The regression verifies a
Gain parameter update increments the session revision and persists the value.
`cargo test -p audiorouter-cli --locked` passes 16 unit tests plus MCP
interoperability; no audio endpoint or machine configuration is touched.

`node set --dry-run` now validates and returns the candidate without requiring
an idempotency key or creating a durable plan. The regression confirms the
candidate contains the requested value while the persisted session revision
and parameter remain unchanged.

Invalid object/array/null values are rejected before validation, and normal
non-dry-run calls still require a bounded idempotency key.

## 2026-09-06 — Recording-list cursor parity

The recordings list method now accepts optional cursor and bounded limit fields.
Without either field it preserves the established array response; with paging
requested it returns an items/nextCursor envelope using stable persisted
recording ordering and recording-ID cursors. Unknown cursor IDs and invalid
limits are rejected. The shared contract describes both response forms, and
the connected UI normalizes the page form back to its existing row list.
Control coverage, contracts/UI typechecks, and UI tests pass; no recording
bytes are read or changed.

The CLI now exposes the same optional recording cursor and limit fields. A
regression verifies `recordings list --limit 1` returns the page envelope while
the existing unpaged command continues to return its legacy array. CLI and
control suites, contracts/UI typechecks, and UI tests pass.

## 2026-09-06 — Session-list CLI cursor parity

`session list` now accepts optional `--cursor ID` in addition to its bounded
`--limit` option. Cursor requests are sent through the authoritative
`sessions.list` control dispatcher and return the backend page envelope with
`items` and `nextCursor`; calls without a cursor retain the existing flat list
shape for compatibility. A two-session regression verifies the second page
and cursor. CLI coverage passes 17 unit tests plus MCP interoperability.

## 2026-09-06 — Graph-history CLI cursor parity

`history <session-id>` now accepts optional `--cursor REVISION` and routes
cursor requests through the authoritative `graph.history` dispatcher. Pages
are bounded to the API's 100-entry history limit and return `items` plus
`nextCursor`; the existing flat response remains available when no cursor is
provided. Limits above 100 are rejected for cursor mode instead of being
silently changed. The CLI suite and MCP interoperability regression pass.

The cursor path is covered by a two-revision SQLite regression: requesting
revision cursor `2` returns revision `1` and the following cursor `1`, proving
that the page boundary is based on the persisted revision ordering.

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

Fixed a race in adjacent `apps.list` and `applications.list` requests by retaining one live application snapshot for 100 ms in the control plane. Both aliases now return the same coherent process identity and audio-session observations while still refreshing promptly. Control tests pass 44 cases with strict Clippy.

The CLI/MCP discovery text now names those observed audio-session fields so
headless clients do not mistake the application result for process-only
enumeration. Both adapters continue to route through the shared authorized
dispatcher; no audio stream or machine setting is touched.

The CLI also accepts the canonical `applications list` spelling in addition
to the legacy `apps list` spelling. A regression confirms both commands
produce identical JSON from the shared dispatcher.

The CLI regression also requires the full observed audio-session field set on
each returned application, keeping headless output aligned with the control
schema and UI.

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

## 2026-09-06 — Session deletion invalidates durable plans

Session deletion now removes all SQLite-retained graph plans for that session
within the same transaction as the session and history deletion. A storage
regression verifies that a retained plan is absent after deletion; control
coverage remains green at 46 tests, with strict Clippy and formatting passing.

## 2026-09-06 — Graph warning acknowledgment validation

Aligned `graph.commit` discovery and validation with API-05 by accepting an
optional nullable `acknowledgments` array of bounded warning IDs. The current
plan generator exposes no warnings, so non-empty acknowledgments are rejected
explicitly rather than treated as authorization; malformed IDs are rejected
before plan lookup. Control coverage is 47 tests, domain coverage 23, and CLI
coverage 9, with strict Clippy passing.

## 2026-09-06 — Durable privacy mute latch

Added `safety.setPrivacyMute`, `privacy mute <on|off>`, and MCP
`set_privacy_mute` with explicit Capture-scope authorization. SQLite-backed
control planes restore the latch on restart; `status.get` reports whether the
latch is durable and explicitly limits its audio effect to the process-local
realtime backend when available. Enable/disable events are discoverable and
read-only clients cannot mutate the latch. Control/storage/CLI coverage passes
at 48/25/9 with strict Clippy.

Startup recovery now fails closed when the persisted latch cannot be read:
the control plane starts muted rather than silently unmuting a capture path.
The redacted diagnostics response exposes the same muted state and whether
the latch is durable or process-local. Targeted control/CLI tests (48/9),
strict Clippy, formatting, and diff checks pass.

## 2026-09-06 — Recording CLI parity

Added `recordings list`, `recordings get`, and `recordings remove-entry` to the
headless CLI. These commands open only the caller-selected absolute SQLite
database and dispatch through the same control-plane methods as MCP/API calls.
The end-to-end CLI regression verifies metadata listing and retrieval, then
removes only the library row and confirms `fileAction: none`; the underlying
recording path is not touched. Nine CLI tests and strict Clippy pass.

## 2026-09-06 — Startup capability reporting

Added read-only `startup.get` API, CLI (`startup get`), and MCP
(`get_startup`) reporting. The response explicitly reports sign-in startup
registration as unavailable in this build; it performs no registration and
does not change machine configuration. Domain, control, and CLI validation is
covered by the workspace test suite (50 control tests, 23 domain tests, and
10 CLI tests).

## 2026-09-06 — Durable recording checkpoints

SQLite now stores versioned `RecorderController` checkpoints separately from
recording metadata. Save validates and atomically replaces a checkpoint; load
restores it through the recording validator and surfaces corruption as an
explicit error; clear removes it idempotently. Storage coverage is 26 tests
with strict Clippy. No audio payloads or file handles are persisted.

## 2026-09-06 — Recovery CLI parity

Added `backup --database <path> --output <new-path>` and
`restore --backup <path> --database <new-path>`. Both commands require
absolute paths and new destinations; restore validates SQLite integrity before
writing. The commands use the existing storage safeguards and do not open
audio devices or change machine configuration.

The validated checkpoint is now inspectable through the authorized
`recordings.recovery` API, `recordings recovery` CLI command, and MCP
`get_recording_recovery` tool. It returns lifecycle metadata only, reports
missing checkpoints explicitly, and preserves the Record-scope boundary.
Control coverage is 51 tests with strict Clippy.
CLI coverage is 12 tests with strict Clippy, including the recovery command
and MCP authorization path.

Removing a recording library entry now removes its associated checkpoint in the
same SQLite transaction, while leaving the recording path untouched. The
storage regression verifies checkpoint cleanup and repeated removal behavior.

Added the bounded one-shot `watch` CLI command over `events.subscribe`. It
forwards the selected session, replay cursor, and validated 1–500 event limit
through the shared control dispatcher without opening audio or mutating
configuration. CLI coverage is 13 tests with strict Clippy.

## 2026-09-06 â€” Corrupted database handling

`Storage::open` now performs a read-only SQLite integrity check before schema
migration. Malformed or damaged files return the explicit `CorruptDatabase`
error and remain byte-for-byte untouched, directing recovery through the
existing validated backup/restore workflow. Storage coverage is 27 tests with
strict Clippy.

The control layer maps this condition to a stable, non-retryable
`corruptDatabase` response with guidance to open a validated backup or restore
into a new destination. Control coverage is 52 tests with strict Clippy.

Storage open now performs a bounded retention sweep for expired idempotency
journal and graph-plan rows after migrations. A reopen regression verifies both
classes are removed while session and file state remain untouched. Storage
coverage is 28 tests with strict Clippy.

Added a process-level MCP stdio interoperability regression. It creates a
temporary SQLite enrollment for an observer client, launches the built
audiorouter-cli binary, exchanges initialize, notifications/initialized,
tools/list, and resources/list over newline-delimited JSON-RPC, verifies the
pinned protocol and 22 tools/3 resources, then closes stdin and confirms clean
server exit. The test passes with strict CLI Clippy; no audio device or machine
configuration is accessed.

The same process client now reads audiorouter://diagnostics and invokes the
read-only get_startup tool after initialization and discovery. Both responses
are validated over the real stdio stream, extending interoperability coverage
beyond listing. The test remains non-audio and temporary-state-only.

The process client also attempts the plan_graph_change tool as an observer.
The server returns an error result over stdio while the authorized read-only
resource and startup calls succeed, proving the grant boundary is preserved
through the external MCP process rather than only in an in-process unit test.

Workspace checkpoint: the complete Rust workspace test run passes after the
MCP and worker changes, including the CLI process-level MCP test, all control
and storage recovery coverage, and the plugin-host worker tests. Strict
formatting/Clippy checks remain green. This checkpoint does not claim native
pipe interoperability, live audio routing, driver lifecycle, or signing.

## 2026-09-06 — Explicit filesystem backup retention

Added Storage::prune_recovery_backups and the headless
backup prune --directory command. The operation requires an absolute,
non-symlink directory, considers only direct audiorouter-backup-*.sqlite
files, retains the newest ten timestamped names, preserves pre-migration and
unrelated files, and reports removed paths. Temporary-directory storage and
CLI regressions pass with strict Clippy. Recordings and the SQLite database
are never pruned by this operation.
## 2026-09-06 — Storage reparse-point boundaries

Storage recovery boundaries now reject Windows reparse points in addition to
portable symlinks. Backup destinations, restore sources, bundle files and
staging roots, and retention directories are checked before their respective
operations. The storage suite passes 30 tests with strict Clippy; deployment
filesystem policy beyond these selected roots remains open.

## 2026-09-06 — Portable crash-loop policy

The domain crate now contains a deterministic `CrashRecoveryTracker` for the
STATE-10 supervisor boundary. It retains only crashes in a ten-minute window,
enters safe mode after three recent crashes, and returns only previously
running, non-recording sessions as automatic recovery candidates. Stable-run
reset and expiry are covered by domain tests; this is policy evidence only and
does not claim persisted crash markers, process restart orchestration, Windows
startup integration, or automatic audio restart.

Read-only control status and diagnostics now expose the persisted recovery
`safeMode` state and its durable/memory provenance. A storage-backed control
regression verifies the latch is visible after three recorded crashes; these
surfaces do not start sessions or resume audio.

Storage now persists the bounded crash markers in a dedicated SQLite table with
expiry, a three-marker cap, count, and explicit clear operations. Storage tests
cover expiry and invalid timestamp rejection. The control/supervisor integration
still remains to be implemented; no automatic restart or audio resume is
claimed.

The policy regression also verifies that safe mode remains latched after the
ten-minute crash timestamps expire and is released only by an explicit stable
run. This prevents an elapsed wall-clock window from silently rearming routes.
Backup and restore destinations now use `symlink_metadata` presence checks
instead of relying on `Path::exists()`. This rejects dangling symlinks before
SQLite creates or restores a file, closing a path-safety bypass; the regression
suite verifies a broken backup destination is preserved/rejected when link
creation is available. Storage coverage is 30 tests with strict Clippy.
## 2026-09-06 — Reparse-point parent protection

Backup and restore now validate their destination parent directories with
reparse metadata before invoking SQLite. Recording rename similarly validates
the source file and both parent directories before moving a path. Existing
relative/missing/existing-destination and non-destructive recovery tests remain
green: storage coverage is 30 tests with strict Clippy.
Bundle export now validates its parent with reparse metadata and checks
destination presence through `symlink_metadata`, preventing a dangling link or
redirected parent from bypassing the new-file policy. The complete storage suite
passes 30 tests with strict Clippy.
Restore and bundle export now have conditional dangling-link regressions in
addition to backup coverage. When link creation is available, each operation
rejects the broken destination before SQLite or ZIP output is created. All 30
storage tests and strict Clippy pass.
The complete locked workspace was rerun after the latched-policy correction and
SQLite marker persistence. All Rust unit, integration, worker-process, MCP, and
doc tests passed, including 28 domain and 32 storage tests; strict workspace
Clippy, formatting, and diff checks also passed. No audio endpoint or machine
configuration was changed.

SQLite now persists the safe-mode latch alongside bounded crash markers. A
file-backed regression verifies that three crashes retain safe mode across a
storage reopen and that the explicit clear operation removes both markers and
the latch. Process supervision and automatic route restart remain unimplemented.

The in-memory recovery policy now caps its retained crash timestamps at three,
matching the SQLite persistence bound. A crash-storm regression verifies that
the tracker remains bounded while still entering latched safe mode.

File-backed storage coverage now reopens the SQLite database and verifies that
two recent crash markers persist across the boundary, then clears them through
the explicit maintenance operation. This strengthens persistence evidence but
does not claim that a supervisor records crashes or automatically restarts
audio sessions.

`CrashRecoveryTracker::decide_recovery` now returns the policy mode and its
automatic-restore session set as one deterministic result. This gives the
future supervisor an atomic decision boundary: safe mode cannot be observed
alongside a stale restore list. It remains portable policy evidence; process
creation, crash recording, and native route restart are still unimplemented.

`Storage::recovery_status` now reads the bounded recent-crash count and the
durable safe-mode latch in one SQLite transaction, giving supervisor wiring a
consistent persisted snapshot. This remains storage evidence only; it does
not record crashes, create processes, or restart audio routes.

The control plane now publishes that snapshot through both `status.get` and
`system.diagnostics`, including the bounded `recentCrashes` count and durable
`safeMode` latch. The control regression verifies both fields without opening
audio or resuming sessions; process supervision remains unimplemented.

The Windows plugin worker now applies Job Object limits for one active process
and 512 MiB of process memory in addition to kill-on-close containment. This
is resource-containment evidence only; it does not establish filesystem or
network isolation for arbitrary plugins.

The explicit recovery maintenance operation is now available as the
authorized `recovery.clearSafeMode` control method and the matching CLI
command. It clears only the persisted crash markers and safe-mode latch; it
does not start sessions, resume routes, or alter audio configuration.

`WorkerSupervisor::record_failure` now lets a native adapter report immediate
worker faults through the same bounded failure/quarantine policy used for
heartbeat timeouts. The transition does not restart processes or claim plugin
execution; those native integration gates remain open.

Worker launch now clears the inherited environment and supplies configuration
only through validated arguments and owned protocol handles. This reduces
backend credential/configuration leakage while retaining the Job Object limits;
it is not a substitute for Windows filesystem/network sandboxing.

MCP now exposes the same maintenance operation as
`clear_recovery_safe_mode`. An operator-scope grant succeeds while a read-only
grant is rejected, and the tool routes through the shared control dispatcher;
no session or audio restart is performed.
The MCP stdio interoperability regression was updated to account for the
published `clear_recovery_safe_mode` tool. The catalog now contains 23 tools,
including the recovery maintenance operation, and the locked workspace test
suite confirms the process-level catalog remains internally consistent.
