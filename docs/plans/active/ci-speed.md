# CI selection and parallel execution — 2026-11-12

Owner approved all three phases. Requirements: ENG-01/02/04/05; M08
verification tooling, without changing hardware or release acceptance gates.
Objective: select affected checks, retain a daily full inventory, shard Edge
across four isolated runners, and separate Rust, UI and release work.

Prerequisites: pinned Node/Rust, Windows native build tools, GitHub artifact
and cache services. Hosted timing/cache evidence requires Actions. Preserve
the unrelated dirty checkout; work on improve/ci-speed from origin/main.

Ordered tasks:
1. Test docs assets, named workflows and known root scripts in the classifier;
   unknown paths remain full. Add daily full CI alongside quality metrics.
2. Build browser fixtures once, transfer them to four one-worker runners.
   Select real-backend specs through transitive local imports for Rust-only
   changes; fail closed on unresolved imports. Full runs retain all specs.
3. Separate Linux UI, Windows Rust and Windows release checks; restore Rust
   caches on PRs and save only trusted main runs. Keep Quality gate fail closed.
4. Run policy/runner tests, workflow syntax validation, documentation and diff
   checks; exercise browser preparation/discovery and available local checks.

Decisions: ordinary shell Rust changes do not build installers on PRs;
packaging manifests, build scripts, release tooling and driver changes do.
Nightly/manual runs exercise every area. No change to branch protection.
No PR opening authorized by implementation approval alone.

Validation matrix: classifier exact enabled sets; gate rejects failed, missing,
cancelled and skipped required jobs; workflow wiring covers full inventory;
browser selection follows helper imports; artifact paths match consumers;
cache saves exclude PRs. Windows and Linux/Actions evidence are distinguished.

Risks: nightly can detect packaging or unmodeled browser coupling later than a
PR. Four runners increase concurrency demand. Cache eviction reduces speed,
never correctness. Artifacts include only generated fixtures, no user data.
Rollback: revert these scoped commits; existing Quality gate name stays stable.
Next: open the owner-approved PR, then verify current-base CI and the first
post-merge trusted cache/nightly runs. One cohesive branch
keeps the classifier, gate and renamed jobs compatible throughout rollout.

## Local evidence — Windows, 2026-11-12

- `node --test tools/quality/policy.test.mjs tools/quality/verify.test.mjs
  tools/metrics/code-size.test.mjs`: 79/79 pass. Exact path-area regressions,
  transitive selection and failures/skips of every new required job covered.
- `node tools/quality/browser-ci.mjs prepare`: pass. Fresh e2e_backend build,
  bundled Vite build, positive/negative browser guard and asset guard pass.
  Guard artifacts: `%TEMP%/audiorouter-browser-guard-zU7VGK` and
  `%TEMP%/audiorouter-bundled-asset-guard-5tASfs` (local transient evidence).
- `CI=1 BROWSER_SCOPE=backend BROWSER_SHARD=1/4 node
  tools/quality/verify.mjs --checks browser`: 91 pass, 10 existing explicit
  skips, 2.4 minutes. Report:
  `%TEMP%/audiorouter-verification/1794531957272-8e2def71-a0b7-4256-a635-6c045e69745d.json`.
- Playwright JSON discovery for each shard compared by spec/project identity:
  all 757 tests split 191/189/188/189; backend 293 tests in 38 files split
  101/51/97/44. Each union is complete and disjoint. Local probe/results in
  `target/check-shards.mjs` and `target/shard-discovery.txt`, intentionally
  untracked. Conservative dynamic imports include some mocked specs.
- UI clean npm install, `typecheck:all`, `format`, `format:check`, `lint` pass.
  No product UI/style changes. `actionlint` 1.7.12 passes ci.yml; downloaded
  Windows archive SHA256 matches its published checksum.
- `node tools/docs/validate.mjs`: 260 Markdown files / 1,475 links pass.
  `git diff --check` passes. Complete tracked diff and new helpers inspected.

An initial discovery invocation from the repository root exposed a cwd-dependent
selector path; the config now resolves both specs and assets relative to its
own URL. Discovery passes from the root and the real UI working directory.
The local terminal inherited conflicting NO_COLOR/FORCE_COLOR values; the
browser command now removes NO_COLOR when FORCE_COLOR is set for its children.
No Rust product code changed. Hosted Linux UI, artifact transfer,
installer/full inventory and Actions timing were pending at this local
checkpoint; the passing hosted evidence follows. Cache size remains unmeasured.

## Hosted qualification — passed

Code head: `abb0ba37696b4df6c177200b13244195e9dd1054`.
[Full manual CI](https://github.com/MrDesjardins/audiorouter/actions/runs/43343093910)
passed all 15 jobs, including Quality gate, on 2026-11-13 UTC (November 12
locally). It exercised the complete inventory, Linux UI, Windows workspace and
shell tests, strict Clippy, native probes, SDK provenance, CLI parity, contract
drift, release tooling and the unsigned installer build/verification.

The single cold run took 20m29s from creation to completion. Windows Rust/native
checks took 19m17s; installer verification 16m29s; Linux UI 1m22s; browser
preparation 2m11s. Edge shard jobs took 8m18s, 8m36s, 12m10s and 6m53s.
These are observations from one run, not warm-cache or general timing promises.
The browser totals are 742 passed and 15 existing explicit skips, with no flaky
success allowance. The unchanged flaky-test failure policy remains enabled.

All four shards verified the same 8,020,825-byte fixture artifact digest:
`498e3b6fb6561173fa44806a58c07e0aec26fdf04d01e7f5876933a003ad6cfc`.
All 27 check-result records in the downloaded verification artifacts are
`passed` (browser has four records); reports identify the clean tested SHA.
Transient local copies: `target/hosted-all-reports`, `target/hosted-full-ci.log`
and `target/hosted-shard-*.log`. Durable review evidence is the linked Actions
run and its bounded-retention artifacts.

Superseded runs 43342703113 and 43342895841 were deliberately cancelled after
adding nested-spec coverage and checksum-pinned workflow lint. Their aggregate
gates failed instead of accepting cancellation, providing a live negative check.
The current code head is the replacement, not a rerun of those older commits.

Every check-run annotation has been read through the API. Only GitHub's
Ubuntu 26 runner-image migration notices appear. Keep the established
ubuntu-latest policy for portable tooling; the hosted UI run passed on its
actual image. Product Windows/hardware acceptance is unchanged.

Build-log warnings were inspected and remain visible:
- Vite reports a 620.62 kB production application chunk above its 500 kB
  advisory threshold. Application source and bundling policy are unchanged;
  changing loading behavior is outside this CI task. No threshold was raised.
- Stream Deck Rollup strips misplaced PURE annotations in zod core util/regex
  comments and reports the core.js/util.js import cycle. Its tests and package
  build passed. The dependency graph is unchanged; no vendor edits or warning
  filters were introduced to hide these diagnostics.
- Pinned actions/download-artifact v8.0.2 emits Node DEP0005 (Buffer constructor)
  inside each artifact download, before the test command. All four downloads
  verified the expected digest and succeeded. Keep the reviewed action SHA;
  an unqualified action upgrade or vendored dependency patch would expand this
  change. This is an action dependency warning, not a browser-test failure.

Branch caches cannot save by design: the first successful trusted main run
will warm the cache. No warm-cache speed or quota claim has been established.

Owner explicitly approved opening the PR on 2026-11-12. Merged current main's
daily-retention change, preserving both active-plan entries; only their shared
insertion point conflicted. Recheck policy, docs and workflow syntax locally.
Next: open the PR for improve/ci-speed and require its current-base gate.
After merge, verify the first trusted cache save and
the scheduled full inventory. The schedule activates only after merge. Keep the
plan active until these rollout observations exist; no release/hardware gate is
waived and no branch protection was modified.

## Owner-requested main update — 2026-11-12

Merged main `32f548267` (PR #142 audit repairs) into improve/ci-speed without
conflicts, merge commit `8fc49b6961558d13e76e4b8ef8e159bcfae74533`.
Windows checks after merge: all 80 policy/runner tests, documentation validation
(262 files / 1,479 links), Actionlint and diff checks pass. No additional product
edits were made. Earlier full-run evidence applies to its recorded SHA; fresh
PR #143 CI must qualify this updated head. Next: push and review current-head CI.
