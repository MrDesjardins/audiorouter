# Independent review of the overnight integration branch (2026-11-24)

Branch `review/overnight-integration`, from `integration/overnight-2026-11-24`
(94eb37ac9, base `origin/main` 0fb41911b). The diff is 148 files, +17,844
−1,071. I read the production (non-test) code of the diff myself and did not
rely on the other evidence files' claims. Tests were read where they changed an
old assertion, or where a claim depended on them.

Result: no defect that blocks the branch. One real defect found and fixed
(PLAN-1 durable eviction), nothing that changes the audio callback, privacy
mute or authorization. Several contract changes and risks need an owner's eye;
they are listed under "Recorded, not changed".

## Fixed in this review

### F1. Durable plan inventory evicted live plans and kept stale ones (PLAN-1)

`crates/storage/src/lib.rs` `save_graph_plan` dropped the oldest rows when the
cap of 100 was reached. The in-memory store (`GraphStore::make_room_for_plan`,
`crates/domain/src/lib.rs`) drops plans whose session moved past their base
revision first and only then the oldest. The two inventories therefore kept
different plans.

Trace (control plane with storage, through `graph.plan` / `graph.commit`):
50 plans of session B, 50 newer plans of session A, one A commit (49 A plans
now stale), then 51 more B plans. Result:

| plans | memory | storage |
| --- | --- | --- |
| 49 old live B plans | kept | **evicted** |
| 49 stale A plans | dropped | **kept** |

Consequence: after a restart the live plans are gone (`planNotFound` on
commit) while the stale plans still occupy the durable cap. Low severity (plans
live five minutes) but it is exactly the case PLAN-1 set out to fix.

Fix: storage drops stale plans first (a plan whose session row has a different
revision), then the oldest. Regression test
`tests::a_full_durable_inventory_drops_stale_plans_before_live_ones`
(storage lib): failed before the fix with "live plan 0 was evicted while stale
plans remained", passes after. `cargo test -p audiorouter-storage --lib`: 114
passed.

## Added tests

- `crates/transport/tests/probe_production_server.rs`: 240 `probe_pipe` calls
  (connect and leave, no request) against the production control server loop,
  then one request per pipe instance. The existing probe tests only used
  `serve_once_with_client`. The production I/O thread ends after 50
  consecutive failed connections, so this checks a probe is not fatal.
  Passes. (It cannot tell that every instance is alive, only that the server
  still answers, because four concurrent clients can be served by one
  instance in turn. 240 probes exceed 4 × 50, so a fatal probe would have
  killed every thread.)

## Recorded, not changed (owner decision or accepted risk)

Severity is my judgment of user impact. "Pre-existing" means the old code
behaved the same.

1. **Medium, contract change: busy pipe now waits for the whole request
   timeout.** `crates/transport/src/lib.rs` `round_trip_with_timeout` passes
   its timeout to `connect_client_waiting` (was a fixed 2 s). The shell uses 60 s
   for everything except `status.get`/`events.subscribe` (2 s) and
   `system.quit` (15 s) (`src-tauri/src/main.rs:1466`). A wedged backend now
   makes a window, HTTP or MCP request wait up to 60 s for a pipe instance
   instead of failing after 2 s. Tauri's `rpc_request` is `command(async)`, so
   the window thread is not blocked. Intended by the change; the owner should
   know the failure latency changed. The new OS-transition retry
   (`forward_os_transition_with_retry`) is unaffected in the common case, and a
   first attempt that waits longer is more likely to deliver, not less.
2. **Medium, contract change: PLAN-1 eviction crosses clients.** The cap of 100
   is shared by every session and client. A client that plans 100 times evicts
   another client's oldest live plan, whose commit then gets `planNotFound`
   instead of `revisionConflict`. By code reading and by
   `a_full_plan_inventory_drops_the_oldest_plan_instead_of_refusing_the_newest`
   (the oldest plan is gone, the newest commits); I did not run a separate
   two-client trace. The clients must replan on `planNotFound`; the UI does
   (graph save re-plans), assistants rely on the error text. Before the change
   the same client got `PlanLimitReached` and every save failed for the plan
   lifetime, so this is a trade, not a regression. A per-client cap would be
   the alternative.
3. **Low-medium, pre-existing, now visible: a recording that stops early shows
   as a green success.** `ui/src/actionMessage.ts` `recordingStoppedEarly`
   (called from `ui/src/App.tsx:811`) keeps the success tone, 8 s lifetime. I
   checked the old rules against the backend's reason text
   ("Recording stopped saving after 0:12 because the file could not be
   written. The file keeps everything up to that point."): the old classifier
   also gave `success` (it matches "stopped", none of the error patterns). So
   the integration preserved the behavior, and `actionMessageLegacyParity.test.ts`
   asserts it. A recording that lost audio should be an error or warning that
   stays on screen. Owner decision; the code comment already asks.
4. **Low, contract: scalar parameter type and length checks.**
   `crates/control/src/authorization.rs` `validate_scalar_param_types` now
   refuses a wrong JSON type for any scalar the schema declares, and a string
   over the published `maxLength`. Callers I checked comply: the UI is typed by
   the generated contracts, the Stream Deck plugin sends `slot` as an integer
   (`Number(...)`) and string IDs. Behavior to know: a float such as `5.0`
   for an `integer` is refused (serde reports it as f64); `null` still means
   omitted. `maxLength` counts characters while the storage limits count bytes,
   so `idempotencyKey` of 100 two-byte characters passes the schema and is then
   refused by the handler ("invalid idempotency key" / "The idempotency key is
   too long."). Same refusal as before the change, only the published schema is
   looser than the real limit. Traced through `dispatch` with and without a
   client identity.
5. **Low, contract: `simple_commit_key` changes the journaled key.** A named
   client's task-method commit key was `"{method}:{key}"` for everyone (two
   clients using one key collided); it is now scoped per client under
   `"{method}:commit"`. A retry of an in-flight task call sent across an
   upgrade, with the same key, would commit a second time instead of replaying.
   The window is the length of one request; accepted.
6. **Low: `operations.get` / `operations.cancel` cost.** For an unknown ID the
   handler derives and reads about 650 journal keys (2 × 325 methods).
   Measured on a file database, 20 not-found calls as an Observer: 1.04 ms each
   in release, 2.2 ms in debug. It runs on the control thread that also
   services audio every 1 ms (AGENTS lesson: keep request handlers short) and
   `operations.get` is not throttled as a mutation. One lookup can delay one
   audio pass; a client looping on it can delay many. Cross-client isolation
   holds: every key embeds the active client (checked by
   `operations_get_finds_a_journaled_operation_of_every_api_method`, which also
   asserts client-b cannot read it). Cheaper alternative: look the ID up by a
   prefix index on the journal instead of probing each method name.
7. **Low: durable and in-memory plan eviction can still differ after a restart**
   when the session row in storage lags memory; fixed for the normal case in F1.
8. **Low, not run: `probe_pipe` treats any probe error as "no app running".**
   `pipe_is_owned_by_a_server` returns false for an error such as a server
   owned by another Windows user. Intended (not this user's app); busy is now
   correctly "owned". A failed `GetNamedPipeServerProcessId` on a live pipe
   would also read as not running. Unlikely.
9. **Info: the planar compressor path has no envelope clamp.** The interleaved
   path clamps to ±120 dB; the planar path repairs non-finite input first and
   so stays finite. No change needed.
10. **Info: `decode_audio` `catch_unwind`.** Safe: no `panic = "abort"` in any
    profile; the only panic hook (`install_panic_log`, shell) logs a
    `shell.panic` record and then calls the default hook; it does not latch safe
    mode. Release builds have overflow checks off, so the WAV `fmt ` multiply
    wraps there instead of panicking; the catch matters for debug and test
    builds. Allocation failure and stack overflow are not caught.

## Reviewed with no defect found

- **Audio path (priority 1).** `crates/dsp/src/timeshift.rs` pause fade: the
  fade reads the audio that was playing, ends at exactly zero on the last frame
  (`(frames - 1 - offset) / frames`), `frames - 1 - offset` cannot underflow
  (`offset < frames`), `pause_fade` is set only for the block where pause
  begins and cleared by `reset`. No allocation, lock or panic added. Gate and
  compressor: non-finite input no longer reaches the detector (interleaved
  paths; planar paths already repaired it); compressor envelope clamped.
  Windowed meter and `RmsWindow`: the non-zero counter cannot underflow (only
  decremented for a slot that was counted), and the sum is forced to exactly zero
  and floored at zero before `sqrt`. Stereo biquad ramp is counted per frame.
  No hot-path allocation introduced.
- **Device-loss recovery (priority 2).** `arm_device_loss_recovery` now
  requires the route's runtime state to be `Running`. Its only caller is the
  invalidation of a running worker, so a playing route still arms; a stopped
  route whose worker stop errored no longer restarts. `RuntimeState` has no
  paused state that this could miss. Privacy mute: only the derived key
  (`toggle:{key}`) changed; keys that fit keep the old text, longer ones hash
  deterministically.
- **Idempotency scoping (priority 3).** `scope_idempotency_key` keeps the
  readable `client\0method\0key` form when it fits (earlier records are still
  found) and otherwise uses `scoped:<sha256>`; a hashed key cannot equal a
  readable one (the readable form contains NUL). `operation_lookup_keys`
  derives candidates the way they are stored.
- **SES-4 (priority 6).** `sessions.delete` finalizes recorders after every
  refusal and before `storage.delete_session`; the finalize result is
  ignored on purpose. A stop error removes the worker, marks the recorder
  failed and saves a recoverable checkpoint, so the file is kept; a storage
  error leaves the session intact with the recording already saved.
- **Worker frame reader (priority 7).** `read_worker_message_into`: the buffer
  grows with received bytes, never reserves beyond the declared length,
  `payload.len() == declared` when the loop ends, `Interrupted` is retried, a
  short stream gives the old "failed to fill whole buffer" error. The scratch
  buffer is released past 1 MiB; the largest legitimate process frame is
  asserted to fit.
- **`probe_pipe` and `connect_client_waiting` (priority 5).** SAFETY comments
  are accurate (live handle, aligned local). The missing-pipe wait is unchanged.
- **Workflow (priority 10).** `.github/workflows/quality.yml` `fuzz` job:
  `fuzz_scale` reaches the script only through `env`, is validated to 1–50, no
  `${{ }}` inside the script, actions pinned to the same SHAs as the other jobs,
  `permissions: contents: read`, nightly schedule unchanged.
- **UI (priority 9).** `--field-border`, `--focus-ring`, `--danger-text` and
  `--accent-text` exist in the dark, light and high-contrast blocks. No new
  hooks, `eslint-disable`, `as unknown` or `dangerouslySetInnerHTML`. The
  `useWorkspaceEvents` `useEffectEvent` callbacks are unchanged in shape. i18n:
  every placeholder in `fr.json` and `zh-Hans.json` matches `en.json` (checked
  with a script); the only key differences are plural categories (`_many` in
  French, `_one` missing in Chinese), which are correct. Tones are declared per
  key in `actionMessageTones.ts`; `actionMessageLegacyParity.test.ts` embeds a
  copy of the old regex rules and compares every key against them, so it is
  not derived from the table.

## Old assertions that changed

| File | Change | Verdict |
| --- | --- | --- |
| `crates/domain/src/lib.rs` plan cap test | `PlanLimitReached` for the 101st plan → oldest dropped, newest kept | Justified: the intended PLAN-1 behavior change; the test now also asserts which plan survives |
| `crates/storage/src/lib.rs` plan inventory test | `InvalidPlan("… exceeds 100 items")` → eviction, plus replace and rejected-plan cases | Justified, stronger |
| `crates/control/src/persistence_tests.rs` plan rollback | filled the table by hand and expected `InvalidRequest`; now injects a SQLite trigger and accepts any non-`Store` error, plan id `plan-1` | Justified by the new eviction (a full table no longer fails). Weaker on the variant: it accepts any non-`Store` error. The rollback assertion (plan not found after failure) is unchanged |
| `crates/control/src/graph_tests.rs` stopped-session guard | one message for four bad values → per-value messages (`must be a string` for a number or bool, old text for null and "") | Justified: the dispatcher type check fires first for wrong types; the handler still refuses null and "" |
| `crates/control/src/setup_check_tests.rs` oversized ID | message contains "ID limit" → exact `sessionId must be at most 128 characters` | Justified, stricter (exact) |
| `crates/engine/tests/deterministic_audio.rs` Time Shift pause | whole segment zero → last frame of the pause block zero, later blocks zero | Justified by DSP-6; the fade shape is asserted in `pause_fade_fades_out…` and the engine lib test |
| `crates/cli/tests/security_boundaries.rs` | `assert!(status.success())` now prints stderr; stderr captured to files | Justified, diagnostic only |
| `ui/src/App.accessibility.test.tsx` focus trap | focus start moved from Cancel to Add connection; wrap target `cancel` → `add` | Justified: the dialog buttons were reordered (decline first); same wrap behavior is asserted from the new last button |
| wording tests (`draft.test.ts`, `draft-connection.test.ts`, `LiveAutosaveSetting`, `NetworkNodeEditor`, `device-binding.pw.ts`, `plainWords` neighbors) | "node"/"draft" → "tool"/"unsaved edits" | Justified: plain-word renames; the connection test gained an `InputOccupiedError` class check in French |
| `actionMessage.test.ts`, `actionText.test.ts`, `backend.test.ts`, `VirtualCablePanel.test.tsx`, `QuitButton.test.tsx`, `useWorkspaceEvents.test.tsx`, `audioUpload.test.ts`, `TestSignalPlaybackControls.test.tsx` | wording classification → structured `UiMessage` tone, lifetime and text checks | Justified: the mechanism moved from regex to declared tones; parity is checked in `actionMessageLegacyParity.test.ts` |
| `ui/e2e/ui-audits.ts` | audits gained a `scope` argument; the `/auto|scroll/` skip line is only re-indented | Justified, no audit weakened |

## Checks run

In this worktree, `CARGO_BUILD_JOBS=6`, own `target`, no shell exe, no default
pipe, no owner database.

- `cargo test -p audiorouter-dsp -p audiorouter-domain -p audiorouter-storage --lib`: pass (88, 84, 114 after my test).
- `cargo test -p audiorouter-control --lib`: 509 passed, 29 ignored (before and after the storage fix).
- `cargo test -p audiorouter-transport`: pass, including `busy_pipe_wait` (3) and the new probe test.
- `cargo test -p audiorouter-engine --lib` (189) and `-p audiorouter-plugin-host --lib` (130): pass.
- `cargo fmt --all -- --check`: clean. `cargo clippy -p audiorouter-storage -p audiorouter-transport --all-targets --all-features -- -D warnings`: clean.
- UI: `typecheck:all`, `lint`, `format:check`, `i18n:check` clean; `npm test` 136 files, 1190 tests pass.
- Playwright (Edge, `e2e_backend` rebuilt first): `contrast-states`, `button-states`, `text-field-feedback`, `ui-language`, `dialog-audits-more`, `device-binding`, `device-used-by`: 78 passed, covering dark, light and high contrast.

## Checks not run

- Clippy for the other crates and for `src-tauri`; `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` (no `src-tauri` file edited by me).
- `cargo test` for `src-tauri`, `audiorouter-cli`, and the integration test binaries `fresh_install_shell`, `live_audio_continuity`, `service_timing_pipe` (need the shell exe or audio hardware or are timing runs).
- The scaled fuzz job and the `bounded_fuzz` integration tests at scale.
- `npm --prefix contracts run check:drift`, docs validation, and the remaining Playwright specs.
- Any run of the built exe; no Windows device or driver evidence was produced.
- A real two-client PLAN-1 trace (see item 2).

## Files not reviewed in depth

Test-only and generated material, read only where an old assertion changed:
`crates/control/src/{contract_cases,contract_tests,bounded_fuzz_tests,fault_interleaving_tests,service_timing_tests}.rs`,
`crates/cli/src/contract_tests.rs`, `src-tauri/src/http_contract_tests.rs`,
`crates/{engine,plugin-host,protocol,recording,storage}/tests/*fuzz*.rs`,
`tests/support/fuzz.rs`, `crates/transport/tests/service_timing_pipe.rs`,
`crates/control/src/plugins_tests.rs`, `recording_tests.rs`, the Windows
`plugin_editor_windows.rs` tests, and the new `ui/e2e` specs and
`ui/e2e/ui-audits-color.ts` beyond the theme run above.
Documentation and evidence under `docs/plans/**` (including the CSV files),
`docs/operations/sdk-setup.md`, `tests/acceptance/find-cmake*.ps1`,
`m06-vst3-sdk.ps1`, `tools/vst-qualification/run.ps1`,
`tools/quality/policy.test.mjs`, and the generated i18n files beyond the
placeholder check. The UI string-to-`say(...)` conversion across `App.tsx` and
about thirty panels was reviewed by its diff of non-message lines and by
typecheck, lint and tests, not line by line.
