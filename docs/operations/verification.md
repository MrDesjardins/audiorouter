# Development verification

Requirements: ENG-01/02/04/05. The command inventory in
[`tools/quality/checks.mjs`](../../tools/quality/checks.mjs) is shared by local
verification and [CI](../../.github/workflows/ci.yml). Run from the repository
root. Keep the pinned Rust toolchain and Node 22.22.3; install locked npm
dependencies with `npm.cmd ci --prefix ui`, `contracts`, and `tools/streamdeck`
before running checks that need them. Windows compilation also needs the
[native toolchain](sdk-setup.md). Supply checks need cargo-deny 0.20.2 and
network access. Release preparation downloads and verifies declared assets.

## Profiles

```powershell
# Inspect the commands without executing them.
node tools/quality/verify.mjs --profile full --list

# Formatting, policy, documentation, lint and all TypeScript projects.
node tools/quality/verify.mjs --profile fast

# Full verification for the changed areas, using the base you intend to merge.
node tools/quality/verify.mjs --profile full --base origin/main

# No base means full verification of every area.
node tools/quality/verify.mjs --profile full

# Run a focused check, or collect independent failures in one run.
node tools/quality/verify.mjs --checks policy,streamdeck-tests
node tools/quality/verify.mjs --profile fast --keep-going
```

Full verification includes the existing Windows workspace/shell tests, contract
drift, Edge suite, release preparation and applicable supply checks. It can take
tens of minutes. It does not establish audio hardware, accessibility, driver,
installer lifecycle or endurance acceptance. A Linux run blocks Windows-only
checks explicitly; it cannot count as a complete full run.

The classifier conservatively selects every area for an unknown executable or
build surface. Contract changes select backend, UI and Stream Deck consumers;
backend changes select browser integration. Documentation validation and policy
tests always run. Main pushes and manual CI runs select all areas. PR CI compares
the tested merge commit with its first parent, the current base tip.

## Results and evidence

Each invocation writes a new JSON report beneath
`%TEMP%/audiorouter-verification` (CI uses `RUNNER_TEMP`), with SHA, dirty-tree
flag, Node/platform, command arguments, timestamps, duration and command exits.
`--output <new-path.json>` chooses a destination; an existing report is never
overwritten. A dirty report identifies an uncommitted working-tree run, not
evidence for the recorded commit alone. Keep logs/traces alongside the report
when diagnosing a failure; reports do not capture child stdout.

Statuses distinguish `passed`, `failed`, `blocked` (such as missing executable
or wrong OS), and `not-run` after an earlier failure. Every selected check must
pass for exit code zero. `--keep-going` preserves failures while running later
checks. An interrupted workflow cannot pass the aggregate gate; an interrupted
local process may have no completed report and is never passing evidence.

CI uploads `verification-<job>` artifacts even after a failed step. Edge failures
also retain Playwright traces and screenshots. Artifacts have bounded retention;
link the run and retain necessary redacted evidence before it expires.

## Guarded existing-device probe

```powershell
node tools/quality/verify.mjs --profile hardware --list
node tools/quality/verify.mjs --profile hardware --allow-live-audio --capture-endpoint '<explicit capture ID>' --render-endpoint '<explicit render ID>' --duration-ms 500
```

This deliberately plays/routes real audio through the selected endpoints.
Use an isolated, declared fixture after authorizing the activity. IDs are never
inferred, and duration is bounded to 100-2000 ms. The short control-route probe
does not replace [continuity/DSP qualification](deterministic-audio-tests.md),
reference capture, plugin recovery or the specified endurance runs. Neither
ordinary profiles nor this probe change Windows default devices or driver trust.

## Merge and publication policy

The `Quality gate` runs after every applicable CI job, including classification,
documentation and verification-policy tests. Required jobs must succeed;
failure, cancellation, missing results and unexpected skips fail the gate.
Only explicitly unaffected jobs can be skipped. This aggregate is scoped to
its workflow's tested commit; check the current PR merge result after base changes.

Main protection was activated after PR #50's full run 39710446637 succeeded.
The effective rules require PRs, resolved threads and the GitHub Actions Quality
gate against the latest base, with no bypass actors. Deletion and force pushes
are restricted. The [active plan](../plans/active/current.md) links API evidence
and the previous settings for rollback; workflow YAML alone does not establish
enforcement. The repository currently has one
collaborator. Require PRs, resolved blocking threads and current checks; owner
review remains necessary, but a mandatory independent approval would prevent
owner-authored PRs from merging. Do not add a bot bypass to hide that limitation.

The nightly quality job publishes proposed README/history files as the
`quality-history-update` artifact from the measured SHA. Download, inspect and
submit those files in a reviewed documentation PR. It no longer pushes to main.
Coverage and short soak metrics remain reports with their existing measured
scope; they do not waive release gates in [M08](../milestones/M08-release.md).

Rollback: revert a scoped tooling/workflow commit. Before changing rules, save
the existing ruleset; retain working protections until the replacement check
has demonstrated a successful current run. Never bypass a failed product check
to merge the quality tooling itself.
