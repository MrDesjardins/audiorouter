# Setup check (UI-27) — active plan

Owner request 2026-11-02: "add a feature that look for these kinds of
misconfiguration inside AudioRouter to help user to have a good
configuration." Branch `feature/setup-check` from `origin/main`
(`dab85acf`). No pull request without the owner's approval.

## Objective

Tell the user, before and while they play, when a saved route and the
Windows devices it binds do not fit together, with a plain fix. Trigger
incidents from 2026-11-02: a stereo-only output refused on an 8-channel
CABLE-B; advice to set that cable to stereo that would have broken the
owner's Surround-to-headphones input; and a feedback loop across two routes
(test route → CABLE-B → owner's `siege-in` → CABLE Input → test route).

## Requirements

- UI-27 (new, [spec](../../spec/09-interface.md)), AUTO-18 (REST and MCP
  current), UI-17 (no layout shift), UI-19 (Status tab).

## Decisions (owner, 2026-11-02)

- Checks: Play-refused device formats, feedback loops, missing or moved
  devices, rate and cable tips.
- New branch from main; build now.
- Judge a device by its use in sessions (lesson of the CABLE-B advice).
- Reuse the Play predicates; never copy them.
- The 16-channel twin is not repeated: the Apps tab (UI-26) already checks
  Discord's choice, and AudioRouter cannot see other apps' choices.

## Tasks

1. Extract shared predicates: `output_endpoint_supported` (fan-out rule in
   `native_bindings.rs`), `spatial_capture_supported` (Surround rule in
   `native_paths.rs`), and the feedback traversal of
   `reject_endpoint_feedback` into a reachable-outputs helper.
2. `setup_check.rs` in `audiorouter-control`: a pure function over sessions,
   running state, device facts and cable return pairs; findings bounded to
   100. Endpoint-binding kinds come from an exhaustive `match` on `NodeKind`.
3. `system.checkSetup` (Read, read-only): `API_METHODS`, input/output
   schemas, authorization parameters, dispatch, `contracts` types,
   `check:drift`.
4. MCP default tool `check_setup` (read-only annotation), recipe entry,
   regenerate `docs/operations/api-reference.md` and site catalogs.
5. UI: Setup check list in the Status tab with Check again, reserved slot,
   keep last result; three-theme screenshots.
6. Quickstart section; release notes line.

## Validation

| Check | Evidence |
| --- | --- |
| One regression per finding, plus "7.1 loopback with Surround is fine" | `setup_check` unit tests |
| Check flags exactly what prepare refuses (format predicates) | shared-predicate tests |
| Cross-session loop reproduces the 2026-11-02 topology | unit test |
| `ControlPlane::dispatch` and `mcp_tool_call` | control and CLI tests |
| Contracts drift, OpenAPI, MCP reachability test | `check:drift`, CLI tests |
| UI unit, lint, format, three themes | vitest, ESLint, Playwright screenshots |
| fmt and Clippy for both manifests | commands |
| Built app run against the owner's devices | only with the owner's approval (one shell at a time) |

## Risks

- Endpoint IDs of a loopback input equal the render ID; direction comes
  from the device list, not the ID text.
- A finding that is wrong erodes trust: only Play refusals are `problem`.

## Rollback

The method and UI are additive; revert the branch commits. No stored data
changes.

## Results (2026-11-02)

Tasks 1–6 are implemented. Checks run on the owner's PC (Windows 11, pinned
Rust 1.96.0, Node 22.22.3):

- `cargo test -p audiorouter-control -p audiorouter-domain`: passed (326
  control lib tests, 14 of them `setup_check`, including the 7.1 Surround
  false-positive guard and the 2026-11-02 cross-session loop).
- `cargo test -p audiorouter-cli`: passed (44 lib, 5 `mcp_stdio`), including
  `check_setup_works_through_mcp_for_an_observer` and the reachability test.
- `cargo test --manifest-path src-tauri/Cargo.toml`: passed (85).
- `cargo fmt` checks and both Clippy runs with `-D warnings`: clean.
- `npm --prefix contracts run check:drift` (138 methods) and typecheck: passed.
- UI format, lint (0 warnings), typecheck, `npm --prefix ui test` (87 files,
  788 tests) and `e2e/setup-check.pw.ts` plus `workbench-organization.pw.ts`
  in dark, light and high contrast: passed; screenshots inspected. The device
  heading below the panel stays at the same position with three findings and
  with none (UI-17).
- Real data: `api call system.checkSetup` on a copy of the owner's database
  (`state.sqlite` with its `-wal`) against the live devices checked 10
  sessions and 15 devices. It reported one true problem, twice (two sessions
  share the input): CABLE-B Input at 2 channels under the Surround input
  `siege-in`, the state left by the advice corrected earlier that day. No
  other finding; no false positive on the 7.1 setup.

- Cost: reading the 15 devices took 167 ms on the first call in a process
  and 9–11 ms after (release build, also on fresh threads), the same as the
  `devices.list` enumeration. The control thread also services running
  audio, so the read runs through `while_servicing_audio`; checking the
  sessions themselves is pure and fast.

Not run: the built desktop app (the owner's release shell is running; one
shell at a time) and the full Playwright suite. `tools/site/check-tool-guide-
catalog.mjs` fails on a clean `origin/main` as well (stale
`crates/control/src/catalog.rs` hash, not touched here); it is left for a
separate change.

## Next action

### Sol review and local build (2026-11-02, owner request)

Owner closed AudioRouter and requested a review of the two setup-check commits,
fixes, an explanation of trigger timing and a build to test. No PR is authorized.
UI-27/UI-17/AUTO-18 review findings to address: stale requests after disconnect,
misleading success when devices were not checked, refresh after Play/Stop or
device-format changes, bypassed-device false positives, and multi-session loops
beyond a pair. Keep pure analysis off the audio service thread as well as device
enumeration. Add focused regressions before fixes, check real dispatch and MCP,
typecheck/compile immediately, then full relevant suites, both Clippy/format
scopes, browser themes and production build. Regenerate the public method
catalog. Build UI before the release shell with `--features custom-protocol`,
stage the matching CLI, verify
timestamps/hashes, then exercise Status in the built app without starting audio.
Preserve the owner's saved data and Windows device configuration. Rollback:
revert the review follow-up; no storage migration. Remaining live-audio
qualification is separate from this read-only Status check.

Next: implement and verify these fixes, then leave the local build for owner
testing. Never call the feature release-qualified from mock/browser tests.

Review fixes implemented:

- Pending UI requests are invalidated on disconnect, backend replacement,
  refresh-key changes and unmount. An initial failed request no longer says
  Checking forever. Empty/unavailable device inventory cannot claim all devices
  fit; clean wording states the limits of the checks actually performed.
- The Status refresh key includes Play/Stop's active session IDs and the full
  exposed device format, not just channels/rate. Equivalent snapshots do not
  start more checks. The panel and quickstart explicitly say saved routes only,
  while Status is open; unsaved drafts and Play's own validation are separate.
- Bypassed device nodes do not generate missing/format findings. Cross-session
  cycle traversal now follows individual signal paths across three or more
  sessions, avoiding false joins of unrelated branches. Problems precede tips
  before truncation. The session filter enforces its declared ID length bound.
- File-backed saved-session reads now use a read-only helper connection without
  creation/migration/pruning/integrity scanning, while the original connection
  remains available to service audio. Device enumeration and pure analysis also
  run through `while_servicing_audio`. Regression verifies no file creation and
  no migration of a diagnostic input.
- Opus added the method to the API but did not regenerate the public website
  method catalog. It now includes `system.checkSetup`. Guide generation also
  needs `generate-social.mjs`; rerunning it preserves existing social metadata.

Scope/evidence limits: the independent tool Properties catalog check still
fails at the unchanged base source hash. An exploratory regenerated snapshot
then exposed existing selector-vs-wire-type/coverage mismatches; it was reverted
instead of falsely marking unrelated tool recordings complete. No NodeKind,
tool setting or Properties control changes in this task, so no tool Properties
video recapture applies. `site/check-guides.mjs` passes. This is an explicitly
open documentation-tooling gate, not a passing check. The full Playwright suite
was not run; the generic HTTP browser scenario requires its disposable fixture
and was skipped. Actual HTTP dispatch is covered by the Windows shell suite.

Build experiment: plain `cargo build --release` launched a localhost development
URL, which the real desktop smoke caught. Rebuilt with
`cargo build --manifest-path src-tauri/Cargo.toml --release --features custom-protocol --locked`
after the final UI build; only that embedded-UI executable is the test artifact.
The matching release CLI is staged beside it with verified identical hashes.

Final verification (Windows 11, Rust 1.96.0, Node 22.22.3):

- Four new regressions reproduced failures before fixes (disconnect, unavailable
  inventory, bypassed devices, three-route cycle). Final focused setup tests:
  17 passed, including severity ordering and the ID bound. Full control library:
  329 passed / 11 opt-in ignored; storage: 100 passed. CLI/domain suites passed,
  including default MCP `check_setup` and reachability. Shell: 85 passed /
  4 opt-in ignored. Both workspace/shell Clippy scopes with `-D warnings` and
  both format checks pass.
- Final UI: 792 tests / 87 files pass; format/lint/typecheck pass. Contracts
  drift: 138 methods / 37 kinds / 20 processors / 22 event categories pass.
- Edge: 11 setup/workbench scenarios pass (one HTTP browser scenario skipped
  for its explicit fixture requirement); final three-theme setup suite adds
  unavailable-inventory state and stable geometry, 3 passed. Dark/light/high
  contrast screenshots inspected under `%TEMP%/audiorouter-playwright-results/`.
- Built native app exercised with a new disposable database/profile: Status
  opens the setup check, Check again returns successfully against 15 Windows
  devices, final explanatory text is embedded, and Audio stopped remains shown.
  No microphone, speaker or cable stream was opened. Backend/shell logs contain
  no error outcomes. Profile, UI text and screenshots retained at
  `target/setup-check-smoke-embedded-2b1bf160bc1047ce9db92b4c48d50c20`.
- Test artifact: `src-tauri/target/release/audiorouter-shell.exe`, SHA-256
  `329290DC3E1903A931BDBFA09876FA32A6124C76FFDA4C68DB43EE8B1EA48EC5`.
  Built 18:36:39 local, after `ui/dist` at 18:34:12. Matching CLI source/staged
  hash: `2C596CBCDFDA004CE145D98BE2C4C20A3F31DD83661A1826B346DA9BF8770B8E`.
- Build warning retained: the 633.03 kB main JS chunk exceeds Vite's 500 kB
  advisory. No threshold was raised. Splitting the editor is independent of
  setup correctness and remains a performance/maintainability follow-up.
  Browser color-environment and React Flow attribution warnings were inspected;
  this change does not establish a license entitlement or alter attribution.
- Guide link/asset check and documentation acceptance pass. The independent
  Properties catalog gate above and full browser suite remain open; neither is
  represented as passing. No installer, release or PR was created.

Next action: owner launches the embedded test executable and opens Status →
Setup check with their real saved sessions. Their Windows settings and saved
database were not modified by this smoke. Review does not qualify live-audio
continuity or the separate release gates.

### Status attention indicator and stopped audio wording (2026-11-02)

Owner requests a red Status indicator for problems and asks why Audio engine
says unavailable. Backend `audio_status_for` uses unavailable for an unprepared
or stopped native worker; it is not a separate engine failure signal.
UI-27/UI-19/UI-17 scope: move the existing read-only setup-check lifecycle to
the App, run after its initial snapshot and on relevant device/saved-route/
running-session changes even outside Status, and share the result with the
panel. No periodic telemetry polling. Reserve a dot slot on the Status tab;
red means a problem finding, with an accessible description. Keep the last
result during refresh and failed reads. Use explicit stopped/idle wording for
zero active sessions and preserve unavailable/disconnected states separately.
The prior tab-open-only trigger decision is superseded by this owner request.

Verify hook request lifetime and refresh behavior, indicator semantics and
stable bounds, stopped-vs-running/disconnected checklist text, full UI checks,
three-theme browser screenshots and contract drift. No API/schema or NodeKind
changes. Rollback: revert the UI-only follow-up. Owner confirmed closure and
authorized rebuilding.

Verification: UI typecheck, lint, production build and 793 tests (87 files)
passed on Windows. Edge reported three passing setup-check cases across dark,
light and high-contrast themes; inspected screenshots. The dot appears before
opening Status, clears after a clean result and preserves tab/device bounds.
Runner teardown stalled after those passing cases; no clean runner exit is
claimed. Existing chunk-size, attribution and color-environment warnings remain.

Embedded release build passed with `custom-protocol --locked`. Artifact:
`src-tauri/target/release/audiorouter-shell.exe`; SHA256
`2C22E42DEDDB9A298C2A8E165E14608E63B9C123DDD04FFB572EBAA97C9E7D32`.
Disposable-profile hidden native launch reached shell.instanceCheck, but the
window was not exposed by computer-use; native visual validation remains for
owner testing. Stopped owned test process 56180, leaving the separate older
local-test executable alone. No audio started or device settings changed.
Owner is starting Siege; live-audio checks remain deferred. Next: owner tests
this local artifact.

PR preparation (2026-11-02): owner requested preparing the PR while testing
tonight. Prepare as draft, preserving the pending owner test and validation
limitations above; do not merge or mark ready. Final UI format check, contract
drift (138 methods, 37 kinds, 20 processors, 22 events) and diff whitespace
checks passed. This follow-up changes only UI and documentation; the earlier
Rust verification remains recorded above. No further live-audio checks.
