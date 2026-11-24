# Rust small defects from the Q4.1 verification (2026-11-24)

Branch `fix/rust-small-defects`, from `integration/overnight-2026-11-24`
`84bfad559`, with the fast-forwarded `fix/instrument-app-usage` (`4f3b4e8f8`,
the OBS and app-usage Instrument fix). Local only: not pushed, no PR.

These are the three items the Q4.1 verification left for the owner, chosen so
that no product policy changed.

## D1. Optional `minLength: 1` strings accepted `""`

`validate_scalar_param_types` (`crates/control/src/authorization.rs`) enforced
the published JSON type and `maxLength` of every scalar parameter but not its
`minLength`. Seven optional strings therefore accepted an empty value that the
published schema refuses:

- `sessions.summary` / `sessions.get` / `sessions.export` / `meters.levels`
  `sessionId` (an empty value fell back to the open session),
- `recordings.recovery` `cursor` (empty meant the first page),
- `toolPresets.list` `kind` (empty meant every kind),
- `toolPresets.delete` `id`.

`ScalarRule` now carries `min_length`, and the dispatcher refuses a shorter
string with `"{name} must be at least {n} character(s)"`. `null` still means
omitted.

Regression: the generated `empty-string:{field}` contract cases already
existed. They were forgiven through `EMPTY_STRING_LENIENT` in
`crates/control/src/contract_cases.rs` (a ratchet that fails once a pair is
fixed but not removed). The list and the forgiver are gone, so every
`minLength: 1` string parameter is covered by the generated cases through
`every_method_refuses_invalid_input_with_a_structured_category`.

Contract note: an empty string now fails as JSON-RPC invalid params (-32602,
no `error.data`) rather than in the handler. Two tests pinned the old shape and
were updated: `graph_commit_can_require_its_session_to_be_stopped_for_recovery_edits`
(empty `requireStoppedSessionId`) and
`an_empty_commit_key_is_refused_for_a_named_client_too` (empty
`idempotencyKey`). Both still require a refusal, identical for a named client
and for trusted dispatch. This matches the existing wrong-type behaviour.

## D2. `probe_pipe` errors read as "app not running"

`pipe_is_owned_by_a_server` returned `bool`, so every probe error (for example
a pipe owned by another Windows user, which the probe refuses to open) became
`false`, and `mcp serve` without `--pipe` treated the app's own database as
free: it either reported a closed app or opened a second control plane beside
the app. Only a missing pipe means "not running" (the comment said so; the
code did not).

`app_pipe_state` now returns `Missing`, `Owned` (available or busy) or
`Unreadable(reason)`. `is_running_app_database` returns
`Result<bool, String>`; `run_mcp_stdio` reports why the pipe could not be
examined and names `--pipe \\.\pipe\audiorouter-control`, instead of starting a
second backend.

Regressions: `a_pipe_that_cannot_be_examined_keeps_its_reason` (new; a name
Windows refuses must keep its reason, where the old boolean dropped it) plus
the existing busy, free-instance and missing probes now assert the tri-state.

## D3. MCP `create_session` as Editor: created, then `permissionDenied`

`create_session` and `start_from_goal` called `sessions.create` (GraphWrite)
and then `sessions.active.set` (SessionControl). An Editor's session was
created, then the open step was refused, leaving a session the caller could
not open. `security_boundaries` already pinned the intent: the complete
workflow needs Operator.

Both tools now check `ClientGrant::can_control_sessions()` (new public method)
before creating anything and refuse with the backend's own denial shape
(`-32001` plus `data.code = permissionDenied`, from `mcp_role_refusal`), so
the refusal still reads as a permission refusal to REST, MCP clients and the
activity log. The Editor gets no orphan session. The descriptions of
`create_session`, `start_from_goal` and `open_session` now state the role.

Regressions: `create_session_refuses_an_editor_before_creating_a_session`
(new; an Editor is refused and `sessions.list` has no extra session, an
Operator still creates and opens), and the existing
`real_mcp_pipe_preserves_roles_and_live_revocation`,
`mcp_stdio_client_interoperates_with_cli_process`,
`local_cowork_identity_interoperates_with_cli_and_observer_writes_are_denied`
and `goals_presets_and_saved_versions_work_through_default_tools` keep their
permission codes.

## Checks (Windows, 2026-11-24)

- `cargo test -p audiorouter-control --lib`: 509 passed, 0 failed, 29 ignored.
- `cargo test -p audiorouter-cli --lib`: 71 passed, 0 failed, 1 ignored.
- `cargo test -p audiorouter-cli --test security_boundaries`: 2 passed.
- `cargo test -p audiorouter-cli --test mcp_stdio`: 5 passed.
- `cargo test --workspace`: every crate and integration suite passed (0 failed).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
- `cargo fmt --all -- --check` and `cargo fmt --manifest-path
  src-tauri/Cargo.toml -- --check`: clean.
- Shell: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
  --all-features -- -D warnings` clean; `cargo test --manifest-path
  src-tauri/Cargo.toml` 142 passed, 4 ignored.
- Real backend through the UI: rebuilt the harness first
  (`cargo build -p audiorouter-control --example e2e_backend`), then
  `npx playwright test --workers=1` from `ui/` (Edge, dev server on 4186):
  862 passed, 16 skipped, 0 failed, 33.0 min, first pass. The same count as the
  integration branch, so the stricter empty-string validation refuses nothing
  the UI sends.
- The unit and integration suites above were run before comment-only edits to
  the two new doc comments.

Not run: the built exe, a live app, hosted CI, and `mcp serve` against a real
app whose pipe belongs to another Windows user (no such server was available;
the tri-state is covered by the invalid-name case). No UI source changed in
this slice; the merged `fix/instrument-app-usage` UI evidence is in the plan.