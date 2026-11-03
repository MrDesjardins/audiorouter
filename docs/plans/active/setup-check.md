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

Owner review of the branch, then a PR only with explicit approval.
