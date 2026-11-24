# Resilience repairs: helper threads, operation lookup and plan cap (2026-11-24)

Overnight, unattended work on branch `fix/resilience-res11-ops-plans`, based on
`origin/main` 0fb41911b (release 0.0.32). Four findings from the audits in
[the resilience audit](../../archived/2026-11-13-resilience-audit.md) and
[the bug audit](../../archived/2026-11-11-bug-audit.md). Each was still present
on `origin/main` (checked by reading the current source), got a regression test
that failed before the fix, and was fixed in the layer that owns it. No API
method, parameter or schema changed, so no contract, REST, OpenAPI or MCP file
changed.

## RES-11a: backend log thread survives a panicking record

- **Still present**: yes. `spawn_backend_rpc_logger` ran `for record in receiver`
  with no panic handling, and the sender lives in a process-wide `OnceLock`, so a
  panic in one record ended the thread and every later record was dropped
  silently.
- **Fix** (`crates/transport/src/lib.rs`): the loop moved into
  `run_backend_log_records`, which wraps each record's write in `catch_unwind`
  and calls `on_panic` once per lost record. In production `on_panic` appends one
  `backendLogWriterPanic` record (event, time, process ID, no request data), so
  the loss leaves a trace in `backend.jsonl`.
- **Tests**: `a_panic_while_writing_one_record_keeps_later_records_logged`
  (injects a panic on the second of four records; requires the other three
  written, one `on_panic`, and the thread to end normally) and
  `the_panic_marker_names_the_event_and_carries_no_request_data`.
- **Failing before**: with the `catch_unwind` temporarily removed the first test
  failed with `the log thread survives a panic: Any { .. }`.

## RES-11b: sleep/lock notifier retries once and logs a failure

- **Still present**: yes. The `audiorouter-os-transition-rpc` thread in
  `src-tauri/src/main.rs` discarded the result of `forward_rpc_request`.
- **Fix**: `forward_os_transition_with_retry` (testable, closures for the send and
  the failure log). A transport error or a JSON-RPC error answer is retried once
  after `OS_TRANSITION_RETRY_PAUSE` (300 ms; Windows gives a sleeping machine
  only a couple of seconds). The second failure is logged once through
  `log_shell_rpc`, which writes `shell.jsonl` (`transportError` or `error`). A
  first-try success or a recovered retry logs nothing. The request keeps its
  idempotency key, so a retry after a lost reply cannot release streams twice.
- **Tests** (shell crate): `os_transition_retries_once_and_logs_one_failure`
  (writes the real `shell.jsonl` record into a temp directory and reads it back),
  `os_transition_recovers_on_the_retry_without_logging`,
  `os_transition_success_sends_once_and_logs_nothing`,
  `os_transition_backend_error_answers_are_retried_and_logged_as_errors`.
- **Failing before**: with the function reduced to the old single send, three of
  the four failed (attempt count 1 instead of 2, no retry success, no log); the
  success test passes before and after by design.

## STO-6: `operations.get` finds every API method's operation

- **Still present**: yes. `operation_lookup_keys` (`crates/control/src/persistence.rs`)
  guessed the method from a hand-kept list of 17.
- **Decision**: the audit proposed new indexed `operation_id` and client columns
  in `operation_journal`. That needs a schema migration, a backfill of existing
  rows and edits to eight INSERT sites in `crates/storage`. I instead made the
  lookup derive its candidates from `API_METHODS` (156 names; one primary-key read
  each, and `operations.get` is rare). This fixes the defect with no schema
  change, and a new method is found without anyone remembering a list. The
  indexed-columns design stays available if lookup cost ever matters; recorded
  here so the owner can overrule it.
- **Tests** (`crates/control/src/persistence_tests.rs`, all through
  `dispatch_authorized_for_client`, i.e. with a client scope as on the pipe):
  `operations_get_finds_a_journaled_operation_of_every_api_method` (walks all
  `API_METHODS`, journals a row per method and requires `completed` with the right
  `operation`, and that another client cannot read it),
  `operations_get_finds_what_task_and_lifecycle_methods_just_did` (real
  `sessions.active.set`, `toolPresets.save`, `system.osTransition`), and
  `operations_cancel_recognises_an_operation_of_a_method_the_old_list_missed`,
  and `operations_get_skips_candidate_keys_too_long_for_the_journal`.
- **Second defect found in review of my own fix**: storage rejects a lookup key
  longer than `MAX_IDEMPOTENCY_KEY_BYTES` (128) with an error, not "no row". The
  old list's longest method name was about 24 characters; `API_METHODS` has
  `audioMedia.importTemporaryRecording` (35), so a long operation ID that fits
  under a short method name made the whole lookup fail when a longer candidate was
  tried first. Candidates over the limit are now skipped (they could never have
  been written). The last test uses an ID sized so `sessions.stop` fits the limit
  exactly and the longest name does not; it failed with the filter removed
  (`found` panic) and passes with it.
- **Failing before**: the walk named about 140 methods that could not be found
  (including `sessions.start`, `sessions.stop`, `nodes.*`), and the real-method
  test failed with `operation not found` for `sessions.active.set`. The
  `operations.cancel` test was not run against the old code (it shares the same
  lookup function).

## PLAN-1: abandoned graph plans no longer block saves

- **Still present**: yes (`GraphStore::plan_graph_with_ttl` refused at 100 plans;
  storage `save_graph_plan` also refused at 100).
- **Decision**: the audit also proposed dropping a session's other plans when a
  commit advances its revision. I did that first and then reverted it: a late
  commit of a plan made stale by another save changed from `revisionConflict`
  (409, "fetch and plan again", documented in `docs/operations/local-http-api.md`
  and handled by the UI and MCP guides) to `PlanNotFound`, which tells the caller
  nothing useful. Stale plans now stay until room is needed and are the first to
  be dropped then, so the error contract is unchanged.
- **Fix**:
  - `crates/domain/src/lib.rs`: `make_room_for_plan` (used by planning and by
    restoring a durable plan) drops expired plans, then, only if the cap is still
    reached, plans whose session has moved past their base revision, then the
    oldest plans (by a new creation sequence, since plan IDs are text), instead
    of refusing the new plan. `PlanLimitReached` remains only for plan-ID counter
    exhaustion.
  - `crates/storage/src/lib.rs`: `save_graph_plan` evicts the oldest rows at the
    cap (after validation, so a rejected plan evicts nothing) instead of
    returning "graph plan inventory exceeds 100 items".
- **Tests**: through `ControlPlane::dispatch` (`graph.plan` and `graph.commit`):
  `abandoned_plans_of_one_session_do_not_block_another_sessions_save` (100 plans
  for session A, commit one, a late commit of another A plan still answers
  `revisionConflict`, a plan for session B succeeds, further B plans never fail,
  all B plans are kept and the stale A plans give up the room, in memory and in
  storage) and
  `a_full_plan_inventory_drops_the_oldest_plan_instead_of_refusing_the_newest`
  (with and without storage). Lower layer:
  `a_full_inventory_drops_stale_plans_before_live_ones_and_a_late_commit_still_conflicts`
  (domain). Three
  older tests asserted the refusal and were rewritten to the new rule:
  `graph_store_bounds_pending_plans_and_prunes_expired_entries`,
  `direct_graph_plan_writes_bound_live_inventory`, and
  `graph_plan_persistence_failure_rolls_back_the_in_memory_plan` (now injects the
  write failure with a SQL trigger on a file database, since a full inventory no
  longer fails).
- **Failing before**: `graph.plan` for the 101st plan failed with "Too many plans
  are waiting" (memory and storage). The first test's final B-plans step also
  fails on the old code at the cap; its earlier `revisionConflict` step passes
  before and after by design (it guards the contract the first draft of the fix
  broke).
- **Behaviour change to know about**: a script that floods `graph.plan` can now
  evict another client's older uncommitted plan (after the stale ones), which
  then commits as `PlanNotFound`. Before, the flood blocked every save for five minutes. A plan
  that is committed right after it is made is never the oldest. If this matters,
  the next step is a per-client share of the cap, which needs the client ID in the
  store.

## Commands and results

All with `CARGO_BUILD_JOBS=6` on Windows 11, Rust 1.96.0 (pinned).

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check`; `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | clean, no unrelated drift |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | pass (after fixing two findings in my new tests) |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | pass (the Tauri macro needs `ui/dist`, which this worktree lacks: I created a git-ignored placeholder for the clippy runs and deleted it afterwards, so no placeholder UI is embedded in anything) |
| `cargo test -p audiorouter-transport -p audiorouter-control -p audiorouter-domain -p audiorouter-storage -p audiorouter-cli` (final run) | pass: transport 43, control 459, domain 84, storage 113 (one storage test was removed with the reverted helper), cli 58 + 5 + 1 (ignored live tests skipped) |
| `cargo test --manifest-path src-tauri/Cargo.toml` | pass: 135 passed, 4 ignored (run before the PLAN-1/STO-6 revision; the shell sources did not change after) |
| `node tools/docs/validate.mjs` | pass |

## Not run

- The packaged shell, a live sleep/resume cycle and the real
  `audiorouter-os-transition-rpc` thread against a busy backend: the retry logic
  is unit-tested with injected sends only. Nothing was launched on the default
  control pipe.
- A panic inside the real `backend.jsonl` writer: the panic is injected into the
  record loop; no natural panic source exists to trigger.
- `npm --prefix contracts run check:drift` and the REST/MCP reachability suite
  beyond the cli crate tests: no API method changed (contracts has no
  `node_modules` in this worktree). The cli tests, including
  `every_api_method_is_reached_by_a_default_mcp_tool_or_excluded`, pass.
- Clock-drift, audio continuity and UI suites: untouched code.
- Other workspace crates' tests (engine, dsp, windows-audio, plugin-host): not
  affected by these files and not rerun.
