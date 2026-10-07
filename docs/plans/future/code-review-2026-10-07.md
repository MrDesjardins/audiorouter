# Repository code review — 2026-10-07

A full survey of the backend, the APIs (named-pipe JSON-RPC and local HTTP),
the MCP server and the frontend. It covers performance, tests, user
experience, reliability, debuggability and other non-functional
requirements. Findings are ranked P0 to P3. Like every plan in this folder,
this is a backlog, not authorized work. Each item needs the user's go-ahead
and a place in the [active plan](../active/current.md).

## How this review was done

- **Source:** `main` at `5a13a04`. The repository is about 129,000 lines of
  Rust (about 78,000 of them production code, the rest tests and tools) and 18,400
  lines of TypeScript.
- **Method:** static reading, metrics (size, `unwrap`/`expect`, `unsafe`,
  locks, sleeps, test counts), and tracing of the request and audio paths.
- **Limits:** the review ran in a Linux container. Nothing was measured on
  Windows or with audio hardware. Items marked **(measure)** need a Windows
  measurement before or while they are fixed. Line numbers refer to
  `5a13a04` and will drift.

## What is already strong

- **Real-time audio path.** The engine's per-block processing allocates
  nothing (no `vec!`, `collect`, `format!`, `Box` or `clone` of buffers found
  in `process*` functions). Network send uses a lock-free queue, and network
  playout uses `try_lock`, so the audio thread never blocks on them.
- **Failure containment.** VST plugins run in supervised worker processes. A
  failed processor on a voice path goes silent. The backend thread catches
  panics and restarts, recording the panic location in `shell.jsonl`.
- **Security basics.** The pipe uses `PIPE_REJECT_REMOTE_CLIENTS`, checks the
  caller's Windows SID and applies per-client grants. The HTTP API compares
  tokens in constant time, checks Host and Origin, limits sizes and time,
  and rate-limits requests. Decoded control values are bounded before
  dispatch.
- **Privacy-aware logs.** Logs record methods, outcomes and graph counts,
  never parameters, paths or audio, and their size is capped.
- **Tests and CI.** There are about 1,150 Rust tests and 480 Vitest tests,
  plus 97 Playwright tests and a sine-wave continuity harness. Windows CI now
  enforces formatting, strict Clippy, the unit tests, contract drift and the
  installer build.
- **MCP design.** Tools have schemas, annotations, idempotency keys,
  structured content, a recipes entry point and an activity log.

## Summary

| ID | Priority | Area | Finding |
| --- | --- | --- | --- |
| [P0-1](#p0-1-blocking-work-runs-on-the-audio-service-thread) | P0 | Backend | Slow requests run on the thread that pumps audio |
| [P0-2](#p0-2-one-pipe-instance-and-a-100-ms-client-budget) | P0 | API | One pipe instance plus a ~100 ms client retry budget cause spurious failures |
| [P0-3](#p0-3-the-shipped-backend-never-writes-backendjsonl) | P0 | Debuggability | The shipped backend never writes `backend.jsonl` |
| [P1-1](#p1-1-shelljsonl-is-flooded-at-20-hz-and-logging-can-block-forever) | P1 | Debuggability | `shell.jsonl` fills at 20 Hz; log writes can wait forever |
| [P1-2](#p1-2-no-proof-that-audio-stays-real-time-and-in-budget) | P1 | Performance | No guard or benchmark for the real-time path (NFR-04/05) |
| [P1-3](#p1-3-sqlite-is-not-tuned-for-commits-on-the-audio-thread) | P1 | Backend | SQLite durability and locking settings are defaults |
| [P1-4](#p1-4-windows-audio-tests-and-browser-tests-never-run-in-ci) | P1 | Tests | `windows-audio` unit tests and Playwright tests never run in CI |
| [P1-5](#p1-5-mcp-exposes-76-tools-including-internal-plumbing) | P1 | MCP | 76 tools, including internal plumbing; one bad line kills the server |
| [P1-6](#p1-6-a-render-error-blanks-the-whole-window) | P1 | Frontend | No error boundary: a render error blanks the window |
| [P2-1](#p2-1-the-control-plane-is-one-21500-line-file) | P2 | Backend | The control plane is one 21,500-line file |
| [P2-2](#p2-2-apptsx-is-a-2600-line-component) | P2 | Frontend | `App.tsx` is a 2,600-line component; no lint or formatter |
| [P2-3](#p2-3-no-way-to-follow-one-request-across-logs) | P2 | Debuggability | No request ID across UI, shell and backend; no verbose mode |
| [P2-4](#p2-4-dependency-and-supply-chain-checks-are-missing) | P2 | Security | No dependency updates or vulnerability audit |
| [P2-5](#p2-5-network-audio-accepts-any-packet-from-the-senders-ip) | P2 | Security | Network audio trusts the sender's IP only |
| [P2-6](#p2-6-http-adapter-polls-and-drops-connections-silently) | P2 | API | HTTP accept loop polls every 10 ms and drops overflow silently |
| [P2-7](#p2-7-dsp-loops-look-up-channels-per-sample) | P2 | Performance | DSP loops look up channels per sample |
| [P2-8](#p2-8-no-coverage-or-soak-automation) | P2 | Tests | No coverage reports; soak and startup NFRs are manual |
| [P3-1](#p3-1-smaller-items) | P3 | Mixed | Smaller cleanups |

## Status: P0 and P1 fixed (2026-10-07, user request "Fix all P0 and P1")

All P0 and P1 items were fixed on branch `claude/zealous-archimedes-fz1lbv`.
The details are in the [active plan](../archived/2026-10-07-maintenance-0.0.12-to-code-review.md#code-review-p0-and-p1-fixes-2026-10-07-user-request).
P2 and P3 remain a backlog.

| ID | Fix | Commit | What remains |
| --- | --- | --- | --- |
| P0-1 | Plug-in scan/inspect, audio decoding, plug-in re-hash and worker start at Play, and the file part of session-file export/import (ZIP, hashing, asset files) run on a worker while the control thread keeps servicing audio every 1 ms | `2cae3569`, see the active plan | SQLite reads and writes stay on the control thread (session and imported audio for export, restored assets for import, saved plug-in state at Play); database backups; the sine-continuity measurement on Windows |
| P0-2 | Four pipe instances; I/O threads survive bad connections; clients wait up to 2 s on a busy pipe | `72aa0731` | — |
| P0-3 | The production loop logs RPCs through a bounded queue to a writer thread | `f8440b20` | — |
| P1-1 | Successful 20 Hz polls are not logged; the log mutex wait is bounded (250 ms); panic hook uses `try_lock` | `f8440b20` | — |
| P1-2 | Counting-allocator test: zero allocations for every tool and layout; `benches/realtime.rs` | `9b1ac2ea` | A bare route costs ~23 µs per quantum whatever the tool (see P2-7) |
| P1-3 | WAL, `synchronous=NORMAL`, 2 s busy timeout, `BEGIN IMMEDIATE` writes | `69ab047c` | — |
| P1-4 | `windows-audio` tests run in CI (device tests ignored); new Playwright job in Edge | `e3621dcc` | Backend-fixture browser tests are first run by that job |
| P1-5 | 38 task-level MCP tools by default, `--advanced-tools` for the rest; bad lines get -32700; refusal without `--pipe` while the app runs | `0fa970f0` | — |
| P1-6 | Error boundaries around the canvas, inspector and side panel, plus a root recovery panel with privacy mute | `8477b97a` | — |

## P0 — fix first

### P0-1 Blocking work runs on the audio service thread

- **Evidence.** One thread owns the control plane, dispatches every request
  and pumps native audio in between (`serve_control_plane_frames`,
  `crates/transport/src/lib.rs:1242`; the pass runs every 1 ms). Several
  handlers do long synchronous work on that thread:
  - plugin scans walk folders and SHA-256-hash every plugin binary
    (`dispatch_plugins_scan`, `crates/control/src/lib.rs:19686`);
  - audio-file sources decode and resample files of up to 64 MB / 120 s
    (`decode_audio_bytes` calls at `crates/control/src/lib.rs:6383`,
    `17309` and `17419`);
  - session bundle export and import (`crates/control/src/lib.rs:16266`,
    `16272`), database backups, and every SQLite commit.
- **Impact.** While audio plays, any of these can delay the next pump past
  the 10 ms WASAPI period. That empties render buffers, which the user hears
  as dropouts. Every other client (window, tray, HTTP, Stream Deck, MCP)
  also waits. The code itself says "keep any new request handler short"; the
  handlers above don't. Affects NFR-04, NFR-07 and NFR-11.
- **Fix.** Move long work to a worker thread and return an operation handle
  (the API already has `operations.get` and `operations.cancel`). The
  control thread then applies the result in a short step. Add a test that
  fails if a pass is delayed more than a few milliseconds while a long
  request runs, using the existing `AudioServicePassTiming` observer.
- **Verify.** **(measure)** Run the sine-continuity harness while scanning a
  large VST folder and while importing a long audio file. Require zero
  glitches.

### P0-2 One pipe instance and a 100 ms client budget

- **Evidence.**
  - The control pipe is created with `nMaxInstances = 1`
    (`crates/transport/src/lib.rs:648`), and the server re-creates it after
    every connection.
  - Each client call opens a new connection. It retries only 20 × 5 ms
    while the pipe is busy or missing (`round_trip`,
    `crates/transport/src/lib.rs:767`, also `816` and `889`), then fails
    with "timed out waiting for a free named-pipe instance".
  - The window alone polls `system.diagnostics` 20 times a second while
    playing (`DIAGNOSTICS_REFRESH_INTERVAL_MS`, `ui/src/App.tsx:88`).
- **Impact.** The window, tray, four HTTP workers (Stream Deck), the MCP
  proxy and the CLI all compete for one connection. Under load, Stream Deck
  keys, MCP calls or tray actions can fail at random with that error. CI
  already hit this exact error (fixed in the test by waiting longer; the
  product is unchanged).
- **Fix.** Serve several pipe instances (dispatch is already serialized
  through a channel, so concurrency stays safe). Or give clients a longer,
  `WaitNamedPipeW`-based budget, with a short one only for the 20 Hz poll.
  Reuse one connection per client where the protocol allows it.
- **Verify.** A Windows test that runs the diagnostics poll at 20 Hz while
  firing HTTP requests from four workers, and requires zero
  pipe-unavailable errors.

### P0-3 The shipped backend never writes `backend.jsonl`

- **Evidence.**
  - `log_backend_rpc` (`crates/transport/src/lib.rs:80`) is called only
    from the bounded `serve_control_connections` loop (line 1003), which
    the CLI's `backend serve` uses.
  - The production loop's `dispatch_control_frame` doesn't log.
  - Yet the window's **Logs** tab asks users to attach `backend.jsonl`, and
    its "Backend RPC" list reads that file (`backend_diagnostics_list`,
    `src-tauri/src/main.rs:863`).
- **Impact.** Support logs from real users have no backend-side record. That
  includes requests that never pass through the shell: HTTP API, Stream
  Deck, MCP and CLI. The Logs tab's backend list is empty in the installed
  app.
- **Fix.** Log from the production loop, keeping the file I/O off the audio
  thread: send a small record over a bounded channel to a logger thread,
  and drop records if it is full. Include the client kind (shell, HTTP,
  MCP, CLI) from the grant, without identifying data.
- **Verify.** A test that runs the production loop and asserts that a
  dispatched request appears in `backend.jsonl`. Also confirm the Logs tab
  lists it.

## P1 — next

### P1-1 `shell.jsonl` is flooded at 20 Hz, and logging can block forever

- **Evidence.**
  - `log_shell_rpc` skips only the pump and heartbeat methods
    (`src-tauri/src/main.rs:52`), so the 20 Hz `system.diagnostics` poll is
    probably logged on every call.
  - Each write opens the file and waits on a cross-process mutex with
    `INFINITE` timeout (`crates/transport/src/lib.rs:65`). The panic hook
    logs through the same path (`install_panic_log`,
    `src-tauri/src/main.rs:1549`).
- **Impact.**
  - **Rotation:** **(measure)** with a 5 MB cap, the file likely rotates
    every few tens of minutes while audio plays, so the useful entries from
    an hour ago are gone by the time a user reports a problem.
  - **Blocking:** a stuck writer in another process blocks the shell.
  - **Deadlock:** a panic while holding the log lock could deadlock.
- **Fix.**
  - Don't log successful high-frequency reads. Keep failures and summarize
    successes, for example one line per minute with counts.
  - Wait on the mutex with a short timeout and skip the record on timeout.
  - In the panic hook, use a lock-free best-effort write.
- **Verify.** Play for 30 minutes with the window open, then check the log's
  size and that it still holds older events.

### P1-2 No proof that audio stays real-time and in budget

- **Evidence.**
  - No test guards against allocation on the processing path (no counting
    allocator).
  - No benchmarks exist (no `benches/`, Criterion or Divan).
  - NFR-04 (callback p99.9 under 50% of the deadline) and NFR-05 (CPU at
    most 10% on average) are checked only by attended runs.
- **Impact.** A regression that allocates or slows a DSP tool would ship
  unnoticed until users hear crackles.
- **Fix.**
  - Add a test-only counting allocator, and assert zero allocations across
    `process` and `process_paths_to_rings` for every tool in
    `crates/engine/tests/tool_combinations.rs` `TOOLS`.
  - Add Criterion benchmarks for the most expensive tools (Denoise,
    spectral, pitch, binaural, EQ) and for a 32-node route. Record
    baselines, and fail CI on a large slowdown, or report it in a
    non-blocking job.
- **Verify.** The allocation test fails if someone adds a `Vec` to a
  `process` function.

### P1-3 SQLite is not tuned for commits on the audio thread

- **Evidence.** The database opens with `PRAGMA foreign_keys = ON` only
  (`crates/storage/src/lib.rs:905`). It sets no `journal_mode`,
  `synchronous` or `busy_timeout`.
- **Impact.**
  - **Slow commits:** with the default rollback journal and full sync,
    each commit flushes to disk several times, on the audio service thread
    (see P0-1).
  - **Lock errors:** without a busy timeout, a second process (CLI, or MCP
    run without `--pipe`) gets an immediate "database is locked" error
    instead of waiting.
- **Fix.**
  - **(measure)** Time commits first.
  - Then consider `journal_mode=WAL` with `synchronous=NORMAL`, and a
    `busy_timeout` of a few hundred milliseconds. Check that the backup and
    restore paths and the integrity checks still hold under WAL.
- **Verify.** Commit latency p95 on the reference PC, before and after.
  Keep the existing corrupt-database and backup tests green.

### P1-4 `windows-audio` tests and browser tests never run in CI

- **Evidence.**
  - CI runs `cargo test --workspace --exclude audiorouter-windows-audio`
    (`.github/workflows/ci.yml:58`). That crate has 105 unit tests, and only
    2 are marked `#[ignore]`. Its test module contains no device COM calls,
    so most are probably hardware-free.
  - The 97 Playwright tests (`ui/e2e/*.pw.ts`) don't run in CI.
- **Impact.**
  - Regressions in format conversion, rings, resampling and network audio
    reach `main` unseen.
  - So do UI regressions that jsdom cannot see: layout, canvas,
    drag-and-drop, and theme rendering, which `AGENTS.md` requires.
- **Fix.**
  - Run `cargo test -p audiorouter-windows-audio`, and mark the tests that
    need devices with `#[ignore = "needs audio devices"]`.
  - Add a CI job for a core Playwright subset against the `e2e_backend`
    example, saving screenshots and traces as artifacts on failure.
- **Verify.** Both jobs green on `main`, with failure artifacts downloadable
  from the run.

### P1-5 MCP exposes 76 tools, including internal plumbing

- **Evidence.**
  - `mcp_tools()` (`crates/cli/src/lib.rs:2387`) lists 76 tools. Besides
    task-level tools (sessions, add or connect tools, play, mute, levels,
    recording), they include `pump_native_*`, `heartbeat_native_bridges`,
    `prepare_native_bridge`, `os_transition`, `provision_virtual_device` and
    a generic `call_api`.
  - A single malformed JSON line ends the server
    (`crates/cli/src/lib.rs:2112` returns an error), instead of replying
    with a JSON-RPC parse error (-32700) and continuing.
- **Impact.**
  - Every assistant turn carries all 76 schemas, which costs context and
    makes wrong tool choices more likely.
  - The pump and bridge tools are leftovers now that the backend pumps
    audio itself.
  - One bad message from a client drops the whole connection.
- **Fix.**
  - Default to a task-level set of about 25 tools, and expose the internal
    ones only with a `--advanced` flag or grant.
  - Return a -32700 parse error and keep reading.
  - Refuse `mcp serve` without `--pipe` when the app's backend is running,
    so two control planes never edit one database.
- **Verify.** An MCP stdio test sends a malformed line followed by a valid
  call. Another test asserts the default tool list.

### P1-6 A render error blanks the whole window

- **Evidence.** The UI has no React error boundary (no
  `componentDidCatch` or `ErrorBoundary`).
- **Impact.** Any exception during render, for example from unexpected
  telemetry or a malformed node from an older session, unmounts the app and
  leaves an empty window. The audio may keep running with no visible
  control.
- **Fix.**
  - Add boundaries around the canvas, the inspector panels and the app root.
  - Show a recovery panel with **Reload window**, and add the error category
    to the client diagnostics.
  - Keep the top bar's **Stop** and privacy mute outside the canvas
    boundary, so audio can still be stopped.
- **Verify.** A Vitest test that throws inside a panel and asserts that the
  recovery panel appears and Stop still works.

## P2 — planned improvements

### P2-1 The control plane is one 21,500-line file

- **Evidence.** `crates/control/src/lib.rs` has 21,534 production lines and
  32,768 lines in total. It contains 113 `dispatch_*` handlers, about 527
  method match arms, and a `ControlPlane` struct whose fields span 138
  lines. It has 242 tests, about one per 90 production lines.
- **Impact.** Hard to review, slow to compile incrementally, and hard to
  find the owner of a behavior. Large merges conflict often. P0-1 also
  needs a clear split between fast handlers and long jobs.
- **Fix.** Split by domain into modules: graph, devices and native
  endpoints, recording, plugins, virtual devices and cable, sessions and
  storage, safety. Use a typed method table instead of string matching.
  Move tests next to each module. Do it as mechanical moves, one domain per
  pull request, with no behavior change.
- **Verify.** Same test results. Each module stays under about 3,000 lines.
- **Status (2026-10-07, user request): split done, as mechanical moves.**
  `lib.rs` now holds the `ControlPlane` struct, `new`, the dispatch entry
  points (`dispatch*`, handshake), `ControlError` and the error-response
  helpers: 1,290 lines including 7 dispatch-core tests. Each domain module
  adds its own `impl ControlPlane` block. Its tests live in
  `<module>_tests.rs` (the `simple_tests.rs` layout), and shared fixtures
  live in `test_support.rs`. Module sizes in lines (code / tests):
  `api_output_schema` 2,045; `recorder_workers` 1,895 / 582;
  `native_dispatch` 1,374 / 359; `sessions` 1,360 / 478; `native_workers`
  1,353 / 1,044; `plugins` 1,323 / 438; `recording` 1,295 / 1,635;
  `api_schema` 952; `native_paths` 942 / 355; `virtual_devices` 928 / 687;
  `native_bindings` 903 / 203; `native_application` 819 / 265; `graph`
  789 / 418; `safety` 699 / 676; `persistence` 693 / 463;
  `recording_library` 670 / 394; `status` 607 / 385; `audio_media`
  603 / 227; `authorization` 592 / 517; `catalog` 460 / 1,174;
  `audio_service` 450 / 22; `network` 392 / 576; `test_support` 257.
  The only change to moved code is visibility: private items and the
  struct fields that other modules use became `pub(crate)`. Public paths
  are re-exported from `lib.rs`, so `cli`, `transport` and the examples
  compile without source changes. `src-tauri` uses only root paths
  (`ControlPlane`, `ClientGrant`, `ClientRole`, `os_transition`), which are
  kept; it was not built here. Checks: 244 `#[test]` functions before and
  after; the `pub` item list is identical (198); all 883 items (functions,
  methods, types, impls) match their originals once whitespace,
  `pub(crate)` and the trailing commas rustfmt adds are ignored; all 960
  comment lines are kept. Workspace Clippy for `x86_64-pc-windows-gnu`
  with `-D warnings` is clean, and `cargo fmt --check` is clean. The tests
  were not run: they need Windows.
  Not done: the typed method table. `dispatch` still matches method
  strings. Replacing that match is a behavior-sensitive change and should
  be its own change. `method_output_schema` is still a single
  1,400-line function.

### P2-2 `App.tsx` is a 2,600-line component

- **Evidence.**
  - `ui/src/App.tsx` has 2,588 lines, 152 `useState` and 51 `useEffect`
    calls.
  - Its longest line is 12,575 characters, and 85 lines exceed 400
    characters. `SessionFlowCanvas.tsx` is similar.
  - The UI has no ESLint (including the React hooks rules) and no Prettier.
- **Impact.** Effects with missing dependencies and stale closures are easy
  to introduce and hard to review. Diffs of one-line JSX blocks are
  unreadable.
- **Fix.**
  - Add Prettier and ESLint with `react-hooks`, then format in one
    mechanical commit, following the same rule as the Rust formatting.
  - Extract the polling loops (diagnostics, pump, events) into hooks, and
    the panels into components.
  - Enforce both in CI and in the pre-commit hook.
- **Verify.** CI lint is green, behavior unchanged, and all Vitest and
  Playwright tests pass.
- **Status (2026-10-07, user request): done in part, Windows CI pending.**
  - ESLint 10 (flat config, typescript-eslint recommended,
    `react-hooks/rules-of-hooks` error, `exhaustive-deps` warning) and
    Prettier 3.9 (`printWidth` 120, `endOfLine` auto) in `ui/`; scripts
    `lint`, `format`, `format:check`. `ui/src` and `ui/e2e` were formatted
    in one mechanical commit (`2697774b`). CI's Windows job runs
    `format:check` and `lint` (errors only fail); the pre-commit hook
    formats staged `ui/src` and `ui/e2e` files.
  - Lint: 52 errors fixed (0 left, `rules-of-hooks` 0). 25
    `exhaustive-deps` warnings remain, each reviewed; most are deliberate
    (stable setters, mount-only effects, 20 Hz telemetry). One was a real
    stale closure: Ctrl+Alt+S after Save started a "temporary preview of
    the unsaved route" instead of the saved session (fixed, regression
    `e2e/shortcut-after-save.pw.ts`).
  - `App.tsx`: 2,610 lines before formatting, 8,543 after Prettier,
    3,069 now (useState 152 -> 74, useEffect 51 -> 33). Moved verbatim:
    61 top-level panels and helpers (`AdvancedPanels`, `NativeDevicePanels`,
    `PluginPanels`, `RecordingPanels`, `NodeEditors`, `AudioFileNodeEditor`,
    `PanelMessage`, `endpointBinding`, `appContext`); the four polling
    loops into hooks (`useWorkspaceEvents`, `useDiagnosticsRefresh`,
    `useNativeCounters`, `useAudioFileStatus`); the inline panels into
    components (`SignalFlowPanel`, `PropertiesPanel`, `SetupWorkbench`,
    `RecordingWorkbench`, `AdvancedWorkbench`, `ConnectionForm`,
    `DeviceTroubleshooting`). Effects keep their dependency arrays; the
    rendered HTML of every side-panel tab and node inspector is unchanged
    in the dark, light and high-contrast themes.
  - Open: the ~1,500-line target is not met. What remains in `AppContent`
    is its state and action handlers (`startSession` alone is ~360
    lines); moving them is a state-flow change and needs its own review.
    `SessionFlowCanvas.tsx` was only formatted.

### P2-3 No way to follow one request across logs

- **Evidence.** UI, shell and backend logs share no request or correlation
  ID, and nothing switches on verbose logging for a support case.
- **Impact.** Matching a user's "Play failed at 20:14" across
  `client diagnostics`, `shell.jsonl` and `backend.jsonl` relies on
  timestamps alone.
- **Fix.**
  - Generate a short request ID in the UI and HTTP adapter, carry it in the
    JSON-RPC request, and log it at each layer.
  - Add an opt-in verbose mode that times out after one hour, still with
    no parameters or paths.
  - Offer a "copy support bundle" action that zips the logs and app
    version.
- **Verify.** One action produces matching IDs in all three logs.
- **Status (2026-10-07, commit `00ecb1ed`).** Done in code; Windows proof
  pending. Requests carry an optional top-level `requestId` (1–32
  characters, `[A-Za-z0-9-]`; invalid values are dropped). The window makes
  one per request, the tray one per action, the HTTP adapter one per request
  (or a valid `X-Request-Id`, echoed in the response), and MCP one per tool
  call. `shell.jsonl`, `backend.jsonl` and `mcp-activity.jsonl` record it,
  and failed window requests add a client diagnostics row with it.
  `diagnostics.getVerbose`/`setVerbose` give a one-hour verbose window
  (routine reads and `durationMs`). Logs → Copy support bundle writes a
  local ZIP. Linux checks: protocol unit tests, UI tests, the Logs tab in
  Chromium in three themes, and Windows-target Clippy. The transport, shell
  and MCP tests and an end-to-end ID match need Windows CI or an attended
  run.

### P2-4 Dependency and supply-chain checks are missing

- **Evidence.**
  - The repository has no `.github/dependabot.yml`, and CI runs no
    `cargo audit`, `cargo deny` or `npm audit`.
  - Actions pin Node 20 releases that GitHub now warns are deprecated.
- **Impact.** Known vulnerabilities in dependencies (Tauri, symphonia,
  rusqlite, zip, React) go unnoticed. Release SBOMs exist, but nobody checks
  them.
- **Fix.**
  - Add Dependabot for Cargo, npm and Actions.
  - Add a `cargo deny` job (advisories, licenses, sources) and `npm audit
    --omit=dev` for `ui`, `contracts` and `tools/streamdeck`.
  - Bump `actions/checkout` and `actions/setup-node`.
- **Verify.** The jobs run on pull requests, and the licenses match
  `THIRD-PARTY-NOTICES.txt`.
- **Status (2026-10-07, user request).** Done, except a first CI run.
  - `.github/dependabot.yml`: weekly Cargo (root, `src-tauri`,
    `tools/m00-wasapi-probe`), npm (`ui`, `contracts`, `tools/streamdeck`)
    and Actions updates, minor and patch grouped.
  - `deny.toml` plus a `supply-chain` CI job (Linux) for both Cargo
    workspaces, limited to the Windows target, and `npm audit --omit=dev
    --audit-level=high` for the three npm projects. Licenses allowed:
    Apache-2.0, MIT, MIT-0, 0BSD, BSD-2/3-Clause, Zlib, Unicode-3.0,
    Unlicense, CC0-1.0, MPL-2.0 and LGPL-3.0-only (mp3lame). The workspace
    crates are now `publish = false`.
  - Advisories: the root workspace is clean. `src-tauri` has five
    "unmaintained" notices for the rust-unic crates (RUSTSEC-2025-0075,
    -0080, -0081, -0098, -0100), via `tauri-utils` 2.9.3 and `urlpattern`
    0.3. They are ignored with a reason until 2027-01-07, because the
    `tauri-utils` 2.10 update also moves several Tauri build crates and
    needs a Windows check. npm: no runtime findings; a high dev-only
    `source-map-js` finding in `ui` was fixed in the lockfile (1.2.2).
  - `actions/checkout` v7.0.1, `setup-node` v7.0.0 and `upload-artifact`
    v7.0.1 (all Node 24), pinned by SHA in every workflow; the Pages
    actions too.
  - Remaining: `THIRD-PARTY-NOTICES.txt` lists only the root workspace's
    crates, not the shell's Tauri tree (`src-tauri/Cargo.lock`).

### P2-5 Network audio accepts any packet from the sender's IP

- **Evidence.** Network Receive accepts datagrams whose source IP matches
  the configured sender (`crates/windows-audio/src/network_audio.rs:698`).
  The traffic has no authentication or encryption.
- **Impact.** Another device on the same LAN can spoof the source address
  and inject audio into a stream that may feed OBS or Discord.
  Eavesdropping is also possible.
- **Fix.** Pair Send and Receive with a shared key shown in both apps, and
  authenticate each packet with a cheap MAC (keyed BLAKE3 or HMAC on a
  counter). Reject replayed sequence numbers. Document the remaining risk
  until then.
- **Verify.** A test with a correct-IP, wrong-key packet expects it to be
  rejected and counted.
- **Status (2026-10-07, user request): fixed, Windows CI pending.** Both
  nodes take an optional `pairingKey`. With a key, the sender's I/O thread
  appends a 16-byte HMAC-SHA256 tag (packet version 2); the receive thread
  verifies it in constant time and drops replays with a 64-packet window
  per stream id, counting `authFailures` (with the reason) and
  `replayedPackets`. Blank keys keep version 1 unchanged. The inspector has
  Generate/Copy and a reserved "Not paired" warning. Portable logic is in
  `crates/protocol/src/network_audio.rs`; details and evidence are in the
  [active plan](../active/current.md#code-review-p2p3-follow-ups-2026-10-07-user-request).
  Still open: encryption (listening stays possible) and replay of an
  earlier stream to a restarted receiver.

### P2-6 HTTP adapter polls and drops connections silently

- **Evidence.**
  - The accept loop uses non-blocking listeners and sleeps 10 ms when
    idle (`src-tauri/src/http_api.rs:153`).
  - When the 32-socket queue is full, `try_send` drops the connection
    without a response (`:145`).
- **Impact.** Up to 10 ms extra latency on every Stream Deck press, and 100
  wake-ups a second while idle. Clients see a hang instead of a 503 error.
- **Fix.** Use blocking accept per listener on its own thread, or a single
  readiness wait. Answer `503` with `Retry-After` when the queue is full.
- **Verify.** Measure the latency of an idle-to-first request, and test the
  queue-full response.
- **Status (2026-10-07): fixed in code, Windows run pending.** Intake moved to
  `src-tauri/src/http_accept.rs` (std only). Each listener has its own
  blocking accept thread, and workers block in `recv`, so nothing polls.
  Stop sets the flag and connects to each listener to wake its accept. A
  thread that cannot be woken (the network address left the PC) is detached
  instead of hanging the caller. A full queue answers 503 with
  `Retry-After: 1` and a JSON `error.message`. The write is bounded at
  100 ms and the linger at 20 ms. Tests cover the 503, a stop under 500 ms
  that frees the port, one accept call while idle, and refused peers. They
  passed natively on Linux (25 repeated runs) through a scratch crate that
  includes the module. Windows-target Clippy is clean. Running the shell
  tests on Windows, and measuring idle-to-first-request latency, are still
  open.

### P2-7 DSP loops look up channels per sample

- **Evidence.**
  - Hot loops call `block.channel(channel).unwrap()[frame]` inside per-frame
    loops, for example the fixed delay (`crates/engine/src/lib.rs:125`),
    gain ramp (`1413`), interleave conversions (`1497`, `1527`), channel
    matrix and mixing (`1572` to `1642`), and resampling (`1683`).
  - That is 29 `unwrap`s in engine production code, all invariant-guarded.
- **Impact.** Per-sample `Option` and bounds checks with frame-major access
  prevent vectorization and cost CPU on every block (NFR-05). A violated
  invariant panics the audio service thread, which restarts the backend.
- **Fix.** After P1-2's benchmarks exist, borrow the channel slices once per
  block, iterate channel-major or with `chunks_exact`, and replace `unwrap`
  with shape checks done once at the top.
- **Verify.** Benchmarks show the gain, and the continuity harness stays
  glitch-free.
- **Status (2026-10-07).** Done except the Windows continuity run. The
  hot loops borrow channel slices once and iterate channel-major; their 21
  `unwrap`s (19 in the kernels, 2 in linked dynamics) are gone (shape checks were already
  at the top; the remaining engine `unwrap`s are in graph compilation and
  pool setup). The ~23 µs per-route cost came from `BlockMeter::observe`:
  each route observes about eight meters per quantum, and each observation
  walked the block about eight times (peak, RMS, per-channel peak twice,
  per-channel RMS, clip count). It is now one vectorizable pass plus the
  sequential f64 sums, with bit-identical results. Median µs per quantum,
  `cargo bench -p audiorouter-engine --bench realtime`, Linux container:

  | Route | Before | After |
  | --- | ---: | ---: |
  | Denoise | 66.3 | 46.4 |
  | SpeechDenoise | 67.0 | 47.1 |
  | SpectralGate | 63.5 | 43.6 |
  | Pitch | 93.5 | 72.9 |
  | ParametricEq | 24.3 | 3.7 |
  | GraphicEq | 34.2 | 13.9 |
  | FirFilter | 23.9 | 3.5 |
  | Compressor | 26.9 | 6.6 |
  | Gate | 36.7 | 16.7 |
  | Limiter | 25.1 | 5.2 |
  | Dehum | 27.6 | 7.7 |
  | Declick | 24.6 | 4.7 |
  | BassTreble | 25.6 | 5.6 |
  | Delay | 25.5 | 5.6 |
  | Meter | 26.0 | 3.9 |
  | Gain | 23.4 | 3.5 |
  | route-32-tools | 680.0 | 507.6 |

  The 32-tool route is now dominated by the DSP tools themselves.

### P2-8 No coverage or soak automation

- **Evidence.** No coverage tooling exists for Rust or the UI. NFR-11
  (24-hour run, 100 sleep/resume cycles), NFR-06 (memory growth) and NFR-14
  (startup time) are checked by hand.
- **Fix.**
  - Report coverage with `cargo llvm-cov` and Vitest `--coverage`; don't gate
    on it at first.
  - Add a nightly or manual Windows job that runs a long route on the
    software render path and records memory and CPU against NFR-05/06.
  - Time the app's startup in the installer smoke test.
- **Verify.** Trend charts in the job summary.
- **Status (2026-10-07, user request).** Done, except a first run on
  GitHub. `.github/workflows/quality.yml` runs nightly and on demand, on
  Windows:
  - **Coverage** (never gated): `cargo llvm-cov` for the whole workspace
    (per-crate table, portable crates totalled apart) and for `src-tauri`;
    Vitest with `@vitest/coverage-v8`. Reports are uploaded. Local Linux
    baseline: portable crates 86.5 % of lines, UI 74.5 % of lines.
  - **Soak:** `crates/engine/examples/soak.rs` runs a heavier W1 (11 tools,
    a Mixer, three outputs) through `RealtimeMixerFanout` for 20 minutes,
    unpaced. It fails on live-heap growth of 1 MiB or more after warm-up
    (NFR-06 allows <10 MiB over 8 h), on any allocation while processing,
    or when p99 per quantum exceeds 50 % of the 2.67 ms quantum (NFR-04
    asks p99.9 on the reference PC; hosted runners are too noisy for that,
    so p99.9 is reported). The workflow also samples Windows private bytes
    and fails on 10 MiB growth. Mean time per quantum is reported as a
    share of the machine against NFR-05 (engine only, a lower bound).
  - **Startup (NFR-14, partial):** builds the release shell and runs the
    `fresh_install_shell` test, which now records the time from launch to
    the backend's first answer. Missing: a signal that the window's UI is
    ready (the 3 s warm / 8 s cold target), cold versus warm starts, and
    W1 startup with devices. NFR-11 (24 h, sleep/resume) stays manual.
  - **Trend:** a CSV history in the Actions cache; each run prints its last
    14 rows as a table (a table, not a chart).

## P3 — when convenient

### P3-1 Smaller items

- **UI pump loop.** It still calls a pump RPC 10 times a second while
  playing, even when the backend pumps audio itself
  (`ui/src/App.tsx:1795`). Read the counters from `system.diagnostics`, and
  remove the legacy 5 ms path once older backends are no longer supported.
  *Status 2026-10-07: done (`e9ca815e`).* The legacy 5 ms path is gone;
  the UI reads the counters once a second from the pump result, since the
  shell always ships its own backend. The three polling loops record one
  Logs-tab row per failure run. React now has its own chunk (main chunk
  635 → 460 kB); the rarely used panels total ~10 kB, so lazy-loading them
  was not worth it.
- **Release profile.** Consider `lto = "thin"` and `codegen-units = 1` for
  release, measured against build time and binary size.
  *Status 2026-10-07: measured, not adopted.* `cargo bench -p
  audiorouter-engine --bench realtime` with both settings (Linux container)
  was within ±2.6 % of the default for every tool and +0.4 % for the
  32-tool route, which is noise. The audio cost is in the DSP code, not in
  cross-crate calls, so the longer release builds would buy nothing.
- **Unsafe audit.** `windows-audio` has 115 `unsafe` blocks. Check that each
  one has the invariant comment `AGENTS.md` requires, and add a
  `clippy::undocumented_unsafe_blocks` lint.
  **Status (2026-10-07): done.** 188 undocumented `unsafe` blocks/impls had
  no `// SAFETY:` comment: windows-audio 98, transport 27, plugin-host 19,
  the engine allocator test 4, the desktop shell 26 and
  `tools/m00-wasapi-probe` 14. Each now states its invariants.
  `undocumented_unsafe_blocks` and `missing_safety_doc` are `deny` through
  `[workspace.lints.clippy]` (every member opts in with
  `[lints] workspace = true`) and `[lints.clippy]` in `src-tauri` and the
  probe, so CI's Clippy covers every target, tests included. Soundness
  fixes made on the way: WASAPI packet copies now reject a stride larger
  than the stream's block size (a safe API could read or write past the
  device buffer); the software-device callback no longer uses its context
  after sending; the SID lookup reads `TOKEN_USER` unaligned; the shell's
  listener and editor windows free their boxed context once, after the
  window is gone; the probe no longer releases COM objects after
  `CoUninitialize`. Open: the bridge and plug-in shared-memory regions
  write through pointers taken from `MmapMut`'s `&[u8]` view
  (`map.as_ptr()`), which Rust's aliasing rules do not allow; switching to
  `memmap2::MmapRaw` is a follow-up. Details are in the
  [active plan](../active/current.md#code-review-p2p3-follow-ups-2026-10-07-user-request).
- **UI bundle.** The main chunk is 632 kB. Split the rarely used panels (API,
  MCP, Advanced) with dynamic imports.
- **Active plan size.** The active plan is long. Archive completed sections
  so a new agent can find the current state quickly.
  *Status 2026-10-07: done.* Completed sections (release 0.0.12, website,
  CI on Windows, code review P0/P1) moved verbatim to
  [an execution record](../archived/2026-10-07-maintenance-0.0.12-to-code-review.md);
  the active plan went from 774 to about 300 lines.
- **Silent catches.** The UI swallows errors in a few `catch {}` blocks
  (`App.tsx:845`, `868`, `1743`, `1795`). Record a diagnostic category in
  each.

## Suggested order

1. **Visible and cheap:** P0-3 and P1-1, the logs users already send.
2. **Audio quality:** P0-1, with P1-2's timing test as its proof.
3. **Integrations:** P0-2, for reliable Stream Deck, HTTP and MCP.
4. **Safety nets:** P1-4, P1-6 and P1-3, measured first.
5. **Maintainability:** P2-1 and P2-2 as mechanical, one-domain-at-a-time
   pull requests, and the rest of P2 alongside feature work.
