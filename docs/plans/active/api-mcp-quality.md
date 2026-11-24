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

Superseded by the second review below: PR #170 merged at `29a77bb9d`
(CI green). The owner now chooses how to resolve finding R2-1.

## Second review: coverage and parity (2026-11-22)

Owner request: "I want to be sure: 1) Work well 2) User can do everything
from both entry points". Read on `main` at `29a77bb9d`, static only.
AudioRouter was running, so nothing was compiled or run. Each item names
the test that should prove it first.

Coverage method: compared the scope of every `API_METHODS` entry (156) with
the grants each entry point really holds, and every backend method the UI
calls (126) with the API. Every UI call is a public API method.

- **REST** forwards with the window's grant (`http_forward` in
  `src-tauri/src/main.rs`; the in-process caller in `dispatch_control_frame`).
  It reaches everything the window does except the 9 methods `http_access`
  refuses (export file and approval, `recordings.setRoot`,
  `devices.setAccess`, the five OBS writes). Once the user has allowed device
  access, every `DeviceAdministration` method works over REST, not only Play.
- **MCP** clients are enrolled with a role only. Storage keeps a role string,
  the setup UI offers Observer, Editor or Operator, and `grant_for_client`
  returns `for_role`. Operator is Read + GraphWrite + SessionControl. No MCP
  client can ever hold Record, PluginScan or DeviceAdministration.

### R2-1 (High): 13 default MCP tools can never succeed

| Tools | Method scope |
|---|---|
| `play`, `toggle_play` | DeviceAdministration |
| `start_recording`, `stop_recording`, `list_recordings`, `list_live_recorders`, `get_recording`, `set_recording_metadata`, `rename_recording` | Record |
| `list_plugins`, `list_plugin_buses`, `list_plugin_programs`, `prepare_plugin_update` | PluginScan |

- Play over MCP is refused by design (AUTO-18, SEC), so it is a documented
  limitation.
- The recording and plugin tools are the defect. Their descriptions say they
  need "recording permission" or "the plugin-scan permission, which the user
  grants", but the user has no way to grant either to an MCP client.
  `list_plugins` tells the assistant to "ask the user to scan in the
  AudioRouter window", which does not change the grant.
- Recipes 1 ("Then play"), 3 (start/stop recording) and 13
  (`list_plugin_buses`, "Operator to play") rely on refused tools. The
  server instructions say "Operator also controls running audio".

**Why CI stayed green:**
- `every_api_method_is_reached_by_a_default_mcp_tool_or_excluded` records a
  method before dispatch, so a refused call counts as reached.
- The read-only hint test drops refused probes.
- The recording tests use hand-built `with_scopes([Record])` grants.

**Proposed test:** run every default tool under `ClientGrant::for_role(Operator)`
(and Observer, Editor). Require success, or a tool listed with the reason it
needs the window.

**Owner decision (authorization change, needs a decision record):**
- (a) Per-client opt-in scopes in MCP setup, for example "may record" and
  "may read the plugin list".
- (b) Move pure reads of stored data to Read: `plugins.inventory` and
  `recordings.list`/`get` metadata. Check first that `plugins.buses` and
  `plugins.programs` do not load plugin code.
- (c) Remove the tools from the default set, list them in
  `MCP_EXCLUDED_METHODS` with reasons, and fix the recipes and instructions.

### R2-2 (High): REST can revoke the window's own enrollment

- `clients.list` is Read, and `clients.revoke` is DeviceAdministration,
  which the REST grant holds after device consent. `http_access` leaves
  `clients.*` open, and `revoke_client` has no guard for the current user's
  SID.
- So a token holder (including a LAN device under HTTP-09) can list the
  shell's SID and revoke it. The running window keeps its in-process grant.
  At the next launch, `main.rs` refuses to start ("This user's AudioRouter
  access was revoked. Restore access with an authorized local
  administrator").
- The same grant lets REST enroll clients (`clients.authorize`), which
  bypasses the window's enrollment confirmation (SEC: "first client
  enrollment" needs specific authorization). It also lets REST call
  `virtualDevices.*` and `virtualRoutes.replace`.
- Fix: refuse `clients.authorize` and `clients.revoke` in `http_access`
  (desktop-only), and refuse revoking the caller's own identity in the
  backend.
- Test: through the HTTP adapter, `clients.revoke` gets 403. A backend
  dispatch revoking the hosting SID is refused.

### R2-3 (Medium): `sessions.start` reports a silent route as running

- `session_start_with_candidate` refuses an unprepared route only when it
  holds an enabled plugin. Otherwise it starts a runtime labelled `"fake"`
  with no device worker. `status.get` and `sessions.summary` then report it
  running or `playing: true`.
- MCP `control_session action=start` and REST `sessions/start` on a route the
  window never prepared therefore return success while nothing plays, and an
  assistant tells the user audio started.
- Fix: refuse with `nodeNotReady` and the "press Play in AudioRouter" text
  when the route has device nodes and no native attachment.
- Test: dispatch `sessions.start` on an unprepared device route and require
  the refusal. Also verify a stop followed by a start, while workers stay
  attached, still restarts native audio.

### R2-4 (Medium): MCP cannot do these window actions

- **Rename a session.** The window does it (`App.tsx`). MCP has only
  `plan_graph_change`, an advanced tool. Add a `rename_session` tool, or
  accept `name` in an existing session tool.
- **Read the OBS status.** `obs.status` and `obs.refresh` are Read scope and
  SEC-15 says MCP has read access. They are excluded as `WINDOW_ONLY` ("refused
  to MCP and HTTP"), which is wrong on both counts: REST serves them. Give
  them a read-only default tool, or correct the exclusion reason.
- **Read the virtual cable buses.** `virtualRoutes.list` and
  `virtualDevices.list` are Read. Only `get_virtual_cable_status` is offered.
  Decide whether an assistant needs the bus list.

### R2-5 (Low): REST shares the window's rate bucket and idempotency scope

- HTTP requests reach the backend under the window's identity. The backend
  mutation bucket (40 burst, 20/s per client) is therefore shared.
- A controller at the adapter's own limit (also 40/20) can make the window's
  edits fail with `rateLimited`, and the reverse.
- Fix: give forwarded HTTP calls their own limiter and idempotency client ID
  (for example `<sid>/http`).

### R2-6 (Low): smaller defects

- An `undo_last_change` or `go_back_to_version` retry with the same key plans
  against the new revision and gets `idempotencyConflict`, not the first
  result. This is safe (no double undo) but the message confuses the
  assistant.
- An unprepared `sessions.start` with a plugin returns a message that names
  `nativePaths.prepare` and HTTP. Neither is available to an assistant.
- `local-http-api.md` is out of date:
  - it says "119 of the 121" methods (there are 156) and "four GET aliases"
    (there are five);
  - it lists 2 desktop-only methods (there are 9).
- HTTP-04 in `docs/spec/16-local-http-api.md` lists 400/403/409/429/503 and
  omits 401, 404, 413 and 500.
- The doc comment of the reachability test sits on
  `default_mcp_adds_and_removes_verified_vst2_key_by_name`.

### Confirmed working (static)

- Every window action is a public API method, so REST covers the window
  apart from the documented desktop-only methods.
- MCP and pipe grants are resolved in the backend from the enrolled role.
- Network-location refusal, error codes, HTTP status mapping, MCP lifecycle
  (notifications, version negotiation, batches) and retry-safe session
  creation read as described above.
- CI on `29a77bb9d` is green.

### Edits written while the owner played (2026-11-22, not compiled or run)

Only `cargo fmt --check` (both manifests) and `tools/docs/validate.mjs` ran.

| Item | Change | Test (not yet run) |
|---|---|---|
| R2-2 | `clients.authorize` and `clients.revoke` are window-only over HTTP (`http_access`, `DESKTOP_ONLY_METHODS`); the dispatcher refuses `clients.revoke` of the caller's own ID with `permissionDenied`. | `a_client_cannot_revoke_its_own_access_but_can_revoke_another`; the existing HTTP policy test walks `DESKTOP_ONLY_METHODS` |
| R2-4 | New default tool `get_obs_status` (`obs.status`, read-only); `obs.refresh` stays excluded with a correct reason. | `default_tools_work_without_keys_or_a_session_id` probe list, reachability and read-only hint tests |
| R2-6 | `local-http-api.md` (window-only list), HTTP-04 status list, SEC client-enrollment note, release notes. | `tools/docs/validate.mjs` passed |

**R2-1, owner chose option (b) (2026-11-22).** Decision record in
`docs/spec/13-security.md`.
- `recordings.list`, `recordings.get`, `recorders.list`, `plugins.list` and
  `plugins.inventory` are now `Read` (domain `API_METHODS`,
  `docs/operations/api-reference.md` and `site/assets/api-methods.json`
  hand-edited: regenerate after the build and diff).
- `plugins.buses` loads plugin code and `plugins.programs` asks a live
  worker, so they stay `PluginScan`. Recording writes stay `Record`.
- Tools no MCP role can use (`start_recording`, `stop_recording`,
  `set_recording_metadata`, `rename_recording`, `list_plugin_buses`,
  `list_plugin_programs`, `prepare_plugin_update`) left `DEFAULT_MCP_TOOLS`,
  got exclusion reasons and an honest description; recipes 3 and 13 and the
  server instructions no longer rely on them.
- Tests: `stored_recording_and_plugin_reads_need_only_read_and_writes_keep_their_scope`;
  `plugins_tests` now lists with a read-only grant. Not run.

**Third pass (owner free to build, 2026-11-22), branch `fix/api-mcp-start-and-limits`.** Run and passing: workspace and `src-tauri` tests, Clippy (both), fmt (both), `check:drift`, and the three real-backend browser specs that press Play (46 tests).
- **R2-3 fixed.** `sessions.start` (REST, MCP `control_session start`) refuses an unprepared route that has an enabled device node (`node_kind_opens_audio_device`, exhaustive match) with `nodeNotReady`; previews, plugin routes (their own message) and device-free routes are unchanged. The window and `sessions.play` prepare first. Four lifecycle tests moved to `session_without_devices()`. Tests: `api_start_refuses_a_device_route_that_is_not_prepared`, `api_start_still_runs_routes_without_audio_devices`, `every_node_kind_is_classified_for_device_use`.
- **R2-5 fixed.** The shell tags HTTP-forwarded frames `adapter: "http"`; the transport budgets them under `<sid>/http` (rate bucket and idempotency scope), believing the tag only from the hosting process (`adapter_client_id`). Tests: `only_the_hosting_process_can_name_the_http_adapter`, `a_forwarding_adapter_is_named_only_when_asked`.
- **R2-6 fixed.** Undo retry with a used key explains itself (`retrying_an_undo_with_the_same_key_explains_itself`); the reachability test doc comment is back on its test.
- **Built exe exercised (2026-11-22).** Release shell built with `custom-protocol` after `npm --prefix ui run build` (exe newer than `ui/dist`); `fresh_install_shell` passed on it. Then a disposable launch (own database and `WEBVIEW2_USER_DATA_FOLDER`, CDP port, output CABLE-A Input and input CABLE Output so nothing audible): window Play went to "Audio running" with no error in the backend log; `sessions.stop` then `sessions.start` returned `native`/`running` (route still prepared, so the guard rightly did not fire); after restarting the shell, `sessions.start` on the unprepared route returned `nodeNotReady` ("Start it with Play…") and the window stayed stopped; `sessions.play` then ran it natively and `sessions.stop` stopped it. Not run: `#[ignore]` hardware/fixture tests beyond `fresh_install_shell`, UI vitest, and the exe was launched with `AUDIOROUTER_ALLOW_DEVICE_ADMIN=1` (the no-variable path is covered only by `fresh_install_shell`).

Still open:
- A per-client opt-in for recording (option a), if assistants should record.
- Session rename for assistants (needs a new API method with schemas, contracts and docs).

### Next action

PR #171 (`c6f59c195`) and PR #174 (`3ef36a1ff`) are merged (status note,
2026-11-24). Owner decisions still open: whether assistants may record
(option a), and whether session rename needs a new API method. Once decided,
the implementation is portable.
