# Resilience implementation — 2026-11-13

## Objective and authority

Owner authorized implementation of every high and medium audit finding,
RES-1 through RES-9, automated regression coverage, and a release version
bump from 0.0.29 to 0.0.30. No PR is authorized. Low findings RES-10/11
remain outside this task.

Requirements: STATE-02/07/08/09/10/11/12, CAP-02/11/12, API-08,
UI-17, SEC-02 and ENG-01/02/04/05; M07 recovery hardening.

## Decisions and prerequisites

- Same exact endpoint returning after device loss or sleep may recover
  automatically under the owner's instruction to execute RES-3. This does
  not authorize restarting a failed processor on a protected voice path,
  replacing endpoints, clearing privacy mute, or resuming recordings.
- Preserve the uncommitted audit/current-plan edits and untracked build
  output. Work is on `fix/resilience` from main.
- Windows is available. Hardware unplug/sleep and large-input timings need
  isolated fixtures; tests must not change the owner's running routes or data.
- Runtime observations are required before confirming RES-6's suspected
  methods. Portable tests do not establish native audio recovery.

## Ordered work

1. Verify audit sources, linked M07 specifications and recovery decisions.
2. RES-5: recover stale subscriptions and tolerate slow backend startup.
3. RES-7/8: durable shell backend state and abnormal-exit evidence.
4. RES-1: daily/pre-migration backups and explicit corruption restoration.
5. RES-2/9: contain safe request panics, remove request panic sites, recover
   eligible routes once while retaining privacy and recording boundaries.
6. RES-4: bounded native pipe response and asynchronous shell callers.
7. RES-3: bounded exact-endpoint recovery, cancellation and OS transitions.
8. RES-6: measure slow operations, align catalog/adapter deadlines and
   reconcile uncertain mutation outcomes without duplicate operations.
9. Bump all application version metadata, regenerate contracts/catalogs as
   needed, inspect complete diff and run relevant checks/runtime exercises.

## Validation matrix and evidence

Each defect needs a regression that fails before the repair. Exercise the
dispatcher and default MCP for API changes; run contract drift and generated
documentation checks. Test storage using disposable files, transport with a
stalled Windows pipe, and recovery using deterministic clocks and exact IDs.
UI requires typecheck, format, lint, unit tests and dark/light/high-contrast
visual evidence for changed presentation. Rust requires both formatting
checks and workspace/shell Clippy. Run the built isolated shell before handoff.
Record commands, results and limitations here as work progresses.

## Risks and rollback

Automatic recovery can restart capture unexpectedly if eligibility is too
broad; persist mute, prohibit recorder/processor-failure recovery, revalidate
exact bindings and cancel on deliberate Stop. Backups require disk space and
must never overwrite the damaged original. Deadlines do not cancel committed
mutations. Rollback is a scoped revert; preserve backups and user databases.

## Progress

- Initial source review confirms RES-5: an empty successful event poll never
  refreshes stale state; startup snapshot retries end after 300 ms of delays.
- RES-5 initial repair: the startup snapshot call now opts into capped
  asynchronous retries for 30 seconds; ordinary refreshes retain their short
  retries. A failed subscription forces a snapshot refresh on the next
  successful poll, including an empty event batch.
- Windows / Node / Vitest: startup regression failed before repair with
  `pipe is starting` visible at 1.9 seconds. After repair, backend and hook
  suites pass 74 tests; UI typecheck passes. Hook regression covers failure
  followed by an empty successful event batch. Initial hook test lacked its
  jsdom annotation; corrected the test environment. Lint found an unused
  initial assignment, removed before rechecking.
- Startup expiry at 30 seconds and superseding-refresh cancellation are
  covered with fake clocks. Lint and docs validation pass (266 files,
  1,488 links); `git diff --check` passes. Changed UI files formatted using
  the installed Prettier from the UI directory. A first npm exec invocation
  from the repo root resolved paths incorrectly and changed no files.
- No native audio, built-shell or three-theme visual verification yet.

### Shell, storage and transport progress

- RES-7: added shell-owned typed backend phase/reason/failure count and
  `backend_state` command. State transitions use the bounded privacy-safe
  shell log. Failed window RPCs expose actionable stopped-state messages.
  Newer-schema errors request an update; corruption requests backup recovery;
  revoked enrollment is distinguished. Remaining: full startup UI use and
  built-shell failure exercises; remove redundant console-only reporting.
- RES-8: per-database `.running` marker is synced before backend startup,
  checked only after claiming the desktop instance, and removed only on the
  completed Tauri Exit event. A prior marker records a durable recovery crash
  and disables autoplay on this launch. Unit coverage models an interrupted
  run and explicit clean exit. Remaining: kill/relaunch of an isolated built
  process and autoplay/safe-mode assertions.
- RES-1: storage now writes a pre-migration snapshot before changing a
  nonempty older database. Desktop backup helper checks each minute and
  creates one consistent backup per UTC day, retaining ten in the database's
  own `.backups` directory. New deterministic retention/idempotency and old
  schema tests pass. Remaining: validated backup listing, explicit startup
  restore, damaged-database preservation and real shell recovery exercise.
- Daily-backup regression exposed orphaned staging WAL/SHM files in existing
  `write_backup`. Fixed by converting the private staging database to DELETE
  journal mode before integrity validation and atomic publication. The
  retention test now asserts exactly ten self-contained files.
- RES-4: native response reads now support a bounded timeout, checked even
  while bytes arrive. Shell status/events use 2 s, Quit 15 s, other methods
  60 s. Tray actions, initial status and right-click refresh use helper
  threads; window Quit is an asynchronous Tauri command. Windows pipe test
  with a delayed server passes (100 ms response deadline). Remaining: Quit
  anyway flow, slow-plugin shell responsiveness exercise, and policy tests.
- Version metadata updated to 0.0.30 in all application Rust/npm manifests,
  Tauri config, and root/shell/probe/npm lockfiles. Cargo check workspace and
  shell pass. Lock regeneration changed only local package versions after
  restoring npm's unrelated removal of Linux libc metadata.

Windows verification before version bump: full storage suite 111 pass / 1
explicit ignore; shell suite 104 pass / 4 explicit ignores; focused Windows
pipe deadline test passes. Clippy with all targets/features and `-D warnings`
passes for storage/transport and shell. Both Rust formatters were applied;
docs validation passes 266 files / 1,489 links and diff whitespace check
passes. No release artifact, hardware action or production database changed.

Next action: complete RES-1's explicit startup restoration workflow using
validated backups and preserved corrupt originals; then RES-2/9 and RES-3.
RES-6 measurement/deadline catalog and RES-4/5/7/8 runtime/UI verification
remain. No finding is declared fully complete yet. No commits or PR created.

### Explicit restoration and request containment

- RES-1 startup workflow implemented: `recovery_backups` and
  `restore_recovery_backup` are shell-only commands because the backend cannot
  serve an API while its database is corrupt. They require the owned backend
  to have terminated in the corrupt-database state; external-pipe clients and
  running backends cannot restore. A shell mutex serializes restoration.
- Recovery accepts only direct named backups in this database's backup
  directory. Read-only size/integrity/application-schema checks reject corrupt,
  foreign, newer-schema and reparse-point candidates without altering them.
  A separate validated copy is sealed with mute on, startup off, no device
  consent and no restored client grants. Original database/WAL/SHM files move
  into a newly reserved `damaged-<pid>-<time>` archive before publication;
  a failed move/publication attempts rollback and retains evidence.
- Added startup diagnostic/recovery component with reserved layout space,
  app-styled select, disabled controls until choices arrive and explicit
  two-click restoration. Success reconnects the UI; routes stay stopped.
  Added the database recovery runbook and documentation-map link.
- Windows shell restoration tests pass: owner's route name survives, original
  damaged bytes survive, restored dispatcher status has zero active routes
  and privacy mute on, grants/capture consent/startup are reset, invalid names
  and invalid backups never move original data. A storage regression proves
  that inspecting an oversized backup does not delete it. UI focused suite
  passes 77 tests; typecheck and lint pass. Visual and real shell-command
  restore exercises remain required.
- RES-2 containment now lives in `ControlPlane::dispatch` so REST, CLI, MCP and
  shell share it. Only catalogued ReadOnly handlers are contained; mutations
  still unwind to supervision. Injected-panic dispatcher tests prove the
  running runtime/generation survives a read failure, authorization scope is
  restored and a subsequent status succeeds; mutation panic still propagates.
  Automatic post-crash route restoration remains unimplemented.
- RES-9: introduced a named `ControlError::Internal` and replaced fallible
  request-path unwrap/expect sites across simple tasks, status, native binding/
  preparation, graph, sessions, recorder/library, plugins and virtual devices.
  Deny unwrap/expect Clippy rules now cover production control and transport.
  Three documented local exceptions remain: fixed bridge-constructor constants
  and the two lifetime-protected audio-worker access helpers. Existing
  `only_the_owning_handle_takes_the_worker_out` covers that ownership boundary.
  The Windows bridge controller's Option accessors still need structural work.
- Full control library suite passes 393 tests / 13 explicit ignores. Control,
  transport and shell Clippy all-target/all-feature `-D warnings` checks pass.
  First test compile exposed the second ControlPlane initializer; its test
  injection field was added before proceeding. No warning has been waived
  without a code-local reason.

### Remaining recovery and timeouts

- RES-2: the backend now preserves a bounded witness of playing, non-recording
  sessions and the shell may restore it after the first eligible restart. The
  existing `session.start` path revalidates devices; restoration never clears
  privacy mute, resumes a recorder, or runs in safe mode. Shell policy and
  portable control tests pass. An attended live-route crash/restart test is not
  run here.
- RES-3: device and sleep recovery uses bounded retries and exact saved endpoint
  identities, cancels on deliberate Stop, and refuses recording routes. Fake
  clock/control tests cover endpoint return, replacement identity, exhaustion,
  flapping and cancellation. Native unplug/replug and actual suspend/resume were
  not performed; those Windows audio gates remain open.
- RES-6: `METHOD_DEADLINES` in the generated contract catalog classifies every
  discovered method and the UI uses a 60-second deadline for slow/state-changing
  methods. Contract drift passes. No production duration measurements on large
  imports or plugin/recorder fixtures were made, and this change does not add an
  operation-status poll for a mutation whose reply is still lost after its
  deadline. That reconciliation remains follow-up work if measurements show a
  real operation can exceed the bound.
- RES-9: control, transport and `windows-audio` request/native paths were
  converted to typed errors or ownership-safe access, with strict Clippy checks
  enabled for control/transport and targeted worker regressions. The remaining
  bridge accessors have documented ownership invariants; the workspace's
  all-target Clippy gate passes.

### Final Windows verification — 2026-11-14

- UI: `npm test` — 119 files, 1,067 tests passed; `npm run format:check`,
  `npm run lint`, `npm run typecheck:all`, and `npm run build` passed. The
  production UI build retains the existing 629.93 kB main-chunk advisory.
  Focused startup/recovery visual and built-shell recovery exercised dark,
  light and high-contrast themes.
- Rust: `cargo test --workspace --locked` passed; `cargo test --manifest-path
  src-tauri/Cargo.toml --locked` passed 109 tests / 4 explicit ignores. Both
  strict Clippy commands (`cargo clippy --workspace --all-targets
  --all-features -- -D warnings` and the same command with the shell manifest)
  pass. Both Rust formatting `--check` commands pass.
- Contracts/docs: `npm --prefix contracts run check:drift` passes (148 methods,
  37 node kinds, 20 processors, 24 event categories); docs acceptance passes
  (267 Markdown files / 1,491 links); `git diff --check` passes.
- Isolated release shell (0.0.30, custom protocol; disposable database and
  WebView profile): hard kill left the synced `.running` marker; relaunch logged
  `shell.previousProcessInterrupted` with `state: interrupted`, and no session
  start was observed. The database was then corrupted in the fixture: all three
  themes offered the validated backup; confirming restore with the final build
  preserved the damaged database under a new archive and returned to the
  workspace. The final restore log had zero backend errors. Recovery
  instructions keep routes stopped/muted and clients/device consent reset.
  No owner database, active route, device, driver or recording was changed.
- `cargo test --workspace --locked` was first interrupted at the 120-second
  command limit while the long ordered-processor test was still running; the
  same command was rerun with a ten-minute limit and completed successfully.

## Next action and gates

Source changes and automated local software checks for RES-1 through RES-9 are
implemented. RES-6's expanded contract-driven deadlines are in place, but
production timings for large imports/exports and explicit status reconciliation
after a mutation reply exceeds the 60-second bound remain unverified; qualify
that piece with representative measurements before calling RES-6 fully closed.
No commit or PR is authorized. Remaining release/qualification gates are live
native endpoint unplug/reconnect and sleep/resume (RES-3), attended recovery
through the owner's normal route, the RES-6 measurement/reconciliation check,
and fresh hosted CI. Do not claim hardware/audio qualification from these
portable and isolated-shell results. RES-10/11 remain outside the authorized
scope.
