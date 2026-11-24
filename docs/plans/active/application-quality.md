# Application quality and regression prevention

Status: implementation authorized by the owner on 2026-10-21 ("Let's do it!").
Progress at 2026-10-25 (main `11737991`, delivery PR #73):

- Q1 is merged (PR #50): the change-to-check classifier, the shared
  checks, the required Quality gate, main ruleset 25451476, and the
  declared-skip run 39713611409.
- Q2's history and presentation-persistence slices are merged. They include
  bounded generated state/persistence cases and mutation checks for critical
  lock, session, storage and redo guards (PRs #52/#53 and stacked follow-ups).
- Q3.1's optimized production-browser smoke and readiness probe are merged.
  The packaged WebView2 smoke (Q3.2) is still open and is the next task.
- Q3.2's packaged runner passes locally on `11737991`: usable canvas,
  normal native bridge, stopped/no-grant state, session save/restart/reload and
  child cleanup. The earlier blank-page reports were caused by the restricted
  launch environment and a one-shot probe that perturbed normal UI refresh;
  see the [investigation evidence](evidence/2026-10-25-packaged-webview-smoke.md).
  The [11-case local fault/cancellation qualification](evidence/2026-10-25-packaged-smoke-guard-qualification.md)
  passes; current hosted Windows CI remains open.
- One Q4 slice, the graph-commit storage-failure recovery test (PR #56), is
  merged.
- 2026-10-23: Edge CI keeps one diagnostic retry, but `failOnFlakyTests`
  now fails the job on any flaky test. PR #65 had let a retried pass count
  as green, contrary to the delivery rules. Proven with a probe that fails
  on its first attempt only: exit 0 without the option, exit 1 with it.
- 2026-10-25, PR #73 CI repair: the full suite now builds all four browser
  entries before running the same 353 discovered scenarios. The original trace
  showed `ERR_NO_BUFFER_SPACE` loading the dev entry, before React mounted.
  Local bundled run: 343 passed / 10 existing skips / zero flaky retries; UI
  unit tests 743 passed. Current hosted PR and packaged qualification remain
  open; see the [CI repair evidence](evidence/2026-10-25-pr73-ci-repair.md).
- Hosted follow-up: the repaired Edge job passed all 343 non-skipped scenarios.
  Nightly exposed an OBS test Instant-underflow assumption and elevated
  WebView2 ignoring profile/debugger overrides. The follow-up adds forward-clock
  expiry boundaries and a verified standard-user tool launcher. Local OBS/control
  tests, both strict Clippy runs and four launcher lifecycle probes pass; wrapped
  packaged qualification is blocked locally by an existing app and requires CI.

Hosted Q3.2 evidence now passes on source 512bc0e3: all 11 qualification cases,
normal packaged UI/native/save/restart/cleanup, standard-user token, coverage
and manual synthetic soak (run 40241297374). Final documentation-head checks
and owner PR review/merge remain the delivery action; exact results and limits
are in the [CI repair evidence](evidence/2026-10-25-pr73-ci-repair.md#passing-packaged-evidence-on-512bc0e3).

Later browser counterexample: fa15bcc5's bundled canvas chunk also failed with
ERR_NO_BUFFER_SPACE before its drag scenario. The CI fixture now delivers exact
compiled HTML/assets directly through Playwright, preserves actual HTTP API/Rust
RPC traffic and fails on missing files. Offline-positive/missing-file-negative
qualification and the full local 343-pass replay succeed; current-head CI remains
required. The fa15bcc5 nightly passed its default 20-minute soak and all packaged
gates. See the [transport evidence](evidence/2026-10-25-pr73-ci-repair.md#compiled-fixture-transport-counterexample-and-repair).

**Q3.2 complete (2026-11-23).** The packaged smoke and its 11-case guard
qualification run in the nightly quality workflow after the exact release
build and pass on hosted Windows every night. Run 45017043920 (source
`59ce477e8`, Windows Server 2025 build 26100, WebView2 157.0.4341.46,
standard-user token, Medium integrity): passed, shell SHA-256
`86945386…ddee6`, native bridge on both launches with no device grant,
save/restart, graceful quit of both children, zero uncaught UI errors. UI
ready 1,203 ms on a fresh profile and 1,867 ms after restart (NFR-14 targets
8 s / 3 s, reported, not gated until Q5 calibrates runner variance). The
2026-11-22 nightly failed only in its coverage job (below), not in Q3.2.

**Q5.3 triage of main's red CI (2026-11-23).** After #178 and #180, `main`
CI and the 2026-11-22 nightly failed on three tests. The last 60 CI runs
were read; each failure is classified from its retained log:

| Test | Seen | Class | Disposition |
| --- | --- | --- | --- |
| `GlobalKeybindsPanel` "bulk delete … failed save retains every binding and selection" | 4 of 60 CI runs + 1 nightly | product | Branch `fix/keybind-selection-after-failed-delete`: a late pruning effect cleared the selection a failed delete had restored. Deterministic regression added. |
| shell `chosen_save_destination_is_approved_and_exported_through_the_real_pipe` | 1 (run 45122522886) | product (Q4.2) | Branch `fix/pipe-guard-claim-race`: a client could take the backend's guard pipe instance before the server claimed it, so the server never started. Transport regression added. |
| `LogFilesPanel` "switches verbose logging and counts down the hour" | 1 (nightly 45017043920 coverage, 5 s timeout) | unknown | Not reproduced: the file passed 5 times alone with coverage and in a full local coverage run (1,166 tests). Not changed on a guess; watch the next nightlies. |

The other recent failures were on branches and were fixed before merge:
CLI parity and `cargo-deny`. The Dependabot xyflow 13 PR (#178) failed its
first run (release build and five App tests). Its four later runs passed
before the owner merged it by hand (auto-merge skipped it), so none of these
failures is still open.

Open for delivery: the two repairs above, consolidated into PR #181 with
owner approval on 2026-11-23; Q4
items 1–7 beyond the storage and guard-pipe slices; Q5; Q6 on each release.
Hardware and release evidence remain separate gates. Existing acceptance
targets remain binding.

## Objective

Catch regressions before merge and before a user receives a build. No process
can guarantee zero bugs; success means reproducible prevention of known failure
classes, visible failures on the exact proposed commit, and reliable recovery.

Requirements: ENG-01 through ENG-05, QUAL-01 through QUAL-06,
UI-02/11/16/17, and the applicable NFR requirements in
[quality](../../spec/14-quality.md). M05 owns editor behavior; M07 owns recovery;
[M08](../../milestones/M08-release.md) remains the release gate.

## Evidence and current coverage

This is a repository inspection at 6d9890e0, not a new CI or hardware result.
The group feature's [review record](https://github.com/MrDesjardins/audiorouter/blob/6d9890e0/docs/plans/active/current.md)
records its local Windows run: 578 UI tests and 41 focused Edge scenarios passed.
That feature remains separate from the Q1 tooling PR. Test count alone did not
prevent the preceding defects.

Existing protections to retain:

- [CI](../../../.github/workflows/ci.yml) already runs Rust formatting, strict
  Clippy, workspace/shell tests, contract drift, UI format/lint/unit/build,
  dependency checks and the full Edge suite for applicable changed areas.
  Main pushes and manual runs classify all areas for verification.
- [Nightly quality](../../../.github/workflows/quality.yml) already collects
  coverage, runs a default 20-minute synthetic engine soak, and launches the
  release shell with fresh-install backend startup measurement. Coverage is
  reported, not gated. The startup measurement does not establish UI readiness.
- [Production canvas testing](../../../ui/playwright.production.config.ts)
  exists, but its explicit scenario list omits the newer group, clipboard and
  layout-history suites. The reviewed CI files do not invoke this command.
- [Whole-app soak tooling](../../../tools/quality/run-app-soak.ps1) exists and
  isolates copied profiles/database state. It requires a separate live-device
  run; the synthetic engine soak does not replace it.
- Fresh-install, deterministic DSP, native continuity, recording integrity,
  contract, installer and recovery harnesses already exist. Expand these
  rather than create competing qualification systems.

Recent escaped defects identify the first priorities: session-scoped state,
multiple undo mechanisms, connector edits interacting with geometry history,
malformed persisted preferences, storage failures, and tool-specific CSS
interacting with shared sizing. Several were missed by isolated happy-path tests.
See the dated review sections in the active plan for reproductions and fixes.

The initial planning inspection did not verify branch protection or remote CI.
Q1 has since audited and activated protection after a full successful CI run;
the [active execution record](current.md) links the settings/evidence. Current
coverage percentages still need Q5 baseline evidence.

## Delivery rules

- Deliver Q1-Q6 as small PRs, with one behavior or gate per reviewable change.
  Separate extraction/refactoring from behavior changes; retain regression
  coverage through both. Do not rewrite the application in one quality PR.
- Every bug fix gets a failing-before/passing-after regression at the owning
  layer, unless a documented hardware prerequisite prevents reproduction.
- Each PR names requirement IDs, changed behavior, failure cases, required
  test layers, commands/results, limitations and rollback.
- Compiler, Rust formatter, TypeScript, ESLint and Clippy own mechanical checks.
  Jev/human review focus on state ownership, concurrency, privacy, lifecycle,
  invariants and missing failure paths. A model finding needs a concrete code
  path or reproduction; repeated reviews do not replace executable evidence.
- No blanket console-error suppression, increased timeouts to hide product
  failures, automatic visual-baseline acceptance, or retries presented as
  first-pass success. A diagnostic rerun retains the original failure.
- CI runs against disposable profiles, databases and synthetic media. Hardware
  runs use declared fixtures/endpoints with user-authorized audio activity;
  never change Windows defaults, driver trust settings or live user sessions
  as an incidental part of verification.

## Q1 - Make verification consistent and enforceable

Priority: first. Owner: CI/maintainer. Requirements: ENG-01/02/04/05.

1. Inventory the existing scripts/jobs, their triggers and actual outputs in
   one change-to-check policy. Measure duration and baseline reliability using
   existing runs; distinguish passed, failed, cancelled, skipped and not run.
2. Provide a local verification entry point using the same checked-in commands
   as CI. Have separate fast, full and guarded hardware profiles. Do not call
   the broad safe-all acceptance script a cheap or hardware-equivalent check.
3. Audit the changed-file classifier with fixtures: shared contracts must run
   consumer checks; core backend changes must run browser integration; workflow,
   toolchain and build-configuration changes must trigger the relevant gates.
   Add Stream Deck compile/test coverage for its files and dependency changes.
4. Add one required aggregate result that fails on required job failure,
   cancellation or unexpected skip. Legitimate unaffected areas must be
   explicitly classified. Check the latest head/merge result, not an older SHA.
5. Inspect and, when implementation is authorized, configure main protection:
   PR review, required aggregate result, resolved blocking review threads, and
   checks against the current base/merge result. Use the repository's supported
   merge mechanism; do not assume a paid merge queue is available.
6. Account for the nightly README/history publisher, which currently pushes to
   main. Publish metrics as artifacts or a reviewed docs PR if direct pushes
   conflict with the protection policy; do not give the publisher a bypass.

Acceptance: intentional format, type, test, drift and classifier defects fail
the expected gate; unrelated docs edits are correctly classified; cancelled
required checks cannot appear merge-ready. Record ruleset evidence, run links,
commands, timing and unavailable permissions. Keep existing protection until
its replacement is verified.

## Q2 - Test state transitions and failure boundaries

Priority: highest product regression value; follows Q1 baseline.
Owner: UI/domain maintainer. Requirements: UI-02/11/16/17, ENG-01/02.

1. Write an invariant catalog for layout/session/history state. Make ownership
   explicit: backend graph, local session presentation, transient selection,
   clipboard and history have distinct lifetimes and persistence boundaries.
2. Extract history/command application and preference codecs from large UI
   event handlers into small pure modules. Reuse existing canvasGeometry and
   layout helpers; keep backend graph validation authoritative. A single layout
   command/history path should own drag, resize, alignment, connector-side
   edits and Arrange. UI shortcut buttons must not maintain competing histories.
3. Add deterministic action-sequence tests, initially using the existing test
   stack. Include select -> move -> lock -> undo; Arrange -> connector edit ->
   undo/redo; undo -> new edit; delete/add -> undo; session switch/reload;
   copy/paste across sessions; mixed group/tool selection; and nested overlaps.
4. Assert outcomes after every action, not just the final screen:
   - Locked rectangles never change through movement, sizing or history.
   - A carried descendant moves once, with offsets preserved and ownership
     frozen for the gesture; partial overlap does not become membership.
   - Undo/redo preserve action order; a new edit clears redo; a no-op does not.
   - Session switching cannot apply old geometry, history or preferences.
   - Layout-only actions do not change graph revision, routing or Save state.
   - New/deleted graph objects do not make old history overwrite newer objects.
5. Test every persistence boundary with null, scalars, arrays, malformed JSON,
   invalid fields, legacy records, oversized input and failed reads/writes.
   Validate bounded dimensions/IDs and preserve valid entries where appropriate.
   Add deterministic rejection and round-trip tests for session/media imports.
6. Add bounded generated sequences and property tests for these invariants.
   PRs use fixed seeds; nightly runs rotate seeds and retain the seed, action
   trace and minimized failing sequence as a permanent regression fixture.
7. Use targeted mutation/fault seeding on critical guards: removing a lock,
   session-key check, stale-revision rejection or storage catch must cause a
   test to fail. Review equivalent mutations rather than adding shallow tests
   solely to raise a score.

Acceptance: every listed invariant and previously escaped failure class has
an executable regression. Critical guard mutations are detected. Refactored
code passes existing unit and browser suites without expanding API authority.

## Q3 - Verify the UI users actually run

Priority: next. Owner: UI/shell maintainer. Requirements: UI-11/17,
NFR-08/14/15, ENG-02/05.

1. Run a bounded production-build browser smoke in CI using the existing
   production harness. Include group/layout/history, clipboard, connections,
   session switching, recording setup and backend error recovery. Retain the
   full existing harness scenarios. Following PR #73's measured dev module-load
   failure, shared CI builds all four entries and runs every scenario against
   bundled assets delivered directly through the browser fixture; development and
   focused-preview HTTP commands remain available. Qualify missing-asset detection
   and offline delivery before the full suite. Compare
   discovery before changing profiles, retain all assertions and the flaky
   failure policy, and require a complete first-pass local and hosted run.
2. Add packaged WebView2 smoke on the exact shell artifact produced by the
   build: ready canvas, native bridge, session load/save/reload, stopped first
   launch, readable errors and recovery after backend restart. Verify bundled
   sidecar paths and hashes, without developer-only grants or old build files.
3. Give tests an explicit application-ready condition: UI loaded, backend
   snapshot applied, current session/generation and connector bounds ready.
   Wait for meaningful state rather than arbitrary sleeps or an unrelated
   style string. Test cold and warm readiness separately.
4. Preserve dark, light and high-contrast checks. Add keyboard-only flows,
   reduced motion, 200% scaling and narrow-window checks for changed panels.
   Verify layout stability during telemetry and messages. Manual Narrator
   acceptance remains distinct from DOM assertions.
5. Triage the recurring ResizeObserver notification using traces on the actual
   packaged UI. Fix its cause or document a narrowly reproducible browser-only
   notification with an owner and follow-up. Fail on unexpected page errors;
   expected negative-test messages need explicit bounded assertions.
6. Retain traces/screenshots, backend logs, environment, seed and tested SHA
   for failures. Screenshots remain in test output paths. Review visual changes
   deliberately; do not automatically rewrite or approve evidence baselines.

Acceptance: a broken production import/asset path, missing host capability,
uncaught render error or session/history regression fails the smoke. The
packaged artifact works with a clean profile and its own matching sidecars.
Browser results never claim native audio or installer qualification.

## Q4 - Strengthen backend, audio and recovery regressions

Priority: offline work can proceed after Q1; live evidence needs hardware.
Owner: backend/audio maintainer. Requirements: ENG-01/02, QUAL-01-06,
NFR-04/09/10/11 and existing architectural privacy/realtime constraints.

1. Extend contract tests for success, invalid input, authorization, stale
   revision, retries and partial failure on high-risk operations. Ensure UI,
   CLI, HTTP and MCP exercise the same backend ownership rules.
2. Test fault interleavings: delayed/out-of-order responses, disconnect during
   save, backend restart, lost events and session changes while requests are
   pending. Confirm no partial graph commit, silent draft loss or cross-session
   application. Use fake devices to exercise recovery deterministically.
3. Expand bounded parser/compiler/import fuzzing and keep minimized failures.
   Verify limits, malformed media, channel maps, graph combinations and the
   protected-voice fail-silent policy. Avoid expensive unbounded fuzzing in PRs.
4. Reuse deterministic audio fixtures for continuity, silence, sample fidelity,
   NaN/Inf rejection, clock mismatch, queue bounds, bypass and graph replacement.
   Exercise every affected worker kind and mixer topology; test burst producers,
   delayed consumers, stalls and restarts, not acknowledgment-paced happy paths.
5. Parse every produced recording and verify headers, frame counts and failure
   intervals. Include multi-path fan-out and disk/write failure while unrelated
   routes continue. File existence or size alone is not acceptance evidence.
6. For changes to audio service, routing or native workers, require the existing
   reference-capture continuity harness on a declared Windows endpoint setup.
   Require a clean reference and zero unexplained glitches. Test transition
   ramps against reference waveforms; retain counter and sample evidence.
7. Exercise privacy mute, worker/plugin failure, endpoint removal/return and
   sleep/resume with the existing guarded native harnesses. Record hardware,
   drivers and negotiated formats; do not treat hosted runners as hardware.

Acceptance: injected failures produce the specified backend result/recovery,
recordings remain valid, protected paths fail silent, and changes touching
native audio have the applicable real Windows evidence before release. Missing
hardware evidence is an explicit blocked gate, not a passing mock result.

### Q4.1 execution entry (2026-11-24, contract tests; offline, no hardware)

Scope: item 1 only (invalid input, authorization, stale revision, retries).
Fault interleavings, fuzzing and the audio items stay open. Branch
`test/q4-contract-tests`, not pushed, no PR.

Tests added. One shared generator, `crates/control/src/contract_cases.rs`,
is compiled into three test builds with `#[path]`, so every adapter runs the
same cases against the same fixtures (a database-backed plane, because an
in-memory plane never exercises storage's key limits):

- `crates/control/src/contract_tests.rs` (dispatcher, 13 tests),
  `crates/cli/src/contract_tests.rs` (MCP through `mcp_tool_call`, 8 tests),
  `src-tauri/src/http_contract_tests.rs` (HTTP on a real listener, 5 tests).
- Coverage, walking `API_METHODS` and the published input schemas (no
  hand-kept method list): 156 methods; 1841 generated invalid-input cases per
  adapter (not-an-object, 40-deep nesting, unknown parameter, omitted params,
  and per field: missing, wrong type, 4097-byte string, empty string, over
  `maxLength`); 4 roles (none, Observer, Editor, Operator) x 156 methods x 3
  adapters for authorization, each compared with dispatch (same category and
  text); every default MCP tool checked against its caller's role through the
  backend calls it really makes; the 5 `baseRevision` methods (graph.plan,
  graph.commit, graph.undoPlan, graph.restorePlan, virtualRoutes.replace) for
  `revisionConflict` (HTTP 409); 12 retry scenarios (nodes.add/set/toggle/
  remove, connections.add/remove, sessions.create/duplicate/delete/active.set,
  safety.togglePrivacyMute/setPrivacyMute, toolPresets.save), each run with a
  short and a 128-byte key: a retry returns the same result and applies once,
  a reused key with different input is `idempotencyConflict`, and two clients
  may use the same key. A test fails if an idempotent method has neither a
  scenario nor a recorded reason (`RETRY_ELSEWHERE`).

Defects found and fixed, each with a regression test that failed first:

1. Wrong-typed and over-long string/number/boolean parameters were silently
   ignored by 14 methods. `validate_method_params` now checks scalar types and
   `maxLength` from the published schema (`authorization.rs`).
2. `graph.commit` with an empty key was accepted for a named client (MCP,
   HTTP, CLI) because the key was scoped before it was validated.
3. A 128-byte key (allowed by every schema) was refused for any named client
   once the client id was added to it, and for every task edit (`nodes.add`,
   ...) because of the `{method}:{key}` prefix. Scoped and derived keys longer
   than the storage limit are now a SHA-256 digest (`persistence.rs`).
4. Task-edit commit keys collided across clients; a named client now has its
   own commit key. Compatibility: a retry that spans an upgrade of a named
   client's task edit is not replayed once.
5. `virtualRoutes.replace` answered a stale `baseRevision` as a malformed
   request; it is now `revisionConflict`. (Its text says "session" for a bus
   route revision.)
6. `safety.togglePrivacyMute` derived `toggle:{key}`, which failed for a long
   key; and MCP `create_session` / `start_from_goal` derived `{key}-open`, which
   made a long key create the session and then report failure. Both are
   bounded and deterministic (`mcp_open_key`).
7. `audioMedia.delete` without `mediaId` returned `{"deleted": false}`; it is
   now invalid params.

Recorded for the owner, not changed:

- `EMPTY_STRING_LENIENT` (7 pairs, e.g. `sessions.get` `sessionId`): an empty
  string where the schema says `minLength: 1` is accepted and means "the
  active session" or "all". Tighten the handlers or relax the schemas; the
  list is a ratchet, so a new pair fails and a fixed one must be removed.
- MCP `create_session` for an Editor creates the session but cannot open it
  (`sessions.active.set` needs SessionControl), and reports permissionDenied.
- `RETRY_ELSEWHERE` lists idempotent methods with no retry scenario yet
  (each with its reason).

Process note: the new code was written with Edit/Write. Two lapses: one
`cat >>` append to `contract_cases.rs` and one two-line `sed -i` replacing
`format!("{key}-open")` in `crates/cli/src/lib.rs`; both results were read back
and are covered by tests.

Commands (this worktree, `CARGO_BUILD_JOBS=6`): `cargo fmt --all -- --check`
and `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` clean;
`cargo clippy --workspace --all-targets --all-features -- -D warnings` and the
same for `src-tauri/Cargo.toml` clean; `cargo test -p audiorouter-control -p
audiorouter-cli -p audiorouter-transport`: all targets pass (control 466 passed
24 ignored, cli lib 66, transport 41 plus integration tests);
`cargo test --bin audiorouter-shell` in `src-tauri`: 136 passed, 4 ignored
(the HTTP invalid-input sweep takes about 2 minutes because it respects the
adapter's rate limit); `npm --prefix contracts run check:drift` passed (156
methods); `node tools/docs/validate.mjs` passed.

Not run: Playwright real-backend UI tests and the in-repo UI/Stream Deck
callers beyond a read of the upload path (chunks stay under the new
`maxLength`); the built exe; any Windows hardware. The UI's handling of the new
`virtualRoutes.replace` `revisionConflict` was not exercised.

## Q5 - Measure reliability and test quality over time

Priority: build on the existing nightly workflow. Owner: quality maintainer.
Requirements: ENG-02/03/04, NFR-04/05/06/08/11/14.

1. Extend existing metrics with first-pass failures/retries, flaky scenarios,
   test duration, open regression age, escaped defects and source SHA. Keep
   coverage/soak/startup metrics in the existing reports/history rather than a
   second dashboard. Reports must say what was not measured.
2. Establish coverage baselines before setting floors. Require explicit
   branch/invariant evidence for changed critical modules. First report coverage
   changes, then ratchet meaningful module baselines; a global percentage is
   not a substitute for sequence, negative-path or mutation tests.
3. Classify failures as product, test or infrastructure using retained evidence.
   A quarantined test needs a tracked defect, owner, bounded expiry and
   replacement protection for any critical behavior. Do not weaken a required
   gate by silently skipping a flaky test or increasing retries.
4. Keep the 20-minute engine soak as a short detector. Add a synthetic whole-app
   soak for UI polling, graph edits, restarts, handles, threads and bounded logs.
   Separately schedule isolated live whole-app endurance on the reference PC.
5. Measure UI-ready cold/warm startup, 64-node input-to-paint and telemetry
   freshness, renderer/backend memory, and callback/queue distributions.
   Separate retained JS heap from allocation churn and OS private bytes.
6. Preserve the existing specification's 8-hour/24-hour and recovery-cycle gates.
   Calibrate runner variability before gating timing regressions; investigate
   regressions on reference hardware rather than discard slow samples.

Acceptance: a seeded leak/error/slow path produces an attributable failed
check or alert with reproducible evidence; nightly failures have an owner and
are reviewed before the next release. A short synthetic soak is never reported
as the full product endurance result.

## Q6 - Ship only qualified, recoverable builds

Priority: mandatory before release. Owner: release maintainer.
Requirements: ENG-05 and existing M08 requirements.

1. Build a release candidate from a clean pinned checkout, recording the SHA,
   profiles, versions, sidecars, notices and artifact hashes. Test those exact
   artifacts, not a separately rebuilt executable that may contain other UI.
2. Complete applicable M08 standard-user install, upgrade, compatible database
   migration/rollback, missing-WebView2 and uninstall-retention checks in a
   disposable Windows environment. Keep driver qualification on its own track.
3. Require all applicable PR/full-suite checks at the candidate SHA, scoped
   fault/recovery evidence and outstanding hardware/accessibility gates.
   Rebuild and rerun affected checks after a candidate fix.
4. Keep a compatible previous package and tested database backup/restore path.
   Publish known issues and migration/recovery instructions; rollback must
   restore a compatible package/configuration pair, not just an old executable.
   No open critical/high-severity release blocker may be waived by a green test
   count; lower-severity known issues need a recorded disposition.
5. Use an opt-in preview build and structured owner feedback before a wider
   release where practical. No new telemetry, cloud service or auto-upload is
   proposed. Publishing still needs the owner's release authorization.
6. Major dependency/compiler upgrades get their own PR and local verification
   first: all TypeScript projects, config/build scripts, Stream Deck, contracts,
   production UI and packaged smoke; check Node engines against CI pins. Keep
   version bumps separate from feature edits and mechanical reformatting.

Acceptance: a candidate cannot be declared qualified from test counts alone;
its exact artifacts, required evidence and rollback are reviewable, with no
unexplained failures or silently waived gates.

## Completion and operating cadence

Start with Q1, then Q2, then Q3. Add Q4 offline checks in small scoped PRs;
collect guarded hardware evidence when the declared setup is available. Q5
runs continuously; Q6 applies to each release. This authorization covers the quality work, not new services, driver trust
changes or deferred product features. Live tests still need declared endpoints.

For every future change:

1. Before coding: name invariants, affected lifetimes, failure modes and test
   layers; update the active plan with steps, prerequisites and rollback.
2. Before merge: reproduce fixed defects; run the change-to-check profile;
   inspect the diff; require current CI and review evidence.
3. Nightly: run bounded generated/fault tests, coverage, soak and trend checks;
   triage failures and retain useful artifacts.
4. Before release: qualify the actual packaged candidate and recovery path,
   including applicable real hardware, accessibility and installer evidence.
5. After an escaped defect: record the reproduction, missed boundary, owning
   layer, regression and smallest durable prevention improvement.

Plan exit criteria: Q1 gates demonstrated; Q2 failure classes guarded; Q3
production/packaged smoke enforced; Q4 applicable backend/audio evidence
present; Q5 failures actionable; Q6 release/rollback evidence complete.

## Next actionable task (2026-11-23)

1. Verify PR #181, which now includes `fix/keybind-selection-after-failed-delete`
   and `fix/pipe-guard-claim-race` by owner request. The fixes remain pending
   delivery to main until that release PR is reviewed and merged.
2. Q4.2, next slice: backend restart while the window, tray, HTTP and MCP
   clients have requests in flight, through the real pipe, with no partial
   graph commit and no cross-session reply. The guard-pipe repair covered
   only the moment the server starts.
3. Q5.1–Q5.2: add first-pass failures and retried tests to the nightly trend
   table, and take module coverage baselines from the nightly coverage
   artifacts before setting any floor.

## Completed: Q3.2 hosted CI qualification

(Completed 2026-11-23; see the status above. Kept for its acceptance terms.)

Finish qualifying the implemented packaged-shell acceptance slice against the exact
release shell binary built by CI, with no installer or live-audio claim. Use a
disposable database, logs and WebView2 user-data directory; refuse to proceed
if the default AudioRouter control pipe is already owned; launch without
developer grants; prove the real shell-to-backend bridge and usable-canvas
readiness; exercise one session save/reload; and terminate only the child
process it started. Capture tested SHA, binary hash, OS/WebView2 version,
timings, output and failure artifacts. Report cold/warm UI-ready timings against
NFR-14's 8 s/3 s targets, but do not gate them until Q5 calibrates hosted-runner
variance. Do not expose a remote-debug port on ordinary launches; enable it
only for this child and bind it to loopback.

Requirements: UI-11/17, NFR-14/15, ENG-02/05, plus M08 evidence discipline.
Prerequisites: Windows Edge/WebView2 and pinned Node/Rust are installed; the
release shell with its embedded UI is built from the current SHA; the default
pipe is free; and the smoke owns a unique temporary profile and database.
This route does not use plugin sidecars. No physical audio device,
driver/default-device change or installer operation is part of this slice.

Ordered tasks:

1. Inspect the existing shell/frontend probe, production readiness signal,
   test harness and build output; record boundaries the harness cannot observe
   without changing product behavior.
2. Add a dedicated opt-in smoke runner/test that starts the built shell with
   isolated paths on its production default pipe, refuses to run while that
   pipe is occupied, and guarantees child cleanup on pass, failure or
   interruption.
3. Attach to that WebView2 instance over loopback CDP. Assert page load, real
   backend bridge response, `canvas-readiness` for the expected session, no
   uncaught page errors, stopped first launch and no implicit device grant.
4. Exercise save, restart/reload and recovery using a disposable session;
   assert durable state through the app/backend contract, not a screenshot
   alone. Retain bounded logs, trace and screenshot on failure.
5. Wire the smoke after the exact shell build in Windows CI, upload evidence
   even on failure, and prove negative cases (missing shell, occupied default
   pipe, bridge/readiness failure) produce attributable failures.
6. Run local Windows checks, inspect the complete diff, update the operational
   runbook and record CI limitations. Do not treat this browser smoke as
   installer, accessibility, hardware-audio or endurance acceptance.

Acceptance: a missing shell, occupied default instance, failed
native bridge, unusable canvas, uncaught UI error, startup permission leak,
failed save/reload or failed cleanup makes the check fail. A clean run records
the exact executable hash and SHA, and leaves the owner's profile and running
AudioRouter processes untouched.

Rollback: remove only the smoke runner/spec, workflow wiring and runbook
references; no product behavior, user data, session format or grant scope
changes are intended. Preserve existing fresh-install and production-browser
checks.

## Validation of the Q3.2 plan update

Local main was updated from `0bfaee77` to `11737991` while preserving the
Q3.2 work. The [2026-10-25 investigation](evidence/2026-10-25-packaged-webview-smoke.md)
records why the original restricted launches and create-only marker were not
valid product-failure evidence, the harness corrections, and two passing
packaged UI runs. Native describes now run through the ordinary Tauri API;
the diagnostic marker is cleared from inherited child environments. A normal
standard-user launch outside the tool process sandbox passed. The [local guard qualification](evidence/2026-10-25-packaged-smoke-guard-qualification.md)
now passes all 11 cases, including real console-break cancellation at three
lifecycle points and observed native/readiness/document/quit faults. Current
hosted Windows CI remains required before Q3.2 is complete.

## Original roadmap planning evidence (2026-10-21)

At roadmap creation, repository files and prior evidence were inspected; no
product tests, CI runs, hardware experiments or application changes were
performed. Documentation validation passed (180 Markdown files, 986 local
links) and `git diff --check` passed. Q3.2 implementation and current
verification progress are recorded in the active plan.
