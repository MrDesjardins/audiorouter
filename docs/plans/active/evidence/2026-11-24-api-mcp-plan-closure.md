# API/MCP plan closure verification — 2026-11-24

Worktree branch `plans/api-mcp`, based on `origin/main` at `53e1de80e`
(PR #187, tree-identical to `origin/integration/overnight-2026-11-24`
`1a9c3c7ca`). Scope: verify the fixes recorded in
`docs/plans/archived/2026-11-24-api-mcp-quality.md` and
`docs/plans/archived/2026-11-24-mcp-setup-ux.md`, port any useful unlanded work, and run
the required checks. No live audio, device, driver, packaged-shell, owner
profile, external connector or OBS check was performed.

## Result

Both plans' implementation is already on `main`. No code change was needed
for either plan: every listed finding has an implementation and a regression
test on `main`, and every commit on the two local branches is either merged
or superseded by a later `main` version. Two owner decisions remain
(assistant recording permission, session rename) and are recorded below.

## API/MCP findings verified against `main`

| # | Finding | Implementation on HEAD | Regression test run |
|---|---|---|---|
| 1 | Network preset path reached the filesystem through MCP | `crates/control/src/network_paths.rs:24,134`; dispatcher and REST layers; `src-tauri/src/http_api.rs:616` | `tests::network_locations_are_refused_for_clients_other_than_the_window` |
| 2 | `create_session`/`duplicate_session` not retry-safe | `crates/cli/src/lib.rs:3204,3456,3797` (client-scoped key hash) | `tests::mcp_session_creation_is_safe_to_retry_with_the_same_key` |
| 3 | Rust debug text in error messages | `crates/control/src/lib.rs:278`; `crates/domain/src/lib.rs:3414` | `tests::every_application_error_message_is_plain_text_with_its_code_in_data` |
| 4 | Hand-kept, wrong annotations | `crates/cli/src/lib.rs:2841-2934`, `openWorldHint:false` at `2929` | `tests::mcp_read_only_hints_match_the_side_effects_of_the_methods_called` |
| 5 | Default tools demanded technical arguments | generated keys `crates/cli/src/lib.rs:3681,3815`; `with_open_session` `crates/control/src/sessions.rs:1233`; `node` naming | `tests::default_tools_work_without_keys_or_a_session_id` |
| 6 | MCP lifecycle vs JSON-RPC/MCP | notifications `crates/cli/src/lib.rs:2252-2260,2346`; initialize negotiation `2322-2336`; ping; `resources/templates/list` `2408`; -32600 before init | `crates/cli/tests/mcp_stdio.rs` (5 tests) |
| 7 | Over-wrapped tool results | `crates/cli/src/lib.rs:3572-3601` (`{result}`/`{error}`, unreachable = `isError` naming "AudioRouter is running") | CLI `tests` suite plus stdio tests |
| 8 | HTTP status codes collapsed to 400 | `src-tauri/src/http_api.rs:768` `http_status_for_error_code`; OpenAPI error responses and app version | `http_api::tests::backend_error_codes_get_their_own_http_status`, `openapi_covers_every_method_and_docs_are_offline` |
| 9 | Idempotency conflict carried `InvalidRequest` | `crates/control/src/persistence.rs:618`; `ControlError::IdempotencyConflict` `crates/control/src/lib.rs:1173` | control suite |
| 10 | Stale/jargon descriptions | `set_privacy_mute` says Operator (`crates/cli/src/lib.rs:3049`); plain text for `list_devices` `2999`, `inspect_routes` `3028`, `get_operation` `3029`, `get_session` `3024`, `describe_capabilities` `2995` | review (text) |
| 11 | Recipes misnumbered | `OUTPUT_SWITCH_RECIPE` is 15, network 14, plugin update 16 (`crates/cli/src/lib.rs:2943,3481`) | `tests::recipes_are_numbered_once_each_in_order` |
| 12 | Uneven plugin tools | `list_plugins` without a folder reads `plugins.inventory` (`crates/cli/src/lib.rs:3146-3147`); `plugins.parameters` excluded with reason (`MCP_EXCLUDED_METHODS`) | reachability and annotation tests |
| 13 | `GET /api/v1/sessions` cannot page | `src-tauri/src/http_api.rs:259-262,491-501,710-725` | `http_api::tests::get_sessions_pages_with_cursor_and_limit_query_parameters` |
| 14 | `list_saved_versions` hides errors | `crates/cli/src/lib.rs:3322-3345` reports the read's own error | CLI `tests` suite |
| R2-2 | REST could revoke/enroll clients | `DESKTOP_ONLY_METHODS` `src-tauri/src/http_api.rs:145-182`; own-ID refusal `crates/control/src/authorization.rs`/dispatcher | `authorization::tests::a_client_cannot_revoke_its_own_access_but_can_revoke_another` |
| R2-3 | `sessions.start` reported a silent route | `crates/control/src/sessions.rs` refuses unprepared device routes with `nodeNotReady` | `sessions::tests::api_start_refuses_a_device_route_that_is_not_prepared` |
| R2-4 | OBS status unreadable by MCP | default tool `get_obs_status` (`crates/cli/src/lib.rs:2993,3174`) | reachability and annotation tests |
| R2-4 | Virtual bus list | decision below | — |
| R2-5 | REST shared the window's budget | `crates/transport/src/lib.rs:2344-2403` (`adapter_client_id`, `<sid>/http`) | `tests::only_the_hosting_process_can_name_the_http_adapter` |
| R2-6 | Undo retry wording; stale HTTP docs; misfiled doc comment | `crates/cli/src/lib.rs:6813`; `docs/operations/local-http-api.md`; `docs/spec/16-local-http-api.md:36` | `tests::retrying_an_undo_with_the_same_key_explains_itself` |
| R2-1 | Recording/plugin writes no MCP role can use | option (b): `recordings.list/get`, `recorders.list`, `plugins.list/inventory` are Read; eight tools moved out of defaults with reasons | `authorization::tests::stored_recording_and_plugin_reads_need_only_read_and_writes_keep_their_scope` |

### Decision: virtual bus inventory stays an expert tool (R2-4)

`virtualDevices.list` and `virtualRoutes.list` stay in
`MCP_EXCLUDED_METHODS` with the `VIRTUAL` reason and remain available to an
advanced client through `list_virtual_devices` / `list_virtual_routes`
(`crates/cli/src/lib.rs:3002-3003,3151-3152`). Rationale: the methods are
read-only, but the useful workflow (install/provision the driver, replace
cross-session routes) is device administration done in the window, which no
MCP role holds; `get_virtual_cable_status` already answers whether the driver
and cables exist, and adding two context-costly default tools would not let an
assistant act on what it read. This resolves the plan's "decide" item without
widening authority or default context.

## Local branches checked

### `fix/rust-small-defects` — nothing to cherry-pick

The branch holds `4f3b4e8f8`, `225574989`, `bfcd8004b`, `295711b4c`,
`d22a58cc6` on top of `84bfad559`. `git branch --contains 4f3b4e8f8` lists
`integration/overnight-2026-11-24`, and `origin/main` `53e1de80e` is
tree-identical to that integration head (`git diff --stat 1a9c3c7ca
53e1de80e` is empty). Their content is on `main`:

- Instrument classification: `ui/src/appUsage.ts:302,316` uses
  `isRouteSource` with `SILENT_BYPASSES`; `ui/src/obsUsage.ts` uses
  `isRouteSource`; the new OBS test is present.
- Empty `minLength: 1` refusal: `crates/control/src/authorization.rs:152,192,249`.
- Probe reason and Editor create refusal: `crates/control/src/authorization.rs:95`
  (`can_control_sessions`), `crates/cli/src/lib.rs:3390,3439`, and the probe
  text "AudioRouter's control pipe could not be examined".
- Docs: `docs/plans/active/evidence/2026-11-24-rust-small-defects.md` is in
  `53e1de80e`'s tree; later commits `64c3e3553`/`1a9c3c7ca` refined the same
  files (rustfmt fix, CodeQL logging, fuzz nonce).

No commit after `84bfad559` was missing from `main`, so no cherry-pick was
made.

### `feature/mcp-setup-ux` — shipped as PR #114; extra commits superseded

`31849e012` "Improve MCP setup and connection UX (#114)" is an ancestor of
`HEAD` and is the merged form of `b01003b66`/`0139e12aa`. On `main`:

- guided path selector: `ui/src/Workbench.tsx:894-910` (`g6.wbPathLocal`,
  `wbPathCowork`, `wbPathChatgpt`);
- roles and boundaries: `ui/src/Workbench.tsx:245-251`,
  `ui/src/i18n/locales/en.json:2783,2869`;
- exact-client refresh: `ui/src/useWorkbenchStatus.ts:26-37,70` refreshes
  `mcp_setup_info` on the two-second cycle (the local pre-review commit
  refreshed only once);
- role-boundary regression: `ui/src/Workbench.test.tsx:171`.

The three later local commits did not land as-is because `main` already has
equivalents or better versions:

- `b1ee0ef12` + `33a7a17b5` (test timing): `main` has the 5 s/15 s/10 s
  timeouts in `ui/src/App.sessionSelection.test.tsx:211,265,273,334,...`, and
  the Playwright teardown race is fixed in `ui/e2e/browser-test.ts:15,74-75`
  by scoping `unroute`, awaiting in-flight handlers, and tolerating an
  already-handled route instead of `unrouteAll`.
- `1b4b7235e` (UI quality bits): `main` has the dismissible route warning
  (`ui/src/App.tsx:388,1198,4103`), `workbench-loading`
  (`ui/src/Workbench.tsx:731`) and the empty-search state.

Nothing useful was unlanded, so nothing was ported.

## Checks run in this worktree (2026-11-24)

Windows, Rust 1.96.0, Node 22.22.3; `CARGO_BUILD_JOBS=4`.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` and `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | pass |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | pass |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | pass (after `npm --prefix ui run build`, needed for the embedded `ui/dist`) |
| `cargo test -p audiorouter-domain -p audiorouter-control -p audiorouter-cli` | pass |
| `cargo test -p audiorouter-cli --test mcp_stdio` | 5 passed |
| `cargo test -p audiorouter-transport` | pass (private voice fixture ignored by design) |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 142 passed, 4 ignored |
| targeted: `mcp_read_only_hints_match...`, `every_api_method_is_reached...`, `default_tools_work_without_keys...`, `recipes_are_numbered_once...`, `retrying_an_undo...`, `mcp_session_creation_is_safe...`, `network_locations_are_refused...`, `every_application_error_message...`, `a_client_cannot_revoke_its_own...`, `api_start_refuses_a_device_route...`, `stored_recording_and_plugin_reads...`, `only_the_hosting_process_can_name_the_http_adapter` | all pass |
| `npm --prefix contracts run check:drift` | pass: 156 methods, 38 node kinds, 20 processors, 24 event categories |
| `node tools/docs/validate.mjs` | pass: 303 Markdown files, 1695 local links (304/1697 on the final run that includes this evidence file) |
| `npm --prefix ui run format:check` | pass |
| `npm --prefix ui run lint` | pass, zero warnings |
| `npm --prefix ui run build` (typecheck:all + vite) | pass |
| `npm --prefix ui test` | 136 files, 1191 tests passed |
| `node site/generate-catalogs.mjs` | ran; `site/assets/api-methods.json` unchanged. It rewrote 39 `site/tools/*.html` pages without the CSP/social tags that the site build adds, so those generated rewrites were reverted; the committed pages were not regenerated as part of this work. |
| `git diff --check` | pass (after the doc edits below) |

Not run: Playwright browser suites, packaged shell, live owner profile,
external ChatGPT connector, OBS connection, any audio/device/driver test.
Those are the human gates listed in the final report.

## Remaining owner decisions

- **R2-1a — may assistants record?** Option (a), a per-client opt-in scope in
  MCP setup, is not implemented; option (b) already shipped. Requires an
  authorization decision and a decision record (SEC/AUTO-18).
- **R2-4 — session rename for assistants.** Needs a new API method with
  schemas, contracts, docs and a default tool, or a decision to leave rename
  window-only.