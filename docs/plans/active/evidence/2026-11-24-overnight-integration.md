# Overnight integration of the 2026-11-24 branches

Branch `integration/overnight-2026-11-24`, created from `origin/main` 0fb41911b
(release 0.0.32). Unattended, local only: not pushed, no PR. The merges and
checks below ran in a separate worktree with `CARGO_BUILD_JOBS=6`; no
`audiorouter-shell.exe` was launched, the default control pipe and the owner's
database were not touched.

## Merges (`git merge --no-ff --no-edit`, in this order)

| # | Branch | Branch tip | Merge commit | Result |
|---|--------|-----------|--------------|--------|
| 1 | `docs/plan-housekeeping-2026-11-24` | 5112e9554 | bf6feb828 | clean |
| 2 | `fix/audit-low-dsp-recording` | ef2c0d5c5 | 5772d7b6a | clean |
| 3 | `fix/resilience-res11-ops-plans` | 31e815842 | b7359bdd7 | clean |
| 4 | `fix/audit-low-remaining` | a6cbc7784 | c5928a51c | clean |
| 5 | `fix/vst-portable-hardening` | 60e02f9ed | 917b438f2 | clean |
| 6 | `feat/l10n-extraction-2026-11-24` | 7f4f93bd5 | ed92bd95f | conflict in `ui/src/App.tsx`, resolved |
| 7 | `fix/cli-probe-busy-pipe` | 19529caa4 | 983b2d890 | clean |
| 8 | `test/ui-quality-audits` | b4dba8f89 | cd0f5a8d0 | conflicts in `ui/src/App.tsx` and `ui/src/SignalFlowPanel.tsx` (not announced in the brief), resolved |
| 9 | `test/q4-contract-tests` | 2951e9b71 | a151ce46b | conflict in `crates/control/src/persistence.rs`, resolved with a semantic fix |
| 10 | `fix/worker-frame-allocation` | 6182e00c0 | bda5e23c6 | clean |
| 11 | `test/q4-fault-interleavings` | cd9ae4bf6 | 423138a62 | clean |
| 12 | `verify/q4-contract-tests` | 7395ad23f | 0e886255f | clean (see below) |

`verify/q4-contract-tests` became final at 7395ad23f (one UI test and a docs entry on
top of `test/q4-contract-tests` 2951e9b71) while step 9 was already merged, so it was
merged afterwards as step 12. It is a descendant of 2951e9b71, so nothing was
merged twice. The Rust checks below ran before this merge; the merge touches only
`docs/plans/active/application-quality.md` and
`ui/src/App.accessibility.test.tsx`, and every UI check, `docs/validate` and
`cargo fmt` were rerun after it (the Playwright suite ran after it too).

## Conflict resolutions

### Step 6: `ui/src/App.tsx` (L10N against UI-1)

Both sides edited the `./draft` and `./actionMessage` import block. L10N replaced
message calls with `say()` / structured messages and added `InputOccupiedError`;
UI-1 (`fix/audit-low-remaining`) replaced the hand-kept source-kind lists with
`rendersOwnAudio` and `supportsApplicationMixerRoute` from `draft.ts`. Resolution:
the L10N `actionMessage` import list is kept whole, and the single `./draft` import
is `InputOccupiedError, rendersOwnAudio, supportsApplicationMixerRoute,
unfedRouteNodes`. The two non-conflicting UI-1 hunks (the `unsupported` filter and
the `bindings` generated-kind test) merged automatically and use the helpers; the
`InputOccupiedError` use is L10N's. Typecheck, lint, the unit suite and the whole
Playwright suite pass on the result.

### Step 8: `ui/src/App.tsx` and `ui/src/SignalFlowPanel.tsx` (UI audits against L10N)

Not listed in the brief. The audit branch makes the application picker report why a
typed path was refused (`addApplicationCapturePathFromPicker` returns the message,
`aria-invalid`, `aria-describedby="application-picker-status"`) and moves Cancel
before Add connection; L10N had turned all those captions into `tr(...)` calls.
Resolution keeps both intents:

- `App.tsx`: the audit's `const message = ...; return message;` shape, with
  `setActionMessage(errorMessage(message))` from L10N.
- `SignalFlowPanel.tsx`: Cancel first and translated (`tr("common.cancel")`, the
  automatic merge had left an untranslated "Cancel" above and L10N's translated one
  below, so the trailing one was removed), the audit's `setPathRefused(true)` kept
  with L10N's `tr("signalFlowDialogs.pathFirst")`, and the status paragraph with the
  audit's `id` and L10N's `tr("signalFlowDialogs.pickerHint")`.

`i18n:check` passes (3738 messages), so no new English string was left outside the
catalogs.

### Step 9: `crates/control/src/persistence.rs` (STO-6 against Q4.1), the semantic fix

- STO-6 (`fix/resilience-res11-ops-plans`) made `operation_lookup_keys` build
  candidates from `API_METHODS` (`client\0method\0id`) and **skip** any candidate
  longer than `MAX_IDEMPOTENCY_KEY_BYTES`, because storage errors on an over-long
  lookup key.
- Q4.1 (`test/q4-contract-tests`) made keys that would pass that limit be **hashed
  when stored** (`scope_idempotency_key`, a `scoped:<sha256>` form) and added a
  `simple_commit_key` with the `method:commit` scope for task-shaped edits.
- Taken literally, the merge would keep skipping the long candidates, so an
  operation stored under a hashed key (a caller using the longest key the schemas
  allow) would answer "operation not found" from `operations.get`, and
  `operations.cancel` would refuse it.

Merged behaviour (`operation_lookup_keys`): every candidate is derived exactly as
it is stored, `scope_idempotency_key(client, spec.name, operation_id)` for each
`API_METHODS` entry, so a candidate that passes the limit becomes the same digest
that was stored and is never skipped. A second pass adds
`scope_idempotency_key(client, "{name}:commit", operation_id)` for the new
`simple_commit_key` scope, after all plain-method keys so the record reports the
method the caller ran. No candidate exceeds the limit, so storage never errors on a
lookup. Keys that still fit stay readable, so earlier records are still found.
`operations.get` and `operations.cancel` share the function, so they behave
consistently.

Tests (`crates/control/src/persistence_tests.rs`, through
`dispatch_authorized_for_client`, the same entry point the pipe uses):

- New `operations_get_and_cancel_find_an_operation_stored_under_a_hashed_key`:
  `toolPresets.save` with a 128-byte `idempotencyKey` as `client-a` (the scoped key
  is over the limit, so it is stored hashed), then `operations.get` returns
  `completed`, `operation: toolPresets.save`, `durable: true`; `client-b` still
  cannot read it; `operations.cancel` answers `alreadyCompleted`; a never-run
  128-byte ID is `operation not found` (-32602). It was confirmed to fail without
  the fix: temporarily restoring the skip filter gave `operations.get said
  Some(JsonRpcError { code: -32602, message: "operation not found" ...})`; the fix
  was then restored.
- STO-6's `operations_get_skips_candidate_keys_too_long_for_the_journal` was renamed
  `operations_get_finds_an_exact_limit_key_and_hashes_longer_candidates` (an
  exact-limit stored key is still found, and a never-stored long ID is "not found",
  not an error). Its other STO-6 tests, including the walk over all 156
  `API_METHODS`, pass unchanged.
- The Q4.1 contract tests (dispatcher, MCP, HTTP) run in the full suite below.

The STO-6 evidence file (`2026-11-24-resilience-ops-plans.md`) was left as the
branch wrote it; this entry supersedes its "skip" wording once both are merged.

## Checks on the final integration HEAD

Run on the tree before the evidence commit (the evidence commit adds one Markdown
file). Unless stated, the Rust and UI checks ran on 423138a62 and the UI checks and
Playwright were rerun on 0e886255f.

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | clean |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| `npm run build` in `ui/` (typecheck + vite build), then `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace` | pass: 1641 passed, 0 failed, 75 ignored (63 test binaries) |
| `cargo test --manifest-path src-tauri/Cargo.toml` | pass: 142 passed, 0 failed, 4 ignored |
| `cargo test -p audiorouter-engine --test tool_combinations --test source_combinations` | pass: 6 + 6 |
| `cargo build -p audiorouter-control --example e2e_backend` | built |
| `npm run i18n:check` | pass: 3 language files, 3738 messages |
| `npm run typecheck:all` | pass |
| `npm run lint` (`--max-warnings 0`) | pass |
| `npm run format:check` | pass |
| `npm test` (vitest) | pass: 136 files, 1190 tests (1189 before the `verify/q4-contract-tests` merge) |
| Playwright, `playwright.config.ts --workers=1`, stock config, port 4186 | pass: 862 passed, 16 skipped, 0 failed, 0 flaky, 36.5 min |
| `npm --prefix contracts ci` and `check:drift` | pass: 156 methods, 38 node kinds, 20 processors, 24 event categories |
| `node --test tools/quality/policy.test.mjs` | pass: 68 of 68 |
| `node tools/docs/validate.mjs` | pass: 299 Markdown files, 1660 local links |

The 16 Playwright skips are the suite's own gates: the Chinese UI audit collector,
the HTTP/Swagger test, ten inspector-stability cases for kinds that guard
themselves, the README screenshot generator and three Gate guide spikes. None was
skipped by this integration.

## Not run

- The packaged shell, `fresh_install_shell`, the built exe and any live-audio
  qualification (the brief forbids launching `audiorouter-shell.exe`). Nothing here
  is evidence for the built app, the embedded UI or real devices.
- `e2e:production-canvas` (the second Playwright config) and `e2e:ci`; only the
  development suite was run.
- Firefox guide-audio sweep, VM/driver runs, hardware and Windows-specific gates
  from the branch plans; the 75 + 4 ignored Rust tests are `#[ignore]` (hardware or
  environment gated) and were not run.
- PR annotations (no PR exists).
- The plan files were merged as the branches wrote them; `docs/plans/active/current.md`
  was not edited.
