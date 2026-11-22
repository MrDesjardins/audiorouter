# REST API and MCP quality review (2026-11-21)

## Objective

Owner request (2026-11-21): analyse the REST API and the MCP server for
defects and improvements so both are top quality. This page records the
findings and, below, the fixes the owner then asked for. Requirements: AUTO-18 (REST and MCP parity), AUTO-06, the
security rules in AGENTS.md (the backend owns authorization and validation).

## Method and evidence

Read on `docs/m03-driver-signing-purchase-plan` at `9e3305a6f`:
`crates/cli/src/lib.rs` (MCP stdio loop, tool catalog, annotations, result
wrapping, recipes, exclusion list), `src-tauri/src/http_api.rs` (REST
adapter, OpenAPI), `crates/control/src/lib.rs` (authorization, error
mapping), `persistence.rs` (idempotency) and `plugins.rs` (preset path).
`npm --prefix contracts run check:drift` passed (156 methods, 38 node kinds,
20 processors, 24 event categories). No Rust tests were run; each finding
below names the regression test that should prove it first.

Confirmed good: window-only methods (`devices.setAccess`,
`recordings.setRoot`, `obs.*`, export approval) are refused by the backend
for MCP clients, not only by the HTTP adapter
(`crates/control/src/lib.rs:827`, `crates/cli/tests/security_boundaries.rs:193`).
The HTTP adapter checks the exact Host and Origin, compares the token in
constant time and bounds headers and body.

## Findings, by severity

### High

1. **A network preset path reaches the filesystem through MCP.** The HTTP
   adapter refuses UNC and network-drive paths (`http_api.rs:184-192, 523`),
   but that check lives only in the adapter. The default MCP tool
   `apply_plugin_preset` (Operator, `SessionControl`) goes straight to
   `plugins.applyPreset`, which calls `canonicalize()` on `presetPath`
   before the "own directory" check (`crates/control/src/plugins.rs:2395`).
   A `\\host\share\x.fxp` path, which a prompt-injected assistant could pass,
   makes Windows open an SMB connection and offer NTLM credentials before the
   path is refused. Fix: move the network-path refusal into the backend (one
   shared helper next to `local_export_path` in `sessions.rs`) for every
   path parameter (`presetPath`, `path`, `directory`, `shellPath`, mapping
   paths), and keep the HTTP check only as a second layer. Test: dispatch
   `plugins.applyPreset` with a UNC path through `ControlPlane::dispatch` and
   through `mcp_tool_call`, and require a refusal with no filesystem call.
2. **`create_session` and `duplicate_session` are not retry-safe.** Both
   generate a fresh session ID on every call (`crates/cli/src/lib.rs:3026,
   3248`). Repeating a call with the same `idempotencyKey`, which is how an
   assistant or script retries after a timeout, sends a different request
   under the same key. The backend then refuses it with "idempotency key is
   already used for a different request". Fix: derive the new session ID from
   the key, for example a hash of it. Test: call each tool twice with one key
   and require the same session and no error.
3. **Error messages are Rust debug text.** `application_error_response`
   builds every message other than audio, node-not-ready and restart errors
   with `format!("{error:?}")` (`crates/control/src/lib.rs`, after the code
   mapping). REST callers and assistants receive text such as
   `Store(RevisionConflict { expected: 112, actual: 113 })` or
   `InvalidRequest("cursor must be a string")`. The desktop UI hides these
   (`ui/src/App.accessibility.test.tsx:167`), but scripts and LLMs read them
   raw, against the recipes' promise that "a failed call names the problem".
   Fix: implement `Display` on `ControlError`/`StoreError` with plain
   sentences, and keep the machine code in `error.data.code`. Test: assert
   message text for each error kind, with no `(` or `{` debug syntax.

### Medium

4. **The MCP tool annotations come from a hand-kept list and are wrong in
   places** (`crates/cli/src/lib.rs:2721`). Problems:
   - `get_recipes`, `list_sessions`, `get_session_summary`, `list_tool_kinds`
     and `get_levels` are not marked read-only, so clients ask the user to
     confirm plain reads.
   - The `toggle_*` tools are marked idempotent, and so are `add_tool` and
     `create_session` when called without a key.
   - `pump_native_*` are marked read-only although they drain audio.
   - `delete_my_preset` and `remove_tool` are not marked destructive.
   - `openWorldHint` is missing.

   Fix: derive `readOnlyHint` from `API_METHODS` `side_effect`, mark the
   toggles and creates explicitly, and test that every default tool's hint
   matches the side effect of the method it calls.
5. **Default tools make assistants supply technical arguments.** The design
   says "a forgotten technical argument never fails the request", but several
   default tools break it:
   - `control_session`, `set_privacy_mute`, `cancel_operation`,
     `commit_session_import`, `set_recording_metadata` and `rename_recording`
     require `idempotencyKey`.
   - `control_session`, `get_session`, `export_session` and `inspect_routes`
     require `sessionId`, although the other tools default to the open
     session.
   - `inspect_routes` takes `destinationNode` and does not accept a node name.
   - `start_recording` and `stop_recording` take `nodeId` where the other
     tools take `node`.

   The `play` description and recipe 6 tell the assistant to use
   `control_session action=stop` with no key, so a call written that way
   fails schema validation. Fix: generate keys and default the session as
   the simple tools already do, and accept `node`.
6. **The MCP lifecycle does not follow JSON-RPC and the MCP spec.**
   - Any notification other than `notifications/initialized` (for example
     `notifications/cancelled`) falls to the `_` arm and gets an error reply
     with `id: null` (`lib.rs:2347`). JSON-RPC forbids replying to
     notifications.
   - `initialize` with an unknown version returns -32602 (`lib.rs:2287`).
     The spec has the server answer with a version it supports, so a newer
     client fails to connect.
   - `ping` is refused before initialization.
   - `resources/templates/list` returns "method not found".

   Check each against the current MCP lifecycle text before fixing.
7. **Tool results are wrapped too heavily** (`lib.rs:3364`).
   - `structuredContent` is the whole JSON-RPC envelope (`jsonrpc`, `id`,
     `result`), and the text block repeats it, which doubles the tokens.
   - No tool declares an `outputSchema`, although `api_output_schema.rs`
     already has one for every method.
   - A backend transport failure (the app not running) is a -32003 protocol
     error (`lib.rs:3358`) instead of `isError: true`, so many clients never
     show it to the model. The message should say "Is AudioRouter running?".
8. **HTTP status codes collapse to 400** (`http_api.rs:565`). `notFound`,
   `idempotencyConflict`, `planExpired`, `internalError`, `storageFailure`
   and `corruptDatabase` all return 400. They should be 404, 409, 409/410,
   500 and 503. OpenAPI (`http_api.rs:237`) declares no 401, 404, 413 or 5xx
   response and no error body schema, and its `info.version` is a fixed
   `1.0.0` instead of the app version. 429 has no `Retry-After`.
9. **Idempotency conflicts carry the wrong code.** `persistence.rs:564`
   returns `InvalidRequest` for a key reused with a different request,
   although `ControlError::IdempotencyConflict` exists. Callers cannot tell
   it apart from a malformed request.

### Low

10. **Descriptions are stale or use jargon.**
    - `set_privacy_mute` says it "requires capture scope", but `API_METHODS`
      lists `SessionControl`.
    - `list_devices` says "authoritative audio endpoint descriptors".
    - `inspect_routes` says "desired upstream route provenance".
    - `get_operation`, `get_session`, `plan_session_import` and
      `describe_capabilities` are one-line technical summaries.

    AUTO-18 asks for plain-language default tools.
11. **The recipes are misnumbered.** `NETWORK_PAIRING_RECIPE` is numbered 8,
    duplicating check_setup's 8, and is printed after 13 (`lib.rs:3536`).
12. **Plugin tools are uneven for assistants.**
    - `list_plugins` is a default tool but needs a `directory` the assistant
      cannot discover.
    - `plugins.inventory` is excluded.
    - `plugins.parameters` is excluded, so an assistant cannot read a VST's
      parameter names to change them.

    Decide whether an Operator assistant should see the remembered inventory
    and a plugin node's parameters.
13. **`GET /api/v1/sessions` cannot page.** Query strings are refused and
    GET has no body (`http_api.rs:420`). Only the POST form takes a cursor.
    Document it, or accept `cursor` and `limit` as query parameters.
14. **`list_saved_versions` hides errors.** When the active-session read
    fails for any reason, including a permission refusal, it reports "no
    session is open" (`lib.rs:3143`).

## Implementation (2026-11-21, owner: "Fix all of them")

Branch `fix/api-mcp-quality` from `origin/main`. The owner chose "edit now,
build later" because AudioRouter was carrying a live call and game, so **the
edits are written but nothing is compiled or tested yet**. Only the
`cargo fmt -- --check` runs (workspace and `src-tauri`) have run, and both are clean.

| # | Change | Regression test (not yet run) |
|---|---|---|
| 1 | New `crates/control/src/network_paths.rs`: `is_network_path` (UNC in any slash spelling, `\\?\UNC\`, `GLOBALROOT`, remote or unmapped drive letters) and `network_location_parameter` (walks nested params for keys ending in `path`, or containing `directory`/`folder`, or equal to `root`). The dispatcher refuses such params with `permissionDenied` + `fieldPath` for every grant except the window. HTTP uses the same helpers as a second layer (403). | `network_locations_are_refused_for_clients_other_than_the_window`, `every_unc_spelling_is_a_network_path`, MCP `default_tools_work_without_keys_or_a_session_id`, HTTP network-path test extended to nested `graph.plan`/`nodes.set` |
| 2 | `create_session`/`duplicate_session` derive the session ID from the idempotency key (`mcp_session_id_for_key`). | `mcp_session_creation_is_safe_to_retry_with_the_same_key` |
| 3 | `Display` for `ControlError` and `StoreError`; responses and logs use it. | `every_application_error_message_is_plain_text_with_its_code_in_data` |
| 4 | Annotations corrected (reads, toggles, creates, pumps, destructive set, `openWorldHint: false`, `title`). | `mcp_read_only_hints_match_the_side_effects_of_the_methods_called` (walks every tool's real backend calls) |
| 5 | Keys generated where missing; `sessions.get/export/start/stop` and `routes.inspect` default to the open session in the backend (`with_open_session`); `routes.inspect` resolves a node name; recording tools take `node`. | `default_tools_work_without_keys_or_a_session_id` |
| 6 | Notifications never get a reply or run; version negotiation; `ping` before init; `resources/templates/list`; not-initialized is -32600; unknown tool is -32602; server `instructions`. | `crates/cli/tests/mcp_stdio.rs` lifecycle steps |
| 7 | `structuredContent` is `{result}` or `{error}` only; unreachable backend is `isError` with "check that AudioRouter is running". | updated envelope test |
| 8 | `http_status_for_error_code` (404, 409, 429 + `Retry-After`, 500, 503); OpenAPI error responses, `Error` schema and the app version. | `backend_error_codes_get_their_own_http_status`, OpenAPI test |
| 9 | Idempotency reuse returns `ControlError::IdempotencyConflict`. | `recording_tests.rs` conflict assertion |
| 10 | Plain-language descriptions; `set_privacy_mute` names Operator. | review |
| 11 | Recipes renumbered 1–16. | `recipes_are_numbered_once_each_in_order` |
| 12 | `list_plugins` without `directory` reads `plugins.inventory`, which is no longer excluded. | annotation/reachability tests |
| 13 | `GET /api/v1/sessions?cursor=&limit=`; any other query string still returns 400. | `get_sessions_pages_with_cursor_and_limit_query_parameters` |
| 14 | `list_saved_versions` reports backend failures instead of "no session is open". | covered by the unreachable-backend path |

Also changed: `get_session` and recording/setup lookups return
`Store(SessionNotFound)` (code `notFound`) instead of an uncoded
`InvalidRequest`. Contracts, `api-reference.md`, `local-http-api.md` and
`docs/spec/13-security.md` are updated.

### Self-review before the build (2026-11-21)

- `\??\UNC\host\share` (the NT form Win32 passes through) got past
  `is_network_path`; now refused, with the test spellings added.
- Refusing every network value would have blocked an assistant's
  whole-session `plan_graph_change` on any session that already holds a
  network file the owner picked in the window. `new_network_location`
  allows values already stored in the targeted session and refuses only new
  ones (dispatcher cases in
  `network_locations_are_refused_for_clients_other_than_the_window`). HTTP stays strict: it cannot see the session.
- `routes.inspect` `destinationNode` bound back to `MAX_ENTITY_ID_BYTES`,
  like every other node reference and the existing catalog test.
  An unknown node is now a plain error that lists the nodes.
- The read-only hint test judged chained tools (undo, toggles) by a probe
  that stopped after its read; it now judges by successful probes only.
- The REST query decoder accepted `%+1`; it now requires two hex digits.

Second pass (owner: "do more search"):
- The exemption applied to any parameter, so `plugins.inspect` or a
  `presetPath` naming a share the open session holds would have passed. It
  now applies only under `/candidate/`; a dispatcher case covers the direct
  form.
- Location keys matched only `…path`, `directory`, `folder`, `root`;
  `directories`, `presetFile`, `paths` slipped through. Now any key
  containing path, director, folder, file, root, location or dll.
  `fieldPath` pointers are RFC 6901 escaped.
- `mcp_session_id_for_key` hashed only the key, but the backend scopes keys
  per client: two assistants using `create-1` would collide. The client ID
  is now part of the hash.
- `with_open_session` now matches `simple_session_id` (empty counts as
  missing, same message).
- Docs: the 400 row named only code-less errors, though `invalidGraph` and
  others are 400 too; the exemption and unmapped drives are documented.
- Checked and fine: imported session files cannot smuggle a network plugin
  (placeholders must match a scanned inventory; media is stored by ID);
  authorization checks parameter names, not per-session access, so the
  open-session default cannot widen a grant; the read-only hints match
  `API_METHODS` for every directly mapped tool (script check); every default
  tool except `open_session`, `duplicate_session` and `delete_session`
  works without a session ID or key; the UI branches on codes, not text;
  `node tools/docs/validate.mjs` passes.

Third pass (owner: "be very critical"):
- HTTP still answered 400 for state and system errors that are not the
  caller's fault. Now `deviceInUse`, `exclusiveModeOnly`, `nodeNotReady`,
  `noUndoAvailable` and `planLimitReached` are 409, `accessDenied` is 403,
  `serviceUnavailable` and `deviceInvalidated` are 503.
- The adapter's own refusals (401, 403, 404, 405, 413, 429, 503 busy or
  unavailable) had no `data.code`, contradicting "branch on data.code".
  `adapter_error_body` now gives every one a code; the network refusal names
  the parameter. Tested through the adapter.
- MCP silently dropped a JSON-RPC batch (allowed by 2025-03-26, which we
  negotiate), so such a client hung. It now gets -32600 (stdio test).
- `invalidGraph` messages were unbounded; now the first five problems and
  "and N more" (domain test).
- The unknown-tool check rebuilt the whole tool catalog per call; names are
  cached once.
- Overclaims corrected: messages are "plain text" (many `invalidRequest`
  texts are terse phrases, not sentences); the CLI acting on its own
  database is not refused network paths; server instructions no longer say
  every `sessionId` defaults to the open session (it filters list tools).
- Breaking changes for integrations are listed in
  `docs/operations/release-notes.md` (Unreleased).

Deliberately not changed:
- No aliases for the renamed MCP arguments (`destinationNode`, `nodeId` on
  recording tools): MCP clients read the schema each session, and aliases
  would show the model two names for one thing. Listed as breaking.
- A buggy client sending `notifications/initialized` with an `id` still
  gets no reply (it is a notification by spec).
- `invalidRequest` texts stay as the handlers write them; rewriting about a
  thousand messages is out of scope.

### Decisions

- No MCP `outputSchema`: results wrap `result`/`error`, some tools combine
  several methods, and clients may reject a result that misses a declared
  schema. The spec recommends repeating the JSON in a text block, so that
  block stays.
- `plugins.parameters` stays excluded: it loads plugin code in a worker.
- `remove_tool` and `disconnect_nodes` are not marked destructive; undo
  restores them.
- Native graph rejections still include the engine's error name
  ("native graph rejected: UnsupportedTopology"); the UI matches on it.

## Validation matrix (pending, run when the owner is free)

1. `cargo build --workspace` and `src-tauri`.
2. `cargo test -p audiorouter-domain -p audiorouter-control -p audiorouter-cli`
   and the `src-tauri` `http_api` tests. Expect fixes where older tests
   assert the old -32602 "session not found", debug-text messages or the
   old MCP envelope.
3. `cargo clippy` for workspace and `src-tauri` with `-D warnings`.
4. `npm --prefix contracts run check:drift`, the UI typecheck, `node tools/docs/validate.mjs`.
5. Regenerate `docs/operations/api-reference.md` and the site catalogs from
   the generator, and diff against the hand edits.

## Rollback

Revert the branch. Machine codes in `error.data.code` are unchanged, apart
from the new `notFound` and `idempotencyConflict` codes, so scripts that branch on codes are unaffected.

## Next action

Run the validation matrix once the owner says the PC is free, fix failures,
then report. No commit or PR without the owner's request and approval.
