# Resilience audit: UI and backend — 2026-11-13

Owner request (2026-11-13): investigate how to make AudioRouter more
resilient, reliable and fault tolerant, mainly the UI (TypeScript) and the
backend (Rust), and report the findings.

## Status and method

- **Read-only.** No product code changed. Nothing was compiled, run or
  tested (builds starve live audio on the owner's PC). Every finding is
  "confirmed by reading" unless it says "needs measurement".
- Base: `main` at `7d70d5029` (0.0.29). Line numbers will drift; each
  finding also names the function.
- This audit asks one question the earlier ones did not: **after something
  fails, does the app get back to working by itself, and does the user learn
  why it failed?** The [bug audit of 2026-11-11](bug-audit-2026-11-11.md)
  and the [plugin host audit](plugin-reliability.md) looked for wrong
  behavior; their findings are not repeated here.

## Summary

The app fails safely almost everywhere: it goes silent, keeps the saved
route and never substitutes a device. What it rarely does is **recover**.
After a device unplug, a sleep, a backend panic or a plugin crash, audio
stays off until the user presses Stop and Play, and three specification
requirements that describe automatic recovery are not implemented
(STATE-10, STATE-11, CAP-11). The second theme is **data safety**: the
desktop app never makes a backup, so a corrupt database has no recovery
path outside the CLI. The third is **visibility**: the reasons the backend
restarts or gives up are printed to a console that release builds do not
have.

| ID | Area | Severity | Summary |
| --- | --- | --- | --- |
| RES-1 | Storage | High | The desktop app never backs up the database and cannot restore one; a corrupt database ends in a dead backend |
| RES-2 | Backend supervision | High | A panic in any request handler stops all audio, and the restarted backend never resumes the routes (STATE-10) |
| RES-3 | Devices, sleep | High | A device change or sleep stops the whole route and nothing restarts it when the same device returns (CAP-11, STATE-11) |
| RES-4 | Shell | Medium | Pipe clients have no response deadline; tray and Quit calls on the main thread freeze the shell while the backend is busy |
| RES-5 | UI | Medium | "Connection lost" does not clear by itself, and startup stops retrying after about half a second |
| RES-6 | UI | Medium | The 5-second request timeout is shorter than several operations the backend allows 10 to 30 seconds (needs measurement) |
| RES-7 | Diagnostics | Medium | Backend start and restart reasons go to `eprintln!`, which release builds discard; the window cannot say why the backend is down |
| RES-8 | Process | Medium | A crash that is not a Rust panic (stack overflow, access violation) leaves no marker, no count toward safe mode and no relaunch |
| RES-9 | Backend | Medium | About 60 `unwrap`/`expect` sites sit on request paths; each one is a full backend restart (see RES-2) |
| RES-10 | Plugins | Low | The plugin worker's automatic restart is built and tested but production always passes zero restarts |
| RES-11 | Threads | Low | The backend log thread and the sleep/lock notifier are not supervised; their failures are silent |

Suggested order: RES-7 first (small, and it makes every later failure
diagnosable), then RES-1, RES-2 with RES-9, RES-5, RES-4, RES-3, RES-6,
RES-8. RES-3 and RES-10 change behavior the owner has rules about, so they
need a decision before code.

## What is already solid

Read and found sound; no finding:

- **Audio thread** (`crates/control/src/audio_thread.rs`): its own MMCSS
  thread, `try_lock` only, catches a pump panic, tolerates a poisoned lock,
  parks errors for the control thread, and a pump deadline stops audio
  through an owner whose lease was not renewed.
- **Backend supervisor** (`src-tauri/src/backend_supervisor.rs`,
  `start_owned_backend`): catches panics, bounded backoff, three failures in
  ten minutes lead to a durable safe mode, the pipe is released before a
  restart.
- **Pipe server** (`crates/transport`): four instances, a 5 s deadline for a
  stalled client's request, same-user and same-logon checks, frame size
  bounds, an I/O thread survives malformed clients.
- **Storage**: WAL, busy timeout, foreign keys, integrity check on first
  open and before a migration, newer-schema refusal, atomic backup
  publication.
- **Plugin workers**: separate processes in kill-on-close job objects,
  heartbeat, quarantine, 30 s control deadline.
- **Local HTTP API and OBS client**: read, write and connect timeouts and
  bounded accept everywhere they were checked.
- **UI**: error boundaries per area plus a root recovery panel that still
  offers privacy mute, global `error` and `unhandledrejection` handlers,
  bounded transports with pending limits, a snapshot cache that keeps the
  last good state, an event cursor with a backend epoch, draft recovery,
  guarded `localStorage`.

---

## RES-1 — No backups and no restore in the desktop app

- **Severity**: High (data loss). **Requirements**: STATE-02, STATE-07,
  STATE-12.
- **Where**: `Storage::backup_database`, `prune_recovery_backups` and
  `restore_backup` in `crates/storage/src/lib.rs` are called only from
  `crates/cli/src/lib.rs` (lines 336 to 369). Nothing in `crates/control`
  or `src-tauri` calls them. `Storage::migrate` (line 1391) does not take a
  backup; the name `audiorouter-pre-migration-*` appears only in the
  retention rule and its test.
- **What is wrong**: the specification promises ten daily backups, a
  pre-migration backup and an offer to restore a validated backup on
  corruption. A user who never runs the CLI has none of these. The database
  also holds imported audio (up to 512 MiB), so it is the only copy of the
  Soundboard and Audio File library.
- **Failure scenario**: a power cut or disk fault corrupts the database.
  `Storage::open_checked` fails in `start_owned_backend`. The supervisor
  retries three times, sets safe mode, fails a fourth time, then
  `recovery_safe_mode` returns `None` and the backend thread exits
  (`main.rs` lines 1870 to 1876). The window shows "Unable to load your
  saved session" with a Reconnect button that can never work. Every route,
  preset and sound is gone, and the app gives no next step.
- **Fix**:
  1. On backend start, after a successful open, write one backup per day
     beside the database and prune to ten (the storage functions exist).
     Run it on a helper thread, since the control thread also serves audio
     requests.
  2. Before `migrate` changes the schema, write
     `audiorouter-pre-migration-<version>-<date>.sqlite`.
  3. When open fails with `CorruptDatabase`, keep the damaged file under a
     new name, and let the startup card list the backups with "Restore this
     one" (a shell command, since there is no backend to ask).
- **Regression tests**: a control-level test that a second start on a new
  day leaves two backups and the eleventh prunes the first; a migration
  test that the pre-migration copy opens at the old schema version; a shell
  test that a corrupt file produces the restore choice, not an exited
  thread.

## RES-2 — One handler panic stops all audio, and nothing resumes it

- **Severity**: High. **Requirement**: STATE-10 ("restore previously
  running non-recording routes after one recoverable crash").
- **Where**: `serve_control_connections_forever_observed`
  (`crates/transport/src/lib.rs` line 1792) wraps the whole serve loop in
  one `catch_unwind`. The header of `backend_supervisor.rs` states the
  current policy: "Reconstructing the control plane intentionally does not
  start sessions or reopen native endpoints". No code reads which sessions
  were running before the failure.
- **What is wrong**: the control plane owns the audio workers. A panic in
  any handler, including a read-only one or an assistant's task method,
  unwinds through the plane, drops every worker and stops every route. The
  supervisor then builds a fresh plane with everything stopped. The user
  hears silence in the middle of a call and has to notice and press Play.
- **Failure scenario**: an MCP assistant calls an insert-node method on an
  unusual route and reaches one of the `unwrap()` calls in
  `crates/control/src/simple.rs` (lines 337 to 435). The Discord route goes
  silent.
- **Fix**, in two independent parts:
  1. **Contain** panics per request for methods that cannot leave the plane
     half-changed: wrap the dispatch of read-only methods (the catalog
     already classifies side effects) in `catch_unwind` and answer with an
     internal error. Keep the present restart for mutating methods, where
     the state may be inconsistent.
  2. **Restore** after one failure: record the running session IDs (they
     are already persisted for `sessions.start` idempotency), and after the
     first restart in the window start the non-recording ones again through
     the normal `session.start` path, which revalidates endpoints. Keep
     privacy mute. Stay stopped on the second failure and in safe mode.
     Recorders never resume (STATE-10).
- **Regression tests**: a transport test with a handler that panics on a
  read method while a worker runs, asserting the worker still pumps; a
  shell-level test that a forced panic with a running route leads to a
  running route after the restart and to stopped routes after the third.

## RES-3 — Device loss and sleep stop the route for good

- **Severity**: High for daily use. **Requirements**: CAP-11 ("reconnection
  uses bounded backoff and immediate endpoint events"), CAP-12, STATE-11
  ("on resume re-enumerate and revalidate before continuing"), UC-06.
- **Where**: `invalidate_changed_native_endpoint_worker`
  (`crates/control/src/native_dispatch.rs` line 1364) stops the whole
  multi-input worker when any bound endpoint changes.
  `plan_os_transition` (`crates/control/src/os_transition.rs`) excludes
  every native session from resume, which today is every session. The UI
  message in `ui/src/useWorkspaceEvents.ts` is "Reconnect it, then press
  Stop and Play."
- **What is wrong**: failing closed is right, and so is never substituting a
  device. But the same device coming back is not a substitution, and the
  specification asks for a bounded automatic reconnect. Today:
  - unplugging headphones also silences the microphone path to Discord in
    the same route, because one worker carries all paths;
  - a USB device that re-enumerates for one second ends the route;
  - every sleep ends the route, including with "start at sign-in" and
    autoplay on, because autoplay runs once at launch.
- **Fix**: keep the stop, add a bounded recovery owned by the backend. While
  a session is marked "interrupted", watch endpoint notifications; when
  every bound endpoint ID is active again, run the normal start path. Limit
  it (for example five attempts over two minutes, then give up with the
  present message), cancel it on any user Stop, and never start a route
  whose privacy mute or recorder state would change. Use the same mechanism
  for Resume. A later step is to stop only the affected path (CAP-13).
- **Decision needed**: the AGENTS.md rule says a failed protected voice path
  stays silent "until deliberate recovery". The owner should confirm that
  the *same* endpoint returning counts as recovery the app may do alone.
  Note the validated lesson of 2026-10-12: a driver update changes endpoint
  IDs, so that case must still end with the present message.
- **Regression tests**: the simulated-clock suite
  (`drift_sim_tests.rs`) style, with an endpoint that disappears and
  returns; a test that a different endpoint with the same name does not
  restart the route; a sleep and resume sequence through `system.osTransition`.

## RES-4 — No response deadline on the pipe client; main-thread calls freeze the shell

- **Severity**: Medium.
- **Where**: `round_trip` (`crates/transport/src/lib.rs` line 1493) connects
  with bounded waits, then calls `read_frame`, a blocking `ReadFile` with no
  deadline. Callers on the Tauri main thread: the tray menu handler and the
  tray right-click refresh (`main.rs` lines 2378 to 2447,
  `refresh_tray_status`), `quit_app` (line 781, a plain `#[tauri::command]`,
  which Tauri runs on the main thread, unlike `rpc_request`, which is marked
  `async`), and the initial tray state in `setup` (line 2362).
- **What is wrong**: the control thread handles one request at a time. While
  it loads a plugin (up to 30 s, `WORKER_CONTROL_TIMEOUT`) or finalizes a
  recorder (up to 10 s), a right-click on the tray icon blocks the shell's
  message loop for that long: the tray menu does not open and Windows marks
  the window "Not responding". If the control thread ever hangs, the shell
  hangs with it and Quit cannot work, so the only way out is Task Manager.
- **Fix**: give `round_trip` a response deadline (the server already uses
  `PeekNamedPipe` polling in `read_exact_by`; reuse it on the client) with a
  per-call value: about 2 s for status reads, the documented bound for
  `system.quit`. Move the tray actions and `quit_app` off the main thread
  (`tauri::command(async)` or a worker thread that posts the result back).
  When the deadline passes on Quit, offer "Quit anyway".
- **Regression test**: a pipe server that accepts and never answers;
  `round_trip` must return an error at the deadline. The existing
  `native_pipe_drops_a_stalled_client_at_the_request_deadline` is the model.

## RES-5 — The window does not reconnect by itself

- **Severity**: Medium.
- **Where**: `useWorkspaceEvents.ts`, the `poll` function. A failed
  `events.subscribe` sets `stale: true` (lines 160 to 166). A later
  successful poll with no events and no resync only moves the cursor
  (line 158) and leaves `stale` set. `SnapshotCache.refresh`
  (`ui/src/backend.ts` line 500) tries three times with 100 and 200 ms
  pauses, then stops; the startup card then waits for a click on Reconnect.
- **What is wrong**:
  - One slow poll is enough. During a 30 s plugin load, `events.subscribe`
    times out at 5 s; the header shows "Status unavailable", Play and saving
    are disabled, and they stay disabled after the backend answers again,
    until the user finds the Reconnect button.
  - At startup the backend runs a full `PRAGMA integrity_check` before the
    pipe exists. With a large sound library that can outlast the three quick
    attempts, so a healthy start shows "Unable to load your saved session".
- **Fix**: when a poll succeeds while the state is stale, refresh the
  snapshot and clear the flag. At startup keep retrying with a capped
  backoff (for example up to 30 s) while showing "Starting…", and show the
  error only after that. Keep the manual button.
- **Regression tests**: a vitest case for the hook: fail one subscribe, then
  succeed with no events, and expect `stale` false; a startup case where the
  backend rejects for two seconds and then answers, with no error text shown
  in between (and, per UI-17, no layout change).

## RES-6 — The 5-second UI timeout is shorter than work the backend allows

- **Severity**: Medium. **Confidence**: needs measurement.
- **Where**: `requestTimeoutMs` in `ui/src/host.ts` gives 60 s only to
  `plugins.*` and `*.prepare`; everything else gets 5 s.
- **What is wrong**: several other methods are allowed to take longer by
  design: `session.stop` and `recorders.stop` (recorder finalization, 10 s
  in `threaded_recorder.rs`), `sessions.importFile`, `sessions.exportFile`
  and `sessions.importCommit` (bundles up to 250 MiB),
  `audioMedia.finishUpload`, `system.checkSetup`,
  `virtualDevices.provision`. When the UI gives up first it reports a
  failure for an operation that then succeeds, and the next action runs
  against a state the UI does not expect. The idempotency keys make a retry
  safe, but the user sees an error.
- **Fix**: move the timeout class into the method catalog (`API_METHODS`
  already carries a side-effect class) and generate the UI table from it, so
  a new slow method cannot be missed. On a timeout of a mutating call, show
  "still working" and poll the operation instead of reporting failure.
- **First step**: turn on verbose logging, run each listed method on a
  large input and read the durations in `backend.jsonl`. Drop the finding
  for any method that stays under a second.

## RES-7 — The backend's reasons are written where nobody can read them

- **Severity**: Medium (it multiplies the cost of every other failure).
- **Where**: 16 `eprintln!` calls in `src-tauri/src/main.rs`, among them
  every supervisor decision (lines 1872 to 1903) and "AudioRouter backend
  unavailable" (line 2223). The file starts with
  `windows_subsystem = "windows"` for release builds, so there is no
  console. `database_open_error` builds a clear message for a newer schema
  ("Update AudioRouter to open it") that only ever reaches `eprintln!`.
- **What is wrong**: in a release build, "restarted after a panic",
  "entered safe mode", "database is from a newer version", "enrollment is
  revoked" and "could not create the data folder" all look the same from
  the window: a pipe timeout. `shell.jsonl` gets the panic location and a
  storage error category, but not the supervisor's decision.
- **Fix**: replace these `eprintln!` calls with a bounded `shell.jsonl`
  record (state, error category, failure count). Keep the last backend
  state in shell memory and expose it through a small command
  (`backend_state`: starting, running, restarting, safe mode, stopped with a
  reason code). The startup card and the Reconnect message then say what
  happened and what to do.
- **Regression test**: a shell test that a `NewerSchema` open error yields
  the state "stopped: newerSchema" and the update text.

## RES-8 — Crashes that are not panics leave no trace

- **Severity**: Medium.
- **Where**: `install_panic_log` and the supervisor see only Rust panics.
  There is no process-level marker, no `RegisterApplicationRestart`, and no
  crash-dump configuration (searched `src-tauri` and `crates`).
- **What is wrong**: a stack overflow (one already happened in the hosted
  E2E run, see `plugin-reliability.md`), an access violation in the Windows
  interop, or the process being killed ends the app at once. The next launch
  does not know the last run crashed: `recovery_crashes` is not incremented,
  so three such crashes in a row never reach safe mode, and a route that
  crashes the app on Play can loop when autoplay is on.
- **Fix**: write a "running" marker beside the database at start and remove
  it on a clean exit. At launch, a marker that is still there counts as one
  recovery crash (so autoplay is skipped and safe mode applies after three)
  and is logged. Optionally register for Windows restart and document how to
  enable local crash dumps for a support bundle.
- **Regression test**: `fresh_install_shell` style: start, kill the process,
  start again, and expect one recorded crash and no autoplay.

## RES-9 — Panic sites on request paths

- **Severity**: Medium because of RES-2; Low once RES-2 is done.
- **Where** (non-test code, counted by a scan that stops at the first
  `#[cfg(test)]`): `crates/control/src/native_bindings.rs` 12 `expect`,
  `simple.rs` 5 `unwrap` and 4 `expect`, `status.rs` 4, `native_paths.rs` 4,
  `native_dispatch.rs` 3 plus an `unreachable!`, `graph.rs` line 472,
  `recording.rs` 2, `plugins.rs` 2, and 19 identical `expect("native bridge
  session remains owned until close")` in `crates/windows-audio/src/lib.rs`
  (lines 1434 to 1659).
- **What is wrong**: each states an invariant that holds today. Each is also
  one refactor away from stopping the user's audio, and `status.rs` is on a
  path polled every second.
- **Fix**: return `ControlError` (an internal-error variant with the
  invariant's name) instead. Then turn on `clippy::unwrap_used` and
  `clippy::expect_used` for non-test code in `crates/control` and
  `crates/transport`, so new ones need an explicit `allow` with a reason.
  This follows the project's own rule of preferring a generated check to a
  hand-kept list.

## RES-10 — Plugin worker restart is built but switched off

- **Severity**: Low (a decision, not a defect).
- **Where**: `SupervisedBusWorkerLoop::spawn` and `spawn_with_parameters`
  (`crates/plugin-host/src/lib.rs` lines 1789 and 1800) pass
  `max_restarts = 0`; no production caller uses
  `spawn_with_restart_policy`. `plugin-reliability.md` notes "no restart on
  this bridge".
- **What it means**: a plugin that crashes once stays silent until Stop and
  Play. That matches the microphone privacy rule for voice paths. For a
  music or game path, one automatic restart with the saved state would be
  safe and is already implemented and bounded (at most two).
- **Decision needed**: allow one restart on paths that carry no microphone
  or captured-application audio, or keep the present rule everywhere.

## RES-11 — Unsupervised helper threads

- **Severity**: Low.
- **Backend log thread**: `backend_log_sink` (`crates/transport/src/lib.rs`
  line 165) stores the sender in a `OnceLock`. If the
  `audiorouter-backend-log` thread ever panics, every later record is
  dropped for the life of the process, with no sign, and `backend.jsonl` is
  the first file the lessons in AGENTS.md say to read. Fix: catch a panic
  per record inside the thread's loop.
- **Sleep and lock notifier**: the `audiorouter-os-transition-rpc` thread
  (`main.rs` line 2295) discards the result of `system.osTransition`. If the
  backend is busy or restarting when Windows announces sleep, the streams
  are not released and nothing is logged. Fix: retry once after a short
  pause and log a failure to `shell.jsonl`.

---

## Not covered

- `crates/windows-audio` device code beyond panic sites, `crates/engine`,
  `crates/dsp`, `crates/recording` internals, the driver and the helper.
- The MCP tunnel process and the CLI.
- UI canvas, editors and save flow beyond connection handling (the bug audit
  covered autosave).
- Nothing here was measured. RES-6 in particular needs timings before work
  starts, and RES-4's "Not responding" claim should be reproduced once with
  a slow fixture plugin before the fix is designed.

## Next action

Owner chooses which findings to repair and answers the two decisions
(RES-3: may the app restart a route when the same device returns; RES-10:
may a non-voice plugin restart once). Each repair then follows the rules in
the bug audit: failing test first, one finding per commit.
