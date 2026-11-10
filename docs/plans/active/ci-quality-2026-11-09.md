# CI and quality nightly repair

Owner authorized this task after completion of the review-fix goal.
Requirements: ENG-02/04, CAP-06/11, SEC desktop export consent lifetime; Windows.
No audio hardware, driver, credentials or new shell is needed for local tests.

## Evidence and decisions

- CI run [42562530739](https://github.com/MrDesjardins/audiorouter/actions/runs/42562530739),
  job 128981956543: `restarted_process_cannot_inherit_a_stale_binding`
  failed after killing/waiting for its helper. Inspect terminated-process
  identity handling; do not relax the fail-closed assertion or add retries.
- Nightly run [42609430776](https://github.com/MrDesjardins/audiorouter/actions/runs/42609430776),
  job 129133740208: export consent test panics subtracting 301 seconds from
  `Instant` on a fresh Windows runner. Missing workspace coverage summary is
  downstream. Shell/UI coverage, startup and soak passed.
- Main has advanced with dependency compatibility fixes; newest main CI
  [42768724084](https://github.com/MrDesjardins/audiorouter/actions/runs/42768724084)
  passed. Preserve those changes; no workflow weakening or speculative fixes.

## Ordered work and validation

1. Reproduce process identity refusal with an owned exited helper; repair
   the owning liveness check and retain denied stale/PID-reuse assertions.
2. Make export consent expiry use a forward deadline so its dispatcher
   regression can expire at now without assuming machine uptime.
3. Run focused regressions, full Windows workspace/shell tests, both Clippy
   scopes and formatting, plus coverage when available. Inspect failure
   annotations and update this plan with exact results and limits.
4. Inspect complete diff, commit, and request human approval before a PR.
   Hosted green results for the repaired head remain a separate gate.

Rollback: revert only this task's commits. No persistent schema migration,
workflow gate change, Windows device change or public API addition planned.

## Implementation evidence

Additional CI evidence: run
[42759080367](https://github.com/MrDesjardins/audiorouter/actions/runs/42759080367),
job 129640384446, failed `target-size.pw.ts` because immediate
`allInnerTexts()` returned zero Settings tabs after the transition click.
The retry passed, but fail-on-flaky correctly failed the job. Add a shared
bounded Settings-tab readiness helper and deterministic delayed-navigation
regression, then use it in the five audits with this immediate enumeration
pattern. Keep all size/contrast/field/motion assertions. Verify focused Edge,
UI formatting/lint/typecheck and the failed job's annotations. This is test
readiness repair; no product controls or styles change.

The original timing-dependent process test passed locally. A deterministic
regression now passes the exact captured inventory entry to the production
binding validator after killing and waiting for its owned helper. It failed
before the liveness repair (the exited identity was accepted), then passed.
Binding now rechecks creation time, full path and exit time through the
existing query-only Windows helper. Unverifiable/exited identities fail
closed; no retries or microphone fallback were added. The stale assertion
still requires NotFound or IdentityChanged and now prints unexpected results.
An exploratory repeat loop was invalidated by recompiling its binary during
the loop; it is not claimed as forty successful baseline runs.

Export approvals now store a five-minute forward expiry deadline. The real
dispatcher regression expires it at now, preserving its refusal assertion
without subtracting from monotonic uptime. The original nightly panic is
the pre-fix evidence; local focused export regression passes after repair.

Read all annotations on both failed owning jobs: three step-exit failures,
with the nightly missing summary downstream of the workspace test panic.
No annotations were silently waived. Full workspace/shell validation follows;
installing pinned cargo-llvm-cov 0.9.1 and llvm-tools-preview for coverage.

Full `cargo test --workspace` passed on Windows after both repairs, including
the process identity and export dispatcher tests. Full shell suite: 99 passed
/ 4 explicitly ignored. Both required all-target/all-feature Clippy scopes,
both Rust formatting checks, 61 policy tests, whitespace checks and docs
acceptance (252 files / 1449 links) passed. Logs: local
`target/ci-repair-workspace.log`, `ci-repair-shell.log`,
`ci-repair-workspace-clippy.log`, `ci-repair-shell-clippy.log`,
`ci-repair-policy.log`. No new shell or physical audio was used; owner's
release shell PID 16028 remains untouched.

Repair commit: `a4562353` on `fix/code-review-2026-11-09`.
UI coverage passed 113 files / 1005 tests, 72.26% lines
(`target/ci-repair-ui-coverage.log`). Instrumented shell coverage passed
99 tests / 4 ignored and generated
`target/ci-repair-coverage/rust-shell.json`. No workflow gates, timeouts,
assertions or retries were weakened. Upstream dependency/UI compatibility
fixes are already on main and its latest recorded CI is green; this branch
does not duplicate those changes.

Workspace instrumented tests passed, including 373 control tests and 146
Windows audio tests (explicit hardware ignores retained). Both Rust JSON
reports were generated, and the nightly's exact `publish-readme-coverage.mjs
summarize` command succeeded with those reports and UI coverage. Evidence:
`target/ci-repair-coverage-workspace.log`, `ci-repair-coverage-shell.log`, and
`ci-repair-coverage/summary.json`. This resolves the missing-summary chain
locally; no new hosted nightly result is claimed.

Settings readiness regression failed before the wait with zero pages, then
passed. The helper waits for visible Settings navigation before taking its
snapshot, retains the greater-than-five guard and lets all five audits
enumerate every page. Development Edge: 18/18 passed. UI format/check, lint
with zero warnings, typecheck, docs acceptance and diff checks passed. The
development React Flow attribution notice comes from the established
hideAttribution setting; licensing/branding authority is not established by
this test-only repair, so it stays documented rather than changing product
settings. The conflicting color-environment warning was avoided by removing
NO_COLOR only in the child test shell, without persistent environment edits.

Compiled-asset Edge checks passed 18/18 with CI=true, one worker and the
unchanged fail-on-flaky policy; no retry was used (41.7 s). Logs:
`target/ci-repair-browser.log`, `ci-repair-browser-build.log`, and
`ci-repair-browser-bundled.log`. Inspected the complete scoped diff and the
owning browser job's failure annotation (step exit, no additional finding).
Full 700-plus browser discovery, hosted nightly soak/startup and the repaired
head's hosted workflows have not been rerun locally. Focused audits cover
the changed test paths; Rust/UI coverage cover the other two repairs.

Next action: human approval to open the reviewed branch's PR, then run its
hosted CI and nightly qualification. This active plan remains open for that
hosted gate; no PR is open and none is implicitly authorized.
