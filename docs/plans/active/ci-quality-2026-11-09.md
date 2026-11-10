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

Next action: finish instrumented coverage and inspect its reports, then
commit the reviewed repair. Hosted validation of this head remains pending.
