# Three-day code review and repairs — 2026-11-15

> **Archived 2026-11-16 — merged in PR #158. Attended M07/M08, driver-helper and load-controlled UDP gates remain open.**

## Objective and scope

Owner requested a full review of changes from November 12–15, prioritized
findings, and repairs. Review committed changes after `9c1868c1a` and the
existing device-binding/theme working changes without discarding them.
Requirements: STATE-02/07/08/10/11/12, CAP-02/11/12, API-08, SEC-02,
UI-17 and ENG-01/02/04/05; M02/M05/M07/M08.

## Prerequisites and decisions

Windows PowerShell workspace; original branch `fix/live-parameter-autosave`
was at `28e2bfbbe`. The review-fix branch is based on current `origin/main` at
`6d691e3a2`. Device-binding and theme-preview work from the original mixed
working tree had already landed separately and is not duplicated here; the
uncommitted original tree was saved in a local stash. Owner authorized a new PR
on 2026-11-15. Hardware, attended device/sleep tests, driver signing and
external credentials are separate gates. Findings describe concrete failures
rather than speculative style concerns.

## Ordered tasks

1. Read current plans, affected specifications and recent changes.
2. Review recovery/privacy, graph publication and persistence, transport/UI,
   plugins and automation; record findings by severity with dispositions.
3. Reproduce defects with focused regression tests, then repair owning code.
4. Run relevant suites, contract drift where applicable, formatting, lint,
   typecheck and strict Clippy; inspect the complete repair diff.
5. Record evidence, remaining gates and next action here and in current plan.

## Validation and rollback

Use disposable databases and synthetic audio; do not alter owner sessions,
devices or the running shell. Test API defects through dispatch and transport
where relevant. Record commands and actual outcomes. Rollback consists of
reverting only this review's scoped edits while retaining pre-existing work.

## Findings and evidence

1. **P1 — Failed plugins could be revived by automatic recovery.** Sleep
   eligibility ignored plugin failure, while the supervisor witness included
   failed routes and was refreshed only on mutations. Reject failed, vanished
   or poisoned-registry bridges for automatic recovery; recheck pending
   recovery and refresh the witness during service passes. Regressions
   `recovery_witness_drops_a_route_when_its_plugin_bridge_disappears` and
   `sleep_never_arms_recovery_for_an_unavailable_plugin` cover service and
   sleep boundaries. Files: safety.rs, native_dispatch.rs, audio_service.rs.
2. **P1 — A stale finding could replace a returned microphone.** With the
   saved endpoint active again and one other same-name capture endpoint,
   `findReplacementCaptureEndpoint` selected the other input. The new
   regression reproduced that selection before repair. Exact active IDs now
   prohibit name-based replacement (`endpointBinding.ts`).
3. **P2 — Malformed stopped-session guards silently disabled protection.**
   `graph.commit` accepted null/false/numeric/empty guard values as absent and
   committed to a running session. Dispatcher coverage now rejects null, false,
   numeric and empty-string guards with an invalid-request error.
4. **P2 — An in-flight event poll could supersede Reconnect.** The hook checked
   reconnect state only before subscription. A delayed response started a new
   snapshot request during a user reconnect. Deferred-response regression
   verifies the late reply is discarded; guards cover asynchronous boundaries
   and errors.
5. **P2 — Changing update channels could retain a prerelease in stable.** A
   previous-channel request failure wrote its cached prerelease fallback under
   the newly selected stable channel. Reproduced in updateCheckHook.test.tsx;
   preference changes now cancel and retire the old request before rechecking.
6. **P2 — VST2 editor callbacks inherited another thread's realtime status.**
   Process-wide `IN_PROCESS_CALL` reported `(2,2)` for a processing callback
   and concurrent non-processing callback. The actual process-entry regression
   expects `(2,1)`. Const thread-local state fixes it without callback locks or
   allocation; all 88 plugin-host library tests pass.
7. **P2 — Device recovery enumerated endpoints on unrelated renders.** Its
   effect depended on a newly allocated merged session array and enumerated
   inactive endpoints even with zero missing-device findings. The merged
   inventory is memoized and recovery enumeration is skipped without
   actionable findings. App regression covers startup and rerender without
   inactive queries.
8. **P2 — Older recovery failures overrode newer recovery success.** Event
   polling always preferred any failure in a batch over a later recovery.
   The regression reproduced an error message after the route had recovered.
   The newest route outcome now decides the notice, including device loss
   after recovery. Also corrected a duplicated sentence in 0.0.29's notes.

Reviewed areas include recovery and supervision, SQLite backup/restoration,
graph publication/plugin reuse/media retention, transport deadlines, UI
reconnect and autosave, update checking, plugin ABI changes, driver lifecycle
and timing changes, contracts and CI/history publication. This review is not
a claim that no other defect can exist or that hardware gates passed.

## Verification (Windows host)

- Red/green reproductions above; control safety suite: 28 passed.
- Control full library: 425 passed / 13 explicit ignores; shell: 109 passed /
  4 explicit ignores. Workspace library run passed, including Windows audio
  162 passed / 4 explicit ignores. Plugin-host after its repair: 88 passed.
- UI suite: 122 files / 1,090 tests passed. UI lint, all-source typecheck and
  format check pass. Contracts drift: 148 methods / 37 kinds / 24 event
  categories match. Docs validation: 270 Markdown files / 1,491 links.
- Quality policy/runner/history/code-size suites: 86 passed. Workspace and
  shell all-target/all-feature strict Clippy passed after the final VST2 edit.
  Both Rust format checks pass. Final event-order regression, all-source UI
  typecheck, lint and format check pass after the final UI edit.
- Native driver portable host tests: `powershell -NoProfile -ExecutionPolicy
  Bypass -File drivers/audiorouter-virtual/tests/build-tests.ps1` passed
  10,915,246 bridge checks under MSVC `/W4 /WX`; evidence in
  `target/driver-unit-release/{compile,tests}.log`. This is not a kernel,
  installed-driver or DPC timing result.
- Bundled Edge: 24 passed in compact-workspace, ux-final-qualification and
  update-notice. All three themes, autosave/manual save, session switching,
  setup navigation and update notices exercised without touching live audio.
- Current-main candidate rechecks: one full UI suite passed 125 files / 1,113
  tests. After removing a duplicated test, the final full rerun passed 1,111
  tests and timed out at 5 seconds in unrelated
  `LogFilesPanel.test.tsx::switches_verbose_logging_and_counts_down_the_hour_in_a_fixed_size_status`;
  that six-test file passed alone immediately afterward. The four focused UI
  files pass 45 tests. Focused stopped-guard and recovery-witness control tests
  pass, as do the Windows-target VST2 thread-local and sleep-recovery tests.
  UI full typecheck, lint and format;
  control/plugin-host strict Clippy; both Rust format checks; contract drift
  (156 methods / 38 kinds / 24 events); docs validation (277 files / 1,519
  links); and `git diff --check` pass.
- UI production build succeeds, with a 634.30 kB main-chunk advisory. This
  review changes correctness guards; bundle restructuring is separate work.
- Full all-target workspace run stopped at driver-helper's executable with
  Windows error 740 (requires elevation); its 30 library tests passed. A
  second all-target run excludes that package and retains a local log at
  `target/review-2026-11-15/workspace-tests.log`. Its other library/integration
  suites passed, including source combinations (5), tool combinations (5),
  deterministic audio (33), plugin workers (41 / 18 explicit ignores),
  storage (113 / 1 ignore), transport (39 / 1 ignore), and real MCP stdio (5).
  Windows-audio failed two real-time UDP tone checks in that run: paired tone
  had 10 discontinuities / 2 underruns and unpaired had 9 / 2. No assertion
  was relaxed. Each isolated rerun passed with zero discontinuities and zero
  underruns. The entire windows-audio library then passed serially: 162 /
  4 explicit ignores, 85.62 s; log `target/review-2026-11-15/windows-audio-serial.log`.
  This records load-sensitive qualification, not a clean all-target run or
  proof of zero network glitches under arbitrary host scheduling load.
- The owner's release shell is running (observed PID 31604). Do not launch
  another shell or stop it. No attended live audio, sleep/unplug, signed
  driver/VM, or installer qualification is claimed.

## Reproducible commands and remaining gates

`cargo test --workspace --lib`; `cargo test --manifest-path src-tauri/Cargo.toml`;
`cargo test --workspace --all-targets --all-features --exclude audiorouter-driver-helper`;
`cargo test -p audiorouter-windows-audio --lib -- --test-threads=1`;
`npm --prefix ui test`; `npm --prefix ui run typecheck:all`;
`npm --prefix ui run lint`; `npm --prefix ui run format:check`;
`npm --prefix contracts run check:drift`; `node tools/docs/validate.mjs`.
Clippy: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
and the same with `--manifest-path src-tauri/Cargo.toml` instead of `--workspace`.
Browser: rebuild `e2e_backend`, then from `ui/` build
`vite.browser-acceptance.config.ts` and run Playwright with
`--config=playwright.ci.config.ts e2e/compact-workspace.pw.ts e2e/ux-final-qualification.pw.ts e2e/update-notice.pw.ts --workers=2`.

Initial browser build invocation used the repository cwd and failed to resolve
its HTML entries; rerunning from `ui/` succeeded. An initial Prettier invocation
also used the wrong cwd and changed no files; the correct UI invocation passed.

No driver install, production session edits or release artifact handoff. The
review fixes are isolated from ignored build outputs and previously merged UI
features.

## Next action

PR #158 is open on `fix/code-review-2026-11-15`; inspect its hosted checks and
annotations. Follow-up M07/M08 gates remain: attended plugin-failure/sleep/device-return
verification on an isolated profile, elevated driver-helper execution, and
load-controlled UDP qualification. Hosted CI, driver signing and installer
qualification remain open.
