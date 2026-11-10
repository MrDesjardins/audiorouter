# Active plan — code review 2026-11-09 (owner request)

Owner request, 2026-11-09: "Do a full code review to find potential bugs or
issues, create a plan to fix those." Baseline: `main` at `feaf2108`
(after PR #130). This file is the review record and the fix plan. No product
source was changed, and nothing was built or run for it.

## How this review was done

Risk-based reading, not a line-by-line proof. Read in full: everything added
since the 2026-11-03 review (`goals.rs`, `compare.rs`, `saved_versions.rs`,
`tool_presets.rs`, `client_credentials.rs`, `ui_draft.rs`, `draftRecovery.ts`,
`ToolCompare.tsx`, `ChangeHistory.tsx`, `Slider.tsx`, `compareRules.ts`, and
the diffs of `sessions.rs`, `native_paths.rs`, `native_dispatch.rs`,
`transport`, `storage`, `domain`, `network_audio.rs`, shell `main.rs`), plus
`http_api.rs`, `http_accept.rs`, `simple.rs`, `audio_service.rs`,
`recording_library.rs`, the commit path in `graph.rs`, the engine Soundboard
and audio-file sources, and the Time Shift DSP.

Not reviewed in depth: `crates/windows-audio/src/lib.rs`, the engine
compiler, `plugin-host`, the driver, most of `App.tsx`/`Workbench.tsx`, CSS,
the site and release tooling. The three earlier reviews (2026-10-07,
2026-11-01, 2026-11-03) covered parts of those.

Every finding below was confirmed by reading the code path end to end. None
was reproduced by a test or in the running app; the first task of each fix is
the failing regression (AGENTS.md, "Done means exercised").

## Requirement IDs

SEC-13 and HTTP-04/06/09 (local API trust boundary), AUTO-18 (REST and MCP
parity), DEC-UI-21b (hold-to-compare), UI-07 (saved versions), UI-05 (tool
presets), SND-16/17 (Soundboard modes and sets), UI-17, ARCH-04 (nothing
panics on the audio path), STATE-02 (draft durability).

## Findings

Status key: **todo**, **decision** (needs the owner), **done**.

### P1 — security

| ID | Finding | Status |
| --- | --- | --- |
| R1 | **A local-API token holder can make the PC load code from a path it chooses.** The HTTP adapter forwards with the desktop grant, and only the nine names in `DESKTOP_ONLY_METHODS` (`src-tauri/src/http_api.rs:144`) are refused. `plugins.scan` (`crates/control/src/plugins.rs:1122`) accepts any absolute directory, including `\\host\share`. A scan result is all `validate_plugin_placeholders` needs, so: scan a share, add a Plugin node, `sessions.play`, and the DLL runs in the plugin worker as the user. The scan alone makes an outbound SMB connection (NTLM hash exposure). With the opt-in LAN listener the token crosses the network unencrypted (HTTP-09 says so), so a peer on the same network can capture it. `plugins.inspect` and `sessions.importFile` take caller paths the same way; `clients.authorize`, `startup.apply` and `virtualDevices.*` are also reachable although SEC-13 asks for a specific decision at "first client enrollment". Export and the recording root already refuse network paths (`sessions.rs:16`, `persistence.rs:442`); these three methods do not. | todo (decided 2026-11-09) |
| R2 | **The desktop-only list is hand-kept.** A new method is HTTP-reachable by default; nothing fails when one is added. The 403 text is also wrong for five of the nine ("Choose the recording folder…" for `devices.setAccess` and `sessions.exportFile`, `http_api.rs:403`). | todo |

### P2 — correctness

| ID | Finding | Status |
| --- | --- | --- |
| R3 | **A refused delete still destroys state.** `delete_session` (`crates/control/src/sessions.rs:219`) calls `clear_comparison` and `forget_revision_notes` before the "stop the session before deleting it" checks and before the storage delete. Deleting a playing session through REST or MCP returns an error, yet the saved-version labels (who and when) are gone from the database, and a held comparison is dropped without republishing, so the route keeps playing with those tools bypassed and nothing left to expire it. | todo |
| R4 | **One comparison slot for every session.** `ControlPlane::comparison` is a single `Option`, and `dispatch_nodes_compare` (`compare.rs:167`) overwrites it. Holding on session A, then holding or releasing on session B (two sessions can play at once), forgets A without republishing it: A stays bypassed until its next save or Stop. The module comment promises a crash or closed window "cannot leave a voice route unprocessed". | todo |
| R5 | **Every window request blocks the shell's main thread.** `rpc_request` and the other non-`async` Tauri commands run inline on the main thread (Tauri 2 `ExecutionContext::Blocking`, confirmed in `tauri-macros` 2.6.3). A slow backend call (plugin scan, export, Play opening devices) stalls the native window frame, the tray menu and every queued request, including meters. | todo (measure first) |
| R6 | **Output Switch output B cannot be wired by a script or assistant.** `connect` (`simple.rs:192`) always takes the first output port, and `connections.add` has no port parameter. An assistant can add an Output Switch but only ever connect A (AUTO-18). `sessions.summary` does not show the source port either. | todo |

### P3 — smaller

| ID | Finding | Status |
| --- | --- | --- |
| R7 | **A looped Soundboard sound with zero frames panics the audio callback.** `resample_linear` (`audio_file_source.rs:219`) returns no frames for a 1–3 frame file above the graph rate, and `decode_audio` does not recheck. `prepare_soundboard` accepts it; a Loop voice then computes `% length` with `length == 0` (`soundboard.rs:298`, `:306`). Once and Hold modes are safe. Needs a degenerate file, but the result is a backend restart. | todo |
| R8 | **Two presets can share one ID.** `toolPresets.save` builds the ID from `name.to_lowercase()` but replaces by `eq_ignore_ascii_case` (`tool_presets.rs:191`, `:205`). "Été" and "été" are both kept under one ID; the picker shows duplicate keys and Delete removes both. A stored list that fails to parse also reads as empty, so the next save overwrites every preset. | todo |
| R9 | **A commit can succeed and report failure.** In `commit_graph_scoped` (`graph.rs:435`) a failed `delete_graph_plan` returns an error after the session and journal are saved. The live route is not updated, no `graph.committed` event is sent, and no version note is written. A retry replays the journal and still skips them. | todo |
| R10 | **The unsaved-edits file is not flushed.** `ui_draft::save` writes the temporary file without `sync_all` before the rename, so a power loss can leave an empty file, which reads as "nothing to restore". | todo |
| R11 | **The compare button stays pressed after the backend ends the hold.** The backend restores the route after 30 s (`MAX_HOLD`); `ToolCompare` keeps `held` and the canvas marking until release. | todo |
| R12 | **The update-check cache is trusted by shape.** `readCache` (`ui/src/updateCheck.ts:59`) checks only `checkedAt`; a stored `update` without a string `tag` throws inside the `useState` initializer at startup, and `update.url` becomes a link unchecked. | todo |
| R13 | **Idle HTTP connections can starve the API.** Four workers each wait up to 2 s for headers before the token is checked, so a client that opens sockets and sends nothing keeps real callers on 503. Local or LAN peers only. | todo (low) |

## Owner decision (2026-11-09)

R1: **reject network paths only.** Loading a DLL that is already on a local
disk is accepted. No method becomes desktop-only, and the window must stay as
easy as it is now: a person who keeps plugins or session files on a NAS or a
mapped drive, and picks them in the AudioRouter window, must still be able to.

Consequence for the design: the check lives in the **HTTP adapter**, not in
the backend handlers. The HTTP adapter is the only remote surface (these
methods are excluded from MCP, and a raw pipe caller is already a local
process of this Windows user). A backend check would also refuse the window's
own folder picker, which the owner ruled out.

Not changed by this decision: export and the recording root keep their
existing backend refusal of network paths for every caller.

## Handoff: instructions for the implementing agent

Read `AGENTS.md` first; it overrides anything here. Then:

- Branch `fix/code-review-2026-11-09` from `main`. Carry this file and the
  `current.md` index line onto the branch (they are uncommitted on `main`).
- One commit per task below, each with its regression test. No pull request
  until the owner approves one. No `Co-Authored-By` or "Generated with" lines.
- For every task: write the test, watch it fail, fix the owning layer, compile
  at once. Use the Edit/Write tools, never a patch script.
- The line numbers in the findings are from `feaf2108`; re-find the code by
  name before editing.
- The owner often has the app running on this PC. Ask before a workspace
  build or full test run; never launch a second `audiorouter-shell`.
- After each task, set its status in the findings tables above to **done**
  with the commit hash, and record command, result and anything not run under
  "Progress" at the end of this file.
- If a finding does not reproduce, do not fix it on a guess: record what the
  test showed and mark it **not reproduced**.

### Task 1 — R3: a refused delete must change nothing

File: `crates/control/src/sessions.rs`, `delete_session`.

- Move `clear_comparison(id)` and `forget_revision_notes(id)` to after
  `self.store.remove_session(id)` succeeds.
- Tests in `sessions_tests.rs`, through `ControlPlane::dispatch`:
  1. with storage, save two revisions, start the session, call
     `sessions.delete`: it is refused, and `graph.versions` still returns
     `savedAtMs` for those revisions;
  2. a storage failure during delete keeps the notes too;
  3. a successful delete removes the `revisionNotes:<id>` value.
- The held-comparison half is covered by Task 2's tests.

### Task 2 — R4: comparisons per session

Files: `crates/control/src/compare.rs`, `lib.rs`, `persistence.rs` (both
construct `comparison: None`).

- Replace `comparison: Option<Comparison>` with a map keyed by session ID
  (drop the `session` field from `Comparison`).
- `with_comparison`, `clear_comparison` and `dispatch_nodes_compare` use the
  entry of their own session only.
- `expire_comparison` runs on every audio service pass: return at once when
  the map is empty, and collect expired IDs without allocating when none
  expired. Republish each expired session as today.
- Tests in `compare_tests.rs` (see how the existing ones get a running
  session): hold on A, hold on B, release B: A is still held and still
  expires and republishes; a refused `sessions.delete` on A leaves A held.

### Task 3 — R1 and R2: network paths and the HTTP method list

Files: `src-tauri/src/http_api.rs` (+ a small module if it grows),
`docs/spec/13-security.md`, `docs/spec/16-local-http-api.md`.

1. Add `fn network_path(text: &str) -> bool` in the shell: true for a path
   starting with `\\` or `//` (this covers UNC, `\\?\UNC\…` and `\\.\`
   device paths), and for a drive letter whose `GetDriveTypeW` is
   `DRIVE_REMOTE` (4) or `DRIVE_NO_ROOT_DIR` (1). `sessions.rs`
   `local_export_path` shows the call and its SAFETY comment. Keep the unsafe
   block that small.
2. Replace `DESKTOP_ONLY_METHODS` with one classification of every method:
   `fn http_access(method: &str) -> HttpAccess` where `HttpAccess` is
   `Open`, `LocalPaths(&'static [&'static str])` (the parameter names that
   hold a path) or `DesktopOnly(&'static str)` (the 403 text for that
   method). Keep a `DESKTOP_ONLY_METHODS`-style accessor if other code
   (`crates/cli` comments, tests, `openapi`) needs the list.
   - `DesktopOnly`: exactly today's nine, each with a true message. No
     additions (owner decision).
   - `LocalPaths`: `plugins.scan` and `plugins.list` (`directory`),
     `plugins.inspect` (`path`), `sessions.importFile` (`path`). Check
     `plugins.inventory`, `plugins.retry` and `sessions.importPlan` in
     `api_schema.rs` and classify them the same way if they take a path.
3. In `handle`, after the body is parsed and before `forward`: for a
   `LocalPaths` method, if any named parameter is a string for which
   `network_path` is true, answer 403 with
   "Over the API, choose a folder on this PC. Network folders can be chosen
   in the AudioRouter window."
4. The generated check (this is R2): a test that walks `API_METHODS` and its
   input schemas from `system.describe`, and fails when a method has a
   string property named `path`, `directory`, `root`, `newPath` or ending in
   `Path`/`Directory` and is classified `Open` without appearing in a short,
   commented allowlist (for example `recordings.rename`, whose destination
   the storage layer confines to the recording's own folder). A new
   path-taking method then cannot ship unclassified.
5. Tests through the real listener (the existing `fixture()`/`call` helpers):
   `plugins.scan` with `\\\\host\\share` and with `//host/share` gets 403 and
   the backend closure is never called; a local path is forwarded; each
   desktop-only method returns its own message; `/openapi.json` still omits
   desktop-only methods and still lists `plugins.scan`.
6. **Window unchanged:** add a test at the `ControlPlane::dispatch` level
   that `plugins.scan` with a UNC-shaped path is *not* refused by the backend
   for the network shape (it may fail for the missing folder). This pins the
   owner's requirement so a later "hardening" does not move the check.
7. Docs: SEC-13 "The localhost API" bullet and HTTP-06 gain one sentence:
   path parameters over HTTP must be on this PC. Regenerate
   `docs/operations/api-reference.md` and run `check:drift` only if the
   generator output changes (no method or schema is added here).

### Task 4 — R7: zero-frame sounds

Files: `crates/engine/src/audio_file_source.rs`, `soundboard.rs`.

- `decode_audio`: return `AudioFileDecodeError::Empty` when `resample_linear`
  yields no frames.
- `SoundboardSource::new`: drop sounds with `frames() == 0`; a slot left with
  none becomes `None`. (Defence in depth: `DecodedAudio` is a public struct.)
- Tests: a 1-frame 96 kHz WAV decoded for 48 kHz is `Empty`; a
  `SoundboardSource` built by hand with an empty sound in a Loop slot renders
  a block without panicking and reports the slot as not loaded.
- Run `cargo test -p audiorouter-engine --test source_combinations`.

### Task 5 — R6: name a port in `connections.add`

Files: `crates/control/src/simple.rs`, `api_schema.rs`,
`api_output_schema.rs` if the summary shape is typed there,
`contracts/src/index.ts`, `crates/cli/src/lib.rs` (MCP tool text and
recipes).

- `connections.add` and `connections.remove` accept optional `fromOutput` and
  `toInput` (port names). Absent: today's behaviour exactly. Unknown name:
  refuse and list the node's ports.
- The duplicate rule becomes "same source port to same destination port",
  so A→x and B→y from one Output Switch are both allowed.
- `nodes.add` with `after`/`between` is unchanged.
- `sessions.summary` connections gain `fromOutput` beside `toInput`.
- Add a `get_recipes` entry: "Send one source to either of two outputs
  (Output Switch)". Update the default MCP tool's description.
- Tests: `simple_tests.rs` through `dispatch` (wire A and B, remove B only);
  one `mcp_tool_call` test doing the same; `npm --prefix contracts run
  check:drift`; regenerate `api-reference.md` and the site catalogs.

### Task 6 — R9: plan cleanup after a durable commit

File: `crates/control/src/graph.rs`, `commit_graph_scoped`.

- After `save_session_with_journal_with_hash` succeeds, a failed
  `delete_graph_plan` must not return early: ignore it (the plan expires by
  its own `expires_at`). Check that a leftover plan row cannot be committed
  twice (the base revision no longer matches) and say so in a comment.
- Test: if the storage crate has a fault hook used by the Q4 storage-failure
  tests, use it; otherwise delete the plan row out from under the commit is
  not a failure case, so add a narrow test seam rather than skipping the
  test. The assertion: the response is a success, `graph.committed` was
  appended, and `graph.versions` labels the new revision.

### Task 7 — R8: preset identity

File: `crates/control/src/tool_presets.rs`.

- One fold for both the ID and the replace check: compare
  `name.to_lowercase()` in both places.
- `load_tool_presets`: when the stored text does not parse, `list` returns
  empty but `save` refuses with "your saved presets could not be read" rather
  than overwriting. Do not delete the stored value.
- Tests through `dispatch`: saving "Été" then "été" leaves one preset; a
  corrupt stored list is still there, byte for byte, after a refused save.

### Task 8 — R5: backend calls off the shell's main thread

File: `src-tauri/src/main.rs`.

- Measure first, on the built shell with a disposable database
  (`AUDIOROUTER_DATABASE`, `WEBVIEW2_USER_DATA_FOLDER`, no other
  `audiorouter-shell` running): log the thread ID and duration inside
  `rpc_request` during a plugin scan of a large folder, and try to drag the
  window. Record the numbers here. If the main thread is not the caller, or
  the stall is under 50 ms, mark R5 **not reproduced** and stop.
- If confirmed: `#[tauri::command(async)]` on `rpc_request` and the other
  commands that call `forward_rpc_request*` and do not need the main thread.
  Leave commands that touch windows, dialogs or the tray where they are.
- Ordering: commands may now overlap. Before relying on it, check
  (a) `set_ui_draft` writes then clears: give `ui_draft::save` a process-wide
  mutex and a unique temporary name; (b) `ToolCompare` hold then release:
  chain the release on the hold's promise in the UI.
- Verify in the built exe: meters keep moving and the window drags during a
  scan; the three-theme check is not needed (no visual change).

### Task 9 — R10, R11, R12

- **R10** `src-tauri/src/ui_draft.rs`: write through a `File`, `sync_all`,
  then rename. Existing tests must still pass.
- **R11** `ui/src/ToolCompare.tsx`: use `expiresInMs` from the hold's result
  to call the local release path when it passes (clear `held`,
  `setComparing([])`; no backend call is needed, but one is harmless). Vitest
  with fake timers. The button row must not move (UI-17).
- **R12** `ui/src/updateCheck.ts`: `readCache` builds the value from checked
  fields: `checkedAt` number, `forVersion` string, `update` either null or
  an object whose `tag` matches `/^v\d+\.\d+\.\d+$/`; rebuild `version` and
  `url` from the tag instead of trusting stored ones. Test with a stored
  `{"checkedAt":1,"forVersion":"…","update":{}}`.

### Task 10 — R13: idle HTTP connections (low)

File: `src-tauri/src/http_api.rs`, `handle`.

- First read: 250 ms timeout; once any byte arrives, the existing 2 s
  deadline applies. Test: four silent sockets, then a real request succeeds
  within one second.
- Skip this task if it makes the existing slow-client tests flaky; record why.

## Validation matrix

| Check | After tasks |
| --- | --- |
| `cargo test -p audiorouter-control --lib` | 1, 2, 3 (step 6), 5, 6, 7 |
| `cargo test -p audiorouter-engine` (all, incl. `source_combinations`, `tool_combinations`) | 4 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 3, 8, 9, 10 |
| `cargo test -p audiorouter-cli` | 5 |
| `npm --prefix contracts run check:drift` | 5 |
| `npm --prefix ui test`, `run lint`, `run typecheck:all`, `run format:check` | 5 (if UI types change), 8, 9 |
| `cargo fmt --all -- --check`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | every Rust commit |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings`, same for `src-tauri` | before handoff |
| `tests/acceptance/docs.ps1` | 3, 5 |
| Built exe (`--features custom-protocol`, exe newer than `ui/dist`): scan a local plugin folder in the window; scan a network folder in the window if one exists; API tab: `plugins.scan` with a `\\` path returns 403; hold-to-compare; delete while playing | before handoff |

No audio hardware, driver or credentials are needed. State plainly in the
handoff which rows were not run.

## Definition of done

Every finding is **done** or **not reproduced** with evidence; the matrix
rows are recorded; the complete diff was read; the window's plugin and
import flows behave exactly as before (Task 3, step 6 and the built-exe
row). Then report branch, commit, results and limits, and wait for the
owner's approval before opening a pull request.

## Risks

- Task 3: a script that scans a network folder over HTTP stops working. None
  is known; the 403 says what to do instead.
- Task 8 changes command ordering from serial to concurrent; that is why it
  starts with a measurement and names the two order-dependent callers.
- Task 2 touches the audio service pass; keep the empty case free.

## Rollback

Each task is its own commit; revert that commit. No migration, no stored-data
change. Task 5 adds optional parameters only.

## Progress

### 2026-11-09 — execution started; Task 1 (R3)

Branch: `fix/code-review-2026-11-09`. Owner authorized workspace builds and
full tests while leaving the existing release shell untouched. Windows,
pinned Rust and installed dependencies are available. The running shell is
in the separate `audiorouter-release-028` checkout; do not launch another.

UI-07 / DEC-UI-21b: added a dispatcher regression saving two revisions,
refusing deletion while playing, injecting a SQLite DELETE trigger failure,
and finally deleting successfully. The regression failed before the fix
(stored revision notes became `None` after refusal), then passed after
moving comparison/note cleanup after successful removal. Command:
`cargo test -p audiorouter-control --lib refused_or_failed_delete_preserves_saved_version_notes -- --nocapture`.
One test passed on Windows. Full control suite: 364 passed, 11 explicitly
ignored, no failures. Both Rust format commands and their check variants,
plus `git diff --check`, passed; formatting touched only the new test.
Held comparison coverage follows in Task 2. Rollback:
revert the Task 1 commit; no data migration.

Task 1 commit: `80b99f19`. Backend regression coverage is complete; the
built-shell delete/compare handoff check remains part of the final gate.

### 2026-11-09 — Task 2 (R4), implementation and portable regressions

Reproduced cross-session state loss: releasing B removed A's hold. Replaced
the single comparison slot with a session-keyed map, including both
constructors and failure cleanup. Expiry removes and republishes each
expired session, without allocation when no hold has expired.

`cargo test -p audiorouter-control --lib compare::tests`: 7 passed. New
coverage includes releasing B, a refused hold on B, refused deletion of A,
and independent expiry of two seeded holds. Full control suite: 366 passed,
11 explicitly ignored. Both Rust format/check commands and diff checks pass.
These use fake runtimes and seeded holds: successful dual native-worker
holds and audible restoration remain unverified and must be exercised before
R4 is marked done. Rollback: revert the Task 2 commit; no migration.

### 2026-11-09 — Task 3 (R1/R2), HTTP policy

SEC-13 / HTTP-06: replaced the production desktop-only list with a single
`http_access` classification. The exact nine desktop-only methods remain;
each now explains its own action. HTTP refuses UNC, device-shaped and
mapped/unavailable drive paths before forwarding plugin scan/list/retry,
inspection/parameters, session import and native bridge mapping paths.
The bridge's fixed local broker device identity is explicitly exempt;
backend `prepare_native_bridge` accepts only that exact identity. No method
became desktop-only, and local DLLs remain allowed.

The real-listener regression failed before the fix (UNC scan returned 200
and reached the forwarding closure), then passed with 403 and zero forwards
for network paths. Local paths still forward; OpenAPI keeps plugin scan and
omits the nine desktop-only methods. A discovery-schema guard reproduced
unclassified bridge path parameters and now covers them. A backend dispatch
test confirms a localhost UNC share reaches the scanner, whose missing-root
error is allowed, rather than being refused by an adapter policy.

Windows checks: HTTP tests 14 passed / 1 explicitly ignored; full shell
tests 95 passed / 4 ignored; focused backend UNC test passed. Documentation
acceptance: 248 Markdown files / 1443 links passed. SEC-13 and HTTP-06 updated;
no API schema or generated reference content changed. Mapped network drive
and actual NAS window testing remain unavailable; local/unused drive type
coverage is included. Built-shell UI verification remains a final gate.

### 2026-11-09 — Task 4 (R7), zero-frame media

ARCH-04 / SND-16/17: reproduced both zero-frame decode and callback `% 0`
panic with a one-frame 96 kHz WAV and a hand-built empty Loop sound.
`decode_audio` now refuses empty resampling output; Soundboard preparation
filters empty sounds and makes emptied slots unavailable. Both regressions
passed after failing before the fix. Full `cargo test -p audiorouter-engine`
passed on Windows, including source combinations 5/5, tool combinations 4/4
and the deterministic audio suite 33/33. Explicit private-fixture ignores
remain ignored; no hardware/audio device was opened. Both Rust format/check
commands and diff checks passed. Rollback: revert Task 4's commit.

### 2026-11-09 — Task 5 (R6), named connection ports

AUTO-18 / GRAPH-17: dispatcher regression first failed with unknown
`fromOutput`. Added optional `fromOutput` / `toInput` to both connection
methods, backend parameter validation, schemas and TypeScript contracts.
Port names resolve in the backend; unknown ports list available names and
invalid parameter types refuse without a save. Existing callers retain the
default selection/removal behavior. Explicit duplicate checks include source
and destination ports. Summary connections now include `fromOutput`.

Default MCP schemas/descriptions and `get_recipes` cover Output Switch A/B.
An actual `mcp_tool_call` regression with Editor authority wires both ports
to the same Mixer, then removes B while retaining A. The dispatcher test
wires separate outputs and checks the summary. No Properties control changed,
so the existing Output Switch guide recordings remain applicable.

Windows results: full control lib 368 passed / 11 ignored; CLI 50 lib + 5
stdio + 1 security test passed; contracts drift passed; UI 1001/1001 passed,
typecheck/lint/format checks passed. Rust formatting/checks and focused
control/CLI Clippy passed. Additional wrong-type port regression failed
before strict optional-text validation, then passed. API reference updated;
there is no API-reference generator in this checkout. API site catalog and
authoritative discovery snapshot regenerated with no content diff.

Catalog regeneration exposed its dependency on the removed production
desktop-only list; the generator now reads `http_access` instead. Full
guide/tool regeneration also rewrites unrelated existing pages/catalog
entries, so those outputs were restored and `--api-only` used (125 public
methods / 130 HTTP operations). No unrelated guide changes retained.
Rollback: revert Task 5's commit; optional fields require no migration.

### 2026-11-09 — Task 6 (R9), cleanup after durable commit

GRAPH-09 / UI-07: used a second SQLite connection and a DELETE trigger on
`graph_plans` to reproduce cleanup failure through `ControlPlane::dispatch`.
Before the fix the durable commit returned `storageFailure`; afterward it
returns success, labels the new version, appends `graph.committed` and
activates the fake running runtime. Same-key retry succeeds; a different key
cannot reuse the leftover plan because its base revision is stale. No test
seam or production fault API was needed. Cleanup is now best effort only
after the durable session/journal write; prior write failures still roll back.

Focused persistence tests: 18/18 passed on Windows; full control suite
369 passed / 11 explicitly ignored; focused control Clippy with all targets,
all features and denied warnings passed. Formatting, both format check
variants and diff checks passed. Native audio publication remains part of
the final runtime gate.
Rollback: revert Task 6's commit; no migration.

### 2026-11-09 — Task 7 (R8), preset identity and unreadable storage

UI-05: both dispatcher regressions failed before the fix: Unicode case
variants shared an ID but kept two entries, and saving over malformed JSON
silently replaced the stored presets. Identity/replacement now share one
Unicode lowercase fold. Listing unreadable presets remains empty; writes
refuse with a recovery message and preserve stored bytes. Delete also
refuses unreadable storage instead of treating corruption as an empty list.

Focused tests 6/6 and full control suite 371 passed / 11 explicitly ignored
on Windows. Both Rust formatting checks, diff checks and focused control
Clippy (all targets/features, warnings denied) passed. No preset
schema, ID algorithm or migration changed; saving an existing folded name
replaces all matching entries. Rollback: revert Task 7's commit.

### 2026-11-09 — Task 8 (R5), measured shell responsiveness and ordering

R5 reproduced in the built shell: plugin scanning ran for 1835 ms on the
main thread. With `#[tauri::command(async)]`, the same local folder scanned
for 1466 ms on a worker thread and the in-progress UI was readable before
completion. [Measurement evidence](evidence/2026-11-09-review-shell-rpc.md)
records the environment, exact thread output and limits. Temporary diagnostic
instrumentation was removed. Commands owning native windows/tray remain on
their existing thread; the file-picker command already dispatches blocking
work asynchronously.

Added an ordering regression that failed before release was chained after
hold. Compare requests now queue, retain their original session/backend on
release, and ignore stale request errors. Concurrent draft-save regression
failed with temporary-file rename collisions before adding a process-wide
mutex and unique create-new temporary files. Focused UI tests 37/37 and
draft tests 4/4 pass. The `sync_all` addition and expiry/cache changes are
kept in Task 9's separate commit.

Full combined validation before splitting commits: UI 1005/1005; shell
99 passed / 4 ignored after the HTTP tests; workspace and shell Clippy
(all targets/features, warnings denied) passed; lint, typecheck, formatting
and diff checks passed. Final built-shell live readings/compare checks
remain required. Vite reports its 500 kB chunk warning: main chunk was
606.27 kB at baseline and 607.25 kB with these changes. Retained because
these fixes add no eager module or asset and changing bundle boundaries is
independent performance work; this is recorded rather than hidden.

### 2026-11-09 — Task 9 (R10/R11/R12)

STATE-02: draft writes now call `File::sync_all` before closing the unique
temporary file and renaming it. Existing round-trip, refused-write and
concurrent-save tests pass (4/4). This verifies the write path and durability
call, not a physical power-cut experiment.

DEC-UI-21b / UI-17: fake-timer regression reproduced the comparison button
remaining pressed after 30 s. The acknowledged hold's `expiresInMs` now
schedules release, clearing pressed state and canvas comparison markers.
Manual release/unmount cancels the timer; sequence checks prevent an old
response or timer from clearing a newer hold. No DOM/layout or CSS changed.

UI-18: malformed-cache regression reproduced a startup `undefined.trim`
exception; another showed an injected version and JavaScript URL surviving
the cache. Cache reads now validate number/string/tag fields and construct
only canonical version and GitHub release URL values. Both regressions pass.
Focused UI tests 40/40; full combined UI suite 1005/1005 and shell 99 passed /
4 ignored were exercised with these changes during Task 8 verification.
Typecheck, lint, UI formatting and both Rust formatting checks pass.
Final three-theme/built-shell comparison verification remains a gate.
Rollback: revert Task 9's commit; invalid caches are ignored, no migration.

### 2026-11-09 — Task 10 (R13), silent HTTP clients

HTTP-06: four silent sockets delayed a real status request by 2.013 s before
the fix. The first read now has a 250 ms timeout; once any byte arrives the
existing 2 s read timeout/deadline behavior applies. Regression now meets
the one-second response bound. A second regression sends one byte, waits
350 ms, then completes a valid request, proving active slow clients retain
their normal allowance. All HTTP tests passed twice, including overflow and
separate header/body packet tests: 17 passed / 1 attended fixture ignored.
Full shell suite: 99 passed / 4 explicitly ignored. Both Clippy scopes and
formatting checks passed with the change. No flaky slow-client result was
observed, so the task was retained. Rollback: revert Task 10's commit.

## Next action

All planned fixes are implemented; final qualification is still required:
Task 2's dual native comparison publication evidence, final uninstrumented
built-shell user paths and three-theme comparison check, then a complete
matrix/diff audit. The owner's shell was closed before Task 8; only owned
isolated verification shells were launched and stopped. Never stop an owner
shell or launch a second shell. Workspace builds/full tests are authorized.
Task 3 commit `c4581bea`;
Task 4 commit `647a5ec2`; Task 2 commit `0be464b0`.
Task 5 commit `ad3f9db9`. Findings remain pending final qualification until
the completion audit verifies all required runtime and matrix evidence.
